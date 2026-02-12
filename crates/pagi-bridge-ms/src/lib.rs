//! pagi-bridge-ms: User-level UI Automation bridge for Microsoft Copilot sidebar.
//!
//! Sovereign Loophole: automate the local Copilot pane (Win+C) with **redacted** content
//! so the Master Orchestrator can use it as a "Sidecar LLM" without exposing high-side data.
//!
//! - **CopilotSeeker**: Locate Copilot window and chat input via UIA.
//! - **InputSimulator**: Clipboard paste + enigo for Win+C, Ctrl+V, Enter.
//! - **OutputScraper**: Poll UIA for Copilot's last response.
//! - **BridgeOrchestrator**: Redact → Paste → Wait → Scrape → (optionally store in Chronos).
//!
//! Windows only; on other platforms the API is stubbed.

#[cfg(windows)]
mod copilot_seeker;
#[cfg(windows)]
mod input_simulator;
#[cfg(windows)]
mod output_scraper;
#[cfg(windows)]
mod orchestrator;

#[cfg(windows)]
pub use copilot_seeker::{CopilotElements, CopilotSeeker};
#[cfg(windows)]
pub use input_simulator::InputSimulator;
#[cfg(windows)]
pub use output_scraper::OutputScraper;
#[cfg(windows)]
pub use orchestrator::BridgeOrchestrator;

/// Stub when not on Windows: bridge is no-op.
#[cfg(not(windows))]
pub fn bridge_unavailable() -> bool {
    true
}
