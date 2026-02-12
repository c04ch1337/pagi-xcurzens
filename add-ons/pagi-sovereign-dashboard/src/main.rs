//! **Sovereign Dashboard** — `pagi status` / `pagi dash` prints a high-fidelity
//! situation report of your AGI's cross-layer state across all 9 knowledge slots.
//!
//! ## Sections
//!
//! 1. **System Integrity** — 9-slot health matrix (entry counts, connection status)
//! 2. **Soma (Slot 8)** — BioGate: sleep, readiness, HR, HRV
//! 3. **Kardia (Slot 7)** — Mental state + Relational Map (people, trust, attachment)
//! 4. **Ethos (Slot 6)** — Active philosophical school and maxims
//! 5. **Oikos (Slot 2)** — Task governance summary and governed tasks
//! 6. **Shadow (Slot 9)** — Vault lock status
//!
//! ## Usage
//!
//! ```text
//! pagi status     — full dashboard (default)
//! pagi dash       — alias for status
//! pagi            — same (no args)
//! pagi --help     — print usage
//! ```

use chrono::Utc;
use comfy_table::modifiers::UTF8_ROUND_CORNERS;
use comfy_table::presets::UTF8_FULL;
use comfy_table::{Attribute, Cell, CellAlignment, Color, ContentArrangement, Table};
use pagi_core::{CoreConfig, GovernanceAction, KbType, KnowledgeStore, SovereignState};
use std::path::Path;

const AGENT_ID: &str = "default";
const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let sub = args.get(1).map(|s| s.as_str()).unwrap_or("status");

    match sub {
        "status" | "dash" => {
            if let Err(e) = run_status() {
                eprintln!("pagi {}: {}", sub, e);
                std::process::exit(1);
            }
        }
        "--help" | "-h" | "help" => {
            println!("PAGI Sovereign Dashboard v{}", VERSION);
            println!();
            println!("Usage: pagi [COMMAND]");
            println!();
            println!("Commands:");
            println!("  status   Full Sovereign Dashboard — cross-layer situation report (default)");
            println!("  dash     Alias for 'status'");
            println!("  help     Print this help message");
            println!();
            println!("The dashboard reads from the KnowledgeStore at {{storage_path}}/pagi_knowledge.");
            println!("Configure via PAGI_CONFIG env var or config/gateway.toml.");
        }
        other => {
            eprintln!("Unknown subcommand '{}'. Use: pagi status | pagi dash | pagi --help", other);
            std::process::exit(1);
        }
    }
}

fn run_status() -> Result<(), String> {
    let config = CoreConfig::load().map_err(|e| format!("Config: {}", e))?;
    let kb_path = Path::new(&config.storage_path).join("pagi_knowledge");
    let port = config.port;

    let state = match KnowledgeStore::open_path(&kb_path) {
        Ok(store) => store.get_full_sovereign_state(AGENT_ID),
        Err(e) => {
            let msg = e.to_string();
            if msg.contains("lock") || msg.contains("locked") {
                // Fallback: fetch from Live Status API when gateway holds the Sled lock
                fetch_sovereign_state_from_api(port)?
            } else {
                return Err(format!(
                    "Cannot open knowledge store at {}: {}",
                    kb_path.display(),
                    e
                ));
            }
        }
    };

    let now = Utc::now().format("%Y-%m-%d %H:%M:%S UTC");
    println!();
    println!("╔══════════════════════════════════════════════════════════════════════╗");
    println!("║   🏛️  PAGI SOVEREIGN DASHBOARD v{}  —  Situation Report            ║", VERSION);
    println!("║   {}                                          ║", now);
    println!("╚══════════════════════════════════════════════════════════════════════╝");
    println!();

    render_sovereign_state(&state);

    println!("  Run `pagi status` at any time to refresh this report.");
    println!();
    Ok(())
}

/// Fetches full sovereign state from the gateway's Live Status API (used when Sled is locked).
fn fetch_sovereign_state_from_api(port: u16) -> Result<SovereignState, String> {
    let url = format!("http://127.0.0.1:{}/api/v1/sovereign-status", port);
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("HTTP client: {}", e))?;
    let mut req = client.get(&url);
    if let Ok(key) = std::env::var("PAGI_API_KEY") {
        let key = key.trim();
        if !key.is_empty() {
            req = req.header("X-API-Key", key);
        }
    }
    let resp = req.send().map_err(|e| {
        format!(
            "Gateway unreachable at {} ({}). Start the gateway or use a different port.",
            url, e
        )
    })?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().unwrap_or_default();
        return Err(format!(
            "Sovereign-status API error {}: {}",
            status,
            if body.is_empty() { "unauthorized? Set PAGI_API_KEY if the endpoint is protected." } else { body.as_str() }
        ));
    }
    resp.json().map_err(|e| format!("Invalid JSON from gateway: {}", e))
}

