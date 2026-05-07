use super::{rendered_non_empty_lines, test_assets, test_md_theme};
use crate::markdown::{highlight_line, parse_markdown, parse_markdown_with_width, resolve_syntax};
use crate::theme::app_theme;
use crate::*;
use ratatui::{
    style::Style,
    text::{Line, Span},
};
use syntect::parsing::SyntaxSet;

#[test]
fn h1_headings_render_double_rule_without_bottom_spacing() {
    let (ss, theme) = test_assets();
    let (lines, _, _) = parse_markdown("# 東京\n", &ss, &theme, &test_md_theme());
    let rendered = rendered_non_empty_lines(&lines);

    assert_eq!(rendered[0], "東京");
    assert_eq!(rendered[1], "═".repeat(display_width("東京")));
}

#[test]
fn loose_list_items_keep_their_markers() {
    let (ss, theme) = test_assets();
    let (lines, _, _) = parse_markdown("- first\n\n- second\n", &ss, &theme, &test_md_theme());
    let rendered: Vec<String> = lines.iter().map(line_plain_text).collect();

    assert!(rendered.iter().any(|line| line.contains("• first")));
    assert!(rendered.iter().any(|line| line.contains("• second")));
}

#[test]
fn ordered_lists_render_numeric_markers() {
    let (ss, theme) = test_assets();
    let (lines, _, _) = parse_markdown("3. third\n4. fourth\n", &ss, &theme, &test_md_theme());
    let rendered: Vec<String> = lines.iter().map(line_plain_text).collect();

    assert!(rendered.iter().any(|line| line.contains("3. third")));
    assert!(rendered.iter().any(|line| line.contains("4. fourth")));
}

#[test]
fn multiline_list_items_keep_marker_only_on_first_line() {
    let (ss, theme) = test_assets();
    let (lines, _, _) = parse_markdown(
        "- first line\n  second line\n",
        &ss,
        &theme,
        &test_md_theme(),
    );
    let rendered: Vec<String> = lines.iter().map(line_plain_text).collect();

    let first = rendered
        .iter()
        .find(|line| line.contains("first line"))
        .unwrap();
    let second = rendered
        .iter()
        .find(|line| line.contains("second line"))
        .unwrap();

    assert!(first.contains("• first line"));
    assert!(!second.contains('•'));
    assert!(second.starts_with("  "));
}

#[test]
fn ordered_lists_preserve_non_default_start_numbers() {
    let (ss, theme) = test_assets();
    let (lines, _, _) = parse_markdown("7. seven\n8. eight\n", &ss, &theme, &test_md_theme());
    let rendered: Vec<String> = lines.iter().map(line_plain_text).collect();

    assert!(rendered.iter().any(|line| line.contains("7. seven")));
    assert!(rendered.iter().any(|line| line.contains("8. eight")));
}

#[test]
fn loose_list_items_render_expected_lines() {
    let (ss, theme) = test_assets();
    let src = "- first loose item\n\n- second loose item after a blank line\n\n- third loose item\n\n  continuation paragraph\n";
    let (lines, _, _) = parse_markdown(src, &ss, &theme, &test_md_theme());
    let rendered = rendered_non_empty_lines(&lines);

    assert_eq!(
        rendered,
        vec![
            "• first loose item",
            "• second loose item after a blank line",
            "• third loose item",
            "  continuation paragraph",
        ]
    );
}

#[test]
fn ordered_loose_lists_render_expected_lines() {
    let (ss, theme) = test_assets();
    let src = "7. seventh item\n\n8. eighth item\n\n   continuation paragraph\n";
    let (lines, _, _) = parse_markdown(src, &ss, &theme, &test_md_theme());
    let rendered = rendered_non_empty_lines(&lines);

    assert_eq!(
        rendered,
        vec![
            "7. seventh item",
            "8. eighth item",
            "   continuation paragraph",
        ]
    );
}

#[test]
fn ordered_lists_render_expected_lines() {
    let (ss, theme) = test_assets();
    let (lines, _, _) = parse_markdown(
        "3. third item\n4. fourth item\n",
        &ss,
        &theme,
        &test_md_theme(),
    );
    let rendered = rendered_non_empty_lines(&lines);

    assert_eq!(rendered, vec!["3. third item", "4. fourth item"]);
}

