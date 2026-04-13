use crate::{
    app::App,
    cli::version_text,
    theme::{app_theme, theme_preset_label, THEME_PRESETS},
};
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Padding, Paragraph},
    Frame,
};

use super::centered_rect;

pub(super) fn render_help_popup(f: &mut Frame) {
    let theme = app_theme();
    let area = centered_rect(56, 16, f.area());
    let section_style = Style::default()
        .fg(theme.ui.toc_primary_active)
        .add_modifier(Modifier::BOLD);
    let key_style = Style::default()
        .fg(theme.ui.toc_accent)
        .add_modifier(Modifier::BOLD);
    let text_style = Style::default().fg(theme.ui.toc_primary_inactive);
    let footer_style = Style::default().fg(theme.ui.status_shortcut_fg);
    let title_style = Style::default()
        .fg(theme.markdown.heading_2)
        .add_modifier(Modifier::BOLD);
    let lines = vec![
        Line::from(vec![Span::styled(version_text().to_string(), title_style)]),
        Line::from(vec![Span::styled(
            "Keyboard shortcuts",
            Style::default().fg(theme.ui.status_shortcut_fg),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Navigation                   Search",
            section_style,
        )]),
        Line::from(vec![
            Span::styled("j/k, ↑/↓   ", key_style),
            Span::styled("scroll", text_style),
            Span::raw("            "),
            Span::styled("/, Ctrl+F  ", key_style),
            Span::styled("search", text_style),
        ]),
        Line::from(vec![
            Span::styled("PgUp/PgDn  ", key_style),
            Span::styled("page", text_style),
            Span::raw("              "),
            Span::styled("n/N        ", key_style),
            Span::styled("next/prev", text_style),
        ]),
        Line::from(vec![
            Span::styled("g/G        ", key_style),
            Span::styled("top/bottom", text_style),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled("Actions", section_style)]),
        Line::from(vec![
            Span::styled("r          ", key_style),
            Span::styled("reload (watch)", text_style),
            Span::raw("    "),
            Span::styled("?          ", key_style),
            Span::styled("show help", text_style),
        ]),
        Line::from(vec![
            Span::styled("t          ", key_style),
            Span::styled("toggle toc", text_style),
            Span::raw("        "),
            Span::styled("q          ", key_style),
            Span::styled("quit", text_style),
        ]),
        Line::from(vec![
            Span::styled("T          ", key_style),
            Span::styled("theme picker", text_style),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled("Esc or ? to close", footer_style)]),
    ];

    f.render_widget(Clear, area);
    f.render_widget(
        Paragraph::new(lines).block(
            Block::default()
                .title("─ Help ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.ui.toc_border))
                .style(Style::default().bg(theme.ui.toc_bg))
                .padding(Padding::new(1, 1, 0, 0)),
        ),
        area,
    );
}

pub(super) fn render_theme_picker(f: &mut Frame, app: &App) {
    let theme = app_theme();
    let area = centered_rect(38, 10, f.area());
    let active = app.theme_picker_reference_preset();
    let footer_style = Style::default().fg(theme.ui.status_shortcut_fg);

    let mut lines = vec![
        Line::from(vec![Span::styled(
            "Choose a theme",
            Style::default().fg(theme.ui.status_shortcut_fg),
        )]),
        Line::from(""),
    ];
    for (idx, preset) in THEME_PRESETS.iter().enumerate() {
        let selected = idx == app.theme_picker_index();
        let is_active = *preset == active;
        let bg = if selected {
            theme.ui.toc_active_bg
        } else {
            theme.ui.toc_bg
        };
        let marker = if selected { "▸ " } else { "  " };
        let name = if is_active {
            format!("{}  ✓", theme_preset_label(*preset))
        } else {
            theme_preset_label(*preset).to_string()
        };
        lines.push(Line::from(vec![
            Span::styled(
                marker,
                Style::default()
                    .fg(theme.ui.toc_accent)
                    .bg(bg)
                    .add_modifier(if selected {
                        Modifier::BOLD
                    } else {
                        Modifier::empty()
                    }),
            ),
            Span::styled(
                name,
                Style::default()
                    .fg(if selected {
                        theme.ui.toc_primary_active
                    } else {
                        theme.ui.toc_primary_inactive
                    })
                    .bg(bg)
                    .add_modifier(if is_active || selected {
                        Modifier::BOLD
                    } else {
                        Modifier::empty()
                    }),
            ),
        ]));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(vec![Span::styled(
        "Enter keep • Esc restore",
        footer_style.bg(theme.ui.toc_bg),
    )]));

    f.render_widget(Clear, area);
    f.render_widget(
        Paragraph::new(lines).block(
            Block::default()
                .title("─ Theme ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.ui.toc_border))
                .style(Style::default().bg(theme.ui.toc_bg))
                .padding(Padding::new(1, 1, 0, 0)),
        ),
        area,
    );
}

pub(super) fn render_file_picker(f: &mut Frame, app: &App) {
    let theme = app_theme();
    let area = centered_rect(78, 20, f.area());
    let title_style = Style::default()
        .fg(theme.markdown.heading_2)
        .add_modifier(Modifier::BOLD);
    let section_style = Style::default().fg(theme.ui.status_shortcut_fg);
    let footer_style = Style::default().fg(theme.ui.status_shortcut_fg);
    let inner_height = area.height.saturating_sub(2) as usize;
    let header_lines = if app.is_fuzzy_file_picker() { 4 } else { 3 };
    let total = app.file_picker_filtered_indices().len();
    let truncation_message = picker_truncation_message(app.file_picker_truncation());
    let max_visible_slots = if app.is_fuzzy_file_picker() {
        if truncation_message.is_some() {
            11
        } else {
            12
        }
    } else {
        13
    };
    let reserved_footer_lines = if truncation_message.is_some() { 3 } else { 2 };
    let visible_slots = inner_height
        .saturating_sub(header_lines + reserved_footer_lines)
        .min(max_visible_slots);
    let start = if visible_slots == 0 || app.file_picker_index() < visible_slots {
        0
    } else {
        app.file_picker_index() + 1 - visible_slots
    };
    let end = (start + visible_slots).min(total);

    let mut lines = vec![
        Line::from(vec![Span::styled("Open a Markdown file", title_style)]),
        Line::from(vec![
            Span::styled("Dir: ", section_style),
            Span::styled(
                app.file_picker_dir().display().to_string(),
                Style::default().fg(theme.ui.toc_primary_inactive),
            ),
        ]),
    ];

    if app.is_fuzzy_file_picker() {
        lines.push(Line::from(vec![
            Span::styled("Query: ", section_style),
            Span::styled(
                if app.file_picker_query().is_empty() {
                    " type to filter ".to_string()
                } else {
                    format!(" {} ", app.file_picker_query())
                },
                Style::default()
                    .fg(if app.file_picker_query().is_empty() {
                        theme.ui.toc_primary_inactive
                    } else {
                        theme.ui.toc_primary_active
                    })
                    .bg(theme.markdown.inline_code_bg),
            ),
        ]));
    }

    lines.push(Line::from(""));

    if app.file_picker_entries().is_empty() {
        lines.push(Line::from(vec![Span::styled(
            if app.is_fuzzy_file_picker() {
                "No Markdown file found in this directory or its subdirectories"
            } else {
                "No folders or Markdown files here"
            },
            Style::default().fg(theme.ui.toc_primary_inactive),
        )]));
    } else if total == 0 {
        lines.push(Line::from(vec![Span::styled(
            "No match for the current query",
            Style::default().fg(theme.ui.toc_primary_inactive),
        )]));
    } else {
        for (idx, entry_idx) in app.file_picker_filtered_indices()[start..end]
            .iter()
            .enumerate()
        {
            let actual_idx = start + idx;
            let selected = actual_idx == app.file_picker_index();
            let entry = &app.file_picker_entries()[*entry_idx];
            let bg = if selected {
                theme.ui.toc_active_bg
            } else {
                theme.ui.toc_bg
            };
            let marker = if selected { "▸ " } else { "  " };
            let label_spans = if app.is_fuzzy_file_picker() {
                highlighted_picker_label(
                    entry.label(),
                    app.file_picker_match_positions(actual_idx),
                    bg,
                    selected,
                )
            } else {
                vec![Span::styled(
                    entry.label().to_string(),
                    Style::default()
                        .fg(theme.ui.toc_primary_inactive)
                        .bg(bg)
                        .add_modifier(if selected {
                            Modifier::BOLD
                        } else {
                            Modifier::empty()
                        }),
                )]
            };
            let mut spans = vec![Span::styled(
                marker,
                Style::default()
                    .fg(theme.ui.toc_accent)
                    .bg(bg)
                    .add_modifier(if selected {
                        Modifier::BOLD
                    } else {
                        Modifier::empty()
                    }),
            )];
            spans.extend(label_spans);
            lines.push(Line::from(spans));
        }
    }

    while lines.len() < inner_height.saturating_sub(reserved_footer_lines) {
        lines.push(Line::from(""));
    }

    if let Some(message) = truncation_message {
        lines.push(Line::from(vec![Span::styled(
            "",
            Style::default().fg(theme.ui.toc_primary_inactive),
        )]));
        lines.push(Line::from(vec![Span::styled(
            message,
            Style::default().fg(theme.markdown.heading_3),
        )]));
    } else {
        lines.push(Line::from(""));
    }

    lines.push(Line::from(vec![Span::styled(
        if app.is_fuzzy_file_picker() {
            "↑/↓ move • enter open • type filter • esc clear • ctrl+c quit"
        } else {
            "enter open • backspace up • ctrl+c quit"
        },
        footer_style.bg(theme.ui.toc_bg),
    )]));

    f.render_widget(Clear, area);
    f.render_widget(
        Paragraph::new(lines).block(
            Block::default()
                .title("─ Files ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.ui.toc_border))
                .style(Style::default().bg(theme.ui.toc_bg))
                .padding(Padding::new(1, 1, 0, 0)),
        ),
        area,
    );
}

pub(super) fn render_picker_loading(f: &mut Frame, app: &App) {
    let theme = app_theme();
    let area = centered_rect(78, 20, f.area());
    let title_style = Style::default()
        .fg(theme.markdown.heading_2)
        .add_modifier(Modifier::BOLD);
    let section_style = Style::default().fg(theme.ui.status_shortcut_fg);
    let footer_style = Style::default().fg(theme.ui.status_shortcut_fg);
    let is_failed = app.is_picker_load_failed();
    let is_fuzzy = matches!(
        app.pending_picker_mode(),
        Some(crate::app::FilePickerMode::Fuzzy)
    );
    let inner_height = area.height.saturating_sub(2) as usize;
    let message = if is_failed {
        app.picker_load_error().unwrap_or("Failed to load files")
    } else {
        "Indexing markdown files..."
    };

    let mut lines = vec![
        Line::from(vec![Span::styled("Open a Markdown file", title_style)]),
        Line::from(vec![
            Span::styled("Dir: ", section_style),
            Span::styled(
                app.pending_picker_dir()
                    .map(|dir| dir.display().to_string())
                    .unwrap_or_else(|| ".".to_string()),
                Style::default().fg(theme.ui.toc_primary_inactive),
            ),
        ]),
    ];

    if is_fuzzy {
        lines.push(Line::from(vec![
            Span::styled("Query: ", section_style),
            Span::styled(
                " type to filter ".to_string(),
                Style::default()
                    .fg(theme.ui.toc_primary_inactive)
                    .bg(theme.markdown.inline_code_bg),
            ),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![Span::styled(
        message,
        Style::default().fg(theme.ui.toc_primary_inactive),
    )]));

    while lines.len() < inner_height.saturating_sub(2) {
        lines.push(Line::from(""));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![Span::styled(
        if is_fuzzy {
            "↑/↓ move • enter open • type filter • esc clear • ctrl+c quit"
        } else {
            "enter open • backspace up • ctrl+c quit"
        },
        footer_style.bg(theme.ui.toc_bg),
    )]));

    f.render_widget(Clear, area);
    f.render_widget(
        Paragraph::new(lines).block(
            Block::default()
                .title("─ Files ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.ui.toc_border))
                .style(Style::default().bg(theme.ui.toc_bg))
                .padding(Padding::new(1, 1, 0, 0)),
        ),
        area,
    );
}

fn picker_truncation_message(
    truncation: Option<crate::app::PickerIndexTruncation>,
) -> Option<&'static str> {
    match truncation {
        Some(crate::app::PickerIndexTruncation::Directory) => {
            Some("Indexing limited: directory limit reached")
        }
        Some(crate::app::PickerIndexTruncation::File) => {
            Some("Indexing limited: file limit reached")
        }
        Some(crate::app::PickerIndexTruncation::Time) => {
            Some("Indexing limited: time limit reached")
        }
        None => None,
    }
}

fn highlighted_picker_label(
    label: &str,
    match_positions: &[usize],
    bg: Color,
    selected: bool,
) -> Vec<Span<'static>> {
    let theme = app_theme();
    let default_style = Style::default()
        .fg(theme.ui.toc_primary_inactive)
        .bg(bg)
        .add_modifier(if selected {
            Modifier::BOLD
        } else {
            Modifier::empty()
        });
    let matched_style = Style::default()
        .fg(theme.ui.toc_accent)
        .bg(bg)
        .add_modifier(if selected {
            Modifier::BOLD
        } else {
            Modifier::empty()
        });

    if match_positions.is_empty() {
        return vec![Span::styled(label.to_string(), default_style)];
    }

    let match_set = match_positions
        .iter()
        .copied()
        .collect::<std::collections::BTreeSet<_>>();
    let mut spans = Vec::new();
    let mut buffer = String::new();
    let mut current_matched = None;

    for (idx, ch) in label.chars().enumerate() {
        let is_matched = match_set.contains(&idx);
        if current_matched == Some(is_matched) || current_matched.is_none() {
            buffer.push(ch);
            current_matched = Some(is_matched);
            continue;
        }

        spans.push(Span::styled(
            std::mem::take(&mut buffer),
            if current_matched == Some(true) {
                matched_style
            } else {
                default_style
            },
        ));
        buffer.push(ch);
        current_matched = Some(is_matched);
    }

    if !buffer.is_empty() {
        spans.push(Span::styled(
            buffer,
            if current_matched == Some(true) {
                matched_style
            } else {
                default_style
            },
        ));
    }

    spans
}
