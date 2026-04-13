use std::path::Path;
use std::process::Command;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum EditorKind {
    Terminal,
    Gui,
}

pub(crate) fn binary_name(editor_cmd: &str) -> &str {
    Path::new(editor_cmd.split_whitespace().next().unwrap_or(editor_cmd))
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(editor_cmd)
}

pub(crate) fn classify(editor_cmd: &str) -> EditorKind {
    match binary_name(editor_cmd) {
        "code" | "codium" | "subl" | "gedit" | "kate" | "mousepad" | "notepad.exe"
        | "notepad++" | "zed" => EditorKind::Gui,
        _ => EditorKind::Terminal,
    }
}

pub(crate) fn split_editor_cmd(cmd: &str) -> (&str, Vec<&str>) {
    let mut parts = cmd.split_whitespace();
    let bin = parts.next().unwrap_or(cmd);
    let args: Vec<&str> = parts.collect();
    (bin, args)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum TerminalEmulator {
    Kitty,
    GnomeTerminal,
    MacTerminal(String),
    WindowsTerminal,
    Termux,
    Unknown,
}

pub(crate) fn detect_terminal_emulator() -> TerminalEmulator {
    if std::env::var("KITTY_PID").is_ok() {
        return TerminalEmulator::Kitty;
    }
    if std::env::var("GNOME_TERMINAL_SCREEN").is_ok() {
        return TerminalEmulator::GnomeTerminal;
    }
    if std::env::var("WT_SESSION").is_ok() {
        return TerminalEmulator::WindowsTerminal;
    }
    if std::env::var("TERMUX_VERSION").is_ok() {
        return TerminalEmulator::Termux;
    }

    if let Ok(tp) = std::env::var("TERM_PROGRAM") {
        match tp.as_str() {
            "iTerm.app" | "iTerm2" | "Apple_Terminal" => {
                return TerminalEmulator::MacTerminal(tp);
            }
            _ => {}
        }
    }

    TerminalEmulator::Unknown
}

pub(crate) fn resolve_editor(cli_editor: Option<&str>) -> String {
    if let Some(e) = cli_editor {
        return e.to_string();
    }
    if let Ok(e) = std::env::var("LEAF_EDITOR") {
        if !e.is_empty() {
            return e;
        }
    }
    if let Ok(e) = std::env::var("VISUAL") {
        if !e.is_empty() {
            return e;
        }
    }
    if let Ok(e) = std::env::var("EDITOR") {
        if !e.is_empty() {
            return e;
        }
    }
    platform_fallback_editor().to_string()
}

fn platform_fallback_editor() -> &'static str {
    if cfg!(target_os = "windows") {
        "notepad.exe"
    } else {
        "nano"
    }
}

pub(crate) fn try_new_tab_command(
    editor: &str,
    file: &Path,
    emulator: &TerminalEmulator,
) -> Option<Command> {
    let (bin, args) = split_editor_cmd(editor);
    let file_str = file.display().to_string();

    match emulator {
        TerminalEmulator::Kitty => {
            let mut cmd = Command::new("kitty");
            cmd.arg("@").arg("launch").arg("--type=tab").arg(bin);
            for a in &args {
                cmd.arg(a);
            }
            cmd.arg(&file_str);
            Some(cmd)
        }
        TerminalEmulator::GnomeTerminal => {
            let mut cmd = Command::new("gnome-terminal");
            cmd.arg("--tab").arg("--").arg(bin);
            for a in &args {
                cmd.arg(a);
            }
            cmd.arg(&file_str);
            Some(cmd)
        }
        TerminalEmulator::MacTerminal(ref tp) => {
            let app_name = if tp == "Apple_Terminal" {
                "Terminal"
            } else {
                "iTerm"
            };
            let escaped_editor = editor.replace('\\', "\\\\").replace('"', "\\\"");
            let escaped_file = file_str.replace('\\', "\\\\").replace('"', "\\\"");
            let script = if tp == "Apple_Terminal" {
                format!(
                    "tell application \"{app_name}\" to do script \"{escaped_editor} {escaped_file}\""
                )
            } else {
                format!(
                    "tell application \"{app_name}\" to tell current window to \
                     create tab with default profile command \"{escaped_editor} {escaped_file}\""
                )
            };
            let mut cmd = Command::new("osascript");
            cmd.arg("-e").arg(script);
            Some(cmd)
        }
        TerminalEmulator::WindowsTerminal => {
            let mut cmd = Command::new("wt");
            cmd.arg("new-tab").arg(bin);
            for a in &args {
                cmd.arg(a);
            }
            cmd.arg(&file_str);
            Some(cmd)
        }
        TerminalEmulator::Termux => {
            let full_cmd = if args.is_empty() {
                format!("{bin} {file_str}")
            } else {
                format!("{bin} {} {file_str}", args.join(" "))
            };
            let mut cmd = Command::new("am");
            cmd.args([
                "startservice",
                "-n",
                "com.termux/.app.TermuxService",
                "-a",
                "com.termux.service_execute",
                "-e",
                "com.termux.execute.command",
            ]);
            cmd.arg(full_cmd);
            Some(cmd)
        }
        TerminalEmulator::Unknown => None,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum EditorResult {
    Opened,
    NeedsSameTerminal,
}

pub(crate) fn open_in_editor(
    editor: &str,
    file: &Path,
    kind: EditorKind,
    emulator: &TerminalEmulator,
) -> Result<EditorResult, String> {
    let (bin, args) = split_editor_cmd(editor);
    match kind {
        EditorKind::Gui => {
            Command::new(bin)
                .args(&args)
                .arg(file)
                .spawn()
                .map_err(|e| format!("{bin}: {e}"))?;
            Ok(EditorResult::Opened)
        }
        EditorKind::Terminal => {
            if let Some(mut cmd) = try_new_tab_command(editor, file, emulator) {
                if cmd.spawn().is_ok() {
                    return Ok(EditorResult::Opened);
                }
            }
            Ok(EditorResult::NeedsSameTerminal)
        }
    }
}

pub(crate) fn check_termux_external_apps() -> bool {
    let home = std::env::var("HOME").unwrap_or_default();
    let path = std::path::Path::new(&home).join(".termux/termux.properties");
    match std::fs::read_to_string(path) {
        Ok(content) => content.lines().any(|line| {
            let trimmed = line.trim();
            !trimmed.starts_with('#')
                && trimmed.contains("allow-external-apps")
                && trimmed.contains("true")
        }),
        Err(_) => false,
    }
}
