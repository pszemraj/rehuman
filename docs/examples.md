# Examples

These are focused Rust recipes. For defaults, options, and error behavior, see [API Reference](api.md).

## Web Scraping Cleanup

```rust
use rehuman::{CleaningOptions, TextCleaner};

let cleaner = TextCleaner::new(CleaningOptions {
    normalize_quotes: true,
    normalize_dashes: true,
    collapse_whitespace: true,
    remove_trailing_whitespace: true,
    ..CleaningOptions::default()
});

let scraped = "“Smart quotes”—with\u{00A0}weird spaces  ";
let result = cleaner.clean(scraped);
assert_eq!(result.text, "\"Smart quotes\"-with weird spaces");
```

## LLM Output Normalization

```rust
use rehuman::{CleaningOptions, TextCleaner, UnicodeNormalizationMode};

let cleaner = TextCleaner::new(CleaningOptions {
    keyboard_only: true,
    unicode_normalization: UnicodeNormalizationMode::NFKC,
    ..CleaningOptions::default()
});

let llm_output = "Ｆｕｌｌｗｉｄｔｈ　text—ok!";
let result = cleaner.clean(llm_output);
assert_eq!(result.text, "Fullwidth text-ok!");
```

## Database Key Normalization

```rust
use rehuman::{CleaningOptions, TextCleaner, UnicodeNormalizationMode};

let cleaner = TextCleaner::new(CleaningOptions {
    unicode_normalization: UnicodeNormalizationMode::NFKC,
    collapse_whitespace: true,
    remove_trailing_whitespace: true,
    normalize_quotes: false,
    normalize_dashes: false,
    ..CleaningOptions::default()
});

let user_input = "Ｐｒｏｄｕｃｔ　１２３  ";
let key = cleaner.clean(user_input).text;
assert_eq!(key, "Product 123");
```

## Preserving Emoji in Keyboard Mode

```rust
use rehuman::{CleaningOptions, EmojiPolicy, TextCleaner};

let cleaner = TextCleaner::new(CleaningOptions {
    keyboard_only: true,
    emoji_policy: EmojiPolicy::Keep,
    ..CleaningOptions::default()
});

let text = "Hello 👋 world";
let result = cleaner.clean(text);
assert_eq!(result.text, "Hello 👋 world");
```
