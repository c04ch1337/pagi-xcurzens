//! OpenRouter Live Mode — Gemini Live-style experience using OpenRouter's streaming APIs
//! Combines: STT (Whisper) → Streaming Chat → TTS → Interruption handling
//! Now with Dynamic KB Selection for on-demand context retrieval

use pagi_voice::{
    AudioTurn, EarConfig, OpenRouterStt, OpenRouterTts, SttBackend, TtsBackend,
    VadState, VoiceEar, VoiceOutput,
};
use pagi_core::{
    KnowledgeStore, MemoryManager, SkillRegistry, KbType, LiveSkillRegistry,
    LiveSkill, SkillExecutionRequest, SkillExecutionResult, SkillPriority, TenantContext,
};
use crate::knowledge_router::{KnowledgeRouter, KbQueryRequest};
use futures_util::StreamExt;
use std::sync::Arc;
use std::collections::VecDeque;
use tracing::{info, warn};
use regex::Regex;

/// Simple message for conversation history
#[derive(Clone)]
struct Message {
    role: String,
    content: String,
}

/// OpenRouter Live session state
pub struct OpenRouterLiveSession {
    stt: OpenRouterStt,
    tts: OpenRouterTts,
    voice_output: Arc<VoiceOutput>,
    knowledge: Arc<KnowledgeStore>,
    kb_router: Arc<KnowledgeRouter>,
    live_skills: Arc<LiveSkillRegistry>,
    _skills: Arc<SkillRegistry>,
    _memory: Arc<MemoryManager>,
    /// Sentence buffer for streaming TTS (speak on sentence boundaries)
    sentence_buffer: String,
    /// Conversation history (in-memory for live session)
    history: VecDeque<Message>,
    /// Regex for detecting KB query requests in LLM output
    kb_query_regex: Regex,
    /// Regex for detecting skill execution requests
    skill_exec_regex: Regex,
    /// Tenant context for skill execution
    tenant_ctx: TenantContext,
}

impl OpenRouterLiveSession {
    /// Create new session from environment variables
    pub fn from_env(
        knowledge: Arc<KnowledgeStore>,
        skills: Arc<SkillRegistry>,
        memory: Arc<MemoryManager>,
    ) -> Result<Self, String> {
        let stt = OpenRouterStt::from_env()
            .map_err(|e| format!("STT init failed: {}", e))?;
        let tts = OpenRouterTts::from_env()
            .map_err(|e| format!("TTS init failed: {}", e))?;
        let voice_output = Arc::new(VoiceOutput::new()
            .map_err(|e| format!("VoiceOutput init failed: {}", e))?);
        
        let kb_router = Arc::new(KnowledgeRouter::new(knowledge.clone()));
        let live_skills = Arc::new(LiveSkillRegistry::default());
        
        // Regex to detect KB query requests: "I need to query KB-[1-9] for [intent]"
        let kb_query_regex = Regex::new(r"I need to query KB-(\d+) for (\w+)")
            .map_err(|e| format!("Regex compile failed: {}", e))?;
        
        // Regex to detect skill execution: "I need to execute [SKILL_NAME] with [PARAMS]"
        let skill_exec_regex = Regex::new(r"I need to execute (\w+) with (.+)")
            .map_err(|e| format!("Skill regex compile failed: {}", e))?;
        
        // Create tenant context
        let tenant_ctx = TenantContext {
            agent_id: "phoenix".to_string(),
            tenant_id: "default".to_string(),
        };
        
        Ok(Self {
            stt,
            tts,
            voice_output,
            knowledge,
            kb_router,
            live_skills,
            _skills: skills,
            _memory: memory,
            sentence_buffer: String::new(),
            history: VecDeque::with_capacity(20),
            kb_query_regex,
            skill_exec_regex,
            tenant_ctx,
        })
    }
    
