use std::io::{self, Stdout};
use std::time::Duration;

use anyhow::Context;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, Paragraph},
    Frame, Terminal,
};

use crate::api::{RealDebridClient, TmdbClient};
use crate::config::{config_path, save_config, Config};
use crate::error::Result;
use crate::ui::components::{Input, Spinner};
use crate::ui::theme::Theme;

const ASCII_ART: &str = r#"
          _              
  _ __ ___ (_)_ __ _   _  
 | '_ ` _ \| | '__| | | | 
 | | | | | | | |  | |_| | 
 |_| |_| |_|_|_|   \__,_| 
"#;

/// Wizard step enum
#[derive(Clone, PartialEq)]
enum Step {
    Welcome,
    RealDebrid,
    RealDebridValidating,
    Tmdb,
    TmdbValidating,
    Complete,
}

impl Step {
    fn index(&self) -> usize {
        match self {
            Step::Welcome => 0,
            Step::RealDebrid | Step::RealDebridValidating => 1,
            Step::Tmdb | Step::TmdbValidating => 2,
            Step::Complete => 3,
        }
    }

    fn total() -> usize {
        4
    }

    fn title(&self) -> &'static str {
        match self {
            Step::Welcome => "Welcome",
            Step::RealDebrid | Step::RealDebridValidating => "Real-Debrid (Optional)",
            Step::Tmdb | Step::TmdbValidating => "TMDB (Required)",
            Step::Complete => "Setup Complete",
        }
    }
}

/// Validation result for API keys
enum ValidationResult {
    None,
    Validating,
    Success(String), // Success message
    Error(String),   // Error message
}

/// Init wizard application
pub struct InitWizard {
    step: Step,
    theme: Theme,
    should_quit: bool,

    // MPV detection
    mpv_installed: bool,

    // Real-Debrid
    rd_input: Input,
    rd_validation: ValidationResult,
    rd_api_key: String,
    rd_username: Option<String>,

    // TMDB
    tmdb_input: Input,
    tmdb_validation: ValidationResult,
    tmdb_api_key: String,

    // Spinner for validation
    spinner: Option<Spinner>,
}

impl InitWizard {
    pub fn new(_config_exists: bool) -> Self {
        let mpv_installed = which::which("mpv").is_ok();

        // Theme::default() uses "auto" mode which will detect terminal background
        Self {
            step: Step::Welcome,
            theme: Theme::default(),
            should_quit: false,
            mpv_installed,
            rd_input: Input::new(),
            rd_validation: ValidationResult::None,
            rd_api_key: String::new(),
            rd_username: None,
            tmdb_input: Input::new(),
            tmdb_validation: ValidationResult::None,
            tmdb_api_key: String::new(),
            spinner: None,
        }
    }

    /// Run the wizard
    pub async fn run(&mut self) -> Result<bool> {
        let mut terminal = self.setup_terminal()?;

        let result = self.run_loop(&mut terminal).await;

        self.restore_terminal(&mut terminal)?;

        result
    }

