use crate::{
    app::{normalize_toc, TocEntry},
    theme::{app_theme, MarkdownTheme},
};
use pulldown_cmark::{
    Alignment, CodeBlockKind, Event as MdEvent, HeadingLevel, Options, Parser, Tag, TagEnd,
};
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};
use std::{
    hash::{Hash, Hasher},
    io,
    path::PathBuf,
};
use syntect::{
    easy::HighlightLines, highlighting::Theme, parsing::SyntaxSet, util::LinesWithEndings,
};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

const TAB_STOP: usize = 4;

#[derive(Clone, Copy)]
enum ListKind {
    Unordered,
    Ordered(u64),
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum LastBlock {
    Other,
    Paragraph,
}

struct ItemState {
    marker_emitted: bool,
    continuation_indent: usize,
}

#[derive(Clone, Copy, Default)]
struct InlineStyleState {
    in_strong: bool,
    in_em: bool,
    in_strike: bool,
    in_link: bool,
}

struct TableBuf {
    alignments: Vec<Alignment>,
    rows: Vec<Vec<String>>,
    header_count: usize,
    current_row: Vec<String>,
    current_cell: String,
    in_header: bool,
}

struct TableBorder<'a> {
    left: &'a str,
    fill: &'a str,
    cross: &'a str,
    right: &'a str,
}

struct CodeBlockRenderContext<'a> {
    ss: &'a SyntaxSet,
    theme: &'a Theme,
    render_width: usize,
    theme_colors: &'a MarkdownTheme,
    blockquote_depth: usize,
    list_stack: &'a [ListKind],
}

pub(crate) fn line_plain_text(line: &Line<'_>) -> String {
    line.spans.iter().map(|s| s.content.as_ref()).collect()
}

pub(crate) fn build_plain_lines(lines: &[Line<'_>]) -> Vec<String> {
    lines.iter().map(line_plain_text).collect()
}

pub(crate) fn hash_str(text: &str) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    text.hash(&mut hasher);
    hasher.finish()
}

pub(crate) fn read_file_state(path: &PathBuf) -> Option<crate::app::FileState> {
    let metadata = std::fs::metadata(path).ok()?;
    Some(crate::app::FileState {
        modified: metadata.modified().ok()?,
        len: metadata.len(),
    })
}

pub(crate) fn hash_file_contents(path: &PathBuf) -> io::Result<u64> {
    std::fs::read_to_string(path).map(|contents| hash_str(&contents))
}

pub(crate) fn truncate_display_width(text: &str, max_width: usize) -> String {
    if display_width(text) <= max_width {
        return text.to_string();
    }
    if max_width == 0 {
        return String::new();
    }

    let mut out = String::new();
    let mut used = 0;
    for ch in text.chars() {
        let ch_w = UnicodeWidthChar::width(ch).unwrap_or(0);
        if used + ch_w > max_width.saturating_sub(1) {
            break;
        }
        out.push(ch);
        used += ch_w;
    }
    out.push('…');
    out
}

pub(crate) fn display_width(text: &str) -> usize {
    let mut width = 0;
    let mut parts = text.split('\t').peekable();
    while let Some(segment) = parts.next() {
        width += UnicodeWidthStr::width(segment);
        if parts.peek().is_some() {
            width += TAB_STOP - (width % TAB_STOP);
        }
    }
    width
}

fn expand_tabs(text: &str, start_width: usize) -> String {
    if !text.contains('\t') {
        return text.to_string();
    }

    let mut out = String::new();
    let mut width = start_width;
    let mut parts = text.split('\t').peekable();
    while let Some(segment) = parts.next() {
        out.push_str(segment);
        width += UnicodeWidthStr::width(segment);
        if parts.peek().is_some() {
            let spaces = TAB_STOP - (width % TAB_STOP);
            out.push_str(&" ".repeat(spaces));
            width += spaces;
        }
    }
    out
}

pub(crate) fn highlight_line<'a>(line: &Line<'a>) -> Line<'a> {
    let theme = &app_theme().markdown;
    Line::from(
        line.spans
            .iter()
            .map(|span| {
                Span::styled(
                    span.content.clone(),
                    span.style.bg(theme.search_highlight_bg),
                )
            })
            .collect::<Vec<_>>(),
    )
}

const DEFAULT_RENDER_WIDTH: usize = 80;

fn strip_frontmatter(src: &str) -> &str {
    let Some(rest) = src.strip_prefix("---\n") else {
        return src;
    };

    let mut offset = 4usize;
    for line in rest.split_inclusive('\n') {
        if line == "---\n" || line == "...\n" || line == "---" || line == "..." {
            return &src[offset + line.len()..];
        }
        offset += line.len();
    }

    src
}

fn syntect_to_color(c: syntect::highlighting::Color) -> Color {
    Color::Rgb(c.r, c.g, c.b)
}

