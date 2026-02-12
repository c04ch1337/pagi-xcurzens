export interface Message {
  id: string;
  role: 'user' | 'agi';
  content: string;
  timestamp: number;
  thoughts?: ThoughtLayer[]; // For multi-layer memory/reasoning
  isError?: boolean;
  isPinned?: boolean;
  /** Set when MoE Gater routed to a local expert (e.g. "Local System Tool", "LanceDB"). */
  expertRouting?: string;
}

export interface ThoughtLayer {
  id: string;
  title: string; // e.g., "Short-term Memory Retrieval", "Planner"
  content: string;
  expanded?: boolean;
}

export interface AppSettings {
  apiUrl: string;
  stream: boolean;
  showThoughts: boolean;
  userAlias?: string;
  userAvatar?: string;
  agiAvatar?: string;
  theme: 'dark' | 'light';
  customLogo?: string;
  customFavicon?: string;
  customCss?: string;
  
  // LLM Agent Settings
  llmModel: string;
  llmTemperature: number;
  llmMaxTokens: number;
  orchestratorPersona: string;

  // Feature / paths (from .env via GET /api/v1/config; user override stored in localStorage)
  preferredWorkspacePath?: string;

  // Persona & Archetype (local form state; backend reads from .env)
  birthSign?: string;
  ascendant?: string;
  jungianShadowFocus?: string;

  // Intelligence Layer (SAO background service)
  intelligenceLayerEnabled?: boolean;

  // Sovereign Security Protocols (KB-05)
  sovereignProtocols?: boolean;
}

/** Feature config returned by GET /api/v1/config (values from .env + MoE toggle). Edit .env and restart gateway to change server values; MoE can be toggled in UI. */
export interface GatewayFeatureConfig {
  fs_access_enabled: boolean;
  fs_root: string;
  llm_mode: string;
  /** Model ID from PAGI_LLM_MODEL (e.g. OpenRouter model). */
  llm_model?: string;
  /** Autonomous tick interval in seconds (PAGI_TICK_RATE_SECS). */
  tick_rate_secs?: number;
  /** Chronos events used for Gater local context (PAGI_LOCAL_CONTEXT_LIMIT). */
  local_context_limit?: number;
  /** Default MoE when KB-6 has no value (PAGI_MOE_DEFAULT). */
  moe_default?: string;
  /** When true, chat is routed via MoE (Sparse Intelligence) to OpenRouter / LanceDB / SystemTool. */
  moe_active?: boolean;
  /** "dense" | "sparse" – current MoE mode (from KB-6 or PAGI_MOE_DEFAULT). */
  moe_mode?: string;
  /** "counselor" | "companion" – Orchestrator role (Counselor = base). */
  orchestrator_role?: string;
  /** @deprecated Use orchestrator_role. Kept for backward compatibility. */
  persona_mode?: string;
}

/** Persona heartbeat event from GET /api/v1/persona/stream (SSE). */
export interface PersonaHeartbeatEvent {
  type: 'persona_heartbeat';
  message: string;
  tick_n?: number;
}

/** Sovereign Reset suggestion (from CounselorSkill when rage detected). */
export interface SovereignResetEvent {
  sovereign_reset_suggested: boolean;
  message?: string;
  health_reminder?: string;
}

/** Wellness report from GET /api/v1/skills/wellness-report (7-day Soma aggregation). */
export interface WellnessReport {
  pillars: Record<string, number>;
  individuation_score: number;
  summary: string;
  is_critical: boolean;
  flags?: string[];
  entries_used: number;
  root_cause?: string;
}

/** KB slot status for onboarding Domain Audit (matches backend init::OnboardingKbSlot). */
export interface OnboardingKbSlot {
  slot_id: number;
  label: string;
  entry_count: number;
  connected: boolean;
}

/** Phoenix Marie onboarding protocol state from GET /api/v1/onboarding/status. */
export interface OnboardingState {
  needs_onboarding: boolean;
  /** "Complete" when KB-01 has user_profile; "Incomplete" otherwise (drives discovery loop). */
  onboarding_status?: string;
  phase1_greeting: string;
  phase2_audit_lines: string[];
  phase3_cta: string;
  kb_status: OnboardingKbSlot[];
  vitality: string;
  /** Profiling questions for archetype-agnostic discovery (from DiscoveryModule). */
  profiling_questions?: string[];
  error?: string;
}

