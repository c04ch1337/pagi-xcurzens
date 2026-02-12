//! Mimir meeting capture: gateway handlers for start/stop/status and Pre-Flight Audio Check.
//!
//! GET /api/v1/mimir/preflight runs the Pre-Flight Audio Check (mic/loopback).
//! POST /api/v1/mimir/start runs preflight then spawns MeetingRecorder and Whisper worker.
//! POST /api/v1/mimir/stop stops capture, builds the Markdown summary, saves to project folder (or data/mimir).
//! GET /api/v1/mimir/status returns current recording state for UI sync.

use {
    axum::extract::State,
    axum::http::StatusCode,
    axum::Json,
    pagi_core::SAORedactor,
    pagi_mimir::{
        create_mimir_stt, run_preflight_audio_check, MeetingRecorder, MeetingStorage,
        MeetingTranscriptRow, TranscriptSegment, WhisperTranscriptWorker,
    },
    std::path::PathBuf,
    std::sync::atomic::Ordering,
    std::sync::Arc,
    std::thread,
    tokio::sync::mpsc,
    tracing,
};

/// Session handle only (recorder runs in a dedicated thread and is !Send on Windows).
pub struct MimirSession {
    pub meeting_id: String,
    pub project_id: Option<String>,
    pub thread_id: String,
    pub stop: Arc<std::sync::atomic::AtomicBool>,
    pub join_handle: thread::JoinHandle<()>,
    pub result_rx: mpsc::Receiver<(String, Option<String>)>,
}

#[derive(serde::Deserialize)]
pub struct MimirStartRequest {
    #[serde(default)]
    pub project_id: Option<String>,
}

#[derive(serde::Serialize)]
pub struct MimirStartResponse {
    pub meeting_id: String,
    pub status: &'static str,
}

#[derive(serde::Serialize)]
pub struct MimirStopResponse {
    pub meeting_id: String,
    pub summary_path: Option<String>,
}

#[derive(serde::Serialize)]
pub struct MimirStatusResponse {
    pub recording: bool,
    pub meeting_id: Option<String>,
}

/// GET /api/v1/mimir/preflight — Run Pre-Flight Audio Check (mic/loopback). Call before "Record Meeting" turns red.
pub async fn mimir_preflight_get() -> Json<serde_json::Value> {
    let report = run_preflight_audio_check();
    Json(serde_json::json!({
        "loopback_active": report.loopback_active,
        "mic_active": report.mic_active,
        "detected_devices": report.detected_devices,
        "user_advice": report.user_advice,
    }))
}

