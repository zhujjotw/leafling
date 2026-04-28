mod frontmatter;
mod latex;
mod mermaid;
mod tables;
pub(crate) mod toc;
pub(crate) mod width;
mod wrapping;

use tables::{handle_table_event, start_table, TableBuf};
pub(crate) use width::{
    build_searchable_lines, display_width, line_plain_text, truncate_display_width,
};

use crate::theme::{app_theme, MarkdownTheme};
use pulldown_cmark::{CodeBlockKind, Event as MdEvent, HeadingLevel, Options, Parser, Tag, TagEnd};
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
use toc::{normalize_toc, TocEntry};
use unicode_width::UnicodeWidthStr;
use width::expand_tabs;
use wrapping::{push_wrapped_code_lines, push_wrapped_prefixed_lines};

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

struct CodeBlockRenderContext<'a> {
    ss: &'a SyntaxSet,
    theme: &'a Theme,
    render_width: usize,
    theme_colors: &'a MarkdownTheme,
    blockquote_depth: usize,
    list_stack: &'a [ListKind],
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

struct CodeLine {
    content_spans: Vec<Span<'static>>,
}

fn highlight_code(
    code: &str,
    lang: &str,
    ss: &SyntaxSet,
    theme: &Theme,
    render_width: usize,
) -> (Vec<CodeLine>, usize, usize) {
    let syntax = resolve_syntax(lang, ss);
    let mut hl = HighlightLines::new(syntax, theme);

    let mut raw: Vec<(Vec<Span<'static>>, usize)> = Vec::new();
    for line_str in LinesWithEndings::from(code) {
        let regions = hl.highlight_line(line_str, ss).unwrap_or_default();
        let mut spans = Vec::new();
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
    let total_lines = raw.len();
    let digit_width = total_lines.max(1).to_string().len();
    let gutter_width = digit_width + 2;
    let max_text = raw.iter().map(|(_, w)| *w).max().unwrap_or(0);
    let max_inner_width = render_width
        .saturating_sub(4)
        .max(UnicodeWidthStr::width(label) + 3);
    let min_inner = (UnicodeWidthStr::width(label) + 3)
        .max(44)
        .min(max_inner_width);
    let inner_width = (max_text + 2 + gutter_width)
        .max(min_inner)
        .min(max_inner_width);

    let mut out = Vec::new();
    for (spans, _text_width) in raw {
        out.push(CodeLine {
            content_spans: spans,
        });
    }
    (out, inner_width, digit_width)
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
        4 => theme.heading_4,
        _ => theme.heading_other,
    };
    let modifier = match level {
        1..=5 => Modifier::BOLD,
        _ => Modifier::ITALIC,
    };
    let heading_style = Style::default().fg(color).add_modifier(modifier);
    let title: String = spans.iter().map(|s| s.content.as_ref()).collect();
    toc.push(TocEntry {
        level,
        title: title.clone(),
        line: lines.len(),
    });
    let styled_spans: Vec<Span<'static>> = spans
        .drain(..)
        .map(|span| {
            let mut style = heading_style;
            if span.style.bg.is_some() {
                style.fg = span.style.fg;
                style.bg = span.style.bg;
                style.sub_modifier = modifier;
            } else if span.style.fg == Some(theme.link_text)
                || span.style.fg == Some(theme.link_icon)
            {
                style.fg = span.style.fg;
            }
            Span::styled(span.content, style)
        })
        .collect();
    lines.push(Line::from(styled_spans));

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
    let (code_lines, inner_width, digit_width) =
        highlight_code(code_buf, code_lang, ctx.ss, ctx.theme, available_width);
    let gutter_width = digit_width + 2;
    let gutter_style = Style::default().fg(ctx.theme_colors.code_gutter);
    let content_width = inner_width.saturating_sub(gutter_width + 1);

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

    for (i, code_line) in code_lines.into_iter().enumerate() {
        let line_num = i + 1;
        let num_gutter = Span::styled(format!("│{:>w$}│", line_num, w = digit_width), gutter_style);
        let blank_gutter = Span::styled(format!("│{:>w$}│", "", w = digit_width), gutter_style);

        let mut first_prefix = prefix.clone();
        first_prefix.push(num_gutter);

        let mut cont_prefix = prefix.clone();
        cont_prefix.push(blank_gutter);

        push_wrapped_code_lines(
            lines,
            code_line.content_spans,
            first_prefix,
            cont_prefix,
            gutter_style,
            content_width,
        );
    }

    let mut footer = prefix;
    footer.push(Span::styled(
        format!(
            "└{}┴{}┘",
            "─".repeat(gutter_width - 2),
            "─".repeat(inner_width.saturating_sub(gutter_width - 1))
        ),
        Style::default().fg(ctx.theme_colors.code_frame),
    ));
    lines.push(Line::from(footer));
    lines.push(Line::from(""));
    code_lang.clear();
    code_buf.clear();
}

struct SpecialBlockCtx<'a, F: Fn(&str) -> Vec<Span<'static>>> {
    label: &'a str,
    content_lines: &'a [&'a str],
    show_line_numbers: bool,
    center: bool,
    make_spans: F,
}

