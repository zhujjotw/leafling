use super::App;
use crate::config::TranslationConfig;
use crate::markdown::hash_str;
use crate::translation::{
    build_bilingual_lines, build_provider, extract_segments, TranslationMsg, TranslationStatus,
};
use std::sync::mpsc;
use std::thread;

impl App {
    /// Toggle translation on/off.
    pub(crate) fn toggle_translation(&mut self) {
        if self.translation.enabled {
            // Turn off
            self.translation.enabled = false;
            self.translation.status = TranslationStatus::Idle;
            self.translation.bilingual_lines = None;
            self.translation.cancel_flag.store(
                true,
                std::sync::atomic::Ordering::Relaxed,
            );
            self.translation.receiver = None;
            self.set_translation_flash(crate::app::TranslationFlash::Deactivated);
            self.refresh_static_caches();
        } else {
            // Check if translation is configured
            if !self.translation_config.is_configured() {
                self.set_translation_flash(crate::app::TranslationFlash::NotConfigured);
                return;
            }
            // Turn on
            self.translation.enabled = true;
            self.translation.cancel_flag.store(
                false,
                std::sync::atomic::Ordering::Relaxed,
            );
            self.set_translation_flash(crate::app::TranslationFlash::Activated);
            self.start_translation();
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
            // All segments are cached, build bilingual lines directly
            self.translation.status = TranslationStatus::Done;
            self.rebuild_bilingual_lines();
            return;
        }

        let (tx, rx) = mpsc::channel();
        self.translation.receiver = Some(rx);

        // Clone all data needed by the thread before spawning
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

                // Check if already in cache (from previous run)
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
                        // Continue with remaining segments
                    }
                }
            }

            let _ = tx.send(TranslationMsg::Done);
        });
    }

    /// Poll the translation background thread for results.
    /// Returns true if state changed (needs redraw).
    pub(crate) fn poll_translation(&mut self) -> bool {
        // Take the receiver temporarily to avoid borrow issues
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
                            // Don't put receiver back
                            self.translation.status = TranslationStatus::Done;
                            self.rebuild_bilingual_lines();
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
                    // Put receiver back
                    self.translation.receiver = rx_taken;
                    break;
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    // Channel closed, don't put back
                    break;
                }
            }
        }
        changed
    }

    /// Rebuild bilingual display lines from current cache.
    pub(crate) fn rebuild_bilingual_lines(&mut self) {
        // We need syntect references but can't hold them here.
        // Instead, we'll store a flag and rebuild when we have access.
        self.translation.bilingual_lines = None;
    }

    /// Rebuild bilingual lines with syntect access (called from runtime).
    pub(crate) fn rebuild_bilingual_lines_with_syntect(
        &mut self,
        ss: &syntect::parsing::SyntaxSet,
        themes: &syntect::highlighting::ThemeSet,
    ) {
        if !self.translation.enabled {
            return;
        }

        let syntect_theme = crate::theme::current_syntect_theme(themes).clone();
        let md_theme = crate::theme::app_theme().markdown;

        let bilingual = build_bilingual_lines(
            &self.source,
            &self.translation.segments,
            &self.translation.cache,
            &md_theme,
            ss,
            &syntect_theme,
            self.render_width,
            self.file_mode,
        );

        self.translation.bilingual_lines = Some(bilingual);
        self.refresh_static_caches();
    }
}
