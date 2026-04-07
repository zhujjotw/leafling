use anyhow::Result;

use crate::theme::{parse_theme_preset, ThemePreset};

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct CliOptions {
    pub(crate) watch: bool,
    pub(crate) debug_input: bool,
    pub(crate) print_help: bool,
    pub(crate) print_version: bool,
    pub(crate) file_arg: Option<String>,
    pub(crate) theme: ThemePreset,
}

pub(crate) fn usage_text() -> &'static str {
    "Usage:  leaf [--watch] [--theme arctic|forest|ocean|solarized-dark] <file.md>\n        echo '# Hello' | leaf"
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
            "--watch" | "-w" => options.watch = true,
            "--debug-input" => options.debug_input = true,
            "--help" | "-h" => options.print_help = true,
            "--version" | "-V" => options.print_version = true,
            "--theme" => {
                let Some(name) = iter.next() else {
                    anyhow::bail!("Missing value for --theme");
                };
                options.theme = parse_theme_preset(name)
                    .ok_or_else(|| anyhow::anyhow!("Unknown theme: {name}"))?;
            }
            _ if arg.starts_with("--theme=") => {
                let name = &arg["--theme=".len()..];
                options.theme = parse_theme_preset(name)
                    .ok_or_else(|| anyhow::anyhow!("Unknown theme: {name}"))?;
            }
            "--" => positional_only = true,
            _ if arg.starts_with('-') => anyhow::bail!("Unknown flag: {arg}"),
            _ if options.file_arg.is_none() => options.file_arg = Some(arg.clone()),
            _ => anyhow::bail!("Too many file arguments"),
        }
    }

    Ok(options)
}
