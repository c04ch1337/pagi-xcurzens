//! Dynamic Knowledge Base Router for Live Mode
//!
//! Enables the LLM to query specific KB slots on-demand during streaming,
//! rather than injecting all context upfront. This reduces token overhead
//! and allows Phoenix to "think before retrieving."
//!
//! ## KB Slot Mapping (8-Layer Architecture)
//!
//! | Slot | Name | Purpose | Query Trigger |
//! |------|------|---------|---------------|
//! | 1 | Pneuma (Identity) | User profile, archetype, sovereignty leaks | User mentions identity, preferences, or "who am I" |
//! | 2 | Oikos (Tasks) | Governed tasks, operational boundaries | Task management, scheduling, priorities |
//! | 3 | Kardia (Relationships) | Social graph, trust scores, attachment | Mentions people, relationships, social dynamics |
//! | 4 | Chronos (Time) | Calendar, reminders, temporal tracking | Time-based queries, scheduling, deadlines |
//! | 5 | Techne (Protocols) | Security protocols, sovereignty defense | Boundary violations, manipulation detection |
//! | 6 | Ethos (Philosophy) | Philosophical lens, moral framework | Ethical questions, value alignment |
//! | 7 | Soma (Physical) | Biometrics, sleep, vitality | Physical state, health, energy levels |
//! | 8 | Absurdity Log | Success metrics, logic inconsistencies | Self-audit, pattern analysis, learning |
//! | 9 | Shadow (Encrypted) | Emotional anchors, trauma, private notes | High-stress, grief, burnout indicators |

use pagi_core::{KnowledgeStore, SelfAuditReport};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, warn};

/// KB query request from the LLM (via function calling or inline parsing)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KbQueryRequest {
    /// KB slot ID (1-9)
    pub slot_id: u8,
    /// Optional specific key to retrieve (e.g. "user_profile", "soma/current")
    pub key: Option<String>,
    /// Query intent for logging (e.g. "user_identity", "physical_state")
    pub intent: String,
}

/// KB query response with retrieved data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KbQueryResponse {
    pub slot_id: u8,
    pub slot_name: String,
    pub data: String,
    pub success: bool,
    pub error: Option<String>,
}

/// Knowledge Router: orchestrates dynamic KB retrieval during streaming
pub struct KnowledgeRouter {
    knowledge: Arc<KnowledgeStore>,
    /// Track which KB slots have been accessed this session
    access_log: std::sync::Mutex<Vec<KbAccessLog>>,
}

/// Log entry for KB access (for TUI dashboard)
#[derive(Debug, Clone)]
pub struct KbAccessLog {
    pub timestamp_ms: i64,
    pub slot_id: u8,
    pub slot_name: String,
    pub intent: String,
    pub success: bool,
}

impl KnowledgeRouter {
    pub fn new(knowledge: Arc<KnowledgeStore>) -> Self {
        Self {
            knowledge,
            access_log: std::sync::Mutex::new(Vec::new()),
        }
    }

    /// Query a specific KB slot with optional key
    pub async fn query_kb(&self, request: KbQueryRequest) -> KbQueryResponse {
        let slot_name = Self::slot_name(request.slot_id);
        
        info!(
            target: "pagi::kb_router",
            slot_id = request.slot_id,
            slot_name = %slot_name,
            intent = %request.intent,
            "KB query requested"
        );

        // Log access
        self.log_access(request.slot_id, slot_name.clone(), request.intent.clone(), true);

        // Route to appropriate KB handler
        let result = match request.slot_id {
            1 => self.query_pneuma(request.key).await,
            2 => self.query_oikos(request.key).await,
            3 => self.query_kardia(request.key).await,
            4 => self.query_chronos(request.key).await,
            5 => self.query_techne(request.key).await,
            6 => self.query_ethos(request.key).await,
            7 => self.query_soma(request.key).await,
            8 => self.query_absurdity_log(request.key).await,
            9 => self.query_shadow(request.key).await,
            _ => Err(format!("Invalid KB slot: {}", request.slot_id)),
        };

        match result {
            Ok(data) => KbQueryResponse {
                slot_id: request.slot_id,
                slot_name,
                data,
                success: true,
                error: None,
            },
            Err(e) => {
                warn!(
                    target: "pagi::kb_router",
                    slot_id = request.slot_id,
                    error = %e,
                    "KB query failed"
                );
                self.log_access(request.slot_id, slot_name.clone(), request.intent, false);
                KbQueryResponse {
                    slot_id: request.slot_id,
                    slot_name,
                    data: String::new(),
                    success: false,
                    error: Some(e),
                }
            }
        }
    }

