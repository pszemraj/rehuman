//! rehuman — Unicode‑safe text cleaning & typographic normalization.

#[cfg(feature = "unorm")]
use unicode_normalization::UnicodeNormalization;

use icu_properties::sets as icu_sets;

mod generated;
mod sets;
use generated::{DASH_MAP, QUOTE_MAP, SPACE_MAP};
pub use sets::{is_emoji, is_hidden_char, is_keyboard_ascii};

const FRACTION_SLASH: char = '\u{2044}';
const HORIZONTAL_ELLIPSIS: char = '\u{2026}';
const MIDLINE_HORIZONTAL_ELLIPSIS: char = '\u{22EF}';

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

        if text.is_empty() {
            return CleaningResult {
                text: String::new(),
                changes_made: 0,
                stats,
            };
        }

        if self.can_use_ascii_fast_path(text) {
            return CleaningResult {
                text: text.to_owned(),
                changes_made: 0,
                stats,
            };
        }

        let mut working = self.apply_unicode_normalization(text);

        if self.options.normalize_line_endings.is_some() {
            let (lf, changed) = to_lf(&working);
            working = lf;
            stats.line_endings_normalized += changed;
        }

        let mut out = String::with_capacity(working.len());
        let mut pending_ws: usize = 0;
        let trim = self.options.remove_trailing_whitespace;
        let collapse = self.options.collapse_whitespace;

        let default_ignorables = icu_sets::default_ignorable_code_point();
        let emoji_modifiers = icu_sets::emoji_modifier();
        let variation_selectors = icu_sets::variation_selector();

        let mut emoji_sequence_active = false;
        let mut dropping_emoji_sequence = false;

        for mut c in working.chars() {
            let is_emoji_char = is_emoji(c);
            let is_modifier = emoji_modifiers.contains(c);
            let is_variation_selector = variation_selectors.contains(c);
            let is_emoji_component = is_modifier || is_variation_selector || c == '\u{200D}';

            if self.options.keyboard_only
                && matches!(self.options.emoji_policy, EmojiPolicy::Drop)
                && is_emoji_char
            {
                stats.emojis_dropped += 1;
                dropping_emoji_sequence = true;
                emoji_sequence_active = false;
                continue;
            }

            if dropping_emoji_sequence {
                if is_emoji_char {
                    stats.emojis_dropped += 1;
                    continue;
                }
                if is_emoji_component || default_ignorables.contains(c) {
                    continue;
                }
                dropping_emoji_sequence = false;
            }

            if is_emoji_char {
                emoji_sequence_active = true;
            } else if !is_emoji_component {
                emoji_sequence_active = false;
            }

            if self.options.remove_hidden && default_ignorables.contains(c) {
                let keep_hidden = emoji_sequence_active
                    && (!self.options.keyboard_only
                        || matches!(self.options.emoji_policy, EmojiPolicy::Keep));
                if !keep_hidden {
                    stats.hidden_chars_removed += 1;
                    continue;
                }
            }

            if self.options.remove_control_chars && is_disallowed_control(c) {
                stats.control_chars_removed += 1;
                continue;
            }

            if self.options.normalize_spaces {
                if let Some(mapped) = SPACE_MAP.get(&c) {
                    c = *mapped;
                    stats.spaces_normalized += 1;
                }
            }

            if c == '\n' {
                if trim {
                    stats.trailing_whitespace_removed += pending_ws;
                    pending_ws = 0;
                } else {
                    flush_pending_whitespace(&mut out, pending_ws, collapse);
                    pending_ws = 0;
                }
                out.push('\n');
                continue;
            }

            if self.options.normalize_dashes {
                if let Some('-') = map_dash(c) {
                    if c != '-' {
                        stats.dashes_normalized += 1;
                    }
                    c = '-';
                }
            }

            if self.options.normalize_quotes {
                if let Some(mapped) = map_quote(c) {
                    if mapped != c {
                        stats.quotes_normalized += 1;
                        c = mapped;
                    }
                }
            }

            if self.options.normalize_other {
                match c {
                    FRACTION_SLASH => {
                        c = '/';
                        stats.other_normalized += 1;
                    }
                    HORIZONTAL_ELLIPSIS | MIDLINE_HORIZONTAL_ELLIPSIS => {
                        flush_pending_whitespace(&mut out, pending_ws, collapse);
                        pending_ws = 0;
                        out.push_str("...");
                        stats.other_normalized += 1;
                        continue;
                    }
                    _ => {}
                }
            }

            if c == ' ' || c == '\t' {
                pending_ws += 1;
                continue;
            } else if pending_ws > 0 {
                flush_pending_whitespace(&mut out, pending_ws, collapse);
                pending_ws = 0;
            }

            if self.options.keyboard_only && !is_keyboard_ascii(c) {
                let keep_emoji = emoji_sequence_active
                    && matches!(self.options.emoji_policy, EmojiPolicy::Keep)
                    && (is_emoji_char || is_emoji_component);
                if keep_emoji {
                    out.push(c);
                } else {
                    stats.non_keyboard_removed += 1;
                    continue;
                }
            } else {
                out.push(c);
            }
        }

        if trim {
            stats.trailing_whitespace_removed += pending_ws;
        } else {
            flush_pending_whitespace(&mut out, pending_ws, collapse);
        }

        let mut text = out;
        if let Some(style) = self.options.normalize_line_endings {
            text = match style {
                LineEndingStyle::Lf => text,
                LineEndingStyle::Crlf => text.replace('\n', "\r\n"),
                LineEndingStyle::Cr => text.replace('\n', "\r"),
            };
        }

        let changes_made = aggregate_changes(&stats);
        CleaningResult {
            text,
            changes_made,
            stats,
        }
    }

    fn apply_unicode_normalization(&self, text: &str) -> String {
        match self.options.unicode_normalization {
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
        }
    }

    fn can_use_ascii_fast_path(&self, text: &str) -> bool {
        if !text.is_ascii() {
            return false;
        }
        if self.options.keyboard_only
            || self.options.collapse_whitespace
            || self.options.normalize_line_endings.is_some()
        {
            return false;
        }
        if !matches!(
            self.options.unicode_normalization,
            UnicodeNormalizationMode::None
        ) {
            return false;
        }

        let mut trailing_ws = 0usize;
        for c in text.chars() {
            if self.options.remove_control_chars && is_disallowed_control(c) {
                return false;
            }
            match c {
                ' ' | '\t' => trailing_ws += 1,
                '\n' | '\r' => {
                    if self.options.remove_trailing_whitespace && trailing_ws > 0 {
                        return false;
                    }
                    trailing_ws = 0;
                }
                _ => trailing_ws = 0,
            }
        }

        if self.options.remove_trailing_whitespace && trailing_ws > 0 {
            return false;
        }

        true
    }
}

