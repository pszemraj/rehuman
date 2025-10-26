//! rehuman — Unicode‑safe text cleaning & typographic normalization.

#[cfg(feature = "unorm")]
use unicode_normalization::UnicodeNormalization;

mod sets;
pub use sets::{is_emoji, is_hidden_char, is_keyboard_ascii};

/// Unicode normalization modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnicodeNormalizationMode {
    None,
    NFD,
    NFC,
    NFKD,
    NFKC,
}

/// Line ending styles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineEndingStyle {
    Lf,   // \n
    Crlf, // \r\n
    Cr,   // \r
}

/// Policy for emoji handling when `keyboard_only` is enabled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmojiPolicy {
    Keep,
    Drop,
}

/// Detailed statistics about cleaning operations.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
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

/// Result of a text cleaning operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CleaningResult {
    pub text: String,
    pub changes_made: usize,
    pub stats: CleaningStats,
}

/// Configuration for cleaning.
#[derive(Debug, Clone)]
pub struct CleaningOptions {
    pub remove_hidden: bool,
    pub remove_trailing_whitespace: bool,
    pub normalize_spaces: bool,
    pub normalize_dashes: bool,
    pub normalize_quotes: bool,
    pub normalize_other: bool, // ellipsis (… -> ...), etc.
    pub keyboard_only: bool,
    pub emoji_policy: EmojiPolicy, // effective only if keyboard_only = true
    pub remove_control_chars: bool, // remove Cc excluding \n, \r, \t
    pub collapse_whitespace: bool,
    pub normalize_line_endings: Option<LineEndingStyle>,
    pub unicode_normalization: UnicodeNormalizationMode,
}

impl Default for CleaningOptions {
    fn default() -> Self {
        Self {
            remove_hidden: true,
            remove_trailing_whitespace: true,
            normalize_spaces: true,
            normalize_dashes: true,
            normalize_quotes: true,
            normalize_other: true,
            keyboard_only: false,
            emoji_policy: EmojiPolicy::Drop,
            remove_control_chars: true,
            collapse_whitespace: false,
            normalize_line_endings: None,
            unicode_normalization: UnicodeNormalizationMode::None,
        }
    }
}

impl CleaningOptions {
    /// Minimal preset: only removes hidden/invisible chars.
    pub fn minimal() -> Self {
        Self {
            remove_hidden: true,
            remove_trailing_whitespace: false,
            normalize_spaces: false,
            normalize_dashes: false,
            normalize_quotes: false,
            normalize_other: false,
            keyboard_only: false,
            emoji_policy: EmojiPolicy::Drop,
            remove_control_chars: false,
            collapse_whitespace: false,
            normalize_line_endings: None,
            unicode_normalization: UnicodeNormalizationMode::None,
        }
    }

    /// Balanced preset for day-to-day text.
    pub fn balanced() -> Self {
        Self {
            unicode_normalization: UnicodeNormalizationMode::NFC,
            collapse_whitespace: false,
            ..Self::default()
        }
    }

    /// Humanize preset for AI/LLM-ish text.
    pub fn humanize() -> Self {
        Self {
            unicode_normalization: UnicodeNormalizationMode::NFKC,
            collapse_whitespace: true,
            ..Self::default()
        }
    }

    /// Aggressive preset: maximum cleanup.
    pub fn aggressive() -> Self {
        Self {
            remove_hidden: true,
            remove_trailing_whitespace: true,
            normalize_spaces: true,
            normalize_dashes: true,
            normalize_quotes: true,
            normalize_other: true,
            keyboard_only: true,
            emoji_policy: EmojiPolicy::Drop,
            remove_control_chars: true,
            collapse_whitespace: true,
            normalize_line_endings: Some(LineEndingStyle::Lf),
            unicode_normalization: UnicodeNormalizationMode::NFKC,
        }
    }
}

/// Main cleaner.
pub struct TextCleaner {
    options: CleaningOptions,
}

