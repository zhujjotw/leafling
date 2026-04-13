use crate::{app::App, theme::app_theme};
use ratatui::{
    style::{Color, Modifier, Style},
    text::Span,
};

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
    if !app.is_watch_enabled() {
        return None;
    }

    let flash_active = app
        .reload_flash_started()
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
    if app.is_search_mode() {
        return Some(vec![Span::styled(
            format!(" /{} ", app.search_draft()),
            Style::default()
                .fg(theme.ui.status_search_fg)
                .bg(theme.ui.status_search_bg),
        )]);
    }

    if app.search_query().is_empty() {
        return None;
    }

    let span = if app.search_match_count() == 0 {
        Span::styled(
            format!(" ✗ {} ", app.search_query()),
            Style::default()
                .fg(theme.ui.status_search_error_fg)
                .bg(theme.ui.status_search_bg),
        )
    } else {
        Span::styled(
            format!(" {}/{} ", app.search_index() + 1, app.search_match_count()),
            Style::default()
                .fg(theme.ui.status_search_match_fg)
                .bg(theme.ui.status_search_bg),
        )
    };
    Some(vec![span])
}

pub(crate) fn status_hint_segments(app: &App) -> &'static [&'static str] {
    if app.is_search_mode() {
        &["enter confirm", "esc cancel"]
    } else if app.is_file_picker_open() {
        if app.is_fuzzy_file_picker() {
            &["↑/↓ move", "enter open", "backspace delete", "ctrl+c quit"]
        } else {
            &["j/k move", "enter open", "backspace up", "ctrl+c quit"]
        }
    } else if app.is_theme_picker_open() {
        &["j/k preview", "enter keep", "esc restore"]
    } else if app.is_help_open() {
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
    left_section.extend(status_filename_section(app.filename()));

    if let Some(section) = status_search_section(app) {
        left_section.extend(section);
    }

    if let Some(section) = status_watch_section(app) {
        left_section.extend(section);
    }

    let mut sections = vec![left_section, status_shortcuts_section(app, bar_bg)];
    if !app.is_file_picker_open() && !app.is_picker_loading() {
        sections.push(status_percent_section(pct, bar_bg));
    }

    join_span_sections(sections, outer_separator)
}