#[test]
fn paragraph_and_following_list_have_no_blank_gap() {
    let (ss, theme) = test_assets();
    let (lines, _, _) = parse_markdown(
        "Intro paragraph\n\n- first\n- second\n",
        &ss,
        &theme,
        &test_md_theme(),
    );
    let rendered: Vec<String> = lines.iter().map(line_plain_text).collect();
    let intro_idx = rendered
        .iter()
        .position(|line| line == "Intro paragraph")
        .unwrap();

    assert_eq!(rendered[intro_idx + 1], "• first");
}

#[test]
fn wrapped_list_items_align_continuation_under_text() {
    let (ss, theme) = test_assets();
    let src = "- First item with enough text to wrap when the terminal is narrow and show continuation alignment.\n8. Eighth item with enough text to wrap and keep numeric alignment readable.\n";
    let (lines, _, _) = parse_markdown_with_width(src, &ss, &theme, 36, &test_md_theme());
    let rendered = rendered_non_empty_lines(&lines);

    assert!(rendered.iter().any(|line| line.starts_with("• First item")));
    assert!(rendered
        .iter()
        .any(|line| line.starts_with("  ") && line.contains("terminal is narrow")));
    assert!(rendered
        .iter()
        .any(|line| line.starts_with("8. Eighth item")));
    assert!(rendered
        .iter()
        .any(|line| line.starts_with("   ") && !line.starts_with("8. ")));
}

#[test]
fn tight_nested_list_separates_parent_and_children() {
    let (ss, theme) = test_assets();
    let src = "- parent\n  - child 1\n  - child 2\n";
    let (lines, _, _) = parse_markdown(src, &ss, &theme, &test_md_theme());
    let rendered = rendered_non_empty_lines(&lines);

    assert_eq!(rendered, vec!["• parent", "  ◦ child 1", "  ◦ child 2"]);
}

#[test]
fn tight_nested_list_three_levels_uses_correct_markers() {
    let (ss, theme) = test_assets();
    let src = "- level 1\n  - level 2\n    - level 3\n";
    let (lines, _, _) = parse_markdown(src, &ss, &theme, &test_md_theme());
    let rendered = rendered_non_empty_lines(&lines);

    assert_eq!(rendered, vec!["• level 1", "  ◦ level 2", "    ▸ level 3"]);
}

#[test]
fn tight_nested_list_unordered_parent_with_ordered_children() {
    let (ss, theme) = test_assets();
    let src = "- parent\n  1. first\n  2. second\n";
    let (lines, _, _) = parse_markdown(src, &ss, &theme, &test_md_theme());
    let rendered = rendered_non_empty_lines(&lines);

    assert_eq!(rendered, vec!["• parent", "  1. first", "  2. second"]);
}

#[test]
fn tight_nested_list_multiline_parent_with_softbreak() {
    let (ss, theme) = test_assets();
    let src = "- parent line one\n  parent line two\n  - child\n";
    let (lines, _, _) = parse_markdown(src, &ss, &theme, &test_md_theme());
    let rendered = rendered_non_empty_lines(&lines);

    assert!(rendered.iter().any(|line| line == "• parent line one"));
    assert!(rendered
        .iter()
        .any(|line| line.starts_with("  ") && line.contains("parent line two")));
    assert!(rendered.iter().any(|line| line == "  ◦ child"));
}

#[test]
fn paragraph_and_following_code_block_have_no_blank_gap() {
    let (ss, theme) = test_assets();
    let src = "Intro paragraph\n\n```rs\nfn main() {}\n```\n";
    let (lines, _, _) = parse_markdown(src, &ss, &theme, &test_md_theme());
    let rendered: Vec<String> = lines.iter().map(line_plain_text).collect();
    let intro_idx = rendered
        .iter()
        .position(|line| line == "Intro paragraph")
        .unwrap();

    assert!(rendered[intro_idx + 1].starts_with("┌─ rs "));
}

#[test]
fn nested_blockquotes_keep_quote_prefix_after_inner_quote_ends() {
    let (ss, theme) = test_assets();
    let src = "> outer\n> > inner\n> outer again\n";
    let (lines, _, _) = parse_markdown(src, &ss, &theme, &test_md_theme());
    let rendered = rendered_non_empty_lines(&lines);

    assert!(rendered.iter().any(|line| line == "▏ outer"));
    assert!(rendered.iter().any(|line| line == "▏ inner"));
    assert!(rendered.iter().any(|line| line == "▏ outer again"));
}