pub(crate) fn resolve_syntax<'a>(
    lang: &str,
    ss: &'a SyntaxSet,
) -> &'a syntect::parsing::SyntaxReference {
    let raw = lang.trim();
    let normalized = raw
        .split(|c: char| c.is_whitespace() || c == ',' || c == '{')
        .next()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();

    let aliases: &[&str] = match normalized.as_str() {
        "ts" | "typescript" => &[
            "JavaScript",
            "js",
            "javascript",
            "TypeScript",
            "ts",
            "typescript",
        ],
        "tsx" => &["JSX", "jsx", "JavaScript", "js", "typescriptreact", "tsx"],
        "js" | "javascript" => &["JavaScript", "js", "javascript"],
        "jsx" => &["JSX", "jsx", "JavaScript React"],
        "shell" | "bash" | "sh" | "zsh" => &["Bourne Again Shell (bash)", "bash", "sh"],
        "py" | "python" => &["Python", "py", "python"],
        "c" => &["C", "c"],
        "cpp" | "cxx" | "cc" | "c++" => &["C++", "cpp", "cxx", "cc"],
        "json" => &["JSON", "json"],
        "toml" => &["TOML", "toml"],
        "java" => &["Java", "java"],
        "kt" | "kotlin" => &["Kotlin", "kt", "kotlin"],
        "ps1" | "powershell" | "pwsh" => &["PowerShell", "ps1", "powershell"],
        "docker" | "dockerfile" => &["Dockerfile", "dockerfile"],
        "yml" | "yaml" => &["YAML", "yml", "yaml"],
        "rs" | "rust" => &["Rust", "rs", "rust"],
        _ if normalized.is_empty() => &[],
        _ => &[],
    };

    ss.find_syntax_by_token(raw)
        .or_else(|| ss.find_syntax_by_extension(raw))
        .or_else(|| ss.find_syntax_by_token(&normalized))
        .or_else(|| ss.find_syntax_by_extension(&normalized))
        .or_else(|| {
            aliases.iter().find_map(|alias| {
                ss.find_syntax_by_token(alias)
                    .or_else(|| ss.find_syntax_by_extension(alias))
                    .or_else(|| ss.find_syntax_by_name(alias))
            })
        })
        .unwrap_or_else(|| ss.find_syntax_plain_text())
}

fn highlight_code(
    code: &str,
    lang: &str,
    ss: &SyntaxSet,
    theme: &Theme,
    render_width: usize,
) -> (Vec<Line<'static>>, usize) {
    let theme_colors = &app_theme().markdown;
    let syntax = resolve_syntax(lang, ss);
    let mut hl = HighlightLines::new(syntax, theme);
    let gutter = Style::default().fg(theme_colors.code_gutter);

    let mut raw: Vec<(Vec<Span<'static>>, usize)> = Vec::new();
    for line_str in LinesWithEndings::from(code) {
        let regions = hl.highlight_line(line_str, ss).unwrap_or_default();
        let mut spans = vec![Span::styled("│ ", gutter)];
        let mut text_width: usize = 0;
        for (st, text) in &regions {
            let t = expand_tabs(text.trim_end_matches('\n'), text_width);
            if t.is_empty() {
                continue;
            }
            text_width += display_width(&t);
            let mut rs = Style::default().fg(syntect_to_color(st.foreground));
            if st
                .font_style
                .contains(syntect::highlighting::FontStyle::BOLD)
            {
                rs = rs.add_modifier(Modifier::BOLD);
            }
            if st
                .font_style
                .contains(syntect::highlighting::FontStyle::ITALIC)
            {
                rs = rs.add_modifier(Modifier::ITALIC);
            }
            if st
                .font_style
                .contains(syntect::highlighting::FontStyle::UNDERLINE)
            {
                rs = rs.add_modifier(Modifier::UNDERLINED);
            }
            spans.push(Span::styled(t, rs));
        }
        raw.push((spans, text_width));
    }

    let label = if lang.is_empty() { "text" } else { lang };
    let max_text = raw.iter().map(|(_, w)| *w).max().unwrap_or(0);
    let max_inner_width = render_width
        .saturating_sub(4)
        .max(UnicodeWidthStr::width(label) + 3);
    let min_inner = (UnicodeWidthStr::width(label) + 3)
        .max(44)
        .min(max_inner_width);
    let inner_width = (max_text + 2).max(min_inner);

    let mut out = Vec::new();
    for (mut spans, text_width) in raw {
        let pad = inner_width.saturating_sub(text_width + 1);
        spans.push(Span::raw(" ".repeat(pad)));
        spans.push(Span::styled("│", gutter));
        out.push(Line::from(spans));
    }
    (out, inner_width)
}

fn block_prefix(in_bq: bool) -> Vec<Span<'static>> {
    let theme = &app_theme().markdown;
    if in_bq {
        vec![Span::styled(
            "▏ ",
            Style::default().fg(theme.blockquote_marker),
        )]
    } else {
        vec![]
    }
}

fn list_item_prefix(
    in_bq: bool,
    list_stack: &[ListKind],
    item_stack: &mut [ItemState],
) -> Vec<Span<'static>> {
    let theme = &app_theme().markdown;
    let mut prefix = block_prefix(in_bq);
    let Some(item) = item_stack.last_mut() else {
        return prefix;
    };

    if item.marker_emitted {
        prefix.push(Span::raw(" ".repeat(item.continuation_indent)));
        return prefix;
    }

    let depth = list_stack.len();
    prefix.push(Span::raw("  ".repeat(depth.saturating_sub(1))));

    let marker = match list_stack.last().copied().unwrap_or(ListKind::Unordered) {
        ListKind::Unordered => match depth {
            1 => "• ".to_string(),
            2 => "◦ ".to_string(),
            _ => "▸ ".to_string(),
        },
        ListKind::Ordered(n) => format!("{n}. "),
    };
    item.continuation_indent = "  ".repeat(depth.saturating_sub(1)).len() + display_width(&marker);
    item.marker_emitted = true;

    let marker_style = match list_stack.last().copied().unwrap_or(ListKind::Unordered) {
        ListKind::Unordered => match depth {
            1 => Style::default().fg(theme.list_level_1),
            2 => Style::default().fg(theme.list_level_2),
            _ => Style::default().fg(theme.list_level_3),
        },
        ListKind::Ordered(_) => Style::default().fg(theme.ordered_list),
    };
    prefix.push(Span::styled(marker, marker_style));
    prefix
}

