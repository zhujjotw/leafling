use super::{test_assets, unique_temp_dir};
use crate::app::{App, AppConfig, FileChange};
use crate::cli::parse_cli;
use crate::markdown::{hash_str, parse_markdown, parse_markdown_with_width, read_file_state};
use crate::*;
use crossterm::event::KeyEventKind;
use std::{
    fs,
    time::{SystemTime, UNIX_EPOCH},
};
use syntect::highlighting::ThemeSet;

#[test]
fn search_matches_across_span_boundaries() {
    let (ss, theme) = test_assets();
    let (lines, toc) = parse_markdown("hello **world**", &ss, &theme);
    let mut app = App::new(lines, toc, "stdin".to_string(), false, false, None, None);

    app.set_search_query("hello world");
    app.run_search();

    assert_eq!(app.search_match_count(), 1);
    assert!(line_plain_text(app.line(app.search_matches()[0]).unwrap()).contains("hello world"));
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
    assert!(err
        .to_string()
        .contains("stdin exceeds the maximum supported size"));
}

#[test]
fn parse_cli_accepts_update_on_its_own() {
    let args = vec!["leaf".to_string(), "--update".to_string()];
    let options = parse_cli(&args).unwrap();

    assert!(options.update);
    assert!(!options.watch);
    assert_eq!(options.file_arg, None);
}

#[test]
fn parse_cli_rejects_update_with_other_flags() {
    let args = vec![
        "leaf".to_string(),
        "--update".to_string(),
        "--watch".to_string(),
    ];

    let err = parse_cli(&args).unwrap_err();
    assert!(err.to_string().contains("--update must be used on its own"));
}

#[test]
fn parse_cli_accepts_picker_on_its_own() {
    let args = vec!["leaf".to_string(), "--picker".to_string()];
    let options = parse_cli(&args).unwrap();

    assert!(options.picker);
    assert!(!options.watch);
    assert_eq!(options.file_arg, None);
}

#[test]
fn parse_cli_accepts_picker_with_watch() {
    let args = vec![
        "leaf".to_string(),
        "--picker".to_string(),
        "--watch".to_string(),
    ];

    let options = parse_cli(&args).unwrap();
    assert!(options.picker);
    assert!(options.watch);
    assert_eq!(options.file_arg, None);
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
    let readme_idx = labels
        .iter()
        .position(|label| *label == "README.md")
        .unwrap();
    assert!(notes_idx < readme_idx);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn fuzzy_file_picker_lists_markdown_files_from_subdirectories() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("leaf-fuzzy-picker-test-{unique}"));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("docs/nested")).unwrap();
    fs::write(root.join("README.md"), "# Demo\n").unwrap();
    fs::write(root.join("docs/guide.md"), "# Guide\n").unwrap();
    fs::write(root.join("docs/nested/deep.markdown"), "# Deep\n").unwrap();
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

    assert!(app.open_fuzzy_file_picker(root.clone()));
    assert!(app.is_fuzzy_file_picker());

    let labels: Vec<_> = app
        .file_picker_filtered_indices()
        .iter()
        .map(|idx| app.file_picker_entries()[*idx].label())
        .collect();
    assert!(labels.contains(&"README.md"));
    assert!(labels.contains(&"docs/guide.md"));
    assert!(labels.contains(&"docs/nested/deep.markdown"));
    assert!(!labels.contains(&"ignore.txt"));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn queued_fuzzy_picker_transitions_from_pending_to_loading_to_open() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("leaf-fuzzy-picker-queued-{unique}"));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("docs")).unwrap();
    fs::write(root.join("README.md"), "# Demo\n").unwrap();
    fs::write(root.join("docs/guide.md"), "# Guide\n").unwrap();

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

    app.queue_fuzzy_file_picker(root.clone());
    assert!(app.has_pending_picker());
    assert_eq!(
        app.pending_picker_mode(),
        Some(crate::app::FilePickerMode::Fuzzy)
    );
    assert_eq!(app.pending_picker_dir(), Some(root.as_path()));
    assert!(!app.is_picker_loading());
    assert!(app.start_pending_picker_loading());
    assert!(app.is_picker_loading());
    app.age_picker_loading_by(std::time::Duration::from_secs(1));
    let mut opened = false;
    for _ in 0..50 {
        if app.poll_picker_loading() {
            opened = app.is_file_picker_open();
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert!(opened);
    assert!(app.is_file_picker_open());
    assert!(app.is_fuzzy_file_picker());
    assert!(!app.has_pending_picker());
    assert!(!app.is_picker_loading());

    let labels: Vec<_> = app
        .file_picker_filtered_indices()
        .iter()
        .map(|idx| app.file_picker_entries()[*idx].label())
        .collect();
    assert!(labels.contains(&"README.md"));
    assert!(labels.contains(&"docs/guide.md"));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn fuzzy_file_picker_uses_depth_first_order_with_hidden_first() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("leaf-fuzzy-picker-order-{unique}"));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join(".private")).unwrap();
    fs::create_dir_all(root.join("docs")).unwrap();
    fs::write(root.join(".draft.md"), "# Hidden\n").unwrap();
    fs::write(root.join(".private/alpha.md"), "# Private\n").unwrap();
    fs::write(root.join("README.md"), "# Demo\n").unwrap();
    fs::write(root.join("docs/guide.md"), "# Guide\n").unwrap();

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

    assert!(app.open_fuzzy_file_picker(root.clone()));

    let labels: Vec<_> = app
        .file_picker_filtered_indices()
        .iter()
        .map(|idx| app.file_picker_entries()[*idx].label())
        .collect();
    assert_eq!(
        labels,
        vec![
            ".draft.md",
            "README.md",
            ".private/alpha.md",
            "docs/guide.md",
        ]
    );

    let _ = fs::remove_dir_all(root);
}