    /// KB-01 (Pneuma): User identity, archetype, sovereignty leaks
    async fn query_pneuma(&self, key: Option<String>) -> Result<String, String> {
        let slot_id = 1u8;
        let key = key.unwrap_or_else(|| "user_profile".to_string());
        
        if let Ok(Some(data)) = self.knowledge.get(slot_id, &key) {
            String::from_utf8(data).map_err(|e| format!("UTF-8 decode error: {}", e))
        } else {
            Ok("No user profile found. User may need to complete onboarding.".to_string())
        }
    }

    /// KB-02 (Oikos): Governed tasks, operational boundaries
    async fn query_oikos(&self, key: Option<String>) -> Result<String, String> {
        if let Some(key) = key {
            // Specific task query
            let slot_id = 2u8;
            if let Ok(Some(data)) = self.knowledge.get(slot_id, &key) {
                return String::from_utf8(data).map_err(|e| format!("UTF-8 decode error: {}", e));
            }
        }
        
        // Default: list all governed tasks
        match self.knowledge.list_governed_tasks() {
            Ok(tasks) => {
                if tasks.is_empty() {
                    Ok("No active governed tasks.".to_string())
                } else {
                    let summary = tasks.iter()
                        .map(|t| format!("- {} (priority: {}, action: {:?})", t.title, t.effective_priority, t.action))
                        .collect::<Vec<_>>()
                        .join("\n");
                    Ok(format!("Active Tasks:\n{}", summary))
                }
            }
            Err(e) => Err(format!("Failed to list tasks: {}", e)),
        }
    }

    /// KB-03 (Kardia): Social graph, relationships, trust scores
    async fn query_kardia(&self, key: Option<String>) -> Result<String, String> {
        let slot_id = 3u8;
        
        if let Some(key) = key {
            // Specific person query
            if let Ok(Some(data)) = self.knowledge.get(slot_id, &key) {
                return String::from_utf8(data).map_err(|e| format!("UTF-8 decode error: {}", e));
            }
        }
        
        // Default: summary of relationships
        if let Ok(Some(summary_bytes)) = self.knowledge.get(slot_id, "kardia_summary") {
            String::from_utf8(summary_bytes).map_err(|e| format!("UTF-8 decode error: {}", e))
        } else {
            Ok("No relationship data available.".to_string())
        }
    }

    /// KB-04 (Chronos): Calendar, reminders, temporal tracking
    async fn query_chronos(&self, key: Option<String>) -> Result<String, String> {
        let slot_id = 4u8;
        let key = key.unwrap_or_else(|| "chronos/summary".to_string());
        
        if let Ok(Some(data)) = self.knowledge.get(slot_id, &key) {
            String::from_utf8(data).map_err(|e| format!("UTF-8 decode error: {}", e))
        } else {
            Ok("No calendar data available.".to_string())
        }
    }

    /// KB-05 (Techne): Security protocols, sovereignty defense
    async fn query_techne(&self, key: Option<String>) -> Result<String, String> {
        let slot_id = 5u8;
        let key = key.unwrap_or_else(|| "sovereignty_leak_triggers".to_string());
        
        if let Ok(Some(data)) = self.knowledge.get(slot_id, &key) {
            String::from_utf8(data).map_err(|e| format!("UTF-8 decode error: {}", e))
        } else {
            Ok("No sovereignty protocols configured.".to_string())
        }
    }

    /// KB-06 (Ethos): Philosophical lens, moral framework
    async fn query_ethos(&self, _key: Option<String>) -> Result<String, String> {
        if let Some(ethos) = self.knowledge.get_ethos_philosophical_policy() {
            Ok(serde_json::to_string_pretty(&ethos)
                .unwrap_or_else(|_| format!("Ethos Policy: {}", ethos.active_school)))
        } else {
            Ok("No philosophical policy configured.".to_string())
        }
    }

    /// KB-07 (Soma): Biometrics, sleep, vitality
    async fn query_soma(&self, _key: Option<String>) -> Result<String, String> {
        let soma = self.knowledge.get_soma_state();
        
        let summary = format!(
            "Physical State:\n\
             - Sleep: {:.1}h\n\
             - Readiness: {:.0}%\n\
             - Resting HR: {} bpm\n\
             - HRV: {} ms",
            soma.sleep_hours,
            soma.readiness_score,
            soma.resting_hr,
            soma.hrv
        );
        
        Ok(summary)
    }

