use crate::{
    markdown::{
        build_plain_lines, hash_file_contents, hash_str, parse_markdown_with_width, read_file_state,
    },
    render::{build_status_bar, build_toc_line_with_index, toc_header_line},
    theme::{
        current_syntect_theme, current_theme_preset, set_theme_preset, theme_preset_index,
        ThemePreset, THEME_PRESETS,
    },
};
use ratatui::text::Line;
use std::{
    path::PathBuf,
    time::{Duration, Instant, SystemTime},
};
use syntect::{highlighting::ThemeSet, parsing::SyntaxSet};

#[derive(Clone)]
pub(crate) struct TocEntry {
    pub(crate) level: u8,
    pub(crate) title: String,
    pub(crate) line: usize,
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
    pub(crate) pct: u16,
    pub(crate) search_mode: bool,
    pub(crate) search_draft_hash: u64,
    pub(crate) search_query_hash: u64,
    pub(crate) search_draft_len: usize,
    pub(crate) search_query_len: usize,
    pub(crate) search_match_count: usize,
    pub(crate) search_idx: usize,
    pub(crate) watch: bool,
    pub(crate) flash_active: bool,
}

#[derive(Clone)]
pub(crate) struct ThemePreviewCacheEntry {
    pub(crate) lines: Vec<Line<'static>>,
    pub(crate) toc: Vec<TocEntry>,
}

