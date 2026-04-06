<p align="center">
  <img src="images/logo-wordmark.svg" alt="leaf" width="360" />
</p>

<p align="center">
  Terminal Markdown previewer — GUI-like experience.
</p>

## Build & install

Build the release binary:

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

Check the installed version:

```bash
leaf --version
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
| `1`–`9` | Jump to TOC section N |
| `/` | Search |
| `n` / `N` | Next / prev match |
| `r` | Force reload (watch mode) |
| `q` | Quit |

## Features

- ✅ **Watch mode** `--watch` / `-w` — reloads every 250ms, with `⟳ reloaded` flash feedback
- ✅ Syntax highlighting (200+ languages, syntect)
- ✅ Unicode box-drawing tables with left / center / right alignment
- ✅ TOC sidebar with active section tracking and two-level navigation
- ✅ Search with match highlighting and `n` / `N`
- ✅ Code blocks `╭─ lang ───╮`
- ✅ Bold, italic, strikethrough, blockquotes, lists, and horizontal rules
- ✅ YAML frontmatter is ignored in both preview and TOC
- ✅ Native stdin input

## Typical AI Workflow

```bash
# Terminal 1: generate the file
aichat "..." > notes.md

# Terminal 2: live watch
leaf --watch notes.md
```

## Roadmap

- [ ] Themes (light / custom)
- [ ] Copy code block `y`
- [ ] Improve search performance on large files
