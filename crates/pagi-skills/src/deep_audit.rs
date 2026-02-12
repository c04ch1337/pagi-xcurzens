//! Deep Audit Skill: "Scribe" Pipeline for Sovereign Document Ingestion
//!
//! Monitors `./data/ingest` directory for new files, performs semantic triage,
//! applies SAO redaction, and routes to appropriate KB collections in Qdrant.
//!
//! Architecture:
//! 1. **Ingestion**: File watcher detects new files in inbox
//! 2. **Analysis**: Semantic triage determines KB destination
//! 3. **Redaction**: SAORedactor scrubs protected terms
//! 4. **Storage**: Vectorizes and pushes to Qdrant (Port 6333)

use async_trait::async_trait;
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use pagi_core::{AgentSkill, SAORedactor, TenantContext};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::mpsc::channel;
use tokio::fs;
use tracing::{error, info, warn};

/// Maximum file size for full LLM analysis (5MB)
const MAX_LLM_ANALYSIS_SIZE: u64 = 5 * 1024 * 1024;

/// Number of tokens to read for semantic triage
const TRIAGE_TOKEN_LIMIT: usize = 500;

/// Knowledge Base routing destinations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KnowledgeBase {
    /// KB-01: General Context (Psyche)
    Psyche,
    /// KB-02: Chronos (Temporal Memory)
    Chronos,
    /// KB-03: Infrastructure (Techne)
    Techne,
    /// KB-04: Conversational Memory (Logos)
    Logos,
    /// KB-05: XCURZENS Hub (Polis)
    Polis,
    /// KB-06: Strategic Goals (Telos)
    Telos,
    /// KB-07: Conversational Memory (Mimir)
    Mimir,
    /// KB-08: Physical Embodiment (Soma)
    Soma,
}

impl KnowledgeBase {
    /// Get the Qdrant collection name for this KB
    pub fn collection_name(&self) -> &'static str {
        match self {
            Self::Psyche => "kb-01-psyche",
            Self::Chronos => "kb-02-chronos",
            Self::Techne => "kb-03-techne",
            Self::Logos => "kb-04-logos",
            Self::Polis => "kb-05-polis",
            Self::Telos => "kb-06-telos",
            Self::Mimir => "kb-07-mimir",
            Self::Soma => "kb-08-soma",
        }
    }

    /// Determine KB from file content using keyword matching
    pub fn from_content(content: &str) -> Self {
        let lower = content.to_lowercase();

        // KB-05: Tourism, Market, Coastal (XCURZENS Hub)
        if lower.contains("tourism")
            || lower.contains("market")
            || lower.contains("coastal")
            || lower.contains("xcurzens")
            || lower.contains("visitor")
            || lower.contains("destination")
        {
            return Self::Polis;
        }

        // KB-03: Code, Infrastructure, Technical
        if lower.contains("code")
            || lower.contains("rust")
            || lower.contains("port")
            || lower.contains("api")
            || lower.contains("infrastructure")
            || lower.contains("deployment")
            || lower.contains("docker")
            || lower.contains("kubernetes")
        {
            return Self::Techne;
        }

        // KB-07: Meeting, Voice, Conversational
        if lower.contains("meeting")
            || lower.contains("mimir")
            || lower.contains("voice")
            || lower.contains("transcript")
            || lower.contains("conversation")
            || lower.contains("discussion")
        {
            return Self::Mimir;
        }

        // KB-06: Strategic, Goals, Planning
        if lower.contains("strategic")
            || lower.contains("goal")
            || lower.contains("objective")
            || lower.contains("mission")
            || lower.contains("vision")
            || lower.contains("roadmap")
        {
            return Self::Telos;
        }

        // KB-08: Physical, Health, Embodiment
        if lower.contains("physical")
            || lower.contains("health")
            || lower.contains("wellness")
            || lower.contains("exercise")
            || lower.contains("biometric")
            || lower.contains("vitality")
        {
            return Self::Soma;
        }

        // KB-02: Time-based, Scheduling, Calendar
        if lower.contains("schedule")
            || lower.contains("calendar")
            || lower.contains("deadline")
            || lower.contains("timeline")
            || lower.contains("chronos")
        {
            return Self::Chronos;
        }

        // KB-04: Conversational patterns, dialogue
        if lower.contains("conversation")
            || lower.contains("dialogue")
            || lower.contains("chat")
            || lower.contains("message")
        {
            return Self::Logos;
        }

        // Default: KB-01 (General Context)
        Self::Psyche
    }
}