fn push_wrapped_prefixed_lines(
    lines: &mut Vec<Line<'static>>,
    body_spans: &mut Vec<Span<'static>>,
    first_prefix: Vec<Span<'static>>,
    continuation_prefix: Vec<Span<'static>>,
    render_width: usize,
) {
    if body_spans.is_empty() {
        return;
    }

    let first_prefix_width: usize = first_prefix
        .iter()
        .map(|span| display_width(span.content.as_ref()))
        .sum();
    let continuation_prefix_width: usize = continuation_prefix
        .iter()
        .map(|span| display_width(span.content.as_ref()))
        .sum();
    let max_width = render_width
        .saturating_sub(first_prefix_width.max(continuation_prefix_width))
        .max(8);

    let mut current_prefix = first_prefix.clone();
    let mut next_prefix = continuation_prefix.clone();
    let mut current_width = 0usize;
    let mut body_started = false;

    let push_current = |lines: &mut Vec<Line<'static>>,
                        current_prefix: &mut Vec<Span<'static>>,
                        next_prefix: &mut Vec<Span<'static>>,
                        body_started: &mut bool,
                        current_width: &mut usize| {
        if *body_started {
            lines.push(Line::from(std::mem::take(current_prefix)));
            *current_prefix = next_prefix.clone();
            *body_started = false;
            *current_width = 0;
        }
    };

    for span in body_spans.drain(..) {
        let style = span.style;
        let mut token = String::new();
        let mut token_is_space = false;

        let mut flush_token = |token: &mut String,
                               token_is_space: bool,
                               lines: &mut Vec<Line<'static>>,
                               current_prefix: &mut Vec<Span<'static>>,
                               body_started: &mut bool,
                               current_width: &mut usize| {
            if token.is_empty() {
                return;
            }

            let token_width = display_width(token);
            if token_is_space {
                let keep_styled_padding = style.bg.is_some();
                if (*body_started || keep_styled_padding)
                    && *current_width + token_width <= max_width
                {
                    current_prefix.push(Span::styled(std::mem::take(token), style));
                    *current_width += token_width;
                    *body_started = true;
                } else {
                    token.clear();
                }
                return;
            }

            if *body_started && *current_width + token_width > max_width {
                push_current(
                    lines,
                    current_prefix,
                    &mut next_prefix,
                    body_started,
                    current_width,
                );
            }

            if token_width <= max_width {
                current_prefix.push(Span::styled(std::mem::take(token), style));
                *current_width += token_width;
                *body_started = true;
                return;
            }

            let mut chunk = String::new();
            let mut chunk_width = 0usize;
            for ch in token.chars() {
                let ch_width = UnicodeWidthChar::width(ch).unwrap_or(0);
                let would_overflow = if *body_started {
                    *current_width + chunk_width + ch_width > max_width
                } else {
                    chunk_width + ch_width > max_width
                };
                if would_overflow {
                    if !chunk.is_empty() {
                        current_prefix.push(Span::styled(std::mem::take(&mut chunk), style));
                        *body_started = true;
                    }
                    push_current(
                        lines,
                        current_prefix,
                        &mut next_prefix,
                        body_started,
                        current_width,
                    );
                    chunk_width = 0;
                }

                chunk.push(ch);
                chunk_width += ch_width;
            }

            if !chunk.is_empty() {
                current_prefix.push(Span::styled(chunk, style));
                *current_width += chunk_width;
                *body_started = true;
            }
            token.clear();
        };

        for ch in span.content.chars() {
            let is_space = ch.is_whitespace();
            if token.is_empty() {
                token_is_space = is_space;
            } else if token_is_space != is_space {
                flush_token(
                    &mut token,
                    token_is_space,
                    lines,
                    &mut current_prefix,
                    &mut body_started,
                    &mut current_width,
                );
                token_is_space = is_space;
            }
            token.push(ch);
        }

        flush_token(
            &mut token,
            token_is_space,
            lines,
            &mut current_prefix,
            &mut body_started,
            &mut current_width,
        );
    }

    if body_started {
        lines.push(Line::from(current_prefix));
    }
}

fn push_wrapped_blockquote_lines(
    lines: &mut Vec<Line<'static>>,
    body_spans: &mut Vec<Span<'static>>,
    render_width: usize,
) {
    let prefix = block_prefix(true);
    push_wrapped_prefixed_lines(lines, body_spans, prefix.clone(), prefix, render_width);
}

fn flush_wrapped_spans(
    lines: &mut Vec<Line<'static>>,
    spans: &mut Vec<Span<'static>>,
    blockquote_depth: usize,
    list_stack: &[ListKind],
    item_stack: &mut [ItemState],
    render_width: usize,
) {
    if blockquote_depth > 0 && item_stack.is_empty() {
        push_wrapped_blockquote_lines(lines, spans, render_width);
    } else if !item_stack.is_empty() {
        let first_prefix = list_item_prefix(blockquote_depth > 0, list_stack, item_stack);
        let continuation_prefix = list_item_prefix(blockquote_depth > 0, list_stack, item_stack);
        push_wrapped_prefixed_lines(
            lines,
            spans,
            first_prefix,
            continuation_prefix,
            render_width,
        );
    } else if !spans.is_empty() {
        let mut all = block_prefix(false);
        all.append(spans);
        lines.push(Line::from(all));
    }
}