#[test]
fn long_blockquotes_wrap_into_multiple_prefixed_lines() {
    let (ss, theme) = test_assets();
    let src = "> This is a long blockquote line that should wrap into multiple quoted lines at narrow widths.\n";
    let (lines, _, _) = parse_markdown_with_width(src, &ss, &theme, 28, &test_md_theme());
    let rendered = rendered_non_empty_lines(&lines);
    let quoted: Vec<_> = rendered
        .into_iter()
        .filter(|line| line.starts_with('▏'))
        .collect();

    assert!(quoted.len() >= 2);
    assert!(quoted.iter().all(|line| line.starts_with("▏ ")));
}

#[test]
fn toc_only_includes_first_two_heading_levels() {
    let (ss, theme) = test_assets();
    let (_, toc, _) = parse_markdown(
        "# One\n## Two\n### Three\n#### Four\n",
        &ss,
        &theme,
        &test_md_theme(),
    );

    assert_eq!(toc.len(), 3);
    assert_eq!(toc[0].level, 1);
    assert_eq!(toc[1].level, 2);
    assert_eq!(toc[2].level, 3);
}

#[test]
fn frontmatter_is_ignored_in_preview_and_toc() {
    let (ss, theme) = test_assets();
    let src = "---\ntitle: Demo\nowner: me\n---\n# Visible\nBody\n";
    let (lines, toc, _) = parse_markdown(src, &ss, &theme, &test_md_theme());
    let rendered = rendered_non_empty_lines(&lines);

    assert!(!rendered.iter().any(|line| line.contains("title: Demo")));
    assert!(rendered.iter().any(|line| line.contains("Visible")));
    assert_eq!(toc.len(), 1);
    assert_eq!(toc[0].title, "Visible");
}

#[test]
fn h2_headings_are_underlined_and_compact() {
    let (ss, theme) = test_assets();
    let (lines, _, _) = parse_markdown_with_width(
        "Intro\n\n## Section\nBody\n",
        &ss,
        &theme,
        40,
        &test_md_theme(),
    );
    let rendered = rendered_non_empty_lines(&lines);

    assert!(rendered.iter().any(|line| line.contains("Section")));
    assert!(rendered.iter().any(|line| line.contains("────")));
}

#[test]
fn rules_use_render_width_without_extra_blank_after() {
    let (ss, theme) = test_assets();
    let (lines, _, _) =
        parse_markdown_with_width("Alpha\n\n---\nBeta\n", &ss, &theme, 24, &test_md_theme());
    let rendered = rendered_non_empty_lines(&lines);
    let rule = rendered
        .iter()
        .find(|line| line.trim_start().starts_with('─'))
        .unwrap();

    assert_eq!(display_width(rule.trim_start()), 24);
    let rule_idx = rendered.iter().position(|line| line == rule).unwrap();
    assert_eq!(rendered[rule_idx + 1], "Beta");
}

#[test]
fn toc_hides_single_h1_when_h2_entries_exist() {
    let toc = vec![
        TocEntry {
            level: 1,
            title: "Doc Title".to_string(),
            line: 0,
        },
        TocEntry {
            level: 2,
            title: "Install".to_string(),
            line: 10,
        },
    ];

    assert!(should_hide_single_h1(&toc));
    assert_eq!(toc_display_level(2, true, false), 1);
    assert_eq!(toc_display_level(3, true, false), 2);
}

#[test]
fn toc_keeps_single_h1_when_no_h2_entries_exist() {
    let toc = vec![TocEntry {
        level: 1,
        title: "Doc Title".to_string(),
        line: 0,
    }];

    assert!(!should_hide_single_h1(&toc));
}

#[test]
fn toc_promotes_h2_when_document_has_no_h1() {
    let toc = vec![
        TocEntry {
            level: 2,
            title: "Build & install".to_string(),
            line: 0,
        },
        TocEntry {
            level: 3,
            title: "Android".to_string(),
            line: 4,
        },
    ];

    assert!(should_promote_h2_when_no_h1(&toc));
    assert_eq!(toc_display_level(2, false, true), 1);
    assert_eq!(toc_display_level(3, false, true), 2);
    let normalized = normalize_toc(toc);
    assert_eq!(normalized.len(), 2);
    assert_eq!(normalized[0].level, 2);
    assert_eq!(normalized[1].level, 3);
}

#[test]
fn resolve_syntax_supports_common_language_aliases() {
    let ss = SyntaxSet::load_defaults_newlines();

    assert_eq!(
        resolve_syntax("py", &ss).name,
        resolve_syntax("python", &ss).name
    );
    assert_eq!(
        resolve_syntax("cpp", &ss).name,
        resolve_syntax("c++", &ss).name
    );
    assert_eq!(resolve_syntax("json", &ss).name, "JSON");
    assert_eq!(
        resolve_syntax("ps1", &ss).name,
        resolve_syntax("powershell", &ss).name
    );
}

