use crate::theme::{current_theme_preset, set_theme_preset, theme_preset_index};
use crate::*;
use crate::app::FileChange;
use crate::markdown::{
    hash_str, parse_markdown, parse_markdown_with_width, read_file_state, resolve_syntax,
};
use crossterm::event::KeyEventKind;
use ratatui::backend::TestBackend;
use ratatui::{text::Line, widgets::Paragraph, Terminal};
use std::{
    fs,
    sync::{Mutex, MutexGuard},
    time::{SystemTime, UNIX_EPOCH},
};
use syntect::{
    highlighting::{Theme, ThemeSet},
    parsing::SyntaxSet,
};

static THEME_TEST_MUTEX: Mutex<()> = Mutex::new(());

fn test_assets() -> (SyntaxSet, Theme) {
    let ss = SyntaxSet::load_defaults_newlines();
    let ts = ThemeSet::load_defaults();
    let theme = ts.themes["base16-ocean.dark"].clone();
    (ss, theme)
}

fn render_buffer(lines: &[Line<'static>]) -> ratatui::buffer::Buffer {
    let width = lines
        .iter()
        .map(|line| line.width())
        .max()
        .unwrap_or(1)
        .max(1) as u16;
    let height = lines.len().max(1) as u16;
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|f| {
            f.render_widget(Paragraph::new(lines.to_vec()), f.area());
        })
        .unwrap();
    terminal.backend().buffer().clone()
}

fn find_symbol(buffer: &ratatui::buffer::Buffer, symbol: &str) -> Option<(u16, u16)> {
    for y in 0..buffer.area.height {
        for x in 0..buffer.area.width {
            if buffer
                .cell((x, y))
                .is_some_and(|cell| cell.symbol() == symbol)
            {
                return Some((x, y));
            }
        }
    }
    None
}

fn rendered_non_empty_lines(lines: &[Line<'static>]) -> Vec<String> {
    lines
        .iter()
        .map(line_plain_text)
        .filter(|line| !line.is_empty())
        .collect()
}

fn lock_theme_test_state() -> MutexGuard<'static, ()> {
    THEME_TEST_MUTEX.lock().unwrap()
}

#[test]
fn search_matches_across_span_boundaries() {
    let (ss, theme) = test_assets();
    let (lines, toc) = parse_markdown("hello **world**", &ss, &theme);
    let mut app = App::new(lines, toc, "stdin".to_string(), false, false, None, None);

    app.set_search_query("hello world");
    app.run_search();

    assert_eq!(app.search_match_count(), 1);
    assert!(
        line_plain_text(app.line(app.search_matches()[0]).unwrap()).contains("hello world")
    );
}

#[test]
fn key_release_events_are_ignored() {
    assert!(should_handle_key(KeyEventKind::Press));
    assert!(should_handle_key(KeyEventKind::Repeat));
    assert!(!should_handle_key(KeyEventKind::Release));
}

#[test]
fn stdin_read_is_rejected_when_over_limit() {
    let mut cursor = std::io::Cursor::new(vec![b'a'; 5]);
    let err = read_stdin_with_limit(&mut cursor, 4).unwrap_err();
    assert!(
        err.to_string()
            .contains("stdin exceeds the maximum supported size")
    );
}

#[test]
fn cancelling_search_clears_query_and_matches() {
    let (ss, theme) = test_assets();
    let (lines, toc) = parse_markdown("alpha\nbeta\nalpha beta\n", &ss, &theme);
    let mut app = App::new(lines, toc, "stdin".to_string(), false, false, None, None);

    app.set_search_query("alpha");
    app.run_search();

    app.begin_search();
    app.set_search_draft("alpha gamma");
    app.cancel_search();

    assert!(!app.is_search_mode());
    assert!(app.search_draft().is_empty());
    assert!(app.search_query().is_empty());
    assert!(app.search_matches().is_empty());
    assert_eq!(app.search_index(), 0);
}

