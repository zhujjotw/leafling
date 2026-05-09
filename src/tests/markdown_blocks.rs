use super::{rendered_non_empty_lines, test_assets, test_md_theme};
use crate::markdown::{parse_markdown, parse_markdown_with_width};
use crate::*;

#[test]
fn h1_headings_render_double_rule_without_bottom_spacing() {
    let (ss, theme) = test_assets();
    let (lines, _, _) = parse_markdown("# 東京\n", &ss, &theme, &test_md_theme());
    let rendered = rendered_non_empty_lines(&lines);

    assert_eq!(rendered[0], "東京");
    assert_eq!(rendered[1], "═".repeat(display_width("東京")));
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
