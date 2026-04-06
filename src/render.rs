use crate::app::App;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
    Frame,
};

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
        app.refresh_toc_cache();
        let toc_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(0)])
            .split(ta);

        f.render_widget(
            Paragraph::new("")
                .style(Style::default().bg(Color::Rgb(18, 18, 22)))
                .block(
                    Block::default()
                        .borders(Borders::RIGHT | Borders::BOTTOM)
                        .border_style(Style::default().fg(Color::Rgb(52, 52, 58)))
                        .style(Style::default().bg(Color::Rgb(18, 18, 22))),
                ),
            toc_chunks[0],
        );
        f.render_widget(
            Paragraph::new(app.toc_display_lines.clone())
                .style(Style::default().bg(Color::Rgb(18, 18, 22)))
                .block(
                    Block::default()
                        .borders(Borders::RIGHT)
                        .border_style(Style::default().fg(Color::Rgb(52, 52, 58)))
                        .style(Style::default().bg(Color::Rgb(18, 18, 22))),
                ),
            toc_chunks[1],
        );
        f.render_widget(
            Paragraph::new(vec![app.toc_header_line.clone()])
                .style(Style::default().bg(Color::Rgb(18, 18, 22))),
            Rect {
                x: toc_chunks[0].x,
                y: toc_chunks[0].y.saturating_add(1),
                width: toc_chunks[0].width.saturating_sub(1),
                height: 1,
            },
        );
    }

    let vh = content_area.height as usize;
    let scroll = app.scroll;
    let active_highlight_line = app.active_highlight_line();
    if let Some(line_idx) = active_highlight_line {
        let _ = app.refresh_highlighted_line_cache(line_idx);
    }
    let render_lines = &app.lines;
    let visible_end = (scroll + vh).min(render_lines.len());
    let mut visible_lines = render_lines[scroll..visible_end].to_vec();

    if let Some(line_idx) = active_highlight_line {
        if (scroll..visible_end).contains(&line_idx) {
            if let Some((_, highlighted_line)) = &app.highlighted_line_cache {
                visible_lines[line_idx - scroll] = highlighted_line.clone();
            }
        }
    }

    f.render_widget(
        Paragraph::new(visible_lines).style(Style::default().bg(Color::Rgb(18, 20, 28))),
        content_area,
    );

    let mut ss_state = ScrollbarState::new(app.total()).position(app.scroll);
    f.render_stateful_widget(
        Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(None)
            .end_symbol(None)
            .track_symbol(Some("│"))
            .thumb_symbol("█"),
        content_area,
        &mut ss_state,
    );

    let pct = app.scroll_percent(vh);
    let bar_bg = status_bar_bg();
    app.refresh_status_cache(pct);

    f.render_widget(
        Paragraph::new(vec![app.status_line.clone()]).style(Style::default().bg(bar_bg)),
        root[1],
    );
}

pub(crate) fn status_bar_bg() -> Color {
    Color::Rgb(18, 20, 32)
}

pub(crate) fn status_separator_style(bar_bg: Color) -> Style {
    Style::default().fg(Color::Rgb(116, 126, 156)).bg(bar_bg)
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
    vec![Span::styled(
        " leaf ",
        Style::default()
            .fg(Color::Rgb(16, 18, 26))
            .bg(Color::Rgb(105, 178, 218))
            .add_modifier(Modifier::BOLD),
    )]
}

pub(crate) fn status_filename_section(filename: &str) -> Vec<Span<'static>> {
    vec![Span::styled(
        format!(" {} ", filename),
        Style::default()
            .fg(Color::Rgb(162, 192, 222))
            .bg(Color::Rgb(24, 28, 44)),
    )]
}

pub(crate) fn status_watch_section(app: &App) -> Option<Vec<Span<'static>>> {
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
                .fg(Color::Rgb(16, 18, 26))
                .bg(Color::Rgb(95, 200, 148))
                .add_modifier(Modifier::BOLD),
        )
    } else {
        Span::styled(
            " ⟳ watch ",
            Style::default()
                .fg(Color::Rgb(95, 200, 148))
                .bg(Color::Rgb(18, 30, 24)),
        )
    };
    Some(vec![span])
}

