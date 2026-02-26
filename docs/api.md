# API Reference

This document describes Rust library behavior (`rehuman` crate): defaults, options, presets, stats, and error handling.
For CLI usage, see [CLI Guide](cli.md). For recipes, see [Examples](examples.md).

---

- [API Reference](#api-reference)
  - [Core Helpers](#core-helpers)
  - [Keyboard-Only Behavior](#keyboard-only-behavior)
  - [TextCleaner](#textcleaner)
    - [CleaningOptions Fields](#cleaningoptions-fields)
    - [Builder API](#builder-api)
    - [Cleaning Statistics](#cleaning-statistics)
  - [Reusing Buffers](#reusing-buffers)
  - [Streaming](#streaming)
  - [Feature Flags](#feature-flags)
  - [Error Handling](#error-handling)

---

## Core Helpers

```rust
use rehuman::{clean, humanize};

let basic = clean("Hi\u{200B}there");             // -> "Hithere"
let fancy = humanize("“Quote”—and…more");         // -> "\"Quote\"-and...more"
```

- `clean` applies the default preset (hidden character removal, spacing fixes) and emits keyboard-safe ASCII (emoji are dropped unless you opt out).
- `humanize` applies the "humanize" preset (default preset + typographic normalization + whitespace collapsing).
- Keyboard-only behavior details are documented in [Keyboard-Only Behavior](#keyboard-only-behavior).

## Keyboard-Only Behavior

When `keyboard_only=true`, the cleaner applies this order:

1. Preserve emoji only if `emoji_policy=Keep`.
2. Handle non-ASCII text by `non_ascii_policy`:
   - `Drop`: remove non-ASCII characters.
   - `Fold`: keep compatibility/decomposition-to-ASCII forms.
   - `Transliterate`: fold first, then transliterate remaining non-ASCII where feasible.
3. If `extended_keyboard=true`, keep curated non-ASCII keyboard symbols (for example `€`, `£`, `§`, `…`) without transliterating.
4. Remove hidden joiners (ZWJ/ZWNJ) unless `preserve_joiners=true`.

Examples:

- `"Café"` -> `"Cafe"`
- `"Straße"` -> `"Strasse"` (with `Transliterate`)
- `"½"` -> `"1/2"` (with `Fold` or `Transliterate`)

## TextCleaner

Use `TextCleaner` when you need precise control.

```rust,no_run
use rehuman::{
    CleaningOptions, EmojiPolicy, NonAsciiPolicy, TextCleaner, UnicodeNormalizationMode,
};

let options = CleaningOptions::builder()
    // Character normalization
    .normalize_quotes(true)
    .normalize_dashes(true)
    .normalize_other(true) // e.g. … -> ...
    // Unicode normalization
    .unicode_normalization(UnicodeNormalizationMode::NFKC)
    // Whitespace handling
    .remove_trailing_whitespace(true)
    .collapse_whitespace(true)
    .normalize_line_endings(Some(rehuman::LineEndingStyle::Lf))
    // Keyboard enforcement
    .keyboard_only(true) // true by default
    .emoji_policy(EmojiPolicy::Drop)
    .non_ascii_policy(NonAsciiPolicy::Transliterate)
    .build();

let cleaner = TextCleaner::new(options);

let result = cleaner
    .try_clean("“Hello—world…”\u{00A0}😀")
    .expect("normalization requires the 'unorm' feature");
assert_eq!(result.text, "\"Hello-world...\"");
println!("dashes normalized: {}", result.stats.dashes_normalized);
```

> Both the Rust API (`clean`) and the `rehuman` CLI share the same defaults: keyboard-only output with emoji removed so the result stays ASCII-safe.

### CleaningOptions Fields

| Field                        | Purpose                                                           |
| ---------------------------- | ----------------------------------------------------------------- |
| `remove_hidden`              | Drop default ignorable characters (ZWSP, BOM, etc.)               |
| `remove_trailing_whitespace` | Trim spaces/tabs before newlines                                  |
| `normalize_spaces`           | Map Unicode space separators to ASCII space                       |
| `normalize_dashes`           | Map dashes (em/en/minus) to ASCII hyphen                          |
| `normalize_quotes`           | Map quotation marks to ASCII quotes                               |
| `normalize_other`            | Misc fixes (ellipsis → `...`, fraction slash → `/`)               |
| `keyboard_only`              | Keep ASCII keyboard characters (plus whitespace)                  |
| `extended_keyboard`          | Allow curated non-ASCII keyboard symbols in keyboard-only mode     |
| `emoji_policy`               | Control emoji in `keyboard_only` mode (`Drop`/`Keep`)             |
| `non_ascii_policy`           | Non-ASCII strategy in `keyboard_only` mode (`Drop`/`Fold`/`Transliterate`) |
| `preserve_joiners`           | Preserve ZWJ/ZWNJ when hidden-character removal is enabled         |
| `remove_control_chars`       | Drop control chars except `\n`, `\r`, `\t`                        |
| `collapse_whitespace`        | Collapse consecutive spaces/tabs to a single space                |
| `normalize_line_endings`     | Force LF/CRLF/CR output                                           |
| `unicode_normalization`      | Unicode normalization mode (`None`, `NFD`, `NFC`, `NFKD`, `NFKC`) |
| `strip_bidi_controls`        | (feature: `security`) Remove Unicode bidi override/control chars  |

### Builder API

Create tailored configurations with the fluent builder:

```rust
let options = CleaningOptions::builder()
    .keyboard_only(true)
    .extended_keyboard(false)
    .emoji_policy(EmojiPolicy::Keep)
    .non_ascii_policy(NonAsciiPolicy::Transliterate)
    .preserve_joiners(false)
    .remove_hidden(false)
    .normalize_line_endings(None)
    .build();
```

The presets (`minimal`, `balanced`, `humanize`, `aggressive`) now spell out every field explicitly, so they serve as documented baselines that you can tweak via the builder.
When the optional `security` feature is enabled, you can opt into bidi-control stripping via `.strip_bidi_controls(true)` on the builder.

### Cleaning Statistics

`TextCleaner::clean` returns a `CleaningResult`:

```rust
pub struct CleaningResult<'a> {
    pub text: std::borrow::Cow<'a, str>,
    pub changes_made: u64,
    pub stats: CleaningStats,
}
```

`CleaningStats` contains detailed counters:

```rust
pub struct CleaningStats {
    pub hidden_chars_removed: u64,
    pub trailing_whitespace_removed: u64,
    pub spaces_normalized: u64,
    pub dashes_normalized: u64,
    pub quotes_normalized: u64,
    pub other_normalized: u64,
    pub control_chars_removed: u64,
    pub line_endings_normalized: u64,
    pub non_keyboard_removed: u64,
    pub non_keyboard_transliterated: u64,
    pub emojis_dropped: u64,
}
```

Use these metrics for monitoring, debugging, or reporting.

## Reusing Buffers

For allocation-sensitive paths, call `TextCleaner::clean_into(input, &mut buffer)` to reuse an existing `String`. The function fills the provided buffer with the cleaned text and returns a `CleaningResult` whose `text` borrows from that buffer.

## Streaming

Use `StreamCleaner` to process arbitrarily chunked input while preserving the line-oriented semantics of the batch cleaner.

```rust
use rehuman::{CleaningOptions, StreamCleaner};

let options = CleaningOptions::balanced();
let mut stream = StreamCleaner::new(options);
let mut chunk_output = String::new();

for chunk in ["first line \n", "second", " line\n"] {
    if let Some(result) = stream.feed(chunk, &mut chunk_output) {
        let emitted = result.text.to_owned();
        chunk_output.clear();
        print!("{}", emitted);
    }
}

if let Some(result) = stream.finish(&mut chunk_output) {
    let emitted = result.text.to_owned();
    print!("{}", emitted);
}

let summary = stream.summary();
println!("changes: {}", summary.changes_made);
```

## Feature Flags

| Flag       | Default  | Description                                                                                                                               |
| ---------- | -------- | ----------------------------------------------------------------------------------------------------------------------------------------- |
| `unorm`    | enabled  | Enables Unicode normalization support via the `unicode-normalization` crate. If disabled, `try_*` APIs return an error and infallible `clean*` APIs panic when normalization is requested. |
| `stats`    | enabled  | Collects per-change counters in the hot path. Disable to skip tracking overhead while keeping change detection accurate.                  |
| `security` | disabled | Enables bidi-control stripping and related helpers (opt-in hardening).                                                                    |

## Error Handling

- The library operates on `&str` and returns a `CleaningResult` whose text is a `Cow<'_, str>` (borrowed when no changes are needed, owned otherwise).
- Prefer `TextCleaner::try_clean` / `try_clean_into` to handle `CleaningError` (for example when Unicode normalization is requested without enabling the `unorm` feature). The infallible variants `clean`/`clean_into` will panic in that scenario.
- CLI helpers surface errors through `anyhow::Result` for ergonomic error messages.
