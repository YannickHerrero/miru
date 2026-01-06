use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::api::Stream;
use crate::ui::theme::Theme;

/// Detail card component for displaying stream/torrent information
pub struct StreamDetailCard;

/// Wrap text to fit within a given width
fn wrap_text(text: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return vec![];
    }

    let mut lines = Vec::new();
    let mut current_line = String::new();
    let mut current_width = 0;

    for word in text.split_whitespace() {
        let word_len = word.chars().count();

        if current_width == 0 {
            // Start of a new line
            if word_len > width {
                // Word is longer than line width, truncate it
                lines.push(format!(
                    "{}...",
                    word.chars()
                        .take(width.saturating_sub(3))
                        .collect::<String>()
                ));
            } else {
                current_line = word.to_string();
                current_width = word_len;
            }
        } else if current_width + 1 + word_len <= width {
            // Word fits on current line
            current_line.push(' ');
            current_line.push_str(word);
            current_width += 1 + word_len;
        } else {
            // Word doesn't fit, start new line
            lines.push(current_line);
            if word_len > width {
                lines.push(format!(
                    "{}...",
                    word.chars()
                        .take(width.saturating_sub(3))
                        .collect::<String>()
                ));
                current_line = String::new();
                current_width = 0;
            } else {
                current_line = word.to_string();
                current_width = word_len;
            }
        }
    }

    // Don't forget the last line
    if !current_line.is_empty() {
        lines.push(current_line);
    }

    lines
}

impl StreamDetailCard {
    /// Render the detail card for a stream
    pub fn render(frame: &mut Frame, area: Rect, stream: &Stream, theme: &Theme) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(theme.border())
            .title(Span::styled(" Source Details ", theme.title()));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        if inner.width < 10 || inner.height < 5 {
            return; // Too small to render anything meaningful
        }

        let mut lines: Vec<Line> = Vec::new();

        // Provider header
        lines.push(Line::from(Span::styled(
            format!("[{}]", stream.provider),
            theme.highlight(),
        )));

        lines.push(Line::from("")); // Spacer

        // Quality + HDR row
        let mut quality_spans: Vec<Span> = Vec::new();
        if let Some(ref quality) = stream.quality {
            quality_spans.push(Span::styled(quality.clone(), theme.highlight()));
        }
        if let Some(ref hdr) = stream.hdr {
            if !quality_spans.is_empty() {
                quality_spans.push(Span::styled("  ", theme.normal()));
            }
            quality_spans.push(Span::styled(hdr.clone(), theme.warning()));
        }
        if !quality_spans.is_empty() {
            lines.push(Line::from(quality_spans));
        }

        // Video codec row
        if let Some(ref codec) = stream.video_codec {
            lines.push(Line::from(vec![
                Span::styled("Video: ", theme.muted()),
                Span::styled(codec.clone(), theme.normal()),
            ]));
        }

        // Audio row
        if let Some(ref audio) = stream.audio {
            lines.push(Line::from(vec![
                Span::styled("Audio: ", theme.muted()),
                Span::styled(audio.clone(), theme.normal()),
            ]));
        }

        // Source type row
        if let Some(ref source) = stream.source_type {
            lines.push(Line::from(vec![
                Span::styled("Source: ", theme.muted()),
                Span::styled(source.clone(), theme.normal()),
            ]));
        }

        // Languages row (with wrapping)
        if !stream.languages.is_empty() {
            lines.push(Line::from(Span::styled("Languages:", theme.muted())));

            let languages_str = stream.languages.join(", ");
            let max_width = inner.width.saturating_sub(2) as usize; // Leave some padding

            // Wrap the languages text
            for line in wrap_text(&languages_str, max_width) {
                lines.push(Line::from(Span::styled(
                    format!("  {}", line),
                    theme.normal(),
                )));
            }
        }

        lines.push(Line::from("")); // Spacer

        // Size and seeders row
        let mut info_spans: Vec<Span> = Vec::new();
        if let Some(ref size) = stream.size {
            info_spans.push(Span::styled(format!("Size: {}", size), theme.muted()));
        }
        if let Some(seeders) = stream.seeders {
            if !info_spans.is_empty() {
                info_spans.push(Span::styled("  ", theme.normal()));
            }
            info_spans.push(Span::styled(format!("Seeders: {}", seeders), theme.muted()));
        }
        if !info_spans.is_empty() {
            lines.push(Line::from(info_spans));
        }

        let paragraph = Paragraph::new(lines);
        frame.render_widget(paragraph, inner);
    }
}
