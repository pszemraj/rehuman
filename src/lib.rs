//! rehuman — Unicode‑safe text cleaning & typographic normalization.

#[cfg(feature = "unorm")]
use unicode_normalization::UnicodeNormalization;

use icu_properties::sets as icu_sets;
use serde::Serialize;
use unicode_segmentation::UnicodeSegmentation;

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
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
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

/// Result of a text cleaning operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CleaningResult {
    pub text: String,
    pub changes_made: u64,
    pub stats: CleaningStats,
}

impl CleaningStats {
    /// Merge another stats snapshot into this one.
    pub fn accumulate(&mut self, other: &CleaningStats) {
        self.hidden_chars_removed = self
            .hidden_chars_removed
            .saturating_add(other.hidden_chars_removed);
        self.trailing_whitespace_removed = self
            .trailing_whitespace_removed
            .saturating_add(other.trailing_whitespace_removed);
        self.spaces_normalized = self
            .spaces_normalized
            .saturating_add(other.spaces_normalized);
        self.dashes_normalized = self
            .dashes_normalized
            .saturating_add(other.dashes_normalized);
        self.quotes_normalized = self
            .quotes_normalized
            .saturating_add(other.quotes_normalized);
        self.other_normalized = self.other_normalized.saturating_add(other.other_normalized);
        self.control_chars_removed = self
            .control_chars_removed
            .saturating_add(other.control_chars_removed);
        self.line_endings_normalized = self
            .line_endings_normalized
            .saturating_add(other.line_endings_normalized);
        self.non_keyboard_removed = self
            .non_keyboard_removed
            .saturating_add(other.non_keyboard_removed);
        self.emojis_dropped = self.emojis_dropped.saturating_add(other.emojis_dropped);
    }
}

#[cfg(feature = "stats")]
macro_rules! record_stat {
    ($stats:expr, $field:ident, $amount:expr) => {{
        $stats.$field = $stats.$field.saturating_add($amount);
    }};
}

#[cfg(not(feature = "stats"))]
macro_rules! record_stat {
    ($stats:expr, $field:ident, $amount:expr) => {{
        let _ = &$stats;
        let _ = stringify!($field);
        let _ = &$amount;
    }};
}

macro_rules! record_change {
    ($changes:expr, $stats:expr, $field:ident) => {{
        record_change!($changes, $stats, $field, 1u64);
    }};
    ($changes:expr, $stats:expr, $field:ident, $amount:expr) => {{
        let amount = ($amount) as u64;
        $changes = $changes.saturating_add(amount);
        record_stat!($stats, $field, amount);
    }};
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
        let mut changes = 0u64;

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
            if changed > 0 {
                record_change!(changes, stats, line_endings_normalized, changed);
            }
        }

        let mut out = String::with_capacity(working.len());
        let mut pending_ws: usize = 0;
        let trim = self.options.remove_trailing_whitespace;
        let collapse = self.options.collapse_whitespace;

        let emoji_classifier = if self.options.keyboard_only || self.options.remove_hidden {
            Some(EmojiClassifier::new())
        } else {
            None
        };
        let default_ignorables = icu_sets::default_ignorable_code_point();

        for grapheme in UnicodeSegmentation::graphemes(working.as_str(), true) {
            if grapheme.is_empty() {
                continue;
            }

            if is_newline_grapheme(grapheme) {
                if trim {
                    if pending_ws > 0 {
                        record_change!(changes, stats, trailing_whitespace_removed, pending_ws);
                        pending_ws = 0;
                    }
                } else {
                    flush_pending_whitespace(&mut out, pending_ws, collapse);
                    pending_ws = 0;
                }
                out.push_str(grapheme);
                continue;
            }

            let is_emoji_cluster = emoji_classifier.as_ref().map_or(false, |classifier| {
                classify_emoji_cluster(grapheme, classifier).is_rendered
            });

            if self.options.keyboard_only
                && matches!(self.options.emoji_policy, EmojiPolicy::Drop)
                && is_emoji_cluster
            {
                record_change!(changes, stats, emojis_dropped);
                continue;
            }

            let keep_hidden = is_emoji_cluster
                && (!self.options.keyboard_only
                    || matches!(self.options.emoji_policy, EmojiPolicy::Keep));

            let mut cluster_buffer = String::new();
            let mut emitted_directly = false;

            for mut c in grapheme.chars() {
                if self.options.remove_hidden && default_ignorables.contains(c) {
                    // TODO: expose a preserve-joiners toggle so ZWJ/ZWNJ handling can be configured.
                    if keep_hidden {
                        cluster_buffer.push(c);
                    } else {
                        record_change!(changes, stats, hidden_chars_removed);
                    }
                    continue;
                }

                if self.options.remove_control_chars && is_disallowed_control(c) {
                    record_change!(changes, stats, control_chars_removed);
                    continue;
                }

                if self.options.normalize_spaces {
                    if let Some(&mapped) = SPACE_MAP.get(&c) {
                        record_change!(changes, stats, spaces_normalized);
                        c = mapped;
                    }
                }

                if self.options.normalize_dashes {
                    if let Some(mapped) = map_dash(c) {
                        if mapped != c {
                            record_change!(changes, stats, dashes_normalized);
                        }
                        c = mapped;
                    }
                }

                if self.options.normalize_quotes {
                    if let Some(mapped) = map_quote(c) {
                        if mapped != c {
                            record_change!(changes, stats, quotes_normalized);
                        }
                        c = mapped;
                    }
                }

                if self.options.normalize_other {
                    match c {
                        FRACTION_SLASH => {
                            c = '/';
                            record_change!(changes, stats, other_normalized);
                        }
                        HORIZONTAL_ELLIPSIS | MIDLINE_HORIZONTAL_ELLIPSIS => {
                            flush_pending_whitespace(&mut out, pending_ws, collapse);
                            pending_ws = 0;
                            out.push_str("...");
                            record_change!(changes, stats, other_normalized);
                            emitted_directly = true;
                            break;
                        }
                        _ => {}
                    }
                }

                cluster_buffer.push(c);
            }

            if emitted_directly {
                continue;
            }

            if cluster_buffer.is_empty() {
                continue;
            }

            if cluster_buffer.chars().all(|ch| matches!(ch, ' ' | '\t')) {
                pending_ws += cluster_buffer.chars().count();
                continue;
            }

            if pending_ws > 0 {
                flush_pending_whitespace(&mut out, pending_ws, collapse);
                pending_ws = 0;
            }

            if self.options.keyboard_only {
                // TODO: offer an optional transliteration path when dropping non-ASCII characters.
                if cluster_buffer.chars().all(is_keyboard_ascii)
                    || (is_emoji_cluster && matches!(self.options.emoji_policy, EmojiPolicy::Keep))
                {
                    out.push_str(&cluster_buffer);
                } else if is_emoji_cluster {
                    record_change!(changes, stats, emojis_dropped);
                } else {
                    let removed = cluster_buffer.chars().count();
                    if removed > 0 {
                        record_change!(changes, stats, non_keyboard_removed, removed);
                    }
                }
            } else {
                out.push_str(&cluster_buffer);
            }
        }

        if trim {
            if pending_ws > 0 {
                record_change!(changes, stats, trailing_whitespace_removed, pending_ws);
            }
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

        CleaningResult {
            text,
            changes_made: changes,
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
        text.is_ascii()
            && !self.options.remove_trailing_whitespace
            && !self.options.collapse_whitespace
            && self.options.normalize_line_endings.is_none()
            && !self.options.remove_control_chars
            && matches!(
                self.options.unicode_normalization,
                UnicodeNormalizationMode::None
            )
    }
}

