use crate::markdown::parse_markdown_with_width;
use crate::theme::MarkdownTheme;
use ratatui::text::Line;
use std::collections::HashMap;

use super::segment::Segment;

/// Build bilingual lines by constructing a new markdown source with translations
/// interleaved after each translatable segment, then rendering it.
pub(crate) fn build_bilingual_lines(
    source: &str,
    segments: &[Segment],
    translations: &HashMap<u64, String>,
    md_theme: &MarkdownTheme,
    ss: &syntect::parsing::SyntaxSet,
    syntect_theme: &syntect::highlighting::Theme,
    render_width: usize,
    file_mode: bool,
) -> Vec<Line<'static>> {
    let bilingual_source =
        build_bilingual_source(source, segments, translations);

    let (bilingual_lines, _, _) = parse_markdown_with_width(
        &bilingual_source,
        ss,
        syntect_theme,
        render_width,
        md_theme,
        file_mode,
    );

    bilingual_lines
}

/// Build a markdown source string with translations interleaved.
///
/// Strategy: iterate through segments in order, and for each one that has
/// a translation, insert the translation after the segment's text in the source.
fn build_bilingual_source(
    source: &str,
    segments: &[Segment],
    translations: &HashMap<u64, String>,
) -> String {
    use crate::markdown::hash_str;

    let mut result = String::with_capacity(source.len() * 2);
    let mut last_end = 0usize;

    for segment in segments {
        let hash = hash_str(&segment.source_text);
        let translation = match translations.get(&hash) {
            Some(t) => t,
            None => continue,
        };

        // Find this segment's text in the remaining source
        let search_area = &source[last_end..];
        if let Some(pos) = find_text_position(search_area, &segment.source_text) {
            let abs_pos = last_end + pos;
            // Find the end of this segment's content in the source.
            // The segment text might span multiple lines in the source.
            let abs_end = find_segment_end(source, abs_pos, &segment.source_text);

            // Append source up to and including this segment
            result.push_str(&source[last_end..abs_end]);

            // Append a blank line separator and the translation
            result.push_str("\n\n");
            result.push_str("> *");
            result.push_str(translation.trim());
            result.push_str("*\n");

            last_end = abs_end;
        }
    }

    // Append remaining source after the last translated segment
    result.push_str(&source[last_end..]);
    result
}

/// Find the position of segment text in the source.
/// The segment text has been normalized (whitespace collapsed) so we need
/// fuzzy matching against the original source.
fn find_text_position(haystack: &str, needle: &str) -> Option<usize> {
    // First try exact match
    if let Some(pos) = haystack.find(needle) {
        return Some(pos);
    }

    // Try to find the needle by matching the start word
    let needle_start = needle.split_whitespace().next()?;

    haystack.find(needle_start).and_then(|pos| {
        // Verify that the text at this position roughly matches
        let remaining = &haystack[pos..];
        let needle_words: Vec<&str> = needle.split_whitespace().collect();
        let mut hay_words = remaining.split_whitespace();

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
    // The needle text should be contained starting near `start`.
    // Find the actual end by looking for the needle text.
    let search_area = &source[start..];

    // Try exact match first
    if let Some(pos) = search_area.find(needle) {
        return start + pos + needle.len();
    }

    // Fuzzy: match word by word and find the end position
    let needle_words: Vec<&str> = needle.split_whitespace().collect();
    let mut word_positions = Vec::new();
    let mut search_from = 0usize;

    for word in &needle_words {
        if let Some(pos) = search_area[search_from..].find(word) {
            let abs_word_pos = search_from + pos;
            word_positions.push(abs_word_pos);
            search_from = abs_word_pos + word.len();
        } else {
            break;
        }
    }

    if let Some(&last_pos) = word_positions.last() {
        let last_word = needle_words.get(word_positions.len() - 1).unwrap();
        let end = start + last_pos + last_word.len();
        // Extend to end of line
        return source[end..]
            .find('\n')
            .map(|n| end + n)
            .unwrap_or(source.len());
    }

    // Last resort: just advance past start
    source[start..]
        .find('\n')
        .map(|n| start + n)
        .unwrap_or(source.len())
}
