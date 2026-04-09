use crate::{
    app::App,
    cli::version_text,
    theme::{app_theme, theme_preset_label, THEME_PRESETS},
};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, Clear, Padding, Paragraph, Scrollbar, ScrollbarOrientation,
        ScrollbarState, Wrap,
    },
    Frame,
};

const CONTENT_HORIZONTAL_PADDING: u16 = 1;
const SCROLLBAR_WIDTH: u16 = 1;

pub(crate) fn ui(f: &mut Frame, app: &mut App) {
    let area = f.area();
    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(area);

    let (toc_area, content_area): (Option<Rect>, Rect) = if app.toc_visible && !app.toc.is_empty() {
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(30), Constraint::Min(0)])
            .split(root[0]);
        (Some(cols[0]), cols[1])
    } else {
        (None, root[0])
    };

    if let Some(ta) = toc_area {
        render_toc_panel(f, app, ta);
    }

    let viewport_height = content_area.height as usize;
    render_content_panel(f, app, content_area, viewport_height);
    render_status_bar(f, app, root[1], viewport_height);

    if app.is_help_open() {
        render_help_popup(f);
    } else if app.is_file_picker_open() {
        render_file_picker(f, app);
    } else if app.is_theme_picker_open() {
        render_theme_picker(f, app);
    }
}

fn render_toc_panel(f: &mut Frame, app: &mut App, area: Rect) {
    let theme = app_theme();
    app.refresh_toc_cache();
    let toc_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(area);

    f.render_widget(
        Paragraph::new("")
            .style(Style::default().bg(theme.ui.toc_bg))
            .block(
                Block::default()
                    .borders(Borders::RIGHT | Borders::BOTTOM)
                    .border_style(Style::default().fg(theme.ui.toc_border))
                    .style(Style::default().bg(theme.ui.toc_bg)),
            ),
        toc_chunks[0],
    );
    f.render_widget(
        Paragraph::new(app.toc_display_lines.clone())
            .style(Style::default().bg(theme.ui.toc_bg))
            .block(
                Block::default()
                    .borders(Borders::RIGHT)
                    .border_style(Style::default().fg(theme.ui.toc_border))
                    .style(Style::default().bg(theme.ui.toc_bg)),
            ),
        toc_chunks[1],
    );
    f.render_widget(
        Paragraph::new(vec![app.toc_header_line.clone()])
            .style(Style::default().bg(theme.ui.toc_bg)),
        Rect {
            x: toc_chunks[0].x,
            y: toc_chunks[0].y.saturating_add(1),
            width: toc_chunks[0].width.saturating_sub(1),
            height: 1,
        },
    );
}

fn render_content_panel(f: &mut Frame, app: &mut App, area: Rect, viewport_height: usize) {
    let theme = app_theme();
    f.render_widget(
        Paragraph::new("").style(Style::default().bg(theme.ui.content_bg)),
        area,
    );
    let content_area = inner_content_area(area);
    let scroll = app.scroll;
    let active_highlight_line = app.active_highlight_line();
    if let Some(line_idx) = active_highlight_line {
        let _ = app.refresh_highlighted_line_cache(line_idx);
    }

    let visible_end = (scroll + viewport_height).min(app.lines.len());
    let mut visible_lines = app.lines[scroll..visible_end].to_vec();

    if let Some(line_idx) = active_highlight_line {
        if (scroll..visible_end).contains(&line_idx) {
            if let Some((_, highlighted_line)) = &app.highlighted_line_cache {
                visible_lines[line_idx - scroll] = highlighted_line.clone();
            }
        }
    }

    f.render_widget(
        Paragraph::new(visible_lines)
            .style(Style::default().bg(theme.ui.content_bg))
            .wrap(Wrap { trim: false }),
        content_area,
    );

    let mut scrollbar_state = ScrollbarState::new(app.total()).position(app.scroll);
    f.render_stateful_widget(
        Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(None)
            .end_symbol(None)
            .track_symbol(Some("│"))
            .thumb_symbol("█"),
        area,
        &mut scrollbar_state,
    );
}

