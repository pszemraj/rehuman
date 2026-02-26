//! Basic example showing manual `CleaningOptions` usage.

use rehuman::{
    CleaningOptions, EmojiPolicy, LineEndingStyle, TextCleaner, UnicodeNormalizationMode,
};

fn main() {
    let options = CleaningOptions {
        normalize_quotes: true,
        normalize_dashes: true,
        normalize_other: true,
        normalize_spaces: true,
        unicode_normalization: UnicodeNormalizationMode::NFKC,
        keyboard_only: true,
        emoji_policy: EmojiPolicy::Keep,
        remove_control_chars: true,
        remove_trailing_whitespace: true,
        collapse_whitespace: true,
        normalize_line_endings: Some(LineEndingStyle::Lf),
        ..CleaningOptions::default()
    };

    let cleaner = TextCleaner::new(options);
    let input = "“Hello — world…”\u{00A0}😀\r\nLine 2   \r\n";
    let out = cleaner.clean(input);
    println!("{}", out.text);
    println!("{:?}", out.stats);
}
