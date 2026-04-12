mod content;
mod modal;
mod status;
mod toc;

use crate::app::App;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    Frame,
};

pub(crate) use status::build_status_bar;
pub(crate) use toc::{build_toc_line_with_index, toc_header_line};

pub(crate) const CONTENT_HORIZONTAL_PADDING: u16 = 1;
pub(crate) const SCROLLBAR_WIDTH: u16 = 1;

pub(crate) fn ui(f: &mut Frame, app: &mut App) {
    let area = f.area();
    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(area);

    let (toc_area, content_area): (Option<Rect>, Rect) = if app.is_toc_visible() && app.has_toc() {
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(30), Constraint::Min(0)])
            .split(root[0]);
        (Some(cols[0]), cols[1])
    } else {
        (None, root[0])
    };

    if let Some(ta) = toc_area {
        toc::render_toc_panel(f, app, ta);
    }

    let viewport_height = content_area.height as usize;
    content::render_content_panel(f, app, content_area, viewport_height);
    content::render_status_bar(f, app, root[1], viewport_height);

    if app.is_help_open() {
        modal::render_help_popup(f);
    } else if app.is_picker_loading() || app.is_picker_load_failed() {
        modal::render_picker_loading(f, app);
    } else if app.is_file_picker_open() {
        modal::render_file_picker(f, app);
    } else if app.is_theme_picker_open() {
        modal::render_theme_picker(f, app);
    }
}

pub(super) fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let popup_width = width.min(area.width.saturating_sub(2)).max(1);
    let popup_height = height.min(area.height.saturating_sub(2)).max(1);
    Rect {
        x: area.x + area.width.saturating_sub(popup_width) / 2,
        y: area.y + area.height.saturating_sub(popup_height) / 2,
        width: popup_width,
        height: popup_height,
    }
}