pub(crate) fn status_search_section(app: &App) -> Option<Vec<Span<'static>>> {
    if app.search_mode {
        return Some(vec![Span::styled(
            format!(" /{}", app.search_draft),
            Style::default()
                .fg(Color::Rgb(240, 210, 95))
                .bg(Color::Rgb(26, 28, 42)),
        )]);
    }

    if app.search_query.is_empty() {
        return None;
    }

    let span = if app.search_matches.is_empty() {
        Span::styled(
            format!(" ✗ {} ", app.search_query),
            Style::default()
                .fg(Color::Rgb(218, 95, 95))
                .bg(Color::Rgb(26, 28, 42)),
        )
    } else {
        Span::styled(
            format!(" {}/{} ", app.search_idx + 1, app.search_matches.len()),
            Style::default()
                .fg(Color::Rgb(115, 208, 148))
                .bg(Color::Rgb(26, 28, 42)),
        )
    };
    Some(vec![span])
}

pub(crate) fn status_hint_segments(app: &App) -> &'static [&'static str] {
    if app.search_mode {
        &["enter confirm", "esc cancel"]
    } else if app.has_active_search() {
        &[
            "enter next",
            "n/N next/prev",
            "/ search",
            "esc clear",
            "q quit",
        ]
    } else {
        &[
            "j/k scroll",
            "g/G top/bot",
            "t toc",
            "/ search",
            "n/N next/prev",
            "q quit",
        ]
    }
}

pub(crate) fn status_shortcuts_section(app: &App, bar_bg: Color) -> Vec<Span<'static>> {
    let separator = Span::styled(" · ", status_separator_style(bar_bg));
    let sections = status_hint_segments(app)
        .iter()
        .map(|segment| {
            vec![Span::styled(
                (*segment).to_string(),
                Style::default().fg(Color::Rgb(58, 68, 98)).bg(bar_bg),
            )]
        })
        .collect();
    join_span_sections(sections, separator)
}

pub(crate) fn status_percent_section(pct: u16, bar_bg: Color) -> Vec<Span<'static>> {
    vec![Span::styled(
        format!("{:>3}% ", pct),
        Style::default().fg(Color::Rgb(105, 178, 218)).bg(bar_bg),
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

    let sections = vec![
        left_section,
        status_shortcuts_section(app, bar_bg),
        status_percent_section(pct, bar_bg),
    ];

    join_span_sections(sections, outer_separator)
}

pub(crate) fn toc_header_line() -> Line<'static> {
    Line::from(vec![Span::styled(
        "  TABLE OF CONTENTS",
        Style::default()
            .fg(Color::Rgb(88, 88, 96))
            .bg(Color::Rgb(18, 18, 22))
            .add_modifier(Modifier::BOLD),
    )])
}

pub(crate) fn build_toc_line_with_index(
    entry: &crate::app::TocEntry,
    display_level: u8,
    top_level_index: Option<usize>,
    active: bool,
) -> Line<'static> {
    let active_bg = Color::Rgb(42, 40, 46);
    let inactive_bg = Color::Rgb(18, 18, 22);

    match display_level {
        1 => {
            let index = top_level_index.unwrap_or(0) + 1;
            let title = crate::markdown::truncate_display_width(&entry.title, 18);
            let bg = if active { active_bg } else { inactive_bg };
            Line::from(vec![
                Span::styled(
                    if active { "▎" } else { " " },
                    Style::default().fg(Color::Rgb(123, 109, 255)).bg(bg),
                ),
                Span::styled("  ", Style::default().bg(bg)),
                Span::styled(
                    format!("{index:02}"),
                    Style::default()
                        .fg(if active {
                            Color::Rgb(123, 109, 255)
                        } else {
                            Color::Rgb(60, 60, 66)
                        })
                        .bg(bg)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(" ", Style::default().bg(bg)),
                Span::styled(
                    title,
                    Style::default()
                        .fg(if active {
                            Color::Rgb(224, 224, 228)
                        } else {
                            Color::Rgb(136, 136, 142)
                        })
                        .bg(bg)
                        .add_modifier(Modifier::BOLD),
                ),
            ])
        }
        _ => Line::from(vec![
            Span::styled(
                if active { "▎" } else { " " },
                Style::default().fg(Color::Rgb(123, 109, 255)),
            ),
            Span::raw("     "),
            Span::styled(
                "•",
                Style::default().fg(if active {
                    Color::Rgb(123, 109, 255)
                } else {
                    Color::Rgb(62, 62, 68)
                }),
            ),
            Span::raw(" "),
            Span::styled(
                crate::markdown::truncate_display_width(&entry.title, 18),
                Style::default()
                    .fg(if active {
                        Color::Rgb(224, 224, 228)
                    } else {
                        Color::Rgb(102, 102, 108)
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
