use anyhow::{Context, Result};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{fs::OpenOptions, io, io::Read, io::Write, path::PathBuf};
use syntect::{highlighting::ThemeSet, parsing::SyntaxSet};

mod app;
mod cli;
mod markdown;
mod render;
mod runtime;
mod terminal;
#[cfg(test)]
mod tests;

use app::App;
use cli::{parse_cli, print_usage, print_version, CliOptions};
use markdown::{hash_str, parse_markdown, read_file_state};
use runtime::run;
use terminal::{finish_with_restore, TerminalSession};

#[cfg(test)]
pub(crate) use app::{
    normalize_toc, should_hide_single_h1, should_promote_h2_when_no_h1, toc_display_level, TocEntry,
};
#[cfg(test)]
pub(crate) use markdown::{display_width, line_plain_text};
#[cfg(test)]
pub(crate) use runtime::should_handle_key;

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
        ..
    } = options;

    if debug_input {
        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open("leaf-debug.log")
            .context("Cannot create leaf-debug.log")?;
        writeln!(file, "leaf debug input log").ok();
    }

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
        if watch {
            eprintln!("Error: --watch requires a file path (stdin cannot be watched)");
            std::process::exit(1);
        }
        let mut buf = String::new();
        io::stdin()
            .read_to_string(&mut buf)
            .context("Cannot read stdin")?;
        if buf.is_empty() {
            print_usage();
            std::process::exit(1);
        }
        (buf, "stdin".to_string(), None)
    };

    let ss = SyntaxSet::load_defaults_newlines();
    let ts = ThemeSet::load_defaults();
    let theme = ts.themes["base16-ocean.dark"].clone();

    let last_file_state = filepath.as_ref().and_then(read_file_state);
    let last_content_hash = hash_str(&src);

    let (lines, toc) = parse_markdown(&src, &ss, &theme);
    let mut app = App::new(
        lines,
        toc,
        filename,
        debug_input,
        watch,
        filepath,
        last_file_state,
    );
    app.set_last_content_hash(last_content_hash);

    let mut stdout = io::stdout();
    let mut session = TerminalSession::enter(&mut stdout)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout))?;
    let run_result = run(&mut terminal, &mut app, &ss, &theme);
    let restore_result = session.restore(&mut terminal);
    finish_with_restore(run_result, restore_result)
}