fn trim_paragraph_gap_before_block(lines: &mut Vec<Line<'static>>, last_block: LastBlock) {
    if last_block == LastBlock::Paragraph
        && lines
            .last()
            .is_some_and(|line| line_plain_text(line).is_empty())
    {
        lines.pop();
    }
}

fn push_heading_lines(
    lines: &mut Vec<Line<'static>>,
    toc: &mut Vec<TocEntry>,
    spans: &mut Vec<Span<'static>>,
    level: u8,
    render_width: usize,
    theme: &MarkdownTheme,
) {
    let color: Color = match level {
        1 => theme.heading_1,
        2 => theme.heading_2,
        3 => theme.heading_3,
        _ => theme.heading_other,
    };
    let style = Style::default().fg(color).add_modifier(match level {
        1..=3 => Modifier::BOLD,
        _ => Modifier::empty(),
    });
    let title: String = spans.iter().map(|s| s.content.as_ref()).collect();
    let rendered_title = if level == 3 {
        format!("{title} ")
    } else {
        title.clone()
    };
    toc.push(TocEntry {
        level,
        title: title.clone(),
        line: lines.len(),
    });
    spans.clear();
    lines.push(Line::from(vec![Span::styled(rendered_title, style)]));

    match level {
        1 => lines.push(Line::from(Span::styled(
            "═".repeat(display_width(&title).min(rule_width(render_width, 0))),
            Style::default().fg(theme.heading_underline),
        ))),
        2 => lines.push(Line::from(Span::styled(
            "─".repeat(display_width(&title).min(rule_width(render_width, 0))),
            Style::default().fg(theme.heading_underline),
        ))),
        _ => {}
    }
}

fn push_code_block_lines(
    lines: &mut Vec<Line<'static>>,
    code_buf: &mut String,
    code_lang: &mut String,
    ctx: CodeBlockRenderContext<'_>,
    item_stack: &mut [ItemState],
) {
    let prefix = if !item_stack.is_empty() {
        list_item_prefix(ctx.blockquote_depth > 0, ctx.list_stack, item_stack)
    } else if ctx.blockquote_depth > 0 {
        block_prefix(true)
    } else {
        Vec::new()
    };
    let prefix_width: usize = prefix
        .iter()
        .map(|span| display_width(span.content.as_ref()))
        .sum();
    let label = if code_lang.is_empty() {
        "text".to_string()
    } else {
        code_lang.clone()
    };
    let available_width = ctx.render_width.saturating_sub(prefix_width);
    let (code_lines, inner_width) =
        highlight_code(code_buf, code_lang, ctx.ss, ctx.theme, available_width);
    let header_width = UnicodeWidthStr::width(label.as_str()) + 3;
    let top_bar = "─".repeat(inner_width.saturating_sub(header_width));
    let mut header = prefix.clone();
    header.extend([
        Span::styled(
            "┌─ ".to_string(),
            Style::default().fg(ctx.theme_colors.code_frame),
        ),
        Span::styled(
            format!("{label} "),
            Style::default().fg(ctx.theme_colors.code_label),
        ),
        Span::styled(
            format!("{top_bar}┐"),
            Style::default().fg(ctx.theme_colors.code_frame),
        ),
    ]);
    lines.push(Line::from(header));
    lines.extend(code_lines.into_iter().map(|line| {
        let mut spans = prefix.clone();
        spans.extend(line.spans);
        Line::from(spans)
    }));
    let mut footer = prefix;
    footer.push(Span::styled(
        format!("└{}┘", "─".repeat(inner_width)),
        Style::default().fg(ctx.theme_colors.code_frame),
    ));
    lines.push(Line::from(footer));
    lines.push(Line::from(""));
    code_lang.clear();
    code_buf.clear();
}

fn inline_text_style(
    theme: &MarkdownTheme,
    blockquote_depth: usize,
    inline: InlineStyleState,
) -> Style {
    let mut style = if blockquote_depth > 0 {
        Style::default()
            .fg(theme.blockquote_text)
            .add_modifier(Modifier::ITALIC)
    } else if inline.in_link {
        Style::default().fg(theme.link_text)
    } else {
        Style::default().fg(theme.text)
    };

    if inline.in_strong {
        style = style.fg(theme.strong_text).add_modifier(Modifier::BOLD);
    }
    if inline.in_em {
        style = style.add_modifier(Modifier::ITALIC);
    }
    if inline.in_strike {
        style = style.add_modifier(Modifier::CROSSED_OUT);
    }

    style
}

fn flush_list_item_spans(
    lines: &mut Vec<Line<'static>>,
    spans: &mut Vec<Span<'static>>,
    list_stack: &[ListKind],
    item_stack: &mut [ItemState],
    blockquote_depth: usize,
    render_width: usize,
) {
    if spans.is_empty() {
        return;
    }

    let first_prefix = list_item_prefix(blockquote_depth > 0, list_stack, item_stack);
    let continuation_prefix = list_item_prefix(blockquote_depth > 0, list_stack, item_stack);
    push_wrapped_prefixed_lines(
        lines,
        spans,
        first_prefix,
        continuation_prefix,
        render_width,
    );
}

