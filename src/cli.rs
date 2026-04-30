use anyhow::Result;

use crate::theme::{parse_theme_preset, ThemePreset};

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct CliOptions {
    pub(crate) picker: bool,
    pub(crate) watch: bool,
    pub(crate) update: bool,
    pub(crate) config: bool,
    pub(crate) debug_input: bool,
    pub(crate) print_help: bool,
    pub(crate) print_version: bool,
    pub(crate) file_arg: Option<String>,
    pub(crate) theme: Option<ThemePreset>,
    pub(crate) editor: Option<String>,
}

pub(crate) fn usage_text() -> &'static str {
    "Usage:  leaf [OPTIONS] [file.md]\n\
     \x20       leaf [--watch] --picker\n\
     \x20       leaf --update\n\
     \x20       echo '# Hello' | leaf\n\
     \n\
     Options:\n\
     \x20 -h, --help                 Show this help message and exit\n\
     \x20 -V, --version              Show version information and exit\n\
     \x20 -w, --watch                Watch the file for changes and reload automatically\n\
     \x20     --theme <PRESET>       Set color theme (arctic|forest|ocean|solarized-dark)\n\
     \x20 -e, --editor <NAME>        Set external editor (nano|vim|code|subl|emacs)\n\
     \x20     --picker               Open the file browser picker\n\
     \x20     --config               Open configuration file in editor\n\
     \x20     --update               Update leaf to the latest version"
}

pub(crate) fn version_text() -> &'static str {
    concat!("leaf ", env!("CARGO_PKG_VERSION"))
}

pub(crate) fn print_usage() {
    println!("{}", usage_text());
}

pub(crate) fn print_version() {
    println!("{}", version_text());
}

pub(crate) fn parse_cli(args: &[String]) -> Result<CliOptions> {
    let mut options = CliOptions::default();
    let mut positional_only = false;
    let mut iter = args.iter().skip(1);

    while let Some(arg) = iter.next() {
        if positional_only {
            if options.file_arg.is_none() {
                options.file_arg = Some(arg.clone());
            } else {
                anyhow::bail!("Too many file arguments");
            }
            continue;
        }

        match arg.as_str() {
            "--picker" => options.picker = true,
            "--watch" | "-w" => options.watch = true,
            "--update" => options.update = true,
            "--config" => options.config = true,
            "--debug-input" => options.debug_input = true,
            "--help" | "-h" => options.print_help = true,
            "--version" | "-V" => options.print_version = true,
            "--theme" => {
                let Some(name) = iter.next() else {
                    anyhow::bail!("Missing value for --theme");
                };
                options.theme = Some(
                    parse_theme_preset(name)
                        .ok_or_else(|| anyhow::anyhow!("Unknown theme: {name}"))?,
                );
            }
            _ if arg.starts_with("--theme=") => {
                let name = &arg["--theme=".len()..];
                options.theme = Some(
                    parse_theme_preset(name)
                        .ok_or_else(|| anyhow::anyhow!("Unknown theme: {name}"))?,
                );
            }
            "--editor" | "-e" => {
                let Some(value) = iter.next() else {
                    anyhow::bail!("Missing value for --editor");
                };
                options.editor = Some(value.clone());
            }
            _ if arg.starts_with("--editor=") => {
                options.editor = Some(arg["--editor=".len()..].to_string());
            }
            "--" => positional_only = true,
            _ if arg.starts_with('-') => anyhow::bail!("Unknown flag: {arg}"),
            _ if options.file_arg.is_none() => options.file_arg = Some(arg.clone()),
            _ => anyhow::bail!("Too many file arguments"),
        }
    }

    if options.update {
        let has_non_update_flags = options.watch
            || options.picker
            || options.debug_input
            || options.config
            || options.file_arg.is_some()
            || options.theme.is_some()
            || options.editor.is_some();
        if has_non_update_flags {
            anyhow::bail!("--update must be used on its own");
        }
    }

    if options.config {
        let has_non_config_flags = options.watch
            || options.picker
            || options.debug_input
            || options.update
            || options.file_arg.is_some()
            || options.theme.is_some()
            || options.editor.is_some();
        if has_non_config_flags {
            anyhow::bail!("--config must be used on its own");
        }
    }

    if options.picker {
        let has_non_picker_flags = options.file_arg.is_some();
        if has_non_picker_flags {
            anyhow::bail!("--picker cannot be combined with a file path");
        }
    }

    Ok(options)
}
