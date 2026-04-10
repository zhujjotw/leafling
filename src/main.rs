use anyhow::{bail, Context, Result};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{fs::OpenOptions, io, io::IsTerminal, io::Read, io::Write, path::PathBuf};
use syntect::{highlighting::ThemeSet, parsing::SyntaxSet};

mod app;
mod cli;
mod markdown;
mod render;
mod runtime;
mod terminal;
#[cfg(test)]
mod tests;
mod theme;

use app::{App, AppConfig};
use cli::{parse_cli, print_usage, print_version, CliOptions};
use markdown::{hash_str, parse_markdown, read_file_state};
use runtime::run;
use terminal::{finish_with_restore, TerminalSession};
use theme::{current_syntect_theme, set_theme_preset};

const MAX_STDIN_BYTES: usize = 8 * 1024 * 1024;

#[cfg(test)]
pub(crate) use app::{
    normalize_toc, should_hide_single_h1, should_promote_h2_when_no_h1, toc_display_level, TocEntry,
};
#[cfg(test)]
pub(crate) use markdown::{display_width, line_plain_text};
#[cfg(test)]
pub(crate) use runtime::should_handle_key;
#[cfg(test)]
pub(crate) use theme::{parse_theme_preset, theme_preset_label, ThemePreset, THEME_PRESETS};
#[cfg(test)]
pub(crate) use read_stdin_limited as read_stdin_with_limit;

fn read_stdin_limited<R: Read>(reader: &mut R, max_bytes: usize) -> Result<String> {
    let mut buf = Vec::with_capacity(max_bytes.min(8192));
    let limit = u64::try_from(max_bytes)
        .ok()
        .and_then(|value| value.checked_add(1))
        .context("stdin size limit is too large")?;
    reader
        .take(limit)
        .read_to_end(&mut buf)
        .context("Cannot read stdin")?;
    if buf.len() > max_bytes {
        bail!(
            "stdin exceeds the maximum supported size of {} bytes",
            max_bytes
        );
    }
    String::from_utf8(buf).context("stdin is not valid UTF-8")
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let options = parse_cli(&args)?;

    if options.print_help {
        print_usage();
        return Ok(());
    }
    if options.print_version {
        print_version();
        return Ok(());
    }
    let CliOptions {
        watch,
        debug_input,
        file_arg,
        theme,
        ..
    } = options;
    set_theme_preset(theme);

    if debug_input {
        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open("leaf-debug.log")
            .context("Cannot create leaf-debug.log")?;
        writeln!(file, "leaf debug input log").ok();
    }

    let mut open_picker_dir = None;
    let (src, filename, filepath) = if let Some(f) = file_arg {
        let path = PathBuf::from(&f);
        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("Cannot read: {}", path.display()))?;
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or(f.clone());
        (content, name, Some(path))
    } else {
        if io::stdin().is_terminal() {
            let cwd = std::env::current_dir().context("Cannot read current directory")?;
            let label = cwd
                .file_name()
                .map(|name| name.to_string_lossy().to_string())
                .unwrap_or_else(|| cwd.display().to_string());
            open_picker_dir = Some(cwd);
            (String::new(), label, None)
        } else {
            if watch {
                eprintln!("Error: --watch requires a file path (stdin cannot be watched)");
                std::process::exit(1);
            }
            let mut stdin = io::stdin().lock();
            let buf = read_stdin_limited(&mut stdin, MAX_STDIN_BYTES)?;
            (buf, "stdin".to_string(), None)
        }
    };

    let ss = SyntaxSet::load_defaults_newlines();
    let ts = ThemeSet::load_defaults();
    let theme = current_syntect_theme(&ts).clone();

    let last_file_state = filepath.as_ref().and_then(read_file_state);
    let last_content_hash = hash_str(&src);

    let (lines, toc) = parse_markdown(&src, &ss, &theme);
    let mut app = App::new_with_source(
        lines,
        toc,
        AppConfig {
            filename,
            source: src,
            debug_input,
            watch,
            filepath,
            last_file_state,
        },
    );
    app.set_last_content_hash(last_content_hash);
    if let Some(dir) = open_picker_dir {
        app.open_file_picker(dir);
    }

    let mut stdout = io::stdout();
    let mut session = TerminalSession::enter(&mut stdout)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout))?;
    let run_result = run(&mut terminal, &mut app, &ss, &ts);
    let restore_result = session.restore(&mut terminal);
    finish_with_restore(run_result, restore_result)
}
