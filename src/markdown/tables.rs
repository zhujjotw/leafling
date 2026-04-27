use crate::theme::app_theme;
use pulldown_cmark::{Alignment, Event as MdEvent, Tag, TagEnd};
use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
};
use unicode_width::UnicodeWidthChar;

use super::latex;
use super::width::{display_width, expand_tabs};

#[derive(Clone, Copy, Default)]
struct CellInlineStyle {
    bold: bool,
    italic: bool,
    strikethrough: bool,
    link: bool,
}

#[derive(Clone)]
enum CellFragment {
    Text(String, CellInlineStyle),
    Code(String),
    InlineMath(String),
    LinkMarker,
}

impl CellFragment {
    fn rendered_text(&self) -> String {
        match self {
            CellFragment::Text(t, _) | CellFragment::Code(t) => t.clone(),
            CellFragment::InlineMath(t) => latex::to_unicode(t),
            CellFragment::LinkMarker => "⌗".to_string(),
        }
    }

    fn display_width(&self) -> usize {
        let w = display_width(&self.rendered_text());
        match self {
            CellFragment::Text(_, _) | CellFragment::LinkMarker => w,
            _ => w + 2,
        }
    }

    fn is_text(&self) -> bool {
        matches!(self, CellFragment::Text(_, _))
    }
}

pub(super) struct TableBuf {
    pub(super) alignments: Vec<Alignment>,
    rows: Vec<Vec<Vec<CellFragment>>>,
    header_count: usize,
    current_row: Vec<Vec<CellFragment>>,
    current_cell: Vec<CellFragment>,
    pub(super) in_header: bool,
    inline_style: CellInlineStyle,
}

struct TableBorder<'a> {
    left: &'a str,
    fill: &'a str,
    cross: &'a str,
    right: &'a str,
}

