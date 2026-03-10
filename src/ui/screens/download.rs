use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::ui::theme::Theme;

pub enum DownloadAction {
    Cancel,
    Back,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DownloadStatus {
    Running,
    Cancelling,
    Completed,
    Failed,
    Cancelled,
}

pub struct DownloadScreen {
    title: String,
    save_dir: String,
    status: DownloadStatus,
    progress_percent: Option<f64>,
    downloaded_bytes: u64,
    total_bytes: Option<u64>,
    speed_bytes: u64,
    peers: Option<usize>,
    message: String,
    file_path: Option<String>,
}

impl DownloadScreen {
    pub fn new(title: String, save_dir: String) -> Self {
        Self {
            title,
            save_dir,
            status: DownloadStatus::Running,
            progress_percent: None,
            downloaded_bytes: 0,
            total_bytes: None,
            speed_bytes: 0,
            peers: None,
            message: "Preparing download...".to_string(),
            file_path: None,
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Option<DownloadAction> {
        match self.status {
            DownloadStatus::Running | DownloadStatus::Cancelling => match key.code {
                KeyCode::Esc | KeyCode::Char('q') => Some(DownloadAction::Cancel),
                _ => None,
            },
            DownloadStatus::Completed | DownloadStatus::Failed | DownloadStatus::Cancelled => {
                match key.code {
                    KeyCode::Esc | KeyCode::Enter | KeyCode::Char('q') => {
                        Some(DownloadAction::Back)
                    }
                    _ => None,
                }
            }
        }
    }

    pub fn set_running(
        &mut self,
        downloaded_bytes: u64,
        total_bytes: Option<u64>,
        speed_bytes: u64,
        peers: Option<usize>,
        message: String,
    ) {
        self.status = DownloadStatus::Running;
        self.downloaded_bytes = downloaded_bytes;
        self.total_bytes = total_bytes;
        self.progress_percent = total_bytes.map(|total| {
            if total == 0 {
                0.0
            } else {
                (downloaded_bytes as f64 / total as f64) * 100.0
            }
        });
        self.speed_bytes = speed_bytes;
        self.peers = peers;
        self.message = message;
    }

    pub fn set_cancelling(&mut self) {
        self.status = DownloadStatus::Cancelling;
        self.message = "Cancelling download...".to_string();
    }

    pub fn set_completed(&mut self, path: String) {
        self.status = DownloadStatus::Completed;
        self.message = "Download completed".to_string();
        self.file_path = Some(path);
    }

    pub fn set_failed(&mut self, error: String) {
        self.status = DownloadStatus::Failed;
        self.message = error;
    }

    pub fn set_cancelled(&mut self, msg: String) {
        self.status = DownloadStatus::Cancelled;
        self.message = msg;
    }

    pub fn is_finished(&self) -> bool {
        matches!(
            self.status,
            DownloadStatus::Completed | DownloadStatus::Failed | DownloadStatus::Cancelled
        )
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2),
                Constraint::Length(2),
                Constraint::Length(1),
                Constraint::Length(8),
                Constraint::Length(2),
                Constraint::Min(0),
            ])
            .margin(1)
            .split(area);

        let title = Paragraph::new(Line::from(vec![Span::styled(
            format!("Download: {}", self.title),
            theme.title(),
        )]));
        frame.render_widget(title, chunks[0]);

        let status_label = match self.status {
            DownloadStatus::Running => "Running",
            DownloadStatus::Cancelling => "Cancelling",
            DownloadStatus::Completed => "Completed",
            DownloadStatus::Failed => "Failed",
            DownloadStatus::Cancelled => "Cancelled",
        };
        let status_line = Paragraph::new(Line::from(vec![
            Span::styled("Status: ", theme.muted()),
            Span::styled(status_label, theme.highlight()),
        ]));
        frame.render_widget(status_line, chunks[1]);

        let mut details = vec![];
        details.push(Line::from(vec![Span::styled(
            format!("Save dir: {}", self.save_dir),
            theme.normal(),
        )]));

        if let Some(percent) = self.progress_percent {
            details.push(Line::from(vec![Span::styled(
                format!("Progress: {:.1}%", percent.min(100.0)),
                theme.normal(),
            )]));
        }

        let total_text = self
            .total_bytes
            .map(format_bytes)
            .unwrap_or_else(|| "unknown".to_string());
        details.push(Line::from(vec![Span::styled(
            format!(
                "Downloaded: {} / {}",
                format_bytes(self.downloaded_bytes),
                total_text
            ),
            theme.normal(),
        )]));

        if self.speed_bytes > 0 {
            details.push(Line::from(vec![Span::styled(
                format!("Speed: {}/s", format_bytes(self.speed_bytes)),
                theme.normal(),
            )]));
        }

        if let Some(peers) = self.peers {
            details.push(Line::from(vec![Span::styled(
                format!("Peers: {}", peers),
                theme.normal(),
            )]));
        }

        details.push(Line::from(vec![Span::styled(
            format!("Message: {}", self.message),
            theme.normal(),
        )]));

        if let Some(path) = &self.file_path {
            details.push(Line::from(vec![Span::styled(
                format!("Saved to: {}", path),
                theme.success(),
            )]));
        }

        frame.render_widget(Paragraph::new(details), chunks[3]);

        let help = if self.is_finished() {
            Line::from(vec![
                Span::styled("Enter/Esc", theme.highlight()),
                Span::styled(" back", theme.muted()),
            ])
        } else {
            Line::from(vec![
                Span::styled("Esc", theme.highlight()),
                Span::styled(" cancel", theme.muted()),
            ])
        };

        let help_widget = Paragraph::new(help).alignment(Alignment::Left);
        frame.render_widget(help_widget, chunks[4]);
    }
}

fn format_bytes(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;

    let b = bytes as f64;
    if b >= GB {
        format!("{:.2} GB", b / GB)
    } else if b >= MB {
        format!("{:.1} MB", b / MB)
    } else if b >= KB {
        format!("{:.1} KB", b / KB)
    } else {
        format!("{} B", bytes)
    }
}
