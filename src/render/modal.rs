use crate::{
    app::App,
    cli::version_text,
    editor::EditorKind,
    theme::{app_theme, theme_preset_label, THEME_PRESETS},
};
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Padding, Paragraph},
    Frame,
};

use super::centered_rect;

const FUZZY_PICKER_FOOTER_INIT: &[&str] = &[
    "↑/↓ move",
    "<char> filter",
    "enter open",
    "esc clear",
    "ctrl+c quit",
];

const FUZZY_PICKER_FOOTER_PREVIEW: &[&str] = &[
    "↑/↓ move",
    "<char> filter",
    "enter open",
    "esc clear",
    "ctrl+c close",
];

const BROWSER_PICKER_FOOTER_INIT: &[&str] = &["↑/↓ move", "enter open", "bsp parent", "q quit"];

const BROWSER_PICKER_FOOTER_PREVIEW: &[&str] =
    &["↑/↓ move", "enter open", "bsp parent", "ctrl+c close"];

const PICKER_FAILED_FOOTER_INIT: &[&str] = &["esc quit", "enter quit", "q quit"];

const PICKER_FAILED_FOOTER_PREVIEW: &[&str] = &["esc close", "enter close", "ctrl+c close"];

fn picker_footer(has_content: bool, is_fuzzy: bool, is_failed: bool) -> &'static [&'static str] {
    if has_content {
        if is_failed {
            PICKER_FAILED_FOOTER_PREVIEW
        } else if is_fuzzy {
            FUZZY_PICKER_FOOTER_PREVIEW
        } else {
            BROWSER_PICKER_FOOTER_PREVIEW
        }
    } else if is_failed {
        PICKER_FAILED_FOOTER_INIT
    } else if is_fuzzy {
        FUZZY_PICKER_FOOTER_INIT
    } else {
        BROWSER_PICKER_FOOTER_INIT
    }
}

fn modal_footer_line(segments: &[&'static str], bg: Color) -> Line<'static> {
    let theme = app_theme();
    let shortcut_style = Style::default().fg(theme.ui.status_shortcut_fg).bg(bg);
    let separator_style = Style::default().fg(theme.ui.status_separator).bg(bg);
    let mut spans = Vec::new();
    for (idx, segment) in segments.iter().enumerate() {
        if idx > 0 {
            spans.push(Span::styled(" · ", separator_style));
        }
        spans.push(Span::styled(*segment, shortcut_style));
    }
    Line::from(spans)
}

pub(super) fn render_help_popup(f: &mut Frame) {
    let theme = app_theme();
    let area = centered_rect(54, 20, f.area());
    let section_style = Style::default()
        .fg(theme.ui.toc_primary_active)
        .add_modifier(Modifier::BOLD);
    let key_style = Style::default()
        .fg(theme.ui.toc_accent)
        .add_modifier(Modifier::BOLD);
    let text_style = Style::default().fg(theme.ui.toc_primary_inactive);

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
            Span::styled("Ctrl+F     ", key_style),
            Span::styled("find", text_style),
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
        Line::from(vec![Span::styled("Watch", section_style)]),
        Line::from(vec![
            Span::styled("Ctrl+W, w  ", key_style),
            Span::styled("toggle watch", text_style),
            Span::raw("      "),
            Span::styled("Ctrl+R, r  ", key_style),
            Span::styled("reload", text_style),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled("Actions", section_style)]),
        Line::from(vec![
            Span::styled("Shift+E    ", key_style),
            Span::styled("editor picker", text_style),
            Span::raw("     "),
            Span::styled("Ctrl+E     ", key_style),
            Span::styled("edit", text_style),
        ]),
        Line::from(vec![
            Span::styled("Shift+P    ", key_style),
            Span::styled("file browser", text_style),
            Span::raw("      "),
            Span::styled("Ctrl+P     ", key_style),
            Span::styled("pick", text_style),
        ]),
        Line::from(vec![
            Span::styled("Shift+T    ", key_style),
            Span::styled("theme picker", text_style),
            Span::raw("      "),
            Span::styled("?          ", key_style),
            Span::styled("help", text_style),
        ]),
        Line::from(vec![
            Span::styled("t          ", key_style),
            Span::styled("toggle toc", text_style),
            Span::raw("        "),
            Span::styled("q          ", key_style),
            Span::styled("quit", text_style),
        ]),
        Line::from(""),
        modal_footer_line(&["esc close", "? close"], theme.ui.toc_bg),
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
    let area = centered_rect(43, 10, f.area());
    let active = app.theme_picker_reference_preset();

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
        let marker = if selected { "▎ " } else { "  " };
        let check = if is_active { "  ✓" } else { "" };
        let modifier = if is_active || selected {
            Modifier::BOLD
        } else {
            Modifier::empty()
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
                theme_preset_label(*preset),
                Style::default()
                    .fg(if selected {
                        theme.ui.toc_primary_active
                    } else {
                        theme.ui.toc_primary_inactive
                    })
                    .bg(bg)
                    .add_modifier(modifier),
            ),
            Span::styled(
                check,
                Style::default()
                    .fg(theme.ui.toc_accent)
                    .bg(bg)
                    .add_modifier(modifier),
            ),
        ]));
    }
    lines.push(Line::from(""));
    lines.push(modal_footer_line(
        &["↑/↓ preview", "enter keep", "esc restore"],
        theme.ui.toc_bg,
    ));

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

    lines.push(modal_footer_line(
        picker_footer(app.has_content(), app.is_fuzzy_file_picker(), false),
        theme.ui.toc_bg,
    ));

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
                " type to filter ",
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
    lines.push(modal_footer_line(
        picker_footer(app.has_content(), is_fuzzy, is_failed),
        theme.ui.toc_bg,
    ));

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

