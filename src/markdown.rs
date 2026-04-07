use crate::{
    app::{normalize_toc, TocEntry},
    theme::app_theme,
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
    const TAB_STOP: usize = 4;

    let mut width = 0;
    for ch in text.chars() {
        if ch == '\t' {
            width += TAB_STOP - (width % TAB_STOP);
        } else {
            width += UnicodeWidthChar::width(ch).unwrap_or(0);
        }
    }
    width
}

fn expand_tabs(text: &str, start_width: usize) -> String {
    const TAB_STOP: usize = 4;

    let mut out = String::new();
    let mut width = start_width;
    for ch in text.chars() {
        if ch == '\t' {
            let spaces = TAB_STOP - (width % TAB_STOP);
            out.push_str(&" ".repeat(spaces));
            width += spaces;
        } else {
            out.push(ch);
            width += UnicodeWidthChar::width(ch).unwrap_or(0);
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

fn resolve_syntax<'a>(lang: &str, ss: &'a SyntaxSet) -> &'a syntect::parsing::SyntaxReference {
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
        vec![Span::styled("▏ ", Style::default().fg(theme.blockquote_marker))]
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
    if body_spans.is_empty() {
        return;
    }

    let prefix = block_prefix(true);
    let prefix_width: usize = prefix.iter().map(|span| display_width(span.content.as_ref())).sum();
    let max_width = render_width.saturating_sub(prefix_width).max(8);

    let mut current_prefix = prefix.clone();
    let mut current_width = 0usize;
    let mut body_started = false;

    let push_current = |lines: &mut Vec<Line<'static>>,
                        current_prefix: &mut Vec<Span<'static>>,
                        body_started: &mut bool,
                        current_width: &mut usize| {
        if *body_started {
            lines.push(Line::from(std::mem::take(current_prefix)));
            *current_prefix = prefix.clone();
            *body_started = false;
            *current_width = 0;
        }
    };

    for span in body_spans.drain(..) {
        let style = span.style;
        let mut token = String::new();
        let mut token_is_space = false;

        let flush_token = |token: &mut String,
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
                if *body_started && *current_width + token_width <= max_width {
                    current_prefix.push(Span::styled(std::mem::take(token), style));
                    *current_width += token_width;
                } else {
                    token.clear();
                }
                return;
            }

            if *body_started && *current_width + token_width > max_width {
                push_current(lines, current_prefix, body_started, current_width);
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
                    push_current(lines, current_prefix, body_started, current_width);
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
                .map(|(ci, width)| wrap_table_cell(row.get(ci).map(|s| s.as_str()).unwrap_or(""), width))
                .collect();
            let row_height = wrapped_cells.iter().map(|lines| lines.len()).max().unwrap_or(1);

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
            .max_by_key(|(_, width)| **width) else {
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
    let mut in_strong = false;
    let mut in_em = false;
    let mut in_strike = false;
    let mut in_link = false;
    let mut list_stack: Vec<ListKind> = Vec::new();
    let mut item_stack: Vec<ItemState> = Vec::new();
    let mut table: Option<TableBuf> = None;
    let mut last_block = LastBlock::Other;

    macro_rules! flush {
        ($prefix:expr) => {{
            if !spans.is_empty() {
                let mut all: Vec<Span<'static>> = $prefix;
                all.append(&mut spans);
                lines.push(Line::from(all));
            }
        }};
    }

    for ev in Parser::new_ext(src, Options::all()) {
        if let Some(ref mut tb) = table {
            match &ev {
                MdEvent::Text(t) => {
                    tb.push_text(t.as_ref());
                    continue;
                }
                MdEvent::Code(t) => {
                    tb.push_text(t.as_ref());
                    continue;
                }
                MdEvent::Start(Tag::TableCell) | MdEvent::End(TagEnd::TableCell) => {
                    if matches!(&ev, MdEvent::End(_)) {
                        tb.end_cell();
                    }
                    continue;
                }
                MdEvent::Start(Tag::TableRow) | MdEvent::End(TagEnd::TableRow) => {
                    if matches!(&ev, MdEvent::End(_)) {
                        tb.end_row();
                    }
                    continue;
                }
                MdEvent::Start(Tag::TableHead) | MdEvent::End(TagEnd::TableHead) => {
                    if matches!(&ev, MdEvent::End(_)) {
                        tb.end_header();
                    } else {
                        tb.in_header = true;
                    }
                    continue;
                }
                MdEvent::Start(Tag::Strong)
                | MdEvent::End(TagEnd::Strong)
                | MdEvent::Start(Tag::Emphasis)
                | MdEvent::End(TagEnd::Emphasis)
                | MdEvent::Start(Tag::Link { .. })
                | MdEvent::End(TagEnd::Link) => {
                    continue;
                }
                MdEvent::End(TagEnd::Table) => {
                    lines.extend(tb.render(render_width));
                    table = None;
                    continue;
                }
                _ => continue,
            }
        }

        match ev {
            MdEvent::Start(Tag::Table(aligns)) => {
                table = Some(TableBuf::new(aligns.clone()));
            }
            MdEvent::Start(Tag::Heading { level, .. }) => {
                in_heading = Some(match level {
                    HeadingLevel::H1 => 1,
                    HeadingLevel::H2 => 2,
                    HeadingLevel::H3 => 3,
                    _ => 4,
                });
            }
            MdEvent::End(TagEnd::Heading(_)) => {
                let lvl = in_heading.unwrap_or(1);
                let color: Color = match lvl {
                    1 => theme_colors.heading_1,
                    2 => theme_colors.heading_2,
                    3 => theme_colors.heading_3,
                    _ => theme_colors.heading_other,
                };
                let style = Style::default().fg(color).add_modifier(match lvl {
                    1 => Modifier::BOLD,
                    2 => Modifier::BOLD,
                    3 => Modifier::BOLD,
                    _ => Modifier::empty(),
                });
                let title: String = spans.iter().map(|s| s.content.as_ref()).collect();
                let rendered_title = if lvl == 3 {
                    format!("{title} ")
                } else {
                    title.clone()
                };
                toc.push(TocEntry {
                    level: lvl,
                    title: title.clone(),
                    line: lines.len(),
                });
                let all = vec![Span::styled(rendered_title, style)];
                spans.clear();
                lines.push(Line::from(all));
                if lvl == 1 {
                    lines.push(Line::from(Span::styled(
                        "═".repeat(display_width(&title).min(rule_width(render_width, 0))),
                        Style::default().fg(theme_colors.heading_underline),
                    )));
                }
                if lvl == 2 {
                    lines.push(Line::from(Span::styled(
                        "─".repeat(display_width(&title).min(rule_width(render_width, 0))),
                        Style::default().fg(theme_colors.heading_underline),
                    )));
                }
                last_block = LastBlock::Other;
                in_heading = None;
            }
            MdEvent::Start(Tag::Paragraph) => {}
            MdEvent::End(TagEnd::Paragraph) => {
                if blockquote_depth > 0 && item_stack.is_empty() {
                    push_wrapped_blockquote_lines(&mut lines, &mut spans, render_width);
                } else {
                    let prefix = if item_stack.is_empty() {
                        block_prefix(false)
                    } else {
                        list_item_prefix(false, &list_stack, &mut item_stack)
                    };
                    flush!(prefix);
                }
                lines.push(Line::from(""));
                last_block = LastBlock::Paragraph;
            }
            MdEvent::Start(Tag::CodeBlock(kind)) => {
                if last_block == LastBlock::Paragraph
                    && item_stack.is_empty()
                    && lines.last().is_some_and(|line| line_plain_text(line).is_empty())
                {
                    lines.pop();
                }
                in_code = true;
                code_buf.clear();
                code_lang = match kind {
                    CodeBlockKind::Fenced(l) => l.to_string(),
                    CodeBlockKind::Indented => String::new(),
                };
                last_block = LastBlock::Other;
            }
            MdEvent::End(TagEnd::CodeBlock) => {
                in_code = false;
                let ld = if code_lang.is_empty() {
                    "text".to_string()
                } else {
                    code_lang.clone()
                };
                let (code_lines, inner_width) =
                    highlight_code(&code_buf, &code_lang, ss, theme, render_width);
                let header_width = UnicodeWidthStr::width(ld.as_str()) + 3;
                let top_bar = "─".repeat(inner_width.saturating_sub(header_width));
                lines.push(Line::from(vec![
                    Span::styled(
                        "┌─ ".to_string(),
                        Style::default().fg(theme_colors.code_frame),
                    ),
                    Span::styled(
                        format!("{} ", ld),
                        Style::default().fg(theme_colors.code_label),
                    ),
                    Span::styled(
                        format!("{}┐", top_bar),
                        Style::default().fg(theme_colors.code_frame),
                    ),
                ]));
                lines.extend(code_lines);
                lines.push(Line::from(Span::styled(
                    format!("└{}┘", "─".repeat(inner_width)),
                    Style::default().fg(theme_colors.code_frame),
                )));
                lines.push(Line::from(""));
                code_lang.clear();
                code_buf.clear();
                last_block = LastBlock::Other;
            }
            MdEvent::Code(text) => {
                spans.push(Span::styled(
                    format!(" {} ", text.as_ref()),
                    Style::default()
                        .fg(theme_colors.inline_code_fg)
                        .bg(theme_colors.inline_code_bg),
                ));
            }
            MdEvent::Start(Tag::BlockQuote(_)) => {
                blockquote_depth += 1;
            }
            MdEvent::End(TagEnd::BlockQuote(_)) => {
                flush!(vec![Span::styled(
                    "▏ ",
                    Style::default().fg(theme_colors.blockquote_marker)
                )]);
                blockquote_depth = blockquote_depth.saturating_sub(1);
                lines.push(Line::from(""));
                last_block = LastBlock::Other;
            }
            MdEvent::Start(Tag::List(start)) => {
                if last_block == LastBlock::Paragraph
                    && item_stack.is_empty()
                    && lines.last().is_some_and(|line| line_plain_text(line).is_empty())
                {
                    lines.pop();
                }
                list_stack.push(match start {
                    Some(n) => ListKind::Ordered(n),
                    None => ListKind::Unordered,
                });
                last_block = LastBlock::Other;
            }
            MdEvent::End(TagEnd::List(_)) => {
                list_stack.pop();
                if list_stack.is_empty() {
                    lines.push(Line::from(""));
                }
                last_block = LastBlock::Other;
            }
            MdEvent::Start(Tag::Item) => {
                item_stack.push(ItemState {
                    marker_emitted: false,
                    continuation_indent: 0,
                });
            }
            MdEvent::End(TagEnd::Item) => {
                if !spans.is_empty() {
                    let mut all =
                        list_item_prefix(blockquote_depth > 0, &list_stack, &mut item_stack);
                    all.append(&mut spans);
                    lines.push(Line::from(all));
                }
                item_stack.pop();
                if let Some(ListKind::Ordered(next)) = list_stack.last_mut() {
                    *next += 1;
                }
            }
            MdEvent::Rule => {
                lines.push(Line::from(Span::styled(
                    "─".repeat(rule_width(render_width, 0)),
                    Style::default().fg(theme_colors.rule),
                )));
                lines.push(Line::from(""));
                last_block = LastBlock::Other;
            }
            MdEvent::Start(Tag::Strong) => in_strong = true,
            MdEvent::End(TagEnd::Strong) => in_strong = false,
            MdEvent::Start(Tag::Emphasis) => in_em = true,
            MdEvent::End(TagEnd::Emphasis) => in_em = false,
            MdEvent::Start(Tag::Strikethrough) => in_strike = true,
            MdEvent::End(TagEnd::Strikethrough) => in_strike = false,
            MdEvent::Start(Tag::Link { .. }) => {
                in_link = true;
                spans.push(Span::styled(
                    "⌗",
                    Style::default().fg(theme_colors.link_icon),
                ));
            }
            MdEvent::End(TagEnd::Link) => in_link = false,
            MdEvent::Text(text) => {
                if in_code {
                    code_buf.push_str(text.as_ref());
                } else {
                    let content = text.to_string();
                    let mut style = if blockquote_depth > 0 {
                        Style::default()
                            .fg(theme_colors.blockquote_text)
                            .add_modifier(Modifier::ITALIC)
                    } else if in_link {
                        Style::default().fg(theme_colors.link_text)
                    } else {
                        Style::default().fg(theme_colors.text)
                    };
                    if in_strong {
                        style = style
                            .fg(theme_colors.strong_text)
                            .add_modifier(Modifier::BOLD);
                    }
                    if in_em {
                        style = style.add_modifier(Modifier::ITALIC);
                    }
                    if in_strike {
                        style = style.add_modifier(Modifier::CROSSED_OUT);
                    }
                    spans.push(Span::styled(content, style));
                }
            }
            MdEvent::SoftBreak | MdEvent::HardBreak => {
                if !in_code {
                    if blockquote_depth > 0 && item_stack.is_empty() {
                        push_wrapped_blockquote_lines(&mut lines, &mut spans, render_width);
                    } else {
                        let prefix = if item_stack.is_empty() {
                            block_prefix(false)
                        } else {
                            list_item_prefix(false, &list_stack, &mut item_stack)
                        };
                        flush!(prefix);
                    }
                }
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
