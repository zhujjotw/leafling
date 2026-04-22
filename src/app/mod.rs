use crate::{
    markdown::{
        build_searchable_lines, hash_file_contents, hash_str, parse_markdown_with_width,
        read_file_state,
        toc::{should_hide_single_h1, should_promote_h2_when_no_h1, toc_display_level, TocEntry},
    },
    render::{build_status_bar, build_toc_line_with_index, toc_header_line},
    theme::{current_syntect_theme, current_theme_preset, theme_preset_index},
};
use ratatui::{layout::Rect, text::Line};
use std::{
    path::PathBuf,
    time::{Duration, Instant, SystemTime},
};
use syntect::{highlighting::ThemeSet, parsing::SyntaxSet};

pub(crate) const FLASH_DURATION_MS: u64 = 1500;

pub(super) mod search;
pub(crate) use search::SearchState;

pub(crate) mod file_picker;
mod fuzzy;
pub(crate) use file_picker::{FilePickerMode, FilePickerState, PickerIndexTruncation};
use file_picker::{PendingPicker, PickerLoadState};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum EditorFlash {
    Opened(String),
    NoFile,
    EditorNotFound(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum WatchFlash {
    Activated,
    Deactivated,
    Stdin,
    NoFile,
    FileNotFound,
    NotActive,
}

pub(super) mod theme_picker;
pub(crate) use theme_picker::ThemePickerState;

use crate::editor::EditorEntry;

pub(crate) struct EditorPickerState {
    pub(super) open: bool,
    pub(super) editors: Vec<EditorEntry>,
    pub(super) index: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FileState {
    pub(crate) modified: SystemTime,
    pub(crate) len: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FileChange {
    Metadata(FileState),
    Content(FileState),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct StatusCacheKey {
    pct: u16,
    search_mode: bool,
    search_draft_hash: u64,
    search_query_hash: u64,
    search_draft_len: usize,
    search_query_len: usize,
    search_match_count: usize,
    search_idx: usize,
    watch: bool,
    flash_active: bool,
    editor_flash_active: bool,
    file_picker_open: bool,
    picker_loading: bool,
    watch_flash_active: bool,
    watch_error: bool,
}

pub(crate) struct AppConfig {
    pub(crate) filename: String,
    pub(crate) source: String,
    pub(crate) debug_input: bool,
    pub(crate) watch: bool,
    pub(crate) filepath: Option<PathBuf>,
    pub(crate) last_file_state: Option<FileState>,
}

pub(crate) struct App {
    pub(super) lines: Vec<Line<'static>>,
    pub(super) plain_lines: Vec<String>,
    pub(super) scroll: usize,
    pub(super) toc: Vec<TocEntry>,
    toc_visible: bool,
    pub(super) search: SearchState,
    pub(super) debug_input: bool,
    pub(super) filename: String,
    pub(super) source: String,
    watch: bool,
    watch_error: bool,
    pub(super) filepath: Option<PathBuf>,
    pub(super) last_file_state: Option<FileState>,
    pub(super) last_content_hash: u64,
    pub(super) last_hash_check: Option<Instant>,
    pub(super) reload_flash: Option<Instant>,
    highlighted_line_cache: Option<(usize, Line<'static>)>,
    toc_display_lines: Vec<Line<'static>>,
    toc_header_line: Line<'static>,
    toc_active_idx: Option<usize>,
    status_line: Line<'static>,
    status_cache_key: Option<StatusCacheKey>,
    pub(super) help_open: bool,
    pub(super) path_popup_open: bool,
    pub(super) file_picker: FilePickerState,
    pub(super) pending_picker: PendingPicker,
    pub(super) picker_load_state: PickerLoadState,
    pub(super) theme_picker: ThemePickerState,
    pub(super) editor_picker: EditorPickerState,
    pub(super) render_width: usize,
    pub(crate) content_area: Rect,
    pub(crate) mouse_position: (u16, u16),
    pub(crate) scrollbar_dragging: bool,
    pub(super) editor_config: Option<String>,
    pub(super) editor_flash: Option<(EditorFlash, Instant)>,
    watch_flash: Option<(WatchFlash, Instant)>,
}

impl App {
    #[cfg(test)]
    pub(crate) fn new(
        lines: Vec<Line<'static>>,
        toc: Vec<TocEntry>,
        filename: String,
        debug_input: bool,
        watch: bool,
        filepath: Option<PathBuf>,
        last_file_state: Option<FileState>,
    ) -> Self {
        let source = lines
            .iter()
            .map(|line| {
                line.spans
                    .iter()
                    .map(|s| s.content.as_ref())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n");
        Self::new_with_source(
            lines,
            toc,
            AppConfig {
                filename,
                source,
                debug_input,
                watch,
                filepath,
                last_file_state,
            },
        )
    }

    pub(crate) fn new_with_source(
        lines: Vec<Line<'static>>,
        toc: Vec<TocEntry>,
        config: AppConfig,
    ) -> Self {
        let AppConfig {
            filename,
            source,
            debug_input,
            watch,
            filepath,
            last_file_state,
        } = config;
        let plain_lines = build_searchable_lines(&lines)
            .into_iter()
            .map(|line| line.to_lowercase())
            .collect();
        let mut app = Self {
            lines,
            plain_lines,
            scroll: 0,
            toc,
            toc_visible: false,
            search: SearchState {
                mode: false,
                draft: String::new(),
                query: String::new(),
                matches: vec![],
                idx: 0,
                draft_hash: 0,
                query_hash: 0,
            },
            debug_input,
            filename,
            source,
            watch,
            watch_error: false,
            filepath,
            last_file_state,
            last_content_hash: 0,
            last_hash_check: None,
            reload_flash: None,
            highlighted_line_cache: None,
            toc_display_lines: Vec::new(),
            toc_header_line: toc_header_line(),
            toc_active_idx: None,
            status_line: Line::default(),
            status_cache_key: None,
            help_open: false,
            path_popup_open: false,
            file_picker: FilePickerState {
                open: false,
                mode: FilePickerMode::Browser,
                dir: PathBuf::from("."),
                entries: Vec::new(),
                filtered: Vec::new(),
                match_positions: Vec::new(),
                index: 0,
                query: String::new(),
                truncation: None,
            },
            pending_picker: PendingPicker::None,
            picker_load_state: PickerLoadState::Idle,
            theme_picker: ThemePickerState {
                open: false,
                index: theme_preset_index(current_theme_preset()),
                original: None,
                preview_cache: vec![None; crate::theme::THEME_PRESETS.len()],
            },
            editor_picker: EditorPickerState {
                open: false,
                editors: Vec::new(),
                index: 0,
            },
            render_width: 80,
            content_area: Rect::default(),
            mouse_position: (0, 0),
            scrollbar_dragging: false,
            editor_config: None,
            editor_flash: None,
            watch_flash: None,
        };
        app.store_current_theme_preview();
        app.refresh_static_caches();
        app
    }

    pub(crate) fn set_last_content_hash(&mut self, last_content_hash: u64) {
        self.last_content_hash = last_content_hash;
    }

    pub(crate) fn is_watch_enabled(&self) -> bool {
        self.watch
    }

    pub(crate) fn is_watch_error(&self) -> bool {
        self.watch_error
    }

    pub(crate) fn set_watch_error(&mut self, error: bool) {
        self.watch_error = error;
    }

    pub(crate) fn debug_input_enabled(&self) -> bool {
        self.debug_input
    }

    pub(crate) fn is_toc_visible(&self) -> bool {
        self.toc_visible
    }

    pub(crate) fn has_toc(&self) -> bool {
        !self.toc.is_empty()
    }

    // Always >= 5 (scroll padding).
    // Use has_content() to check for actual content.
    pub(crate) fn total(&self) -> usize {
        self.lines.len()
    }

    pub(crate) fn scroll(&self) -> usize {
        self.scroll
    }

    pub(crate) fn visible_lines(&self, start: usize, end: usize) -> &[Line<'static>] {
        &self.lines[start..end]
    }

    pub(crate) fn highlighted_line_cache(&self) -> Option<&(usize, Line<'static>)> {
        self.highlighted_line_cache.as_ref()
    }

    pub(crate) fn toc_display_lines(&self) -> &[Line<'static>] {
        &self.toc_display_lines
    }

    pub(crate) fn toc_header_line(&self) -> &Line<'static> {
        &self.toc_header_line
    }

    pub(crate) fn status_line(&self) -> &Line<'static> {
        &self.status_line
    }

    pub(crate) fn filename(&self) -> &str {
        &self.filename
    }

    pub(crate) fn replace_content(&mut self, lines: Vec<Line<'static>>, toc: Vec<TocEntry>) {
        self.plain_lines = build_searchable_lines(&lines)
            .into_iter()
            .map(|line| line.to_lowercase())
            .collect();
        self.lines = lines;
        self.toc = toc;
        self.highlighted_line_cache = None;
        self.toc_header_line = toc_header_line();
        self.refresh_static_caches();
    }

    #[cfg(test)]
    pub(crate) fn line(&self, idx: usize) -> Option<&Line<'static>> {
        self.lines.get(idx)
    }

    pub(crate) fn active_toc_index(&self) -> Option<usize> {
        let hide_single_h1 = should_hide_single_h1(&self.toc);
        let mut first_visible = None;
        let mut active = None;
        for (idx, entry) in self
            .toc
            .iter()
            .enumerate()
            .filter(|(_, entry)| !(hide_single_h1 && entry.level == 1))
        {
            if first_visible.is_none() {
                first_visible = Some((idx, entry.line));
            }
            if entry.line > self.scroll {
                break;
            }
            active = Some(idx);
        }

        let (first_idx, first_line) = first_visible?;
        if self.scroll < first_line {
            Some(first_idx)
        } else {
            active.or(Some(first_idx))
        }
    }

    pub(crate) fn refresh_highlighted_line_cache(&mut self, line_idx: usize) -> Option<()> {
        let needs_refresh = self
            .highlighted_line_cache
            .as_ref()
            .map(|(cached_idx, _)| *cached_idx != line_idx)
            .unwrap_or(true);
        if needs_refresh {
            let line = self.lines.get(line_idx)?;
            self.highlighted_line_cache = Some((line_idx, crate::markdown::highlight_line(line)));
        }
        Some(())
    }

    pub(crate) fn refresh_toc_cache(&mut self) {
        let hide_single_h1 = should_hide_single_h1(&self.toc);
        let promote_h2_root = should_promote_h2_when_no_h1(&self.toc);
        let active_idx = self.active_toc_index();
        if self.toc_active_idx == active_idx && !self.toc_display_lines.is_empty() {
            return;
        }

        self.toc_active_idx = active_idx;
        let mut top_level_index = 0usize;
        self.toc_display_lines = self
            .toc
            .iter()
            .enumerate()
            .filter(|(_, entry)| !(hide_single_h1 && entry.level == 1))
            .map(|(idx, entry)| {
                let display_level = toc_display_level(entry.level, hide_single_h1, promote_h2_root);
                let line = build_toc_line_with_index(
                    entry,
                    display_level,
                    (display_level == 1).then_some(top_level_index),
                    active_idx == Some(idx),
                );
                if display_level == 1 {
                    top_level_index += 1;
                }
                line
            })
            .collect();
    }

    pub(crate) fn refresh_status_cache(&mut self, pct: u16) {
        let cache_key = StatusCacheKey {
            pct,
            search_mode: self.search.mode,
            search_draft_hash: self.search.draft_hash,
            search_query_hash: self.search.query_hash,
            search_draft_len: self.search.draft.len(),
            search_query_len: self.search.query.len(),
            search_match_count: self.search.matches.len(),
            search_idx: self.search.idx,
            watch: self.watch,
            flash_active: self
                .reload_flash
                .map(|t| t.elapsed() < Duration::from_millis(FLASH_DURATION_MS))
                .unwrap_or(false),
            editor_flash_active: self
                .editor_flash
                .as_ref()
                .map(|(_, t)| t.elapsed() < Duration::from_millis(FLASH_DURATION_MS))
                .unwrap_or(false),
            file_picker_open: self.is_file_picker_open(),
            picker_loading: self.is_picker_loading(),
            watch_flash_active: self
                .watch_flash
                .as_ref()
                .map(|(_, t)| t.elapsed() < Duration::from_millis(FLASH_DURATION_MS))
                .unwrap_or(false),
            watch_error: self.watch_error,
        };

        if self.status_cache_key.as_ref() == Some(&cache_key) {
            return;
        }

        self.status_line = Line::from(build_status_bar(self, pct));
        self.status_cache_key = Some(cache_key);
    }

    pub(crate) fn refresh_static_caches(&mut self) {
        self.toc_active_idx = None;
        self.toc_display_lines.clear();
        self.refresh_toc_cache();
        self.status_cache_key = None;
    }

    pub(crate) fn open_help(&mut self) {
        self.help_open = true;
    }

    pub(crate) fn close_help(&mut self) {
        self.help_open = false;
    }

    pub(crate) fn is_help_open(&self) -> bool {
        self.help_open
    }

    pub(crate) fn open_path_popup(&mut self) {
        self.path_popup_open = true;
    }

    pub(crate) fn close_path_popup(&mut self) {
        self.path_popup_open = false;
    }

    pub(crate) fn is_path_popup_open(&self) -> bool {
        self.path_popup_open
    }

    pub(crate) fn is_popup_open(&self) -> bool {
        self.help_open
            || self.path_popup_open
            || self.file_picker.open
            || self.theme_picker.open
            || self.editor_picker.open
            || self.is_picker_loading()
            || self.is_picker_load_failed()
    }

    pub(crate) fn clear_reload_flash(&mut self) {
        self.reload_flash = None;
    }

    pub(crate) fn reload_flash_started(&self) -> Option<Instant> {
        self.reload_flash
    }

    pub(crate) fn set_editor_config(&mut self, editor: Option<String>) {
        self.editor_config = editor;
    }

    pub(crate) fn editor_config(&self) -> Option<&str> {
        self.editor_config.as_deref()
    }

    pub(crate) fn set_editor_flash(&mut self, flash: EditorFlash) {
        self.editor_flash = Some((flash, Instant::now()));
    }

    pub(crate) fn editor_flash(&self) -> Option<&(EditorFlash, Instant)> {
        self.editor_flash.as_ref()
    }

    pub(crate) fn clear_editor_flash(&mut self) {
        self.editor_flash = None;
    }

    pub(crate) fn toggle_watch(&mut self) {
        let p = match &self.filepath {
            None => {
                self.set_watch_flash(if self.filename == "stdin" {
                    WatchFlash::Stdin
                } else {
                    WatchFlash::NoFile
                });
                return;
            }
            Some(p) => p,
        };
        if !p.exists() {
            self.set_watch_flash(WatchFlash::FileNotFound);
            return;
        }
        self.watch = !self.watch;
        self.set_watch_flash(if self.watch {
            WatchFlash::Activated
        } else {
            WatchFlash::Deactivated
        });
        if self.watch {
            self.last_file_state = None;
            self.last_content_hash = hash_str(&self.source);
            self.last_hash_check = Some(Instant::now());
            self.watch_error = false;
        }
    }

    pub(crate) fn watch_flash(&self) -> Option<(&WatchFlash, &Instant)> {
        self.watch_flash.as_ref().map(|(f, t)| (f, t))
    }

    pub(crate) fn set_watch_flash(&mut self, flash: WatchFlash) {
        self.watch_flash = Some((flash, Instant::now()));
    }

    pub(crate) fn watch_flash_for_no_file(&self) -> WatchFlash {
        if self.filename == "stdin" {
            WatchFlash::Stdin
        } else {
            WatchFlash::NoFile
        }
    }

    pub(crate) fn clear_watch_flash(&mut self) {
        self.watch_flash = None;
    }

    pub(crate) fn filepath(&self) -> Option<&std::path::Path> {
        self.filepath.as_deref()
    }

    pub(crate) fn has_content(&self) -> bool {
        self.filepath.is_some() || !self.source.is_empty()
    }

    pub(crate) fn picker_dir(&self) -> PathBuf {
        std::env::current_dir()
            .ok()
            .or_else(|| {
                self.filepath
                    .as_ref()
                    .and_then(|p| p.parent().map(|d| d.to_path_buf()))
            })
            .unwrap_or_default()
    }

    pub(crate) fn open_editor_picker(&mut self) {
        let editors = crate::editor::scan_available_editors();
        let current = self
            .editor_config
            .as_deref()
            .map(crate::editor::binary_name);
        let index = current
            .and_then(|bin| {
                editors
                    .iter()
                    .position(|e| crate::editor::binary_name(&e.command) == bin)
            })
            .unwrap_or(0);
        self.editor_picker.editors = editors;
        self.editor_picker.index = index;
        self.editor_picker.open = true;
    }

    pub(crate) fn close_editor_picker(&mut self) {
        if let Some(entry) = self.editor_picker.editors.get(self.editor_picker.index) {
            self.editor_config = Some(entry.command.clone());
        }
        self.editor_picker.open = false;
    }

    pub(crate) fn cancel_editor_picker(&mut self) {
        self.editor_picker.open = false;
    }

    pub(crate) fn is_editor_picker_open(&self) -> bool {
        self.editor_picker.open
    }

    pub(crate) fn move_editor_picker_up(&mut self) {
        let len = self.editor_picker.editors.len();
        if len > 0 {
            self.editor_picker.index = (self.editor_picker.index + len - 1) % len;
        }
    }

    pub(crate) fn move_editor_picker_down(&mut self) {
        let len = self.editor_picker.editors.len();
        if len > 0 {
            self.editor_picker.index = (self.editor_picker.index + 1) % len;
        }
    }

    pub(crate) fn editor_picker_index(&self) -> usize {
        self.editor_picker.index
    }

    pub(crate) fn editor_picker_entries(&self) -> &[EditorEntry] {
        &self.editor_picker.editors
    }

    pub(crate) fn set_last_file_state(&mut self, state: FileState) {
        self.last_file_state = Some(state);
    }

    pub(crate) fn scroll_down(&mut self, n: usize) {
        self.scroll = (self.scroll + n).min(self.total().saturating_sub(1));
    }

    pub(crate) fn scroll_up(&mut self, n: usize) {
        self.scroll = self.scroll.saturating_sub(n);
    }

    pub(crate) fn scroll_top(&mut self) {
        self.scroll = 0;
    }

    pub(crate) fn scroll_bottom(&mut self) {
        self.scroll = self.total().saturating_sub(1);
    }

    pub(crate) fn scroll_to(&mut self, position: usize) {
        self.scroll = position.min(self.total().saturating_sub(1));
    }

    pub(crate) fn toggle_toc(&mut self) {
        self.toc_visible = !self.toc_visible;
    }

    pub(crate) fn request_reload(&mut self, ss: &SyntaxSet, themes: &ThemeSet) -> bool {
        self.last_file_state = None;
        self.reload(ss, themes)
    }

    pub(crate) fn jump_to_toc(&mut self, idx: usize) {
        if let Some(e) = self.toc.get(idx) {
            self.scroll = e.line;
        }
    }

    pub(crate) fn scroll_percent(&self, vh: usize) -> u16 {
        if self.total() <= vh {
            return 100;
        }
        ((self.scroll * 100) / (self.total() - vh).max(1)) as u16
    }

    pub(crate) fn sync_render_width(
        &mut self,
        render_width: usize,
        ss: &SyntaxSet,
        themes: &ThemeSet,
    ) -> bool {
        let next_width = render_width.max(20);
        if self.render_width == next_width {
            return false;
        }
        self.render_width = next_width;
        self.reparse_source(ss, themes);
        true
    }

    pub(crate) fn check_modified(&mut self) -> Option<FileChange> {
        const HASH_FALLBACK_INTERVAL: Duration = Duration::from_secs(2);

        let path = self.filepath.as_ref()?;
        let state = read_file_state(path)?;
        match self.last_file_state {
            Some(prev) if state.modified != prev.modified || state.len != prev.len => {
                Some(FileChange::Metadata(state))
            }
            Some(_) => {
                let should_hash = self
                    .last_hash_check
                    .map(|checked_at| checked_at.elapsed() >= HASH_FALLBACK_INTERVAL)
                    .unwrap_or(true);
                if !should_hash {
                    return None;
                }
                self.last_hash_check = Some(Instant::now());
                let current_hash = hash_file_contents(path).ok()?;
                (current_hash != self.last_content_hash).then_some(FileChange::Content(state))
            }
            None => Some(FileChange::Metadata(state)),
        }
    }

    pub(crate) fn reparse_source(&mut self, ss: &SyntaxSet, themes: &ThemeSet) {
        let theme = current_syntect_theme(themes);
        let old_total = self.total();
        let (new_lines, new_toc) =
            parse_markdown_with_width(&self.source, ss, theme, self.render_width);
        let new_total = new_lines.len();

        if old_total > 0 {
            self.scroll = ((self.scroll as f64 / old_total as f64) * new_total as f64) as usize;
            self.scroll = self.scroll.min(new_total.saturating_sub(1));
        }

        self.invalidate_theme_preview_cache();
        self.store_theme_preview(current_theme_preset(), &new_lines, &new_toc);
        self.replace_content(new_lines, new_toc);
        if !self.search.query.is_empty() && !self.search.mode {
            self.run_search();
        }
    }

    pub(crate) fn load_path(&mut self, path: PathBuf, ss: &SyntaxSet, themes: &ThemeSet) -> bool {
        let src = match std::fs::read_to_string(&path) {
            Ok(src) => src,
            Err(_) => return false,
        };
        let filename = path
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| path.display().to_string());
        let file_state = read_file_state(&path);
        let content_hash = hash_str(&src);
        let theme = current_syntect_theme(themes);
        let (lines, toc) = parse_markdown_with_width(&src, ss, theme, self.render_width);

        self.filename = filename;
        self.source = src;
        self.filepath = Some(path);
        self.last_file_state = file_state;
        self.last_content_hash = content_hash;
        self.last_hash_check = Some(Instant::now());
        self.reload_flash = None;
        self.scroll = 0;
        self.help_open = false;
        self.file_picker.open = false;
        self.theme_picker.open = false;
        self.search.mode = false;
        self.reset_search_state();
        self.invalidate_theme_preview_cache();
        self.store_theme_preview(current_theme_preset(), &lines, &toc);
        self.replace_content(lines, toc);
        true
    }

    pub(crate) fn reload(&mut self, ss: &SyntaxSet, themes: &ThemeSet) -> bool {
        let path = match &self.filepath {
            Some(p) => p,
            None => return false,
        };
        let src = match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(_) => return false,
        };
        let file_state = read_file_state(path);
        let content_hash = hash_str(&src);
        self.source = src;

        self.reparse_source(ss, themes);
        self.last_file_state = file_state;
        self.last_content_hash = content_hash;
        self.last_hash_check = Some(Instant::now());
        if self.watch_flash.is_none() {
            self.reload_flash = Some(Instant::now());
        }
        true
    }
}
