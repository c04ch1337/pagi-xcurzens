//! Pre-flight audio check: verify mic and optional loopback are available before recording.
//!
//! On Windows, loopback typically requires "Stereo Mix" or a virtual audio cable.
//! Phoenix can advise: "Loopback not detected. Please enable 'Stereo Mix' in Windows Sound Settings."

use cpal::traits::{DeviceTrait, HostTrait};
use serde::Serialize;

/// Names that suggest a loopback / "what you hear" input (Windows: Stereo Mix, etc.).
const LOOPBACK_DEVICE_PATTERNS: &[&str] = &[
    "stereo mix",
    "wave out mix",
    "what u hear",
    "loopback",
    "system output",
];

/// Result of the pre-flight audio check. JSON-serializable for API and skill response.
#[derive(Debug, Clone, Serialize)]
pub struct PreFlightAudioReport {
    /// True if at least one input device matches a loopback pattern (e.g. Stereo Mix).
    pub loopback_active: bool,
    /// True if a default input device (mic) is available.
    pub mic_active: bool,
    /// Human-readable list of detected input and output device names.
    pub detected_devices: DetectedDevices,
    /// Optional message for the user when loopback is missing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_advice: Option<String>,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct DetectedDevices {
    pub inputs: Vec<String>,
    pub outputs: Vec<String>,
}

/// Run the pre-flight audio check using cpal. Safe to call from any thread.
pub fn run_preflight_audio_check() -> PreFlightAudioReport {
    let host = cpal::default_host();

    let default_input = host.default_input_device();
    let mic_active = default_input.is_some();

    let inputs: Vec<String> = host
        .input_devices()
        .map(|devices| {
            devices
                .filter_map(|d| d.name().ok())
                .collect()
        })
        .unwrap_or_default();

    let outputs: Vec<String> = host
        .output_devices()
        .map(|devices| {
            devices
                .filter_map(|d| d.name().ok())
                .collect()
        })
        .unwrap_or_default();

    let loopback_active = inputs
        .iter()
        .any(|name| {
            let lower = name.to_lowercase();
            LOOPBACK_DEVICE_PATTERNS.iter().any(|p| lower.contains(*p))
        });

    let user_advice = if !loopback_active && mic_active {
        Some(
            "Loopback not detected. Please enable 'Stereo Mix' in Windows Sound Settings, or install a virtual audio cable, to capture system audio.".to_string()
        )
    } else if !mic_active {
        Some(
            "No default input device (microphone) found. Please check your sound settings.".to_string()
        )
    } else {
        None
    };

    PreFlightAudioReport {
        loopback_active,
        mic_active,
        detected_devices: DetectedDevices { inputs, outputs },
        user_advice,
    }
}