fn handle_table_event(
    table: &mut Option<TableBuf>,
    ev: &MdEvent<'_>,
    lines: &mut Vec<Line<'static>>,
    render_width: usize,
) -> bool {
    let Some(tb) = table.as_mut() else {
        return false;
    };

    match ev {
        MdEvent::Text(t) | MdEvent::Code(t) => {
            tb.push_text(t.as_ref());
            true
        }
        MdEvent::Start(Tag::TableCell) => true,
        MdEvent::End(TagEnd::TableCell) => {
            tb.end_cell();
            true
        }
        MdEvent::Start(Tag::TableRow) => true,
        MdEvent::End(TagEnd::TableRow) => {
            tb.end_row();
            true
        }
        MdEvent::Start(Tag::TableHead) => {
            tb.in_header = true;
            true
        }
        MdEvent::End(TagEnd::TableHead) => {
            tb.end_header();
            true
        }
        MdEvent::Start(Tag::Strong)
        | MdEvent::End(TagEnd::Strong)
        | MdEvent::Start(Tag::Emphasis)
        | MdEvent::End(TagEnd::Emphasis)
        | MdEvent::Start(Tag::Link { .. })
        | MdEvent::End(TagEnd::Link) => true,
        MdEvent::End(TagEnd::Table) => {
            let rendered = tb.render(render_width);
            lines.extend(rendered);
            *table = None;
            true
        }
        _ => true,
    }
}

fn start_list(
    lines: &mut Vec<Line<'static>>,
    last_block: LastBlock,
    list_stack: &mut Vec<ListKind>,
    start: Option<u64>,
) {
    trim_paragraph_gap_before_block(lines, last_block);
    list_stack.push(match start {
        Some(n) => ListKind::Ordered(n),
        None => ListKind::Unordered,
    });
}

fn start_table(table: &mut Option<TableBuf>, aligns: &[Alignment]) {
    *table = Some(TableBuf::new(aligns.to_vec()));
}

fn end_list(lines: &mut Vec<Line<'static>>, list_stack: &mut Vec<ListKind>) {
    list_stack.pop();
    if list_stack.is_empty() {
        lines.push(Line::from(""));
    }
}

fn start_item(item_stack: &mut Vec<ItemState>) {
    item_stack.push(ItemState {
        marker_emitted: false,
        continuation_indent: 0,
    });
}

fn end_item(
    lines: &mut Vec<Line<'static>>,
    spans: &mut Vec<Span<'static>>,
    list_stack: &mut [ListKind],
    item_stack: &mut Vec<ItemState>,
    blockquote_depth: usize,
    render_width: usize,
) {
    flush_list_item_spans(
        lines,
        spans,
        list_stack,
        item_stack,
        blockquote_depth,
        render_width,
    );
    item_stack.pop();
    if let Some(ListKind::Ordered(next)) = list_stack.last_mut() {
        *next += 1;
    }
}

fn end_blockquote(
    lines: &mut Vec<Line<'static>>,
    spans: &mut Vec<Span<'static>>,
    blockquote_depth: &mut usize,
    theme: &MarkdownTheme,
) {
    if !spans.is_empty() {
        let mut all = vec![Span::styled(
            "▏ ",
            Style::default().fg(theme.blockquote_marker),
        )];
        all.append(spans);
        lines.push(Line::from(all));
    }
    *blockquote_depth = blockquote_depth.saturating_sub(1);
    lines.push(Line::from(""));
}

fn push_rule_line(lines: &mut Vec<Line<'static>>, render_width: usize, theme: &MarkdownTheme) {
    lines.push(Line::from(Span::styled(
        "─".repeat(rule_width(render_width, 0)),
        Style::default().fg(theme.rule),
    )));
    lines.push(Line::from(""));
}

fn push_inline_code_span(spans: &mut Vec<Span<'static>>, text: &str, theme: &MarkdownTheme) {
    spans.push(Span::styled(
        format!(" {} ", text),
        Style::default()
            .fg(theme.inline_code_fg)
            .bg(theme.inline_code_bg),
    ));
}

fn push_link_marker(spans: &mut Vec<Span<'static>>, theme: &MarkdownTheme) {
    spans.push(Span::styled("⌗", Style::default().fg(theme.link_icon)));
}

fn handle_inline_style_event(
    ev: &MdEvent<'_>,
    inline: &mut InlineStyleState,
    spans: &mut Vec<Span<'static>>,
    theme: &MarkdownTheme,
) -> bool {
    match ev {
        MdEvent::Start(Tag::Strong) => {
            inline.in_strong = true;
            true
        }
        MdEvent::End(TagEnd::Strong) => {
            inline.in_strong = false;
            true
        }
        MdEvent::Start(Tag::Emphasis) => {
            inline.in_em = true;
            true
        }
        MdEvent::End(TagEnd::Emphasis) => {
            inline.in_em = false;
            true
        }
        MdEvent::Start(Tag::Strikethrough) => {
            inline.in_strike = true;
            true
        }
        MdEvent::End(TagEnd::Strikethrough) => {
            inline.in_strike = false;
            true
        }
        MdEvent::Start(Tag::Link { .. }) => {
            inline.in_link = true;
            push_link_marker(spans, theme);
            true
        }
        MdEvent::End(TagEnd::Link) => {
            inline.in_link = false;
            true
        }
        _ => false,
    }
}

