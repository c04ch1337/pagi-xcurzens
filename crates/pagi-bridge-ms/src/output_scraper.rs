//! OutputScraper: poll the UIA tree for Copilot's last response text.
//! Detects when Copilot has finished "typing" and reads the response element.

use std::thread;
use std::time::Duration;
use uiautomation::types::ControlType;
use uiautomation::{UIAutomation, UIElement};

pub struct OutputScraper {
    automation: UIAutomation,
    /// Max wait for response (ms)
    timeout_ms: u64,
    /// Poll interval (ms)
    poll_interval_ms: u64,
}

impl OutputScraper {
    pub fn new() -> Result<Self, uiautomation::Error> {
        let automation = UIAutomation::new()?;
        Ok(Self {
            automation,
            timeout_ms: 60_000,
            poll_interval_ms: 1_500,
        })
    }

    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }

    pub fn with_poll_interval(mut self, ms: u64) -> Self {
        self.poll_interval_ms = ms;
        self
    }

    /// Find the last assistant response in the Copilot window (Text or Document elements with substantial content).
    pub fn find_last_response(&self, window: &UIElement) -> Result<Option<String>, uiautomation::Error> {
        let matcher = self
            .automation
            .create_matcher()
            .from_ref(window)
            .depth(15)
            .timeout(2000)
            .control_type(ControlType::Text);
        let elements = matcher.find_all().unwrap_or_default();
        let mut best: Option<String> = None;
        for el in elements.iter().rev() {
            if let Ok(name) = el.get_name() {
                let name = name.trim();
                if name.len() > 20 && !name.eq_ignore_ascii_case("ask me anything") {
                    best = Some(name.to_string());
                    break;
                }
            }
        }
        Ok(best)
    }

    /// Poll until we get a non-empty response or timeout. Returns the scraped text if any.
    pub fn wait_for_response(&self, window: &UIElement) -> Result<Option<String>, uiautomation::Error> {
        let deadline = std::time::Instant::now() + Duration::from_millis(self.timeout_ms);
        while std::time::Instant::now() < deadline {
            if let Ok(Some(text)) = self.find_last_response(window) {
                if !text.is_empty() {
                    return Ok(Some(text));
                }
            }
            thread::sleep(Duration::from_millis(self.poll_interval_ms));
        }
        Ok(None)
    }
}
