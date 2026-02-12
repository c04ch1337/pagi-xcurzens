//! Real-Time Live Mode Dashboard (TUI)
//!
//! Displays KB access patterns, streaming status, and voice activity in real-time
//! using ratatui. Launch with `--live --dashboard` flag.

#[cfg(feature = "tui-dashboard")]
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Gauge},
    Terminal, Frame,
};

#[cfg(feature = "tui-dashboard")]
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

use crate::knowledge_router::KbAccessLog;
use std::io;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Skill execution record for dashboard
#[derive(Clone, Debug)]
pub struct SkillExecutionRecord {
    pub skill_name: String,
    pub timestamp_ms: i64,
    pub success: bool,
    pub duration_ms: u64,
}

/// Dashboard state shared between live session and TUI
pub struct DashboardState {
    /// Recent KB access log
    pub kb_access_log: Arc<Mutex<Vec<KbAccessLog>>>,
    /// Current streaming status
    pub streaming_active: Arc<Mutex<bool>>,
    /// Voice activity status
    pub voice_active: Arc<Mutex<bool>>,
    /// Last user input
    pub last_user_input: Arc<Mutex<String>>,
    /// Last assistant response
    pub last_assistant_response: Arc<Mutex<String>>,
    /// KB slot access counts
    pub kb_slot_counts: Arc<Mutex<[u32; 9]>>,
    /// Skills queue size
    pub skills_queue_size: Arc<Mutex<usize>>,
    /// Recent skill executions
    pub skill_executions: Arc<Mutex<Vec<SkillExecutionRecord>>>,
}

impl Default for DashboardState {
    fn default() -> Self {
        Self {
            kb_access_log: Arc::new(Mutex::new(Vec::new())),
            streaming_active: Arc::new(Mutex::new(false)),
            voice_active: Arc::new(Mutex::new(false)),
            last_user_input: Arc::new(Mutex::new(String::new())),
            last_assistant_response: Arc::new(Mutex::new(String::new())),
            kb_slot_counts: Arc::new(Mutex::new([0; 9])),
            skills_queue_size: Arc::new(Mutex::new(0)),
            skill_executions: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

impl DashboardState {
    /// Update KB access log
    pub fn log_kb_access(&self, log: KbAccessLog) {
        if let Ok(mut access_log) = self.kb_access_log.lock() {
            access_log.push(log.clone());
            
            // Keep only last 50 entries
            if access_log.len() > 50 {
                access_log.drain(0..access_log.len() - 50);
            }
        }
        
        // Update slot counts
        if let Ok(mut counts) = self.kb_slot_counts.lock() {
            if log.slot_id >= 1 && log.slot_id <= 9 {
                counts[(log.slot_id - 1) as usize] += 1;
            }
        }
    }
    
    /// Set streaming status
    pub fn set_streaming(&self, active: bool) {
        if let Ok(mut status) = self.streaming_active.lock() {
            *status = active;
        }
    }
    
    /// Set voice activity status
    pub fn set_voice_active(&self, active: bool) {
        if let Ok(mut status) = self.voice_active.lock() {
            *status = active;
        }
    }
    
    /// Update last user input
    pub fn set_user_input(&self, input: String) {
        if let Ok(mut last) = self.last_user_input.lock() {
            *last = input;
        }
    }
    
    /// Update last assistant response
    pub fn set_assistant_response(&self, response: String) {
        if let Ok(mut last) = self.last_assistant_response.lock() {
            *last = response;
        }
    }
    
    /// Update skills queue size
    pub fn set_skills_queue_size(&self, size: usize) {
        if let Ok(mut queue_size) = self.skills_queue_size.lock() {
            *queue_size = size;
        }
    }
    
    /// Log skill execution
    pub fn log_skill_execution(&self, record: SkillExecutionRecord) {
        if let Ok(mut executions) = self.skill_executions.lock() {
            executions.push(record);
            
            // Keep only last 20 executions
            if executions.len() > 20 {
                executions.drain(0..executions.len() - 20);
            }
        }
    }
    
    /// Get skills queue size
    pub fn get_skills_queue_size(&self) -> usize {
        self.skills_queue_size.lock().map(|s| *s).unwrap_or(0)
    }
    
    /// Get recent skill executions
    pub fn get_skill_executions(&self) -> Vec<SkillExecutionRecord> {
        if let Ok(executions) = self.skill_executions.lock() {
            executions.clone()
        } else {
            Vec::new()
        }
    }
    
    /// PLACEHOLDER: This method needs to be completed based on the rest of the file
    fn _placeholder_for_continuation(&self) {
        if let Ok(mut last) = self.last_assistant_response.lock() {
            *last = response;
        }
    }
}

#[cfg(feature = "tui-dashboard")]
/// Run the TUI dashboard
pub fn run_dashboard(state: Arc<DashboardState>) -> Result<(), io::Error> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Run app
    let res = run_app(&mut terminal, state);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        eprintln!("Dashboard error: {:?}", err);
    }

    Ok(())
}

#[cfg(feature = "tui-dashboard")]
fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    state: Arc<DashboardState>,
) -> Result<(), io::Error> {
    let mut last_tick = Instant::now();
    let tick_rate = Duration::from_millis(250);

    loop {
        terminal.draw(|f| ui(f, &state))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if key.code == KeyCode::Char('q') {
                    return Ok(());
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
        }
    }
}