    fn setup_terminal(&self) -> Result<Terminal<CrosstermBackend<Stdout>>> {
        enable_raw_mode().context("Failed to enable raw mode")?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen).context("Failed to enter alternate screen")?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend).context("Failed to create terminal")?;
        Ok(terminal)
    }

    fn restore_terminal(&self, terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
        disable_raw_mode().context("Failed to disable raw mode")?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen)
            .context("Failed to leave alternate screen")?;
        terminal.show_cursor().context("Failed to show cursor")?;
        Ok(())
    }

    async fn run_loop(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    ) -> Result<bool> {
        loop {
            terminal.draw(|f| self.render(f))?;

            // Handle validation in progress
            if matches!(self.step, Step::RealDebridValidating | Step::TmdbValidating) {
                self.handle_validation().await;
                continue;
            }

            if self.should_quit {
                return Ok(false);
            }

            if self.step == Step::Complete {
                // Wait for final keypress then exit
                if event::poll(Duration::from_millis(100))? {
                    if let Event::Key(key) = event::read()? {
                        if key.kind == KeyEventKind::Press {
                            return Ok(true);
                        }
                    }
                }
                continue;
            }

            // Poll for events with timeout for spinner animation
            if event::poll(Duration::from_millis(50))? {
                if let Event::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press {
                        self.handle_key(key.code);
                    }
                }
            }
        }
    }

    async fn handle_validation(&mut self) {
        match &self.step {
            Step::RealDebridValidating => {
                let key = self.rd_input.get_value().to_string();
                if key.is_empty() {
                    // Skip validation for empty key (user skipped)
                    self.rd_api_key = String::new();
                    self.rd_validation =
                        ValidationResult::Success("Using direct P2P streaming".to_string());
                    self.step = Step::Tmdb;
                    self.spinner = None;
                } else {
                    let client = RealDebridClient::new(key.clone());
                    match client.validate_key().await {
                        Ok(user) => {
                            self.rd_api_key = key;
                            self.rd_username = Some(user.username.clone());
                            self.rd_validation = ValidationResult::Success(format!(
                                "Logged in as: {}",
                                user.username
                            ));
                            self.step = Step::Tmdb;
                        }
                        Err(e) => {
                            self.rd_validation =
                                ValidationResult::Error(format!("Validation failed: {}", e));
                            self.step = Step::RealDebrid;
                        }
                    }
                    self.spinner = None;
                }
            }
            Step::TmdbValidating => {
                let key = self.tmdb_input.get_value().to_string();
                let client = TmdbClient::new(key.clone());
                match client.search_all("test").await {
                    Ok(_) => {
                        self.tmdb_api_key = key;
                        self.tmdb_validation =
                            ValidationResult::Success("TMDB configured successfully".to_string());

                        // Save config
                        let config =
                            Config::new(self.rd_api_key.clone(), self.tmdb_api_key.clone());
                        if let Err(e) = save_config(&config) {
                            self.tmdb_validation =
                                ValidationResult::Error(format!("Failed to save config: {}", e));
                            self.step = Step::Tmdb;
                        } else {
                            self.step = Step::Complete;
                        }
                    }
                    Err(e) => {
                        self.tmdb_validation =
                            ValidationResult::Error(format!("Validation failed: {}", e));
                        self.step = Step::Tmdb;
                    }
                }
                self.spinner = None;
            }
            _ => {}
        }
    }

    fn handle_key(&mut self, key: KeyCode) {
        match &self.step {
            Step::Welcome => self.handle_welcome_key(key),
            Step::RealDebrid => self.handle_rd_key(key),
            Step::Tmdb => self.handle_tmdb_key(key),
            _ => {}
        }
    }

    fn handle_welcome_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Enter => {
                self.step = Step::RealDebrid;
            }
            KeyCode::Esc | KeyCode::Char('q') => {
                self.should_quit = true;
            }
            _ => {}
        }
    }

    fn handle_rd_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Enter => {
                self.spinner = Some(Spinner::new("Validating..."));
                self.rd_validation = ValidationResult::Validating;
                self.step = Step::RealDebridValidating;
            }
            KeyCode::Esc => {
                self.step = Step::Welcome;
                self.rd_validation = ValidationResult::None;
            }
            KeyCode::Backspace => {
                self.rd_input.backspace();
                self.rd_validation = ValidationResult::None;
            }
            KeyCode::Delete => {
                self.rd_input.delete();
                self.rd_validation = ValidationResult::None;
            }
            KeyCode::Left => {
                self.rd_input.move_left();
            }
            KeyCode::Right => {
                self.rd_input.move_right();
            }
            KeyCode::Home => {
                self.rd_input.move_start();
            }
            KeyCode::End => {
                self.rd_input.move_end();
            }
            KeyCode::Char(c) => {
                self.rd_input.insert(c);
                self.rd_validation = ValidationResult::None;
            }
            _ => {}
        }
    }

    fn handle_tmdb_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Enter => {
                if self.tmdb_input.get_value().is_empty() {
                    self.tmdb_validation =
                        ValidationResult::Error("API key is required".to_string());
                } else {
                    self.spinner = Some(Spinner::new("Validating..."));
                    self.tmdb_validation = ValidationResult::Validating;
                    self.step = Step::TmdbValidating;
                }
            }
            KeyCode::Esc => {
                self.step = Step::RealDebrid;
                self.tmdb_validation = ValidationResult::None;
            }
            KeyCode::Backspace => {
                self.tmdb_input.backspace();
                self.tmdb_validation = ValidationResult::None;
            }
            KeyCode::Delete => {
                self.tmdb_input.delete();
                self.tmdb_validation = ValidationResult::None;
            }
            KeyCode::Left => {
                self.tmdb_input.move_left();
            }
            KeyCode::Right => {
                self.tmdb_input.move_right();
            }
            KeyCode::Home => {
                self.tmdb_input.move_start();
            }
            KeyCode::End => {
                self.tmdb_input.move_end();
            }
            KeyCode::Char(c) => {
                self.tmdb_input.insert(c);
                self.tmdb_validation = ValidationResult::None;
            }
            _ => {}
        }
    }

    fn render(&mut self, frame: &mut Frame) {
        let area = frame.area();

        // Create centered box
        let outer_block = Block::default()
            .borders(Borders::ALL)
            .border_style(self.theme.border())
            .title(Span::styled(" miru setup ", self.theme.title()));

        let inner_area = self.centered_rect(70, 80, area);
        frame.render_widget(outer_block.clone(), inner_area);

        let content_area = outer_block.inner(inner_area);

        // Layout: progress bar, title, content, help
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2), // Progress bar
                Constraint::Length(8), // ASCII art / title
                Constraint::Min(10),   // Content
                Constraint::Length(2), // Help text
            ])
            .margin(1)
            .split(content_area);

        self.render_progress_bar(frame, chunks[0]);
        self.render_header(frame, chunks[1]);

        match &self.step {
            Step::Welcome => self.render_welcome(frame, chunks[2]),
            Step::RealDebrid => self.render_real_debrid(frame, chunks[2]),
            Step::RealDebridValidating => self.render_validating(frame, chunks[2]),
            Step::Tmdb => self.render_tmdb(frame, chunks[2]),
            Step::TmdbValidating => self.render_validating(frame, chunks[2]),
            Step::Complete => self.render_complete(frame, chunks[2]),
        }

        self.render_help(frame, chunks[3]);
    }

    fn render_progress_bar(&self, frame: &mut Frame, area: Rect) {
        let progress = (self.step.index() as f64) / (Step::total() as f64 - 1.0);
        let label = format!(
            "Step {}/{}: {}",
            self.step.index() + 1,
            Step::total(),
            self.step.title()
        );

        let gauge = Gauge::default()
            .block(Block::default())
            .gauge_style(self.theme.highlight())
            .ratio(progress)
            .label(Span::styled(label, self.theme.normal()));

        frame.render_widget(gauge, area);
    }

    fn render_header(&self, frame: &mut Frame, area: Rect) {
        let text = if self.step == Step::Welcome {
            ASCII_ART.to_string()
        } else {
            format!("\n{}", self.step.title())
        };

        let paragraph = Paragraph::new(text)
            .style(self.theme.title())
            .alignment(Alignment::Center);

        frame.render_widget(paragraph, area);
    }

    fn render_welcome(&self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2), // Subtitle
                Constraint::Length(1), // Spacer
                Constraint::Min(6),    // Checklist
            ])
            .split(area);

        // Subtitle
        let subtitle = Paragraph::new("Before you start, make sure you have the following:")
            .style(self.theme.normal())
            .alignment(Alignment::Center);
        frame.render_widget(subtitle, chunks[0]);

        // Checklist
        let mpv_status = if self.mpv_installed {
            Line::from(vec![
                Span::styled("  [", self.theme.muted()),
                Span::styled("x", self.theme.info()),
                Span::styled("] ", self.theme.muted()),
                Span::styled("MPV media player ", self.theme.normal()),
                Span::styled("(installed)", self.theme.info()),
            ])
        } else {
            Line::from(vec![
                Span::styled("  [ ] ", self.theme.muted()),
                Span::styled("MPV media player ", self.theme.normal()),
                Span::styled("(NOT FOUND)", self.theme.error()),
            ])
        };

        let mpv_link = if !self.mpv_installed {
            Line::from(vec![
                Span::styled("      Install from: ", self.theme.muted()),
                Span::styled("https://mpv.io/installation/", self.theme.highlight()),
            ])
        } else {
            Line::from("")
        };

        let tmdb_line = Line::from(vec![
            Span::styled("  [ ] ", self.theme.muted()),
            Span::styled("TMDB API key ", self.theme.normal()),
            Span::styled("(required, free)", self.theme.warning()),
        ]);
        let tmdb_link = Line::from(vec![
            Span::styled("      Get yours at: ", self.theme.muted()),
            Span::styled(
                "https://www.themoviedb.org/settings/api",
                self.theme.highlight(),
            ),
        ]);

        let rd_line = Line::from(vec![
            Span::styled("  [ ] ", self.theme.muted()),
            Span::styled("Real-Debrid API key ", self.theme.normal()),
            Span::styled("(optional, paid)", self.theme.muted()),
        ]);
        let rd_link = Line::from(vec![
            Span::styled("      Sign up at: ", self.theme.muted()),
            Span::styled(
                "http://real-debrid.com/?id=16544328",
                self.theme.highlight(),
            ),
        ]);

        let mut lines = vec![
            mpv_status,
            mpv_link,
            Line::from(""),
            tmdb_line,
            tmdb_link,
            Line::from(""),
            rd_line,
            rd_link,
        ];

        if !self.mpv_installed {
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled("  WARNING: ", self.theme.warning()),
                Span::styled(
                    "MPV is not installed. You won't be able to play videos.",
                    self.theme.normal(),
                ),
            ]));
        }

        let checklist = Paragraph::new(lines);
        frame.render_widget(checklist, chunks[2]);
    }

    fn render_real_debrid(&mut self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Description
                Constraint::Length(1), // Spacer
                Constraint::Length(2), // Link
                Constraint::Length(1), // Spacer
                Constraint::Length(3), // Input
                Constraint::Length(2), // Validation message
                Constraint::Min(0),    // Spacer
            ])
            .split(area);

        // Description
        let desc = Paragraph::new(vec![
            Line::from("Real-Debrid provides faster cached streaming for popular content."),
            Line::from("Without it, miru uses direct P2P streaming (free, but may buffer)."),
        ])
        .style(self.theme.normal())
        .alignment(Alignment::Center);
        frame.render_widget(desc, chunks[0]);

        // Link
        let link = Paragraph::new(vec![Line::from(vec![
            Span::styled("Get your API key at: ", self.theme.muted()),
            Span::styled("https://real-debrid.com/apitoken", self.theme.highlight()),
        ])])
        .alignment(Alignment::Center);
        frame.render_widget(link, chunks[2]);

        // Input
        let input_area = self.centered_rect(60, 100, chunks[4]);
        self.rd_input
            .render(frame, input_area, " API Key (Enter to skip) ", &self.theme);

        // Validation message
        self.render_validation_message(frame, chunks[5], &self.rd_validation);
    }

    fn render_tmdb(&mut self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2), // Description
                Constraint::Length(1), // Spacer
                Constraint::Length(3), // Links
                Constraint::Length(1), // Spacer
                Constraint::Length(3), // Input
                Constraint::Length(2), // Validation message
                Constraint::Min(0),    // Spacer
            ])
            .split(area);

        // Description
        let desc = Paragraph::new("TMDB is required to search for movies, TV shows, and anime.")
            .style(self.theme.normal())
            .alignment(Alignment::Center);
        frame.render_widget(desc, chunks[0]);

        // Links
        let links = Paragraph::new(vec![
            Line::from(vec![
                Span::styled("Get your API key at: ", self.theme.muted()),
                Span::styled(
                    "https://www.themoviedb.org/settings/api",
                    self.theme.highlight(),
                ),
            ]),
            Line::from(vec![Span::styled(
                "Use the \"API Key (v3 auth)\", not the Read Access Token.",
                self.theme.muted(),
            )]),
        ])
        .alignment(Alignment::Center);
        frame.render_widget(links, chunks[2]);

        // Input
        let input_area = self.centered_rect(60, 100, chunks[4]);
        self.tmdb_input
            .render(frame, input_area, " API Key ", &self.theme);

        // Validation message
        self.render_validation_message(frame, chunks[5], &self.tmdb_validation);
    }

    fn render_validating(&self, frame: &mut Frame, area: Rect) {
        if let Some(spinner) = &self.spinner {
            let centered = self.centered_rect(50, 20, area);
            spinner.render(frame, centered, &self.theme);
        }
    }

    fn render_complete(&self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Success message
                Constraint::Length(1), // Spacer
                Constraint::Length(2), // Config path
                Constraint::Length(1), // Spacer
                Constraint::Min(0),    // Summary / warnings
            ])
            .split(area);

        // Success message
        let success = Paragraph::new(vec![Line::from(vec![
            Span::styled("✓ ", self.theme.info()),
            Span::styled("Setup complete!", self.theme.info()),
        ])])
        .alignment(Alignment::Center);
        frame.render_widget(success, chunks[0]);

        // Config path
        let config = Paragraph::new(vec![Line::from(vec![
            Span::styled("Configuration saved to: ", self.theme.muted()),
            Span::styled(config_path().display().to_string(), self.theme.normal()),
        ])])
        .alignment(Alignment::Center);
        frame.render_widget(config, chunks[2]);

        // Summary
        let mut summary_lines = vec![];

        // Real-Debrid status
        if let Some(username) = &self.rd_username {
            summary_lines.push(Line::from(vec![
                Span::styled("  Real-Debrid: ", self.theme.muted()),
                Span::styled(format!("✓ Logged in as {}", username), self.theme.info()),
            ]));
        } else {
            summary_lines.push(Line::from(vec![
                Span::styled("  Real-Debrid: ", self.theme.muted()),
                Span::styled("Using direct P2P streaming", self.theme.normal()),
            ]));
        }

        // TMDB status
        summary_lines.push(Line::from(vec![
            Span::styled("  TMDB: ", self.theme.muted()),
            Span::styled("✓ Configured", self.theme.info()),
        ]));

        // MPV warning
        if !self.mpv_installed {
            summary_lines.push(Line::from(""));
            summary_lines.push(Line::from(vec![
                Span::styled("  ⚠ ", self.theme.warning()),
                Span::styled(
                    "MPV was not found. Configure the player path:",
                    self.theme.warning(),
                ),
            ]));
            summary_lines.push(Line::from(vec![Span::styled(
                "    miru config --set player_command=<path>",
                self.theme.muted(),
            )]));
        }

        let summary = Paragraph::new(summary_lines);
        frame.render_widget(summary, chunks[4]);
    }

    fn render_validation_message(
        &self,
        frame: &mut Frame,
        area: Rect,
        validation: &ValidationResult,
    ) {
        let line = match validation {
            ValidationResult::None => return,
            ValidationResult::Validating => return,
            ValidationResult::Success(msg) => Line::from(vec![
                Span::styled("✓ ", self.theme.info()),
                Span::styled(msg, self.theme.info()),
            ]),
            ValidationResult::Error(msg) => Line::from(vec![
                Span::styled("✗ ", self.theme.error()),
                Span::styled(msg, self.theme.error()),
            ]),
        };

        let paragraph = Paragraph::new(line).alignment(Alignment::Center);
        frame.render_widget(paragraph, area);
    }

    fn render_help(&self, frame: &mut Frame, area: Rect) {
        let help = match &self.step {
            Step::Welcome => Line::from(vec![
                Span::styled("Enter", self.theme.highlight()),
                Span::styled(" continue • ", self.theme.muted()),
                Span::styled("Esc", self.theme.highlight()),
                Span::styled(" quit", self.theme.muted()),
            ]),
            Step::RealDebrid | Step::Tmdb => Line::from(vec![
                Span::styled("Enter", self.theme.highlight()),
                Span::styled(" submit • ", self.theme.muted()),
                Span::styled("Esc", self.theme.highlight()),
                Span::styled(" back", self.theme.muted()),
            ]),
            Step::RealDebridValidating | Step::TmdbValidating => {
                Line::from(vec![Span::styled("Validating...", self.theme.muted())])
            }
            Step::Complete => Line::from(vec![
                Span::styled("Press any key", self.theme.highlight()),
                Span::styled(" to start using miru", self.theme.muted()),
            ]),
        };

        let paragraph = Paragraph::new(help).alignment(Alignment::Center);
        frame.render_widget(paragraph, area);
    }

    /// Create a centered rect
    fn centered_rect(&self, percent_x: u16, percent_y: u16, area: Rect) -> Rect {
        let popup_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Percentage(percent_y),
                Constraint::Percentage((100 - percent_y) / 2),
            ])
            .split(area);

        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ])
            .split(popup_layout[1])[1]
    }
}
