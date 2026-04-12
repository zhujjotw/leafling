use crate::theme::app_theme;
use pulldown_cmark::{Alignment, Event as MdEvent, Tag, TagEnd};
use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
};
use unicode_width::UnicodeWidthChar;

use super::width::{display_width, expand_tabs};

pub(super) struct TableBuf {
    pub(super) alignments: Vec<Alignment>,
    rows: Vec<Vec<String>>,
    header_count: usize,
    current_row: Vec<String>,
    current_cell: String,
    pub(super) in_header: bool,
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
