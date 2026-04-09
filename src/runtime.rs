use crate::{
    app::{App, FileChange},
    render::ui,
};
use anyhow::Result;
use crossterm::event::{self, poll, Event, KeyCode, KeyEventKind, KeyModifiers, MouseEventKind};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{fs::OpenOptions, io, io::Write, time::Duration};
use syntect::{highlighting::ThemeSet, parsing::SyntaxSet};

const CONTENT_HORIZONTAL_PADDING: u16 = 1;
const SCROLLBAR_WIDTH: u16 = 1;

pub(crate) fn should_handle_key(kind: KeyEventKind) -> bool {
    !matches!(kind, KeyEventKind::Release)
}

pub(crate) fn debug_log(enabled: bool, message: &str) {
    if !enabled {
        return;
    }
    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open("leaf-debug.log")
    {
        let _ = writeln!(file, "{message}");
    }
}

pub(crate) fn run(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    ss: &SyntaxSet,
    themes: &ThemeSet,
) -> Result<()> {
    const WATCH_INTERVAL: Duration = Duration::from_millis(250);
    const FLASH_DURATION: Duration = Duration::from_millis(1500);
    const MOUSE_SCROLL_STEP: usize = 3;
    let mut needs_redraw = true;
    sync_render_width(terminal, app, ss, themes)?;

    loop {
        if needs_redraw {
            terminal.draw(|f| ui(f, app))?;
            needs_redraw = false;
        }

        let poll_timeout = if app.watch {
            let flash_timeout = app.reload_flash.and_then(|started| {
                let elapsed = started.elapsed();
                (elapsed < FLASH_DURATION).then_some(FLASH_DURATION - elapsed)
            });
            flash_timeout
                .map(|remaining| remaining.min(WATCH_INTERVAL))
                .unwrap_or(WATCH_INTERVAL)
        } else {
            Duration::MAX
        };

        let event_available = if app.watch { poll(poll_timeout)? } else { true };

        if event_available {
            match event::read()? {
                Event::Key(key) => {
                    debug_log(
                        app.debug_input,
                        &format!(
                            "key_event kind={:?} code={:?} modifiers={:?} search_mode={} query={:?} draft={:?} matches={} idx={}",
                            key.kind,
                            key.code,
                            key.modifiers,
                            app.search_mode,
                            app.search_query,
                            app.search_draft,
                            app.search_matches.len(),
                            app.search_idx
                        ),
                    );
                    if !should_handle_key(key.kind) {
                        continue;
                    }
                    let mut state_changed = true;
                    if app.is_help_open() {
                        match key.code {
                            KeyCode::Esc | KeyCode::Char('?') => app.close_help(),
                            _ => state_changed = false,
                        }
                    } else if app.is_file_picker_open() {
                        match key.code {
                            KeyCode::Char('q') => break,
                            KeyCode::Char('?') => app.open_help(),
                            KeyCode::Enter => {
                                state_changed = app.activate_file_picker_selection(ss, themes);
                            }
                            KeyCode::Char('j') | KeyCode::Down => app.move_file_picker_down(),
                            KeyCode::Char('k') | KeyCode::Up => app.move_file_picker_up(),
                            KeyCode::Backspace | KeyCode::Char('h') | KeyCode::Left => {
                                state_changed = app.open_file_picker_parent();
                            }
                            _ => state_changed = false,
                        }
                    } else if app.is_theme_picker_open() {
                        match key.code {
                            KeyCode::Esc => {
                                app.restore_theme_picker_preview(ss, themes);
                                needs_redraw = true;
                                state_changed = false;
                            }
                            KeyCode::Enter => app.close_theme_picker(),
                            KeyCode::Char('j') | KeyCode::Down => {
                                app.move_theme_picker_down();
                            }
                            KeyCode::Char('k') | KeyCode::Up => {
                                app.move_theme_picker_up();
                            }
                            KeyCode::Char(c) if c.is_ascii_digit() && c != '0' => {
                                if let Some(n) = c.to_digit(10) {
                                    let idx = n as usize - 1;
                                    if !app.set_theme_picker_index(idx) {
                                        state_changed = false;
                                    }
                                }
                            }
                            _ => state_changed = false,
                        }
                        if state_changed {
                            if let Some(preset) = app.selected_theme_preset() {
                                app.preview_theme_preset(preset, ss, themes);
                            }
                        }
                    } else if app.search_mode {
                        match key.code {
                            KeyCode::Esc => app.cancel_search(),
                            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                app.cancel_search();
                            }
                            KeyCode::Enter => app.confirm_search(),
                            KeyCode::Backspace => {
                                app.search_draft.pop();
                            }
                            KeyCode::Char(c) => {
                                app.search_draft.push(c);
                            }
                            _ => state_changed = false,
                        }
                    } else {
                        match key.code {
                            KeyCode::Esc if app.has_active_search() => app.clear_active_search(),
                            KeyCode::Enter if app.has_active_search() => app.next_match(),
                            KeyCode::Char('q') => break,
                            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                if app.has_active_search() {
                                    app.clear_active_search();
                                } else {
                                    break;
                                }
                            }
                            KeyCode::Char('j') | KeyCode::Down => app.scroll_down(1),
                            KeyCode::Char('k') | KeyCode::Up => app.scroll_up(1),
                            KeyCode::Char('d') | KeyCode::PageDown => app.scroll_down(20),
                            KeyCode::Char('u') | KeyCode::PageUp => app.scroll_up(20),
                            KeyCode::Char('g') | KeyCode::Home => {
                                app.scroll = 0;
                            }
                            KeyCode::Char('G') | KeyCode::End => {
                                app.scroll = app.total().saturating_sub(1);
                            }
                            KeyCode::Char('t') => {
                                app.toc_visible = !app.toc_visible;
                            }
                            KeyCode::Char('T') => {
                                app.open_theme_picker();
                            }
                            KeyCode::Char('?') => {
                                app.open_help();
                            }
                            KeyCode::Char('r') if app.watch => {
                                app.last_file_state = None;
                                app.reload(ss, themes);
                            }
                            KeyCode::Char('f')
                                if key.modifiers.contains(KeyModifiers::CONTROL) =>
                            {
                                app.begin_search()
                            }
                            KeyCode::Char('/') => app.begin_search(),
                            KeyCode::Char('n') => app.next_match(),
                            KeyCode::Char('N') => app.prev_match(),
                            KeyCode::Char(c) if c.is_ascii_digit() && c != '0' => {
                                if let Some(n) = c.to_digit(10) {
                                    app.jump_to_toc(n as usize - 1);
                                }
                            }
                            _ => state_changed = false,
                        }
                    }
                    if sync_render_width(terminal, app, ss, themes)? {
                        needs_redraw = true;
                    }
                    if state_changed {
                        needs_redraw = true;
                    }
                }
                Event::Mouse(mouse) => {
                    let state_changed = if app.is_file_picker_open() || app.is_theme_picker_open() {
                        false
                    } else {
                        match mouse.kind {
                            MouseEventKind::ScrollUp => {
                                app.scroll_up(MOUSE_SCROLL_STEP);
                                true
                            }
                            MouseEventKind::ScrollDown => {
                                app.scroll_down(MOUSE_SCROLL_STEP);
                                true
                            }
                            _ => false,
                        }
                    };
                    if state_changed {
                        needs_redraw = true;
                    }
                }
                Event::Resize(_, _) => {
                    let _ = sync_render_width(terminal, app, ss, themes)?;
                    needs_redraw = true;
                }
                _ => {}
            }
        }

        if app.watch {
            if let Some(change) = app.check_modified() {
                std::thread::sleep(Duration::from_millis(50));
                if app.reload(ss, themes) {
                    app.last_file_state = Some(match change {
                        FileChange::Metadata(state) | FileChange::Content(state) => state,
                    });
                    needs_redraw = true;
                }
            }
            if let Some(t) = app.reload_flash {
                if t.elapsed() >= FLASH_DURATION {
                    app.reload_flash = None;
                    needs_redraw = true;
                }
            }
        }
    }
    Ok(())
}

fn sync_render_width(
    terminal: &Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    ss: &SyntaxSet,
    themes: &ThemeSet,
) -> Result<bool> {
    let area = terminal.size()?;
    let content_width = if app.toc_visible && !app.toc.is_empty() {
        area.width.saturating_sub(30)
    } else {
        area.width
    };
    let effective_width = content_width
        .saturating_sub(CONTENT_HORIZONTAL_PADDING.saturating_mul(2))
        .saturating_sub(SCROLLBAR_WIDTH);
    Ok(app.sync_render_width(effective_width as usize, ss, themes))
}
