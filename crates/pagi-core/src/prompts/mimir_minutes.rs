//! Mimir minute-generation prompt: turn raw transcript into SAO-ready structured Markdown.
//!
//! Extract Who, What, When; identify Action Items and SAO context.
//! Redaction: high-side codenames not in the allowed list can be replaced with [PROTECTED_TERM].

/// System instruction for the minute-generation model.
pub const MIMIR_MINUTES_SYSTEM: &str = r#"You are the Architect's scribe for SAO (Special Access Office) meeting documentation.
Your output must be professional, structured, and suitable for sovereign audit trails.

Rules:
- Extract and list: Who (participants or roles), What (decisions/topics), When (time references if present).
- Identify explicit Action Items with owner and deadline when stated.
- Flag any mention of sensitive project names, codenames, or "SAO" context for potential redaction.
- Format the summary as structured Markdown: use tables for Who/What/When and a bullet list for Action Items.
- If the transcript mentions high-side codenames that are not in the allowed list (e.g. "Project: PROOFPOINT"), replace them with [PROTECTED_TERM] in the final markdown unless the user has provided a Sovereign Override.
- Do not invent participants or actions; only report what is stated or clearly implied in the transcript."#;

/// User prompt template: placeholder is replaced with the actual transcript.
pub const MIMIR_MINUTES_USER_TEMPLATE: &str = r#"Convert the following meeting transcript into a structured "Architect's View" summary.

Transcript:
---
{transcript}
---

Produce:
1. **Participants / Roles** (table or list)
2. **Decisions & Topics** (table: Topic | Summary)
3. **Action Items** (list with Owner and Due when stated)
4. **SAO / Sensitive context** (brief note if any protected terms were redacted or flagged)

Use Markdown tables and bullets. Keep the summary concise and audit-ready."#;

/// Build the user prompt with the given transcript.
pub fn mimir_minutes_user_prompt(transcript: &str) -> String {
    MIMIR_MINUTES_USER_TEMPLATE.replace("{transcript}", transcript)
}
