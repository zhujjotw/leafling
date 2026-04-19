use super::App;
use std::{
    fs,
    path::PathBuf,
    sync::mpsc::{self, Receiver, TryRecvError},
    thread,
    time::{Duration, Instant},
};
use syntect::{highlighting::ThemeSet, parsing::SyntaxSet};

const MAX_FUZZY_PICKER_DIRS_VISITED: usize = 5_000;
const MAX_FUZZY_PICKER_FILES_INDEXED: usize = 10_000;
const MAX_FUZZY_PICKER_INDEX_DURATION: Duration = Duration::from_secs(5);
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

    pub(super) fn label_lower(&self) -> &str {
        &self.label_lower
    }

    pub(super) fn file_name_lower(&self) -> &str {
        &self.file_name_lower
    }

    pub(super) fn file_name_offset(&self) -> usize {
        self.file_name_offset
    }

    pub(super) fn path_depth(&self) -> usize {
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
    pub(super) open: bool,
    pub(super) mode: FilePickerMode,
    pub(super) dir: PathBuf,
    pub(super) entries: Vec<FilePickerEntry>,
    pub(super) filtered: Vec<usize>,
    pub(super) match_positions: Vec<Vec<usize>>,
    pub(super) index: usize,
    pub(super) query: String,
    pub(super) truncation: Option<PickerIndexTruncation>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FilePickerMode {
    Browser,
    Fuzzy,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum PendingPicker {
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

pub(crate) struct PickerIndexResult {
    pub(crate) entries: Vec<FilePickerEntry>,
    pub(crate) truncated: Option<PickerIndexTruncation>,
}

pub(crate) enum PickerLoadState {
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

impl App {
    pub(super) fn min_picker_loading_duration() -> Duration {
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
                    if super::fuzzy::is_ignored_fuzzy_picker_dir_name(
                        name.to_string_lossy().as_ref(),
                    ) {
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
                super::fuzzy::fuzzy_entry_sort_key(left)
                    .cmp(&super::fuzzy::fuzzy_entry_sort_key(right))
            });
            dirs.sort_by_key(|path| super::fuzzy::fuzzy_directory_sort_key(dir, path));

            entries.extend(files);
            if truncated.is_some() {
                break;
            }
            dirs.reverse();
            stack.extend(dirs);
        }

        Ok(PickerIndexResult { entries, truncated })
    }

    pub(super) fn refresh_file_picker_matches(&mut self) {
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
                super::fuzzy::fuzzy_match(entry, &query).map(|(score, positions)| {
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

    pub(crate) fn close_file_picker(&mut self) {
        self.file_picker.open = false;
        self.file_picker.query.clear();
        self.file_picker.entries.clear();
        self.file_picker.filtered.clear();
        self.file_picker.match_positions.clear();
        self.file_picker.index = 0;
        self.file_picker.truncation = None;
    }

    pub(crate) fn cancel_picker_loading(&mut self) {
        self.picker_load_state = PickerLoadState::Idle;
        self.pending_picker = PendingPicker::None;
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
}