#[derive(Clone, Copy)]
struct EmojiClusterContext {
    is_rendered: bool,
}

struct EmojiClassifier {
    emoji: icu_properties::sets::CodePointSetDataBorrowed<'static>,
    emoji_presentation: icu_properties::sets::CodePointSetDataBorrowed<'static>,
    extended_pictographic: icu_properties::sets::CodePointSetDataBorrowed<'static>,
}

impl EmojiClassifier {
    fn new() -> Self {
        Self {
            emoji: icu_sets::emoji(),
            emoji_presentation: icu_sets::emoji_presentation(),
            extended_pictographic: icu_sets::extended_pictographic(),
        }
    }
}

fn classify_emoji_cluster(grapheme: &str, classifier: &EmojiClassifier) -> EmojiClusterContext {
    let mut has_emoji_presentation = false;
    let mut has_extended_pictographic = false;
    let mut has_emoji = false;
    let mut has_vs16 = false;
    let mut has_zwj = false;
    let mut has_keycap = false;

    for c in grapheme.chars() {
        if classifier.emoji_presentation.contains(c) {
            has_emoji_presentation = true;
        }
        if classifier.extended_pictographic.contains(c) {
            has_extended_pictographic = true;
        }
        if classifier.emoji.contains(c) {
            has_emoji = true;
        }
        match c {
            '\u{FE0F}' => has_vs16 = true,   // Variation Selector-16
            '\u{200D}' => has_zwj = true,    // Zero Width Joiner
            '\u{20E3}' => has_keycap = true, // Combining Enclosing Keycap
            _ => {}
        }
    }

    let is_rendered = has_emoji_presentation
        || has_extended_pictographic
        || (has_emoji && (has_vs16 || has_zwj || has_keycap));

    EmojiClusterContext { is_rendered }
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

fn is_newline_grapheme(g: &str) -> bool {
    matches!(g, "\n" | "\r" | "\r\n")
}

// ----------------- helpers -----------------

fn to_lf(s: &str) -> (String, u64) {
    // Convert CRLF, CR and NEL (U+0085) to LF and count conversions.
    let mut out = String::with_capacity(s.len());
    let mut changed = 0u64;
    let mut it = s.chars().peekable();
    while let Some(c) = it.next() {
        if c == '\r' {
            if matches!(it.peek(), Some('\n')) {
                it.next(); // consume LF
            }
            out.push('\n');
            changed = changed.saturating_add(1);
        } else if c == '\u{0085}' {
            out.push('\n');
            changed = changed.saturating_add(1);
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
