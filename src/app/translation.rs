use super::App;
use crate::markdown::hash_str;
use crate::translation::{
    build_provider, build_translated_source, extract_segments, TranslationMsg, TranslationStatus,
};
use std::sync::mpsc;
use std::thread;

impl App {
    /// Toggle between original and translated view.
    pub(crate) fn toggle_translation(&mut self) {
        if self.is_translated_view() {
            // Switch back to original
            self.translation.translated_lines = None;
            self.set_translation_flash(crate::app::TranslationFlash::Deactivated);
            self.refresh_static_caches();
        } else {
            // Switch to translated (if available)
            if !self.translation_config.is_configured() {
                self.set_translation_flash(crate::app::TranslationFlash::NotConfigured);
                return;
            }
            if matches!(self.translation.status, TranslationStatus::Done) {
                // Already translated, just show it
                self.rebuild_translated_lines();
                self.set_translation_flash(crate::app::TranslationFlash::Activated);
                self.refresh_static_caches();
            } else {
                // Not done yet, start or show loading
                self.set_translation_flash(crate::app::TranslationFlash::Activated);
            }
        }
    }

    /// Start the background translation thread.
    pub(crate) fn start_translation(&mut self) {
        let segments = extract_segments(&self.source);
        if segments.is_empty() {
            self.translation.status = TranslationStatus::Done;
            return;
        }

        let total = segments.len();
        self.translation.segments = segments;
        self.translation.status = TranslationStatus::Loading {
            completed: 0,
            total,
        };

        // Filter to only segments that need translation (not in cache)
        let uncached: Vec<_> = self
            .translation
            .segments
            .iter()
            .filter(|s| {
                let h = hash_str(&s.source_text);
                !self.translation.cache.contains_key(&h)
            })
            .cloned()
            .collect();

        if uncached.is_empty() {
            // All segments are cached, build translated lines directly
            self.translation.status = TranslationStatus::Done;
            self.rebuild_translated_lines();
            return;
        }

        let (tx, rx) = mpsc::channel();
        self.translation.receiver = Some(rx);

        let provider_type = self.translation_config.provider.clone().unwrap_or_else(|| "deepl".to_string());
        let endpoint = self.translation_config.api_endpoint.clone().unwrap_or_default();
        let api_key = self.translation_config.api_key.clone().unwrap_or_default();
        let source_lang = self.translation_config.source_lang().to_string();
        let target_lang = self.translation_config.target_lang().to_string();
        let cancel_flag = self.translation.cancel_flag.clone();
        let existing_cache = self.translation.cache.clone();

        thread::spawn(move || {
            let provider = build_provider(&provider_type, &endpoint, &api_key);
            let mut cache = existing_cache;

            for (idx, segment) in uncached.iter().enumerate() {
                if cancel_flag.load(std::sync::atomic::Ordering::Relaxed) {
                    let _ = tx.send(TranslationMsg::Done);
                    return;
                }

                let h = hash_str(&segment.source_text);

                if cache.contains_key(&h) {
                    let _ = tx.send(TranslationMsg::Progress {
                        completed: idx + 1,
                        total: uncached.len(),
                        hash: h,
                        translation: cache[&h].clone(),
                    });
                    continue;
                }

                match provider.translate(&segment.source_text, &source_lang, &target_lang) {
                    Ok(translation) => {
                        cache.insert(h, translation.clone());
                        let _ = tx.send(TranslationMsg::Progress {
                            completed: idx + 1,
                            total: uncached.len(),
                            hash: h,
                            translation,
                        });
                    }
                    Err(e) => {
                        let _ = tx.send(TranslationMsg::Error(format!("{e}")));
                    }
                }
            }

            let _ = tx.send(TranslationMsg::Done);
        });
    }

    /// Poll the translation background thread for results.
    /// Returns true if state changed (needs redraw).
    pub(crate) fn poll_translation(&mut self) -> bool {
        let mut rx_taken = self.translation.receiver.take();
        let rx = match rx_taken.as_mut() {
            Some(rx) => rx,
            None => return false,
        };

        let mut changed = false;
        loop {
            match rx.try_recv() {
                Ok(msg) => {
                    match msg {
                        TranslationMsg::Progress {
                            completed,
                            total,
                            hash,
                            translation,
                        } => {
                            self.translation.cache.insert(hash, translation);
                            self.translation.status = TranslationStatus::Loading { completed, total };
                            changed = true;
                        }
                        TranslationMsg::Done => {
                            self.translation.status = TranslationStatus::Done;
                            self.rebuild_translated_lines();
                            changed = true;
                            return changed;
                        }
                        TranslationMsg::Error(msg) => {
                            let _ = msg;
                            changed = true;
                        }
                    }
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {
                    self.translation.receiver = rx_taken;
                    break;
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    break;
                }
            }
        }
        changed
    }

    /// Build translated display lines from current cache.
    fn rebuild_translated_lines(&mut self) {
        let translated_source = build_translated_source(
            &self.source,
            &self.translation.segments,
            &self.translation.cache,
        );

        let ss = syntect::parsing::SyntaxSet::load_defaults_newlines();
        let ts = syntect::highlighting::ThemeSet::load_defaults();
        let syntect_theme = crate::theme::current_syntect_theme(&ts).clone();
        let md_theme = crate::theme::app_theme().markdown;

        let (lines, _, _) = crate::markdown::parse_markdown_with_width(
            &translated_source,
            &ss,
            &syntect_theme,
            self.render_width,
            &md_theme,
            self.file_mode,
        );

        self.translation.translated_lines = Some(lines);
    }
}
