<p align="center">
  <img src="images/logo-wordmark.svg" alt="leaf" width="360" />
</p>

<p align="center">
  Terminal Markdown previewer — GUI-like experience.
</p>

## Install or update

Install or update to the latest published binary.

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
# Preview a file
leaf TESTING.md

# Watch mode — reloads automatically on save
leaf --watch TESTING.md
leaf -w TESTING.md

# Open a dash-prefixed filename
leaf -- -notes.md

# Pipe from stdin
claude "explain Rust lifetimes" | leaf
cat TESTING.md | leaf

# Open the file picker in the current directory
leaf
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
- ✅ Syntax highlighting with common language aliases like `py`, `cpp`, `json`, `toml`, `ps1`, and `dockerfile`
- ✅ Unicode box-drawing tables with left / center / right alignment
- ✅ TOC sidebar with active section tracking and two-level navigation
- ✅ Search with match highlighting, `/`, `Ctrl+F`, and `n` / `N`
- ✅ Code blocks `┌─ lang ───┐`
- ✅ Bold, italic, strikethrough, blockquotes, lists, and horizontal rules
- ✅ YAML frontmatter is ignored in both preview and TOC
- ✅ Native stdin input with bounded size
- ✅ File picker when launched without a file
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