#[test]
fn confirm_search_uses_draft_and_updates_matches() {
    let (ss, theme) = test_assets();
    let (lines, toc) = parse_markdown("alpha\nbeta\nbeta\n", &ss, &theme);
    let mut app = App::new(lines, toc, "stdin".to_string(), false, false, None, None);

    app.begin_search();
    app.set_search_draft("beta");
    app.confirm_search();

    assert!(!app.is_search_mode());
    assert!(app.search_draft().is_empty());
    assert_eq!(app.search_query(), "beta");
    assert_eq!(app.search_match_count(), 2);
}

#[test]
fn confirm_search_with_new_query_restarts_from_first_match() {
    let (ss, theme) = test_assets();
    let (lines, toc) = parse_markdown("alpha\nbeta\nbeta again\n", &ss, &theme);
    let mut app = App::new(lines, toc, "stdin".to_string(), false, false, None, None);

    app.set_search_query("alpha");
    app.run_search();

    app.begin_search();
    app.set_search_draft("beta");
    app.confirm_search();

    assert_eq!(app.search_query(), "beta");
    assert_eq!(app.search_index(), 0);
    assert_eq!(app.scroll(), app.search_matches()[0]);
    assert_eq!(app.search_match_count(), 2);
}

#[test]
fn enter_in_normal_mode_advances_active_search() {
    let (ss, theme) = test_assets();
    let (lines, toc) = parse_markdown("alpha\nbeta alpha\nalpha again\n", &ss, &theme);
    let mut app = App::new(lines, toc, "stdin".to_string(), false, false, None, None);

    app.set_search_query("alpha");
    app.run_search();
    let second_match = app.search_matches()[1];

    app.next_match();

    assert_eq!(app.search_index(), 1);
    assert_eq!(app.scroll(), second_match);
}

#[test]
fn ctrl_c_cancels_search_prompt_and_clears_active_query() {
    let (ss, theme) = test_assets();
    let (lines, toc) = parse_markdown("alpha\nbeta\n", &ss, &theme);
    let mut app = App::new(lines, toc, "stdin".to_string(), false, false, None, None);

    app.set_search_query("alpha");
    app.run_search();

    app.begin_search();
    app.push_search_draft('z');
    app.cancel_search();

    assert!(!app.is_search_mode());
    assert!(app.search_query().is_empty());
    assert!(app.search_matches().is_empty());
    assert_eq!(app.search_index(), 0);
}

#[test]
fn esc_clears_active_search_from_normal_mode() {
    let (ss, theme) = test_assets();
    let (lines, toc) = parse_markdown("alpha\nbeta alpha\n", &ss, &theme);
    let mut app = App::new(lines, toc, "stdin".to_string(), false, false, None, None);

    app.set_search_query("alpha");
    app.run_search();
    app.clear_active_search();

    assert!(!app.is_search_mode());
    assert!(app.search_draft().is_empty());
    assert!(app.search_query().is_empty());
    assert!(app.search_matches().is_empty());
    assert_eq!(app.search_index(), 0);
}

#[test]
fn ctrl_c_clears_active_search_before_exit() {
    let (ss, theme) = test_assets();
    let (lines, toc) = parse_markdown("alpha\nbeta alpha\n", &ss, &theme);
    let mut app = App::new(lines, toc, "stdin".to_string(), false, false, None, None);

    app.set_search_query("alpha");
    app.run_search();
    app.clear_active_search();

    assert!(!app.has_active_search());
    assert!(app.search_query().is_empty());
    assert!(app.search_matches().is_empty());
}

#[test]
fn active_highlight_line_is_none_without_search_matches() {
    let (ss, theme) = test_assets();
    let (lines, toc) = parse_markdown("alpha\nbeta\n", &ss, &theme);
    let app = App::new(lines, toc, "stdin".to_string(), false, false, None, None);

    assert_eq!(app.active_highlight_line(), None);
}