/// POST /api/v1/mimir/start — Run preflight, then start meeting capture (mic + optional loopback), create meeting in Chronos, spawn Whisper worker.
pub async fn mimir_start_post(
    State(state): State<crate::AppState>,
    Json(body): Json<MimirStartRequest>,
) -> (StatusCode, Json<serde_json::Value>) {
    if state.mimir_session.lock().await.is_some() {
        return (
            StatusCode::CONFLICT,
            Json(serde_json::json!({
                "error": "A meeting is already recording. Stop it first."
            })),
        );
    }

    // Pre-Flight: ensure mic (and optionally loopback) are available before turning record "red".
    let preflight = run_preflight_audio_check();
    if !preflight.mic_active {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "Pre-flight failed: no default input (microphone) detected.",
                "user_advice": preflight.user_advice,
                "loopback_active": preflight.loopback_active,
                "mic_active": false,
                "detected_devices": preflight.detected_devices,
            })),
        );
    }

    let chronos_path = state.chronos_db.path().to_path_buf();
    let storage = match MeetingStorage::new(chronos_path.clone()) {
        Ok(s) => Arc::new(s),
        Err(e) => {
            tracing::warn!(target: "pagi::mimir", "MeetingStorage open failed: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": format!("Storage: {}", e) })),
            );
        }
    };

    let project_id = body.project_id.as_deref().filter(|s| !s.is_empty());
    let title = format!(
        "Meeting {}",
        chrono::Local::now().format("%Y-%m-%d %H:%M")
    );
    let meeting = match storage.create_meeting(&title, project_id) {
        Ok(m) => m,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": format!("Create meeting: {}", e) })),
            );
        }
    };
    let meeting_id = meeting.id.clone();

    // Auto-create a Chronos (KB-04) thread under the active project so the transcript can render live in the sidebar.
    // Title format: "Meeting: [Date] [Time]".
    let thread_title = format!(
        "Meeting: {}",
        chrono::Local::now().format("%Y-%m-%d %H:%M")
    );
    let chronos_db = Arc::clone(&state.chronos_db);
    let pid_for_thread = project_id.map(|s| s.to_string());
    let thread_id = match tokio::task::spawn_blocking(move || {
        chronos_db
            .create_thread(&thread_title, pid_for_thread.as_deref())
            .map(|t| t.id)
    })
    .await
    {
        Ok(Ok(id)) => id,
        Ok(Err(e)) => {
            tracing::warn!(target: "pagi::mimir", "Chronos create_thread failed: {}", e);
            uuid::Uuid::new_v4().to_string()
        }
        Err(e) => {
            tracing::warn!(target: "pagi::mimir", "Chronos create_thread join failed: {}", e);
            uuid::Uuid::new_v4().to_string()
        }
    };

    let stop = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let (result_tx, result_rx) = mpsc::channel(1);
    let project_id_owned = project_id.map(String::from);
    let local_path_snapshot = if let Some(ref pid) = project_id_owned {
        let assocs = state.project_associations.read().await;
        assocs.get(pid).map(|a| a.local_path.clone())
    } else {
        None
    };

    // Capture near-live transcript segments and append them into Chronos messages so the UI updates in real time.
    let (segment_tx, mut segment_rx) = tokio::sync::mpsc::unbounded_channel::<TranscriptSegment>();
    let chronos_db_for_pump = Arc::clone(&state.chronos_db);
    let tid_for_pump = thread_id.clone();
    let pid_for_pump = project_id_owned.clone();
    let meeting_id_for_pump = meeting_id.clone();
    let segment_pump_handle = tokio::spawn(async move {
        while let Some(seg) = segment_rx.recv().await {
            let content = seg.text.trim().to_string();
            if content.is_empty() {
                continue;
            }

            let metadata_json = serde_json::json!({
                "source": "mimir",
                "meeting_id": meeting_id_for_pump,
                "timestamp_sec": seg.timestamp,
            })
            .to_string();

            let chronos_db = Arc::clone(&chronos_db_for_pump);
            let tid = tid_for_pump.clone();
            let pid = pid_for_pump.clone();
            let _ = tokio::task::spawn_blocking(move || {
                let _ = chronos_db.ensure_thread_exists(&tid, "", pid.as_deref());
                let _ = chronos_db.append_message(
                    &tid,
                    pid.as_deref(),
                    "assistant",
                    &content,
                    Some(&metadata_json),
                );
            })
            .await;
        }
    });

    let stop_th = Arc::clone(&stop);
    let meeting_id_th = meeting_id.clone();
    let _project_id_th = project_id_owned.clone();
    let thread_id_th = thread_id.clone();
    let chronos_db_th = Arc::clone(&state.chronos_db);
    let data_dir = chronos_path
        .parent()
        .and_then(|p| p.parent())
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| chronos_path.clone());
    let join_handle = thread::spawn(move || {
        let storage = match MeetingStorage::new(chronos_path) {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!(target: "pagi::mimir", "Thread: MeetingStorage open failed: {}", e);
                let _ = result_tx.try_send((meeting_id_th, None));
                return;
            }
        };
        let recorder = match MeetingRecorder::new(60 * 5, true) {
            Ok(r) => r,
            Err(e) => {
                tracing::warn!(target: "pagi::mimir", "Thread: MeetingRecorder::new failed: {}", e);
                let _ = result_tx.try_send((meeting_id_th, None));
                return;
            }
        };
        let buffer = recorder.buffer();
        let stt = match create_mimir_stt() {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!(target: "pagi::mimir", "Thread: STT init failed: {}", e);
                let _ = result_tx.try_send((meeting_id_th, None));
                return;
            }
        };
        let storage = Arc::new(storage);
        let worker = WhisperTranscriptWorker::new(
            buffer,
            stt,
            15,
            meeting_id_th.clone(),
            Some(Arc::clone(&storage)),
            Some(segment_tx),
        );
        worker.run_blocking(stop_th);

        let transcripts = match storage.list_transcripts(&meeting_id_th) {
            Ok(t) => t,
            Err(e) => {
                tracing::warn!(target: "pagi::mimir", "Thread: list_transcripts failed: {}", e);
                let _ = result_tx.try_send((meeting_id_th.clone(), None));
                return;
            }
        };
        let title = format!("Meeting {}", chrono::Local::now().format("%Y-%m-%d %H:%M"));
        let transcript_body: String = transcripts
            .iter()
            .map(|r| format!("- [{:.0}s] {}", r.timestamp_sec, r.text))
            .collect::<Vec<_>>()
            .join("\n");
        let mut redactor = SAORedactor::load_from_data_dir(&data_dir).unwrap_or_else(|_| SAORedactor::empty());
        if let Some(ref lp) = local_path_snapshot {
            let sao_policy = PathBuf::from(lp).join(".sao_policy");
            if sao_policy.exists() {
                let _ = redactor.merge_terms_from_path(&sao_policy);
            }
        }
        let transcript_body = redactor.sanitize_transcript(transcript_body);
        let summary_md = format!(
            "# {}\n\n*Recorded: {}*\n\n## Transcript\n\n{}",
            title,
            chrono::Local::now().format("%Y-%m-%d %H:%M"),
            transcript_body
        );
        let summary_path = if let Some(ref lp) = local_path_snapshot {
            let p = PathBuf::from(lp).join(format!(
                "meeting_{}.md",
                chrono::Local::now().format("%Y%m%d_%H%M")
            ));
            if let Some(parent) = p.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            if std::fs::write(&p, &summary_md).is_ok() {
                p.to_str().map(String::from)
            } else {
                None
            }
        } else {
            None
        };
        let path_str = summary_path.or_else(|| {
            let fallback = PathBuf::from("data")
                .join("mimir")
                .join(format!("{}.md", meeting_id_th));
            if let Some(parent) = fallback.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            if std::fs::write(&fallback, &summary_md).is_ok() {
                fallback.to_str().map(String::from)
            } else {
                None
            }
        });
        if let Err(e) = storage.end_meeting(&meeting_id_th, path_str.as_deref()) {
            tracing::warn!(target: "pagi::mimir", "Thread: end_meeting failed: {}", e);
        }

        // Best-effort: rename the Chronos thread to a summarized title.
        // Try intelligent LLM-based title first, fall back to heuristic.
        let rt = match tokio::runtime::Runtime::new() {
            Ok(r) => r,
            Err(e) => {
                tracing::warn!(target: "pagi::mimir", "Thread: runtime failed: {}", e);
                let _ = result_tx.try_send((meeting_id_th, path_str));
                return;
            }
        };
        
        let next_title = rt.block_on(async {
            // Try intelligent title generation first
            if let Some(title) = generate_intelligent_title(&transcripts).await {
                Some(title)
            } else {
                // Fall back to heuristic
                derive_meeting_thread_title(&transcripts)
            }
        });
        
        if let Some(title) = next_title {
            let chronos_db = Arc::clone(&chronos_db_th);
            let tid = thread_id_th.clone();
            rt.block_on(async move {
                let _ = tokio::task::spawn_blocking(move || chronos_db.rename_thread(&tid, &title)).await;
            });
        }

        let _ = result_tx.try_send((meeting_id_th, path_str));
    });

    let session = MimirSession {
        meeting_id: meeting_id.clone(),
        project_id: project_id_owned,
        thread_id: thread_id.clone(),
        stop,
        join_handle,
        result_rx,
    };
    state.mimir_session.lock().await.replace(session);

    // Spawn the segment pump handle separately (not stored in session since it's async).
    tokio::spawn(segment_pump_handle);

    tracing::info!(target: "pagi::mimir", "Starting capture for project {:?}, thread_id={}", project_id, thread_id);
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "meeting_id": meeting_id,
            "thread_id": thread_id,
            "project_id": project_id,
            "status": "recording"
        })),
    )
}

