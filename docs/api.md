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

let cleaner = TextCleaner::new(CleaningOptions {
    // Character normalization
    normalize_quotes: true,
    normalize_dashes: true,
    normalize_other: true, // e.g. … -> ...

    // Unicode normalization
    unicode_normalization: UnicodeNormalizationMode::NFKC,

    // Whitespace handling
    remove_trailing_whitespace: true,
    collapse_whitespace: true,
    normalize_line_endings: Some(rehuman::LineEndingStyle::Lf),

    // Keyboard enforcement
    keyboard_only: true,
    emoji_policy: EmojiPolicy::Drop,

    ..CleaningOptions::default()
});

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

### Cleaning Statistics

`TextCleaner::clean` returns a `CleaningResult`:

```rust
pub struct CleaningResult {
    pub text: String,
    pub changes_made: usize,
    pub stats: CleaningStats,
}
```

`CleaningStats` contains detailed counters:

```rust
pub struct CleaningStats {
    pub hidden_chars_removed: usize,
    pub trailing_whitespace_removed: usize,
    pub spaces_normalized: usize,
    pub dashes_normalized: usize,
    pub quotes_normalized: usize,
    pub other_normalized: usize,
    pub control_chars_removed: usize,
    pub line_endings_normalized: usize,
    pub non_keyboard_removed: usize,
    pub emojis_dropped: usize,
}
```

Use these metrics for monitoring, debugging, or reporting.

## Feature Flags

| Flag    | Default | Description                                                                                                                |
| ------- | ------- | -------------------------------------------------------------------------------------------------------------------------- |
| `unorm` | enabled | Enables Unicode normalization support via the `unicode-normalization` crate. Disable if you want to avoid that dependency. |

## Error Handling

- The library operates on `&str` and returns owned `String` outputs.
- `TextCleaner::clean` never fails; it always produces a `CleaningResult`.
- CLI helpers surface errors through `anyhow::Result` for ergonomic error messages.
