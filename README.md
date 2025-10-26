# rehuman

Unicode-safe text cleaning & typographic normalization for Rust.

This crate is a rewrite of [humanize-ai-lib](https://github.com/Nordth/humanize-ai-lib) by [Nordth](https://github.com/Nordth) in typescript, adapted for Rust and expanded with additional features/coverage.

## Features, Functionality

- Strip invisible/control chars (ZWSP, bidi isolates, BOM, etc.)
- Normalize spaces (NBSP, narrow NBSP, figure/ideographic space → ` `)
- Normalize dashes/quotes/ellipsis to plain ASCII
- Optional Unicode NFC/NFD/NFKC/NFKD (feature `unorm` - enabled by default)
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

### Keyboard-only & emoji policy

`keyboard_only` restricts output to ASCII keyboard characters plus whitespace (`\n`, `\r`, `\t`).
When `keyboard_only = true`, you can choose what to do with emoji via:

- `emoji_policy = EmojiPolicy::Drop` (default): drop emoji
- `emoji_policy = EmojiPolicy::Keep`: allow emoji to pass through even when non-ASCII

### Minimal API

```rust
use rehuman::{clean, humanize};

let a = clean("Hi\u{200B}there");             // default preset
let b = humanize("“Quote”—and…more");          // humanize preset
```

### CLI

Two binaries ship with the crate:

- `rehuman` cleans text. `cargo run -- <args>` runs it directly, or `cargo install --path .` makes it globally available.
  - `rehuman notes.txt` cleans a file (<= 5 MB) and writes normalized text to stdout; redirect if you want a file: `rehuman notes.txt > notes.clean.txt`.
  - Pipe data the same way: `curl https://example.com | rehuman --stats`. Stats are written to stderr so pipes stay clean.
  - Defaults enforce keyboard-only output and drop emoji. Override with flags such as `--keep-emoji`, `--keyboard-only=false`, `--unicode-normalization nfkc`, or `--line-endings crlf`.
  - Persist your preferred knobs with `--save-config`. Config lives under the platform config dir (e.g. `~/.config/rehuman/config.toml`); use `--config <path>` to point elsewhere.
  - Inspect or manage settings with `--print-config` (shows the effective TOML) and `--reset-config` (removes the stored defaults before running).
  - Batch-friendly helpers: `--stream` processes input line-by-line, `--inplace` rewrites the provided file safely, `--stats-json` emits machine-readable summaries, and `--exit-code` sets the status to `1` when changes were made.
- `ishuman` checks whether text would change. It prints `1` when no normalization is needed and `0` otherwise.
  - `ishuman notes.txt` uses the same config/flags as `rehuman`. Add `--stats` for a breakdown (stderr), `--json` for a structured response, or `--exit-code` to surface the verdict via the process status.

## Roadmap

- International keyboard mode with a curated non-ASCII allowlist (€, £, §, …).
- Optional transliteration path when dropping non-ASCII characters in keyboard-only mode.
- Toggle to preserve join controls and additional ellipsis variants.
- Automate Unicode data refresh (CI job + script).
- Benchmark suite (open an issue if you need this sooner than later).

Unicode tables are generated during `cargo build` from the official Unicode Character Database, so the binaries remain self-contained-no runtime downloads needed.

## Feature flags

- `unorm` *(default)* - uses `unicode-normalization` for NFC/NFD/NFKC/NFKD.

## License

MIT