#[cfg(feature = "tui-dashboard")]
fn ui(f: &mut Frame, state: &DashboardState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Header
            Constraint::Length(5),  // Status
            Constraint::Length(12), // KB Slot Activity
            Constraint::Min(10),    // KB Access Log
            Constraint::Length(6),  // Conversation
        ])
        .split(f.area());

    // Header
    render_header(f, chunks[0]);

    // Status
    render_status(f, chunks[1], state);

    // KB Slot Activity
    render_kb_slots(f, chunks[2], state);

    // KB Access Log
    render_access_log(f, chunks[3], state);

    // Conversation
    render_conversation(f, chunks[4], state);
}

#[cfg(feature = "tui-dashboard")]
fn render_header(f: &mut Frame, area: Rect) {
    let header = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("🎙️ PHOENIX LIVE MODE DASHBOARD", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("Press 'q' to quit | Real-time KB monitoring", Style::default().fg(Color::Gray)),
        ]),
    ])
    .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::Cyan)));
    
    f.render_widget(header, area);
}

#[cfg(feature = "tui-dashboard")]
fn render_status(f: &mut Frame, area: Rect, state: &DashboardState) {
    let streaming = state.streaming_active.lock().map(|s| *s).unwrap_or(false);
    let voice = state.voice_active.lock().map(|v| *v).unwrap_or(false);
    
    let streaming_status = if streaming { "🟢 STREAMING" } else { "⚪ IDLE" };
    let voice_status = if voice { "🔴 SPEAKING" } else { "⚪ SILENT" };
    
    let status = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("Streaming: ", Style::default().fg(Color::White)),
            Span::styled(streaming_status, Style::default().fg(if streaming { Color::Green } else { Color::Gray })),
            Span::raw("  |  "),
            Span::styled("Voice: ", Style::default().fg(Color::White)),
            Span::styled(voice_status, Style::default().fg(if voice { Color::Red } else { Color::Gray })),
        ]),
    ])
    .block(Block::default().borders(Borders::ALL).title("Status").border_style(Style::default().fg(Color::White)));
    
    f.render_widget(status, area);
}

