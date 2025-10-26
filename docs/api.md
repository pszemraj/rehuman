# API Reference

---

- [API Reference](#api-reference)
  - [Core Helpers](#core-helpers)
  - [TextCleaner](#textcleaner)
    - [CleaningOptions Fields](#cleaningoptions-fields)
    - [Cleaning Statistics](#cleaning-statistics)
  - [Feature Flags](#feature-flags)
  - [Error Handling](#error-handling)

---

## Core Helpers

```rust
use rehuman::{clean, humanize};

let basic = clean("Hi\u{200B}there");             // -> "Hi there"
let fancy = humanize("“Quote”—and…more");         // -> "\"Quote\"-and...more"
```

- `clean` applies the default preset (hidden character removal, spacing fixes).
- `humanize` applies the "humanize" preset (default preset + typographic normalization + whitespace collapsing).

## TextCleaner

Use `TextCleaner` when you need precise control.

```rust
use rehuman::{
    CleaningOptions, EmojiPolicy, TextCleaner, UnicodeNormalizationMode,
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
    .keyboard_only(true)
    .emoji_policy(EmojiPolicy::Drop)
    .build();

let cleaner = TextCleaner::new(options);

let result = cleaner.clean("“Hello—world…”\u{00A0}😀");
assert_eq!(result.text, "\"Hello-world...\"");
println!("dashes normalized: {}", result.stats.dashes_normalized);
```

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
| `emoji_policy`               | Control emoji in `keyboard_only` mode (`Drop`/`Keep`)             |
| `remove_control_chars`       | Drop control chars except `\n`, `\r`, `\t`                        |
| `collapse_whitespace`        | Collapse consecutive spaces/tabs to a single space                |
| `normalize_line_endings`     | Force LF/CRLF/CR output                                           |
| `unicode_normalization`      | Unicode normalization mode (`None`, `NFD`, `NFC`, `NFKD`, `NFKC`) |

### Builder API

Create tailored configurations with the fluent builder:

```rust
let options = CleaningOptions::builder()
    .keyboard_only(true)
    .emoji_policy(EmojiPolicy::Keep)
    .remove_hidden(false)
    .normalize_line_endings(None)
    .build();
```

The presets (`minimal`, `balanced`, `humanize`, `aggressive`) now spell out every field explicitly, so they serve as documented baselines that you can tweak via the builder.

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

| Flag     | Default | Description                                                                                                                |
| -------- | ------- | -------------------------------------------------------------------------------------------------------------------------- |
| `unorm`  | enabled | Enables Unicode normalization support via the `unicode-normalization` crate. If disabled, requesting normalization will panic at runtime. |
| `stats`  | enabled | Collects per-change counters in the hot path. Disable to skip tracking overhead while keeping change detection accurate.   |

## Error Handling

- The library operates on `&str` and returns a `CleaningResult` whose text is a `Cow<'_, str>` (borrowed when no changes are needed, owned otherwise).
- `TextCleaner::clean` never fails; it always produces a `CleaningResult`.
- CLI helpers surface errors through `anyhow::Result` for ergonomic error messages.
