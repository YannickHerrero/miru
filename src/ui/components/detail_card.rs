use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

use crate::api::{Episode, Media, MediaType};
use crate::ui::theme::{Theme, STAR};

/// Detail card component for displaying media information
pub struct DetailCard;

impl DetailCard {
    /// Render the detail card for a media item
    pub fn render(frame: &mut Frame, area: Rect, media: &Media, theme: &Theme) {
        Self::render_with_episode(frame, area, media, None, theme);
    }

    /// Render the detail card for a media item with optional episode details
    pub fn render_with_episode(
        frame: &mut Frame,
        area: Rect,
        media: &Media,
        episode: Option<&Episode>,
        theme: &Theme,
    ) {
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
        let width = inner.width as usize;

        // Episode-specific info (shown above show info when an episode is selected)
        if let Some(ep) = episode {
            // Episode title
            let ep_title = format!("{}. {}", ep.number, ep.title);
            lines.push(Line::from(Span::styled(
                truncate_str(&ep_title, width),
                theme.highlight(),
            )));

            // Episode metadata line: air date, runtime, rating
            let mut ep_meta: Vec<Span> = Vec::new();

            if let Some(ref air_date) = ep.air_date {
                ep_meta.push(Span::styled(format_air_date(air_date), theme.normal()));
            }

            if let Some(runtime) = ep.runtime {
                if !ep_meta.is_empty() {
                    ep_meta.push(Span::styled("  ", theme.normal()));
                }
                ep_meta.push(Span::styled(format!("{} min", runtime), theme.muted()));
            }

            if let Some(score) = ep.vote_average {
                if !ep_meta.is_empty() {
                    ep_meta.push(Span::styled("  ", theme.normal()));
                }
                ep_meta.push(Span::styled(
                    format!("{} {:.1}", STAR, score),
                    theme.warning(),
                ));
            }

            if !ep_meta.is_empty() {
                lines.push(Line::from(ep_meta));
            }

            // Episode overview/synopsis
            if let Some(ref overview) = ep.overview {
                if !overview.is_empty() {
                    lines.push(Line::from("")); // Spacer

                    // Reserve space for show info below (title + type/status + year/score + genres = ~6 lines)
                    let reserved_for_show = 7;
                    let used_lines = lines.len();
                    let available_height = inner
                        .height
                        .saturating_sub(used_lines as u16 + reserved_for_show + 1);

                    if available_height > 0 {
                        let wrapped = wrap_text(overview, width);
                        let max_lines = available_height as usize;

                        for (i, line) in wrapped.iter().enumerate() {
                            if i >= max_lines {
                                if let Some(last) = lines.last_mut() {
                                    *last = Line::from(Span::styled(
                                        truncate_str(&format!("{}...", line), width),
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

            // Separator between episode and show info
            lines.push(Line::from("")); // Spacer
            let separator = "─".repeat(width.min(40));
            lines.push(Line::from(Span::styled(separator, theme.muted())));
            lines.push(Line::from("")); // Spacer
        }

        // Show-level info (always shown)

        // Title
        lines.push(Line::from(Span::styled(
            truncate_str(&media.title, width),
            theme.highlight(),
        )));

        // Original title (if different)
        if let Some(ref original) = media.title_original {
            if original != &media.title {
                lines.push(Line::from(Span::styled(
                    truncate_str(original, width),
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
                truncate_str(&genres_str, width),
                theme.muted(),
            )));
        }

        // Description (only show when no episode is selected, to save space)
        if episode.is_none() {
            if let Some(ref desc) = media.description {
                if !desc.is_empty() {
                    lines.push(Line::from("")); // Spacer

                    // Calculate available height for description
                    let used_lines = lines.len();
                    let available_height = inner.height.saturating_sub(used_lines as u16 + 1);

                    if available_height > 0 {
                        // Wrap text to fit width
                        let wrapped = wrap_text(desc, width);
                        let max_lines = available_height as usize;

                        for (i, line) in wrapped.iter().enumerate() {
                            if i >= max_lines {
                                // Add ellipsis to last line if truncated
                                if let Some(last) = lines.last_mut() {
                                    *last = Line::from(Span::styled(
                                        truncate_str(&format!("{}...", line), width),
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

/// Format an air date string from "YYYY-MM-DD" to a more readable format
fn format_air_date(date: &str) -> String {
    let parts: Vec<&str> = date.split('-').collect();
    if parts.len() == 3 {
        let month = match parts[1] {
            "01" => "Jan",
            "02" => "Feb",
            "03" => "Mar",
            "04" => "Apr",
            "05" => "May",
            "06" => "Jun",
            "07" => "Jul",
            "08" => "Aug",
            "09" => "Sep",
            "10" => "Oct",
            "11" => "Nov",
            "12" => "Dec",
            _ => parts[1],
        };
        // Remove leading zero from day
        let day = parts[2].trim_start_matches('0');
        format!("{} {} {}", month, day, parts[0])
    } else {
        date.to_string()
    }
}