fn flush_pending_whitespace(out: &mut String, pending: usize, collapse: bool) {
    if pending == 0 {
        return;
    }
    if collapse {
        out.push(' ');
    } else {
        for _ in 0..pending {
            out.push(' ');
        }
    }
}

fn is_disallowed_control(c: char) -> bool {
    let cu = c as u32;
    ((cu <= 0x1F) || (0x7F..=0x9F).contains(&cu)) && c != '\n' && c != '\r' && c != '\t'
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
    DASH_MAP.get(&c).copied()
}

fn map_quote(c: char) -> Option<char> {
    QUOTE_MAP.get(&c).copied()
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

    #[test]
    fn fraction_slash_maps_to_ascii() {
        let cleaner = TextCleaner::new(CleaningOptions::default());
        let out = cleaner.clean("1\u{2044}2");
        assert_eq!(out.text, "1/2");
        assert_eq!(out.stats.other_normalized, 1);
    }

    #[test]
    fn keeps_variation_selector_for_emoji() {
        let cleaner = TextCleaner::new(CleaningOptions::default());
        let out = cleaner.clean("👍\u{FE0F}");
        assert_eq!(out.text, "👍\u{FE0F}");
        assert_eq!(out.stats.hidden_chars_removed, 0);
    }

    #[test]
    fn drops_emoji_sequence_when_policy_drop() {
        let cleaner = TextCleaner::new(CleaningOptions {
            keyboard_only: true,
            emoji_policy: EmojiPolicy::Drop,
            ..CleaningOptions::default()
        });
        let out = cleaner.clean("👍\u{FE0F}");
        assert_eq!(out.text, "");
        assert_eq!(out.stats.emojis_dropped, 1);
    }
}
