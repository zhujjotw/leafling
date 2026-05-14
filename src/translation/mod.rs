mod provider;
mod render;
mod segment;

pub(crate) use provider::{build_provider, TranslationProvider};
pub(crate) use render::build_translated_source;
pub(crate) use segment::{extract_segments, Segment, SegmentKind};

use crate::markdown::hash_str;
use ratatui::text::Line;
use std::collections::HashMap;
use std::sync::mpsc;

#[derive(Clone, Debug)]
pub(crate) enum TranslationStatus {
    Idle,
    Loading { completed: usize, total: usize },
    Done,
    Error(String),
}

/// Message sent from the background translation thread.
pub(crate) enum TranslationMsg {
    Progress {
        completed: usize,
        total: usize,
        hash: u64,
        translation: String,
    },
    Done,
    Error(String),
}

/// State for the translation feature.
pub(crate) struct TranslationState {
    pub(crate) status: TranslationStatus,
    pub(crate) segments: Vec<Segment>,
    pub(crate) cache: HashMap<u64, String>,
    pub(crate) translated_lines: Option<Vec<Line<'static>>>,
    pub(crate) receiver: Option<mpsc::Receiver<TranslationMsg>>,
    pub(crate) cancel_flag: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl TranslationState {
    pub(crate) fn new() -> Self {
        Self {
            status: TranslationStatus::Idle,
            segments: Vec::new(),
            cache: HashMap::new(),
            translated_lines: None,
            receiver: None,
            cancel_flag: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    pub(crate) fn invalidate(&mut self) {
        self.segments.clear();
        self.translated_lines = None;
    }
}
