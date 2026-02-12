//! Identity Orchestrator — XCURZENS voice and Traveler UI brand filter.
//! Intercepts LLM string outputs and enforces professional, coastal, high-bandwidth voice
//! with Navy and Orange HTML wrappers for the Traveler UI.

/// Navy (primary) and Orange (accent) — XCURZENS brand for Traveler UI.
pub const BRAND_NAVY: &str = "#051C55";
pub const BRAND_ORANGE: &str = "#FA921C";

/// Root Sovereign for auth and logs.
pub const ROOT_SOVEREIGN: &str = "Jamey";

/// Filters LLM string output into the pagi-xcurzens voice and wraps it for the Traveler UI.
/// - Voice: professional, coastal, high-bandwidth.
/// - Output: HTML with Navy (#051C55) and Orange (#FA921C) wrappers suitable for Traveler UI.
pub fn brand_filter(llm_output: &str) -> String {
    let voice_adjusted = enforce_xcurzens_voice(llm_output);
    wrap_for_traveler_ui(&voice_adjusted)
}

/// Enforces XCURZENS voice: professional, coastal, high-bandwidth.
/// Light normalization; preserves content while aligning tone.
fn enforce_xcurzens_voice(s: &str) -> String {
    let t = s.trim();
    if t.is_empty() {
        return String::new();
    }
    // Ensure first character is uppercase for professional tone
    let mut out = t.to_string();
    if let Some(c) = out.chars().next() {
        if c.is_ascii_lowercase() {
            out.replace_range(0..c.len_utf8(), &c.to_uppercase().to_string());
        }
    }
    out
}

/// Wraps content in HTML with Navy primary and Orange accent for Traveler UI.
fn wrap_for_traveler_ui(content: &str) -> String {
    format!(
        r#"<div class="xcurzens-traveler" style="color: {};"><span class="xcurzens-accent" style="color: {};">▸</span> {} </div>"#,
        BRAND_NAVY,
        BRAND_ORANGE,
        html_escape(content)
    )
}

/// Escapes HTML to prevent injection in Traveler UI.
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn brand_filter_includes_navy_and_orange() {
        let out = brand_filter("hello");
        assert!(out.contains(BRAND_NAVY));
        assert!(out.contains(BRAND_ORANGE));
    }

    #[test]
    fn html_escaped() {
        let out = brand_filter("<script>");
        assert!(!out.contains("<script>"));
        assert!(out.contains("&lt;script&gt;"));
    }
}