    /// Start the live loop: Ear → STT → Streaming Chat → TTS → Playback
    pub async fn run(mut self) -> Result<(), String> {
        info!("🎙️ Starting OpenRouter Live Mode");
        
        // Start VoiceEar
        let ear = VoiceEar::new(EarConfig::default())
            .start_listening()
            .map_err(|e| format!("Ear start failed: {}", e))?;
        
        let mut handle = ear.handle;
        let mut vad_state_rx = ear.vad_state_rx;
        
        loop {
            enum Event {
                Turn(AudioTurn),
                VadState(VadState),
                Closed,
            }
            
            let event = if let Some(ref mut rx) = vad_state_rx {
                tokio::select! {
                    turn = handle.recv_turn() => match turn {
                        Some(t) => Event::Turn(t),
                        None => Event::Closed,
                    },
                    state = rx.recv() => match state {
                        Some(s) => Event::VadState(s),
                        None => continue,
                    },
                }
            } else {
                match handle.recv_turn().await {
                    Some(t) => Event::Turn(t),
                    None => Event::Closed,
                }
            };
            
            match event {
                Event::VadState(VadState::Speech) => {
                    // User interrupted — stop playback immediately
                    if self.voice_output.is_playing() {
                        info!("🛑 User interrupted — stopping playback");
                        self.voice_output.stop();
                        self.sentence_buffer.clear();
                    }
                }
                Event::VadState(_) => {}
                Event::Turn(turn) => {
                    // Transcribe audio turn
                    let text = match self.stt.transcribe_turn(&turn) {
                        Ok(t) => t,
                        Err(e) => {
                            warn!("STT failed: {}", e);
                            continue;
                        }
                    };
                    
                    if text.trim().is_empty() {
                        continue;
                    }
                    
                    info!("👤 User: {}", text);
                    
                    // Process with streaming chat
                    if let Err(e) = self.process_streaming_chat(&text).await {
                        warn!("Chat processing failed: {}", e);
                    }
                }
                Event::Closed => break,
            }
        }
        
        Ok(())
    }
    
    /// Process user input with OpenRouter streaming chat
    async fn process_streaming_chat(&mut self, user_text: &str) -> Result<(), String> {
        // Build system instruction with KB query instructions
        let system_instruction = self.build_system_instruction_with_kb_router().await?;
        
        // Build OpenRouter streaming request
        let client = reqwest::Client::new();
        let api_key = pagi_skills::resolve_api_key_from_vault_or_env("OPENROUTER_API_KEY")
            .or_else(|| pagi_skills::resolve_api_key_from_vault_or_env("PAGI_LLM_API_KEY"))
            .ok_or("OPENROUTER_API_KEY not set (check .env or vault: POST /api/v1/config/vault/set)")?;
        
        let model = std::env::var("PAGI_LLM_MODEL")
            .unwrap_or_else(|_| "anthropic/claude-3.5-sonnet".to_string());
        
        let mut messages = vec![
            serde_json::json!({
                "role": "system",
                "content": system_instruction
            })
        ];
        
        // Add history
        for msg in &self.history {
            messages.push(serde_json::json!({
                "role": msg.role,
                "content": msg.content
            }));
        }
        
        // Add current user message
        messages.push(serde_json::json!({
            "role": "user",
            "content": user_text
        }));
        
        let body = serde_json::json!({
            "model": model,
            "messages": messages,
            "stream": true,
            "temperature": 0.7,
        });
        
        let response = client
            .post("https://openrouter.ai/api/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", api_key))
            .header("HTTP-Referer", "https://pagi.local")
            .header("X-Title", "PAGI Live Mode")
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;
        
        if !response.status().is_success() {
            return Err(format!("API error: {}", response.status()));
        }
        