#[test]
fn code_block_box_renders_right_border_in_one_column() {
    let (ss, theme) = test_assets();
    let md = "```ts\nconst city = \"東京\";\n\tconsole.log(city)\n```";
    let (lines, _) = parse_markdown(md, &ss, &theme);
    let buffer = render_buffer(&lines);

    let (right_x, start_y) = find_symbol(&buffer, "┐").unwrap();
    let (_, end_y) = find_symbol(&buffer, "┘").unwrap();

    for y in start_y + 1..end_y {
        assert_eq!(
            buffer.cell((right_x, y)).unwrap().symbol(),
            "│",
            "missing code block right border at row {y}"
        );
    }
}

#[test]
fn table_render_right_border_stays_aligned() {
    let (ss, theme) = test_assets();
    let md = "| Name | Value |\n| --- | --- |\n| 東京 | 12 |\n| tab\tcell | ok |";
    let (lines, _) = parse_markdown(md, &ss, &theme);
    let buffer = render_buffer(&lines);

    let (right_x, start_y) = find_symbol(&buffer, "┐").unwrap();
    let (_, end_y) = find_symbol(&buffer, "┘").unwrap();

    for y in start_y + 1..end_y {
        let symbol = buffer.cell((right_x, y)).unwrap().symbol();
        assert!(
            matches!(symbol, "│" | "┤" | "╡"),
            "unexpected table edge symbol {symbol:?} at row {y}"
        );
    }
}

