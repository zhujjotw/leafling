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
    fs,
    path::PathBuf,
    sync::mpsc::{self, Receiver, TryRecvError},
    thread,
    time::{Duration, Instant, SystemTime},
};
use syntect::{highlighting::ThemeSet, parsing::SyntaxSet};

const MAX_FUZZY_PICKER_DIRS_VISITED: usize = 5_000;
const MAX_FUZZY_PICKER_FILES_INDEXED: usize = 10_000;
const MAX_FUZZY_PICKER_INDEX_DURATION: Duration = Duration::from_secs(5);
const IGNORED_FUZZY_PICKER_DIRS: &[&str] = &[
    ".git",
    "node_modules",
    "target",
    ".venv",
    "venv",
    "vendor",
    "var",
    "dist",
    "build",
    ".next",
    ".cache",
];

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
}

#[derive(Clone)]
pub(crate) struct ThemePreviewCacheEntry {
    lines: Vec<Line<'static>>,
    toc: Vec<TocEntry>,
}

pub(crate) struct SearchState {
    mode: bool,
    draft: String,
    query: String,
    matches: Vec<usize>,
    idx: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct FilePickerEntry {
    label: String,
    path: PathBuf,
    label_lower: String,
    file_name: String,
    file_name_lower: String,
    file_name_offset: usize,
    path_depth: usize,
}

impl FilePickerEntry {
    fn new(label: String, path: PathBuf) -> Self {
        let file_name = Self::file_name_component(&label).to_string();
        let file_name_offset = label
            .rfind(std::path::MAIN_SEPARATOR)
            .map(|idx| label[..idx + 1].chars().count())
            .unwrap_or(0);
        let path_depth = label.matches(std::path::MAIN_SEPARATOR).count();

        Self {
            label_lower: label.to_lowercase(),
            file_name_lower: file_name.to_lowercase(),
            label,
            path,
            file_name,
            file_name_offset,
            path_depth,
        }
    }

    pub(crate) fn label(&self) -> &str {
        &self.label
    }

    fn label_lower(&self) -> &str {
        &self.label_lower
    }

    fn file_name_lower(&self) -> &str {
        &self.file_name_lower
    }

    fn file_name_offset(&self) -> usize {
        self.file_name_offset
    }

    fn path_depth(&self) -> usize {
        self.path_depth
    }

    fn file_name_component(path: &str) -> &str {
        path.rsplit(std::path::MAIN_SEPARATOR)
            .next()
            .unwrap_or(path)
    }

