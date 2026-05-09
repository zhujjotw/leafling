use super::App;
use crate::editor::EditorEntry;

pub(crate) struct EditorPickerState {
    pub(super) open: bool,
    pub(super) editors: Vec<EditorEntry>,
    pub(super) index: usize,
}

impl App {
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
}