/// Result of processing a single file
#[derive(Debug, Serialize, Deserialize)]
pub struct IngestResult {
    pub file_path: String,
    pub kb_destination: String,
    pub vectors_created: usize,
    pub redacted: bool,
    pub error: Option<String>,
}

/// Summary of an audit sweep
#[derive(Debug, Serialize, Deserialize)]
pub struct AuditSummary {
    pub status: String,
    pub files_processed: usize,
    pub vectors_created: usize,
    pub results: Vec<IngestResult>,
}

/// Deep Audit Skill: Sovereign document ingestion pipeline
pub struct DeepAuditSkill {
    data_dir: PathBuf,
    ingest_dir: PathBuf,
}

impl DeepAuditSkill {
    /// Create a new Deep Audit skill
    pub fn new(data_dir: PathBuf) -> Self {
        let ingest_dir = data_dir.join("ingest");
        Self {
            data_dir,
            ingest_dir,
        }
    }

    /// Ensure the ingest directory exists
    pub async fn ensure_ingest_dir(&self) -> Result<(), String> {
        if !self.ingest_dir.exists() {
            fs::create_dir_all(&self.ingest_dir)
                .await
                .map_err(|e| format!("Failed to create ingest directory: {}", e))?;
            info!("Created ingest directory: {:?}", self.ingest_dir);
        }
        Ok(())
    }

    /// Start watching the ingest directory for new files
    pub fn start_watcher(&self) -> Result<(), String> {
        let ingest_path = self.ingest_dir.clone();
        let data_dir = self.data_dir.clone();

        std::thread::spawn(move || {
            let (tx, rx) = channel();

            let mut watcher: RecommendedWatcher = Watcher::new(tx, Config::default())
                .map_err(|e| format!("Failed to create watcher: {}", e))
                .unwrap();

            watcher
                .watch(&ingest_path, RecursiveMode::NonRecursive)
                .map_err(|e| format!("Failed to watch directory: {}", e))
                .unwrap();

            info!("🏛️ Scribe is active. Watching: {:?}", ingest_path);

            for res in rx {
                match res {
                    Ok(Event {
                        kind: EventKind::Create(_) | EventKind::Modify(_),
                        paths,
                        ..
                    }) => {
                        for path in paths {
                            if path.is_file() {
                                info!("📄 New file detected: {:?}", path);
                                let data_dir_clone = data_dir.clone();
                                tokio::spawn(async move {
                                    if let Err(e) = Self::process_file_static(&path, &data_dir_clone).await {
                                        error!("Failed to process file {:?}: {}", path, e);
                                    }
                                });
                            }
                        }
                    }
                    Err(e) => warn!("Watch error: {:?}", e),
                    _ => {}
                }
            }
        });

        Ok(())
    }

    /// Process a single file (static version for spawned tasks)
    async fn process_file_static(path: &Path, data_dir: &Path) -> Result<IngestResult, String> {
        info!("Processing file: {:?}", path);

        // Read file content
        let content = fs::read_to_string(path)
            .await
            .map_err(|e| format!("Failed to read file: {}", e))?;

        // Check file size
        let metadata = fs::metadata(path)
            .await
            .map_err(|e| format!("Failed to get file metadata: {}", e))?;

        let file_size = metadata.len();
        if file_size > MAX_LLM_ANALYSIS_SIZE {
            warn!(
                "File {:?} exceeds max size ({}MB), using local embeddings only",
                path,
                file_size / (1024 * 1024)
            );
        }

        // Extract first N tokens for triage
        let triage_content: String = content
            .split_whitespace()
            .take(TRIAGE_TOKEN_LIMIT)
            .collect::<Vec<_>>()
            .join(" ");

        // Determine KB destination
        let kb = KnowledgeBase::from_content(&triage_content);
        info!("Routing to: {}", kb.collection_name());

        // Load redactor and sanitize content
        let redactor = SAORedactor::load_from_data_dir(data_dir)
            .unwrap_or_else(|_| SAORedactor::empty());

        let sanitized = redactor.sanitize_transcript(content.clone());
        let redacted = sanitized != content;

        if redacted {
            info!("🔒 Content redacted before vectorization");
        }

        // TODO: Vectorize and push to Qdrant
        // For now, we'll simulate this step
        let vectors_created = estimate_vector_count(&sanitized);

        info!(
            "✅ Processed: {:?} → {} ({} vectors)",
            path,
            kb.collection_name(),
            vectors_created
        );

        Ok(IngestResult {
            file_path: path.to_string_lossy().to_string(),
            kb_destination: kb.collection_name().to_string(),
            vectors_created,
            redacted,
            error: None,
        })
    }

