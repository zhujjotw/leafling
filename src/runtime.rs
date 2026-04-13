use crate::{
    app::{App, EditorFlash, FileChange},
    editor::{
        self, check_termux_external_apps, classify, open_in_editor, split_editor_cmd, EditorKind,
        EditorResult, TerminalEmulator,
    },
    render::{ui, CONTENT_HORIZONTAL_PADDING, SCROLLBAR_WIDTH},
};
use anyhow::Result;
use crossterm::event::{self, poll, Event, KeyCode, KeyEventKind, KeyModifiers, MouseEventKind};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{
    fs::OpenOptions,
    io,
    io::Write,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use syntect::{highlighting::ThemeSet, parsing::SyntaxSet};

pub(crate) fn should_handle_key(kind: KeyEventKind) -> bool {
    !matches!(kind, KeyEventKind::Release)
}

pub(crate) fn debug_log(enabled: bool, message: &str) {
    if !enabled {
        return;
    }
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0);
    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open("leaf-debug.log")
    {
        let _ = writeln!(file, "[{timestamp}] {message}");
    }
}

pub(crate) fn prepare_initial_picker_state(
    area_width: usize,
    app: &mut App,
    ss: &SyntaxSet,
    themes: &ThemeSet,
) -> Result<()> {
    debug_log(
        app.debug_input_enabled(),
        &format!("prepare_initial_picker_state start area_width={area_width}"),
    );
    sync_render_width_for_app(area_width, app, ss, themes);
    if app.has_pending_picker() && !app.is_picker_loading() {
        let _ = app.start_pending_picker_loading();
    }
    debug_log(
        app.debug_input_enabled(),
        &format!(
            "prepare_initial_picker_state end picker_loading={} pending_picker={}",
            app.is_picker_loading(),
            app.has_pending_picker()
        ),
    );
    Ok(())
}

