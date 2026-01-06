use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

use crate::api::{Media, MediaType};
use crate::ui::theme::{Theme, STAR};

/// Detail card component for displaying media information
pub struct DetailCard;

impl DetailCard {
    /// Render the detail card for a media item
    pub fn render(frame: &mut Frame, area: Rect, media: &Media, theme: &Theme) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(theme.border())
            .title(Span::styled(" Details ", theme.title()));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        if inner.width < 10 || inner.height < 5 {
            return; // Too small to render anything meaningful
        }

        let mut lines: Vec<Line> = Vec::new();

        // Title
        lines.push(Line::from(Span::styled(
            truncate_str(&media.title, inner.width as usize),
            theme.highlight(),
        )));

        // Original title (if different)
        if let Some(ref original) = media.title_original {
            if original != &media.title {
                lines.push(Line::from(Span::styled(
                    truncate_str(original, inner.width as usize),
                    theme.muted(),
                )));
            }
        }

        lines.push(Line::from("")); // Spacer

        // Type badge with color + Format + Status
        let type_style = match media.media_type {
            MediaType::Movie => theme.accent(),
            MediaType::TvShow => theme.info(),
        };

        let mut meta_spans = vec![Span::styled(
            format!("[{}]", media.media_type.label()),
            type_style,
        )];

        if let Some(ref format) = media.format {
            // Don't show format if it's the same as the type label
            let type_label = media.media_type.label();
            if format != type_label && format != "TV" {
                meta_spans.push(Span::styled(format!("  {}", format), theme.muted()));
            }
        }

        if let Some(ref status) = media.status {
            meta_spans.push(Span::styled(format!("  {}", status), theme.muted()));
        }

        lines.push(Line::from(meta_spans));

        // Year and Score
        let mut info_spans: Vec<Span> = Vec::new();

        if let Some(year) = media.year {
            info_spans.push(Span::styled(format!("{}", year), theme.normal()));
        }

        if let Some(score) = media.score {
            if score > 0.0 {
                if !info_spans.is_empty() {
                    info_spans.push(Span::styled("  ", theme.normal()));
                }
                info_spans.push(Span::styled(
                    format!("{} {:.1}", STAR, score),
                    theme.warning(),
                ));
            }
        }

        if !info_spans.is_empty() {
            lines.push(Line::from(info_spans));
        }

        // Seasons for TV shows
        let mut count_info: Vec<Span> = Vec::new();
        if media.media_type == MediaType::TvShow {
            if let Some(seasons) = media.seasons {
                count_info.push(Span::styled(format!("{} seasons", seasons), theme.muted()));
            }
            if let Some(eps) = media.episodes {
                if !count_info.is_empty() {
                    count_info.push(Span::styled(" / ", theme.muted()));
                }
                count_info.push(Span::styled(format!("{} episodes", eps), theme.muted()));
            }
        }

        if !count_info.is_empty() {
            lines.push(Line::from(count_info));
        }

        // Genres
        if !media.genres.is_empty() {
            lines.push(Line::from("")); // Spacer
            let genres_str = media.genres.join(", ");
            lines.push(Line::from(Span::styled(
                truncate_str(&genres_str, inner.width as usize),
                theme.muted(),
            )));
        }

        // Description
        if let Some(ref desc) = media.description {
            if !desc.is_empty() {
                lines.push(Line::from("")); // Spacer

                // Calculate available height for description
                let used_lines = lines.len();
                let available_height = inner.height.saturating_sub(used_lines as u16 + 1);

                if available_height > 0 {
                    // Wrap text to fit width
                    let wrapped = wrap_text(desc, inner.width as usize);
                    let max_lines = available_height as usize;

                    for (i, line) in wrapped.iter().enumerate() {
                        if i >= max_lines {
                            // Add ellipsis to last line if truncated
                            if let Some(last) = lines.last_mut() {
                                *last = Line::from(Span::styled(
                                    truncate_str(&format!("{}...", line), inner.width as usize),
                                    theme.normal(),
                                ));
                            }
                            break;
                        }
                        lines.push(Line::from(Span::styled(line.clone(), theme.normal())));
                    }
                }
            }
        }

        let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false });
        frame.render_widget(paragraph, inner);
    }
}

/// Truncate a string to fit within a given width
fn truncate_str(s: &str, max_width: usize) -> String {
    if s.chars().count() <= max_width {
        s.to_string()
    } else if max_width > 3 {
        format!("{}...", s.chars().take(max_width - 3).collect::<String>())
    } else {
        s.chars().take(max_width).collect()
    }
}

/// Wrap text to fit within a given width
fn wrap_text(text: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current_line = String::new();
    let mut current_width = 0;

    for word in text.split_whitespace() {
        let word_len = word.chars().count();

        if current_width == 0 {
            // Start of a new line
            if word_len > width {
                // Word is longer than line width, truncate it
                lines.push(truncate_str(word, width));
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
                lines.push(truncate_str(word, width));
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