        // Process SSE stream
        let mut stream = response.bytes_stream();
        let mut full_response = String::new();
        
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| format!("Stream error: {}", e))?;
            let text = String::from_utf8_lossy(&chunk);
            
            // Parse SSE format: "data: {...}\n\n"
            for line in text.lines() {
                if line.starts_with("data: ") {
                    let json_str = &line[6..];
                    if json_str == "[DONE]" {
                        break;
                    }
                    
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(json_str) {
                        if let Some(delta) = json["choices"][0]["delta"]["content"].as_str() {
                            full_response.push_str(delta);
                            self.sentence_buffer.push_str(delta);
                            
                            // Check for KB query request
                            if let Some(kb_request) = self.detect_kb_query(&self.sentence_buffer) {
                                info!("🔍 KB query detected: slot={}, intent={}", kb_request.slot_id, kb_request.intent);
                                
                                // Pause streaming, query KB, inject result
                                let kb_response = self.kb_router.query_kb(kb_request).await;
                                
                                if kb_response.success {
                                    // Clear buffer and inject KB data
                                    self.sentence_buffer.clear();
                                    full_response.push_str(&format!("\n[KB-{} Retrieved: {}]\n", kb_response.slot_id, kb_response.data));
                                    
                                    info!("✓ KB-{} data injected into stream", kb_response.slot_id);
                                } else {
                                    warn!("✗ KB-{} query failed: {:?}", kb_response.slot_id, kb_response.error);
                                }
                                
                                continue;
                            }
                            
                            // Check for skill execution request
                            if let Some(skill_request) = self.detect_skill_execution(&self.sentence_buffer) {
                                info!("⚡ Skill execution detected: {}", skill_request.skill_name);
                                
                                // Execute skill with KB-05 security validation
                                match self.execute_skill_with_validation(skill_request).await {
                                    Ok(result) => {
                                        self.sentence_buffer.clear();
                                        full_response.push_str(&format!(
                                            "\n[Skill '{}' executed: {}]\n",
                                            result.skill_name,
                                            if result.success { "✓ Success" } else { "✗ Failed" }
                                        ));
                                        
                                        info!("✓ Skill '{}' completed in {}ms", result.skill_name, result.duration_ms);
                                    }
                                    Err(e) => {
                                        warn!("✗ Skill execution failed: {}", e);
                                        full_response.push_str(&format!("\n[Skill execution blocked: {}]\n", e));
                                    }
                                }
                                
                                continue;
                            }
                            
                            // Check for sentence boundary (., !, ?)
                            if self.sentence_buffer.contains('.')
                                || self.sentence_buffer.contains('!')
                                || self.sentence_buffer.contains('?') {
                                
                                // Extract complete sentence
                                if let Some(sentence) = self.extract_sentence() {
                                    // Speak immediately (non-blocking)
                                    self.speak_async(sentence)?;
                                }
                            }
                        }
                    }
                }
            }
        }
        
        // Speak any remaining text
        if !self.sentence_buffer.trim().is_empty() {
            let remaining = self.sentence_buffer.clone();
            self.sentence_buffer.clear();
            self.speak_async(remaining)?;
        }
        
        info!("🤖 Phoenix: {}", full_response);
        
        // Store in history
        self.history.push_back(Message {
            role: "user".to_string(),
            content: user_text.to_string(),
        });
        self.history.push_back(Message {
            role: "assistant".to_string(),
            content: full_response,
        });
        
        // Keep history limited to last 20 messages
        while self.history.len() > 20 {
            self.history.pop_front();
        }
        
        Ok(())
    }
    
    /// Extract complete sentence from buffer
    fn extract_sentence(&mut self) -> Option<String> {
        let delimiters = ['.', '!', '?'];
        
        for delimiter in delimiters {
            if let Some(pos) = self.sentence_buffer.find(delimiter) {
                let sentence = self.sentence_buffer[..=pos].to_string();
                self.sentence_buffer = self.sentence_buffer[pos+1..].to_string();
                return Some(sentence);
            }
        }
        
        None
    }
    
    /// Speak text synchronously (blocking TTS, non-blocking playback)
    fn speak_async(&self, text: String) -> Result<(), String> {
        // TTS is blocking, so run it synchronously
        match self.tts.synthesize(&text) {
            Ok(audio_bytes) => {
                if let Err(e) = self.voice_output.play_bytes(&audio_bytes) {
                    warn!("Playback failed: {}", e);
                }
            }
            Err(e) => {
                warn!("TTS failed: {}", e);
            }
        }
        
        Ok(())
    }
    
    /// Detect KB query request in LLM output
    fn detect_kb_query(&self, text: &str) -> Option<KbQueryRequest> {
        if let Some(captures) = self.kb_query_regex.captures(text) {
            let slot_id = captures.get(1)?.as_str().parse::<u8>().ok()?;
            let intent = captures.get(2)?.as_str().to_string();
            
            if slot_id >= 1 && slot_id <= 9 {
                return Some(KbQueryRequest {
                    slot_id,
                    key: None,
                    intent,
                });
            }
        }
        None
    }
    
    /// Build system instruction with KB router instructions
    async fn build_system_instruction_with_kb_router(&self) -> Result<String, String> {
        let mut parts = Vec::new();
        
        // Add KB query system instructions
        parts.push(KnowledgeRouter::system_prompt_instructions());
        
        // Add base context from KB-01, KB-03, KB-09 (lightweight)
        parts.push(self.build_base_context().await?);
        
        Ok(parts.join("\n\n"))
    }
    
    /// Build base context from KB-01, KB-03, KB-09 (lightweight initial context)
    async fn build_base_context(&self) -> Result<String, String> {
        let mut parts = Vec::new();
        
        // KB-01: User Identity (Slot 1)
        let kb1_slot = 1u8;
        if let Ok(Some(identity_bytes)) = self.knowledge.get(kb1_slot, "user_profile") {
            if let Ok(identity_str) = String::from_utf8(identity_bytes) {
                parts.push(format!("User Profile: {}", identity_str));
            }
        }
        
        // KB-03: Kardia (Slot 3)
        let kb3_slot = 3u8;
        if let Ok(Some(kardia_bytes)) = self.knowledge.get(kb3_slot, "kardia_summary") {
            if let Ok(kardia_str) = String::from_utf8(kardia_bytes) {
                parts.push(format!("Active Relationships: {}", kardia_str));
            }
        }
        
        // KB-09: Shadow Vault (emotional tone)
        let anchors = self.knowledge.get_active_shadow_anchors();
        if !anchors.is_empty() {
            let shadow_summary = anchors.iter()
                .map(|(_key, a)| format!("{} (intensity: {:.2})", a.anchor_type, a.intensity))
                .collect::<Vec<_>>()
                .join(", ");
            parts.push(format!("Current Emotional Context: {}", shadow_summary));
        }
        
        // Default system instruction if no context available
        if parts.is_empty() {
            parts.push("You are Phoenix, a sovereign AI assistant with deep emotional intelligence.".to_string());
        }
        
        if parts.is_empty() {
            parts.push("You are Phoenix, a sovereign AI assistant with deep emotional intelligence.".to_string());
        }
        
        Ok(parts.join("\n\n"))
    }
    
    /// Get KB access log for dashboard
    pub fn get_kb_access_log(&self) -> Vec<crate::knowledge_router::KbAccessLog> {
        self.kb_router.get_access_log()
    }
    
    /// Detect skill execution request in LLM output
    fn detect_skill_execution(&self, text: &str) -> Option<SkillExecutionRequest> {
        if let Some(captures) = self.skill_exec_regex.captures(text) {
            let skill_name = captures.get(1)?.as_str().to_string();
            let params_str = captures.get(2)?.as_str();
            
            // Parse params as JSON
            let params = serde_json::from_str(params_str)
                .unwrap_or_else(|_| serde_json::json!({"raw": params_str}));
            
            return Some(SkillExecutionRequest {
                skill_name,
                params,
                priority: SkillPriority::Normal,
                security_context: None,
            });
        }
        None
    }
    
    /// Execute skill with KB-05 security validation
    async fn execute_skill_with_validation(
        &self,
        request: SkillExecutionRequest,
    ) -> Result<SkillExecutionResult, String> {
        let start_time = std::time::Instant::now();
        
        // Get skill from registry
        let skill = self.live_skills.get(&request.skill_name)
            .ok_or_else(|| format!("Unknown skill: {}", request.skill_name))?;
        
        // KB-05 Security Validation
        if skill.requires_security_check() {
            info!("🛡️ Running KB-05 security validation for skill '{}'", request.skill_name);
            
            if let Err(e) = skill.validate_security(&self.knowledge, &request.params).await {
                warn!("🚫 KB-05 blocked skill '{}': {}", request.skill_name, e);
                return Err(format!("Security validation failed: {}", e));
            }
            
            info!("✓ KB-05 security validation passed for skill '{}'", request.skill_name);
        }
        
        // Execute skill
        match skill.execute(&self.tenant_ctx, &self.knowledge, request.params.clone()).await {
            Ok(output) => {
                let duration_ms = start_time.elapsed().as_millis() as u64;
                let energy_used = match skill.energy_cost() {
                    pagi_core::EnergyCost::Minimal => 50,
                    pagi_core::EnergyCost::Low => 200,
                    pagi_core::EnergyCost::Medium => 1000,
                    pagi_core::EnergyCost::High => 3000,
                    pagi_core::EnergyCost::VeryHigh => 6000,
                };
                
                Ok(SkillExecutionResult {
                    skill_name: request.skill_name,
                    success: true,
                    output,
                    error: None,
                    energy_used,
                    duration_ms,
                })
            }
            Err(e) => {
                let duration_ms = start_time.elapsed().as_millis() as u64;
                Ok(SkillExecutionResult {
                    skill_name: request.skill_name,
                    success: false,
                    output: serde_json::json!({}),
                    error: Some(e.to_string()),
                    energy_used: 0,
                    duration_ms,
                })
            }
        }
    }
    
    /// Get live skills queue size
    pub fn get_skills_queue_size(&self) -> usize {
        self.live_skills.queue_size()
    }
}

/// Start OpenRouter Live session in a dedicated thread
pub fn start_openrouter_live_session(
    log_tx: tokio::sync::broadcast::Sender<String>,
    knowledge: Arc<KnowledgeStore>,
    skills: Arc<SkillRegistry>,
    memory: Arc<MemoryManager>,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let rt = match tokio::runtime::Runtime::new() {
            Ok(r) => r,
            Err(e) => {
                warn!(target: "pagi::voice", "Live mode runtime failed: {}", e);
                return;
            }
        };
        
        rt.block_on(async move {
            match OpenRouterLiveSession::from_env(knowledge, skills, memory) {
                Ok(session) => {
                    info!(target: "pagi::voice", "🌐 OpenRouter Live Mode started");
                    let _ = log_tx.send("🎙️ Phoenix is listening (OpenRouter Live)".to_string());
                    
                    if let Err(e) = session.run().await {
                        warn!(target: "pagi::voice", "Live session error: {}", e);
                    }
                }
                Err(e) => {
                    warn!(target: "pagi::voice", "Failed to start OpenRouter Live: {}", e);
                }
            }
        });
    })
}
