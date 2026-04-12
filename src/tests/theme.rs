use super::{lock_theme_test_state, test_assets};
use crate::app::{App, AppConfig};
use crate::markdown::parse_markdown;
use crate::theme::{current_theme_preset, set_theme_preset, theme_preset_index};
use crate::*;
use syntect::highlighting::ThemeSet;

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
