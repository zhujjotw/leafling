<p align="center">
  <img src="images/logo-wordmark.svg" alt="leafling" width="360" />
</p>

<p align="center">
  Terminal Markdown previewer with bilingual translation.<br>
  <em>Read English docs in Chinese — right in your terminal.</em>
</p>

<p align="center">
  <a href="https://github.com/RivoLink/leaf"><img src="https://img.shields.io/badge/fork-leaf-blue" alt="fork"></a>
  <img src="https://img.shields.io/badge/language-Rust-orange" alt="rust">
  <img src="https://img.shields.io/badge/translation-DeepL%20%7C%20LLM-green" alt="translation">
</p>

---

## Why leafling?

### The Pain Point

As a Chinese developer, you constantly encounter excellent English documentation — READMEs, RFCs, design docs, API references. You want to read them, but:

- **Switching between the doc and a browser translator** breaks your flow. You lose context every time you switch windows.
- **Browser translators mangle code blocks and markdown formatting**, making technical docs unreadable.
- **You already live in the terminal**. Your editor, your git, your build tools are all there. Why leave it just to read a doc?
- **Existing terminal readers only show the original text**. You're stuck copy-pasting paragraphs into translation tools.

You end up either reading slowly with a dictionary, or giving up on the doc entirely.

### The Solution

**leafling** adds real-time bilingual translation to [leaf](https://github.com/RivoLink/leaf), a beautiful terminal Markdown previewer. One keyboard shortcut turns any English Markdown document into a side-by-side English-Chinese reading experience — without ever leaving your terminal.

Press `Ctrl+T` — the document transforms:

```
## Getting Started                  ## Getting Started

Install the dependencies:           Install the dependencies:

                                    > 译文：安装依赖项：

                                    > 接下来，运行开发服务器...

Next, run the dev server:           Next, run the dev server:
```

Key design decisions:

- **Original text always visible** — translations appear below each paragraph, not replacing it. You can compare and verify.
- **Markdown formatting preserved** — bold, italic, links, and code are translated in context, not broken.
- **Paragraph-level caching** — each paragraph is translated once and cached. Toggling translation on/off is instant.
- **Background translation** — the UI stays responsive while translations load. You see progress in the status bar.
- **Multiple providers** — supports DeepL API and OpenAI-compatible LLM endpoints. Use whichever you have.

## Features (inherited from leaf)

- Live preview with auto-reload (watch mode)
- Syntax highlighting for code blocks
- LaTeX/Math rendering
- Mermaid diagram support
- Table rendering with Unicode borders
- Table of Contents sidebar
- Full-text search with highlighting
- Multiple color themes (Arctic, Forest, Ocean, Solarized)
- File picker with fuzzy matching
- Editor integration (Ctrl+E)
- Mouse support

## Install

From source (requires [Rust](https://rustup.rs)):

```bash
git clone https://github.com/zhujjotw/leafling.git
cd leafling
cargo build --release
cp target/release/leafling /usr/local/bin/
```

## Configuration

Create or edit the config file:

- Linux / macOS: `~/.config/leafling/config.toml`
- Windows: `%APPDATA%\leafling\config.toml`

### Translation setup

```toml
[translation]
# Required: choose a provider
provider = "deepl"          # "deepl" or "llm"

# DeepL (free tier)
api_endpoint = "https://api-free.deepl.com/v2/translate"
api_key = "your-deepl-api-key"

# Or use an LLM provider (OpenAI-compatible)
# provider = "llm"
# api_endpoint = "https://api.openai.com/v1/chat/completions"
# api_key = "your-openai-api-key"

# Language settings (optional, defaults shown)
source_lang = "EN"
target_lang = "ZH"
```

Without the `[translation]` section, leafling works exactly like leaf — press `Ctrl+T` and it will tell you translation is not configured.

### Full config example

```toml
theme = "ocean"
watch = false

[translation]
provider = "deepl"
api_endpoint = "https://api-free.deepl.com/v2/translate"
api_key = "your-key"
source_lang = "EN"
target_lang = "ZH"
```

## Usage

```bash
# Preview a file
leafling README.md

# Preview with translation (press Ctrl+T to toggle)
leafling README.md
```

### Keyboard shortcuts

| Key | Action |
|-----|--------|
| `Ctrl+T` | Toggle bilingual translation |
| `j/k`, `Up/Down` | Scroll |
| `PgUp/PgDn` | Page scroll |
| `g/G` | Jump to top/bottom |
| `t` | Toggle table of contents |
| `Ctrl+F`, `/` | Search |
| `n/N` | Next/previous search match |
| `Ctrl+W`, `w` | Toggle watch mode |
| `Ctrl+E` | Open in editor |
| `Shift+T` | Theme picker |
| `Ctrl+P` | File picker (fuzzy) |
| `Shift+P` | File browser |
| `p` | Show file path |
| `?` | Help |
| `q` | Quit |

## Architecture

Translation is implemented as an independent module (`src/translation/`) with four components:

- **provider.rs** — Translation provider trait with DeepL and LLM implementations
- **segment.rs** — Extracts translatable segments (headings, paragraphs, list items) from Markdown, skipping code blocks and math
- **render.rs** — Builds bilingual display by interleaving original and translated content
- **mod.rs** — State management, background thread coordination, caching

Translations run in a background thread using `mpsc` channels (same pattern as leaf's file picker), keeping the UI responsive.

## Acknowledgments

leafling is a fork of [leaf](https://github.com/RivoLink/leaf) by [RivoLink](https://github.com/RivoLink). All credit for the excellent terminal Markdown previewer goes to the original author.

## License

MIT