fn heading_level(level: HeadingLevel) -> u8 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        _ => 4,
    }
}

fn start_heading(in_heading: &mut Option<u8>, level: HeadingLevel) {
    *in_heading = Some(heading_level(level));
}

fn end_heading(
    lines: &mut Vec<Line<'static>>,
    toc: &mut Vec<TocEntry>,
    spans: &mut Vec<Span<'static>>,
    in_heading: &mut Option<u8>,
    render_width: usize,
    theme: &MarkdownTheme,
) {
    push_heading_lines(
        lines,
        toc,
        spans,
        in_heading.unwrap_or(1),
        render_width,
        theme,
    );
    *in_heading = None;
}

fn start_code_block(
    lines: &mut Vec<Line<'static>>,
    last_block: LastBlock,
    in_code: &mut bool,
    code_buf: &mut String,
    code_lang: &mut String,
    kind: &CodeBlockKind<'_>,
) {
    trim_paragraph_gap_before_block(lines, last_block);
    *in_code = true;
    code_buf.clear();
    *code_lang = match kind {
        CodeBlockKind::Fenced(lang) => lang.to_string(),
        CodeBlockKind::Indented => String::new(),
    };
}

fn end_line_break(
    lines: &mut Vec<Line<'static>>,
    spans: &mut Vec<Span<'static>>,
    in_code: bool,
    blockquote_depth: usize,
    list_stack: &[ListKind],
    item_stack: &mut [ItemState],
    render_width: usize,
) {
    if !in_code {
        flush_wrapped_spans(
            lines,
            spans,
            blockquote_depth,
            list_stack,
            item_stack,
            render_width,
        );
    }
}

fn end_paragraph(
    lines: &mut Vec<Line<'static>>,
    spans: &mut Vec<Span<'static>>,
    blockquote_depth: usize,
    list_stack: &[ListKind],
    item_stack: &mut [ItemState],
    render_width: usize,
) {
    flush_wrapped_spans(
        lines,
        spans,
        blockquote_depth,
        list_stack,
        item_stack,
        render_width,
    );
    lines.push(Line::from(""));
}

fn push_text_event(
    spans: &mut Vec<Span<'static>>,
    code_buf: &mut String,
    text: &str,
    in_code: bool,
    theme: &MarkdownTheme,
    blockquote_depth: usize,
    inline: InlineStyleState,
) {
    if in_code {
        code_buf.push_str(text);
    } else {
        spans.push(Span::styled(
            text.to_string(),
            inline_text_style(theme, blockquote_depth, inline),
        ));
    }
}

impl TableBuf {
    fn new(alignments: Vec<Alignment>) -> Self {
        Self {
            alignments,
            rows: vec![],
            header_count: 0,
            current_row: vec![],
            current_cell: String::new(),
            in_header: false,
        }
    }
    fn push_text(&mut self, t: &str) {
        self.current_cell.push_str(t);
    }
    fn end_cell(&mut self) {
        let cell = std::mem::take(&mut self.current_cell).trim().to_string();
        self.current_row.push(cell);
    }
    fn end_row(&mut self) {
        let row = std::mem::take(&mut self.current_row);
        if !row.is_empty() {
            self.rows.push(row);
        }
    }
    fn end_header(&mut self) {
        self.end_row();
        self.header_count = self.rows.len();
        self.in_header = false;
    }

    fn render(&self, render_width: usize) -> Vec<Line<'static>> {
        let theme = &app_theme().markdown;
        if self.rows.is_empty() {
            return vec![];
        }
        let col_count = self.rows.iter().map(|r| r.len()).max().unwrap_or(0);
        if col_count == 0 {
            return vec![];
        }

        let mut col_widths: Vec<usize> = vec![1; col_count];
        let mut min_widths: Vec<usize> = vec![4; col_count];
        for row in &self.rows {
            for (ci, cell) in row.iter().enumerate() {
                if ci < col_count {
                    col_widths[ci] = col_widths[ci].max(display_width(cell));
                    min_widths[ci] = min_widths[ci].max(min_table_cell_width(cell));
                }
            }
        }

        fit_table_widths(&mut col_widths, &min_widths, render_width);

        let border = Style::default().fg(theme.table_border);
        let sep = Style::default().fg(theme.table_separator);
        let header = Style::default()
            .fg(theme.table_header)
            .add_modifier(Modifier::BOLD);
        let cell = Style::default().fg(theme.table_cell);
        let ind = "";

        let mut out: Vec<Line<'static>> = Vec::new();
        out.push(self.hline(
            ind,
            TableBorder {
                left: "┌",
                fill: "─",
                cross: "┬",
                right: "┐",
            },
            &col_widths,
            border,
        ));

