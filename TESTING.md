# Testing

This project has two kinds of testing:

- automated unit tests with `cargo test`
- manual end-to-end checks using the fixture in this file

## Quick Start

Run automated tests:

```bash
cargo test
```

Open the manual fixture from a file:

```bash
leaf TESTING.md
```

Open it in watch mode:

```bash
leaf --watch TESTING.md
```

Open it through stdin:

```bash
cat TESTING.md | leaf
```

## Manual Coverage

Use the fixture in the `Manual Fixture` section of this file to verify the current feature set.

### Rendering

Confirm these render correctly:

- headings and TOC entries
- paragraphs with normal spacing
- bold, italic, strikethrough, inline code, and links
- blockquotes with multiple paragraphs
- unordered, loose, nested, and ordered lists
- horizontal rules
- tables with left, center, and right alignment
- fenced code blocks with language labels
- wide characters such as `東京`

### Navigation And Search

Use these keys while viewing the fixture:

- `j`, `k`, `d`, `u` for scrolling
- `g`, `G` for top and bottom
- `t` to toggle the TOC
- `1` through `9` to jump to TOC entries
- `/` to search for `tokyo-signal`
- `n` and `N` to move through search matches

### Watch Mode

While running `leaf --watch TESTING.md`:

- edit one of the repeated search terms
- confirm the content reloads automatically
- confirm the `⟳ reloaded` indicator appears
- press `r` to force a reload manually

### Stdin Mode

While running `cat TESTING.md | leaf`:

- confirm the content matches file-backed rendering
- confirm watch mode is not available

### Startup And Error Handling

Run these checks manually:

```bash
leaf --watch
leaf /path/that/does/not/exist.md
leaf --help
```

Confirm each command exits cleanly and does not leave the terminal in raw mode or an alternate screen.

## Notes

The fixture intentionally includes repeated search terms, loose list items, ordered lists starting at non-`1` values, tables, code blocks, and wide characters because those are easy places for terminal Markdown renderers to regress.

## Manual Fixture

Open this file in `leaf` and use the content below as the end-to-end render sample.

Search terms:

- `tokyo-signal`
- `watch-reload-marker`
- `table-edge-check`
- `unicode-width-check`

### Navigation

This section exists to populate the TOC and provide enough content for scrolling and search.

tokyo-signal appears here once.

#### Repeated Search Block

tokyo-signal appears here twice.

tokyo-signal appears here three times.

watch-reload-marker appears here once.

watch-reload-marker appears here twice.

### Paragraph Styles

Plain paragraph text should render with the default body style and spacing.

This line mixes **bold**, *italic*, ~~strikethrough~~, and `inline code` in a single paragraph.

This paragraph also includes a [link to Rust](https://www.rust-lang.org/) so link styling and the leading link marker can be checked.

### Blockquote

> This is a blockquote with *emphasis* and `inline code`.
>
> The second quoted paragraph ensures paragraph flushing still keeps the quote prefix.
>
> unicode-width-check is present here too.

### Lists

#### Unordered Tight List

- first bullet
- second bullet
- third bullet with `inline code`

#### Unordered Loose List

- first loose item

- second loose item after a blank line

- third loose item with two paragraphs

  This continuation paragraph should keep list structure without repeating the bullet.

#### Nested List

- outer one
  - inner one
  - inner two
    - deeper item
- outer two

#### Ordered List

3. item three
4. item four
5. item five

#### Ordered Loose List

7. item seven

8. item eight with a second paragraph

   This continuation paragraph should align with item eight instead of relying on the numeric marker.

### Rules

The horizontal rule below should span cleanly.

---

The document should continue normally after the rule.

### Tables

| Name | Align Left | Center | Right |
| --- | :--- | :---: | ---: |
| Alpha | left | mid | 12 |
| Beta | table-edge-check | centered | 345 |
| Tokyo | unicode-width-check 東京 | wide | 6789 |
| Tabs | tab	value | cell | 10 |

The table above is intended to check borders, alignment, tab expansion, and wide-character handling.

### Code Blocks

```rust
fn main() {
    let city = "東京";
    println!("tokyo-signal: {city}");
}
```

```bash
printf '%s\n' "watch-reload-marker"
leaf --watch TESTING.md
```

```yaml
search:
  primary: tokyo-signal
  secondary: unicode-width-check
```

### Wide Characters

These lines are here to verify width calculations:

- 東京
- café
- naïve

### Long Scroll Area

Line 01: scrolling sample
Line 02: scrolling sample
Line 03: scrolling sample
Line 04: scrolling sample
Line 05: scrolling sample
Line 06: scrolling sample
Line 07: scrolling sample
Line 08: scrolling sample
Line 09: scrolling sample
Line 10: scrolling sample
Line 11: scrolling sample
Line 12: scrolling sample
Line 13: scrolling sample
Line 14: scrolling sample
Line 15: scrolling sample
Line 16: scrolling sample
Line 17: scrolling sample
Line 18: scrolling sample
Line 19: scrolling sample
Line 20: scrolling sample
Line 21: scrolling sample
Line 22: scrolling sample
Line 23: scrolling sample
Line 24: scrolling sample
Line 25: scrolling sample
Line 26: scrolling sample
Line 27: scrolling sample
Line 28: scrolling sample
Line 29: scrolling sample
Line 30: scrolling sample
