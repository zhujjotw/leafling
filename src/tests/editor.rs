use crate::*;

#[test]
fn binary_name_simple() {
    assert_eq!(binary_name("nano"), "nano");
}

#[test]
fn binary_name_full_path() {
    assert_eq!(binary_name("/usr/bin/code"), "code");
}

#[test]
fn binary_name_with_args() {
    assert_eq!(binary_name("emacs -nw"), "emacs");
}

#[test]
fn binary_name_path_with_args() {
    assert_eq!(binary_name("/usr/bin/emacs -nw"), "emacs");
}

#[test]
fn binary_name_windows() {
    assert_eq!(binary_name("notepad.exe"), "notepad");
}

#[test]
fn classify_gui_editors() {
    assert_eq!(classify("code"), EditorKind::Gui);
    assert_eq!(classify("codium"), EditorKind::Gui);
    assert_eq!(classify("subl"), EditorKind::Gui);
    assert_eq!(classify("gedit"), EditorKind::Gui);
    assert_eq!(classify("kate"), EditorKind::Gui);
    assert_eq!(classify("mousepad"), EditorKind::Gui);
    assert_eq!(classify("notepad.exe"), EditorKind::Gui);
    assert_eq!(classify("notepad++"), EditorKind::Gui);
    assert_eq!(classify("zed"), EditorKind::Gui);
}

#[test]
fn classify_terminal_editors() {
    assert_eq!(classify("nano"), EditorKind::Terminal);
    assert_eq!(classify("vim"), EditorKind::Terminal);
    assert_eq!(classify("nvim"), EditorKind::Terminal);
    assert_eq!(classify("micro"), EditorKind::Terminal);
    assert_eq!(classify("helix"), EditorKind::Terminal);
    assert_eq!(classify("emacs"), EditorKind::Terminal);
}

#[test]
fn classify_unknown_defaults_to_terminal() {
    assert_eq!(classify("some-unknown-editor"), EditorKind::Terminal);
}

#[test]
fn classify_full_path() {
    assert_eq!(classify("/usr/bin/code"), EditorKind::Gui);
    assert_eq!(classify("/usr/local/bin/nano"), EditorKind::Terminal);
}

#[test]
fn classify_with_args() {
    assert_eq!(classify("emacs -nw"), EditorKind::Terminal);
    assert_eq!(classify("/usr/bin/code --new-window"), EditorKind::Gui);
}

#[test]
fn split_editor_cmd_simple() {
    let (bin, args) = split_editor_cmd("nano");
    assert_eq!(bin, "nano");
    assert!(args.is_empty());
}

#[test]
fn split_editor_cmd_with_args() {
    let (bin, args) = split_editor_cmd("emacs -nw");
    assert_eq!(bin, "emacs");
    assert_eq!(args, vec!["-nw"]);
}

#[test]
fn split_editor_cmd_path_with_args() {
    let (bin, args) = split_editor_cmd("/usr/bin/emacs -nw --no-splash");
    assert_eq!(bin, "/usr/bin/emacs");
    assert_eq!(args, vec!["-nw", "--no-splash"]);
}

#[test]
fn resolve_editor_cli_takes_priority() {
    let result = resolve_editor(Some("vim"));
    assert_eq!(result, "vim");
}

#[test]
fn resolve_editor_fallback_is_not_empty() {
    let result = resolve_editor(None);
    assert!(!result.is_empty());
}