        for (ri, row) in self.rows.iter().enumerate() {
            let is_hdr = ri < self.header_count;
            let wrapped_cells: Vec<Vec<String>> = col_widths
                .iter()
                .copied()
                .enumerate()
                .take(col_count)
                .map(|(ci, width)| {
                    wrap_table_cell(row.get(ci).map(|s| s.as_str()).unwrap_or(""), width)
                })
                .collect();
            let row_height = wrapped_cells
                .iter()
                .map(|lines| lines.len())
                .max()
                .unwrap_or(1);

            for line_idx in 0..row_height {
                let mut spans = vec![Span::raw(ind), Span::styled("│", border)];
                for (ci, width) in col_widths.iter().copied().enumerate().take(col_count) {
                    let txt = wrapped_cells[ci]
                        .get(line_idx)
                        .map(|s| s.as_str())
                        .unwrap_or("");
                    let align = self.alignments.get(ci).copied().unwrap_or(Alignment::None);
                    let pad = align_cell(txt, width, align);
                    let st = if is_hdr { header } else { cell };
                    spans.push(Span::raw(" "));
                    spans.push(Span::styled(pad, st));
                    spans.push(Span::raw(" "));
                    spans.push(Span::styled("│", border));
                }
                out.push(Line::from(spans));
            }

            if is_hdr && ri == self.header_count - 1 {
                out.push(self.hline(
                    ind,
                    TableBorder {
                        left: "╞",
                        fill: "═",
                        cross: "╪",
                        right: "╡",
                    },
                    &col_widths,
                    sep,
                ));
            } else if !is_hdr && ri < self.rows.len() - 1 {
                out.push(self.hline(
                    ind,
                    TableBorder {
                        left: "├",
                        fill: "─",
                        cross: "┼",
                        right: "┤",
                    },
                    &col_widths,
                    border,
                ));
            }
        }

        out.push(self.hline(
            ind,
            TableBorder {
                left: "└",
                fill: "─",
                cross: "┴",
                right: "┘",
            },
            &col_widths,
            border,
        ));
        out.push(Line::from(""));
        out
    }

    fn hline(
        &self,
        indent: &str,
        border: TableBorder<'_>,
        col_widths: &[usize],
        style: Style,
    ) -> Line<'static> {
        let mut spans = vec![
            Span::raw(indent.to_string()),
            Span::styled(border.left.to_string(), style),
        ];
        for (ci, &w) in col_widths.iter().enumerate() {
            spans.push(Span::styled(border.fill.repeat(w + 2), style));
            if ci < col_widths.len() - 1 {
                spans.push(Span::styled(border.cross.to_string(), style));
            }
        }
        spans.push(Span::styled(border.right.to_string(), style));
        Line::from(spans)
    }
}

fn min_table_cell_width(text: &str) -> usize {
    let max_word = text
        .split_whitespace()
        .map(display_width)
        .max()
        .unwrap_or(0)
        .min(12);
    max_word.max(4)
}

fn fit_table_widths(col_widths: &mut [usize], min_widths: &[usize], render_width: usize) {
    if col_widths.is_empty() {
        return;
    }

    let col_count = col_widths.len();
    let border_width = 3 * col_count + 1;
    let available = render_width.saturating_sub(border_width).max(col_count);
    let min_total: usize = min_widths.iter().sum();

    if min_total >= available {
        let mut widths = vec![1; col_count];
        let mut remaining = available.saturating_sub(col_count);
        let mut order: Vec<usize> = (0..col_count).collect();
        order.sort_by_key(|&idx| std::cmp::Reverse(min_widths[idx]));
        for idx in order {
            if remaining == 0 {
                break;
            }
            let extra = (min_widths[idx].saturating_sub(1)).min(remaining);
            widths[idx] += extra;
            remaining -= extra;
        }
        col_widths.copy_from_slice(&widths);
        return;
    }

    while col_widths.iter().sum::<usize>() > available {
        let Some((idx, _)) = col_widths
            .iter()
            .enumerate()
            .filter(|(idx, width)| **width > min_widths[*idx])
            .max_by_key(|(_, width)| **width)
        else {
            break;
        };
        col_widths[idx] -= 1;
    }
}