#[test]
fn narrow_tables_fit_render_width_and_wrap_cells() {
    let (ss, theme) = test_assets();
    let md = "| Column | Description | Value |\n| --- | --- | ---: |\n| Width | Terminal-dependent layout behavior | 80 |\n";
    let (lines, _, _) = parse_markdown_with_width(md, &ss, &theme, 36, &test_md_theme());
    let rendered = rendered_non_empty_lines(&lines);

    assert!(rendered.len() >= 6);
    assert!(rendered.iter().all(|line| display_width(line) <= 36));
}

#[test]
fn wrapped_list_inline_code_keeps_left_padding_in_rendered_line() {
    let (ss, theme) = test_assets();
    let source = "- `leaf --theme ocean README.md` exercises wrapping inside a list item.\n";
    let (lines, _, _) = parse_markdown_with_width(source, &ss, &theme, 22, &test_md_theme());

    let target = lines
        .iter()
        .find(|line| line_plain_text(line).contains("leaf --theme"))
        .expect("expected wrapped inline-code line");

    assert!(
        target
            .spans
            .iter()
            .any(|span| span.style.bg.is_some() && span.content.starts_with(' ')),
        "expected a background-styled span with left padding"
    );
}

#[test]
fn code_block_inside_list_item_is_indented_and_has_no_blank_gap_before() {
    let (ss, theme) = test_assets();
    let md = "To put a code block within a list item, the code block needs\nto be indented *twice* -- 8 spaces or two tabs:\n\n*   A list item with a code block:\n\n        <code goes here>\n";
    let (lines, _, _) = parse_markdown(md, &ss, &theme, &test_md_theme());
    let rendered = rendered_non_empty_lines(&lines);

    let item_idx = rendered
        .iter()
        .position(|line| line.contains("A list item with a code block:"))
        .expect("missing list item line");
    let header_idx = rendered
        .iter()
        .position(|line| line.contains("┌─ text"))
        .expect("missing code block header");
    let code_idx = rendered
        .iter()
        .position(|line| line.contains("<code goes here>"))
        .expect("missing code line");

    assert_eq!(
        header_idx,
        item_idx + 1,
        "expected no blank gap before code block"
    );
    assert!(rendered[header_idx].starts_with("  "));
    assert!(rendered[code_idx].starts_with("  "));
}

#[test]
fn inline_latex_renders_with_latex_style() {
    let (ss, theme) = test_assets();
    let (lines, _, _) = parse_markdown(
        "The formula $x^2 + y^2$ is here.\n",
        &ss,
        &theme,
        &test_md_theme(),
    );

    let latex_line = lines
        .iter()
        .find(|line| line_plain_text(line).contains("x² + y²"))
        .expect("expected a line containing inline latex content");

    let latex_span = latex_line
        .spans
        .iter()
        .find(|span| span.content.contains("x² + y²"))
        .expect("expected a span with latex content");

    assert!(
        latex_span.style.bg.is_some(),
        "inline latex should have a background color"
    );
}

#[test]
fn display_latex_renders_in_framed_block() {
    let (ss, theme) = test_assets();
    let (lines, _, _) = parse_markdown("$$E = mc^2$$\n", &ss, &theme, &test_md_theme());
    let rendered = rendered_non_empty_lines(&lines);

    assert!(
        rendered.iter().any(|line| line.contains("┌─ latex")),
        "expected latex block header"
    );
    assert!(
        rendered.iter().any(|line| line.contains("E = mc²")),
        "expected latex content"
    );
    assert!(
        rendered.iter().any(|line| line.contains("└")),
        "expected latex block footer"
    );
}

#[test]
fn inline_latex_is_searchable() {
    let (ss, theme) = test_assets();
    let (lines, _, _) = parse_markdown(
        "Check $\\alpha + \\beta$ here.\n",
        &ss,
        &theme,
        &test_md_theme(),
    );
    let searchable: Vec<String> = lines.iter().map(line_plain_text).collect();

    assert!(
        searchable.iter().any(|line| line.contains("α + β")),
        "latex content should be searchable"
    );
}

#[test]
fn display_latex_in_blockquote_has_quote_prefix() {
    let (ss, theme) = test_assets();
    let (lines, _, _) = parse_markdown("> $$F = ma$$\n", &ss, &theme, &test_md_theme());
    let rendered = rendered_non_empty_lines(&lines);

    let header = rendered
        .iter()
        .find(|line| line.contains("┌─ latex"))
        .expect("expected latex block header in blockquote");
    assert!(
        header.starts_with('▏'),
        "latex block in blockquote should have quote prefix"
    );
}