pub(super) fn handle_table_event(
    table: &mut Option<TableBuf>,
    ev: &MdEvent<'_>,
    lines: &mut Vec<Line<'static>>,
    render_width: usize,
) -> bool {
    let Some(tb) = table.as_mut() else {
        return false;
    };

    match ev {
        MdEvent::Text(t) => {
            tb.push_text(t.as_ref());
            true
        }
        MdEvent::Code(t) => {
            tb.push_code(t.as_ref());
            true
        }
        MdEvent::InlineMath(t) => {
            tb.push_inline_math(t.as_ref());
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
        MdEvent::Start(Tag::Strong) => {
            tb.inline_style.bold = true;
            true
        }
        MdEvent::End(TagEnd::Strong) => {
            tb.inline_style.bold = false;
            true
        }
        MdEvent::Start(Tag::Emphasis) => {
            tb.inline_style.italic = true;
            true
        }
        MdEvent::End(TagEnd::Emphasis) => {
            tb.inline_style.italic = false;
            true
        }
        MdEvent::Start(Tag::Strikethrough) => {
            tb.inline_style.strikethrough = true;
            true
        }
        MdEvent::End(TagEnd::Strikethrough) => {
            tb.inline_style.strikethrough = false;
            true
        }
        MdEvent::Start(Tag::Link { .. }) => {
            tb.inline_style.link = true;
            tb.push_link_marker();
            true
        }
        MdEvent::End(TagEnd::Link) => {
            tb.inline_style.link = false;
            true
        }
        MdEvent::End(TagEnd::Table) => {
            let rendered = tb.render(render_width);
            lines.extend(rendered);
            *table = None;
            true
        }
        _ => true,
    }
}

pub(super) fn start_table(table: &mut Option<TableBuf>, aligns: &[Alignment]) {
    *table = Some(TableBuf::new(aligns.to_vec()));
}

impl TableBuf {
    fn new(alignments: Vec<Alignment>) -> Self {
        Self {
            alignments,
            rows: vec![],
            header_count: 0,
            current_row: vec![],
            current_cell: vec![],
            in_header: false,
            inline_style: CellInlineStyle::default(),
        }
    }
    fn push_text(&mut self, t: &str) {
        self.current_cell
            .push(CellFragment::Text(t.to_string(), self.inline_style));
    }
    fn push_link_marker(&mut self) {
        self.current_cell.push(CellFragment::LinkMarker);
    }
    fn push_code(&mut self, t: &str) {
        self.current_cell.push(CellFragment::Code(t.to_string()));
    }
    fn push_inline_math(&mut self, t: &str) {
        self.current_cell
            .push(CellFragment::InlineMath(t.to_string()));
    }
    fn end_cell(&mut self) {
        let mut frags = std::mem::take(&mut self.current_cell);
        if let Some(CellFragment::Text(t, _)) = frags.first_mut() {
            *t = t.trim_start().to_string();
        }
        if let Some(CellFragment::Text(t, _)) = frags.last_mut() {
            *t = t.trim_end().to_string();
        }
        self.current_row.push(frags);
        self.inline_style = CellInlineStyle::default();
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
                    col_widths[ci] = col_widths[ci].max(fragments_display_width(cell));
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

        let empty_cell: Vec<CellFragment> = vec![];
        for (ri, row) in self.rows.iter().enumerate() {
            let is_hdr = ri < self.header_count;
            let wrapped_cells: Vec<Vec<Vec<CellFragment>>> = col_widths
                .iter()
                .copied()
                .enumerate()
                .take(col_count)
                .map(|(ci, width)| wrap_table_cell(row.get(ci).unwrap_or(&empty_cell), width))
                .collect();
            let row_height = wrapped_cells
                .iter()
                .map(|lines| lines.len())
                .max()
                .unwrap_or(1);

            for line_idx in 0..row_height {
                let mut spans = vec![Span::raw(ind), Span::styled("│", border)];
                for (ci, width) in col_widths.iter().copied().enumerate().take(col_count) {
                    let frags = wrapped_cells[ci].get(line_idx).unwrap_or(&empty_cell);
                    let align = self.alignments.get(ci).copied().unwrap_or(Alignment::None);
                    let base_style = if is_hdr { header } else { cell };
                    let cell_spans = align_cell(frags, width, align, base_style, is_hdr, theme);
                    spans.push(Span::raw(" "));
                    spans.extend(cell_spans);
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

fn fragments_display_width(frags: &[CellFragment]) -> usize {
    frags.iter().map(|f| f.display_width()).sum()
}

fn min_table_cell_width(frags: &[CellFragment]) -> usize {
    let mut max_width = 4usize;
    for frag in frags {
        let w = if frag.is_text() {
            frag.rendered_text()
                .split_whitespace()
                .map(display_width)
                .max()
                .unwrap_or(0)
                .min(12)
        } else {
            frag.display_width()
        };
        max_width = max_width.max(w);
    }
    max_width
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

fn wrap_table_cell(frags: &[CellFragment], width: usize) -> Vec<Vec<CellFragment>> {
    if width == 0 {
        return vec![vec![]];
    }
    if frags.is_empty() {
        return vec![vec![]];
    }

    let mut lines: Vec<Vec<CellFragment>> = Vec::new();
    let mut current_line: Vec<CellFragment> = Vec::new();
    let mut current_width = 0usize;
    let mut glue = false;

    for frag in frags {
        match frag {
            CellFragment::Text(t, style) => {
                let expanded = expand_tabs(t, 0);
                let style = *style;
                for word in expanded.split_whitespace() {
                    let word_width = display_width(word);

                    if word_width > width {
                        if !current_line.is_empty() || current_width > 0 {
                            lines.push(std::mem::take(&mut current_line));
                            current_width = 0;
                        }
                        glue = false;
                        let mut chunk = String::new();
                        let mut chunk_width = 0usize;
                        for ch in word.chars() {
                            let ch_width = UnicodeWidthChar::width(ch).unwrap_or(0);
                            if chunk_width + ch_width > width && !chunk.is_empty() {
                                lines.push(vec![CellFragment::Text(
                                    std::mem::take(&mut chunk),
                                    style,
                                )]);
                                chunk_width = 0;
                            }
                            chunk.push(ch);
                            chunk_width += ch_width;
                        }
                        if !chunk.is_empty() {
                            current_line.push(CellFragment::Text(chunk, style));
                            current_width = chunk_width;
                        }
                        continue;
                    }

                    let needs_sep = current_width > 0 && !glue;
                    glue = false;
                    let sep = if needs_sep { 1 } else { 0 };
                    if current_width + sep + word_width > width && current_width > 0 {
                        lines.push(std::mem::take(&mut current_line));
                        current_width = 0;
                    } else if needs_sep {
                        current_line.push(CellFragment::Text(" ".to_string(), style));
                        current_width += 1;
                    }
                    current_line.push(CellFragment::Text(word.to_string(), style));
                    current_width += word_width;
                }
            }
            CellFragment::LinkMarker => {
                if current_width + 1 > width && current_width > 0 {
                    lines.push(std::mem::take(&mut current_line));
                    current_width = 0;
                }
                if current_width > 0 {
                    current_line.push(CellFragment::Text(
                        " ".to_string(),
                        CellInlineStyle::default(),
                    ));
                    current_width += 1;
                }
                current_line.push(frag.clone());
                current_width += 1;
                glue = true;
            }
            CellFragment::Code(_) | CellFragment::InlineMath(_) => {
                let frag_width = frag.display_width();
                let sep = if current_width == 0 { 0 } else { 1 };
                if current_width + sep + frag_width > width && current_width > 0 {
                    lines.push(std::mem::take(&mut current_line));
                    current_width = 0;
                }
                if current_width > 0 {
                    current_line.push(CellFragment::Text(
                        " ".to_string(),
                        CellInlineStyle::default(),
                    ));
                    current_width += 1;
                }
                current_line.push(frag.clone());
                current_width += frag_width;
            }
        }
    }

    if !current_line.is_empty() {
        lines.push(current_line);
    }
    if lines.is_empty() {
        lines.push(vec![]);
    }
    lines
}

fn align_cell(
    frags: &[CellFragment],
    width: usize,
    align: Alignment,
    base_style: Style,
    is_header: bool,
    theme: &crate::theme::MarkdownTheme,
) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    let mut content_width = 0usize;

    for frag in frags {
        match frag {
            CellFragment::Text(t, inline) => {
                let expanded = expand_tabs(t, 0);
                content_width += display_width(&expanded);
                let mut style = base_style;
                if inline.bold {
                    style = style.add_modifier(Modifier::BOLD);
                    if !is_header {
                        style = style.fg(theme.strong_text);
                    }
                }
                if inline.italic {
                    style = style.add_modifier(Modifier::ITALIC);
                }
                if inline.strikethrough {
                    style = style.add_modifier(Modifier::CROSSED_OUT);
                }
                if inline.link {
                    style = style.fg(theme.link_text).add_modifier(Modifier::UNDERLINED);
                }
                spans.push(Span::styled(expanded, style));
            }
            CellFragment::LinkMarker => {
                spans.push(Span::styled("⌗", Style::default().fg(theme.link_icon)));
                content_width += 1;
            }
            CellFragment::Code(_) | CellFragment::InlineMath(_) => {
                let styled = format!(" {} ", frag.rendered_text());
                content_width += display_width(&styled);
                let (fg, bg) = match frag {
                    CellFragment::Code(_) => (theme.inline_code_fg, theme.inline_code_bg),
                    _ => (theme.latex_inline_fg, theme.latex_inline_bg),
                };
                spans.push(Span::styled(styled, Style::default().fg(fg).bg(bg)));
            }
        }
    }

    if content_width < width {
        let pad = width - content_width;
        match align {
            Alignment::Right => {
                spans.insert(0, Span::styled(" ".repeat(pad), base_style));
            }
            Alignment::Center => {
                let l = pad / 2;
                spans.insert(0, Span::styled(" ".repeat(l), base_style));
                spans.push(Span::styled(" ".repeat(pad - l), base_style));
            }
            _ => {
                spans.push(Span::styled(" ".repeat(pad), base_style));
            }
        }
    }

    spans
}
