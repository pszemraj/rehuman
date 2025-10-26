# rehuman

Unicode-safe text cleaning & typographic normalization for Rust.

- Strip invisible/control chars (ZWSP, bidi isolates, BOM, etc.)
- Normalize spaces (NBSP, narrow NBSP, figure/ideographic space → ` `)
- Normalize dashes/quotes/ellipsis to plain ASCII
- Optional Unicode NFC/NFD/NFKC/NFKD (feature `unorm` — enabled by default)
- Line-ending normalization (LF/CRLF/CR)
- Collapsing/trimming whitespace
- **Keyboard-only** filter with **emoji policy** (keep/drop)
- Per-operation stats

```rust
use rehuman::{TextCleaner, CleaningOptions, UnicodeNormalizationMode, EmojiPolicy};

let cleaner = TextCleaner::new(CleaningOptions {
    normalize_quotes: true,
    normalize_dashes: true,
    unicode_normalization: UnicodeNormalizationMode::NFKC,
    keyboard_only: false,
    ..CleaningOptions::default()
});

let out = cleaner.clean("“Hello—world…”\u{00A0}😀");
assert_eq!(out.text, "\"Hello-world...\" 😀");
```

## Keyboard-only & emoji policy

`keyboard_only` restricts output to ASCII keyboard characters plus whitespace (`\n`, `\r`, `\t`).
When `keyboard_only = true`, you can choose what to do with emoji via:

- `emoji_policy = EmojiPolicy::Drop` (default): drop emoji
- `emoji_policy = EmojiPolicy::Keep`: allow emoji to pass through even when non-ASCII

## Minimal API

```rust
use rehuman::{clean, humanize};

let a = clean("Hi\u{200B}there");             // default preset
let b = humanize("“Quote”—and…more");          // humanize preset
```

## CLI

This crate ships a `rehuman` command-line tool for quick one-off normalization.

- `rehuman notes.txt` cleans a file (up to 5 MB) and prints the result to stdout. Redirect if you want to save: `rehuman notes.txt > notes.clean.txt`.
- Pipe data the same way: `curl https://example.com | rehuman --stats`.
- Default CLI behavior enforces keyboard-only output and drops emoji. Override with flags such as `--keep-emoji`, `--keyboard-only=false`, `--unicode-normalization nfkc`, or `--line-endings crlf`.
- Set custom defaults by passing `--save-config`; the resolved options are stored under the platform config dir (e.g. `~/.config/rehuman/config.toml`). Use `--config <path>` to point at an alternative file.

## Feature flags

- `unorm` *(default)* – uses `unicode-normalization` for NFC/NFD/NFKC/NFKD.

## License

MIT