#[test]
fn table_inline_code_has_code_style() {
    let (ss, theme) = test_assets();
    let md = "| A |\n|---|\n| `code` |\n";
    let (lines, _, _) = parse_markdown(md, &ss, &theme, &test_md_theme());
    let app_theme = app_theme();
    let theme_colors = &app_theme.markdown;

    let has_code_span = lines.iter().any(|line| {
        line.spans.iter().any(|span| {
            span.style.bg == Some(theme_colors.inline_code_bg) && span.content.contains("code")
        })
    });
    assert!(
        has_code_span,
        "inline code in table cell should have inline_code_bg"
    );
}

#[test]
fn table_inline_code_has_padding() {
    let (ss, theme) = test_assets();
    let md = "| A |\n|---|\n| `x` |\n";
    let (lines, _, _) = parse_markdown(md, &ss, &theme, &test_md_theme());
    let app_theme = app_theme();
    let theme_colors = &app_theme.markdown;

    let has_padded_span = lines.iter().any(|line| {
        line.spans
            .iter()
            .any(|span| span.style.bg == Some(theme_colors.inline_code_bg) && span.content == " x ")
    });
    assert!(
        has_padded_span,
        "inline code in table should be padded with spaces"
    );
}

#[test]
fn table_inline_math_renders_with_latex_style() {
    let (ss, theme) = test_assets();
    let md = "| A |\n|---|\n| $\\alpha$ |\n";
    let (lines, _, _) = parse_markdown(md, &ss, &theme, &test_md_theme());
    let app_theme = app_theme();
    let theme_colors = &app_theme.markdown;

    let has_math_span = lines.iter().any(|line| {
        line.spans.iter().any(|span| {
            span.style.bg == Some(theme_colors.latex_inline_bg) && span.content.contains('α')
        })
    });
    assert!(
        has_math_span,
        "inline math in table cell should have latex_inline_bg and render Unicode"
    );
}

#[test]
fn table_mixed_text_and_code_renders_both_styles() {
    let (ss, theme) = test_assets();
    let md = "| A |\n|---|\n| hello `world` bye |\n";
    let (lines, _, _) = parse_markdown(md, &ss, &theme, &test_md_theme());
    let app_theme = app_theme();
    let theme_colors = &app_theme.markdown;

    let table_line = lines
        .iter()
        .find(|line| line.spans.iter().any(|span| span.content.contains("hello")));
    let line = table_line.expect("should find line with 'hello'");

    let has_text = line.spans.iter().any(|span| {
        span.style.fg == Some(theme_colors.table_cell) && span.content.contains("hello")
    });
    let has_code = line.spans.iter().any(|span| {
        span.style.bg == Some(theme_colors.inline_code_bg) && span.content.contains("world")
    });
    assert!(has_text, "text fragment should use table_cell style");
    assert!(has_code, "code fragment should use inline_code_bg style");
}

#[test]
fn table_without_inline_styles_renders_normally() {
    let (ss, theme) = test_assets();
    let md = "| A | B |\n|---|---|\n| one | two |\n";
    let (lines, _, _) = parse_markdown(md, &ss, &theme, &test_md_theme());
    let rendered = rendered_non_empty_lines(&lines);

    assert!(rendered.iter().any(|line| line.contains("one")));
    assert!(rendered.iter().any(|line| line.contains("two")));
}

#[test]
fn table_inline_code_col_width_includes_padding() {
    let (ss, theme) = test_assets();
    let md = "| A |\n|---|\n| `longcode` |\n";
    let (lines, _, _) = parse_markdown(md, &ss, &theme, &test_md_theme());
    let rendered = rendered_non_empty_lines(&lines);

    let top_border = rendered.iter().find(|l| l.contains('┌')).unwrap();
    let cell_line = rendered.iter().find(|l| l.contains("longcode")).unwrap();
    let top_width = display_width(top_border);
    let cell_width = display_width(cell_line);
    assert_eq!(
        top_width, cell_width,
        "border and cell lines should have same width"
    );
}

#[test]
fn table_code_adjacent_text_no_extra_space() {
    let (ss, theme) = test_assets();
    let md = "| A |\n|---|\n| `code`:text |\n";
    let (lines, _, _) = parse_markdown(md, &ss, &theme, &test_md_theme());
    let rendered = rendered_non_empty_lines(&lines);
    let cell_line = rendered
        .iter()
        .find(|l| l.contains("code") && l.contains(":text"))
        .expect("should find line with code and :text");
    assert!(
        !cell_line.contains("  :text"),
        "no extra space before :text — got: {cell_line}"
    );
}

