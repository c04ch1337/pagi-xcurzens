//! Audio capture engine: cpal mic + optional loopback → synchronized mono 16 kHz PCM.
//!
//! On Windows, loopback typically requires "Stereo Mix" or a virtual audio cable
//! (exposed as an input device). Pre-flight audio check should verify loopback is active.

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, Stream, StreamConfig};
use std::sync::Arc;
use std::time::Instant;
use tracing::{info, warn};

/// Target sample rate for Whisper (Whisper-Core standard).
pub const WHISPER_SAMPLE_RATE: u32 = 16000;

/// Names that suggest a loopback / "what you hear" input (Windows: Stereo Mix, etc.).
const LOOPBACK_DEVICE_PATTERNS: &[&str] = &[
    "stereo mix",
    "wave out mix",
    "what u hear",
    "loopback",
    "system output",
];

/// Meeting recorder: captures from default input and optional loopback into a single mono 16 kHz buffer.
pub struct MeetingRecorder {
    /// Shared buffer: mono f32 at 16 kHz (Whisper standard). New samples appended by capture threads.
    buffer: Arc<MimirAudioBuffer>,
    /// Streams to keep alive (input device streams).
    _streams: Vec<Stream>,
}

/// Shared PCM buffer. Writer threads append; reader (Whisper worker) takes chunks.
pub struct MimirAudioBuffer {
    samples: std::sync::Mutex<Vec<f32>>,
    max_samples: usize,
    started_at: Instant,
}

impl MimirAudioBuffer {
    /// max_samples = e.g. 30 seconds at 16 kHz = 480_000.
    pub fn new(max_samples: usize) -> Self {
        Self {
            samples: std::sync::Mutex::new(Vec::with_capacity(max_samples.min(960_000))),
            max_samples,
            started_at: Instant::now(),
        }
    }

    /// Append mono f32 samples (must already be 16 kHz).
    pub fn push(&self, samples: &[f32]) {
        let mut g = self.samples.lock().unwrap();
        g.extend_from_slice(samples);
        let len = g.len();
        if len > self.max_samples {
            let drop = len - self.max_samples;
            g.drain(..drop);
        }
    }

    /// Take up to `n` samples from the front (for Whisper chunk). Returns (samples, elapsed_sec).
    pub fn take_chunk(&self, n: usize) -> (Vec<f32>, f64) {
        let mut g = self.samples.lock().unwrap();
        let len = g.len().min(n);
        let chunk: Vec<f32> = g.drain(..len).collect();
        let elapsed = self.started_at.elapsed().as_secs_f64();
        (chunk, elapsed)
    }

    /// Total samples currently in buffer.
    pub fn len(&self) -> usize {
        self.samples.lock().unwrap().len()
    }

    pub fn started_at(&self) -> Instant {
        self.started_at
    }
}

impl MeetingRecorder {
    /// Build recorder: default input device + optional loopback (first input matching loopback pattern).
    /// Both streams are mixed and resampled to mono 16 kHz.
    pub fn new(
        chunk_duration_secs: u64,
        use_loopback: bool,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let host = cpal::default_host();
        let default_input = host
            .default_input_device()
            .ok_or_else(|| "No default input device")?;
        let default_name = default_input.name().unwrap_or_else(|_| "Default Input".to_string());

        let mut devices_to_use: Vec<(cpal::Device, String)> = vec![(default_input, default_name)];

        if use_loopback {
            if let Some(loopback) = host
                .input_devices()?
                .find(|d| Self::is_loopback_device(d))
            {
                let name = loopback.name().unwrap_or_else(|_| "Loopback".to_string());
                info!("Mimir: using loopback input: {}", name);
                devices_to_use.push((loopback, name));
            } else {
                warn!(
                    "Mimir: no loopback input found (enable Stereo Mix or virtual cable). Using mic only."
                );
            }
        }

        let max_samples = (chunk_duration_secs as usize) * (WHISPER_SAMPLE_RATE as usize) * 2;
        let buffer = Arc::new(MimirAudioBuffer::new(max_samples));

        let mut streams = Vec::new();
        for (device, label) in devices_to_use {
            let stream = Self::build_input_stream(Arc::clone(&buffer), &device, &label)?;
            stream.play()?;
            streams.push(stream);
        }

        info!(
            "Mimir: recording from {} device(s), mono {} Hz",
            streams.len(),
            WHISPER_SAMPLE_RATE
        );
        Ok(Self {
            buffer,
            _streams: streams,
        })
    }