/// Renders the full dashboard from a SovereignState (direct store or API).
fn render_sovereign_state(state: &SovereignState) {
    print_system_integrity_from_state(state);
    print_soma_from_state(state);
    print_ethos_from_state(state);
    print_kardia_from_state(state);
    print_oikos_from_state(state);
    print_shadow_from_state(state);
}

// ─────────────────────────────────────────────────────────────────────────────
// Section renderers (from SovereignState — direct store or Live Status API)
// ─────────────────────────────────────────────────────────────────────────────

fn print_system_integrity_from_state(state: &SovereignState) {
    println!("  ┌─ SYSTEM INTEGRITY ─ 9-Slot Knowledge Matrix ─────────────────────┐");
    println!();

    let statuses = &state.kb_statuses;

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec![
            Cell::new("Slot")
                .set_alignment(CellAlignment::Center)
                .add_attribute(Attribute::Bold),
            Cell::new("Domain")
                .set_alignment(CellAlignment::Center)
                .add_attribute(Attribute::Bold),
            Cell::new("Entries")
                .set_alignment(CellAlignment::Center)
                .add_attribute(Attribute::Bold),
            Cell::new("Status")
                .set_alignment(CellAlignment::Center)
                .add_attribute(Attribute::Bold),
        ]);

    for status in statuses.iter() {
        let slot_cell = Cell::new(format!("KB-{}", status.slot_id))
            .set_alignment(CellAlignment::Center);

        let domain_cell = Cell::new(&status.name);

        let entries_cell = Cell::new(status.entry_count)
            .set_alignment(CellAlignment::Right);

        let (status_text, status_color) = if let Some(ref err) = status.error {
            if err.as_str().contains("LOCKED") {
                ("🔒 LOCKED", Color::Yellow)
            } else {
                ("✗ ERROR", Color::Red)
            }
        } else if status.connected && status.entry_count > 0 {
            ("● ACTIVE", Color::Green)
        } else if status.connected {
            ("○ EMPTY", Color::DarkYellow)
        } else {
            ("✗ DISCONNECTED", Color::Red)
        };

        let status_cell = Cell::new(status_text)
            .set_alignment(CellAlignment::Center)
            .fg(status_color);

        table.add_row(vec![slot_cell, domain_cell, entries_cell, status_cell]);
    }

    println!("{table}");

    // Summary line
    let total_entries: usize = statuses.iter().map(|s| s.entry_count).sum();
    let active_count = statuses.iter().filter(|s| s.connected && s.entry_count > 0).count();
    let error_count = statuses.iter().filter(|s| s.error.is_some()).count();

    println!(
        "  Total entries: {}  |  Active slots: {}/9  |  Errors: {}",
        total_entries, active_count, error_count
    );
    println!();
}