impl TextCleaner {
    pub fn new(options: CleaningOptions) -> Self {
        Self { options }
    }

    pub fn clean(&self, text: &str) -> CleaningResult {
        let mut stats = CleaningStats::default();

        // 1) Unicode Normalization (if enabled)
        let mut input = match self.options.unicode_normalization {
            UnicodeNormalizationMode::None => text.to_owned(),
            #[cfg(feature = "unorm")]
            UnicodeNormalizationMode::NFD => text.nfd().collect(),
            #[cfg(feature = "unorm")]
            UnicodeNormalizationMode::NFC => text.nfc().collect(),
            #[cfg(feature = "unorm")]
            UnicodeNormalizationMode::NFKD => text.nfkd().collect(),
            #[cfg(feature = "unorm")]
            UnicodeNormalizationMode::NFKC => text.nfkc().collect(),
            #[cfg(not(feature = "unorm"))]
            _ => text.to_owned(),
        };

        // 2) Normalize line endings to LF internally + count (only if requested)
        if self.options.normalize_line_endings.is_some() {
            let (lf, changed) = to_lf(&input);
            input = lf;
            stats.line_endings_normalized += changed;
        }

        // 3) Single pass over chars with look-ahead buffer for trimming & collapsing
        let mut out = String::with_capacity(input.len());
        let mut pending_ws: usize = 0;
        let trim = self.options.remove_trailing_whitespace;
        let collapse = self.options.collapse_whitespace;

        for mut c in input.chars() {
            // Remove control chars (except allowed whitespace)
            if self.options.remove_control_chars {
                let cu = c as u32;
                let is_cc = (cu <= 0x1F) || (0x7F..=0x9F).contains(&cu);
                if is_cc && c != '\n' && c != '\r' && c != '\t' {
                    stats.control_chars_removed += 1;
                    continue;
                }
            }

            // Remove hidden/invisible format chars
            if self.options.remove_hidden && is_hidden_char(c) {
                stats.hidden_chars_removed += 1;
                continue;
            }

            // Normalize spaces
            if self.options.normalize_spaces {
                c = normalize_space_like(c, &mut stats.spaces_normalized);
            }

            // Handle CR/LF without changing style unless normalize_line_endings was set
            if c == '\n' || c == '\r' {
                if trim {
                    stats.trailing_whitespace_removed += pending_ws;
                    pending_ws = 0;
                } else {
                    // flush pending spaces
                    for _ in 0..pending_ws {
                        out.push(' ');
                    }
                    pending_ws = 0;
                }
                out.push(c);
                continue;
            }

            // Normalize punctuation
            if self.options.normalize_dashes {
                if let Some('-') = map_dash(c) {
                    c = '-';
                    stats.dashes_normalized += 1;
                }
            }
            if self.options.normalize_quotes {
                if let Some(q) = map_quote(c) {
                    c = q;
                    stats.quotes_normalized += 1;
                }
            }
            if self.options.normalize_other && c == '…' {
                // Flush pending spaces before multi-char insert
                if pending_ws > 0 {
                    if collapse {
                        out.push(' ');
                    } else {
                        for _ in 0..pending_ws {
                            out.push(' ');
                        }
                    }
                    pending_ws = 0;
                }
                out.push_str("...");
                stats.other_normalized += 1;
                continue;
            }

            // Accumulate spaces/tabs, flush later
            if c == ' ' || c == '\t' {
                pending_ws += 1;
                continue;
            } else if pending_ws > 0 {
                if collapse {
                    out.push(' ');
                } else {
                    for _ in 0..pending_ws {
                        out.push(' ');
                    }
                }
                pending_ws = 0;
            }

            // Keyboard-only filter
            if self.options.keyboard_only {
                if is_keyboard_ascii(c) {
                    out.push(c);
                } else if is_emoji(c) {
                    if matches!(self.options.emoji_policy, EmojiPolicy::Keep) {
                        out.push(c);
                    } else {
                        stats.emojis_dropped += 1;
                    }
                } else {
                    stats.non_keyboard_removed += 1;
                }
            } else {
                out.push(c);
            }
        }

        // End of text: either flush or drop pending spaces based on trimming option
        if trim {
            stats.trailing_whitespace_removed += pending_ws;
        } else {
            for _ in 0..pending_ws {
                out.push(' ');
            }
        }

        // 4) Emit requested EOL style
        if let Some(style) = self.options.normalize_line_endings {
            let out = match style {
                LineEndingStyle::Lf => out,
                LineEndingStyle::Crlf => out.replace('\n', "\r\n"),
                LineEndingStyle::Cr => out.replace('\n', "\r"),
            };
            let changes_made = aggregate_changes(&stats);
            return CleaningResult {
                text: out,
                changes_made,
                stats,
            };
        }

        let changes_made = aggregate_changes(&stats);
        CleaningResult {
            text: out,
            changes_made,
            stats,
        }
    }
}