fn inner_content_area(area: Rect) -> Rect {
    Rect {
        x: area.x.saturating_add(CONTENT_HORIZONTAL_PADDING),
        y: area.y,
        width: area
            .width
            .saturating_sub(CONTENT_HORIZONTAL_PADDING.saturating_mul(2))
            .saturating_sub(SCROLLBAR_WIDTH),
        height: area.height,
    }
}

fn render_status_bar(f: &mut Frame, app: &mut App, area: Rect, viewport_height: usize) {
    let pct = app.scroll_percent(viewport_height);
    let bar_bg = status_bar_bg();
    app.refresh_status_cache(pct);

    f.render_widget(
        Paragraph::new(vec![app.status_line.clone()]).style(Style::default().bg(bar_bg)),
        area,
    );
}

pub(crate) fn status_bar_bg() -> Color {
    app_theme().ui.status_bg
}

pub(crate) fn status_separator_style(bar_bg: Color) -> Style {
    Style::default()
        .fg(app_theme().ui.status_separator)
        .bg(bar_bg)
}

pub(crate) fn join_span_sections(
    sections: Vec<Vec<Span<'static>>>,
    separator: Span<'static>,
) -> Vec<Span<'static>> {
    let mut joined = Vec::new();
    for (idx, section) in sections.into_iter().enumerate() {
        if idx > 0 {
            joined.push(separator.clone());
        }
        joined.extend(section);
    }
    joined
}

pub(crate) fn status_brand_section() -> Vec<Span<'static>> {
    let theme = app_theme();
    vec![Span::styled(
        " leaf ",
        Style::default()
            .fg(theme.ui.status_brand_fg)
            .bg(theme.ui.status_brand_bg)
            .add_modifier(Modifier::BOLD),
    )]
}

pub(crate) fn status_filename_section(filename: &str) -> Vec<Span<'static>> {
    let theme = app_theme();
    vec![Span::styled(
        format!(" {} ", filename),
        Style::default()
            .fg(theme.ui.status_filename_fg)
            .bg(theme.ui.status_filename_bg),
    )]
}

pub(crate) fn status_watch_section(app: &App) -> Option<Vec<Span<'static>>> {
    let theme = app_theme();
    if !app.watch {
        return None;
    }

    let flash_active = app
        .reload_flash
        .map(|t| t.elapsed() < std::time::Duration::from_millis(1500))
        .unwrap_or(false);
    let span = if flash_active {
        Span::styled(
            " ⟳ reloaded ",
            Style::default()
                .fg(theme.ui.status_reloaded_fg)
                .bg(theme.ui.status_reloaded_bg)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        Span::styled(
            " ⟳ watch ",
            Style::default()
                .fg(theme.ui.status_watch_fg)
                .bg(theme.ui.status_watch_bg),
        )
    };
    Some(vec![span])
}

pub(crate) fn status_search_section(app: &App) -> Option<Vec<Span<'static>>> {
    let theme = app_theme();
    if app.search_mode {
        return Some(vec![Span::styled(
            format!(" /{}", app.search_draft),
            Style::default()
                .fg(theme.ui.status_search_fg)
                .bg(theme.ui.status_search_bg),
        )]);
    }

    if app.search_query.is_empty() {
        return None;
    }

    let span = if app.search_matches.is_empty() {
        Span::styled(
            format!(" ✗ {} ", app.search_query),
            Style::default()
                .fg(theme.ui.status_search_error_fg)
                .bg(theme.ui.status_search_bg),
        )
    } else {
        Span::styled(
            format!(" {}/{} ", app.search_idx + 1, app.search_matches.len()),
            Style::default()
                .fg(theme.ui.status_search_match_fg)
                .bg(theme.ui.status_search_bg),
        )
    };
    Some(vec![span])
}