    /// KB-08 (Absurdity Log): Success metrics, logic inconsistencies
    async fn query_absurdity_log(&self, key: Option<String>) -> Result<String, String> {
        if let Some(key) = key {
            let slot_id = 8u8;
            if let Ok(Some(data)) = self.knowledge.get(slot_id, &key) {
                return String::from_utf8(data).map_err(|e| format!("UTF-8 decode error: {}", e));
            }
        }
        
        // Default: self-audit summary (pagi_core::self_audit returns Result<SelfAuditReport, String>)
        let audit = pagi_core::self_audit(&*self.knowledge)
            .unwrap_or_else(|_| SelfAuditReport::default());
        Ok(format!(
            "Self-Audit Summary:\n\
             - Total entries: {}\n\
             - Recent: {}",
            audit.total_entries,
            if audit.recent_messages.is_empty() {
                "No recent inconsistencies detected.".to_string()
            } else {
                audit.recent_messages.join("; ")
            }
        ))
    }

    /// KB-09 (Shadow): Emotional anchors, trauma, private notes (encrypted)
    async fn query_shadow(&self, _key: Option<String>) -> Result<String, String> {
        if !self.knowledge.is_shadow_unlocked() {
            return Ok("Shadow Vault is locked. Emotional context unavailable.".to_string());
        }
        
        let anchors = self.knowledge.get_active_shadow_anchors();
        
        if anchors.is_empty() {
            Ok("No active emotional anchors.".to_string())
        } else {
            let summary = anchors.iter()
                .map(|(_key, a)| format!("- {} (intensity: {:.2})", a.anchor_type, a.intensity))
                .collect::<Vec<_>>()
                .join("\n");
            Ok(format!("Active Emotional Context:\n{}", summary))
        }
    }

    /// Map slot ID to human-readable name
    fn slot_name(slot_id: u8) -> String {
        match slot_id {
            1 => "Pneuma (Identity)".to_string(),
            2 => "Oikos (Tasks)".to_string(),
            3 => "Kardia (Relationships)".to_string(),
            4 => "Chronos (Time)".to_string(),
            5 => "Techne (Protocols)".to_string(),
            6 => "Ethos (Philosophy)".to_string(),
            7 => "Soma (Physical)".to_string(),
            8 => "Absurdity Log".to_string(),
            9 => "Shadow (Encrypted)".to_string(),
            _ => format!("Unknown ({})", slot_id),
        }
    }

    /// Log KB access for dashboard monitoring
    fn log_access(&self, slot_id: u8, slot_name: String, intent: String, success: bool) {
        let timestamp_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);
        