fn aggregate_changes(stats: &CleaningStats) -> usize {
    stats.hidden_chars_removed
        + stats.trailing_whitespace_removed
        + stats.spaces_normalized
        + stats.dashes_normalized
        + stats.quotes_normalized
        + stats.other_normalized
        + stats.control_chars_removed
        + stats.line_endings_normalized
        + stats.non_keyboard_removed
        + stats.emojis_dropped
}

// ----------------- helpers -----------------

fn to_lf(s: &str) -> (String, usize) {
    // Convert CRLF, CR and NEL (U+0085) to LF and count conversions.
    let mut out = String::with_capacity(s.len());
    let mut changed = 0usize;
    let mut it = s.chars().peekable();
    while let Some(c) = it.next() {
        if c == '\r' {
            if matches!(it.peek(), Some('\n')) {
                it.next(); // consume LF
            }
            out.push('\n');
            changed += 1;
        } else if c == '\u{0085}' {
            out.push('\n');
            changed += 1;
        } else {
            out.push(c);
        }
    }
    (out, changed)
}

fn map_dash(c: char) -> Option<char> {
    match c {
        '\u{2010}' | // hyphen
        '\u{2011}' | // non-breaking hyphen
        '\u{2012}' | // figure dash
        '\u{2013}' | // en dash
        '\u{2014}' | // em dash
        '\u{2015}' | // horizontal bar
        '\u{2212}' | // minus
        '\u{FE58}' | // small em dash
        '\u{FE63}' | // small hyphen-minus
        '\u{FF0D}'   // fullwidth hyphen-minus
            => Some('-'),
        _ => None,
    }
}

fn map_quote(c: char) -> Option<char> {
    match c {
        // double
        '\u{201C}' | '\u{201D}' | '\u{201E}' | '\u{201F}' | '\u{00AB}' | '\u{00BB}'
        | '\u{2033}' | '\u{2034}' | '\u{301D}' | '\u{301E}' | '\u{301F}' => Some('\"'),
        // single / apostrophe-like
        '\u{2018}' | '\u{2019}' | '\u{201A}' | '\u{201B}' | '\u{2032}' | '\u{2035}'
        | '\u{2039}' | '\u{203A}' | '\u{02BC}' => Some('\''),
        _ => None,
    }
}

fn normalize_space_like(c: char, counter: &mut usize) -> char {
    match c {
        '\u{00A0}' | // NBSP
        '\u{1680}' | // Ogham space mark
        '\u{2000}'..='\u{200A}' | // en/em/thin/hair/etc.
        '\u{202F}' | // narrow no-break space
        '\u{205F}' | // medium mathematical space
        '\u{3000}'   // ideographic space
            => { *counter += 1; ' ' }
        _ => c,
    }
}