/// POST /api/v1/mimir/stop — Stop capture, build Markdown summary, write to project folder, end meeting.
pub async fn mimir_stop_post(
    State(state): State<crate::AppState>,
) -> (StatusCode, Json<serde_json::Value>) {
    let mut guard = state.mimir_session.lock().await;
    let session = match guard.take() {
        Some(s) => s,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "error": "No meeting is recording." })),
            );
        }
    };

    session.stop.store(true, Ordering::Relaxed);
    let join_handle = session.join_handle;
    let mut result_rx = session.result_rx;
    let meeting_id = session.meeting_id.clone();
    let thread_id = session.thread_id.clone();
    drop(guard);

    let _ = tokio::task::spawn_blocking(move || join_handle.join()).await;
    let summary_path = result_rx
        .recv()
        .await
        .map(|(_, path)| path)
        .unwrap_or(None);

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "meeting_id": meeting_id,
            "thread_id": thread_id,
            "summary_path": summary_path
        })),
    )
}

fn derive_meeting_thread_title(transcripts: &[MeetingTranscriptRow]) -> Option<String> {
    if transcripts.is_empty() {
        return None;
    }

    // Heuristic: find a salient ALLCAPS token (often a product/acronym) in early segments.
    let joined = transcripts
        .iter()
        .take(24)
        .map(|r| r.text.as_str())
        .collect::<Vec<_>>()
        .join(" ");

    let mut best_caps: Option<String> = None;
    for token in joined
        .split(|c: char| !c.is_ascii_alphanumeric() && c != '-' && c != '_')
        .map(str::trim)
        .filter(|t| t.len() >= 4)
    {
        let has_alpha = token.chars().any(|c| c.is_ascii_alphabetic());
        let is_all_caps = token
            .chars()
            .filter(|c| c.is_ascii_alphabetic())
            .all(|c| c.is_ascii_uppercase());
        if has_alpha && is_all_caps {
            if best_caps.as_ref().map(|b| token.len() > b.len()).unwrap_or(true) {
                best_caps = Some(token.to_string());
            }
        }
    }
    if let Some(caps) = best_caps {
        return Some(format!("SAO Briefing regarding {}", caps));
    }

    // Fallback: first N words of first segment.
    let first = transcripts[0].text.trim();
    if first.is_empty() {
        return None;
    }
    let words = first
        .split_whitespace()
        .take(8)
        .collect::<Vec<_>>()
        .join(" ");
    Some(format!("Meeting: {}", words))
}

