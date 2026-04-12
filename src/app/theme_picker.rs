use crate::{
    markdown::{parse_markdown_with_width, toc::TocEntry},
    theme::{
        current_syntect_theme, current_theme_preset, set_theme_preset, theme_preset_index,
        ThemePreset, THEME_PRESETS,
    },
};
use ratatui::text::Line;
use syntect::{highlighting::ThemeSet, parsing::SyntaxSet};

use super::App;

#[derive(Clone)]
pub(crate) struct ThemePreviewCacheEntry {
    pub(super) lines: Vec<Line<'static>>,
    pub(super) toc: Vec<TocEntry>,
}

pub(crate) struct ThemePickerState {
    pub(super) open: bool,
    pub(super) index: usize,
    pub(super) original: Option<ThemePreset>,
    pub(super) preview_cache: Vec<Option<ThemePreviewCacheEntry>>,
}

impl App {
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

    pub(crate) fn theme_picker_index(&self) -> usize {
        self.theme_picker.index
    }

    #[cfg(test)]
    pub(crate) fn theme_picker_original(&self) -> Option<ThemePreset> {
        self.theme_picker.original
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

    pub(crate) fn store_theme_preview(
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

    pub(crate) fn store_current_theme_preview(&mut self) {
        let preset = current_theme_preset();
        let idx = theme_preset_index(preset);
        if let Some(slot) = self.theme_picker.preview_cache.get_mut(idx) {
            *slot = Some(ThemePreviewCacheEntry {
                lines: self.lines.clone(),
                toc: self.toc.clone(),
            });
        }
    }

    pub(crate) fn invalidate_theme_preview_cache(&mut self) {
        self.theme_picker.preview_cache.fill(None);
    }

    #[cfg(test)]
    pub(crate) fn has_cached_theme_preview(&self, preset: ThemePreset) -> bool {
        self.theme_picker
            .preview_cache
            .get(theme_preset_index(preset))
            .and_then(|entry| entry.as_ref())
            .is_some()
    }
}
