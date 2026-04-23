use super::{rendered_non_empty_lines, test_assets};
use crate::markdown::{parse_markdown, parse_markdown_with_width, resolve_syntax};
use crate::*;
use syntect::parsing::SyntaxSet;

#[test]
fn h1_headings_render_double_rule_without_bottom_spacing() {
    let (ss, theme) = test_assets();
    let (lines, _) = parse_markdown("# 東京\n", &ss, &theme);
    let rendered = rendered_non_empty_lines(&lines);

    assert_eq!(rendered[0], "東京");
    assert_eq!(rendered[1], "═".repeat(display_width("東京")));
}

#[test]
fn loose_list_items_keep_their_markers() {
    let (ss, theme) = test_assets();
    let (lines, _) = parse_markdown("- first\n\n- second\n", &ss, &theme);
    let rendered: Vec<String> = lines.iter().map(line_plain_text).collect();

    assert!(rendered.iter().any(|line| line.contains("• first")));
    assert!(rendered.iter().any(|line| line.contains("• second")));
}

#[test]
fn ordered_lists_render_numeric_markers() {
    let (ss, theme) = test_assets();
    let (lines, _) = parse_markdown("3. third\n4. fourth\n", &ss, &theme);
    let rendered: Vec<String> = lines.iter().map(line_plain_text).collect();

    assert!(rendered.iter().any(|line| line.contains("3. third")));
    assert!(rendered.iter().any(|line| line.contains("4. fourth")));
}

#[test]
fn multiline_list_items_keep_marker_only_on_first_line() {
    let (ss, theme) = test_assets();
    let (lines, _) = parse_markdown("- first line\n  second line\n", &ss, &theme);
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
    let (lines, _) = parse_markdown("7. seven\n8. eight\n", &ss, &theme);
    let rendered: Vec<String> = lines.iter().map(line_plain_text).collect();

    assert!(rendered.iter().any(|line| line.contains("7. seven")));
    assert!(rendered.iter().any(|line| line.contains("8. eight")));
}

#[test]
fn loose_list_items_render_expected_lines() {
    let (ss, theme) = test_assets();
    let src = "- first loose item\n\n- second loose item after a blank line\n\n- third loose item\n\n  continuation paragraph\n";
    let (lines, _) = parse_markdown(src, &ss, &theme);
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
    let (lines, _) = parse_markdown(src, &ss, &theme);
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
    let (lines, _) = parse_markdown("3. third item\n4. fourth item\n", &ss, &theme);
    let rendered = rendered_non_empty_lines(&lines);

    assert_eq!(rendered, vec!["3. third item", "4. fourth item"]);
}

#[test]
fn paragraph_and_following_list_have_no_blank_gap() {
    let (ss, theme) = test_assets();
    let (lines, _) = parse_markdown("Intro paragraph\n\n- first\n- second\n", &ss, &theme);
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
    let (lines, _) = parse_markdown_with_width(src, &ss, &theme, 36);
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
fn paragraph_and_following_code_block_have_no_blank_gap() {
    let (ss, theme) = test_assets();
    let src = "Intro paragraph\n\n```rs\nfn main() {}\n```\n";
    let (lines, _) = parse_markdown(src, &ss, &theme);
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
    let (lines, _) = parse_markdown(src, &ss, &theme);
    let rendered = rendered_non_empty_lines(&lines);

    assert!(rendered.iter().any(|line| line == "▏ outer"));
    assert!(rendered.iter().any(|line| line == "▏ inner"));
    assert!(rendered.iter().any(|line| line == "▏ outer again"));
}

#[test]
fn long_blockquotes_wrap_into_multiple_prefixed_lines() {
    let (ss, theme) = test_assets();
    let src = "> This is a long blockquote line that should wrap into multiple quoted lines at narrow widths.\n";
    let (lines, _) = parse_markdown_with_width(src, &ss, &theme, 28);
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
    let (_, toc) = parse_markdown("# One\n## Two\n### Three\n#### Four\n", &ss, &theme);

    assert_eq!(toc.len(), 3);
    assert_eq!(toc[0].level, 1);
    assert_eq!(toc[1].level, 2);
    assert_eq!(toc[2].level, 3);
}

#[test]
fn frontmatter_is_ignored_in_preview_and_toc() {
    let (ss, theme) = test_assets();
    let src = "---\ntitle: Demo\nowner: me\n---\n# Visible\nBody\n";
    let (lines, toc) = parse_markdown(src, &ss, &theme);
    let rendered = rendered_non_empty_lines(&lines);

    assert!(!rendered.iter().any(|line| line.contains("title: Demo")));
    assert!(rendered.iter().any(|line| line.contains("Visible")));
    assert_eq!(toc.len(), 1);
    assert_eq!(toc[0].title, "Visible");
}

#[test]
fn h2_headings_are_underlined_and_compact() {
    let (ss, theme) = test_assets();
    let (lines, _) = parse_markdown_with_width("Intro\n\n## Section\nBody\n", &ss, &theme, 40);
    let rendered = rendered_non_empty_lines(&lines);

    assert!(rendered.iter().any(|line| line.contains("Section")));
    assert!(rendered.iter().any(|line| line.contains("────")));
}

#[test]
fn rules_use_render_width_without_extra_blank_after() {
    let (ss, theme) = test_assets();
    let (lines, _) = parse_markdown_with_width("Alpha\n\n---\nBeta\n", &ss, &theme, 24);
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
    let (lines, _) = parse_markdown_with_width(md, &ss, &theme, 36);
    let rendered = rendered_non_empty_lines(&lines);

    assert!(rendered.len() >= 6);
    assert!(rendered.iter().all(|line| display_width(line) <= 36));
}

#[test]
fn wrapped_list_inline_code_keeps_left_padding_in_rendered_line() {
    let (ss, theme) = test_assets();
    let source = "- `leaf --theme ocean README.md` exercises wrapping inside a list item.\n";
    let (lines, _) = parse_markdown_with_width(source, &ss, &theme, 22);

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
    let (lines, _) = parse_markdown(md, &ss, &theme);
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
    let (lines, _) = parse_markdown("The formula $x^2 + y^2$ is here.\n", &ss, &theme);

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
    let (lines, _) = parse_markdown("$$E = mc^2$$\n", &ss, &theme);
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
    let (lines, _) = parse_markdown("Check $\\alpha + \\beta$ here.\n", &ss, &theme);
    let searchable: Vec<String> = lines.iter().map(line_plain_text).collect();

    assert!(
        searchable.iter().any(|line| line.contains("α + β")),
        "latex content should be searchable"
    );
}

#[test]
fn display_latex_in_blockquote_has_quote_prefix() {
    let (ss, theme) = test_assets();
    let (lines, _) = parse_markdown("> $$F = ma$$\n", &ss, &theme);
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
