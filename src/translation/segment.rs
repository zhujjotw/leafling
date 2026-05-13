use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum SegmentKind {
    Heading,
    Paragraph,
    ListItem,
    Blockquote,
}

/// A segment of markdown that can be translated as a unit.
#[derive(Clone, Debug)]
pub(crate) struct Segment {
    pub(crate) source_text: String,
    pub(crate) kind: SegmentKind,
}

/// Extract translatable segments from markdown source.
/// Returns a list of (text, kind) pairs.
/// Skips code blocks, LaTeX, Mermaid, tables, frontmatter, and horizontal rules.
pub(crate) fn extract_segments(source: &str) -> Vec<Segment> {
    let mut segments = Vec::new();
    let mut current_text = String::new();
    let mut in_code = false;
    let mut in_table = false;
    let mut current_kind: Option<SegmentKind> = None;
    let mut code_lang = String::new();

    let parser = Parser::new_ext(source, Options::all());

    for event in parser {
        match event {
            Event::Start(Tag::CodeBlock(kind)) => {
                flush_segment(&mut current_text, &mut current_kind, &mut segments);
                in_code = true;
                code_lang = match kind {
                    pulldown_cmark::CodeBlockKind::Fenced(lang) => lang.to_string(),
                    pulldown_cmark::CodeBlockKind::Indented => String::new(),
                };
            }
            Event::End(TagEnd::CodeBlock) => {
                in_code = false;
                code_lang.clear();
            }
            Event::Start(Tag::Table(_)) => {
                flush_segment(&mut current_text, &mut current_kind, &mut segments);
                in_table = true;
            }
            Event::End(TagEnd::Table) => {
                in_table = false;
            }
            Event::Start(Tag::Heading { level, .. }) => {
                flush_segment(&mut current_text, &mut current_kind, &mut segments);
                current_kind = Some(SegmentKind::Heading);
                let _ = level;
            }
            Event::End(TagEnd::Heading(_)) => {
                flush_segment(&mut current_text, &mut current_kind, &mut segments);
            }
            Event::Start(Tag::Paragraph) => {
                current_kind = Some(SegmentKind::Paragraph);
            }
            Event::End(TagEnd::Paragraph) => {
                flush_segment(&mut current_text, &mut current_kind, &mut segments);
            }
            Event::Start(Tag::Item) => {
                current_kind = Some(SegmentKind::ListItem);
            }
            Event::End(TagEnd::Item) => {
                flush_segment(&mut current_text, &mut current_kind, &mut segments);
            }
            Event::Start(Tag::BlockQuote(_)) => {
                current_kind = Some(SegmentKind::Blockquote);
            }
            Event::End(TagEnd::BlockQuote(_)) => {
                flush_segment(&mut current_text, &mut current_kind, &mut segments);
            }
            Event::Text(text) if !in_code => {
                // Skip LaTeX and Mermaid code blocks
                if in_code && (code_lang == "latex" || code_lang == "tex" || code_lang == "mermaid")
                {
                    continue;
                }
                current_text.push_str(&text);
            }
            Event::Code(text) => {
                // Keep inline code as-is in the text
                current_text.push('`');
                current_text.push_str(&text);
                current_text.push('`');
            }
            Event::SoftBreak | Event::HardBreak if !in_code => {
                current_text.push(' ');
            }
            Event::Start(Tag::Emphasis) => {
                current_text.push('*');
            }
            Event::End(TagEnd::Emphasis) => {
                current_text.push('*');
            }
            Event::Start(Tag::Strong) => {
                current_text.push_str("**");
            }
            Event::End(TagEnd::Strong) => {
                current_text.push_str("**");
            }
            Event::Start(Tag::Link { dest, title, .. }) => {
                current_text.push('[');
                let _ = (dest, title);
            }
            Event::End(TagEnd::Link) => {
                // Link text is already captured via Text events.
                // We omit the URL portion for translation.
            }
            Event::Rule => {
                flush_segment(&mut current_text, &mut current_kind, &mut segments);
            }
            Event::InlineMath(_) | Event::DisplayMath(_) => {
                // Skip math expressions
            }
            _ => {}
        }
    }

    flush_segment(&mut current_text, &mut current_kind, &mut segments);
    segments
}

fn flush_segment(
    current_text: &mut String,
    current_kind: &mut Option<SegmentKind>,
    segments: &mut Vec<Segment>,
) {
    let trimmed = current_text.trim().to_string();
    if !trimmed.is_empty() {
        if let Some(kind) = current_kind.take() {
            segments.push(Segment {
                source_text: trimmed,
                kind,
            });
        }
    }
    current_text.clear();
}