pub(crate) fn run(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    ss: &SyntaxSet,
    themes: &ThemeSet,
    initial_draw_done: bool,
) -> Result<()> {
    const WATCH_INTERVAL: Duration = Duration::from_millis(250);
    const FLASH_DURATION: Duration = Duration::from_millis(1500);
    const MOUSE_SCROLL_STEP: usize = 3;
    const RESIZE_DEBOUNCE: Duration = Duration::from_millis(120);
    const PICKER_LOAD_POLL_INTERVAL: Duration = Duration::from_millis(50);
    let mut needs_redraw = !initial_draw_done;
    let mut pending_resize: Option<Instant> = None;
    sync_render_width(terminal, app, ss, themes)?;

    loop {
        if app.poll_picker_loading() {
            needs_redraw = true;
        }

        if needs_redraw {
            terminal.draw(|f| ui(f, app))?;
            needs_redraw = false;
        }

        let flash_timeout = app.reload_flash_started().and_then(|started| {
            let elapsed = started.elapsed();
            (elapsed < FLASH_DURATION).then_some(FLASH_DURATION - elapsed)
        });
        let editor_flash_timeout = app.editor_flash().and_then(|(_, started)| {
            let elapsed = started.elapsed();
            (elapsed < EDITOR_FLASH_DURATION).then_some(EDITOR_FLASH_DURATION - elapsed)
        });
        let resize_timeout = pending_resize.and_then(|started| {
            let elapsed = started.elapsed();
            (elapsed < RESIZE_DEBOUNCE).then_some(RESIZE_DEBOUNCE - elapsed)
        });
        let poll_timeout = [
            if app.is_watch_enabled() {
                Some(WATCH_INTERVAL)
            } else {
                None
            },
            if app.is_picker_loading() {
                Some(PICKER_LOAD_POLL_INTERVAL)
            } else {
                None
            },
            flash_timeout,
            editor_flash_timeout,
            resize_timeout,
        ]
        .into_iter()
        .flatten()
        .min()
        .unwrap_or(Duration::MAX);

        let event_available = if poll_timeout == Duration::MAX {
            true
        } else {
            poll(poll_timeout)?
        };

        if event_available {
            match event::read()? {
                Event::Key(key) => {
                    debug_log(
                        app.debug_input_enabled(),
                        &format!(
                            "key_event kind={:?} code={:?} modifiers={:?} search_mode={} query={:?} draft={:?} matches={} idx={}",
                            key.kind,
                            key.code,
                            key.modifiers,
                            app.is_search_mode(),
                            app.search_query(),
                            app.search_draft(),
                            app.search_match_count(),
                            app.search_index()
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
                    } else if app.is_picker_loading() {
                        match key.code {
                            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                break;
                            }
                            _ => state_changed = false,
                        }
                    } else if app.is_picker_load_failed() {
                        match key.code {
                            KeyCode::Esc | KeyCode::Enter => break,
                            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                break;
                            }
                            _ => state_changed = false,
                        }
                    } else if app.is_file_picker_open() {
                        match key.code {
                            KeyCode::Char('?') => app.open_help(),
                            KeyCode::Enter => {
                                state_changed = app.activate_file_picker_selection(ss, themes);
                            }
                            KeyCode::Char('q') if app.is_browser_file_picker() => break,
                            KeyCode::Char('j') | KeyCode::Down if app.is_browser_file_picker() => {
                                app.move_file_picker_down()
                            }
                            KeyCode::Char('k') | KeyCode::Up if app.is_browser_file_picker() => {
                                app.move_file_picker_up()
                            }
                            KeyCode::Down if app.is_fuzzy_file_picker() => {
                                app.move_file_picker_down()
                            }
                            KeyCode::Up if app.is_fuzzy_file_picker() => app.move_file_picker_up(),
                            KeyCode::Esc => {
                                if app.is_browser_file_picker()
                                    || app.file_picker_query().is_empty()
                                {
                                    state_changed = false;
                                } else {
                                    app.clear_file_picker_query();
                                }
                            }
                            KeyCode::Char('h') | KeyCode::Left if app.is_browser_file_picker() => {
                                state_changed = app.open_file_picker_parent();
                            }
                            KeyCode::Backspace if app.is_browser_file_picker() => {
                                state_changed = app.open_file_picker_parent();
                            }
                            KeyCode::Backspace => app.pop_file_picker_query(),
                            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                break;
                            }
                            KeyCode::Char(c)
                                if app.is_fuzzy_file_picker()
                                    && !key.modifiers.contains(KeyModifiers::CONTROL) =>
                            {
                                app.push_file_picker_query(c);
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
                    } else if app.is_search_mode() {
                        match key.code {
                            KeyCode::Esc => app.cancel_search(),
                            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                app.cancel_search();
                            }
                            KeyCode::Enter => app.confirm_search(),
                            KeyCode::Backspace => app.pop_search_draft(),
                            KeyCode::Char(c) => app.push_search_draft(c),
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
                            KeyCode::Char('g') | KeyCode::Home => app.scroll_top(),
                            KeyCode::Char('G') | KeyCode::End => app.scroll_bottom(),
                            KeyCode::Char('t') => app.toggle_toc(),
                            KeyCode::Char('T') => {
                                app.open_theme_picker();
                            }
                            KeyCode::Char('?') => {
                                app.open_help();
                            }
                            KeyCode::Char('r') if app.is_watch_enabled() => {
                                app.request_reload(ss, themes);
                            }
                            KeyCode::Char('f') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                app.begin_search()
                            }
                            KeyCode::Char('/') => app.begin_search(),
                            KeyCode::Char('n') => app.next_match(),
                            KeyCode::Char('N') => app.prev_match(),
                            KeyCode::Char('e') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                handle_open_in_editor(terminal, app, ss, themes)?;
                            }
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
                    pending_resize = Some(Instant::now());
                }
                _ => {}
            }
        }

        if pending_resize
            .map(|started| started.elapsed() >= RESIZE_DEBOUNCE)
            .unwrap_or(false)
        {
            pending_resize = None;
            if sync_render_width(terminal, app, ss, themes)? {
                needs_redraw = true;
            }
        }

        if app.is_watch_enabled() {
            if let Some(change) = app.check_modified() {
                std::thread::sleep(Duration::from_millis(50));
                if app.reload(ss, themes) {
                    app.set_last_file_state(match change {
                        FileChange::Metadata(state) | FileChange::Content(state) => state,
                    });
                    needs_redraw = true;
                }
            }
            if let Some(t) = app.reload_flash_started() {
                if t.elapsed() >= FLASH_DURATION {
                    app.clear_reload_flash();
                    needs_redraw = true;
                }
            }
        }

        if let Some((_, started)) = app.editor_flash() {
            if started.elapsed() >= EDITOR_FLASH_DURATION {
                app.clear_editor_flash();
                needs_redraw = true;
            }
        }
    }
    Ok(())
}