#[test]
fn fuzzy_file_picker_uses_depth_first_file_order() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("leaf-fuzzy-picker-bfs-{unique}"));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("a/deep")).unwrap();
    fs::create_dir_all(root.join("b")).unwrap();
    fs::write(root.join("z-root.md"), "# Root\n").unwrap();
    fs::write(root.join("a/a-child.md"), "# Child A\n").unwrap();
    fs::write(root.join("b/b-child.md"), "# Child B\n").unwrap();
    fs::write(root.join("a/deep/a-deep.md"), "# Deep\n").unwrap();

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

    assert!(app.open_fuzzy_file_picker(root.clone()));

    let labels: Vec<_> = app
        .file_picker_filtered_indices()
        .iter()
        .map(|idx| app.file_picker_entries()[*idx].label())
        .collect();
    assert_eq!(
        labels,
        vec![
            "z-root.md",
            "a/a-child.md",
            "a/deep/a-deep.md",
            "b/b-child.md"
        ]
    );

    let _ = fs::remove_dir_all(root);
}

#[test]
fn fuzzy_file_picker_keeps_depth_first_order_when_query_is_empty() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("leaf-fuzzy-picker-empty-query-{unique}"));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join(".nvm")).unwrap();
    fs::create_dir_all(root.join("projects")).unwrap();
    fs::write(root.join(".nvm/README.md"), "# Hidden Readme\n").unwrap();
    fs::write(root.join(".nvm/ROADMAP.md"), "# Hidden Roadmap\n").unwrap();
    fs::write(root.join("projects/README.md"), "# Project Readme\n").unwrap();

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

    assert!(app.open_fuzzy_file_picker(root.clone()));

    let labels: Vec<_> = app
        .file_picker_filtered_indices()
        .iter()
        .map(|idx| app.file_picker_entries()[*idx].label())
        .collect();
    assert_eq!(
        labels,
        vec![".nvm/README.md", ".nvm/ROADMAP.md", "projects/README.md"]
    );

    let _ = fs::remove_dir_all(root);
}