#[test]
fn table_render_right_border_stays_aligned_with_emoji_cells() {
    let (ss, theme) = test_assets();
    let md = "| Critère | Note |\n| --- | --- |\n| Tests | ✅ Bonne couverture |\n| Sécurité | ⚠ Quelques points |\n";
    let (lines, _) = parse_markdown(md, &ss, &theme);
    let buffer = render_buffer(&lines);

    let (right_x, start_y) = find_symbol(&buffer, "┐").unwrap();
    let (_, end_y) = find_symbol(&buffer, "┘").unwrap();

    for y in start_y + 1..end_y {
        let symbol = buffer.cell((right_x, y)).unwrap().symbol();
        assert!(
            matches!(symbol, "│" | "┤" | "╡"),
            "unexpected emoji-table edge symbol {symbol:?} at row {y}"
        );
    }
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
    let intro_idx = rendered.iter().position(|line| line == "Intro paragraph").unwrap();

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
    assert!(rendered.iter().any(|line| line.starts_with("8. Eighth item")));
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
    let intro_idx = rendered.iter().position(|line| line == "Intro paragraph").unwrap();

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
fn parse_theme_preset_supports_ocean_and_forest() {
    assert_eq!(parse_theme_preset("arctic"), Some(ThemePreset::Arctic));
    assert_eq!(parse_theme_preset("ocean"), Some(ThemePreset::OceanDark));
    assert_eq!(parse_theme_preset("forest"), Some(ThemePreset::Forest));
    assert_eq!(
        parse_theme_preset("solarized-dark"),
        Some(ThemePreset::SolarizedDark)
    );
    assert_eq!(parse_theme_preset("nope"), None);
}

#[test]
fn resolve_syntax_supports_common_language_aliases() {
    let ss = SyntaxSet::load_defaults_newlines();

    assert_eq!(resolve_syntax("py", &ss).name, resolve_syntax("python", &ss).name);
    assert_eq!(resolve_syntax("cpp", &ss).name, resolve_syntax("c++", &ss).name);
    assert_eq!(resolve_syntax("json", &ss).name, "JSON");
    assert_eq!(resolve_syntax("ps1", &ss).name, resolve_syntax("powershell", &ss).name);
}

#[test]
fn theme_presets_are_in_alphabetical_order() {
    let labels: Vec<_> = THEME_PRESETS
        .iter()
        .map(|preset| theme_preset_label(*preset))
        .collect();
    let mut sorted = labels.clone();
    sorted.sort();
    assert_eq!(labels, sorted);
}

#[test]
fn theme_picker_restores_original_preset_on_escape() {
    let _guard = lock_theme_test_state();
    let (ss, theme) = test_assets();
    let ts = ThemeSet::load_defaults();
    let (lines, toc) = parse_markdown("# Demo\n", &ss, &theme);
    let mut app = App::new_with_source(
        lines,
        toc,
        AppConfig {
            filename: "stdin".to_string(),
            source: "# Demo\n".to_string(),
            debug_input: false,
            watch: false,
            filepath: None,
            last_file_state: None,
        },
    );

    let original = current_theme_preset();
    set_theme_preset(ThemePreset::OceanDark);
    app.open_theme_picker();
    assert!(app.set_theme_picker_index(theme_preset_index(ThemePreset::Forest)));
    app.preview_theme_preset(ThemePreset::Forest, &ss, &ts);

    assert_eq!(current_theme_preset(), ThemePreset::Forest);

    app.restore_theme_picker_preview(&ss, &ts);

    assert_eq!(current_theme_preset(), ThemePreset::OceanDark);
    assert!(!app.is_theme_picker_open());
    assert_eq!(app.theme_picker_original(), None);
    set_theme_preset(original);
}

#[test]
fn theme_picker_caches_previewed_themes_for_reuse() {
    let _guard = lock_theme_test_state();
    let (ss, theme) = test_assets();
    let ts = ThemeSet::load_defaults();
    let (lines, toc) = parse_markdown("# Demo\n\n```rs\nfn main() {}\n```\n", &ss, &theme);
    let mut app = App::new_with_source(
        lines,
        toc,
        AppConfig {
            filename: "stdin".to_string(),
            source: "# Demo\n\n```rs\nfn main() {}\n```\n".to_string(),
            debug_input: false,
            watch: false,
            filepath: None,
            last_file_state: None,
        },
    );

    let original = current_theme_preset();
    set_theme_preset(ThemePreset::OceanDark);
    app.open_theme_picker();
    app.preview_theme_preset(ThemePreset::Forest, &ss, &ts);

    assert!(app.has_cached_theme_preview(ThemePreset::Forest));
    assert_eq!(current_theme_preset(), ThemePreset::Forest);

    app.preview_theme_preset(ThemePreset::OceanDark, &ss, &ts);
    assert_eq!(current_theme_preset(), ThemePreset::OceanDark);
    assert!(app.has_cached_theme_preview(ThemePreset::OceanDark));
    set_theme_preset(original);
}

#[test]
fn file_picker_lists_dirs_then_markdown_files_only() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("leaf-picker-test-{unique}"));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("notes")).unwrap();
    fs::write(root.join("README.md"), "# Demo\n").unwrap();
    fs::write(root.join("draft.markdown"), "# Draft\n").unwrap();
    fs::write(root.join("ignore.txt"), "nope\n").unwrap();

    let mut app = App::new_with_source(
        Vec::new(),
        Vec::new(),
        AppConfig {
            filename: "picker".to_string(),
            source: String::new(),
            debug_input: false,
            watch: false,
            filepath: None,
            last_file_state: None,
        },
    );

    assert!(app.open_file_picker(root.clone()));

    let labels: Vec<_> = app
        .file_picker_entries()
        .iter()
        .map(|entry| entry.label())
        .collect();
    assert!(labels.contains(&"notes/"));
    assert!(labels.contains(&"README.md"));
    assert!(labels.contains(&"draft.markdown"));
    assert!(!labels.contains(&"ignore.txt"));

    let notes_idx = labels.iter().position(|label| *label == "notes/").unwrap();
    let readme_idx = labels.iter().position(|label| *label == "README.md").unwrap();
    assert!(notes_idx < readme_idx);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn check_modified_detects_file_metadata_change() {
    let (ss, theme) = test_assets();
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("leaf-check-modified-{unique}.md"));
    fs::write(&path, "# Before\n").unwrap();

    let src = fs::read_to_string(&path).unwrap();
    let (lines, toc) = parse_markdown(&src, &ss, &theme);
    let state = read_file_state(&path).unwrap();
    let mut app = App::new_with_source(
        lines,
        toc,
        AppConfig {
            filename: path.file_name().unwrap().to_string_lossy().to_string(),
            source: src.clone(),
            debug_input: false,
            watch: true,
            filepath: Some(path.clone()),
            last_file_state: Some(state),
        },
    );
    app.set_last_content_hash(hash_str(&src));

    std::thread::sleep(std::time::Duration::from_millis(10));
    fs::write(&path, "# After\nextra\n").unwrap();

    let change = app.check_modified();
    assert!(matches!(
        change,
        Some(FileChange::Metadata(_)) | Some(FileChange::Content(_))
    ));

    let _ = fs::remove_file(path);
}

