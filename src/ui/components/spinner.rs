use std::time::{Duration, Instant};

use ratatui::{
    layout::{Alignment, Rect},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::ui::theme::{Theme, SPINNER_FRAMES};

/// Animated loading spinner
pub struct Spinner {
    /// Start time for animation and elapsed display
    start_time: Instant,
    /// Message to display alongside spinner
    message: String,
}

impl Spinner {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            start_time: Instant::now(),
            message: message.into(),
        }
    }

    /// Get the current spinner frame
    fn current_frame(&self) -> &'static str {
        let elapsed = self.start_time.elapsed().as_millis();
        let frame_index = (elapsed / 80) as usize % SPINNER_FRAMES.len();
        SPINNER_FRAMES[frame_index]
    }

    /// Get elapsed time as a formatted string
    fn elapsed_string(&self) -> Option<String> {
        let elapsed = self.start_time.elapsed();
        if elapsed >= Duration::from_secs(2) {
            Some(format!("{}s", elapsed.as_secs()))
        } else {
            None
        }
    }

    /// Render the spinner
    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let spinner_char = self.current_frame();

        let mut spans = vec![
            Span::styled(format!("{} ", spinner_char), theme.highlight()),
            Span::styled(&self.message, theme.normal()),
        ];

        if let Some(elapsed) = self.elapsed_string() {
            spans.push(Span::styled(format!(" ({})", elapsed), theme.muted()));
        }

        let line = Line::from(spans);
        let paragraph = Paragraph::new(line).alignment(Alignment::Center);

        frame.render_widget(paragraph, area);
    }

    /// Update the message
    #[allow(dead_code)]
    pub fn set_message(&mut self, message: impl Into<String>) {
        self.message = message.into();
    }
}