pub(crate) struct App {
    pub(crate) lines: Vec<Line<'static>>,
    pub(crate) plain_lines: Vec<String>,
    pub(crate) folded_plain_lines: Option<Vec<String>>,
    pub(crate) scroll: usize,
    pub(crate) toc: Vec<TocEntry>,
    pub(crate) toc_visible: bool,
    pub(crate) search_mode: bool,
    pub(crate) search_draft: String,
    pub(crate) search_query: String,
    pub(crate) search_matches: Vec<usize>,
    pub(crate) search_idx: usize,
    pub(crate) debug_input: bool,
    pub(crate) filename: String,
    pub(crate) source: String,
    pub(crate) watch: bool,
    pub(crate) filepath: Option<PathBuf>,
    pub(crate) last_file_state: Option<FileState>,
    pub(crate) last_content_hash: u64,
    pub(crate) last_hash_check: Option<Instant>,
    pub(crate) reload_flash: Option<Instant>,
    pub(crate) highlighted_line_cache: Option<(usize, Line<'static>)>,
    pub(crate) toc_display_lines: Vec<Line<'static>>,
    pub(crate) toc_header_line: Line<'static>,
    pub(crate) toc_active_idx: Option<usize>,
    pub(crate) status_line: Line<'static>,
    pub(crate) status_cache_key: Option<StatusCacheKey>,
    pub(crate) theme_picker_open: bool,
    pub(crate) theme_picker_index: usize,
    pub(crate) theme_picker_original: Option<ThemePreset>,
    pub(crate) theme_preview_cache: Vec<Option<ThemePreviewCacheEntry>>,
    pub(crate) render_width: usize,
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
            filename,
            source,
            debug_input,
            watch,
            filepath,
            last_file_state,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new_with_source(
        lines: Vec<Line<'static>>,
        toc: Vec<TocEntry>,
        filename: String,
        source: String,
        debug_input: bool,
        watch: bool,
        filepath: Option<PathBuf>,
        last_file_state: Option<FileState>,
    ) -> Self {
        let plain_lines = build_plain_lines(&lines);
        let mut app = Self {
            lines,
            plain_lines,
            folded_plain_lines: None,
            scroll: 0,
            toc,
            toc_visible: false,
            search_mode: false,
            search_draft: String::new(),
            search_query: String::new(),
            search_matches: vec![],
            search_idx: 0,
            debug_input,
            filename,
            source,
            watch,
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
            theme_picker_open: false,
            theme_picker_index: theme_preset_index(current_theme_preset()),
            theme_picker_original: None,
            theme_preview_cache: vec![None; crate::theme::THEME_PRESETS.len()],
            render_width: 80,
        };
        app.store_current_theme_preview();
        app.refresh_static_caches();
        app
    }

    pub(crate) fn set_last_content_hash(&mut self, last_content_hash: u64) {
        self.last_content_hash = last_content_hash;
    }

    pub(crate) fn total(&self) -> usize {
        self.lines.len()
    }

    pub(crate) fn replace_content(&mut self, lines: Vec<Line<'static>>, toc: Vec<TocEntry>) {
        self.plain_lines = build_plain_lines(&lines);
        self.folded_plain_lines = None;
        self.lines = lines;
        self.toc = toc;
        self.highlighted_line_cache = None;
        self.toc_header_line = toc_header_line();
        self.refresh_static_caches();
    }

    pub(crate) fn active_highlight_line(&self) -> Option<usize> {
        if self.search_matches.is_empty() {
            None
        } else {
            Some(self.search_matches[self.search_idx])
        }
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

    pub(crate) fn folded_plain_lines(&mut self) -> &[String] {
        if self.folded_plain_lines.is_none() {
            self.folded_plain_lines = Some(
                self.plain_lines
                    .iter()
                    .map(|line| line.to_lowercase())
                    .collect(),
            );
        }
        self.folded_plain_lines.as_deref().unwrap_or(&[])
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
            search_mode: self.search_mode,
            search_draft_hash: hash_str(&self.search_draft),
            search_query_hash: hash_str(&self.search_query),
            search_draft_len: self.search_draft.len(),
            search_query_len: self.search_query.len(),
            search_match_count: self.search_matches.len(),
            search_idx: self.search_idx,
            watch: self.watch,
            flash_active: self
                .reload_flash
                .map(|t| t.elapsed() < Duration::from_millis(1500))
                .unwrap_or(false),
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

    pub(crate) fn invalidate_theme_preview_cache(&mut self) {
        self.theme_preview_cache.fill(None);
    }

    fn store_theme_preview(
        &mut self,
        preset: ThemePreset,
        lines: &[Line<'static>],
        toc: &[TocEntry],
    ) {
        let idx = theme_preset_index(preset);
        if let Some(slot) = self.theme_preview_cache.get_mut(idx) {
            *slot = Some(ThemePreviewCacheEntry {
                lines: lines.to_vec(),
                toc: toc.to_vec(),
            });
        }
    }

    fn store_current_theme_preview(&mut self) {
        let preset = current_theme_preset();
        let lines = self.lines.clone();
        let toc = self.toc.clone();
        self.store_theme_preview(preset, &lines, &toc);
    }

    pub(crate) fn open_theme_picker(&mut self) {
        self.theme_picker_open = true;
        let current = current_theme_preset();
        self.theme_picker_index = theme_preset_index(current);
        self.theme_picker_original = Some(current);
        self.store_current_theme_preview();
    }

    pub(crate) fn close_theme_picker(&mut self) {
        self.theme_picker_open = false;
        self.theme_picker_original = None;
    }

    pub(crate) fn is_theme_picker_open(&self) -> bool {
        self.theme_picker_open
    }

    pub(crate) fn theme_picker_index(&self) -> usize {
        self.theme_picker_index
    }

    pub(crate) fn theme_picker_reference_preset(&self) -> ThemePreset {
        self.theme_picker_original.unwrap_or(current_theme_preset())
    }

    pub(crate) fn move_theme_picker_up(&mut self) {
        let total = THEME_PRESETS.len();
        if total == 0 {
            return;
        }
        if self.theme_picker_index == 0 {
            self.theme_picker_index = total - 1;
        } else {
            self.theme_picker_index -= 1;
        }
    }

    pub(crate) fn move_theme_picker_down(&mut self) {
        let total = THEME_PRESETS.len();
        if total == 0 {
            return;
        }
        self.theme_picker_index = (self.theme_picker_index + 1) % total;
    }

    pub(crate) fn set_theme_picker_index(&mut self, idx: usize) -> bool {
        if idx < THEME_PRESETS.len() {
            self.theme_picker_index = idx;
            true
        } else {
            false
        }
    }

    pub(crate) fn selected_theme_preset(&self) -> Option<ThemePreset> {
        THEME_PRESETS.get(self.theme_picker_index).copied()
    }

    pub(crate) fn preview_theme_preset(
        &mut self,
        preset: ThemePreset,
        ss: &SyntaxSet,
        themes: &ThemeSet,
    ) {
        if current_theme_preset() == preset {
            return;
        }
        set_theme_preset(preset);
        let cached = self
            .theme_preview_cache
            .get(theme_preset_index(preset))
            .and_then(|entry| entry.as_ref())
            .cloned();
        if let Some(entry) = cached {
            self.replace_content(entry.lines, entry.toc);
            return;
        }

        let theme = current_syntect_theme(themes);
        let (new_lines, new_toc) =
            parse_markdown_with_width(&self.source, ss, theme, self.render_width);
        self.store_theme_preview(preset, &new_lines, &new_toc);
        self.replace_content(new_lines, new_toc);
    }

    pub(crate) fn restore_theme_picker_preview(&mut self, ss: &SyntaxSet, themes: &ThemeSet) {
        if let Some(original) = self.theme_picker_original {
            self.preview_theme_preset(original, ss, themes);
        }
        self.close_theme_picker();
    }

    pub(crate) fn scroll_down(&mut self, n: usize) {
        self.scroll = (self.scroll + n).min(self.total().saturating_sub(1));
    }

    pub(crate) fn scroll_up(&mut self, n: usize) {
        self.scroll = self.scroll.saturating_sub(n);
    }

    pub(crate) fn jump_to_toc(&mut self, idx: usize) {
        if let Some(e) = self.toc.get(idx) {
            self.scroll = e.line;
        }
    }

    pub(crate) fn run_search(&mut self) {
        let q = self.search_query.to_lowercase();
        if q.is_empty() {
            return;
        }
        let search_matches = {
            let folded_lines = self.folded_plain_lines();
            folded_lines
                .iter()
                .enumerate()
                .filter(|(_, line)| line.contains(&q))
                .map(|(i, _)| i)
                .collect()
        };
        self.search_matches = search_matches;
        self.search_idx = 0;
        if let Some(&f) = self.search_matches.first() {
            self.scroll = f;
        }
    }

    pub(crate) fn begin_search(&mut self) {
        self.search_mode = true;
        self.search_draft = self.search_query.clone();
        crate::runtime::debug_log(
            self.debug_input,
            &format!(
                "begin_search query={:?} draft={:?} matches={} idx={}",
                self.search_query,
                self.search_draft,
                self.search_matches.len(),
                self.search_idx
            ),
        );
    }

    pub(crate) fn reset_search_state(&mut self) {
        self.search_draft.clear();
        self.search_query.clear();
        self.search_matches.clear();
        self.search_idx = 0;
    }

    pub(crate) fn cancel_search(&mut self) {
        self.search_mode = false;
        self.reset_search_state();
        crate::runtime::debug_log(self.debug_input, "cancel_search cleared query and matches");
    }

    pub(crate) fn confirm_search(&mut self) {
        self.search_mode = false;
        let draft = std::mem::take(&mut self.search_draft);
        self.search_query = draft;
        if self.search_query.is_empty() {
            self.reset_search_state();
            crate::runtime::debug_log(
                self.debug_input,
                "confirm_search empty query -> cleared matches",
            );
            return;
        }
        self.run_search();
        crate::runtime::debug_log(
            self.debug_input,
            &format!(
                "confirm_search query={:?} matches={} idx={} scroll={}",
                self.search_query,
                self.search_matches.len(),
                self.search_idx,
                self.scroll
            ),
        );
    }

    pub(crate) fn clear_active_search(&mut self) {
        self.search_mode = false;
        self.reset_search_state();
        crate::runtime::debug_log(
            self.debug_input,
            "clear_active_search cleared query and matches",
        );
    }

    pub(crate) fn has_active_search(&self) -> bool {
        !self.search_query.is_empty() || !self.search_matches.is_empty()
    }

    pub(crate) fn next_match(&mut self) {
        if self.search_matches.is_empty() {
            return;
        }
        self.search_idx = (self.search_idx + 1) % self.search_matches.len();
        self.scroll = self.search_matches[self.search_idx];
    }

    pub(crate) fn prev_match(&mut self) {
        if self.search_matches.is_empty() {
            return;
        }
        if self.search_idx == 0 {
            self.search_idx = self.search_matches.len() - 1;
        } else {
            self.search_idx -= 1;
        }
        self.scroll = self.search_matches[self.search_idx];
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
        if !self.search_query.is_empty() && !self.search_mode {
            self.run_search();
        }
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
        self.reload_flash = Some(Instant::now());
        true
    }
}

pub(crate) fn should_hide_single_h1(toc: &[TocEntry]) -> bool {
    let h1_count = toc.iter().filter(|entry| entry.level == 1).count();
    let has_h2 = toc.iter().any(|entry| entry.level == 2);
    h1_count == 1 && has_h2
}

pub(crate) fn should_promote_h2_when_no_h1(toc: &[TocEntry]) -> bool {
    !toc.iter().any(|entry| entry.level == 1) && toc.iter().any(|entry| entry.level == 2)
}

pub(crate) fn toc_display_level(level: u8, hide_single_h1: bool, promote_h2_root: bool) -> u8 {
    if hide_single_h1 || promote_h2_root {
        match level {
            2 => 1,
            3 => 2,
            _ => level,
        }
    } else {
        level
    }
}

pub(crate) fn normalize_toc(mut toc: Vec<TocEntry>) -> Vec<TocEntry> {
    if should_hide_single_h1(&toc) || should_promote_h2_when_no_h1(&toc) {
        toc.retain(|entry| matches!(entry.level, 1..=3));
    } else {
        toc.retain(|entry| matches!(entry.level, 1..=2));
    }
    toc
}
