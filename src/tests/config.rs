use crate::*;

#[test]
fn parse_full_config() {
    let toml = r#"
theme = "forest"
editor = "vim"
watch = true
"#;
    let config: LeafConfig = toml::from_str(toml).unwrap();
    assert_eq!(config.theme.as_deref(), Some("forest"));
    assert_eq!(config.editor.as_deref(), Some("vim"));
    assert_eq!(config.watch, Some(true));
}

#[test]
fn parse_partial_config_theme_only() {
    let toml = r#"theme = "arctic""#;
    let config: LeafConfig = toml::from_str(toml).unwrap();
    assert_eq!(config.theme.as_deref(), Some("arctic"));
    assert_eq!(config.editor, None);
    assert_eq!(config.watch, None);
}

#[test]
fn parse_empty_config() {
    let toml = "";
    let config: LeafConfig = toml::from_str(toml).unwrap();
    assert_eq!(config.theme, None);
    assert_eq!(config.editor, None);
    assert_eq!(config.watch, None);
}

#[test]
fn parse_invalid_toml_returns_default() {
    let toml = "not valid {{{{ toml";
    let config: Result<LeafConfig, _> = toml::from_str(toml);
    assert!(config.is_err());
    let fallback = config.unwrap_or_default();
    assert_eq!(fallback.theme, None);
    assert_eq!(fallback.editor, None);
    assert_eq!(fallback.watch, None);
}

#[test]
fn unknown_fields_are_ignored() {
    let toml = r#"
theme = "ocean"
unknown_field = 42
"#;
    let config: LeafConfig = toml::from_str(toml).unwrap();
    assert_eq!(config.theme.as_deref(), Some("ocean"));
}

#[test]
fn invalid_theme_is_not_a_known_preset() {
    let toml = r#"theme = "nonexistent""#;
    let config: LeafConfig = toml::from_str(toml).unwrap();
    assert_eq!(config.theme.as_deref(), Some("nonexistent"));
    assert!(parse_theme_preset("nonexistent").is_none());
}

#[test]
fn config_path_returns_some() {
    let path = config_path();
    assert!(path.is_some());
    let path = path.unwrap();
    assert!(path.ends_with("config.toml"));
    assert!(path.to_string_lossy().contains("leaf"));
}