#[test]
fn fuzzy_file_picker_filters_entries_by_query() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("leaf-fuzzy-picker-query-{unique}"));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("docs")).unwrap();
    fs::write(root.join("README.md"), "# Demo\n").unwrap();
    fs::write(root.join("docs/guide.md"), "# Guide\n").unwrap();

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

    assert!(app.open_fuzzy_file_picker(root.clone()));
    app.push_file_picker_query('g');
    app.push_file_picker_query('u');

    let labels: Vec<_> = app
        .file_picker_filtered_indices()
        .iter()
        .map(|idx| app.file_picker_entries()[*idx].label())
        .collect();
    assert_eq!(labels, vec!["docs/guide.md"]);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn fuzzy_file_picker_does_not_match_directory_segments() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("leaf-fuzzy-picker-cla-{unique}"));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join(".notes/backup")).unwrap();
    fs::write(root.join(".notes/backup/PLAN.md"), "# Plan\n").unwrap();
    fs::write(root.join("claude.md"), "# Claude\n").unwrap();

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

    assert!(app.open_fuzzy_file_picker(root.clone()));
    app.push_file_picker_query('c');
    app.push_file_picker_query('l');
    app.push_file_picker_query('a');

    let labels: Vec<_> = app
        .file_picker_filtered_indices()
        .iter()
        .map(|idx| app.file_picker_entries()[*idx].label())
        .collect();
    assert!(labels.contains(&"claude.md"));
    assert!(!labels.contains(&".notes/backup/PLAN.md"));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn fuzzy_file_picker_tracks_match_positions_for_highlighting() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("leaf-fuzzy-picker-highlight-{unique}"));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("claude.md"), "# Claude\n").unwrap();

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

    assert!(app.open_fuzzy_file_picker(root.clone()));
    app.push_file_picker_query('c');
    app.push_file_picker_query('l');
    app.push_file_picker_query('a');

    let labels: Vec<_> = app
        .file_picker_filtered_indices()
        .iter()
        .map(|idx| app.file_picker_entries()[*idx].label())
        .collect();
    assert_eq!(labels, vec!["claude.md"]);
    assert_eq!(app.file_picker_match_positions(0), &[0, 1, 2]);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn fuzzy_file_picker_prefers_compact_matches() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("leaf-fuzzy-picker-compact-{unique}"));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("case.md"), "# Case\n").unwrap();
    fs::write(root.join("ciase.md"), "# Ciase\n").unwrap();

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

    assert!(app.open_fuzzy_file_picker(root.clone()));
    app.push_file_picker_query('c');
    app.push_file_picker_query('a');

    let labels: Vec<_> = app
        .file_picker_filtered_indices()
        .iter()
        .map(|idx| app.file_picker_entries()[*idx].label())
        .collect();
    assert_eq!(labels, vec!["case.md", "ciase.md"]);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn fuzzy_file_picker_prefers_contiguous_matches_over_earlier_scattered_matches() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("leaf-fuzzy-picker-contiguous-{unique}"));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join(".notes/todo")).unwrap();
    fs::create_dir_all(root.join(".notes/tests")).unwrap();
    fs::write(root.join(".notes/todo/review-chatgpt.md"), "# ChatGPT\n").unwrap();
    fs::write(root.join(".notes/tests/themes-showcase.md"), "# Showcase\n").unwrap();

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

    assert!(app.open_fuzzy_file_picker(root.clone()));
    app.push_file_picker_query('c');
    app.push_file_picker_query('a');

    let labels: Vec<_> = app
        .file_picker_filtered_indices()
        .iter()
        .map(|idx| app.file_picker_entries()[*idx].label())
        .collect();
    let showcase_idx = labels
        .iter()
        .position(|label| *label == ".notes/tests/themes-showcase.md")
        .unwrap();
    let chatgpt_idx = labels
        .iter()
        .position(|label| *label == ".notes/todo/review-chatgpt.md")
        .unwrap();
    assert!(showcase_idx < chatgpt_idx);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn fuzzy_file_picker_prefers_filename_prefix_matches() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("leaf-fuzzy-picker-prefix-{unique}"));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("todo-case.md"), "# Todo\n").unwrap();
    fs::write(root.join("case-study.md"), "# Case\n").unwrap();

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

    assert!(app.open_fuzzy_file_picker(root.clone()));
    app.push_file_picker_query('c');
    app.push_file_picker_query('a');

    let labels: Vec<_> = app
        .file_picker_filtered_indices()
        .iter()
        .map(|idx| app.file_picker_entries()[*idx].label())
        .collect();
    assert_eq!(labels, vec!["case-study.md", "todo-case.md"]);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn fuzzy_file_picker_prefers_token_boundary_matches() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("leaf-fuzzy-picker-boundary-{unique}"));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("alpha-case.md"), "# Boundary\n").unwrap();
    fs::write(root.join("alphacase.md"), "# Plain\n").unwrap();

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

    assert!(app.open_fuzzy_file_picker(root.clone()));
    app.push_file_picker_query('c');
    app.push_file_picker_query('a');

    let labels: Vec<_> = app
        .file_picker_filtered_indices()
        .iter()
        .map(|idx| app.file_picker_entries()[*idx].label())
        .collect();
    assert_eq!(labels, vec!["alpha-case.md", "alphacase.md"]);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn fuzzy_file_picker_prefers_shallower_paths_on_equal_scores() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("leaf-fuzzy-picker-depth-{unique}"));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("nested/deeper")).unwrap();
    fs::write(root.join("case.md"), "# Root\n").unwrap();
    fs::write(root.join("nested/deeper/case.md"), "# Nested\n").unwrap();

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

    assert!(app.open_fuzzy_file_picker(root.clone()));
    app.push_file_picker_query('c');
    app.push_file_picker_query('a');
    app.push_file_picker_query('s');
    app.push_file_picker_query('e');

    let labels: Vec<_> = app
        .file_picker_filtered_indices()
        .iter()
        .map(|idx| app.file_picker_entries()[*idx].label())
        .collect();
    assert_eq!(labels, vec!["case.md", "nested/deeper/case.md"]);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn fuzzy_file_picker_allows_q_in_query() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("leaf-fuzzy-picker-q-{unique}"));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("query.md"), "# Query\n").unwrap();

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

    assert!(app.open_fuzzy_file_picker(root.clone()));
    app.push_file_picker_query('q');
    assert_eq!(app.file_picker_query(), "q");

    let labels: Vec<_> = app
        .file_picker_filtered_indices()
        .iter()
        .map(|idx| app.file_picker_entries()[*idx].label())
        .collect();
    assert_eq!(labels, vec!["query.md"]);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn fuzzy_file_picker_skips_ignored_technical_directories() {
    let root = unique_temp_dir("leaf-fuzzy-picker-ignore");
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join(".git")).unwrap();
    fs::create_dir_all(root.join("target")).unwrap();
    fs::create_dir_all(root.join("vendor")).unwrap();
    fs::create_dir_all(root.join("var")).unwrap();
    fs::create_dir_all(root.join(".notes")).unwrap();
    fs::write(root.join(".git/ignored.md"), "# Ignored\n").unwrap();
    fs::write(root.join("target/ignored.md"), "# Ignored\n").unwrap();
    fs::write(root.join("vendor/ignored.md"), "# Ignored\n").unwrap();
    fs::write(root.join("var/ignored.md"), "# Ignored\n").unwrap();
    fs::write(root.join(".notes/kept.md"), "# Kept\n").unwrap();

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

    assert!(app.open_fuzzy_file_picker(root.clone()));

    let labels: Vec<_> = app
        .file_picker_filtered_indices()
        .iter()
        .map(|idx| app.file_picker_entries()[*idx].label())
        .collect();
    assert_eq!(labels, vec![".notes/kept.md"]);
    assert_eq!(app.file_picker_truncation(), None);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn fuzzy_file_picker_reports_directory_limit_truncation() {
    let root = unique_temp_dir("leaf-fuzzy-picker-dir-limit");
    let _ = fs::remove_dir_all(&root);
    for idx in 0..5_050usize {
        let dir = root.join(format!("nested-{idx:04}"));
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join(format!("file-{idx:04}.md")), "# File\n").unwrap();
    }

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

    assert!(app.open_fuzzy_file_picker(root.clone()));
    assert_eq!(
        app.file_picker_truncation(),
        Some(crate::app::PickerIndexTruncation::Directory)
    );
    assert!(!app.file_picker_entries().is_empty());

    let _ = fs::remove_dir_all(root);
}

#[test]
fn fuzzy_file_picker_reports_file_limit_truncation() {
    let root = unique_temp_dir("leaf-fuzzy-picker-file-limit");
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    for idx in 0..10_050usize {
        fs::write(root.join(format!("file-{idx:05}.md")), "# File\n").unwrap();
    }

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

    assert!(app.open_fuzzy_file_picker(root.clone()));
    assert_eq!(
        app.file_picker_truncation(),
        Some(crate::app::PickerIndexTruncation::File)
    );
    assert_eq!(app.file_picker_entries().len(), 10_000);

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

    assert!(matches!(
        app.check_modified(),
        Some(FileChange::Metadata(_))
    ));

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
    assert_eq!(
        app.total(),
        parse_markdown_with_width(source, &ss, &theme, 20).0.len()
    );
}