#[test]
fn reload_returns_false_when_file_cannot_be_read() {
    let (ss, _theme) = test_assets();
    let ts = ThemeSet::load_defaults();
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("leaf-reload-fail-{unique}.md"));
    fs::write(&path, "# Demo\n").unwrap();

    let mut app = App::new_with_source(
        Vec::new(),
        Vec::new(),
        AppConfig {
            filename: "picker".to_string(),
            source: String::new(),
            debug_input: false,
            watch: true,
            filepath: None,
            last_file_state: None,
        },
    );
    assert!(app.load_path(path.clone(), &ss, &ts));

    fs::remove_file(&path).unwrap();
    assert!(!app.reload(&ss, &ts));
}

#[test]
fn sync_render_width_preserves_scroll_proportion() {
    let (ss, theme) = test_assets();
    let ts = ThemeSet::load_defaults();
    let source = (0..12)
        .map(|idx| {
            format!(
                "Paragraph {idx} has enough repeated content to wrap differently when the render width changes significantly across reparses."
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n");
    let (lines, toc) = parse_markdown_with_width(&source, &ss, &theme, 80);
    let mut app = App::new_with_source(
        lines,
        toc,
        AppConfig {
            filename: "stdin".to_string(),
            source,
            debug_input: false,
            watch: false,
            filepath: None,
            last_file_state: None,
        },
    );

    app.scroll_down(8);
    let old_scroll = app.scroll();
    let old_total = app.total();
    assert!(app.sync_render_width(24, &ss, &ts));

    let new_total = app.total();
    let expected = ((old_scroll as f64 / old_total as f64) * new_total as f64) as usize;
    assert_eq!(app.scroll(), expected.min(new_total.saturating_sub(1)));
}

#[test]
fn check_modified_reports_metadata_when_no_previous_file_state() {
    let (ss, theme) = test_assets();
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("leaf-check-modified-initial-{unique}.md"));
    fs::write(&path, "# Initial\n").unwrap();

    let src = fs::read_to_string(&path).unwrap();
    let (lines, toc) = parse_markdown(&src, &ss, &theme);
    let mut app = App::new_with_source(
        lines,
        toc,
        AppConfig {
            filename: path.file_name().unwrap().to_string_lossy().to_string(),
            source: src.clone(),
            debug_input: false,
            watch: true,
            filepath: Some(path.clone()),
            last_file_state: None,
        },
    );
    app.set_last_content_hash(hash_str(&src));

    assert!(matches!(app.check_modified(), Some(FileChange::Metadata(_))));

    let _ = fs::remove_file(path);
}

#[test]
fn sync_render_width_returns_false_when_clamped_width_is_unchanged() {
    let (ss, theme) = test_assets();
    let ts = ThemeSet::load_defaults();
    let source = "One paragraph that does not matter much for this width clamp test.";
    let (lines, toc) = parse_markdown_with_width(source, &ss, &theme, 20);
    let mut app = App::new_with_source(
        lines,
        toc,
        AppConfig {
            filename: "stdin".to_string(),
            source: source.to_string(),
            debug_input: false,
            watch: false,
            filepath: None,
            last_file_state: None,
        },
    );

    assert!(app.sync_render_width(10, &ss, &ts));
    assert!(!app.sync_render_width(10, &ss, &ts));
    assert_eq!(app.total(), parse_markdown_with_width(source, &ss, &theme, 20).0.len());
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

    assert_eq!(header_idx, item_idx + 1, "expected no blank gap before code block");
    assert!(rendered[header_idx].starts_with("  "));
    assert!(rendered[code_idx].starts_with("  "));
}