/// Convenience: clean with default options.
pub fn clean(text: &str) -> CleaningResult {
    TextCleaner::new(CleaningOptions::default()).clean(text)
}

/// Convenience: clean with the humanize preset.
pub fn humanize(text: &str) -> CleaningResult {
    TextCleaner::new(CleaningOptions::humanize()).clean(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn removes_hidden() {
        let c = TextCleaner::new(CleaningOptions {
            remove_hidden: true,
            ..CleaningOptions::minimal()
        });
        let out = c.clean("Hello\u{200B}World");
        assert_eq!(out.text, "HelloWorld");
        assert_eq!(out.stats.hidden_chars_removed, 1);
    }

    #[test]
    fn removes_mongolian_vowel_separator() {
        let c = TextCleaner::new(CleaningOptions::default());
        let out = c.clean("Hello\u{180E}World");
        assert_eq!(out.text, "HelloWorld");
        assert!(out.stats.hidden_chars_removed >= 1);
    }

    #[test]
    fn normalizes_spaces_and_dashes_quotes_and_ellipsis() {
        let c = TextCleaner::new(CleaningOptions::default());
        let out = c.clean("\u{201C}Hi\u{201D}\u{00A0}\u{2014} ok…");
        assert_eq!(out.text, "\"Hi\" - ok...");
        assert!(out.stats.spaces_normalized >= 1);
        assert!(out.stats.dashes_normalized >= 1);
        assert!(out.stats.quotes_normalized >= 2);
        assert!(out.stats.other_normalized >= 1);
    }

    #[test]
    fn trims_trailing_ws() {
        let c = TextCleaner::new(CleaningOptions {
            remove_trailing_whitespace: true,
            ..CleaningOptions::minimal()
        });
        let out = c.clean("a  \n b\t\t\n");
        assert_eq!(out.text, "a\n b\n");
        assert!(out.stats.trailing_whitespace_removed >= 3);
    }

    #[test]
    fn collapses_ws() {
        let c = TextCleaner::new(CleaningOptions {
            collapse_whitespace: true,
            ..CleaningOptions::minimal()
        });
        let out = c.clean("a    b\t\tc");
        assert_eq!(out.text, "a b c");
    }

    #[test]
    fn keyboard_only_with_emoji_policy() {
        let c = TextCleaner::new(CleaningOptions {
            keyboard_only: true,
            emoji_policy: EmojiPolicy::Keep,
            ..CleaningOptions::minimal()
        });
        let out = c.clean("Hello😀世界");
        assert_eq!(out.text, "Hello😀");
        assert!(out.stats.non_keyboard_removed >= 2);
    }

    #[test]
    fn normalize_eol_crlf_to_lf_and_back() {
        let c = TextCleaner::new(CleaningOptions {
            normalize_line_endings: Some(LineEndingStyle::Lf),
            ..CleaningOptions::minimal()
        });
        let out = c.clean("a\r\nb\rc\u{0085}");
        assert_eq!(out.text, "a\nb\nc\n");
        assert!(out.stats.line_endings_normalized >= 3);
    }

    #[test]
    fn default_cleaning_matches_keyboard_equivalent() {
        let out = clean("“Hello—world…”\u{00A0}😀");
        assert_eq!(out.text, "\"Hello-world...\" 😀");
        assert_eq!(out.stats.quotes_normalized, 2);
        assert_eq!(out.stats.dashes_normalized, 1);
        assert_eq!(out.stats.other_normalized, 1);
        assert_eq!(out.stats.spaces_normalized, 1);
        assert_eq!(out.changes_made, 5);
    }

    #[test]
    fn keyboard_only_drops_non_ascii_and_emoji() {
        let cleaner = TextCleaner::new(CleaningOptions {
            keyboard_only: true,
            ..CleaningOptions::default()
        });
        let out = cleaner.clean("Ascii😀世界");
        assert_eq!(out.text, "Ascii");
        assert_eq!(out.stats.emojis_dropped, 1);
        assert!(out.stats.non_keyboard_removed >= 2);
    }

    #[test]
    fn ts_whitespace_scenarios() {
        let input = "Hello\u{200B}\u{00A0}World!  ";

        let cleaner = TextCleaner::new(CleaningOptions::default());
        let out = cleaner.clean(input);
        assert_eq!(out.text, "Hello World!");
        assert_eq!(out.changes_made, 4);

        let cleaner = TextCleaner::new(CleaningOptions {
            remove_trailing_whitespace: false,
            ..CleaningOptions::default()
        });
        let out = cleaner.clean(input);
        assert_eq!(out.text, "Hello World!  ");
        assert_eq!(out.changes_made, 2);

        let cleaner = TextCleaner::new(CleaningOptions {
            remove_hidden: false,
            ..CleaningOptions::default()
        });
        let out = cleaner.clean(input);
        assert_eq!(out.text, "Hello\u{200B} World!");
        assert_eq!(out.changes_made, 3);

        let cleaner = TextCleaner::new(CleaningOptions {
            normalize_spaces: false,
            ..CleaningOptions::default()
        });
        let out = cleaner.clean(input);
        assert_eq!(out.text, "Hello\u{00A0}World!");
        assert_eq!(out.changes_made, 3);
    }

    #[test]
    fn ts_dashes_case() {
        let cleaner = TextCleaner::new(CleaningOptions::default());
        let out = cleaner.clean("I — super — man – 💪");
        assert_eq!(out.text, "I - super - man - 💪");
        assert_eq!(out.stats.dashes_normalized, 3);
        assert_eq!(out.changes_made, 3);
    }

    #[test]
    fn ts_quotes_case() {
        let cleaner = TextCleaner::new(CleaningOptions::default());
        let out = cleaner.clean("Angular “quote” «marks» looks„ like Christmas «« tree");
        assert_eq!(
            out.text,
            "Angular \"quote\" \"marks\" looks\" like Christmas \"\" tree"
        );
        assert_eq!(out.stats.quotes_normalized, 7);
        assert_eq!(out.changes_made, 7);
    }

    #[test]
    fn maps_additional_quotes_and_primes() {
        let cleaner = TextCleaner::new(CleaningOptions::default());
        let out = cleaner.clean("‹left› ‟double‟ ′prime′ ″double″");
        assert_eq!(out.text, "'left' \"double\" 'prime' \"double\"");
        assert!(out.stats.quotes_normalized >= 6);
    }

    #[test]
    fn minus_sign_normalizes() {
        let cleaner = TextCleaner::new(CleaningOptions::default());
        let out = cleaner.clean("5 \u{2212} 3");
        assert_eq!(out.text, "5 - 3");
        assert!(out.stats.dashes_normalized >= 1);
    }

    #[test]
    fn narrow_nbsp_is_normalized() {
        let cleaner = TextCleaner::new(CleaningOptions::default());
        let out = cleaner.clean("5\u{202F}MB");
        assert_eq!(out.text, "5 MB");
        assert_eq!(out.stats.spaces_normalized, 1);
        assert_eq!(out.changes_made, 1);
    }

    #[test]
    fn every_space_like_char_collapses_to_ascii_space() {
        let cleaner = TextCleaner::new(CleaningOptions::default());
        let mut samples = vec!['\u{00A0}', '\u{1680}'];
        samples.extend((0x2000..=0x200A).filter_map(std::char::from_u32));
        samples.push('\u{202F}');
        samples.push('\u{205F}');
        samples.push('\u{3000}');

        for ch in samples {
            let input = format!("a{ch}b");
            let out = cleaner.clean(&input);
            assert_eq!(out.text, "a b", "failed for U+{:04X}", ch as u32);
            assert_eq!(
                out.stats.spaces_normalized, 1,
                "expected a single normalization for U+{:04X}",
                ch as u32
            );
        }
    }
}
