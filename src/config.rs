use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use anyhow::Context;
use serde::Deserialize;

use crate::theme::{resolve_theme_selection, CustomThemeConfig};

const DEFAULT_CONFIG: &str = include_str!("../config.toml");

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub(crate) struct LeafConfig {
    pub(crate) theme: Option<String>,
    pub(crate) editor: Option<String>,
    pub(crate) watch: Option<bool>,
    pub(crate) extras: Vec<String>,
    pub(crate) themes: BTreeMap<String, CustomThemeConfig>,
    #[serde(skip)]
    pub(crate) config_dir: Option<PathBuf>,
}

pub(crate) fn load_config() -> (LeafConfig, Option<String>) {
    let Some(path) = config_path() else {
        return (LeafConfig::default(), None);
    };
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return (LeafConfig::default(), None),
    };
    let mut config = match toml::from_str::<LeafConfig>(&content) {
        Ok(c) => c,
        Err(_) => {
            return (
                LeafConfig::default(),
                Some("Could not parse config.toml, using defaults".to_string()),
            );
        }
    };
    config.config_dir = path.parent().map(Path::to_path_buf);
    let warning = config
        .theme
        .as_deref()
        .and_then(|name| {
            resolve_theme_selection(name, &config.themes, config.config_dir.as_deref()).err()
        })
        .map(|message| format!("{message} in config, using default"));
    (config, warning)
}

pub(crate) fn config_path() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        std::env::var("APPDATA")
            .ok()
            .map(|dir| PathBuf::from(dir).join("leaf").join("config.toml"))
    }
    #[cfg(not(target_os = "windows"))]
    {
        let base = std::env::var("XDG_CONFIG_HOME")
            .ok()
            .filter(|s| !s.is_empty())
            .or_else(|| std::env::var("HOME").ok().map(|h| format!("{h}/.config")))?;
        Some(PathBuf::from(base).join("leaf").join("config.toml"))
    }
}

pub(crate) fn run_config() -> anyhow::Result<()> {
    let path = config_path().context("Cannot determine config directory")?;

    if !path.exists() {
        println!("Creating default config.toml...");
        write_default_config(&path)?;
    }

    println!("Configuration file: {}", path.display());
    open_config_in_editor(&path)
}

fn write_default_config(dest: &Path) -> anyhow::Result<()> {
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Cannot create config directory: {}", parent.display()))?;
    }
    std::fs::write(dest, DEFAULT_CONFIG)
        .with_context(|| format!("Cannot write config file: {}", dest.display()))
}

fn open_config_in_editor(path: &Path) -> anyhow::Result<()> {
    let (config, _) = load_config();
    let editor = crate::editor::resolve_editor(None, config.editor.as_deref());

    if try_launch_editor(&editor, path) {
        return Ok(());
    }

    if let Some(fallback) = crate::editor::resolve_fallback_editor(&editor) {
        try_launch_editor(fallback, path);
    }

    Ok(())
}

fn try_launch_editor(editor: &str, path: &Path) -> bool {
    let (bin, args) = crate::editor::split_editor_cmd(editor);
    std::process::Command::new(bin)
        .args(args)
        .arg(path)
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}