fn push_special_block_lines<F: Fn(&str) -> Vec<Span<'static>>>(
    lines: &mut Vec<Line<'static>>,
    render_width: usize,
    theme: &MarkdownTheme,
    blockquote_depth: usize,
    list_stack: &[ListKind],
    item_stack: &mut [ItemState],
    ctx: SpecialBlockCtx<'_, F>,
) {
    let label = ctx.label;
    let content_lines = ctx.content_lines;
    let show_line_numbers = ctx.show_line_numbers;
    let center = ctx.center;
    let prefix = if !item_stack.is_empty() {
        list_item_prefix(blockquote_depth > 0, list_stack, item_stack)
    } else if blockquote_depth > 0 {
        block_prefix(true)
    } else {
        Vec::new()
    };
    let prefix_width: usize = prefix
        .iter()
        .map(|span| display_width(span.content.as_ref()))
        .sum();
    let available_width = render_width.saturating_sub(prefix_width);
    let frame_style = Style::default().fg(theme.code_frame);
    let label_style = Style::default().fg(theme.code_label);
    let gutter_style = Style::default().fg(theme.code_gutter);

    let total_lines = content_lines.len().max(1);
    let (digit_width, gutter_width) = if show_line_numbers {
        let dw = total_lines.to_string().len();
        (dw, dw + 2)
    } else {
        (0, 1)
    };

    let max_text = content_lines
        .iter()
        .map(|l| display_width(l))
        .max()
        .unwrap_or(0);
    let max_inner_width = available_width
        .saturating_sub(2)
        .max(UnicodeWidthStr::width(label) + 3);
    let min_inner = (UnicodeWidthStr::width(label) + 3)
        .max(44)
        .min(max_inner_width);
    let inner_width = (max_text + 2 + gutter_width)
        .max(min_inner)
        .min(max_inner_width);
    let content_width = inner_width.saturating_sub(gutter_width + 1);

    let header_width = UnicodeWidthStr::width(label) + 3;
    let top_bar = "─".repeat(inner_width.saturating_sub(header_width));
    let mut header = prefix.clone();
    header.extend([
        Span::styled("┌─ ".to_string(), frame_style),
        Span::styled(format!("{label} "), label_style),
        Span::styled(format!("{top_bar}┐"), frame_style),
    ]);
    lines.push(Line::from(header));

    let center_pad = if center {
        " ".repeat(content_width.saturating_sub(max_text) / 2)
    } else {
        String::new()
    };
    let border_only = if !show_line_numbers {
        Some(Span::styled("│".to_string(), gutter_style))
    } else {
        None
    };

    for (i, content_line) in content_lines.iter().enumerate() {
        let mut content_spans = (ctx.make_spans)(content_line);
        if !center_pad.is_empty() {
            content_spans.insert(0, Span::raw(center_pad.clone()));
        }

        let mut first_prefix = prefix.clone();
        let mut cont_prefix = prefix.clone();
        if let Some(ref border) = border_only {
            first_prefix.push(border.clone());
            cont_prefix.push(border.clone());
        } else {
            let line_num = i + 1;
            first_prefix.push(Span::styled(
                format!("│{:>w$}│", line_num, w = digit_width),
                gutter_style,
            ));
            cont_prefix.push(Span::styled(
                format!("│{:>w$}│", "", w = digit_width),
                gutter_style,
            ));
        }

        push_wrapped_code_lines(
            lines,
            content_spans,
            first_prefix,
            cont_prefix,
            gutter_style,
            content_width,
        );
    }

    let mut footer = prefix;
    if show_line_numbers {
        footer.push(Span::styled(
            format!(
                "└{}┴{}┘",
                "─".repeat(gutter_width - 2),
                "─".repeat(inner_width.saturating_sub(gutter_width - 1))
            ),
            frame_style,
        ));
    } else {
        footer.push(Span::styled(
            format!("└{}┘", "─".repeat(inner_width)),
            frame_style,
        ));
    }
    lines.push(Line::from(footer));
    lines.push(Line::from(""));
}