const EDITOR_FLASH_DURATION: Duration = Duration::from_millis(2000);

fn handle_open_in_editor(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    ss: &SyntaxSet,
    themes: &ThemeSet,
) -> Result<()> {
    let filepath = match app.filepath() {
        Some(p) => p.to_path_buf(),
        None => {
            app.set_editor_flash(EditorFlash::NoFile);
            return Ok(());
        }
    };

    let editor_cmd = match app.editor_config() {
        Some(e) => e.to_string(),
        None => {
            app.set_editor_flash(EditorFlash::EditorNotFound("no editor configured".into()));
            return Ok(());
        }
    };

    let emulator = editor::detect_terminal_emulator();
    let kind = classify(&editor_cmd);

    if emulator == TerminalEmulator::Termux && kind == EditorKind::Terminal {
        let allowed = match app.termux_external_apps() {
            Some(v) => v,
            None => {
                let v = check_termux_external_apps();
                app.set_termux_external_apps(v);
                v
            }
        };
        if !allowed {
            app.set_editor_flash(EditorFlash::TermuxPermission);
        }
    }

    let effective_emulator =
        if emulator == TerminalEmulator::Termux && app.termux_external_apps() == Some(false) {
            TerminalEmulator::Unknown
        } else {
            emulator
        };

    match open_in_editor(&editor_cmd, &filepath, kind, &effective_emulator) {
        Ok(EditorResult::Opened) => {
            let name = editor::binary_name(&editor_cmd).to_string();
            app.set_editor_flash(EditorFlash::Opened(name));
        }
        Ok(EditorResult::NeedsSameTerminal) => {
            let (bin, args) = split_editor_cmd(&editor_cmd);
            crossterm::terminal::disable_raw_mode()?;
            crossterm::execute!(io::stdout(), crossterm::terminal::LeaveAlternateScreen)?;

            let status = std::process::Command::new(bin)
                .args(&args)
                .arg(&filepath)
                .status();

            crossterm::terminal::enable_raw_mode()?;
            crossterm::execute!(io::stdout(), crossterm::terminal::EnterAlternateScreen)?;
            terminal.clear()?;
            app.reload(ss, themes);

            if let Err(e) = status {
                app.set_editor_flash(EditorFlash::EditorNotFound(format!("{bin}: {e}")));
            }
        }
        Err(msg) => {
            app.set_editor_flash(EditorFlash::EditorNotFound(msg));
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
    Ok(sync_render_width_for_app(
        area.width as usize,
        app,
        ss,
        themes,
    ))
}

fn sync_render_width_for_app(
    area_width: usize,
    app: &mut App,
    ss: &SyntaxSet,
    themes: &ThemeSet,
) -> bool {
    let content_width = if app.is_toc_visible() && app.has_toc() {
        area_width.saturating_sub(30)
    } else {
        area_width
    };
    let effective_width = content_width
        .saturating_sub(CONTENT_HORIZONTAL_PADDING as usize * 2)
        .saturating_sub(SCROLLBAR_WIDTH as usize);
    app.sync_render_width(effective_width, ss, themes)
}