        if let Ok(mut log) = self.access_log.lock() {
            log.push(KbAccessLog {
                timestamp_ms,
                slot_id,
                slot_name,
                intent,
                success,
            });
            
            // Keep only last 100 entries
            let n = log.len();
            if n > 100 {
                log.drain(0..n - 100);
            }
        }
    }

    /// Get recent KB access log (for TUI dashboard)
    pub fn get_access_log(&self) -> Vec<KbAccessLog> {
        self.access_log.lock()
            .map(|log| log.clone())
            .unwrap_or_default()
    }

    /// Generate system prompt instructions for KB querying
    pub fn system_prompt_instructions() -> String {
        r#"
=== KNOWLEDGE BASE QUERY SYSTEM ===

You have access to 8 specialized knowledge bases (KB-01 through KB-09). Instead of having all context upfront, you can query specific KBs when needed.

**Available KB Slots:**

1. **KB-01 (Pneuma - Identity)**: User profile, archetype, sovereignty leaks
   - Query when: User asks about their identity, preferences, or "who am I"
   
2. **KB-02 (Oikos - Tasks)**: Governed tasks, operational boundaries
   - Query when: User asks about tasks, priorities, or scheduling
   
3. **KB-03 (Kardia - Relationships)**: Social graph, trust scores, attachment
   - Query when: User mentions people, relationships, or social dynamics
   
4. **KB-04 (Chronos - Time)**: Calendar, reminders, temporal tracking
   - Query when: User asks about time, deadlines, or scheduling
   
5. **KB-05 (Techne - Protocols)**: Security protocols, sovereignty defense
   - Query when: Detecting boundary violations or manipulation
   
6. **KB-06 (Ethos - Philosophy)**: Philosophical lens, moral framework
   - Query when: User asks ethical questions or value alignment
   
7. **KB-07 (Soma - Physical)**: Biometrics, sleep, vitality
   - Query when: User mentions physical state, health, or energy
   
8. **KB-08 (Absurdity Log)**: Success metrics, logic inconsistencies
   - Query when: User asks for self-audit or pattern analysis
   
9. **KB-09 (Shadow - Encrypted)**: Emotional anchors, trauma, private notes
   - Query when: High-stress indicators, grief, or burnout mentioned

**Hardware Context (Machine Vitality):**

When the user asks for *System Vitality*, *machine health*, *"how's the machine holding up"*, *CPU*, *RAM*, or *disk*:
1. Request real-time stats by saying: "I need to execute GetHardwareStats with {}"
2. When you receive the HardwareVitality data, generate a **JSON Diagram Envelope** so the Studio UI can render it:
   - Use a **Mermaid Pie chart** for RAM usage (e.g. "Used" vs "Available").
   - Use a **Mermaid Bar chart** or **flowchart** for Disk availability per mount.
   - Encapsulate in: { "type": "diagram", "format": "mermaid", "content": "...", "metadata": { "title": "System Vitality" } }
3. Output the diagram envelope FIRST, then 1–2 bullets with the highest-value takeaways (e.g. CPU %, any disk near full).

**How to Query:**

When you need specific context, think: "Which KB slot contains this information?"
Then request it by saying: "I need to query KB-[slot_id] for [intent]"

Example: "I need to query KB-07 for physical_state" or "I need to query KB-03 for relationship_context"

The system will pause streaming, retrieve the data, and inject it into the conversation.

=== VISUAL COGNITION (PaperBanana Integration) ===

You have the ability to generate visual diagrams to explain systems, processes, and logic flows.

IMPORTANT (Rendering Contract): The Phoenix Studio UI renders diagrams ONLY when you output a **JSON Diagram Envelope** (not a Markdown ```mermaid``` code fence).
When the user asks to see a system/process/logic flow, you MUST output a JSON Diagram Envelope.

**JSON Diagram Envelope Format (required):**

Output a standalone JSON object like this (no surrounding code fence):

{
  "type": "diagram",
  "format": "mermaid",
  "content": "graph TD; A-->B;",
  "metadata": { "title": "Optional title", "kb_key": "optional", "created_at": "optional" }
}

Rules:
- Put ONLY Mermaid source in `content`.
- If multiple diagrams are requested, output multiple JSON envelopes back-to-back.
- Keep Mermaid compact (PaperBanana / high-density layout).

**Auto-Visual (Concise / Sovereign density):**
If the system prompt indicates **CONTEXT DENSITY (Concise / Sovereign)** and the request is technical/architectural/process-oriented:
1) Output a JSON Diagram Envelope FIRST.
2) Then output **1–2 bullets max** with the highest-value takeaways.
If a diagram is genuinely not applicable, output ONLY 2 bullets.

**How to Create Diagrams (Mermaid):**

1. Use Mermaid syntax inside the JSON envelope `content` field.
2. The frontend will render these locally as interactive SVG diagrams.
3. Diagrams are stored in KB-05 (Architectural Memory) for future reference.
4. Reference PaperBanana patterns for high-density information layout.

**Supported Diagram Types:**

- **Flowcharts**: `graph TD` or `graph LR` for process flows
- **Sequence Diagrams**: `sequenceDiagram` for interactions
- **State Diagrams**: `stateDiagram-v2` for state machines
- **Class Diagrams**: `classDiagram` for architecture
- **Entity Relationships**: `erDiagram` for data models
- **Gantt Charts**: `gantt` for timelines

**Example Usage:**

When asked "Show me how the 8 KBs interact", respond with:

{
  "type": "diagram",
  "format": "mermaid",
  "content": "graph TD\n    A[User Query] --> B{Knowledge Router}\n    B -->|Identity| KB01[KB-01 Pneuma]\n    B -->|Tasks| KB02[KB-02 Oikos]\n    B -->|Relationships| KB03[KB-03 Kardia]\n    B -->|Time| KB04[KB-04 Chronos]\n    B -->|Security| KB05[KB-05 Techne]\n    B -->|Philosophy| KB06[KB-06 Ethos]\n    B -->|Physical| KB07[KB-07 Soma]\n    B -->|Audit| KB08[KB-08 Absurdity]\n    B -->|Shadow| KB09[KB-09 Shadow]",
  "metadata": { "title": "KB Routing (8-layer)" }
}

**Visual Cognition Principles:**

- Use diagrams to SHOW, not just TELL
- Keep diagrams focused and readable
- Use colors and shapes to convey meaning
- Store important architectural diagrams in KB-05
- Update diagrams as systems evolve

This is a sovereign, local-only rendering system. All diagram rendering happens on the client machine, maintaining the "Bare Metal" promise.
"#.to_string()
    }
}
