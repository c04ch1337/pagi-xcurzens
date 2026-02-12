//! CopilotSeeker: locate the Copilot sidebar window and chat input using UIA.
//!
//! Tries window name "Copilot" / "Microsoft Copilot" and standard edit/input patterns.

use uiautomation::types::ControlType;
use uiautomation::{UIAutomation, UIElement};

/// Result of seeking Copilot UI: optional window and optional chat input element.
#[derive(Debug)]
pub struct CopilotElements {
    pub window: Option<UIElement>,
    pub chat_input: Option<UIElement>,
}

/// Locates the Copilot window (sidebar) and the chat input field via UI Automation.
pub struct CopilotSeeker {
    automation: UIAutomation,
    /// Timeout in ms for finding elements
    timeout_ms: u64,
    /// Search depth in UIA tree
    depth: u32,
}

impl CopilotSeeker {
    pub fn new() -> Result<Self, uiautomation::errors::Error> {
        let automation = UIAutomation::new()?;
        Ok(Self {
            automation,
            timeout_ms: 5000,
            depth: 12,
        })
    }

    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }

    pub fn with_depth(mut self, depth: u32) -> Self {
        self.depth = depth;
        self
    }

    /// Find the Copilot window by name (sidebar title often "Copilot" or "Microsoft Copilot").
    pub fn find_copilot_window(&self) -> Result<Option<UIElement>, uiautomation::errors::Error> {
        let root = self.automation.get_root_element()?;
        // Try exact then partial match
        let matcher = self
            .automation
            .create_matcher()
            .from(root)
            .depth(self.depth)
            .timeout(self.timeout_ms)
            .contains_name("Copilot");
        match matcher.find_first() {
            Ok(el) => Ok(Some(el)),
            Err(_) => Ok(None),
        }
    }

    /// Find the chat input (Edit control or document) inside the Copilot window.
    /// Common patterns: AutomationId containing "input", Edit control, or "Ask me anything".
    pub fn find_chat_input(&self, window: &UIElement) -> Result<Option<UIElement>, uiautomation::errors::Error> {
        // Try Edit control first (standard text box)
        let matcher_edit = self
            .automation
            .create_matcher()
            .from_ref(window)
            .depth(8)
            .timeout(2000)
            .control_type(ControlType::Edit);
        if let Ok(el) = matcher_edit.find_first() {
            return Ok(Some(el));
        }
        // Fallback: element with name containing "Ask" or "Type"
        let matcher_ask = self
            .automation
            .create_matcher()
            .from_ref(window)
            .depth(8)
            .timeout(2000)
            .contains_name("Ask");
        if let Ok(el) = matcher_ask.find_first() {
            return Ok(Some(el));
        }
        Ok(None)
    }

    /// One-shot: find Copilot window and its chat input.
    pub fn find_all(&self) -> Result<CopilotElements, uiautomation::errors::Error> {
        let window = self.find_copilot_window()?;
        let chat_input = window
            .as_ref()
            .and_then(|w| self.find_chat_input(w).ok().flatten());
        Ok(CopilotElements { window, chat_input })
    }

    /// Focus the Copilot sidebar (Win+C) and return the input element if found.
    /// Caller should call InputSimulator::open_copilot_sidebar() first, then wait, then seek.
    pub fn focus_input_and_seek(&self) -> Result<Option<UIElement>, uiautomation::errors::Error> {
        let elements = self.find_all()?;
        if let Some(ref input) = elements.chat_input {
            let _ = input.set_focus();
            return Ok(Some(input.clone()));
        }
        Ok(None)
    }
}