/// Generate an intelligent meeting title using LLM (OpenRouter).
/// Extracts first 500 chars of transcript and asks the LLM for a concise 3-7 word title.
/// Falls back to heuristic title if LLM call fails or API key is missing.
async fn generate_intelligent_title(transcripts: &[MeetingTranscriptRow]) -> Option<String> {
    use pagi_core::prompts::{MIMIR_TITLE_SYSTEM, mimir_title_user_prompt};
    use serde::{Deserialize, Serialize};
    
    if transcripts.is_empty() {
        return None;
    }

    // Build transcript excerpt (first 500 chars)
    let joined = transcripts
        .iter()
        .take(24)
        .map(|r| format!("[{:.0}s] {}", r.timestamp_sec, r.text))
        .collect::<Vec<_>>()
        .join("\n");
    
    let excerpt = if joined.len() > 500 {
        &joined[..500]
    } else {
        &joined
    };

    // Try to get API key
    let api_key = std::env::var("OPENROUTER_API_KEY")
        .or_else(|_| {
            use pagi_core::UserConfig;
            UserConfig::load()
                .ok()
                .and_then(|c| c.get_api_key())
                .ok_or_else(|| std::env::VarError::NotPresent)
        });
    
    if let Ok(key) = api_key {
        // Build the request using the title generation prompt
        let user_prompt = mimir_title_user_prompt(excerpt);
        
        #[derive(Serialize)]
        struct ChatRequest {
            model: String,
            messages: Vec<ChatMessage>,
            temperature: f32,
            max_tokens: u32,
        }
        
        #[derive(Serialize)]
        struct ChatMessage {
            role: String,
            content: String,
        }
        
        #[derive(Deserialize)]
        struct ChatResponse {
            choices: Vec<ChatChoice>,
        }
        
        #[derive(Deserialize)]
        struct ChatChoice {
            message: ChatMessageResponse,
        }
        
        #[derive(Deserialize)]
        struct ChatMessageResponse {
            content: String,
        }
        
        let client = reqwest::Client::new();
        let url = "https://openrouter.ai/api/v1/chat/completions";
        
        let body = ChatRequest {
            model: "meta-llama/llama-3.3-70b-instruct".to_string(),
            messages: vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: MIMIR_TITLE_SYSTEM.to_string(),
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: user_prompt,
                },
            ],
            temperature: 0.3,
            max_tokens: 50,
        };
        
        match client
            .post(url)
            .header("Authorization", format!("Bearer {}", key))
            .header("HTTP-Referer", "https://pagi-sovereign.local")
            .header("X-Title", "PAGI-Mimir-TitleGen")
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
        {
            Ok(res) if res.status().is_success() => {
                if let Ok(chat_res) = res.json::<ChatResponse>().await {
                    if let Some(choice) = chat_res.choices.first() {
                        let title = choice.message.content.trim().trim_matches('"').to_string();
                        if !title.is_empty() && title.len() < 100 {
                            tracing::info!(target: "pagi::mimir", "Generated intelligent title: {}", title);
                            return Some(title);
                        }
                    }
                }
            }
            Ok(res) => {
                tracing::warn!(target: "pagi::mimir", "LLM title generation failed: HTTP {}", res.status());
            }
            Err(e) => {
                tracing::warn!(target: "pagi::mimir", "LLM title generation request failed: {}", e);
            }
        }
    }

    // Fallback to heuristic if LLM fails
    None
}

/// GET /api/v1/mimir/status — Return current recording state for UI (e.g. "Recording..." pulse).
pub async fn mimir_status_get(
    State(state): State<crate::AppState>,
) -> Json<serde_json::Value> {
    let guard = state.mimir_session.lock().await;
    if let Some(ref s) = *guard {
        Json(serde_json::json!({
            "recording": true,
            "meeting_id": s.meeting_id,
            "thread_id": s.thread_id,
            "project_id": s.project_id,
        }))
    } else {
        Json(serde_json::json!({
            "recording": false,
            "meeting_id": null,
            "thread_id": null,
            "project_id": null,
        }))
    }
}