#[test]
fn table_bold_adjacent_text_no_extra_space() {
    let (ss, theme) = test_assets();
    let md = "| A |\n|---|\n| **bold**:text |\n";
    let (lines, _, _) = parse_markdown(md, &ss, &theme, &test_md_theme());
    let rendered = rendered_non_empty_lines(&lines);
    let cell_line = rendered
        .iter()
        .find(|l| l.contains("bold") && l.contains(":text"))
        .expect("should find line with bold and :text");
    assert!(
        !cell_line.contains(" :text"),
        "no space before :text — got: {cell_line}"
    );
}

#[test]
fn table_apostrophe_no_split() {
    let (ss, theme) = test_assets();
    let md = "| A |\n|---|\n| apos'trophe |\n";
    let (lines, _, _) = parse_markdown(md, &ss, &theme, &test_md_theme());
    let rendered = rendered_non_empty_lines(&lines);
    let cell_line = rendered
        .iter()
        .find(|l| l.contains("apos"))
        .expect("should find line with apos");
    assert!(
        !cell_line.contains(" \u{2019} "),
        "no spaces around smart apostrophe — got: {cell_line}"
    );
}

#[test]
fn mermaid_block_renders_in_framed_block() {
    let (ss, theme) = test_assets();
    let (lines, _, _) = parse_markdown(
        "```mermaid\ngraph TD\n  A --> B\n```\n",
        &ss,
        &theme,
        &test_md_theme(),
    );
    let rendered = rendered_non_empty_lines(&lines);

    assert!(
        rendered.iter().any(|line| line.contains("┌─ mermaid")),
        "expected mermaid block header"
    );
    assert!(
        rendered.iter().any(|line| line.contains('A')),
        "expected node A in rendered content"
    );
    assert!(
        rendered.iter().any(|line| line.contains('B')),
        "expected node B in rendered content"
    );
    assert!(
        rendered.iter().any(|line| line.contains("└")),
        "expected mermaid block footer"
    );
}

#[test]
fn mermaid_block_in_blockquote_has_quote_prefix() {
    let (ss, theme) = test_assets();
    let (lines, _, _) = parse_markdown(
        "> ```mermaid\n> graph LR\n>   X --> Y\n> ```\n",
        &ss,
        &theme,
        &test_md_theme(),
    );
    let rendered = rendered_non_empty_lines(&lines);

    let header = rendered
        .iter()
        .find(|line| line.contains("┌─ mermaid"))
        .expect("expected mermaid block header in blockquote");
    assert!(
        header.starts_with('▏'),
        "mermaid block in blockquote should have quote prefix"
    );
}

#[test]
fn mermaid_content_is_searchable() {
    let (ss, theme) = test_assets();
    let (lines, _, _) = parse_markdown(
        "```mermaid\nsequenceDiagram\n  A->>B: Hello\n```\n",
        &ss,
        &theme,
        &test_md_theme(),
    );
    let searchable = markdown::width::build_searchable_lines(&lines);

    assert!(
        searchable.iter().any(|line| line.contains('A')),
        "mermaid content should be searchable (node A)"
    );
    assert!(
        searchable.iter().any(|line| line.contains("Hello")),
        "mermaid content should be searchable (message Hello)"
    );
}

#[test]
fn mermaid_rendered_block_has_no_gutter() {
    let (ss, theme) = test_assets();
    let (lines, _, _) = parse_markdown(
        "```mermaid\ngraph TD\n  A --> B\n  B --> C\n```\n",
        &ss,
        &theme,
        &test_md_theme(),
    );
    let rendered = rendered_non_empty_lines(&lines);
    let content_lines: Vec<_> = rendered
        .iter()
        .filter(|l| !l.contains('┌') && !l.contains('└'))
        .collect();

    assert!(
        content_lines.iter().all(|line| !line.contains("│1│")),
        "rendered mermaid should not have numbered gutter"
    );
}

#[test]
fn mermaid_fallback_has_numbered_gutter() {
    let (ss, theme) = test_assets();
    let src = "```mermaid\ngantt\n  title Schedule\n  section Dev\n```\n";
    let (lines, _, _) = parse_markdown(src, &ss, &theme, &test_md_theme());
    let rendered = rendered_non_empty_lines(&lines);

    assert!(
        rendered.iter().any(|line| line.contains("│1│")),
        "fallback mermaid should have numbered gutter"
    );
}

