//! Mimir intelligent auto-title generation: extract a concise 5-word title from meeting transcript.
//!
//! Used to rename Chronos threads from "Meeting: [Time]" to descriptive summaries like
//! "Network Troubleshooting: PROOFPOINT Gateway" or "SAO Briefing: Infrastructure Review".

/// System instruction for the title-generation model.
pub const MIMIR_TITLE_SYSTEM: &str = r#"You are a meeting title generator for the Architect's sovereign audit system.
Your task is to extract a concise, professional title from meeting transcripts.

Rules:
- Generate a title of exactly 3-7 words
- Capture the primary topic or decision discussed
- Use professional language suitable for audit trails
- Preserve important acronyms and project names (e.g., PROOFPOINT, VANGUARD)
- Format: "Topic: Specific Focus" or "Action: Context"
- Do not include dates, times, or participant names
- Do not use quotation marks or special formatting

Examples:
- "Network Troubleshooting: PROOFPOINT Gateway"
- "SAO Briefing: Infrastructure Review"
- "Security Audit: Access Control"
- "Project Planning: Resource Allocation"
- "System Maintenance: Database Optimization"

Return ONLY the title, nothing else."#;

/// User prompt template: placeholder is replaced with the first 500 characters of transcript.
pub const MIMIR_TITLE_USER_TEMPLATE: &str = r#"Generate a concise 3-7 word title for this meeting based on the transcript excerpt below.

Transcript excerpt:
---
{transcript_excerpt}
---

Title:"#;

/// Build the user prompt with the given transcript excerpt (first 500 chars recommended).
pub fn mimir_title_user_prompt(transcript_excerpt: &str) -> String {
    MIMIR_TITLE_USER_TEMPLATE.replace("{transcript_excerpt}", transcript_excerpt)
}