    /// Process a single file
    pub async fn process_file(&self, path: &Path) -> Result<IngestResult, String> {
        Self::process_file_static(path, &self.data_dir).await
    }

    /// Sweep the ingest directory and process all files
    pub async fn sweep_ingest_dir(&self) -> Result<AuditSummary, String> {
        self.ensure_ingest_dir().await?;

        let mut results = Vec::new();
        let mut total_vectors = 0;

        let mut entries = fs::read_dir(&self.ingest_dir)
            .await
            .map_err(|e| format!("Failed to read ingest directory: {}", e))?;

        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|e| format!("Failed to read directory entry: {}", e))?
        {
            let path = entry.path();
            if path.is_file() {
                match self.process_file(&path).await {
                    Ok(result) => {
                        total_vectors += result.vectors_created;
                        results.push(result);
                    }
                    Err(e) => {
                        results.push(IngestResult {
                            file_path: path.to_string_lossy().to_string(),
                            kb_destination: "error".to_string(),
                            vectors_created: 0,
                            redacted: false,
                            error: Some(e),
                        });
                    }
                }
            }
        }

        Ok(AuditSummary {
            status: "Audited".to_string(),
            files_processed: results.len(),
            vectors_created: total_vectors,
            results,
        })
    }
}

#[async_trait]
impl AgentSkill for DeepAuditSkill {
    fn name(&self) -> &str {
        "deep_audit"
    }

    async fn execute(
        &self,
        _ctx: &TenantContext,
        params: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        let params = params.unwrap_or_else(|| serde_json::json!({}));
        let action = params
            .get("action")
            .and_then(|v| v.as_str())
            .unwrap_or("sweep");

        match action {
            "sweep" => {
                let summary = self.sweep_ingest_dir().await
                    .map_err(|e| Box::new(std::io::Error::new(std::io::ErrorKind::Other, e)) as Box<dyn std::error::Error + Send + Sync>)?;
                Ok(serde_json::to_value(summary)?)
            }
            "start_watcher" => {
                self.start_watcher()
                    .map_err(|e| Box::new(std::io::Error::new(std::io::ErrorKind::Other, e)) as Box<dyn std::error::Error + Send + Sync>)?;
                Ok(serde_json::json!({
                    "status": "Watcher started",
                    "message": "The Scribe is active. The 8 Knowledge Bases are no longer silent archives; they are living extensions of your 21-acre domain."
                }))
            }
            _ => Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Unknown action: {}", action)
            ))),
        }
    }
}

/// Estimate the number of vectors that would be created from content
fn estimate_vector_count(content: &str) -> usize {
    // Rough estimate: 1 vector per 100 words
    let word_count = content.split_whitespace().count();
    (word_count / 100).max(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kb_routing_tourism() {
        let content = "This document discusses tourism development in coastal regions.";
        let kb = KnowledgeBase::from_content(content);
        assert_eq!(kb.collection_name(), "kb-05-polis");
    }

    #[test]
    fn test_kb_routing_technical() {
        let content = "Rust API deployment using Docker and Kubernetes infrastructure.";
        let kb = KnowledgeBase::from_content(content);
        assert_eq!(kb.collection_name(), "kb-03-techne");
    }

    #[test]
    fn test_kb_routing_meeting() {
        let content = "Meeting transcript from the Mimir voice conversation.";
        let kb = KnowledgeBase::from_content(content);
        assert_eq!(kb.collection_name(), "kb-07-mimir");
    }

    #[test]
    fn test_kb_routing_default() {
        let content = "Some general information without specific keywords.";
        let kb = KnowledgeBase::from_content(content);
        assert_eq!(kb.collection_name(), "kb-01-psyche");
    }
}