    fn is_dir_like(&self) -> bool {
        self.label == ".." || self.label.ends_with('/')
    }
}

pub(crate) struct FilePickerState {
    open: bool,
    mode: FilePickerMode,
    dir: PathBuf,
    entries: Vec<FilePickerEntry>,
    filtered: Vec<usize>,
    match_positions: Vec<Vec<usize>>,
    index: usize,
    query: String,
    truncation: Option<PickerIndexTruncation>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FilePickerMode {
    Browser,
    Fuzzy,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum PendingPicker {
    None,
    Browser(PathBuf),
    Fuzzy(PathBuf),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PickerIndexTruncation {
    Directory,
    File,
    Time,
}

struct PickerIndexResult {
    entries: Vec<FilePickerEntry>,
    truncated: Option<PickerIndexTruncation>,
}

enum PickerLoadState {
    Idle,
    Loading {
        mode: FilePickerMode,
        dir: PathBuf,
        started_at: Instant,
        receiver: Receiver<std::io::Result<PickerIndexResult>>,
        pending_result: Option<std::io::Result<PickerIndexResult>>,
    },
    Failed {
        mode: FilePickerMode,
        dir: PathBuf,
        message: String,
    },
}

pub(crate) struct ThemePickerState {
    open: bool,
    index: usize,
    original: Option<ThemePreset>,
    preview_cache: Vec<Option<ThemePreviewCacheEntry>>,
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
    lines: Vec<Line<'static>>,
    plain_lines: Vec<String>,
    folded_plain_lines: Option<Vec<String>>,
    scroll: usize,
    toc: Vec<TocEntry>,
    toc_visible: bool,
    search: SearchState,
    debug_input: bool,
    filename: String,
    source: String,
    watch: bool,
    filepath: Option<PathBuf>,
    last_file_state: Option<FileState>,
    last_content_hash: u64,
    last_hash_check: Option<Instant>,
    reload_flash: Option<Instant>,
    highlighted_line_cache: Option<(usize, Line<'static>)>,
    toc_display_lines: Vec<Line<'static>>,
    toc_header_line: Line<'static>,
    toc_active_idx: Option<usize>,
    status_line: Line<'static>,
    status_cache_key: Option<StatusCacheKey>,
    help_open: bool,
    file_picker: FilePickerState,
    pending_picker: PendingPicker,
    picker_load_state: PickerLoadState,
    theme_picker: ThemePickerState,
    render_width: usize,
}

impl App {
    fn min_picker_loading_duration() -> Duration {
        Duration::from_millis(500)
    }

    fn is_markdown_path(path: &std::path::Path) -> bool {
        matches!(
            path.extension().and_then(|ext| ext.to_str()),
            Some("md" | "markdown" | "mdown" | "mkd")
        )
    }

    fn build_file_picker_entries(dir: &std::path::Path) -> std::io::Result<Vec<FilePickerEntry>> {
        let mut entries = Vec::new();

        if let Some(parent) = dir.parent() {
            entries.push(FilePickerEntry::new("..".to_string(), parent.to_path_buf()));
        }

        let mut dirs = Vec::new();
        let mut files = Vec::new();
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            let file_type = match entry.file_type() {
                Ok(file_type) => file_type,
                Err(_) => continue,
            };
            let name = entry.file_name().to_string_lossy().to_string();

            if file_type.is_dir() {
                dirs.push(FilePickerEntry::new(format!("{name}/"), path));
            } else if file_type.is_file() && Self::is_markdown_path(&path) {
                files.push(FilePickerEntry::new(name, path));
            }
        }

        dirs.sort_by(|left, right| left.label_lower().cmp(right.label_lower()));
        files.sort_by(|left, right| left.label_lower().cmp(right.label_lower()));
        entries.extend(dirs);
        entries.extend(files);
        Ok(entries)
    }

    fn is_ignored_fuzzy_picker_dir_name(name: &str) -> bool {
        IGNORED_FUZZY_PICKER_DIRS.contains(&name)
    }

    fn fuzzy_directory_sort_key(root: &std::path::Path, path: &std::path::Path) -> (bool, String) {
        let label = path
            .strip_prefix(root)
            .unwrap_or(path)
            .display()
            .to_string();
        (
            !label
                .split(std::path::MAIN_SEPARATOR)
                .next()
                .unwrap_or(&label)
                .starts_with('.'),
            label.to_lowercase(),
        )
    }

    fn build_fuzzy_file_picker_entries(
        dir: &std::path::Path,
    ) -> std::io::Result<PickerIndexResult> {
        let mut entries = Vec::new();
        let mut stack = vec![dir.to_path_buf()];
        let started_at = Instant::now();
        let mut dirs_visited = 0usize;
        let mut files_indexed = 0usize;
        let mut truncated = None;

        while let Some(current_dir) = stack.pop() {
            if started_at.elapsed() >= MAX_FUZZY_PICKER_INDEX_DURATION {
                truncated = Some(PickerIndexTruncation::Time);
                break;
            }
            if dirs_visited >= MAX_FUZZY_PICKER_DIRS_VISITED {
                truncated = Some(PickerIndexTruncation::Directory);
                break;
            }
            dirs_visited += 1;

            let mut dirs = Vec::new();
            let mut files = Vec::new();

            let read_dir = match fs::read_dir(&current_dir) {
                Ok(read_dir) => read_dir,
                Err(err) => {
                    if current_dir == dir {
                        return Err(err);
                    }
                    continue;
                }
            };

            for entry in read_dir {
                if started_at.elapsed() >= MAX_FUZZY_PICKER_INDEX_DURATION {
                    truncated = Some(PickerIndexTruncation::Time);
                    break;
                }
                let entry = match entry {
                    Ok(entry) => entry,
                    Err(_) => continue,
                };
                let path = entry.path();
                let file_type = match entry.file_type() {
                    Ok(file_type) => file_type,
                    Err(_) => continue,
                };

                if file_type.is_dir() {
                    let name = entry.file_name();
                    if Self::is_ignored_fuzzy_picker_dir_name(name.to_string_lossy().as_ref()) {
                        continue;
                    }
                    dirs.push(path);
                    continue;
                }

                if file_type.is_file() && Self::is_markdown_path(&path) {
                    if files_indexed >= MAX_FUZZY_PICKER_FILES_INDEXED {
                        truncated = Some(PickerIndexTruncation::File);
                        break;
                    }
                    let label = path
                        .strip_prefix(dir)
                        .unwrap_or(&path)
                        .display()
                        .to_string();
                    files.push(FilePickerEntry::new(label, path));
                    files_indexed += 1;
                }
            }

            files.sort_by(|left, right| {
                Self::fuzzy_entry_sort_key(left).cmp(&Self::fuzzy_entry_sort_key(right))
            });
            dirs.sort_by_key(|path| Self::fuzzy_directory_sort_key(dir, path));

            entries.extend(files);
            if truncated.is_some() {
                break;
            }
            dirs.reverse();
            stack.extend(dirs);
        }

        Ok(PickerIndexResult { entries, truncated })
    }

    fn fuzzy_entry_sort_key(entry: &FilePickerEntry) -> (bool, &str) {
        let first_component = entry
            .label
            .split(std::path::MAIN_SEPARATOR)
            .next()
            .unwrap_or(entry.label());
        (!first_component.starts_with('.'), entry.label_lower())
    }

    fn fuzzy_component_match(candidate: &str, query: &str) -> Option<(usize, Vec<usize>)> {
        if let Some(start) = candidate.find(query) {
            let start_chars = candidate[..start].chars().count();
            let query_len = query.chars().count();
            let len_diff = candidate.chars().count().saturating_sub(query_len);
            let prefix_bonus = usize::from(start_chars == 0).saturating_mul(80);
            let boundary_bonus =
                usize::from(Self::is_match_boundary(candidate, start_chars)).saturating_mul(40);
            let score = start_chars
                .saturating_mul(10)
                .saturating_add(len_diff)
                .saturating_sub(prefix_bonus)
                .saturating_sub(boundary_bonus);
            let positions = (start_chars..start_chars + query_len).collect::<Vec<_>>();
            return Some((score, positions));
        }

        let mut search_from = 0usize;
        let mut positions = Vec::with_capacity(query.len());

        for needle in query.chars() {
            let found = candidate[search_from..]
                .char_indices()
                .find(|(_, ch)| *ch == needle)
                .map(|(idx, _)| search_from + idx)?;
            let char_pos = candidate[..found].chars().count();
            positions.push(char_pos);
            search_from = found + needle.len_utf8();
        }

        let first = *positions.first()?;
        let last = *positions.last()?;
        let span = last.saturating_sub(first);
        let gaps = positions
            .windows(2)
            .map(|window| window[1].saturating_sub(window[0]).saturating_sub(1))
            .sum::<usize>();
        let len_diff = candidate
            .chars()
            .count()
            .saturating_sub(query.chars().count());
        let prefix_bonus = usize::from(first == 0).saturating_mul(80);
        let boundary_bonus =
            usize::from(Self::is_match_boundary(candidate, first)).saturating_mul(40);
        let score = 1_000usize
            .saturating_add(gaps.saturating_mul(120))
            .saturating_add(first.saturating_mul(10))
            .saturating_add(span)
            .saturating_add(len_diff)
            .saturating_sub(prefix_bonus)
            .saturating_sub(boundary_bonus);
        Some((score, positions))
    }

    fn is_match_boundary(candidate: &str, char_pos: usize) -> bool {
        if char_pos == 0 {
            return true;
        }

        candidate
            .chars()
            .nth(char_pos.saturating_sub(1))
            .is_some_and(|ch| matches!(ch, '-' | '_' | '.' | ' '))
    }

    fn fuzzy_match(entry: &FilePickerEntry, query: &str) -> Option<(usize, Vec<usize>)> {
        if query.is_empty() {
            return Some((0, Vec::new()));
        }

        let (score, positions) = Self::fuzzy_component_match(entry.file_name_lower(), query)?;
        Some((
            score,
            positions
                .into_iter()
                .map(|position| entry.file_name_offset() + position)
                .collect(),
        ))
    }

    fn refresh_file_picker_matches(&mut self) {
        if self.is_browser_file_picker() {
            self.file_picker.filtered = (0..self.file_picker.entries.len()).collect();
            self.file_picker.match_positions = vec![Vec::new(); self.file_picker.filtered.len()];
            self.file_picker.index = self
                .file_picker
                .index
                .min(self.file_picker.filtered.len().saturating_sub(1));
            return;
        }

        let query = self.file_picker.query.trim().to_lowercase();
        if query.is_empty() {
            self.file_picker.filtered = (0..self.file_picker.entries.len()).collect();
            self.file_picker.match_positions = vec![Vec::new(); self.file_picker.filtered.len()];
            self.file_picker.index = self
                .file_picker
                .index
                .min(self.file_picker.filtered.len().saturating_sub(1));
            return;
        }

        let mut filtered = self
            .file_picker
            .entries
            .iter()
            .enumerate()
            .filter_map(|(idx, entry)| {
                Self::fuzzy_match(entry, &query).map(|(score, positions)| {
                    (
                        idx,
                        score,
                        entry.path_depth(),
                        entry.file_name_lower(),
                        entry.label_lower(),
                        positions,
                    )
                })
            })
            .collect::<Vec<_>>();

        filtered.sort_by(
            |(left_idx, left_score, left_depth, left_name, left_label, _),
             (right_idx, right_score, right_depth, right_name, right_label, _)| {
                left_score
                    .cmp(right_score)
                    .then_with(|| left_depth.cmp(right_depth))
                    .then_with(|| left_name.cmp(right_name))
                    .then_with(|| left_label.cmp(right_label))
                    .then_with(|| left_idx.cmp(right_idx))
            },
        );

        self.file_picker.filtered = filtered.iter().map(|(idx, ..)| *idx).collect();
        self.file_picker.match_positions = filtered
            .into_iter()
            .map(|(_, _, _, _, _, positions)| positions)
            .collect();
        if self.file_picker.filtered.is_empty()
            || self.file_picker.index >= self.file_picker.filtered.len()
        {
            self.file_picker.index = 0;
        }
    }

    fn selected_file_picker_entry(&self) -> Option<&FilePickerEntry> {
        let idx = *self.file_picker.filtered.get(self.file_picker.index)?;
        self.file_picker.entries.get(idx)
    }

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
        let plain_lines = build_plain_lines(&lines);
        let mut app = Self {
            lines,
            plain_lines,
            folded_plain_lines: None,
            scroll: 0,
            toc,
            toc_visible: false,
            search: SearchState {
                mode: false,
                draft: String::new(),
                query: String::new(),
                matches: vec![],
                idx: 0,
            },
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
            help_open: false,
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
            render_width: 80,
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

    pub(crate) fn debug_input_enabled(&self) -> bool {
        self.debug_input
    }

    pub(crate) fn is_toc_visible(&self) -> bool {
        self.toc_visible
    }

    pub(crate) fn has_toc(&self) -> bool {
        !self.toc.is_empty()
    }

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
        self.plain_lines = build_plain_lines(&lines);
        self.folded_plain_lines = None;
        self.lines = lines;
        self.toc = toc;
        self.highlighted_line_cache = None;
        self.toc_header_line = toc_header_line();
        self.refresh_static_caches();
    }

    pub(crate) fn active_highlight_line(&self) -> Option<usize> {
        if self.search.matches.is_empty() {
            None
        } else {
            Some(self.search.matches[self.search.idx])
        }
    }

    pub(crate) fn is_search_mode(&self) -> bool {
        self.search.mode
    }

    pub(crate) fn search_draft(&self) -> &str {
        &self.search.draft
    }

    pub(crate) fn search_query(&self) -> &str {
        &self.search.query
    }

    #[cfg(test)]
    pub(crate) fn set_search_query(&mut self, query: impl Into<String>) {
        self.search.query = query.into();
    }

    pub(crate) fn search_match_count(&self) -> usize {
        self.search.matches.len()
    }

    pub(crate) fn search_index(&self) -> usize {
        self.search.idx
    }

    #[cfg(test)]
    pub(crate) fn search_matches(&self) -> &[usize] {
        &self.search.matches
    }

    #[cfg(test)]
    pub(crate) fn line(&self, idx: usize) -> Option<&Line<'static>> {
        self.lines.get(idx)
    }

    #[cfg(test)]
    pub(crate) fn set_search_draft(&mut self, draft: impl Into<String>) {
        self.search.draft = draft.into();
    }

    pub(crate) fn pop_search_draft(&mut self) {
        self.search.draft.pop();
    }

    pub(crate) fn push_search_draft(&mut self, ch: char) {
        self.search.draft.push(ch);
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
            search_mode: self.search.mode,
            search_draft_hash: hash_str(&self.search.draft),
            search_query_hash: hash_str(&self.search.query),
            search_draft_len: self.search.draft.len(),
            search_query_len: self.search.query.len(),
            search_match_count: self.search.matches.len(),
            search_idx: self.search.idx,
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
        self.theme_picker.preview_cache.fill(None);
    }

    fn store_theme_preview(
        &mut self,
        preset: ThemePreset,
        lines: &[Line<'static>],
        toc: &[TocEntry],
    ) {
        let idx = theme_preset_index(preset);
        if let Some(slot) = self.theme_picker.preview_cache.get_mut(idx) {
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
        self.theme_picker.open = true;
        let current = current_theme_preset();
        self.theme_picker.index = theme_preset_index(current);
        self.theme_picker.original = Some(current);
        self.store_current_theme_preview();
    }

    pub(crate) fn close_theme_picker(&mut self) {
        self.theme_picker.open = false;
        self.theme_picker.original = None;
    }

    pub(crate) fn is_theme_picker_open(&self) -> bool {
        self.theme_picker.open
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

    pub(crate) fn open_file_picker(&mut self, dir: PathBuf) -> bool {
        self.open_file_picker_with_mode(dir, FilePickerMode::Browser)
    }

    #[cfg(test)]
    pub(crate) fn open_fuzzy_file_picker(&mut self, dir: PathBuf) -> bool {
        self.open_file_picker_with_mode(dir, FilePickerMode::Fuzzy)
    }

    pub(crate) fn queue_file_picker(&mut self, dir: PathBuf) {
        self.pending_picker = PendingPicker::Browser(dir);
    }

    pub(crate) fn queue_fuzzy_file_picker(&mut self, dir: PathBuf) {
        self.pending_picker = PendingPicker::Fuzzy(dir);
    }

    pub(crate) fn has_pending_picker(&self) -> bool {
        !matches!(self.pending_picker, PendingPicker::None)
    }

    pub(crate) fn start_pending_picker_loading(&mut self) -> bool {
        if !self.has_pending_picker() || !matches!(self.picker_load_state, PickerLoadState::Idle) {
            return false;
        }

        let pending = std::mem::replace(&mut self.pending_picker, PendingPicker::None);
        let (mode, dir) = match pending {
            PendingPicker::Browser(dir) => (FilePickerMode::Browser, dir),
            PendingPicker::Fuzzy(dir) => (FilePickerMode::Fuzzy, dir),
            PendingPicker::None => return false,
        };

        let worker_dir = dir.clone();
        let (tx, rx) = mpsc::channel();
        crate::runtime::debug_log(
            self.debug_input,
            &format!("picker_loading spawn mode={mode:?} dir={}", dir.display()),
        );
        thread::spawn(move || {
            let result = match mode {
                FilePickerMode::Browser => {
                    Self::build_file_picker_entries(&worker_dir).map(|entries| PickerIndexResult {
                        entries,
                        truncated: None,
                    })
                }
                FilePickerMode::Fuzzy => Self::build_fuzzy_file_picker_entries(&worker_dir),
            };
            let _ = tx.send(result);
        });

        self.picker_load_state = PickerLoadState::Loading {
            mode,
            dir,
            started_at: Instant::now(),
            receiver: rx,
            pending_result: None,
        };
        true
    }

    pub(crate) fn is_picker_loading(&self) -> bool {
        matches!(self.picker_load_state, PickerLoadState::Loading { .. })
    }

    pub(crate) fn is_picker_load_failed(&self) -> bool {
        matches!(self.picker_load_state, PickerLoadState::Failed { .. })
    }

    pub(crate) fn pending_picker_mode(&self) -> Option<FilePickerMode> {
        match &self.picker_load_state {
            PickerLoadState::Loading { mode, .. } | PickerLoadState::Failed { mode, .. } => {
                Some(*mode)
            }
            PickerLoadState::Idle => match self.pending_picker {
                PendingPicker::Browser(..) => Some(FilePickerMode::Browser),
                PendingPicker::Fuzzy(..) => Some(FilePickerMode::Fuzzy),
                PendingPicker::None => None,
            },
        }
    }

    pub(crate) fn pending_picker_dir(&self) -> Option<&std::path::Path> {
        match &self.picker_load_state {
            PickerLoadState::Loading { dir, .. } | PickerLoadState::Failed { dir, .. } => {
                Some(dir.as_path())
            }
            PickerLoadState::Idle => match &self.pending_picker {
                PendingPicker::Browser(dir) | PendingPicker::Fuzzy(dir) => Some(dir.as_path()),
                PendingPicker::None => None,
            },
        }
    }

    pub(crate) fn picker_load_error(&self) -> Option<&str> {
        match &self.picker_load_state {
            PickerLoadState::Failed { message, .. } => Some(message.as_str()),
            PickerLoadState::Idle | PickerLoadState::Loading { .. } => None,
        }
    }

    fn install_loaded_file_picker(
        &mut self,
        dir: PathBuf,
        mode: FilePickerMode,
        result: PickerIndexResult,
    ) -> bool {
        self.file_picker.open = true;
        self.file_picker.mode = mode;
        self.file_picker.dir = dir;
        self.file_picker.entries = result.entries;
        self.file_picker.query.clear();
        self.file_picker.index = 0;
        self.file_picker.truncation = if mode == FilePickerMode::Fuzzy {
            result.truncated
        } else {
            None
        };
        self.refresh_file_picker_matches();
        true
    }

    pub(crate) fn poll_picker_loading(&mut self) -> bool {
        let state = std::mem::replace(&mut self.picker_load_state, PickerLoadState::Idle);
        match state {
            PickerLoadState::Loading {
                mode,
                dir,
                started_at,
                receiver,
                mut pending_result,
            } => {
                if pending_result.is_none() {
                    pending_result = match receiver.try_recv() {
                        Ok(result) => {
                            crate::runtime::debug_log(
                                self.debug_input,
                                &format!(
                                    "picker_loading worker_finished mode={mode:?} dir={}",
                                    dir.display()
                                ),
                            );
                            Some(result)
                        }
                        Err(TryRecvError::Empty) => None,
                        Err(TryRecvError::Disconnected) => Some(Err(std::io::Error::other(
                            "Picker loading worker disconnected",
                        ))),
                    };
                }

                if started_at.elapsed() < Self::min_picker_loading_duration() {
                    self.picker_load_state = PickerLoadState::Loading {
                        mode,
                        dir,
                        started_at,
                        receiver,
                        pending_result,
                    };
                    return false;
                }

                match pending_result {
                    Some(Ok(result)) => {
                        crate::runtime::debug_log(
                            self.debug_input,
                            &format!(
                                "picker_loading install mode={mode:?} dir={} entries={}",
                                dir.display(),
                                result.entries.len()
                            ),
                        );
                        self.install_loaded_file_picker(dir, mode, result)
                    }
                    Some(Err(err)) => {
                        crate::runtime::debug_log(
                            self.debug_input,
                            &format!(
                                "picker_loading failed mode={mode:?} dir={} error={}",
                                dir.display(),
                                err
                            ),
                        );
                        self.picker_load_state = PickerLoadState::Failed {
                            mode,
                            dir,
                            message: err.to_string(),
                        };
                        true
                    }
                    None => {
                        self.picker_load_state = PickerLoadState::Loading {
                            mode,
                            dir,
                            started_at,
                            receiver,
                            pending_result: None,
                        };
                        false
                    }
                }
            }
            PickerLoadState::Failed { .. } => {
                self.picker_load_state = state;
                false
            }
            PickerLoadState::Idle => {
                self.picker_load_state = PickerLoadState::Idle;
                false
            }
        }
    }

    #[cfg(test)]
    pub(crate) fn age_picker_loading_by(&mut self, duration: Duration) {
        if let PickerLoadState::Loading {
            mode,
            dir,
            started_at,
            receiver,
            pending_result,
        } = std::mem::replace(&mut self.picker_load_state, PickerLoadState::Idle)
        {
            let adjusted = started_at.checked_sub(duration).unwrap_or(started_at);
            self.picker_load_state = PickerLoadState::Loading {
                mode,
                dir,
                started_at: adjusted,
                receiver,
                pending_result,
            };
        }
    }

    fn open_file_picker_with_mode(&mut self, dir: PathBuf, mode: FilePickerMode) -> bool {
        let result = match mode {
            FilePickerMode::Browser => {
                Self::build_file_picker_entries(&dir).map(|entries| PickerIndexResult {
                    entries,
                    truncated: None,
                })
            }
            FilePickerMode::Fuzzy => Self::build_fuzzy_file_picker_entries(&dir),
        };

        match result {
            Ok(result) => self.install_loaded_file_picker(dir, mode, result),
            Err(_) => false,
        }
    }

    pub(crate) fn is_fuzzy_file_picker(&self) -> bool {
        self.file_picker.mode == FilePickerMode::Fuzzy
    }

    pub(crate) fn is_browser_file_picker(&self) -> bool {
        self.file_picker.mode == FilePickerMode::Browser
    }

    pub(crate) fn is_file_picker_open(&self) -> bool {
        self.file_picker.open
    }

    pub(crate) fn file_picker_dir(&self) -> &std::path::Path {
        &self.file_picker.dir
    }

    pub(crate) fn file_picker_entries(&self) -> &[FilePickerEntry] {
        &self.file_picker.entries
    }

    pub(crate) fn file_picker_filtered_indices(&self) -> &[usize] {
        &self.file_picker.filtered
    }

    pub(crate) fn file_picker_match_positions(&self, filtered_idx: usize) -> &[usize] {
        self.file_picker
            .match_positions
            .get(filtered_idx)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub(crate) fn file_picker_index(&self) -> usize {
        self.file_picker.index
    }

    pub(crate) fn file_picker_query(&self) -> &str {
        &self.file_picker.query
    }

    pub(crate) fn file_picker_truncation(&self) -> Option<PickerIndexTruncation> {
        self.file_picker.truncation
    }

    pub(crate) fn move_file_picker_up(&mut self) {
        let total = self.file_picker.filtered.len();
        if total == 0 {
            return;
        }
        if self.file_picker.index == 0 {
            self.file_picker.index = total - 1;
        } else {
            self.file_picker.index -= 1;
        }
    }

    pub(crate) fn move_file_picker_down(&mut self) {
        let total = self.file_picker.filtered.len();
        if total == 0 {
            return;
        }
        self.file_picker.index = (self.file_picker.index + 1) % total;
    }

    pub(crate) fn push_file_picker_query(&mut self, ch: char) {
        if self.is_browser_file_picker() {
            return;
        }
        self.file_picker.query.push(ch);
        self.refresh_file_picker_matches();
    }

    pub(crate) fn pop_file_picker_query(&mut self) {
        if self.is_browser_file_picker() {
            return;
        }
        self.file_picker.query.pop();
        self.refresh_file_picker_matches();
    }

    pub(crate) fn clear_file_picker_query(&mut self) {
        if self.is_browser_file_picker() {
            return;
        }
        self.file_picker.query.clear();
        self.refresh_file_picker_matches();
    }

    pub(crate) fn open_file_picker_parent(&mut self) -> bool {
        if self.is_fuzzy_file_picker() {
            return false;
        }
        let Some(parent) = self.file_picker.dir.parent() else {
            return false;
        };
        self.open_file_picker(parent.to_path_buf())
    }

    pub(crate) fn theme_picker_index(&self) -> usize {
        self.theme_picker.index
    }

    #[cfg(test)]
    pub(crate) fn theme_picker_original(&self) -> Option<ThemePreset> {
        self.theme_picker.original
    }

    pub(crate) fn clear_reload_flash(&mut self) {
        self.reload_flash = None;
    }

    pub(crate) fn reload_flash_started(&self) -> Option<Instant> {
        self.reload_flash
    }

    pub(crate) fn set_last_file_state(&mut self, state: FileState) {
        self.last_file_state = Some(state);
    }

    pub(crate) fn theme_picker_reference_preset(&self) -> ThemePreset {
        self.theme_picker.original.unwrap_or(current_theme_preset())
    }

    pub(crate) fn move_theme_picker_up(&mut self) {
        let total = THEME_PRESETS.len();
        if total == 0 {
            return;
        }
        if self.theme_picker.index == 0 {
            self.theme_picker.index = total - 1;
        } else {
            self.theme_picker.index -= 1;
        }
    }

    pub(crate) fn move_theme_picker_down(&mut self) {
        let total = THEME_PRESETS.len();
        if total == 0 {
            return;
        }
        self.theme_picker.index = (self.theme_picker.index + 1) % total;
    }

    pub(crate) fn set_theme_picker_index(&mut self, idx: usize) -> bool {
        if idx < THEME_PRESETS.len() {
            self.theme_picker.index = idx;
            true
        } else {
            false
        }
    }

    pub(crate) fn selected_theme_preset(&self) -> Option<ThemePreset> {
        THEME_PRESETS.get(self.theme_picker.index).copied()
    }

    #[cfg(test)]
    pub(crate) fn has_cached_theme_preview(&self, preset: ThemePreset) -> bool {
        self.theme_picker
            .preview_cache
            .get(theme_preset_index(preset))
            .and_then(|entry| entry.as_ref())
            .is_some()
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
            .theme_picker
            .preview_cache
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
        if let Some(original) = self.theme_picker.original {
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

    pub(crate) fn scroll_top(&mut self) {
        self.scroll = 0;
    }

    pub(crate) fn scroll_bottom(&mut self) {
        self.scroll = self.total().saturating_sub(1);
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

    pub(crate) fn run_search(&mut self) {
        let q = self.search.query.to_lowercase();
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
        self.search.matches = search_matches;
        self.search.idx = 0;
        if let Some(&f) = self.search.matches.first() {
            self.scroll = f;
        }
    }

    pub(crate) fn begin_search(&mut self) {
        self.search.mode = true;
        self.search.draft = self.search.query.clone();
        crate::runtime::debug_log(
            self.debug_input,
            &format!(
                "begin_search query={:?} draft={:?} matches={} idx={}",
                self.search.query,
                self.search.draft,
                self.search.matches.len(),
                self.search.idx
            ),
        );
    }

    pub(crate) fn reset_search_state(&mut self) {
        self.search.draft.clear();
        self.search.query.clear();
        self.search.matches.clear();
        self.search.idx = 0;
    }

    pub(crate) fn cancel_search(&mut self) {
        self.search.mode = false;
        self.reset_search_state();
        crate::runtime::debug_log(self.debug_input, "cancel_search cleared query and matches");
    }

    pub(crate) fn confirm_search(&mut self) {
        self.search.mode = false;
        let draft = std::mem::take(&mut self.search.draft);
        self.search.query = draft;
        if self.search.query.is_empty() {
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
                self.search.query,
                self.search.matches.len(),
                self.search.idx,
                self.scroll
            ),
        );
    }

    pub(crate) fn clear_active_search(&mut self) {
        self.search.mode = false;
        self.reset_search_state();
        crate::runtime::debug_log(
            self.debug_input,
            "clear_active_search cleared query and matches",
        );
    }

    pub(crate) fn has_active_search(&self) -> bool {
        !self.search.query.is_empty() || !self.search.matches.is_empty()
    }

    pub(crate) fn next_match(&mut self) {
        if self.search.matches.is_empty() {
            return;
        }
        self.search.idx = (self.search.idx + 1) % self.search.matches.len();
        self.scroll = self.search.matches[self.search.idx];
    }

    pub(crate) fn prev_match(&mut self) {
        if self.search.matches.is_empty() {
            return;
        }
        if self.search.idx == 0 {
            self.search.idx = self.search.matches.len() - 1;
        } else {
            self.search.idx -= 1;
        }
        self.scroll = self.search.matches[self.search.idx];
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

    pub(crate) fn activate_file_picker_selection(
        &mut self,
        ss: &SyntaxSet,
        themes: &ThemeSet,
    ) -> bool {
        let Some(entry) = self.selected_file_picker_entry().cloned() else {
            return false;
        };
        if self.is_browser_file_picker() && entry.is_dir_like() {
            self.open_file_picker(entry.path)
        } else {
            self.load_path(entry.path, ss, themes)
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
