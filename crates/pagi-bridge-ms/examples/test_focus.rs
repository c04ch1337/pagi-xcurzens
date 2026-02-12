//! Verification example: open Copilot sidebar (Win+C) and focus the chat input.
//! Run on Windows without admin rights. Confirms UIA can find the Copilot pane and input.
//!
//! Usage: cargo run -p pagi-bridge-ms --example test_focus

fn main() {
    #[cfg(windows)]
    {
        let redactor = pagi_core::SAORedactor::empty();
        let bridge = match pagi_bridge_ms::BridgeOrchestrator::new(redactor) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("BridgeOrchestrator::new failed: {}", e);
                std::process::exit(1);
            }
        };
        println!("Opening Copilot sidebar and focusing input...");
        match bridge.focus_copilot_input() {
            Ok(true) => println!("OK: Copilot sidebar opened and input focused."),
            Ok(false) => println!("Copilot opened but input element not found (sidebar UI may differ)."),
            Err(e) => {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
    }
    #[cfg(not(windows))]
    {
        println!("test_focus is only supported on Windows.");
        std::process::exit(1);
    }
}