fn print_soma_from_state(state: &SovereignState) {
    println!("  ┌─ SOMA (Slot 8) ─ Body / BioGate ────────────────────────────────┐");
    println!();

    let soma = &state.soma;
    let bio_active = state.bio_gate_active;

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec![
            Cell::new("Metric")
                .add_attribute(Attribute::Bold),
            Cell::new("Value")
                .set_alignment(CellAlignment::Right)
                .add_attribute(Attribute::Bold),
            Cell::new("Assessment")
                .add_attribute(Attribute::Bold),
        ]);

    // Sleep hours
    let sleep_assessment = if soma.sleep_hours == 0.0 {
        ("No data", Color::DarkYellow)
    } else if soma.sleep_hours < 6.0 {
        ("⚠ INSUFFICIENT", Color::Red)
    } else if soma.sleep_hours < 7.0 {
        ("Marginal", Color::Yellow)
    } else {
        ("✓ Good", Color::Green)
    };
    table.add_row(vec![
        Cell::new("Sleep"),
        Cell::new(format!("{:.1}h", soma.sleep_hours)).set_alignment(CellAlignment::Right),
        Cell::new(sleep_assessment.0).fg(sleep_assessment.1),
    ]);

    // Readiness
    let readiness_assessment = if soma.readiness_score >= 100 && soma.sleep_hours == 0.0 {
        ("No data", Color::DarkYellow)
    } else if soma.readiness_score < 50 {
        ("⚠ LOW", Color::Red)
    } else if soma.readiness_score < 70 {
        ("Moderate", Color::Yellow)
    } else {
        ("✓ Good", Color::Green)
    };
    table.add_row(vec![
        Cell::new("Readiness"),
        Cell::new(format!("{}/100", soma.readiness_score)).set_alignment(CellAlignment::Right),
        Cell::new(readiness_assessment.0).fg(readiness_assessment.1),
    ]);

    // Resting HR
    let hr_assessment = if soma.resting_hr == 0 {
        ("No data", Color::DarkYellow)
    } else if soma.resting_hr > 90 {
        ("⚠ Elevated", Color::Red)
    } else if soma.resting_hr > 75 {
        ("Normal-high", Color::Yellow)
    } else {
        ("✓ Good", Color::Green)
    };
    table.add_row(vec![
        Cell::new("Resting HR"),
        Cell::new(format!("{} bpm", soma.resting_hr)).set_alignment(CellAlignment::Right),
        Cell::new(hr_assessment.0).fg(hr_assessment.1),
    ]);

    // HRV
    let hrv_assessment = if soma.hrv == 0 {
        ("No data", Color::DarkYellow)
    } else if soma.hrv < 30 {
        ("⚠ Low", Color::Red)
    } else if soma.hrv < 50 {
        ("Moderate", Color::Yellow)
    } else {
        ("✓ Good", Color::Green)
    };
    table.add_row(vec![
        Cell::new("HRV (RMSSD)"),
        Cell::new(format!("{} ms", soma.hrv)).set_alignment(CellAlignment::Right),
        Cell::new(hrv_assessment.0).fg(hrv_assessment.1),
    ]);

    println!("{table}");

    // BioGate status
    if bio_active {
        println!(
            "  ⚡ BioGate: {} — supportive tone engaged, grace_multiplier={:.1}",
            "ACTIVE",
            pagi_core::SomaState::GRACE_MULTIPLIER_OVERRIDE
        );
    } else {
        println!("  BioGate: inactive (biological metrics within normal range)");
    }
    println!();
}

fn print_ethos_from_state(state: &SovereignState) {
    println!("  ┌─ ETHOS (Slot 6) ─ Philosophical Lens ───────────────────────────┐");
    println!();

    let ethos = &state.ethos;

    match &ethos {
        Some(p) => {
            let mut table = Table::new();
            table
                .load_preset(UTF8_FULL)
                .apply_modifier(UTF8_ROUND_CORNERS)
                .set_content_arrangement(ContentArrangement::Dynamic)
                .set_header(vec![
                    Cell::new("Property")
                        .add_attribute(Attribute::Bold),
                    Cell::new("Value")
                        .add_attribute(Attribute::Bold),
                ]);

            table.add_row(vec![
                Cell::new("Active School"),
                Cell::new(&p.active_school)
                    .add_attribute(Attribute::Bold)
                    .fg(Color::Cyan),
            ]);

            table.add_row(vec![
                Cell::new("Tone Weight"),
                Cell::new(format!("{:.1} / 1.0", p.tone_weight)),
            ]);

            if !p.core_maxims.is_empty() {
                let maxims_display: Vec<String> = p
                    .core_maxims
                    .iter()
                    .enumerate()
                    .map(|(i, m)| format!("{}. {}", i + 1, m))
                    .collect();
                table.add_row(vec![
                    Cell::new("Core Maxims"),
                    Cell::new(maxims_display.join("\n")),
                ]);
            }

            println!("{table}");
        }
        None => {
            println!("  (No philosophical policy set — use EthosSync to configure e.g. Stoic, Growth-Mindset)");
        }
    }
    println!();
}