#[test]
fn mermaid_pie_renders_bar_chart() {
    let (ss, theme) = test_assets();
    let src = "```mermaid\npie title Languages\n  \"Rust\" : 65\n  \"Go\" : 35\n```\n";
    let (lines, _, _) = parse_markdown(src, &ss, &theme, &test_md_theme());
    let rendered = rendered_non_empty_lines(&lines);

    assert!(
        rendered.iter().any(|line| line.contains("┌─ mermaid")),
        "expected mermaid block header for pie"
    );
    assert!(
        rendered.iter().any(|line| line.contains("Languages")),
        "expected pie chart title"
    );
    assert!(
        rendered.iter().any(|line| line.contains("Rust")),
        "expected Rust label in pie chart"
    );
    assert!(
        rendered.iter().any(|line| line.contains('█')),
        "expected bar characters in pie chart"
    );
    assert!(
        rendered.iter().any(|line| line.contains('%')),
        "expected percentage in pie chart"
    );
}

#[test]
fn mermaid_unsupported_type_falls_back_to_colored_source() {
    let (ss, theme) = test_assets();
    let src = "```mermaid\ngantt\n  title Schedule\n  section Phase 1\n```\n";
    let (lines, _, _) = parse_markdown(src, &ss, &theme, &test_md_theme());
    let rendered = rendered_non_empty_lines(&lines);

    assert!(
        rendered.iter().any(|line| line.contains("┌─ mermaid")),
        "expected mermaid block header"
    );
    assert!(
        rendered.iter().any(|line| line.contains("gantt")),
        "unsupported type should show source (gantt keyword)"
    );
}

#[test]
fn mermaid_empty_block_renders_without_crash() {
    let (ss, theme) = test_assets();
    let (lines, _, _) = parse_markdown("```mermaid\n```\n", &ss, &theme, &test_md_theme());
    let rendered = rendered_non_empty_lines(&lines);

    assert!(
        rendered.iter().any(|line| line.contains("┌─ mermaid")),
        "expected mermaid block header for empty block"
    );
    assert!(
        rendered.iter().any(|line| line.contains("└")),
        "expected mermaid block footer for empty block"
    );
}

#[test]
fn blockquote_bold_link_preserves_link_color() {
    let (ss, theme) = test_assets();
    let src = "> text [**lien bold**](https://rivolink.mg)\n";
    let (lines, _, _) = parse_markdown(src, &ss, &theme, &test_md_theme());
    let app_theme = app_theme();
    let theme_colors = &app_theme.markdown;

    let bq_line = &lines[0];
    let link_span = bq_line.spans.iter().find(|s| s.content.as_ref() == "lien");
    assert!(link_span.is_some(), "should find 'lien' span");
    let span = link_span.unwrap();
    assert_eq!(
        span.style.fg,
        Some(theme_colors.link_text),
        "bold link in blockquote should preserve link_text color"
    );
}

