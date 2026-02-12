//! Conversation Topic Indexer: Autonomous Memory Evolution (KB-04 Optimization)
//!
//! **Mission**: Solve the linear scan bottleneck in `get_recent_conversation()` by creating
//! a topic-based index that allows Phoenix to retrieve conversations by semantic topic
//! (e.g., "All conversations about 'The Forge'") without scanning the entire history.
//!
//! **Design Philosophy**:
//! - Every N conversation exchanges (default: 10), summarize the topic using the LLM bridge
//! - Store the topic summary in a sub-index within KB-04 (Chronos) under `topic_index/{agent_id}/{topic_id}`
//! - Enable fast retrieval by topic keyword without full history scan
//!
//! **Autonomous Safety**:
//! - Read-only diagnostic mode: Analyzes conversation patterns without modification
//! - Write mode: Creates topic index entries (requires Ethos alignment check)
//! - Logs all operations to KB-08 (Soma) for sovereign oversight

use pagi_core::{AgentSkill, EventRecord, KbType, KnowledgeStore, TenantContext};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

const SKILL_NAME: &str = "conversation_topic_indexer";
const TOPIC_INDEX_PREFIX: &str = "topic_index/";
const DEFAULT_BATCH_SIZE: usize = 10;

#[derive(Debug, Deserialize)]
struct TopicIndexerArgs {
    /// Agent ID to index (defaults to context agent)
    #[serde(default)]
    agent_id: Option<String>,
    
    /// Number of conversation exchanges to batch before summarizing (default: 10)
    #[serde(default = "default_batch_size")]
    batch_size: usize,
    
    /// Mode: "diagnostic" (read-only analysis) or "index" (create topic entries)
    #[serde(default = "default_mode")]
    mode: String,
    
    /// Optional: specific topic to search for in diagnostic mode
    #[serde(default)]
    search_topic: Option<String>,
}

fn default_batch_size() -> usize {
    DEFAULT_BATCH_SIZE
}

fn default_mode() -> String {
    "diagnostic".to_string()
}

/// Topic summary record stored in KB-04 under `topic_index/{agent_id}/{topic_id}`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopicSummary {
    /// Unique identifier for this topic cluster
    pub topic_id: String,
    /// LLM-generated topic summary (e.g., "Discussion about The Forge safety mechanisms")
    pub topic: String,
    /// Conversation key range covered by this topic (start key)
    pub conversation_start_key: String,
    /// Conversation key range covered by this topic (end key)
    pub conversation_end_key: String,
    /// Number of exchanges in this topic cluster
    pub exchange_count: usize,
    /// Unix timestamp (ms) when this topic was indexed
    pub indexed_at_ms: i64,
}

impl TopicSummary {
    pub fn to_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(self).unwrap_or_default()
    }
    
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        serde_json::from_slice(bytes).ok()
    }
}

/// Conversation Topic Indexer: Autonomous memory optimization skill
pub struct ConversationTopicIndexer {
    store: Arc<KnowledgeStore>,
    model_router: Option<Arc<crate::ModelRouter>>,
}

impl ConversationTopicIndexer {
    pub fn new(store: Arc<KnowledgeStore>) -> Self {
        Self {
            store,
            model_router: None,
        }
    }
    
    pub fn with_model_router(store: Arc<KnowledgeStore>, router: Arc<crate::ModelRouter>) -> Self {
        Self {
            store,
            model_router: Some(router),
        }
    }
    
    /// Diagnostic mode: Analyze conversation history and identify indexing opportunities
    async fn run_diagnostic(
        &self,
        agent_id: &str,
        batch_size: usize,
        search_topic: Option<String>,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        let slot_id = KbType::Chronos.slot_id();
        let prefix = format!("conversation/{}/", agent_id);
        
        // Scan all conversation keys for this agent
        let keys = self.store.scan_keys(slot_id)?;
        let conversation_keys: Vec<String> = keys
            .into_iter()
            .filter(|k| k.starts_with(&prefix))
            .collect();
        
        let total_exchanges = conversation_keys.len();
        let potential_topics = (total_exchanges + batch_size - 1) / batch_size;
        
        // Check existing topic index
        let topic_keys: Vec<String> = self.store.scan_keys(slot_id)?
            .into_iter()
            .filter(|k| k.starts_with(&format!("{}{}/", TOPIC_INDEX_PREFIX, agent_id)))
            .collect();
        
        let indexed_topics = topic_keys.len();
        
        // If search_topic is provided, scan existing topics for matches
        let mut matching_topics = Vec::new();
        let search_query = search_topic.clone();
        if let Some(ref search_term) = search_topic {
            for topic_key in &topic_keys {
                if let Ok(Some(bytes)) = self.store.get(slot_id, topic_key) {
                    if let Some(summary) = TopicSummary::from_bytes(&bytes) {
                        if summary.topic.to_lowercase().contains(&search_term.to_lowercase()) {
                            matching_topics.push(serde_json::json!({
                                "topic_id": summary.topic_id,
                                "topic": summary.topic,
                                "exchange_count": summary.exchange_count,
                                "conversation_range": format!("{} → {}",
                                    summary.conversation_start_key,
                                    summary.conversation_end_key),
                            }));
                        }
                    }
                }
            }
        }
        
        Ok(serde_json::json!({
            "status": "diagnostic_complete",
            "skill": SKILL_NAME,
            "agent_id": agent_id,
            "analysis": {
                "total_conversation_exchanges": total_exchanges,
                "potential_topic_clusters": potential_topics,
                "indexed_topics": indexed_topics,
                "indexing_coverage": if potential_topics > 0 {
                    format!("{:.1}%", (indexed_topics as f64 / potential_topics as f64) * 100.0)
                } else {
                    "0%".to_string()
                },
                "batch_size": batch_size,
            },
            "search_results": if !matching_topics.is_empty() {
                Some(serde_json::json!({
                    "query": search_query,
                    "matches": matching_topics,
                }))
            } else {
                None
            },
            "recommendation": if indexed_topics < potential_topics {
                format!("Run in 'index' mode to create {} new topic summaries", 
                    potential_topics - indexed_topics)
            } else {
                "Topic index is up to date".to_string()
            },
        }))
    }
    
