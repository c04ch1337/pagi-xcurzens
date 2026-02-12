//! InputSimulator: open Copilot sidebar (Win+C), paste via clipboard (Ctrl+V), submit (Enter).
//! Uses arboard for clipboard and enigo for key simulation. Clears clipboard after paste for sovereign security.

use arboard::Clipboard;
use enigo::{Direction, Enigo, Key, Keyboard, Settings};
use std::thread;
use std::time::Duration;
use tracing::debug;

pub struct InputSimulator {
    /// Short delay between key events (ms)
    key_delay_ms: u64,
}

impl Default for InputSimulator {
    fn default() -> Self {
        Self { key_delay_ms: 50 }
    }
}

impl InputSimulator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_key_delay(mut self, ms: u64) -> Self {
        self.key_delay_ms = ms;
        self
    }

    fn delay(&self) {
        thread::sleep(Duration::from_millis(self.key_delay_ms));
    }

    /// Simulate Win+C to open/focus the Copilot sidebar.
    pub fn open_copilot_sidebar(&self) -> Result<(), String> {
        let mut enigo = Enigo::new(&Settings::default()).map_err(|e| e.to_string())?;
        let _ = enigo.key(Key::Meta, Direction::Press);
        self.delay();
        let _ = enigo.key(Key::Unicode('c'), Direction::Click);
        self.delay();
        let _ = enigo.key(Key::Meta, Direction::Release);
        Ok(())
    }

    /// Copy text to clipboard, then simulate Ctrl+V and Enter. Clears clipboard when done.
    /// Call this after focusing the Copilot input (e.g. CopilotSeeker::focus_input_and_seek).
    pub fn paste_and_submit(&self, text: &str) -> Result<(), String> {
        let mut clipboard = Clipboard::new().map_err(|e| e.to_string())?;
        clipboard.set_text(text).map_err(|e| e.to_string())?;
        self.delay();

        let mut enigo = Enigo::new(&Settings::default()).map_err(|e| e.to_string())?;
        let _ = enigo.key(Key::Control, Direction::Press);
        self.delay();
        let _ = enigo.key(Key::Unicode('v'), Direction::Click);
        self.delay();
        let _ = enigo.key(Key::Control, Direction::Release);
        self.delay();
        let _ = enigo.key(Key::Return, Direction::Click);

        // Sovereign security: clear clipboard immediately after paste
        if let Ok(mut cb) = Clipboard::new() {
            let _ = cb.clear();
        }
        debug!(target: "pagi::bridge_ms", "Pasted and submitted; clipboard cleared");
        Ok(())
    }

    /// Only paste (Ctrl+V) without Enter. Useful if caller wants to review before submit.
    pub fn paste_only(&self, text: &str) -> Result<(), String> {
        let mut clipboard = Clipboard::new().map_err(|e| e.to_string())?;
        clipboard.set_text(text).map_err(|e| e.to_string())?;
        self.delay();

        let mut enigo = Enigo::new(&Settings::default()).map_err(|e| e.to_string())?;
        let _ = enigo.key(Key::Control, Direction::Press);
        self.delay();
        let _ = enigo.key(Key::Unicode('v'), Direction::Click);
        self.delay();
        let _ = enigo.key(Key::Control, Direction::Release);

        if let Ok(mut cb) = Clipboard::new() {
            let _ = cb.clear();
        }
        Ok(())
    }
}