    fn is_loopback_device(device: &cpal::Device) -> bool {
        let name = device.name().unwrap_or_default().to_lowercase();
        LOOPBACK_DEVICE_PATTERNS
            .iter()
            .any(|p| name.contains(*p))
    }

    /// Build one input stream; resample and mix to mono 16 kHz into shared buffer.
    fn build_input_stream(
        buffer: Arc<MimirAudioBuffer>,
        device: &cpal::Device,
        _label: &str,
    ) -> Result<Stream, Box<dyn std::error::Error + Send + Sync>> {
        let config = device.default_input_config().map_err(|e| format!("{:?}", e))?;
        let sample_rate = config.sample_rate().0;
        let channels = config.channels() as usize;

        let stream_config: StreamConfig = config.clone().into();
        let buffer_ref = Arc::clone(&buffer);
        let need_resample = sample_rate != WHISPER_SAMPLE_RATE;

        let stream = match config.sample_format() {
            SampleFormat::F32 => device.build_input_stream(
                &stream_config,
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    let mono_16k = if need_resample || channels > 1 {
                        Self::to_mono_16k(data, channels, sample_rate)
                    } else {
                        data.to_vec()
                    };
                    buffer_ref.push(&mono_16k);
                },
                move |err| warn!("Mimir audio stream error: {}", err),
                None,
            )?,
            SampleFormat::I16 => {
                let buffer_ref2 = Arc::clone(&buffer);
                device.build_input_stream(
                    &stream_config,
                    move |data: &[i16], _: &cpal::InputCallbackInfo| {
                        let f32_samples: Vec<f32> = data
                            .iter()
                            .map(|&s| s as f32 / 32768.0f32)
                            .collect();
                        let mono_16k = if need_resample || channels > 1 {
                            Self::to_mono_16k(&f32_samples, channels, sample_rate)
                        } else {
                            f32_samples
                        };
                        buffer_ref2.push(&mono_16k);
                    },
                    move |err| warn!("Mimir audio stream error: {}", err),
                    None,
                )?
            }
            _ => return Err("Unsupported sample format (need F32 or I16)".into()),
        };

        Ok(stream)
    }

    /// Convert interleaved multi-channel at any rate to mono 16 kHz (linear interpolation).
    fn to_mono_16k(samples: &[f32], channels: usize, from_rate: u32) -> Vec<f32> {
        if channels == 0 || samples.is_empty() {
            return Vec::new();
        }
        let mono: Vec<f32> = if channels == 1 {
            samples.to_vec()
        } else {
            samples
                .chunks_exact(channels)
                .map(|c| c.iter().sum::<f32>() / channels as f32)
                .collect()
        };
        if from_rate == WHISPER_SAMPLE_RATE {
            return mono;
        }
        // Resample to 16 kHz
        let out_len = (mono.len() as u64 * WHISPER_SAMPLE_RATE as u64 / from_rate as u64) as usize;
        let mut out = Vec::with_capacity(out_len);
        for i in 0..out_len {
            let src_idx = (i as f64 * from_rate as f64 / WHISPER_SAMPLE_RATE as f64) as usize;
            if src_idx >= mono.len() {
                break;
            }
            out.push(mono[src_idx]);
        }
        out
    }

    /// Reference to the shared buffer (for Whisper worker).
    pub fn buffer(&self) -> Arc<MimirAudioBuffer> {
        Arc::clone(&self.buffer)
    }
}
