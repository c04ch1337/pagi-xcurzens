//! BridgeOrchestrator: Redact → Open Copilot → Paste → Wait → Scrape.
//! Any string sent to Copilot MUST be run through SAORedactor first.

use crate::copilot_seeker::CopilotSeeker;
use crate::input_simulator::InputSimulator;
use crate::output_scraper::OutputScraper;
use pagi_core::SAORedactor;
use std::path::Path;
use std::time::Duration;
use std::thread;

pub struct BridgeOrchestrator {
    seeker: CopilotSeeker,
    input: InputSimulator,
    scraper: OutputScraper,
    redactor: SAORedactor,
}

impl BridgeOrchestrator {
    /// Build orchestrator with a redactor. Text passed to `send_to_copilot` is always sanitized.
    pub fn new(redactor: SAORedactor) -> Result<Self, uiautomation::Error> {
        let seeker = CopilotSeeker::new()?;
        let scraper = OutputScraper::new()?;
        Ok(Self {
            seeker,
            input: InputSimulator::new(),
            scraper,
            redactor,
        })
    }

    /// Load redactor from default path (e.g. data/protected_terms.txt) and create orchestrator.
    pub fn with_redactor_from_path(data_dir: &Path) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let redactor = SAORedactor::load_from_data_dir(data_dir).unwrap_or_else(|_| SAORedactor::empty());
        Self::new(redactor).map_err(|e| e.into())
    }

    /// Send text to Copilot: MUST be redacted. Opens sidebar, pastes, submits, waits for response, returns scraped text.
    /// If you pass raw transcript, it is sanitized internally.
    pub fn send_to_copilot(&self, raw_text: String) -> Result<String, String> {
        let sanitized = self.redactor.sanitize_transcript(raw_text);
        self.send_redacted_to_copilot(sanitized)
    }

    /// Send already-redacted text (e.g. from caller who used the same redactor). Use when you have already sanitized.
    pub fn send_redacted_to_copilot(&self, redacted_text: String) -> Result<String, String> {
        self.input.open_copilot_sidebar()?;
        thread::sleep(Duration::from_millis(800));

        let input_el = self.seeker.focus_input_and_seek().map_err(|e| e.to_string())?;
        if input_el.is_none() {
            return Err("Could not find Copilot chat input".to_string());
        }

        self.input.paste_and_submit(&redacted_text)?;
        thread::sleep(Duration::from_millis(1500));

        let window = self.seeker.find_copilot_window().map_err(|e| e.to_string())?
            .ok_or("Copilot window not found after submit")?;
        let response = self.scraper.wait_for_response(&window).map_err(|e| e.to_string())?;
        Ok(response.unwrap_or_else(|| "No response captured (timeout or no text element)".to_string()))
    }

    /// Focus-only flow: open sidebar and focus input without sending. For verification (e.g. test_focus).
    pub fn focus_copilot_input(&self) -> Result<bool, String> {
        self.input.open_copilot_sidebar()?;
        thread::sleep(Duration::from_millis(1000));
        let input = self.seeker.focus_input_and_seek().map_err(|e| e.to_string())?;
        Ok(input.is_some())
    }
}
