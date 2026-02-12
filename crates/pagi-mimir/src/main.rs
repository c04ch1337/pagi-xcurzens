//! Mimir CLI: meeting recording with near-live transcription and .md summary.
//!
//! Usage:
//!   cargo run -p pagi-mimir -- --record [--duration 30] [--project "Project: SAO Update"]
//!
//! Records from default mic (and optional loopback), transcribes every 15–30s via Whisper-Core,
//! stores in Chronos, and writes a summary .md to the associated project folder.

use pagi_mimir::{MeetingRecorder, MeetingStorage, WhisperTranscriptWorker, create_mimir_stt};
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tracing::info;

fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();

    let mut args = std::env::args().skip(1);
    let record = args.next().as_deref() == Some("--record");
    let mut duration_secs: u64 = 30;
    let mut project_name: Option<String> = None;
    let mut use_loopback = false;

    while let Some(a) = args.next() {
        match a.as_str() {
            "--duration" => {
                if let Some(d) = args.next() {
                    duration_secs = d.parse().unwrap_or(30);
                }
            }
            "--project" => {
                project_name = args.next();
            }
            "--loopback" => use_loopback = true,
            _ => {}
        }
    }

    if !record {
        eprintln!("Mimir — Meeting Documentation Layer");
        eprintln!("  --record           Start recording (default 30s, or --duration N)");
        eprintln!("  --duration N        Recording length in seconds (default 30)");
        eprintln!("  --project \"Name\"    Associate meeting with Chronos project (by name)");
        eprintln!("  --loopback          Also capture system output (Stereo Mix / virtual cable)");
        eprintln!();
        eprintln!("Requires WHISPER_MODEL_PATH for real transcription (else placeholder).");
        eprintln!("Chronos DB: PAGI_STORAGE_PATH or ./data → data/pagi_chronos/chronos.sqlite");
        return Ok(());
    }

    info!("Mimir: starting recording for {}s", duration_secs);

    let storage = Arc::new(MeetingStorage::open_default()?);
    let project_id = project_name
        .as_ref()
        .and_then(|n| storage.get_project_by_name(n).ok().flatten())
        .map(|p| p.id.clone());
    let project_id = project_id.as_deref();

    let title = format!(
        "Meeting {}",
        chrono::Local::now().format("%Y-%m-%d %H:%M")
    );
    let meeting = storage.create_meeting(&title, project_id)?;
    let meeting_id = meeting.id.clone();

    let recorder = MeetingRecorder::new(60 * 5, use_loopback)?;
    let buffer = recorder.buffer();

    let stt = create_mimir_stt()?;
    let worker = WhisperTranscriptWorker::new(
        buffer,
        stt,
        15,
        meeting_id.clone(),
        Some(Arc::clone(&storage)),
        None,
    );

    let stop = Arc::new(AtomicBool::new(false));
    let stop_worker = Arc::clone(&stop);
    let worker_handle = thread::spawn(move || {
        worker.run_blocking(stop_worker);
    });

    info!("Mimir: recording... (Ctrl+C to stop)");
    thread::sleep(Duration::from_secs(duration_secs));
    stop.store(true, std::sync::atomic::Ordering::Relaxed);
    let _ = worker_handle.join();

    let transcripts = storage.list_transcripts(&meeting_id)?;
    let summary_md = format!(
        "# {}\n\n*Recorded: {}*\n\n## Transcript\n\n{}",
        title,
        chrono::Local::now().format("%Y-%m-%d %H:%M"),
        transcripts
            .iter()
            .map(|r| format!("- [{:.0}s] {}", r.timestamp_sec, r.text))
            .collect::<Vec<_>>()
            .join("\n")
    );

    let (summary_path, _project_folder) = project_folder_and_path(storage.as_ref(), project_id)?;
    if let Some(path) = summary_path {
        std::fs::write(&path, &summary_md)?;
        info!("Mimir: summary written to {}", path.display());
        storage.end_meeting(&meeting_id, path.to_str())?;
    } else {
        let fallback = MeetingStorage::default_path();
        let fallback = fallback.parent().unwrap().join("mimir").join(format!("{}.md", meeting_id));
        if let Some(p) = fallback.parent() {
            let _ = std::fs::create_dir_all(p);
        }
        std::fs::write(&fallback, &summary_md)?;
        info!("Mimir: summary written to {}", fallback.display());
        storage.end_meeting(&meeting_id, fallback.to_str())?;
    }

    info!("Mimir: done. Meeting id = {}", meeting_id);
    Ok(())
}

/// Resolve project folder path (from gateway project_associations or Chronos project name) and summary path.
fn project_folder_and_path(
    storage: &MeetingStorage,
    project_id: Option<&str>,
) -> Result<(Option<PathBuf>, Option<PathBuf>), Box<dyn std::error::Error + Send + Sync>> {
    let project_id = match project_id {
        Some(id) => id,
        None => return Ok((None, None)),
    };
    let name = storage.get_project_name(project_id)?.unwrap_or_else(|| project_id.to_string());
    let data_base = std::env::var("PAGI_STORAGE_PATH").unwrap_or_else(|_| "./data".to_string());
    let assoc_path = PathBuf::from(&data_base).join("project_associations.json");
    let content = match std::fs::read_to_string(&assoc_path) {
        Ok(c) => c,
        Err(_) => return Ok((None, None)),
    };
    let assoc: serde_json::Value = serde_json::from_str(&content).unwrap_or(serde_json::json!({}));
    let local_path = assoc
        .get(project_id)
        .and_then(|v| v.get("local_path"))
        .and_then(|v| v.as_str());
    let local_path = match local_path {
        Some(p) => PathBuf::from(p),
        None => PathBuf::from(&data_base).join("mimir").join(name.replace(' ', "_")),
    };
    let _ = std::fs::create_dir_all(&local_path);
    let summary_path = local_path.join(format!("meeting_{}.md", chrono::Local::now().format("%Y%m%d_%H%M")));
    Ok((Some(summary_path), Some(local_path)))
}