    /// Index mode: Create topic summaries for unindexed conversation batches
    async fn run_indexing(
        &self,
        agent_id: &str,
        batch_size: usize,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        // Require model router for LLM-based topic summarization
        let router = self.model_router.as_ref()
            .ok_or("Model router required for indexing mode")?;
        
        let slot_id = KbType::Chronos.slot_id();
        let prefix = format!("conversation/{}/", agent_id);
        
        // Get all conversation keys, sorted
        let mut conversation_keys: Vec<String> = self.store.scan_keys(slot_id)?
            .into_iter()
            .filter(|k| k.starts_with(&prefix))
            .collect();
        conversation_keys.sort();
        
        // Get existing topic index to avoid re-indexing
        let existing_topics: Vec<String> = self.store.scan_keys(slot_id)?
            .into_iter()
            .filter(|k| k.starts_with(&format!("{}{}/", TOPIC_INDEX_PREFIX, agent_id)))
            .collect();
        
        let mut indexed_count = 0;
        let mut topics_created = Vec::new();
        
        // Process conversations in batches
        for (batch_idx, chunk) in conversation_keys.chunks(batch_size).enumerate() {
            if chunk.is_empty() {
                continue;
            }
            
            let topic_id = format!("topic_{:04}", batch_idx);
            let topic_key = format!("{}{}/{}", TOPIC_INDEX_PREFIX, agent_id, topic_id);
            
            // Skip if already indexed
            if existing_topics.contains(&topic_key) {
                continue;
            }
            
            // Gather conversation content for this batch
            let mut batch_content = Vec::new();
            for conv_key in chunk {
                if let Ok(Some(bytes)) = self.store.get(slot_id, conv_key) {
                    if let Ok(text) = String::from_utf8(bytes) {
                        batch_content.push(text);
                    }
                }
            }
            
            if batch_content.is_empty() {
                continue;
            }
            
            // Generate topic summary using LLM
            let summarization_prompt = format!(
                "Analyze the following conversation exchanges and provide a concise topic summary (max 100 chars):\n\n{}\n\nTopic:",
                batch_content.join("\n---\n")
            );
            
            let topic_summary = router.generate_text_raw(&summarization_prompt)
                .await
                .unwrap_or_else(|_| format!("Conversation batch {}", batch_idx));
            
            // Create topic summary record
            let summary = TopicSummary {
                topic_id: topic_id.clone(),
                topic: topic_summary.trim().to_string(),
                conversation_start_key: chunk.first().unwrap().clone(),
                conversation_end_key: chunk.last().unwrap().clone(),
                exchange_count: chunk.len(),
                indexed_at_ms: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_millis() as i64)
                    .unwrap_or(0),
            };
            
            // Store in KB-04 topic index
            self.store.insert(slot_id, &topic_key, &summary.to_bytes())?;
            
            topics_created.push(serde_json::json!({
                "topic_id": summary.topic_id,
                "topic": summary.topic,
                "exchange_count": summary.exchange_count,
            }));
            
            indexed_count += 1;
        }
        
        // Log success to KB-08 (Soma)
        let reflection = EventRecord::now(
            "Chronos",
            format!("Topic indexer created {} new topic summaries for agent {}", 
                indexed_count, agent_id),
        )
        .with_skill(SKILL_NAME)
        .with_outcome(format!("indexed_{}_topics", indexed_count));
        
        self.store.append_chronos_event(agent_id, &reflection)?;
        
        Ok(serde_json::json!({
            "status": "indexing_complete",
            "skill": SKILL_NAME,
            "agent_id": agent_id,
            "topics_created": indexed_count,
            "topics": topics_created,
            "message": format!("Successfully indexed {} conversation topics", indexed_count),
        }))
    }
}

#[async_trait::async_trait]
impl AgentSkill for ConversationTopicIndexer {
    fn name(&self) -> &str {
        SKILL_NAME
    }
    
    async fn execute(
        &self,
        ctx: &TenantContext,
        payload: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        let args: TopicIndexerArgs = payload
            .and_then(|v| serde_json::from_value(v).ok())
            .unwrap_or(TopicIndexerArgs {
                agent_id: None,
                batch_size: DEFAULT_BATCH_SIZE,
                mode: "diagnostic".to_string(),
                search_topic: None,
            });
        
        let agent_id = args.agent_id
            .as_deref()
            .unwrap_or_else(|| ctx.resolved_agent_id());
        
        let batch_size = args.batch_size.max(5).min(50); // Safety bounds
        
        match args.mode.as_str() {
            "diagnostic" => {
                self.run_diagnostic(agent_id, batch_size, args.search_topic).await
            }
            "index" => {
                // Check Ethos alignment before modifying KB-04
                if let Some(policy) = self.store.get_ethos_policy() {
                    let alignment = policy.allows(SKILL_NAME, "topic_index");
                    if let pagi_core::AlignmentResult::Fail { reason } = alignment {
                        return Ok(serde_json::json!({
                            "status": "blocked_by_ethos",
                            "skill": SKILL_NAME,
                            "reason": reason,
                        }));
                    }
                }
                
                self.run_indexing(agent_id, batch_size).await
            }
            _ => {
                Err(format!("Invalid mode '{}'. Use 'diagnostic' or 'index'", args.mode).into())
            }
        }
    }
}