#[cfg(feature = "tui-dashboard")]
fn render_kb_slots(f: &mut Frame, area: Rect, state: &DashboardState) {
    let counts = state.kb_slot_counts.lock().map(|c| *c).unwrap_or([0; 9]);
    let max_count = counts.iter().max().copied().unwrap_or(1).max(1);
    
    let slot_names = [
        "KB-01 Pneuma (Identity)",
        "KB-02 Oikos (Tasks)",
        "KB-03 Kardia (Relations)",
        "KB-04 Chronos (Time)",
        "KB-05 Techne (Protocols)",
        "KB-06 Ethos (Philosophy)",
        "KB-07 Soma (Physical)",
        "KB-08 Absurdity Log",
        "KB-09 Shadow (Encrypted)",
    ];
    
    let items: Vec<ListItem> = counts.iter().enumerate()
        .map(|(i, &count)| {
            let ratio = count as f64 / max_count as f64;
            let bar_width = (ratio * 20.0) as usize;
            let bar = "█".repeat(bar_width);
            
            let color = match i {
                0 => Color::Cyan,    // Identity
                1 => Color::Yellow,  // Tasks
                2 => Color::Magenta, // Relations
                3 => Color::Blue,    // Time
                4 => Color::Red,     // Protocols
                5 => Color::Green,   // Philosophy
                6 => Color::LightBlue, // Physical
                7 => Color::Gray,    // Absurdity
                8 => Color::DarkGray, // Shadow
                _ => Color::White,
            };
            
            ListItem::new(Line::from(vec![
                Span::styled(format!("{:<25}", slot_names[i]), Style::default().fg(color)),
                Span::styled(format!("{:>3} ", count), Style::default().fg(Color::White)),
                Span::styled(bar, Style::default().fg(color)),
            ]))
        })
        .collect();
    
    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("KB Slot Activity").border_style(Style::default().fg(Color::Yellow)));
    
    f.render_widget(list, area);
}

#[cfg(feature = "tui-dashboard")]
fn render_access_log(f: &mut Frame, area: Rect, state: &DashboardState) {
    let log = state.kb_access_log.lock().map(|l| l.clone()).unwrap_or_default();
    
    let items: Vec<ListItem> = log.iter().rev().take(10)
        .map(|entry| {
            let timestamp = chrono::DateTime::from_timestamp_millis(entry.timestamp_ms)
                .map(|dt| dt.format("%H:%M:%S").to_string())
                .unwrap_or_else(|| "??:??:??".to_string());
            
            let status_icon = if entry.success { "✓" } else { "✗" };
            let status_color = if entry.success { Color::Green } else { Color::Red };
            
            ListItem::new(Line::from(vec![
                Span::styled(format!("[{}] ", timestamp), Style::default().fg(Color::Gray)),
                Span::styled(status_icon, Style::default().fg(status_color)),
                Span::raw(" "),
                Span::styled(format!("{:<25}", entry.slot_name), Style::default().fg(Color::Cyan)),
                Span::styled(format!("({})", entry.intent), Style::default().fg(Color::Yellow)),
            ]))
        })
        .collect();
    
    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("KB Access Log (Recent 10)").border_style(Style::default().fg(Color::Green)));
    
    f.render_widget(list, area);
}

#[cfg(feature = "tui-dashboard")]
fn render_conversation(f: &mut Frame, area: Rect, state: &DashboardState) {
    let user_input = state.last_user_input.lock().map(|s| s.clone()).unwrap_or_default();
    let assistant_response = state.last_assistant_response.lock().map(|s| s.clone()).unwrap_or_default();
    
    let user_preview = if user_input.len() > 60 {
        format!("{}...", &user_input[..60])
    } else {
        user_input
    };
    
    let assistant_preview = if assistant_response.len() > 60 {
        format!("{}...", &assistant_response[..60])
    } else {
        assistant_response
    };
    
    let conversation = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("👤 User: ", Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD)),
            Span::raw(user_preview),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("🤖 Phoenix: ", Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)),
            Span::raw(assistant_preview),
        ]),
    ])
    .block(Block::default().borders(Borders::ALL).title("Conversation").border_style(Style::default().fg(Color::Magenta)));
    
    f.render_widget(conversation, area);
}

#[cfg(not(feature = "tui-dashboard"))]
pub fn run_dashboard(_state: Arc<DashboardState>) -> Result<(), io::Error> {
    eprintln!("TUI dashboard requires 'tui-dashboard' feature. Rebuild with: cargo build --features tui-dashboard");
    Ok(())
}