pub(crate) fn status_hint_segments(app: &App) -> &'static [&'static str] {
    if app.search_mode {
        &["enter confirm", "esc cancel"]
    } else if app.file_picker_open {
        &["j/k move", "enter open", "backspace up", "q quit"]
    } else if app.theme_picker_open {
        &["j/k preview", "enter keep", "esc restore"]
    } else if app.help_open {
        &["esc close", "? close"]
    } else if app.has_active_search() {
        &[
            "enter next",
            "n/N next/prev",
            "/ search",
            "? help",
            "T theme",
            "esc clear",
            "q quit",
        ]
    } else {
        &[
            "j/k scroll",
            "g/G top/bot",
            "t toc",
            "T theme",
            "/ search",
            "? help",
            "n/N next/prev",
            "q quit",
        ]
    }
}

fn render_help_popup(f: &mut Frame) {
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
        Line::from(vec![Span::styled(
            version_text().to_string(),
            title_style,
        )]),
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
        Line::from(vec![Span::styled(
            "Actions",
            section_style,
        )]),
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
        Line::from(vec![Span::styled(
            "Esc or ? to close",
            footer_style,
        )]),
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

fn render_theme_picker(f: &mut Frame, app: &App) {
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

fn render_file_picker(f: &mut Frame, app: &App) {
    let theme = app_theme();
    let area = centered_rect(70, 18, f.area());
    let title_style = Style::default()
        .fg(theme.markdown.heading_2)
        .add_modifier(Modifier::BOLD);
    let section_style = Style::default().fg(theme.ui.status_shortcut_fg);
    let footer_style = Style::default().fg(theme.ui.status_shortcut_fg);
    let inner_height = area.height.saturating_sub(2) as usize;
    let visible_slots = inner_height.saturating_sub(5);
    let total = app.file_picker_entries.len();
    let start = if visible_slots == 0 || app.file_picker_index < visible_slots {
        0
    } else {
        app.file_picker_index + 1 - visible_slots
    };
    let end = (start + visible_slots).min(total);

    let mut lines = vec![
        Line::from(vec![Span::styled("Open a Markdown file", title_style)]),
        Line::from(vec![Span::styled(
            app.file_picker_dir.display().to_string(),
            section_style,
        )]),
        Line::from(""),
    ];

    if total == 0 {
        lines.push(Line::from(vec![Span::styled(
            "No folders or Markdown files here",
            Style::default().fg(theme.ui.toc_primary_inactive),
        )]));
    } else {
        for (idx, entry) in app.file_picker_entries[start..end].iter().enumerate() {
            let actual_idx = start + idx;
            let selected = actual_idx == app.file_picker_index;
            let bg = if selected {
                theme.ui.toc_active_bg
            } else {
                theme.ui.toc_bg
            };
            let marker = if selected { "▸ " } else { "  " };
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
                    entry.label.clone(),
                    Style::default()
                        .fg(if entry.is_dir {
                            theme.ui.toc_primary_active
                        } else {
                            theme.ui.toc_primary_inactive
                        })
                        .bg(bg)
                        .add_modifier(if selected {
                            Modifier::BOLD
                        } else {
                            Modifier::empty()
                        }),
                ),
            ]));
        }
    }

    while lines.len() < inner_height.saturating_sub(1) {
        lines.push(Line::from(""));
    }
    lines.push(Line::from(vec![Span::styled(
        "Enter open • Backspace up • q quit",
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

fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let popup_width = width.min(area.width.saturating_sub(2)).max(1);
    let popup_height = height.min(area.height.saturating_sub(2)).max(1);
    Rect {
        x: area.x + area.width.saturating_sub(popup_width) / 2,
        y: area.y + area.height.saturating_sub(popup_height) / 2,
        width: popup_width,
        height: popup_height,
    }
}

pub(crate) fn status_shortcuts_section(app: &App, bar_bg: Color) -> Vec<Span<'static>> {
    let theme = app_theme();
    let separator = Span::styled(" · ", status_separator_style(bar_bg));
    let sections = status_hint_segments(app)
        .iter()
        .map(|segment| {
            vec![Span::styled(
                (*segment).to_string(),
                Style::default().fg(theme.ui.status_shortcut_fg).bg(bar_bg),
            )]
        })
        .collect();
    join_span_sections(sections, separator)
}

pub(crate) fn status_percent_section(pct: u16, bar_bg: Color) -> Vec<Span<'static>> {
    let theme = app_theme();
    vec![Span::styled(
        format!("{:>3}% ", pct),
        Style::default().fg(theme.ui.status_percent_fg).bg(bar_bg),
    )]
}

pub(crate) fn build_status_bar(app: &App, pct: u16) -> Vec<Span<'static>> {
    let bar_bg = status_bar_bg();
    let outer_separator = Span::raw(" ");

    let mut left_section = status_brand_section();
    left_section.extend(status_filename_section(&app.filename));

    if let Some(section) = status_search_section(app) {
        left_section.extend(section);
    }

    if let Some(section) = status_watch_section(app) {
        left_section.extend(section);
    }

    let mut sections = vec![left_section, status_shortcuts_section(app, bar_bg)];
    if !app.is_file_picker_open() {
        sections.push(status_percent_section(pct, bar_bg));
    }

    join_span_sections(sections, outer_separator)
}

