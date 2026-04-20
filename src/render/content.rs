use crate::{app::App, theme::app_theme};
use ratatui::{
    layout::Rect,
    style::Style,
    widgets::{Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap},
    Frame,
};

use super::{CONTENT_HORIZONTAL_PADDING, SCROLLBAR_WIDTH};

pub(super) fn render_content_panel(
    f: &mut Frame,
    app: &mut App,
    area: Rect,
    viewport_height: usize,
) {
    let theme = app_theme();
    f.render_widget(
        Paragraph::new("").style(Style::default().bg(theme.ui.content_bg)),
        area,
    );
    let content_area = inner_content_area(area);
    let scroll = app.scroll();
    let active_highlight_line = app.active_highlight_line();
    if let Some(line_idx) = active_highlight_line {
        let _ = app.refresh_highlighted_line_cache(line_idx);
    }

    let visible_end = (scroll + viewport_height).min(app.total());
    let mut visible_lines = app.visible_lines(scroll, visible_end).to_vec();

    if let Some(line_idx) = active_highlight_line {
        if (scroll..visible_end).contains(&line_idx) {
            if let Some((_, highlighted_line)) = app.highlighted_line_cache() {
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

    let (mouse_col, mouse_row) = app.mouse_position;
    let sb_x = area.x + area.width - SCROLLBAR_WIDTH;
    let on_sb_column = mouse_col >= sb_x
        && mouse_col < sb_x + SCROLLBAR_WIDTH
        && mouse_row >= area.y
        && mouse_row < area.y + area.height;

    let total = app.total();
    let track_len = area.height as usize;
    let mouse_on_thumb = on_sb_column && track_len > 0 && total > 0 && {
        let thumb_size = (track_len * track_len / total).max(1).min(track_len);
        let max_offset = track_len.saturating_sub(thumb_size);
        let thumb_offset = if total <= 1 {
            0
        } else {
            app.scroll() * max_offset / (total - 1)
        };
        let thumb_top = area.y as usize + thumb_offset;
        let thumb_bottom = thumb_top + thumb_size;
        let row = mouse_row as usize;
        row >= thumb_top && row < thumb_bottom
    };

    let mut scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .begin_symbol(None)
        .end_symbol(None)
        .track_symbol(Some("│"))
        .thumb_symbol("█");
    if mouse_on_thumb || app.scrollbar_dragging {
        scrollbar = scrollbar.thumb_style(Style::default().fg(theme.ui.scrollbar_hover));
    }

    let mut scrollbar_state = ScrollbarState::new(app.total()).position(app.scroll());
    f.render_stateful_widget(scrollbar, area, &mut scrollbar_state);
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

pub(super) fn render_status_bar(f: &mut Frame, app: &mut App, area: Rect, viewport_height: usize) {
    let pct = app.scroll_percent(viewport_height);
    let bar_bg = super::status::status_bar_bg();
    app.refresh_status_cache(pct);

    f.render_widget(
        Paragraph::new(vec![app.status_line().clone()]).style(Style::default().bg(bar_bg)),
        area,
    );
}