#[test]
fn link_spans_detected_for_all_link_types() {
    let (ss, theme) = test_assets();
    let md = "\
[Simple](https://example.com/simple)

**[Bold link](https://example.com/bold)**

*[Italic link](https://example.com/italic)*

~~[Strike link](https://example.com/strike)~~

[Internal](#section)

### [Heading link](https://example.com/heading)

> [Blockquote link](https://example.com/quote)

[A](https://example.com/a) and [B](https://example.com/b)
";
    let (_, _, link_spans) = parse_markdown(md, &ss, &theme, &test_md_theme());

    let urls: Vec<&str> = link_spans.iter().map(|ls| ls.url.as_str()).collect();

    assert!(
        urls.contains(&"https://example.com/simple"),
        "simple link missing: {urls:?}"
    );
    assert!(
        urls.contains(&"https://example.com/bold"),
        "bold link missing: {urls:?}"
    );
    assert!(
        urls.contains(&"https://example.com/italic"),
        "italic link missing: {urls:?}"
    );
    assert!(
        urls.contains(&"https://example.com/strike"),
        "strikethrough link missing: {urls:?}"
    );
    assert!(
        urls.contains(&"#section"),
        "internal link missing: {urls:?}"
    );
    assert!(
        urls.contains(&"https://example.com/heading"),
        "heading link missing: {urls:?}"
    );
    assert!(
        urls.contains(&"https://example.com/quote"),
        "blockquote link missing: {urls:?}"
    );
    assert!(
        urls.contains(&"https://example.com/a"),
        "multi-link A missing: {urls:?}"
    );
    assert!(
        urls.contains(&"https://example.com/b"),
        "multi-link B missing: {urls:?}"
    );

    for ls in &link_spans {
        assert!(
            ls.end_col > ls.start_col,
            "link {:?} has zero width (start={} end={})",
            ls.url,
            ls.start_col,
            ls.end_col,
        );
    }
}

#[test]
fn link_spans_in_table_are_detected() {
    let (ss, theme) = test_assets();
    let md = "\
| Name | Link |
|------|------|
| Test | [example](https://example.com/table) |
";
    let (_, _, link_spans) = parse_markdown(md, &ss, &theme, &test_md_theme());

    let urls: Vec<&str> = link_spans.iter().map(|ls| ls.url.as_str()).collect();
    assert!(
        urls.contains(&"https://example.com/table"),
        "table link missing: {urls:?}"
    );
}

#[test]
fn highlight_line_single_match() {
    let theme = test_md_theme();
    let line_bg = theme.search_highlight_bg;
    let match_bg = theme.search_match_bg;
    let line = Line::from(vec![Span::raw("hello world")]);
    let result = highlight_line(&line, &theme, "world");
    assert_eq!(result.spans.len(), 2);
    assert_eq!(result.spans[0].content.as_ref(), "hello ");
    assert_eq!(result.spans[0].style.bg, Some(line_bg));
    assert_eq!(result.spans[1].content.as_ref(), "world");
    assert_eq!(result.spans[1].style.bg, Some(match_bg));
    assert!(result.spans[1]
        .style
        .add_modifier
        .contains(ratatui::style::Modifier::BOLD));
}

#[test]
fn highlight_line_multiple_matches() {
    let theme = test_md_theme();
    let match_bg = theme.search_match_bg;
    let line = Line::from(vec![Span::raw("abcabcabc")]);
    let result = highlight_line(&line, &theme, "abc");
    assert_eq!(result.spans.len(), 3);
    for span in &result.spans {
        assert_eq!(span.content.as_ref(), "abc");
        assert_eq!(span.style.bg, Some(match_bg));
        assert!(span
            .style
            .add_modifier
            .contains(ratatui::style::Modifier::BOLD));
    }
}

#[test]
fn highlight_line_case_insensitive() {
    let theme = test_md_theme();
    let line_bg = theme.search_highlight_bg;
    let match_bg = theme.search_match_bg;
    let line = Line::from(vec![Span::raw("Hello World")]);
    let result = highlight_line(&line, &theme, "hello");
    assert_eq!(result.spans.len(), 2);
    assert_eq!(result.spans[0].content.as_ref(), "Hello");
    assert_eq!(result.spans[0].style.bg, Some(match_bg));
    assert!(result.spans[0]
        .style
        .add_modifier
        .contains(ratatui::style::Modifier::BOLD));
    assert_eq!(result.spans[1].content.as_ref(), " World");
    assert_eq!(result.spans[1].style.bg, Some(line_bg));
    assert!(!result.spans[1]
        .style
        .add_modifier
        .contains(ratatui::style::Modifier::BOLD));
}

#[test]
fn highlight_line_cross_span() {
    let theme = test_md_theme();
    let line_bg = theme.search_highlight_bg;
    let match_bg = theme.search_match_bg;
    let bold = Style::default().add_modifier(ratatui::style::Modifier::BOLD);
    let line = Line::from(vec![Span::styled("hel", bold), Span::raw("lo world")]);
    let result = highlight_line(&line, &theme, "hello");
    assert_eq!(result.spans[0].content.as_ref(), "hel");
    assert_eq!(result.spans[0].style.bg, Some(match_bg));
    assert!(result.spans[0]
        .style
        .add_modifier
        .contains(ratatui::style::Modifier::BOLD));
    assert_eq!(result.spans[1].content.as_ref(), "lo");
    assert_eq!(result.spans[1].style.bg, Some(match_bg));
    assert!(result.spans[1]
        .style
        .add_modifier
        .contains(ratatui::style::Modifier::BOLD));
    assert_eq!(result.spans[2].content.as_ref(), " world");
    assert_eq!(result.spans[2].style.bg, Some(line_bg));
}

#[test]
fn highlight_line_no_match_returns_clone() {
    let theme = test_md_theme();
    let line = Line::from(vec![Span::raw("hello world")]);
    let result = highlight_line(&line, &theme, "xyz");
    assert_eq!(result.spans.len(), 1);
    assert_eq!(result.spans[0].content.as_ref(), "hello world");
    assert_eq!(result.spans[0].style.bg, None);
}