fn wrap_table_cell(text: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return vec![String::new()];
    }
    let expanded = expand_tabs(text, 0);
    if expanded.is_empty() {
        return vec![String::new()];
    }

    let mut lines = Vec::new();
    let mut current = String::new();
    let mut current_width = 0usize;

    for word in expanded.split_whitespace() {
        let word_width = display_width(word);

        if word_width > width {
            if !current.is_empty() {
                lines.push(std::mem::take(&mut current));
                current_width = 0;
            }
            let mut chunk = String::new();
            let mut chunk_width = 0usize;
            for ch in word.chars() {
                let ch_width = UnicodeWidthChar::width(ch).unwrap_or(0);
                if chunk_width + ch_width > width && !chunk.is_empty() {
                    lines.push(std::mem::take(&mut chunk));
                    chunk_width = 0;
                }
                chunk.push(ch);
                chunk_width += ch_width;
            }
            if !chunk.is_empty() {
                current = chunk;
                current_width = chunk_width;
            }
            continue;
        }

        let sep = if current.is_empty() { 0 } else { 1 };
        if current_width + sep + word_width > width && !current.is_empty() {
            lines.push(std::mem::take(&mut current));
            current_width = 0;
        }
        if !current.is_empty() {
            current.push(' ');
            current_width += 1;
        }
        current.push_str(word);
        current_width += word_width;
    }

    if !current.is_empty() {
        lines.push(current);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

fn align_cell(text: &str, width: usize, align: Alignment) -> String {
    let text = expand_tabs(text, 0);
    let len = display_width(&text);
    if len >= width {
        return text;
    }
    let pad = width - len;
    match align {
        Alignment::Right => format!("{}{}", " ".repeat(pad), text),
        Alignment::Center => {
            let l = pad / 2;
            format!("{}{}{}", " ".repeat(l), text, " ".repeat(pad - l))
        }
        _ => format!("{}{}", text, " ".repeat(pad)),
    }
}

pub(crate) fn parse_markdown(
    src: &str,
    ss: &SyntaxSet,
    theme: &Theme,
) -> (Vec<Line<'static>>, Vec<TocEntry>) {
    parse_markdown_with_width(src, ss, theme, DEFAULT_RENDER_WIDTH)
}

fn rule_width(render_width: usize, indent: usize) -> usize {
    render_width.saturating_sub(indent).max(8)
}

pub(crate) fn parse_markdown_with_width(
    src: &str,
    ss: &SyntaxSet,
    theme: &Theme,
    render_width: usize,
) -> (Vec<Line<'static>>, Vec<TocEntry>) {
    let theme_colors = &app_theme().markdown;
    let src = strip_frontmatter(src);
    let mut lines: Vec<Line<'static>> = Vec::new();
    let mut toc: Vec<TocEntry> = Vec::new();

    let mut spans: Vec<Span<'static>> = Vec::new();
    let mut in_heading: Option<u8> = None;
    let mut in_code = false;
    let mut code_lang = String::new();
    let mut code_buf = String::new();
    let mut blockquote_depth = 0usize;
    let mut inline = InlineStyleState::default();
    let mut list_stack: Vec<ListKind> = Vec::new();
    let mut item_stack: Vec<ItemState> = Vec::new();
    let mut table: Option<TableBuf> = None;
    let mut last_block = LastBlock::Other;

    for ev in Parser::new_ext(src, Options::all()) {
        if table.is_some() && handle_table_event(&mut table, &ev, &mut lines, render_width) {
            continue;
        }
        if handle_inline_style_event(&ev, &mut inline, &mut spans, theme_colors) {
            continue;
        }

        match ev {
            MdEvent::Start(Tag::Table(aligns)) => {
                start_table(&mut table, &aligns);
            }
            MdEvent::Start(Tag::Heading { level, .. }) => {
                start_heading(&mut in_heading, level);
            }
            MdEvent::End(TagEnd::Heading(_)) => {
                end_heading(
                    &mut lines,
                    &mut toc,
                    &mut spans,
                    &mut in_heading,
                    render_width,
                    theme_colors,
                );
                last_block = LastBlock::Other;
            }
            MdEvent::Start(Tag::Paragraph) => {}
            MdEvent::End(TagEnd::Paragraph) => {
                end_paragraph(
                    &mut lines,
                    &mut spans,
                    blockquote_depth,
                    &list_stack,
                    &mut item_stack,
                    render_width,
                );
                last_block = LastBlock::Paragraph;
            }
            MdEvent::Start(Tag::CodeBlock(kind)) => {
                start_code_block(
                    &mut lines,
                    last_block,
                    &mut in_code,
                    &mut code_buf,
                    &mut code_lang,
                    &kind,
                );
                last_block = LastBlock::Other;
            }
            MdEvent::End(TagEnd::CodeBlock) => {
                in_code = false;
                push_code_block_lines(
                    &mut lines,
                    &mut code_buf,
                    &mut code_lang,
                    CodeBlockRenderContext {
                        ss,
                        theme,
                        render_width,
                        theme_colors,
                        blockquote_depth,
                        list_stack: &list_stack,
                    },
                    &mut item_stack,
                );
                last_block = LastBlock::Other;
            }
            MdEvent::Code(text) => {
                push_inline_code_span(&mut spans, text.as_ref(), theme_colors);
            }
            MdEvent::Start(Tag::BlockQuote(_)) => {
                blockquote_depth += 1;
            }
            MdEvent::End(TagEnd::BlockQuote(_)) => {
                end_blockquote(&mut lines, &mut spans, &mut blockquote_depth, theme_colors);
                last_block = LastBlock::Other;
            }
            MdEvent::Start(Tag::List(start)) => {
                start_list(&mut lines, last_block, &mut list_stack, start);
                last_block = LastBlock::Other;
            }
            MdEvent::End(TagEnd::List(_)) => {
                end_list(&mut lines, &mut list_stack);
                last_block = LastBlock::Other;
            }
            MdEvent::Start(Tag::Item) => {
                start_item(&mut item_stack);
            }
            MdEvent::End(TagEnd::Item) => {
                end_item(
                    &mut lines,
                    &mut spans,
                    &mut list_stack,
                    &mut item_stack,
                    blockquote_depth,
                    render_width,
                );
                last_block = LastBlock::Other;
            }
            MdEvent::Rule => {
                push_rule_line(&mut lines, render_width, theme_colors);
                last_block = LastBlock::Other;
            }
            MdEvent::Text(text) => {
                push_text_event(
                    &mut spans,
                    &mut code_buf,
                    text.as_ref(),
                    in_code,
                    theme_colors,
                    blockquote_depth,
                    inline,
                );
            }
            MdEvent::SoftBreak | MdEvent::HardBreak => {
                end_line_break(
                    &mut lines,
                    &mut spans,
                    in_code,
                    blockquote_depth,
                    &list_stack,
                    &mut item_stack,
                    render_width,
                );
            }
            _ => {}
        }
    }

    if !spans.is_empty() {
        lines.push(Line::from(spans));
    }
    for _ in 0..5 {
        lines.push(Line::from(""));
    }
    (lines, normalize_toc(toc))
}