fn print_kardia_from_state(state: &SovereignState) {
    println!("  ┌─ KARDIA (Slot 7) ─ Heart / Relational ──────────────────────────┐");
    println!();

    let mental = &state.mental;

    let mut mental_table = Table::new();
    mental_table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec![
            Cell::new("Mental State Metric")
                .add_attribute(Attribute::Bold),
            Cell::new("Value")
                .set_alignment(CellAlignment::Right)
                .add_attribute(Attribute::Bold),
            Cell::new("Status")
                .add_attribute(Attribute::Bold),
        ]);

    // Relational stress
    let stress_status = if mental.relational_stress > 0.7 {
        ("⚠ HIGH — empathetic tone active", Color::Red)
    } else if mental.relational_stress > 0.4 {
        ("Moderate", Color::Yellow)
    } else {
        ("✓ Low", Color::Green)
    };
    mental_table.add_row(vec![
        Cell::new("Relational Stress"),
        Cell::new(format!("{:.2}", mental.relational_stress)).set_alignment(CellAlignment::Right),
        Cell::new(stress_status.0).fg(stress_status.1),
    ]);

    // Burnout risk
    let burnout_status = if mental.burnout_risk > 0.7 {
        ("⚠ HIGH", Color::Red)
    } else if mental.burnout_risk > 0.4 {
        ("Moderate", Color::Yellow)
    } else {
        ("✓ Low", Color::Green)
    };
    mental_table.add_row(vec![
        Cell::new("Burnout Risk"),
        Cell::new(format!("{:.2}", mental.burnout_risk)).set_alignment(CellAlignment::Right),
        Cell::new(burnout_status.0).fg(burnout_status.1),
    ]);

    // Grace multiplier
    let grace_status = if mental.grace_multiplier >= 1.5 {
        ("Physical load adjustment active", Color::Yellow)
    } else if mental.grace_multiplier > 1.0 {
        ("Slightly elevated", Color::DarkYellow)
    } else {
        ("✓ Normal", Color::Green)
    };
    mental_table.add_row(vec![
        Cell::new("Grace Multiplier"),
        Cell::new(format!("{:.2}x", mental.grace_multiplier)).set_alignment(CellAlignment::Right),
        Cell::new(grace_status.0).fg(grace_status.1),
    ]);

    println!("{mental_table}");

    // Tone indicators
    if mental.has_physical_load_adjustment() {
        println!("  → Physical load adjustment applied (gentler tone)");
    }
    if mental.needs_empathetic_tone() {
        println!("  → Empathetic tone active (high emotional load)");
    }
    println!();

    let people = &state.people;
    if people.is_empty() {
        println!("  Relational Map: (empty — use KardiaMap to add people)");
    } else {
        println!("  Relational Map ({} contacts):", people.len());
        println!();

        let mut people_table = Table::new();
        people_table
            .load_preset(UTF8_FULL)
            .apply_modifier(UTF8_ROUND_CORNERS)
            .set_content_arrangement(ContentArrangement::Dynamic)
            .set_header(vec![
                Cell::new("Name")
                    .add_attribute(Attribute::Bold),
                Cell::new("Relationship")
                    .add_attribute(Attribute::Bold),
                Cell::new("Trust")
                    .set_alignment(CellAlignment::Center)
                    .add_attribute(Attribute::Bold),
                Cell::new("Attachment")
                    .add_attribute(Attribute::Bold),
                Cell::new("Triggers")
                    .add_attribute(Attribute::Bold),
            ]);

        for p in people.iter().take(15) {
            let rel = if p.relationship.is_empty() { "—" } else { &p.relationship };
            let style = if p.attachment_style.is_empty() { "—" } else { &p.attachment_style };
            let triggers = if p.triggers.is_empty() {
                "—".to_string()
            } else {
                p.triggers.join(", ")
            };

            let trust_color = if p.trust_score >= 0.7 {
                Color::Green
            } else if p.trust_score >= 0.4 {
                Color::Yellow
            } else {
                Color::Red
            };

            let trust_bar = trust_bar_ascii(p.trust_score);

            people_table.add_row(vec![
                Cell::new(&p.name),
                Cell::new(rel),
                Cell::new(format!("{} {:.2}", trust_bar, p.trust_score))
                    .set_alignment(CellAlignment::Center)
                    .fg(trust_color),
                Cell::new(style),
                Cell::new(triggers),
            ]);
        }

        if people.len() > 15 {
            people_table.add_row(vec![
                Cell::new(format!("... and {} more", people.len() - 15)),
                Cell::new(""),
                Cell::new(""),
                Cell::new(""),
                Cell::new(""),
            ]);
        }

        println!("{people_table}");
    }
    println!();
}

