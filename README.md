<p align="center">
  <img src="images/logo-wordmark.svg" alt="leaf" width="360" />
</p>

<p align="center">
  Terminal Markdown previewer — GUI-like experience.
</p>

## Install

Install the latest published binary.

macOS / Linux / Android / Termux:

```bash
curl -fsSL https://raw.githubusercontent.com/RivoLink/leaf/main/scripts/install.sh | sh
```

Windows:

```powershell
irm https://raw.githubusercontent.com/RivoLink/leaf/main/scripts/install.ps1 | iex
```

Verify the installation:

```bash
leaf --version
```

## Update

Update an existing installation to the latest published release:

```bash
leaf --update
```

`leaf --update` downloads the matching published asset, verifies it against the published `checksums.txt` SHA256, and then installs it.

On Windows, if replacing the running `.exe` is blocked by the OS, rerun the PowerShell installer from the install section.

## Build

Clone the repository:

```bash
git clone https://github.com/RivoLink/leaf.git
cd leaf
```

Build the release binary locally:

```bash
cargo build --release
```

Create a local bin directory if needed and symlink `leaf` into it:

```bash
mkdir -p ~/.local/bin
ln -sf "$(pwd)/target/release/leaf" ~/.local/bin/leaf
```

If `~/.local/bin` is not already on your `PATH`, add it to `~/.bashrc` or `~/.zshrc`:

```bash
export PATH="$HOME/.local/bin:$PATH"
```

## Usage

```bash
# Open a Markdown file
leaf TESTING.md

# Watch mode — reloads automatically on save
leaf --watch TESTING.md
leaf -w TESTING.md

# Open the fuzzy Markdown picker
leaf

# Open the classic directory browser picker
leaf --picker

# Open the fuzzy Markdown picker, then watch the selected file
leaf -w

# Open the classic directory browser picker, then watch the selected file
leaf -w --picker

# Open a dash-prefixed filename
leaf -- -notes.md

# Stream Markdown from another CLI tool
claude "explain Rust lifetimes" | leaf

# Preview a local file through stdin
cat TESTING.md | leaf

```

## Keybindings

| Key | Action |
|---|---|
| `j` / `↓` | Scroll down |
| `k` / `↑` | Scroll up |
| `d` / PgDn | Page down (20 lines) |
| `u` / PgUp | Page up (20 lines) |
| `g` / Home | Top |
| `G` / End | Bottom |
| `t` | Toggle TOC sidebar |
| `T` | Open theme picker |
| `?` | Show help popup |
| `1`–`9` | Jump to TOC section N |
| `/` / `Ctrl+F` | Search |
| `n` / `N` | Next / prev match |
| `r` | Force reload (watch mode) |
| `q` | Quit |

## Features

- ✅ **Watch mode** `--watch` / `-w` — reloads every 250ms, with `⟳ reloaded` flash feedback
- ✅ `leaf --watch` can start from the picker and begin watching after file selection
- ✅ Syntax highlighting with common language aliases like `py`, `cpp`, `json`, `toml`, `ps1`, and `dockerfile`
- ✅ Unicode box-drawing tables with left / center / right alignment
- ✅ TOC sidebar with active section tracking and two-level navigation
- ✅ Search with match highlighting, `/`, `Ctrl+F`, and `n` / `N`
- ✅ Code blocks `┌─ lang ───┐`
- ✅ Bold, italic, strikethrough, blockquotes, lists, and horizontal rules
- ✅ YAML frontmatter is ignored in both preview and TOC
- ✅ Native stdin input with bounded size
- ✅ `leaf --update` to fetch, verify via published SHA256, and install the latest release on supported platforms
- ✅ Fuzzy Markdown picker when launched without a file
- ✅ Classic directory browser picker with `leaf --picker`
- ✅ Theme picker with runtime preview
- ✅ Help modal with in-app shortcuts

## Typical AI Workflow

```bash
# Terminal 1: generate the file
aichat "..." > notes.md

# Terminal 2: live watch
leaf --watch notes.md
```

## Roadmap

- [x] Themes (light / custom)
- [ ] Copy code block `y`
- [ ] Code block horizontal scroll
- [ ] Improve search performance on large files

## Troubleshooting

### Windows: missing Visual C++ runtime

If `leaf.exe` does not start on Windows and reports a missing MSVC runtime, install the latest supported Microsoft Visual C++ Redistributable from Microsoft Learn:

- https://learn.microsoft.com/fr-fr/cpp/windows/latest-supported-vc-redist?view=msvc-170

Direct download for the latest supported **X64** Microsoft Visual C++ Redistributable:

- https://aka.ms/vc14/vc_redist.x64.exe

For `leaf-windows-x86_64.exe`, the relevant package is the latest supported **X64** Visual C++ v14 Redistributable.

### Windows: update or file replacement error

If `leaf --update` fails on Windows with an error about replacing, renaming, or writing `leaf.exe`, the running executable was likely locked by the OS.

Close any terminal session still running `leaf`, then rerun the PowerShell installer from the install section:

```powershell
irm https://raw.githubusercontent.com/RivoLink/leaf/main/scripts/install.ps1 | iex
```

## Uninstall

macOS / Linux / Android / Termux:

```bash
rm -f ~/.local/bin/leaf
```

Windows:

```powershell
Remove-Item "$env:LOCALAPPDATA\Programs\leaf\leaf.exe" -Force
```
