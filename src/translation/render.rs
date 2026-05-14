use crate::markdown::hash_str;
use std::collections::HashMap;

use super::segment::{extract_segments, Segment};

/// Build a translated markdown source by replacing each segment's English
/// text with its Chinese translation.
pub(crate) fn build_translated_source(
    source: &str,
    segments: &[Segment],
    translations: &HashMap<u64, String>,
) -> String {
    if translations.is_empty() {
        return source.to_string();
    }

    // For each segment with a translation, replace it in the source.
    let mut result = String::with_capacity(source.len());
    let mut last_end = 0usize;

    for segment in segments {
        let hash = hash_str(&segment.source_text);
        let translation = match translations.get(&hash) {
            Some(t) => t,
            None => continue,
        };

        // Find this segment's text in the remaining source
        if let Some(pos) = find_text_position(&source[last_end..], &segment.source_text) {
            let abs_pos = last_end + pos;
            let abs_end = find_segment_end(source, abs_pos, &segment.source_text);

            // Append source up to this segment
            result.push_str(&source[last_end..abs_pos]);

            // Append the translated text instead of the original
            result.push_str(translation);
            result.push('\n');

            last_end = abs_end;
        }
    }

    // Append remaining source
    result.push_str(&source[last_end..]);
    result
}

/// Find the position of segment text in the source.
fn find_text_position(haystack: &str, needle: &str) -> Option<usize> {
    if let Some(pos) = haystack.find(needle) {
        return Some(pos);
    }

    // Fuzzy: match by first word then verify
    let needle_start = needle.split_whitespace().next()?;
    haystack.find(needle_start).and_then(|pos| {
        let needle_words: Vec<&str> = needle.split_whitespace().collect();
        let mut hay_words = haystack[pos..].split_whitespace();
        for expected in &needle_words {
            match hay_words.next() {
                Some(actual) if actual == *expected => continue,
                _ => return None,
            }
        }
        Some(pos)
    })
}

/// Find where the segment ends in the source after the given position.
fn find_segment_end(source: &str, start: usize, needle: &str) -> usize {
    let search_area = &source[start..];

    if let Some(pos) = search_area.find(needle) {
        return start + pos + needle.len();
    }

    let needle_words: Vec<&str> = needle.split_whitespace().collect();
    let mut search_from = 0usize;

    for word in &needle_words {
        if let Some(pos) = search_area[search_from..].find(word) {
            let abs_word_pos = search_from + pos;
            search_from = abs_word_pos + word.len();
        } else {
            break;
        }
    }

    // Extend to end of line
    let end_pos = start + search_from;
    source[end_pos..]
        .find('\n')
        .map(|n| end_pos + n)
        .unwrap_or(source.len())
}