fn push_latex_block_lines(
    lines: &mut Vec<Line<'static>>,
    content: &str,
    render_width: usize,
    theme: &MarkdownTheme,
    blockquote_depth: usize,
    list_stack: &[ListKind],
    item_stack: &mut [ItemState],
) {
    let rendered = latex::to_unicode(content);
    let content_lines: Vec<&str> = rendered.lines().collect();
    let content_style = Style::default().fg(theme.latex_block_fg);
    push_special_block_lines(
        lines,
        render_width,
        theme,
        blockquote_depth,
        list_stack,
        item_stack,
        SpecialBlockCtx {
            label: "latex",
            content_lines: &content_lines,
            show_line_numbers: true,
            center: false,
            make_spans: |line| vec![Span::styled(line.to_string(), content_style)],
        },
    );
}

fn push_mermaid_block_lines(
    lines: &mut Vec<Line<'static>>,
    content: &str,
    render_width: usize,
    theme: &MarkdownTheme,
    blockquote_depth: usize,
    list_stack: &[ListKind],
    item_stack: &mut [ItemState],
) {
    let rendered = mermaid::render(content);
    let use_rendered = rendered.is_some();
    let content_lines: Vec<&str> = if let Some(ref r) = rendered {
        r.lines().collect()
    } else {
        content.lines().collect()
    };
    let content_style = Style::default().fg(theme.mermaid_block_fg);
    push_special_block_lines(
        lines,
        render_width,
        theme,
        blockquote_depth,
        list_stack,
        item_stack,
        SpecialBlockCtx {
            label: "mermaid",
            content_lines: &content_lines,
            show_line_numbers: !use_rendered,
            center: use_rendered,
            make_spans: |line| {
                if use_rendered {
                    vec![Span::styled(line.to_string(), content_style)]
                } else {
                    mermaid::colorize_line(line, theme)
                }
            },
        },
    );
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
        Style::default()
            .fg(theme.link_text)
            .add_modifier(Modifier::UNDERLINED)
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

fn push_inline_latex_span(spans: &mut Vec<Span<'static>>, text: &str, theme: &MarkdownTheme) {
    let rendered = latex::to_unicode(text);
    spans.push(Span::styled(
        format!(" {rendered} "),
        Style::default()
            .fg(theme.latex_inline_fg)
            .bg(theme.latex_inline_bg),
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
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
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
    let (src, fm_pairs) = frontmatter::extract_frontmatter(src);
    let mut lines: Vec<Line<'static>> = Vec::new();

    if let Some(ref pairs) = fm_pairs {
        let vertical = frontmatter::is_vertical(pairs);
        let tb = TableBuf::from_key_value_pairs(pairs, vertical);
        lines.extend(tb.render(render_width));
    }
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
                if code_lang == "latex" || code_lang == "tex" {
                    push_latex_block_lines(
                        &mut lines,
                        &code_buf,
                        render_width,
                        theme_colors,
                        blockquote_depth,
                        &list_stack,
                        &mut item_stack,
                    );
                    code_buf.clear();
                    code_lang.clear();
                } else if code_lang == "mermaid" {
                    push_mermaid_block_lines(
                        &mut lines,
                        &code_buf,
                        render_width,
                        theme_colors,
                        blockquote_depth,
                        &list_stack,
                        &mut item_stack,
                    );
                    code_buf.clear();
                    code_lang.clear();
                } else {
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
                }
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
            MdEvent::InlineMath(text) => {
                push_inline_latex_span(&mut spans, text.as_ref(), theme_colors);
            }
            MdEvent::DisplayMath(text) => {
                if !spans.is_empty() {
                    lines.push(Line::from(std::mem::take(&mut spans)));
                }
                trim_paragraph_gap_before_block(&mut lines, last_block);
                push_latex_block_lines(
                    &mut lines,
                    text.as_ref(),
                    render_width,
                    theme_colors,
                    blockquote_depth,
                    &list_stack,
                    &mut item_stack,
                );
                last_block = LastBlock::Other;
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