pub(crate) fn toc_header_line() -> Line<'static> {
    let theme = app_theme();
    Line::from(vec![Span::styled(
        "  TABLE OF CONTENTS",
        Style::default()
            .fg(theme.ui.toc_header_fg)
            .bg(theme.ui.toc_bg)
            .add_modifier(Modifier::BOLD),
    )])
}

pub(crate) fn build_toc_line_with_index(
    entry: &crate::app::TocEntry,
    display_level: u8,
    top_level_index: Option<usize>,
    active: bool,
) -> Line<'static> {
    let theme = app_theme();
    let active_bg = theme.ui.toc_active_bg;
    let inactive_bg = theme.ui.toc_inactive_bg;

    match display_level {
        1 => {
            let index = top_level_index.unwrap_or(0) + 1;
            let title = crate::markdown::truncate_display_width(&entry.title, 18);
            let bg = if active { active_bg } else { inactive_bg };
            Line::from(vec![
                Span::styled(
                    if active { "▎" } else { " " },
                    Style::default().fg(theme.ui.toc_accent).bg(bg),
                ),
                Span::styled("  ", Style::default().bg(bg)),
                Span::styled(
                    format!("{index:02}"),
                    Style::default()
                        .fg(if active {
                            theme.ui.toc_accent
                        } else {
                            theme.ui.toc_index_inactive
                        })
                        .bg(bg)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(" ", Style::default().bg(bg)),
                Span::styled(
                    title,
                    Style::default()
                        .fg(if active {
                            theme.ui.toc_primary_active
                        } else {
                            theme.ui.toc_primary_inactive
                        })
                        .bg(bg)
                        .add_modifier(Modifier::BOLD),
                ),
            ])
        }
        _ => Line::from(vec![
            Span::styled(
                if active { "▎" } else { " " },
                Style::default().fg(theme.ui.toc_accent),
            ),
            Span::raw("     "),
            Span::styled(
                "•",
                Style::default().fg(if active {
                    theme.ui.toc_accent
                } else {
                    theme.ui.toc_secondary_inactive
                }),
            ),
            Span::raw(" "),
            Span::styled(
                crate::markdown::truncate_display_width(&entry.title, 18),
                Style::default()
                    .fg(if active {
                        theme.ui.toc_secondary_text_active
                    } else {
                        theme.ui.toc_secondary_text_inactive
                    })
                    .add_modifier(if active {
                        Modifier::BOLD
                    } else {
                        Modifier::empty()
                    }),
            ),
        ]),
    }
}