/** User profile payload for KB-01 Discovery (POST /api/v1/onboarding/user-profile). */
export interface UserProfilePayload {
  astro_archetype?: string;
  sovereignty_leaks?: string;
  tone_preference?: 'Strictly Technical' | 'Therapeutic Peer' | string;
  birthday?: string; // ISO date format (YYYY-MM-DD)
  birth_time?: string; // Optional: HH:MM format
  birth_location?: string; // City, State/Country
  [key: string]: string | undefined;
}

/** KB slot status (matches backend store::KbStatus). */
export interface KbStatus {
  slot_id: number;
  name: string;
  tree_name: string;
  connected: boolean;
  entry_count: number;
  error?: string;
}

/** Full sovereign state (matches backend store::SovereignState). Used by dashboard/live APIs. */
export interface SovereignState {
  kb_statuses: KbStatus[];
  soma: Record<string, unknown>;
  bio_gate_active: boolean;
  ethos?: unknown;
  mental: Record<string, unknown>;
  people: unknown[];
  governance_summary?: string;
  governed_tasks: unknown[];
  shadow_unlocked: boolean;
  moe_mode?: string;
}

/** Intelligence insights from GET /api/v1/intelligence/insights (SAO background analysis). */
export interface IntelligenceInsights {
  pattern_result: {
    detected: boolean;
    categories: string[];
    root_cause: string;
    counter_measure?: string;
  };
  heuristic_result: {
    roi_score: number;
    is_resource_drain: boolean;
    threat_level: string;
  };
  domain_integrity: number;
  soma_balance: {
    spirit: number;
    mind: number;
    body: number;
    is_critical: boolean;
  };
  timestamp_ms: number;
}

export interface ApiResponse {
  response: string;
  thoughts?: ThoughtLayer[];
  /** Set when MoE routed to an expert (e.g. "Local System Tool", "LanceDB"). */
  expert_routing?: string;
  // Fallback for simple backends
  thought?: string;
}

/** Config status (GET /api/v1/config/status) – for Briefing Room theming and humanity gauge. */
export interface ConfigStatus {
  humanity_ratio: number;
  current_active_archetype: string;
  persona_blend?: string;
  primary_archetype?: string;
  secondary_archetype?: string;
  archetype_auto_switch_enabled?: boolean;
}

/** Per-archetype usage in the report window (Identity Mix / Mirror of Intent). */
export interface ArchetypeUsageBreakdown {
  by_archetype: Record<string, number>;
  total_turns: number;
  summary: string;
}

/** Sovereign Health Report (KB-08 Analytics) from GET /api/v1/health-report. */
export interface ShieldedEvent {
  timestamp_ms: number;
  message: string;
  category: string;
  date: string;
}

export interface TransitCorrelationEntry {
  date: string;
  transit_summary: string;
  event_kind: string;
}

export interface HealthReport {
  leak_stats: { by_category: Record<string, number>; total_shielded: number };
  transit_vulnerability_score: number;
  transit_correlations: TransitCorrelationEntry[];
  efficiency_score: number;
  phoenix_summary: string;
  archetype_label: string | null;
  user_name: string | null;
  shielded_events: ShieldedEvent[];
  window_start: string;
  window_end: string;
  /** Identity Mix: percentage of time in each archetype (Virgo, Pisces, Capricorn, etc.). */
  archetype_usage_breakdown?: ArchetypeUsageBreakdown | null;
  /** 0–1 average rest (sleep) score when Vitality Shield data present. */
  vitality_score?: number | null;
  /** Rest vs. output per day for correlation graph. */
  rest_vs_output?: RestVsOutputEntry[] | null;
}

/** One day's rest (sleep hours) and output (turns + shielded) for Rest vs. Output. */
export interface RestVsOutputEntry {
  date: string;
  rest_score: number;
  output_score: number;
}