pub(super) fn render_editor_picker(f: &mut Frame, app: &App) {
    let theme = app_theme();
    let entries = app.editor_picker_entries();
    let selected = app.editor_picker_index();
    let current_editor = app.editor_config().map(crate::editor::binary_name);

    let section_style = Style::default()
        .fg(theme.ui.toc_primary_active)
        .add_modifier(Modifier::BOLD);

    let title_style = Style::default().fg(theme.ui.status_shortcut_fg);

    let mut lines: Vec<Line<'static>> = Vec::new();
    lines.push(Line::from(vec![Span::styled(
        "Choose an editor",
        title_style,
    )]));
    lines.push(Line::from(""));

    if entries.is_empty() {
        lines.push(Line::from(vec![Span::styled(
            "No editors found",
            Style::default().fg(theme.ui.status_error_fg),
        )]));
    } else {
        let has_terminal = entries.iter().any(|e| e.kind == EditorKind::Terminal);
        let has_gui = entries.iter().any(|e| e.kind == EditorKind::Gui);

        let mk_line = |entry: &crate::editor::EditorEntry, idx: usize| -> Line<'static> {
            let is_selected = idx == selected;
            let is_current = current_editor == Some(crate::editor::binary_name(&entry.command));
            let bg = if is_selected {
                theme.ui.toc_active_bg
            } else {
                theme.ui.toc_bg
            };
            let fg = if is_selected {
                theme.ui.toc_primary_active
            } else {
                theme.ui.toc_primary_inactive
            };
            let mut modifier = Modifier::empty();
            if is_selected || is_current {
                modifier |= Modifier::BOLD;
            }
            let marker = if is_selected { "▎ " } else { "  " };
            let check = if is_current { "  ✓" } else { "" };
            Line::from(vec![
                Span::styled(
                    marker.to_string(),
                    Style::default()
                        .fg(theme.ui.toc_accent)
                        .bg(bg)
                        .add_modifier(modifier),
                ),
                Span::styled(
                    entry.name.clone(),
                    Style::default().fg(fg).bg(bg).add_modifier(modifier),
                ),
                Span::styled(
                    check.to_string(),
                    Style::default()
                        .fg(theme.ui.toc_accent)
                        .bg(bg)
                        .add_modifier(modifier),
                ),
            ])
        };

        let mut item_idx = 0usize;
        if has_terminal {
            lines.push(Line::from(vec![Span::styled("Terminal", section_style)]));
            for entry in entries.iter().filter(|e| e.kind == EditorKind::Terminal) {
                lines.push(mk_line(entry, item_idx));
                item_idx += 1;
            }
        }
        if has_gui {
            if has_terminal {
                lines.push(Line::from(""));
            }
            lines.push(Line::from(vec![Span::styled("GUI", section_style)]));
            for entry in entries.iter().filter(|e| e.kind == EditorKind::Gui) {
                lines.push(mk_line(entry, item_idx));
                item_idx += 1;
            }
        }
    }

    lines.push(Line::from(""));
    lines.push(modal_footer_line(
        &["↑/↓ move", "enter confirm", "esc cancel"],
        theme.ui.toc_bg,
    ));

    let height = (lines.len() as u16 + 2).min(18);
    let area = centered_rect(42, height, f.area());

    f.render_widget(Clear, area);
    f.render_widget(
        Paragraph::new(lines).block(
            Block::default()
                .title("─ Editor ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.ui.toc_border))
                .style(Style::default().bg(theme.ui.toc_bg))
                .padding(Padding::new(1, 1, 0, 0)),
        ),
        area,
    );
}