fn print_oikos_from_state(state: &SovereignState) {
    println!("  ┌─ OIKOS (Slot 2) ─ Task Governance ──────────────────────────────┐");
    println!();

    let summary = &state.governance_summary;
    match summary {
        Some(s) => {
            println!("  Latest Governance Summary:");
            for line in s.lines().take(20) {
                println!("    {}", line);
            }
            if s.lines().count() > 20 {
                println!("    ...");
            }
            println!();
        }
        None => {
            println!("  (No governance summary yet — run OikosTaskGovernor to evaluate tasks)");
            println!();
        }
    }

    let tasks = &state.governed_tasks;
    if !tasks.is_empty() {
        println!("  Governed Tasks ({} total):", tasks.len());
        println!();

        let mut task_table = Table::new();
        task_table
            .load_preset(UTF8_FULL)
            .apply_modifier(UTF8_ROUND_CORNERS)
            .set_content_arrangement(ContentArrangement::Dynamic)
            .set_header(vec![
                Cell::new("Task")
                    .add_attribute(Attribute::Bold),
                Cell::new("Difficulty")
                    .set_alignment(CellAlignment::Center)
                    .add_attribute(Attribute::Bold),
                Cell::new("Priority")
                    .set_alignment(CellAlignment::Center)
                    .add_attribute(Attribute::Bold),
                Cell::new("Action")
                    .set_alignment(CellAlignment::Center)
                    .add_attribute(Attribute::Bold),
            ]);

        for task in tasks.iter().take(10) {
            let difficulty_str = format!("{:?}", task.difficulty);

            let priority_color = if task.effective_priority >= 0.8 {
                Color::Red
            } else if task.effective_priority >= 0.5 {
                Color::Yellow
            } else {
                Color::Green
            };

            let (action_str, action_color) = match &task.action {
                GovernanceAction::Proceed => ("✓ Proceed", Color::Green),
                GovernanceAction::Postpone { reason } => {
                    let _ = reason; // used below
                    ("⏸ Postpone", Color::Yellow)
                }
                GovernanceAction::Simplify { suggestion } => {
                    let _ = suggestion;
                    ("✂ Simplify", Color::Cyan)
                }
                GovernanceAction::Deprioritize { reason } => {
                    let _ = reason;
                    ("↓ Deprioritize", Color::DarkYellow)
                }
            };

            task_table.add_row(vec![
                Cell::new(&task.title),
                Cell::new(&difficulty_str).set_alignment(CellAlignment::Center),
                Cell::new(format!("{:.2}", task.effective_priority))
                    .set_alignment(CellAlignment::Center)
                    .fg(priority_color),
                Cell::new(action_str)
                    .set_alignment(CellAlignment::Center)
                    .fg(action_color),
            ]);
        }

        if tasks.len() > 10 {
            task_table.add_row(vec![
                Cell::new(format!("... and {} more", tasks.len() - 10)),
                Cell::new(""),
                Cell::new(""),
                Cell::new(""),
            ]);
        }

        println!("{task_table}");
    } else if summary.is_none() {
        println!("  No governed tasks. Add tasks via OikosTaskGovernor.");
    }
    println!();
}

fn print_shadow_from_state(state: &SovereignState) {
    println!("  ┌─ SHADOW (Slot 9) ─ The Vault ───────────────────────────────────┐");
    println!();

    let shadow_unlocked = state.shadow_unlocked;

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec![
            Cell::new("Property")
                .add_attribute(Attribute::Bold),
            Cell::new("Status")
                .add_attribute(Attribute::Bold),
        ]);

    if shadow_unlocked {
        table.add_row(vec![
            Cell::new("Vault"),
            Cell::new("🔓 UNLOCKED (PAGI_SHADOW_KEY set)")
                .fg(Color::Green)
                .add_attribute(Attribute::Bold),
        ]);
        table.add_row(vec![
            Cell::new("Encryption"),
            Cell::new("AES-256-GCM active"),
        ]);
    } else {
        table.add_row(vec![
            Cell::new("Vault"),
            Cell::new("🔒 LOCKED")
                .fg(Color::Yellow)
                .add_attribute(Attribute::Bold),
        ]);
        table.add_row(vec![
            Cell::new("Encryption"),
            Cell::new("Set PAGI_SHADOW_KEY to unlock"),
        ]);
    }

    if let Some(shadow_status) = state.kb_statuses.iter().find(|s| s.slot_id == KbType::Shadow.slot_id()) {
        table.add_row(vec![
            Cell::new("Entries"),
            Cell::new(format!("{}", shadow_status.entry_count)),
        ]);
    }

    println!("{table}");
    println!();
}

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Renders a small ASCII trust bar: `[████░░░░░░]` for a 0.0–1.0 score.
fn trust_bar_ascii(score: f32) -> String {
    let filled = (score * 10.0).round() as usize;
    let empty = 10_usize.saturating_sub(filled);
    format!("[{}{}]", "█".repeat(filled), "░".repeat(empty))
}
