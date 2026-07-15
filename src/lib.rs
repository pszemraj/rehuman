//! rehuman — Unicode‑safe text cleaning & typographic normalization.

use deunicode::deunicode_char;
use icu_properties::{props, CodePointSetData, CodePointSetDataBorrowed};
use serde::Serialize;
use std::borrow::Cow;
use std::fmt;
#[cfg(feature = "unorm")]
use unicode_normalization::UnicodeNormalization;
use unicode_segmentation::UnicodeSegmentation;

mod generated;
mod sets;
use generated::{DASH_MAP, GREEK_MAP, QUOTE_MAP, SPACE_MAP};
use sets::{is_common_symbol_or_punctuation, is_latin_script};
pub use sets::{is_emoji, is_extended_keyboard_char, is_hidden_char, is_keyboard_ascii};

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

/// Policy for handling non-ASCII graphemes in `keyboard_only` mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NonAsciiPolicy {
    /// Remove non-ASCII graphemes when they are not explicitly normalized elsewhere.
    Drop,
    /// Keep only compatibility-decomposed ASCII output.
    Fold,
    /// Fold first, then apply transliteration fallbacks before dropping.
    Transliterate,
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
    pub non_keyboard_transliterated: u64,
    pub emojis_dropped: u64,
    #[cfg(feature = "security")]
    pub bidi_controls_removed: u64,
}

/// Result of a text cleaning operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CleaningResult<'a> {
    pub text: Cow<'a, str>,
    pub changes_made: u64,
    pub stats: CleaningStats,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Errors produced by fallible cleaning APIs.
pub enum CleaningError {
    /// A Unicode normalization mode was requested without the `unorm` feature.
    NormalizationUnavailable { requested: UnicodeNormalizationMode },
}

impl fmt::Display for CleaningError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CleaningError::NormalizationUnavailable { requested } => write!(
                f,
                "Unicode normalization {:?} requested but the 'unorm' feature is disabled",
                requested
            ),
        }
    }
}

impl std::error::Error for CleaningError {}

impl CleaningStats {
    /// Merge another stats snapshot into this one.
    ///
    /// # Arguments
    /// - `other`: Additional counters to accumulate into `self`.
    #[cfg(feature = "stats")]
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
        self.non_keyboard_transliterated = self
            .non_keyboard_transliterated
            .saturating_add(other.non_keyboard_transliterated);
        self.emojis_dropped = self.emojis_dropped.saturating_add(other.emojis_dropped);
        #[cfg(feature = "security")]
        {
            self.bidi_controls_removed = self
                .bidi_controls_removed
                .saturating_add(other.bidi_controls_removed);
        }
    }

    #[cfg(not(feature = "stats"))]
    #[inline]
    /// No-op stats accumulation when the `stats` feature is disabled.
    ///
    /// # Arguments
    /// - `_`: Ignored stats payload.
    pub fn accumulate(&mut self, _: &CleaningStats) {
        // No-op when stats are disabled.
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CleaningOptions {
    pub remove_hidden: bool,
    pub remove_trailing_whitespace: bool,
    pub normalize_spaces: bool,
    pub normalize_dashes: bool,
    pub normalize_quotes: bool,
    pub normalize_other: bool, // ellipsis (… -> ...), etc.
    pub keyboard_only: bool,
    pub extended_keyboard: bool, // curated non-ASCII allowlist in keyboard mode
    pub emoji_policy: EmojiPolicy, // effective only if keyboard_only = true
    pub non_ascii_policy: NonAsciiPolicy, // effective only if keyboard_only = true
    pub preserve_joiners: bool,  // keep ZWJ/ZWNJ when remove_hidden is enabled
    pub remove_control_chars: bool, // remove Cc excluding \n, \r, \t
    pub collapse_whitespace: bool,
    pub normalize_line_endings: Option<LineEndingStyle>,
    pub unicode_normalization: UnicodeNormalizationMode,
    #[cfg_attr(not(feature = "security"), doc(hidden))]
    pub strip_bidi_controls: bool,
}

#[derive(Debug, Clone)]
/// Builder for [`CleaningOptions`].
pub struct CleaningOptionsBuilder {
    options: CleaningOptions,
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
            keyboard_only: true,
            extended_keyboard: false,
            emoji_policy: EmojiPolicy::Drop,
            non_ascii_policy: NonAsciiPolicy::Transliterate,
            preserve_joiners: false,
            remove_control_chars: true,
            collapse_whitespace: false,
            normalize_line_endings: None,
            unicode_normalization: UnicodeNormalizationMode::None,
            strip_bidi_controls: false,
        }
    }
}

impl CleaningOptions {
    /// Start a new [`CleaningOptionsBuilder`] with default values.
    ///
    /// # Returns
    /// A builder initialized from [`CleaningOptions::default`].
    pub fn builder() -> CleaningOptionsBuilder {
        CleaningOptionsBuilder {
            options: CleaningOptions::default(),
        }
    }

    /// Minimal preset: only removes hidden/invisible chars.
    ///
    /// # Returns
    /// A conservative preset that performs minimal transformations.
    pub fn minimal() -> Self {
        Self {
            remove_hidden: true,
            remove_trailing_whitespace: false,
            normalize_spaces: false,
            normalize_dashes: false,
            normalize_quotes: false,
            normalize_other: false,
            keyboard_only: false,
            extended_keyboard: false,
            emoji_policy: EmojiPolicy::Drop,
            non_ascii_policy: NonAsciiPolicy::Transliterate,
            preserve_joiners: false,
            remove_control_chars: false,
            collapse_whitespace: false,
            normalize_line_endings: None,
            unicode_normalization: UnicodeNormalizationMode::None,
            strip_bidi_controls: false,
        }
    }

    /// Balanced preset for day-to-day text.
    ///
    /// # Returns
    /// A general-purpose preset for normal prose cleanup.
    pub fn balanced() -> Self {
        Self {
            remove_hidden: true,
            remove_trailing_whitespace: true,
            normalize_spaces: true,
            normalize_dashes: true,
            normalize_quotes: true,
            normalize_other: true,
            keyboard_only: false,
            extended_keyboard: false,
            emoji_policy: EmojiPolicy::Drop,
            non_ascii_policy: NonAsciiPolicy::Transliterate,
            preserve_joiners: false,
            remove_control_chars: true,
            unicode_normalization: UnicodeNormalizationMode::NFC,
            collapse_whitespace: false,
            normalize_line_endings: None,
            strip_bidi_controls: false,
        }
    }

    /// Humanize preset for AI/LLM-ish text.
    ///
    /// # Returns
    /// A preset tuned for typographic normalization and whitespace cleanup.
    pub fn humanize() -> Self {
        Self {
            remove_hidden: true,
            remove_trailing_whitespace: true,
            normalize_spaces: true,
            normalize_dashes: true,
            normalize_quotes: true,
            normalize_other: true,
            keyboard_only: false,
            extended_keyboard: false,
            emoji_policy: EmojiPolicy::Drop,
            non_ascii_policy: NonAsciiPolicy::Transliterate,
            preserve_joiners: false,
            remove_control_chars: true,
            unicode_normalization: UnicodeNormalizationMode::NFKC,
            collapse_whitespace: true,
            normalize_line_endings: None,
            strip_bidi_controls: false,
        }
    }

    /// Aggressive preset: maximum cleanup.
    ///
    /// # Returns
    /// A strict preset that targets keyboard-safe output.
    pub fn aggressive() -> Self {
        Self {
            remove_hidden: true,
            remove_trailing_whitespace: true,
            normalize_spaces: true,
            normalize_dashes: true,
            normalize_quotes: true,
            normalize_other: true,
            keyboard_only: true,
            extended_keyboard: false,
            emoji_policy: EmojiPolicy::Drop,
            non_ascii_policy: NonAsciiPolicy::Transliterate,
            preserve_joiners: false,
            remove_control_chars: true,
            collapse_whitespace: true,
            normalize_line_endings: Some(LineEndingStyle::Lf),
            unicode_normalization: UnicodeNormalizationMode::NFKC,
            strip_bidi_controls: true,
        }
    }

    /// Code-safe preset for docs/source-like content.
    ///
    /// # Returns
    /// A preset that preserves semantic punctuation and Unicode glyphs while
    /// still removing hidden/control noise.
    pub fn code_safe() -> Self {
        Self {
            remove_hidden: true,
            remove_trailing_whitespace: true,
            normalize_spaces: true,
            normalize_dashes: false,
            normalize_quotes: false,
            normalize_other: false,
            keyboard_only: false,
            extended_keyboard: false,
            emoji_policy: EmojiPolicy::Keep,
            non_ascii_policy: NonAsciiPolicy::Transliterate,
            preserve_joiners: true,
            remove_control_chars: true,
            collapse_whitespace: false,
            normalize_line_endings: None,
            unicode_normalization: UnicodeNormalizationMode::None,
            strip_bidi_controls: false,
        }
    }
}

impl CleaningOptionsBuilder {
    /// Set `remove_hidden`.
    ///
    /// # Arguments
    /// - `value`: Whether to remove default-ignorable code points.
    ///
    /// # Returns
    /// Updated builder.
    pub fn remove_hidden(mut self, value: bool) -> Self {
        self.options.remove_hidden = value;
        self
    }

    /// Set `remove_trailing_whitespace`.
    ///
    /// # Arguments
    /// - `value`: Whether trailing spaces/tabs are trimmed per line.
    ///
    /// # Returns
    /// Updated builder.
    pub fn remove_trailing_whitespace(mut self, value: bool) -> Self {
        self.options.remove_trailing_whitespace = value;
        self
    }

    /// Set `normalize_spaces`.
    ///
    /// # Arguments
    /// - `value`: Whether Unicode spaces normalize to ASCII space.
    ///
    /// # Returns
    /// Updated builder.
    pub fn normalize_spaces(mut self, value: bool) -> Self {
        self.options.normalize_spaces = value;
        self
    }

    /// Set `normalize_dashes`.
    ///
    /// # Arguments
    /// - `value`: Whether Unicode dash variants normalize to `-`.
    ///
    /// # Returns
    /// Updated builder.
    pub fn normalize_dashes(mut self, value: bool) -> Self {
        self.options.normalize_dashes = value;
        self
    }

    /// Set `normalize_quotes`.
    ///
    /// # Arguments
    /// - `value`: Whether curly/Unicode quotes normalize to ASCII quotes.
    ///
    /// # Returns
    /// Updated builder.
    pub fn normalize_quotes(mut self, value: bool) -> Self {
        self.options.normalize_quotes = value;
        self
    }

    /// Set `normalize_other`.
    ///
    /// # Arguments
    /// - `value`: Whether miscellaneous replacements (for example ellipsis)
    ///   are applied.
    ///
    /// # Returns
    /// Updated builder.
    pub fn normalize_other(mut self, value: bool) -> Self {
        self.options.normalize_other = value;
        self
    }

    /// Set `keyboard_only`.
    ///
    /// # Arguments
    /// - `value`: Whether output is restricted to keyboard-safe characters.
    ///
    /// # Returns
    /// Updated builder.
    pub fn keyboard_only(mut self, value: bool) -> Self {
        self.options.keyboard_only = value;
        self
    }

    /// Set `extended_keyboard`.
    ///
    /// # Arguments
    /// - `value`: Whether curated non-ASCII keyboard characters are allowed.
    ///
    /// # Returns
    /// Updated builder.
    pub fn extended_keyboard(mut self, value: bool) -> Self {
        self.options.extended_keyboard = value;
        self
    }

    /// Set `emoji_policy`.
    ///
    /// # Arguments
    /// - `policy`: Emoji handling policy when `keyboard_only` is enabled.
    ///
    /// # Returns
    /// Updated builder.
    pub fn emoji_policy(mut self, policy: EmojiPolicy) -> Self {
        self.options.emoji_policy = policy;
        self
    }

    /// Set `non_ascii_policy`.
    ///
    /// # Arguments
    /// - `policy`: Non-ASCII handling strategy in keyboard-only mode.
    ///
    /// # Returns
    /// Updated builder.
    pub fn non_ascii_policy(mut self, policy: NonAsciiPolicy) -> Self {
        self.options.non_ascii_policy = policy;
        self
    }

    /// Set `preserve_joiners`.
    ///
    /// # Arguments
    /// - `value`: Whether ZWJ/ZWNJ are preserved when `remove_hidden` is enabled.
    ///
    /// # Returns
    /// Updated builder.
    pub fn preserve_joiners(mut self, value: bool) -> Self {
        self.options.preserve_joiners = value;
        self
    }

    /// Set `remove_control_chars`.
    ///
    /// # Arguments
    /// - `value`: Whether control characters are removed.
    ///
    /// # Returns
    /// Updated builder.
    pub fn remove_control_chars(mut self, value: bool) -> Self {
        self.options.remove_control_chars = value;
        self
    }

    /// Set `collapse_whitespace`.
    ///
    /// # Arguments
    /// - `value`: Whether consecutive whitespace runs collapse to single spaces.
    ///
    /// # Returns
    /// Updated builder.
    pub fn collapse_whitespace(mut self, value: bool) -> Self {
        self.options.collapse_whitespace = value;
        self
    }

    /// Set `normalize_line_endings`.
    ///
    /// # Arguments
    /// - `value`: Optional target line-ending style.
    ///
    /// # Returns
    /// Updated builder.
    pub fn normalize_line_endings(mut self, value: Option<LineEndingStyle>) -> Self {
        self.options.normalize_line_endings = value;
        self
    }

    /// Set `unicode_normalization`.
    ///
    /// # Arguments
    /// - `mode`: Unicode normalization mode to apply before cleaning.
    ///
    /// # Returns
    /// Updated builder.
    pub fn unicode_normalization(mut self, mode: UnicodeNormalizationMode) -> Self {
        self.options.unicode_normalization = mode;
        self
    }

    #[cfg_attr(not(feature = "security"), doc(hidden))]
    /// Set `strip_bidi_controls`.
    ///
    /// # Arguments
    /// - `value`: Whether bidirectional controls are removed.
    ///
    /// # Returns
    /// Updated builder.
    pub fn strip_bidi_controls(mut self, value: bool) -> Self {
        self.options.strip_bidi_controls = value;
        self
    }

    /// Build an immutable [`CleaningOptions`] value.
    ///
    /// # Returns
    /// Finalized options struct.
    pub fn build(self) -> CleaningOptions {
        self.options
    }
}

/// Main cleaner.
pub struct TextCleaner {
    options: CleaningOptions,
}

impl TextCleaner {
    /// Create a cleaner from explicit options.
    ///
    /// # Arguments
    /// - `options`: Cleaning behavior configuration.
    ///
    /// # Returns
    /// A reusable [`TextCleaner`].
    pub fn new(options: CleaningOptions) -> Self {
        Self { options }
    }

    /// Borrow the cleaner options.
    ///
    /// # Returns
    /// Immutable reference to configured [`CleaningOptions`].
    pub fn options(&self) -> &CleaningOptions {
        &self.options
    }

    /// Clean text and panic on unavailable normalization features.
    ///
    /// # Arguments
    /// - `text`: Input text to normalize.
    ///
    /// # Returns
    /// Cleaned output and stats.
    ///
    /// # Errors
    /// This infallible wrapper does not return errors; use
    /// [`TextCleaner::try_clean`] for error handling.
    ///
    /// # Panics
    /// Panics when a normalization mode requires the `unorm` feature but it is
    /// not enabled.
    pub fn clean<'a>(&self, text: &'a str) -> CleaningResult<'a> {
        self.try_clean(text).unwrap_or_else(|err| {
            panic!(
                "clean() failed: {err}. Enable the 'unorm' feature or call try_clean() to handle the error"
            )
        })
    }

    /// Clean text into a caller-provided buffer and panic on unavailable
    /// normalization features.
    ///
    /// # Arguments
    /// - `text`: Input text to normalize.
    /// - `out`: Output buffer to reuse.
    ///
    /// # Returns
    /// A result borrowing from `out`.
    ///
    /// # Errors
    /// This infallible wrapper does not return errors; use
    /// [`TextCleaner::try_clean_into`] for error handling.
    ///
    /// # Panics
    /// Panics when a normalization mode requires the `unorm` feature but it is
    /// not enabled.
    pub fn clean_into<'output>(
        &self,
        text: &str,
        out: &'output mut String,
    ) -> CleaningResult<'output> {
        self.try_clean_into(text, out).unwrap_or_else(|err| {
            panic!(
                "clean_into() failed: {err}. Enable the 'unorm' feature or call try_clean_into() to handle the error"
            )
        })
    }

    /// Fallible variant of [`TextCleaner::clean`].
    ///
    /// # Arguments
    /// - `text`: Input text to normalize.
    ///
    /// # Returns
    /// Cleaned output and stats.
    ///
    /// # Errors
    /// Returns [`CleaningError::NormalizationUnavailable`] when normalization
    /// was requested without the `unorm` feature.
    pub fn try_clean<'a>(&self, text: &'a str) -> Result<CleaningResult<'a>, CleaningError> {
        self.try_clean_with_context(text, false)
    }

    /// Fallible variant of [`TextCleaner::clean_into`].
    ///
    /// # Arguments
    /// - `text`: Input text to normalize.
    /// - `out`: Output buffer to reuse.
    ///
    /// # Returns
    /// A result borrowing from `out`.
    ///
    /// # Errors
    /// Returns [`CleaningError::NormalizationUnavailable`] when normalization
    /// was requested without the `unorm` feature.
    pub fn try_clean_into<'output>(
        &self,
        text: &str,
        out: &'output mut String,
    ) -> Result<CleaningResult<'output>, CleaningError> {
        self.try_clean_into_with_context(text, out, false)
    }

    /// Clean text while preserving context about previously emitted output.
    ///
    /// # Arguments
    /// - `text`: Input chunk to clean.
    /// - `has_prior_output`: Whether earlier chunks already emitted output.
    ///
    /// # Returns
    /// Cleaned output and stats.
    ///
    /// # Errors
    /// Returns [`CleaningError::NormalizationUnavailable`] when normalization
    /// was requested without the `unorm` feature.
    pub fn try_clean_with_context<'a>(
        &self,
        text: &'a str,
        has_prior_output: bool,
    ) -> Result<CleaningResult<'a>, CleaningError> {
        if text.is_empty() {
            return Ok(CleaningResult {
                text: Cow::Borrowed(text),
                changes_made: 0,
                stats: CleaningStats::default(),
            });
        }

        if self.can_use_ascii_fast_path(text) {
            return Ok(CleaningResult {
                text: Cow::Borrowed(text),
                changes_made: 0,
                stats: CleaningStats::default(),
            });
        }

        let working = self.normalize_input(text)?;
        let mut buffer = String::with_capacity(working.len());
        let (changes, stats) = self.clean_into_internal(working, &mut buffer, has_prior_output);
        Ok(CleaningResult {
            text: Cow::Owned(buffer),
            changes_made: changes,
            stats,
        })
    }

    /// Buffer-reusing context-aware cleaner.
    ///
    /// # Arguments
    /// - `text`: Input chunk to clean.
    /// - `out`: Output buffer to reuse.
    /// - `has_prior_output`: Whether earlier chunks already emitted output.
    ///
    /// # Returns
    /// A result borrowing from `out`.
    ///
    /// # Errors
    /// Returns [`CleaningError::NormalizationUnavailable`] when normalization
    /// was requested without the `unorm` feature.
    pub fn try_clean_into_with_context<'output>(
        &self,
        text: &str,
        out: &'output mut String,
        has_prior_output: bool,
    ) -> Result<CleaningResult<'output>, CleaningError> {
        out.clear();

        if text.is_empty() {
            return Ok(CleaningResult {
                text: Cow::Borrowed(out.as_str()),
                changes_made: 0,
                stats: CleaningStats::default(),
            });
        }

        if self.can_use_ascii_fast_path(text) {
            out.push_str(text);
            return Ok(CleaningResult {
                text: Cow::Borrowed(out.as_str()),
                changes_made: 0,
                stats: CleaningStats::default(),
            });
        }

        let working = self.normalize_input(text)?;
        let (changes, stats) = self.clean_into_internal(working, out, has_prior_output);
        Ok(CleaningResult {
            text: Cow::Borrowed(out.as_str()),
            changes_made: changes,
            stats,
        })
    }

    fn clean_into_internal(
        &self,
        working_input: Cow<'_, str>,
        out: &mut String,
        has_prior_output: bool,
    ) -> (u64, CleaningStats) {
        // Without the `stats` feature, record_stat! never mutates the struct.
        #[cfg_attr(not(feature = "stats"), allow(unused_mut))]
        let mut stats = CleaningStats::default();
        let mut changes = 0u64;

        let mut working = working_input;

        let mut line_ending_conversions = LineEndingCounts::default();
        if self.options.normalize_line_endings.is_some() {
            let (lf, counts) = to_lf(working.as_ref());
            line_ending_conversions = counts;
            working = Cow::Owned(lf);
        }

        out.clear();
        out.reserve(working.len());

        let mut pending_ws: usize = 0;
        let mut cap_next_whitespace = false;
        let mut drop_leading_whitespace = false;
        let mut emitted_anything = has_prior_output;
        let trim = self.options.remove_trailing_whitespace;
        let collapse = self.options.collapse_whitespace;

        let mut emoji_classifier: Option<EmojiClassifier> = None;
        let mut cluster_buffer = String::new();
        #[cfg(feature = "security")]
        let bidi_controls: Option<CodePointSetDataBorrowed<'static>> =
            if self.options.strip_bidi_controls {
                Some(CodePointSetData::new::<props::BidiControl>())
            } else {
                None
            };

        for grapheme in UnicodeSegmentation::graphemes(working.as_ref(), true) {
            if grapheme.is_empty() {
                continue;
            }

            if is_newline_grapheme(grapheme) {
                if trim {
                    if pending_ws > 0 {
                        record_change!(changes, stats, trailing_whitespace_removed, pending_ws);
                        pending_ws = 0;
                        cap_next_whitespace = false;
                    }
                } else {
                    flush_pending_whitespace(out, pending_ws, collapse);
                    pending_ws = 0;
                    cap_next_whitespace = false;
                }
                out.push_str(grapheme);
                emitted_anything = true;
                drop_leading_whitespace = false;
                continue;
            }

            // U+2028/U+2029 LINE/PARAGRAPH SEPARATOR: when line-ending
            // normalization is on, `to_lf` already folded them to `\n` above.
            // Otherwise fold them here as part of space normalization — mirroring
            // the newline branch so surrounding whitespace is trimmed identically —
            // instead of passing them through (or dropping them in keyboard mode).
            if self.options.normalize_spaces
                && self.options.normalize_line_endings.is_none()
                && matches!(grapheme, "\u{2028}" | "\u{2029}")
            {
                if trim {
                    if pending_ws > 0 {
                        record_change!(changes, stats, trailing_whitespace_removed, pending_ws);
                        pending_ws = 0;
                        cap_next_whitespace = false;
                    }
                } else {
                    flush_pending_whitespace(out, pending_ws, collapse);
                    pending_ws = 0;
                    cap_next_whitespace = false;
                }
                out.push('\n');
                record_change!(changes, stats, spaces_normalized);
                emitted_anything = true;
                drop_leading_whitespace = false;
                continue;
            }

            let mut emoji_cluster_cache: Option<bool> = None;
            let mut ensure_emoji_cluster = |classifier: &mut Option<EmojiClassifier>| -> bool {
                if let Some(value) = emoji_cluster_cache {
                    return value;
                }
                if grapheme.is_ascii() {
                    emoji_cluster_cache = Some(false);
                    return false;
                }
                if !self.options.keyboard_only && !self.options.remove_hidden {
                    emoji_cluster_cache = Some(false);
                    return false;
                }
                let classifier = classifier.get_or_insert_with(EmojiClassifier::new);
                let value = classify_emoji_cluster(grapheme, classifier).is_rendered;
                emoji_cluster_cache = Some(value);
                value
            };

            cluster_buffer.clear();
            cluster_buffer.reserve(grapheme.len());
            let mut emitted_directly = false;
            // Per-grapheme tally of interior default-ignorable chars (VS16/ZWJ) we
            // remove. Committed to `hidden_chars_removed` only if the cluster is
            // emitted; discarded if the whole cluster is dropped as an emoji, so a
            // dropped emoji is billed once (emojis_dropped) rather than twice.
            let mut cluster_hidden_removed = 0u64;

            for mut c in grapheme.chars() {
                #[cfg(feature = "security")]
                if let Some(set) = bidi_controls {
                    if set.contains(c) {
                        record_change!(changes, stats, bidi_controls_removed);
                        continue;
                    }
                }

                if self.options.remove_hidden && is_hidden_char(c) {
                    let keep_hidden = (self.options.preserve_joiners && is_joiner(c))
                        || ((!self.options.keyboard_only
                            || matches!(self.options.emoji_policy, EmojiPolicy::Keep))
                            && ensure_emoji_cluster(&mut emoji_classifier));
                    if keep_hidden {
                        cluster_buffer.push(c);
                    } else {
                        record_change!(changes, stats, hidden_chars_removed);
                        cluster_hidden_removed = cluster_hidden_removed.saturating_add(1);
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
                            if pending_ws > 0 {
                                if drop_leading_whitespace && !emitted_anything {
                                    pending_ws = 0;
                                } else {
                                    flush_pending_whitespace(out, pending_ws, collapse);
                                    pending_ws = 0;
                                }
                            }
                            out.push_str("...");
                            emitted_anything = true;
                            drop_leading_whitespace = false;
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
                let count = cluster_buffer.chars().count();
                if cap_next_whitespace {
                    pending_ws = 1;
                    cap_next_whitespace = false;
                } else {
                    pending_ws = pending_ws.saturating_add(count);
                }
                continue;
            }

            if self.options.keyboard_only {
                let is_emoji_cluster = ensure_emoji_cluster(&mut emoji_classifier);
                if is_emoji_cluster && matches!(self.options.emoji_policy, EmojiPolicy::Keep) {
                    if pending_ws > 0 {
                        if drop_leading_whitespace && !emitted_anything {
                            pending_ws = 0;
                        } else {
                            flush_pending_whitespace(out, pending_ws, collapse);
                            pending_ws = 0;
                        }
                    }
                    out.push_str(&cluster_buffer);
                    emitted_anything = true;
                    drop_leading_whitespace = false;
                    continue;
                }

                if let Some(rewrite) = rewrite_cluster_to_keyboard_ascii(
                    &cluster_buffer,
                    self.options.non_ascii_policy,
                    self.options.extended_keyboard,
                ) {
                    if rewrite.non_ascii_transliterated > 0 {
                        record_change!(
                            changes,
                            stats,
                            non_keyboard_transliterated,
                            rewrite.non_ascii_transliterated
                        );
                    }
                    if rewrite.non_ascii_removed > 0 {
                        record_change!(
                            changes,
                            stats,
                            non_keyboard_removed,
                            rewrite.non_ascii_removed
                        );
                    }
                    cluster_buffer = rewrite.text;

                    if pending_ws > 0 {
                        if drop_leading_whitespace && !emitted_anything {
                            pending_ws = 0;
                        } else {
                            flush_pending_whitespace(out, pending_ws, collapse);
                            pending_ws = 0;
                        }
                    }
                    out.push_str(&cluster_buffer);
                    emitted_anything = true;
                    drop_leading_whitespace = false;
                } else if is_emoji_cluster {
                    // The emoji cluster is dropped as a unit; its interior VS16/ZWJ
                    // removals are part of that single drop, not separate hidden
                    // removals. Roll them back so changes_made bills the cluster once.
                    if cluster_hidden_removed > 0 {
                        changes = changes.saturating_sub(cluster_hidden_removed);
                        #[cfg(feature = "stats")]
                        {
                            stats.hidden_chars_removed = stats
                                .hidden_chars_removed
                                .saturating_sub(cluster_hidden_removed);
                        }
                    }
                    record_change!(changes, stats, emojis_dropped);
                    cluster_buffer.clear();
                    cap_next_whitespace = true;
                    drop_leading_whitespace = pending_ws == 0 && !emitted_anything;
                    if pending_ws > 0 {
                        pending_ws = 1;
                    }
                } else {
                    let removed = cluster_buffer
                        .chars()
                        .filter(|c| !is_keyboard_allowed(*c, self.options.extended_keyboard))
                        .count();
                    if removed > 0 {
                        record_change!(changes, stats, non_keyboard_removed, removed);
                    }
                    cluster_buffer.clear();
                    cap_next_whitespace = true;
                    drop_leading_whitespace = pending_ws == 0 && !emitted_anything;
                    if pending_ws > 0 {
                        pending_ws = 1;
                    }
                }
            } else {
                if pending_ws > 0 {
                    if drop_leading_whitespace && !emitted_anything {
                        pending_ws = 0;
                    } else {
                        flush_pending_whitespace(out, pending_ws, collapse);
                        pending_ws = 0;
                    }
                }
                out.push_str(&cluster_buffer);
                emitted_anything = true;
                drop_leading_whitespace = false;
            }
        }

        if trim {
            if pending_ws > 0 {
                record_change!(changes, stats, trailing_whitespace_removed, pending_ws);
            }
        } else if pending_ws > 0 {
            if drop_leading_whitespace && !emitted_anything {
                // drop leading whitespace that only existed due to removed clusters
            } else {
                flush_pending_whitespace(out, pending_ws, collapse);
            }
        }

        match self.options.normalize_line_endings {
            Some(LineEndingStyle::Lf) => {
                let total = line_ending_conversions.total();
                if total > 0 {
                    record_change!(changes, stats, line_endings_normalized, total);
                }
            }
            Some(style @ (LineEndingStyle::Crlf | LineEndingStyle::Cr)) => {
                let restamp_changes = restamp_line_endings_mut(style, out);
                let baseline = match style {
                    LineEndingStyle::Crlf => line_ending_conversions.crlf,
                    LineEndingStyle::Cr => line_ending_conversions.cr,
                    LineEndingStyle::Lf => unreachable!(),
                };
                let net = restamp_changes.saturating_sub(baseline);
                if net > 0 {
                    record_change!(changes, stats, line_endings_normalized, net);
                }
            }
            None => {}
        }

        (changes, stats)
    }

    fn normalize_input<'a>(&self, text: &'a str) -> Result<Cow<'a, str>, CleaningError> {
        match self.options.unicode_normalization {
            UnicodeNormalizationMode::None => Ok(Cow::Borrowed(text)),
            #[cfg(feature = "unorm")]
            UnicodeNormalizationMode::NFD => Ok(Cow::Owned(text.nfd().collect())),
            #[cfg(feature = "unorm")]
            UnicodeNormalizationMode::NFC => Ok(Cow::Owned(text.nfc().collect())),
            #[cfg(feature = "unorm")]
            UnicodeNormalizationMode::NFKD => Ok(Cow::Owned(text.nfkd().collect())),
            #[cfg(feature = "unorm")]
            UnicodeNormalizationMode::NFKC => Ok(Cow::Owned(text.nfkc().collect())),
            #[cfg(not(feature = "unorm"))]
            mode => Err(CleaningError::NormalizationUnavailable { requested: mode }),
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
            // keyboard_only strips ASCII control chars (except \n \r \t) regardless
            // of remove_control_chars; the fast path must not silently keep them.
            && (!self.options.keyboard_only || text.bytes().all(is_fast_path_safe_ascii_byte))
    }
}

#[derive(Debug, Clone)]
/// Summary of cumulative streaming cleanup work.
pub struct StreamSummary {
    /// Aggregated counters over all emitted chunks.
    pub stats: CleaningStats,
    /// Total transformations across all emitted chunks.
    pub changes_made: u64,
}

/// Characters that end a flushable chunk: LF plus U+2028 LINE SEPARATOR and
/// U+2029 PARAGRAPH SEPARATOR, which batch cleaning folds to `\n` under
/// `normalize_spaces`. All three are hard grapheme-cluster breaks, so a chunk
/// split after any of them cleans identically to the unsplit text.
const STREAM_FLUSH_BOUNDARIES: [char; 3] = ['\n', '\u{2028}', '\u{2029}'];

/// Incremental cleaner that processes text in line-delimited chunks.
pub struct StreamCleaner {
    cleaner: TextCleaner,
    buffer: String,
    total_stats: CleaningStats,
    total_changes: u64,
    has_emitted_output: bool,
}

impl StreamCleaner {
    /// Construct a streaming cleaner from options.
    ///
    /// # Arguments
    /// - `options`: Cleaning behavior configuration.
    ///
    /// # Returns
    /// A new [`StreamCleaner`].
    pub fn new(options: CleaningOptions) -> Self {
        Self {
            cleaner: TextCleaner::new(options),
            buffer: String::new(),
            total_stats: CleaningStats::default(),
            total_changes: 0,
            has_emitted_output: false,
        }
    }

    /// Construct a streaming cleaner from an existing [`TextCleaner`].
    ///
    /// # Arguments
    /// - `cleaner`: Preconfigured text cleaner.
    ///
    /// # Returns
    /// A new [`StreamCleaner`].
    pub fn from_cleaner(cleaner: TextCleaner) -> Self {
        Self {
            cleaner,
            buffer: String::new(),
            total_stats: CleaningStats::default(),
            total_changes: 0,
            has_emitted_output: false,
        }
    }

    /// Feed one input chunk and emit cleaned output only after a line
    /// boundary (LF, U+2028 LINE SEPARATOR, or U+2029 PARAGRAPH SEPARATOR)
    /// is available.
    ///
    /// # Arguments
    /// - `chunk`: Incoming text data.
    /// - `out`: Reusable output buffer.
    ///
    /// # Returns
    /// `Some(CleaningResult)` when at least one complete line was processed,
    /// otherwise `None`.
    ///
    /// # Panics
    /// Panics when normalization is requested but the `unorm` feature is disabled.
    pub fn feed<'out>(
        &mut self,
        chunk: &str,
        out: &'out mut String,
    ) -> Option<CleaningResult<'out>> {
        out.clear();
        if chunk.is_empty() {
            return None;
        }
        self.buffer.push_str(chunk);
        let last_boundary = self.buffer.rfind(STREAM_FLUSH_BOUNDARIES)?;
        let boundary_len = self.buffer[last_boundary..]
            .chars()
            .next()
            .expect("rfind returned the start of a char")
            .len_utf8();
        let flush_end = last_boundary + boundary_len;
        let to_process = self.buffer[..flush_end].to_owned();
        self.buffer.drain(..flush_end);
        Some(self.process_owned_chunk(to_process, out))
    }

    /// Flush remaining buffered text at end-of-stream.
    ///
    /// # Arguments
    /// - `out`: Reusable output buffer.
    ///
    /// # Returns
    /// Final cleaned chunk when buffered content remains, otherwise `None`.
    pub fn finish<'out>(&mut self, out: &'out mut String) -> Option<CleaningResult<'out>> {
        out.clear();
        if self.buffer.is_empty() {
            return None;
        }
        let remainder = std::mem::take(&mut self.buffer);
        Some(self.process_owned_chunk(remainder, out))
    }

    /// Return cumulative stream statistics.
    ///
    /// # Returns
    /// Aggregate counters and total change count.
    pub fn summary(&self) -> StreamSummary {
        StreamSummary {
            stats: self.total_stats.clone(),
            changes_made: self.total_changes,
        }
    }

    fn process_owned_chunk<'out>(
        &mut self,
        chunk: String,
        out: &'out mut String,
    ) -> CleaningResult<'out> {
        let result = self
            .cleaner
            .try_clean_into_with_context(&chunk, out, self.has_emitted_output)
            .unwrap_or_else(|err| {
                panic!(
                    "StreamCleaner::feed failed: {err}. Enable the 'unorm' feature or use try_clean"
                )
            });
        let emitted = result.text.as_ref();

        self.total_stats.accumulate(&result.stats);
        self.total_changes = self.total_changes.saturating_add(result.changes_made);
        if !emitted.is_empty() {
            self.has_emitted_output = true;
        }

        result
    }
}

#[derive(Clone, Copy)]
struct EmojiClusterContext {
    is_rendered: bool,
}

struct EmojiClassifier {
    emoji: CodePointSetDataBorrowed<'static>,
    emoji_presentation: CodePointSetDataBorrowed<'static>,
    extended_pictographic: CodePointSetDataBorrowed<'static>,
}

impl EmojiClassifier {
    fn new() -> Self {
        Self {
            emoji: CodePointSetData::new::<props::Emoji>(),
            emoji_presentation: CodePointSetData::new::<props::EmojiPresentation>(),
            extended_pictographic: CodePointSetData::new::<props::ExtendedPictographic>(),
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

fn is_joiner(c: char) -> bool {
    matches!(c, '\u{200C}' | '\u{200D}')
}

fn is_keyboard_allowed(c: char, extended_keyboard: bool) -> bool {
    is_keyboard_ascii(c) || (extended_keyboard && is_extended_keyboard_char(c))
}

/// ASCII byte that `keyboard_only` leaves unchanged, so an all-ASCII input is
/// safe to return verbatim via the fast path: printable ASCII plus the three
/// retained whitespace controls. Excludes C0 controls and DEL (0x7F).
fn is_fast_path_safe_ascii_byte(b: u8) -> bool {
    matches!(b, b'\n' | b'\r' | b'\t') || (0x20..0x7F).contains(&b)
}

#[derive(Debug, Default, Clone, Copy)]
struct LineEndingCounts {
    crlf: u64,
    cr: u64,
    nel: u64,
    ls: u64,
    ps: u64,
}

impl LineEndingCounts {
    fn total(&self) -> u64 {
        self.crlf + self.cr + self.nel + self.ls + self.ps
    }
}

// ----------------- helpers -----------------

fn to_lf(s: &str) -> (String, LineEndingCounts) {
    // Convert CRLF, CR, NEL (U+0085), LS (U+2028) and PS (U+2029) to LF and track conversions.
    let mut out = String::with_capacity(s.len());
    let mut counts = LineEndingCounts::default();
    let mut it = s.chars().peekable();
    while let Some(c) = it.next() {
        if c == '\r' {
            if matches!(it.peek(), Some('\n')) {
                it.next(); // consume LF
                counts.crlf = counts.crlf.saturating_add(1);
            } else {
                counts.cr = counts.cr.saturating_add(1);
            }
            out.push('\n');
        } else if c == '\u{0085}' {
            out.push('\n');
            counts.nel = counts.nel.saturating_add(1);
        } else if c == '\u{2028}' {
            out.push('\n');
            counts.ls = counts.ls.saturating_add(1);
        } else if c == '\u{2029}' {
            out.push('\n');
            counts.ps = counts.ps.saturating_add(1);
        } else {
            out.push(c);
        }
    }
    (out, counts)
}

fn restamp_line_endings_mut(style: LineEndingStyle, text: &mut String) -> u64 {
    match style {
        LineEndingStyle::Lf => 0,
        LineEndingStyle::Crlf => {
            let lf_count = text.as_bytes().iter().filter(|&&b| b == b'\n').count() as u64;
            if lf_count > 0 {
                let restamped = text.replace('\n', "\r\n");
                *text = restamped;
            }
            lf_count
        }
        LineEndingStyle::Cr => {
            let lf_count = text.as_bytes().iter().filter(|&&b| b == b'\n').count() as u64;
            if lf_count > 0 {
                let restamped = text.replace('\n', "\r");
                *text = restamped;
            }
            lf_count
        }
    }
}

fn map_dash(c: char) -> Option<char> {
    DASH_MAP.get(&c).copied()
}

fn map_quote(c: char) -> Option<char> {
    QUOTE_MAP.get(&c).copied()
}

#[derive(Debug)]
struct KeyboardAsciiRewrite {
    text: String,
    non_ascii_transliterated: u64,
    non_ascii_removed: u64,
}

fn rewrite_cluster_to_keyboard_ascii(
    cluster: &str,
    policy: NonAsciiPolicy,
    extended_keyboard: bool,
) -> Option<KeyboardAsciiRewrite> {
    let mut out = String::with_capacity(cluster.len());
    let mut non_ascii_transliterated = 0u64;
    let mut non_ascii_removed = 0u64;

    for c in cluster.chars() {
        if is_keyboard_allowed(c, extended_keyboard) {
            out.push(c);
            continue;
        }

        // Precedence, most-specific first:
        //   1. compat_override  — meaning-preserving ASCII for chars whose NFKD
        //      would silently invert sense (`≠` -> `=`). Runs in BOTH Fold and
        //      Transliterate, *before* NFKD, so the negation is never lost.
        //   2. NFKD compatibility fold — `½`->`1/2`, `™`->`TM`, fullwidth, etc.
        //   3. curated symbol map + scoped deunicode — Transliterate only.
        //   4. drop.
        let mapped = match policy {
            NonAsciiPolicy::Drop => false,
            NonAsciiPolicy::Fold => {
                append_mapping(compat_override(c), &mut out, extended_keyboard)
                    || append_folded_non_ascii(c, &mut out, extended_keyboard)
            }
            NonAsciiPolicy::Transliterate => {
                append_mapping(compat_override(c), &mut out, extended_keyboard)
                    || append_folded_non_ascii(c, &mut out, extended_keyboard)
                    || append_transliterated_non_ascii(c, &mut out, extended_keyboard)
            }
        };

        if mapped {
            non_ascii_transliterated = non_ascii_transliterated.saturating_add(1);
        } else {
            non_ascii_removed = non_ascii_removed.saturating_add(1);
        }
    }

    if out.is_empty() {
        None
    } else {
        Some(KeyboardAsciiRewrite {
            text: out,
            non_ascii_transliterated,
            non_ascii_removed,
        })
    }
}

#[cfg(feature = "unorm")]
fn append_folded_non_ascii(c: char, out: &mut String, extended_keyboard: bool) -> bool {
    let mut added = false;
    let source_is_space = c.is_whitespace();
    for decomposed in c.to_string().nfkd() {
        // Spacing-modifier diacritics (´ ¨ ¯ ¸ ˜ …) decompose to <space + combining
        // mark>. Emitting that leading space turns the glyph into whitespace, which
        // both mistranslates it and breaks idempotence (the space later escapes
        // trailing-whitespace trimming). Suppress decomposition spaces unless the
        // source character was itself whitespace (e.g. NBSP -> space is correct).
        if decomposed == ' ' && !source_is_space {
            continue;
        }
        if is_keyboard_allowed(decomposed, extended_keyboard) {
            out.push(decomposed);
            added = true;
        } else if let Some(mapped) = map_compatibility_ascii(decomposed) {
            out.push(mapped);
            added = true;
        }
    }
    added
}

#[cfg(not(feature = "unorm"))]
fn append_folded_non_ascii(c: char, out: &mut String, _: bool) -> bool {
    if let Some(mapped) = map_compatibility_ascii(c) {
        out.push(mapped);
        true
    } else {
        false
    }
}

fn append_transliterated_non_ascii(c: char, out: &mut String, extended_keyboard: bool) -> bool {
    // Curated, high-quality ASCII first: Latin letters that must be spelled out
    // (ß -> ss), the symbol glyphs LLMs emit constantly (-> for arrows, etc.),
    // and Greek letters used as math symbols (lambda, Delta, ...).
    if let Some(mapping) = transliteration_override(c)
        .or_else(|| symbol_translit(c))
        .or_else(|| greek_translit(c))
    {
        return append_mapping(Some(mapping), out, extended_keyboard);
    }

    // Long-tail fallback via deunicode, scoped by Unicode properties: Latin
    // script, or script-neutral symbols/punctuation. The scope is deliberate:
    // deunicode romanizes scripts (世 -> "Shi "), which we do NOT want, so
    // letters of concrete scripts are never passed to it and continue to drop.
    // Emoji-property chars are excluded for the same reason: deunicode expands
    // them to English names (✂ -> "scissors"), which is worse than dropping them
    // with the rest of the emoji. Curated entries above still win for the emoji
    // marks we do want (© ® ✓ …). The output is trimmed because deunicode pads
    // some mappings with spaces that would escape trailing-whitespace trimming.
    if is_latin_script(c) || (is_common_symbol_or_punctuation(c) && !is_emoji(c)) {
        if let Some(mapped) = deunicode_char(c) {
            return append_mapping(Some(mapped.trim()), out, extended_keyboard);
        }
    }
    false
}

/// Apply an optional ASCII mapping, returning whether anything was emitted.
/// Every char is funneled through [`append_ascii_mapping`], which discards any
/// non-keyboard output, so a table entry can never break the keyboard-only ASCII
/// invariant even if it is wrong.
fn append_mapping(mapping: Option<&str>, out: &mut String, extended_keyboard: bool) -> bool {
    match mapping {
        Some(text) => {
            let before = out.len();
            append_ascii_mapping(text, out, extended_keyboard);
            out.len() > before
        }
        None => false,
    }
}

/// Greek letters spell out to their English names under `Transliterate`. Greek
/// is the one deliberate exception to the "letter scripts drop" rule: LLM
/// output overwhelmingly uses Greek letters as math/stats symbols ("lambda =
/// 0.5", "Delta x"), where silent deletion destroys meaning. The cost is that
/// actual Greek-language prose becomes concatenated letter names; that trade
/// was made consciously. Other scripts (CJK, Cyrillic, Arabic, ...) still drop.
///
/// The table is generated in `build.rs` from the 24 base letter names: every
/// Greek-script character whose NFKD form reduces to a base letter (accented,
/// polytonic, final/lunate sigma, math symbol variants) maps to that name.
fn greek_translit(c: char) -> Option<&'static str> {
    GREEK_MAP.get(&c).copied()
}

/// Meaning-preserving ASCII for characters whose NFKD decomposition would
/// otherwise *invert* their sense: negated relational operators decompose to the
/// un-negated base plus U+0338 COMBINING LONG SOLIDUS OVERLAY, the overlay is then
/// dropped, and `≠` becomes `=`. Applied before NFKD in every fold/translit mode.
/// Only the three operators whose base is itself ASCII can actually invert today;
/// they are the mandatory members. See `negated_operators_are_not_inverted`.
fn compat_override(c: char) -> Option<&'static str> {
    Some(match c {
        '\u{2260}' => "!=", // ≠ NOT EQUAL TO     (NFKD -> "=")
        '\u{226E}' => "!<", // ≮ NOT LESS-THAN    (NFKD -> "<")
        '\u{226F}' => "!>", // ≯ NOT GREATER-THAN (NFKD -> ">")
        _ => return None,
    })
}

/// Curated symbol -> ASCII overrides for glyphs where the automatic layers get
/// it wrong. This table is deliberately minimal; most symbols are handled
/// without it. Do NOT add an entry unless BOTH automatic layers fail:
///
/// - NFKD runs first (`™`->`TM`, `½`->`1/2`) — an entry it covers is dead code;
/// - the `deunicode` fallback runs after — an entry returning the same string
///   deunicode already gives is redundant (verify with `deunicode_char`).
///
/// Legitimate reasons to be here: deunicode is lossy for the char (`→`->`-`,
/// `≔`->`=`, `£`->`PS`), or the char is Emoji-classified so the fallback never
/// sees it (`©`, `✅`) yet its meaning is worth keeping.
fn symbol_translit(c: char) -> Option<&'static str> {
    Some(match c {
        // --- Arrows: deunicode collapses direction/shaft (`→`->`-`, `⇒`->`=`).
        //     Double arrows use `==>` (not `=>`) so they never collide with the
        //     `<=`/`>=` output of the relational operators. ---
        '\u{2192}' => "->",
        '\u{2190}' => "<-",
        '\u{2194}' => "<->",
        '\u{2191}' => "^",
        '\u{2193}' => "v",
        '\u{2195}' => "^v",
        '\u{21D2}' => "==>",
        '\u{21D0}' => "<==",
        '\u{21D4}' => "<==>",
        '\u{21A6}' => "|->",
        '\u{21B5}' => "<-'",
        '\u{23CE}' => "<-'",
        '\u{27F6}' => "-->",
        '\u{27F5}' => "<--",
        '\u{27F7}' => "<-->",
        '\u{27F9}' => "==>",
        '\u{27F8}' => "<==",
        '\u{27FA}' => "<==>",
        '\u{2794}' => "->",
        '\u{2799}' => "->",
        '\u{279C}' => "->",
        '\u{27A4}' => "->",
        '\u{21CC}' => "<=>", // chemical equilibrium (deunicode gives bare "=")
        '\u{21CB}' => "<=>",
        // --- Math operators deunicode flattens to a bare sign or single letter ---
        '\u{2254}' => ":=", // deunicode drops the colon ("=")
        '\u{2255}' => "=:",
        '\u{2218}' => "o",   // function composition (deunicode gives "*")
        '\u{22EE}' => "...", // vertical ellipsis (deunicode gives "|")
        '\u{2243}' => "~=",
        '\u{2248}' => "~=",
        '\u{2261}' => "===",
        '\u{00B1}' => "+/-", // deunicode gives "+-"
        '\u{2213}' => "-/+",
        '\u{2211}' => "sum", // deunicode gives "S"
        '\u{220F}' => "prod",
        '\u{222B}' => "int",
        '\u{2207}' => "grad",
        '\u{2206}' => "delta",
        '\u{2205}' => "{}",
        '\u{2208}' => "in",
        '\u{220B}' => "ni",
        '\u{2227}' => "and",
        '\u{2228}' => "or",
        '\u{2200}' => "forall",
        '\u{2203}' => "exists",
        // --- Negated operators: deunicode strips the negation entirely ---
        '\u{2270}' => "!<=",
        '\u{2271}' => "!>=",
        '\u{2209}' => "!in",
        '\u{220C}' => "!ni",
        '\u{2224}' => "!|",
        '\u{2226}' => "!||",
        '\u{2241}' => "!~",
        '\u{2244}' => "!~=",
        '\u{2249}' => "!~~",
        '\u{2262}' => "!==",
        // --- Bullets read as list markers, not asterisks ---
        '\u{2022}' => "-",
        '\u{2023}' => "-",
        '\u{2043}' => "-",
        '\u{2027}' => "-",
        '\u{25E6}' => "o",
        '\u{25CB}' => "o",
        // --- Latin-1 currency/marks where deunicode is wrong ---
        '\u{00A2}' => "c",    // deunicode: "C/"
        '\u{00A3}' => "GBP",  // deunicode: "PS"
        '\u{00A5}' => "JPY",  // deunicode: "Y="
        '\u{00A7}' => "S",    // deunicode: "SS"
        '\u{2030}' => "0/00", // per mille (deunicode gives "%0")
        // --- Shapes/checks where deunicode is lossy or the char is emoji ---
        '\u{25A1}' => "[ ]",
        '\u{25B6}' => ">", // emoji-classified, so the fallback never sees it
        '\u{25C0}' => "<", // emoji-classified
        '\u{25BC}' => "v", // lowercase for symmetry with the ↓/▲ family
        '\u{25C6}' => "<>",
        '\u{25C7}' => "<>",
        '\u{2713}' => "[x]", // deunicode: "OK"
        '\u{2714}' => "[x]", // deunicode: "checkmark"
        '\u{2717}' => "[ ]",
        '\u{2718}' => "[ ]",
        '\u{2611}' => "[x]", // emoji-classified
        '\u{25AA}' => "-",   // small squares used as list bullets (emoji-classified)
        '\u{25AB}' => "-",
        // --- Meaning-bearing emoji marks: Emoji-classified, so they would drop
        //     under EmojiPolicy::Drop, but they carry pass/fail/alert semantics
        //     LLMs rely on. Curated entries win over the emoji drop; pictorial
        //     emoji still drop. EmojiPolicy::Keep still keeps these as-is. ---
        '\u{2705}' => "[x]", // white heavy check mark
        '\u{274C}' => "[ ]", // cross mark
        '\u{274E}' => "[ ]", // negative squared cross mark
        '\u{2716}' => "x",   // heavy multiplication x
        '\u{26A0}' => "[!]", // warning sign
        '\u{2757}' => "!",
        '\u{2755}' => "!",
        '\u{2753}' => "?",
        '\u{2754}' => "?",
        '\u{2B50}' => "*", // star ratings
        // --- Letterlike marks: emoji-classified or better than the fallback ---
        '\u{00A9}' => "(c)", // emoji-classified
        '\u{00AE}' => "(r)", // emoji-classified
        '\u{00B5}' => "u",   // micro sign is GC=Ll, so no gate reaches it
        '\u{2126}' => "ohm", // else GREEK_MAP would give "Omega"
        // --- Spacing-modifier diacritics where the fallback misreads them ---
        '\u{02DA}' => "deg", // ˚ RING ABOVE, used as a degree sign (deunicode: "@")
        '\u{02BB}' => "'",   // ʻ okina (deunicode gives a backtick, a markdown hazard)
        // --- Technical / keyboard keys (deunicode gives "#", "*", "<", ...) ---
        '\u{2318}' => "Cmd",
        '\u{2325}' => "Opt",
        '\u{2303}' => "Ctrl",
        '\u{21E7}' => "Shift",
        '\u{2387}' => "Alt",
        '\u{238B}' => "Esc",
        '\u{232B}' => "Bksp",
        '\u{2326}' => "Del",
        '\u{23CF}' => "Eject", // emoji-classified, so the fallback never sees it
        _ => return None,
    })
}

fn append_ascii_mapping(mapped: &str, out: &mut String, extended_keyboard: bool) {
    for c in mapped.chars() {
        if is_keyboard_allowed(c, extended_keyboard) {
            out.push(c);
        } else if let Some(compat) = map_compatibility_ascii(c) {
            out.push(compat);
        }
    }
}

fn transliteration_override(c: char) -> Option<&'static str> {
    match c {
        'ß' => Some("ss"),
        'ẞ' => Some("SS"),
        _ => None,
    }
}

fn map_compatibility_ascii(c: char) -> Option<char> {
    match c {
        FRACTION_SLASH => Some('/'),
        _ => None,
    }
}

/// Convenience: clean with default options.
///
/// # Arguments
/// - `text`: Input text to clean.
///
/// # Returns
/// Cleaned output and statistics.
///
/// The default preset emits keyboard-safe output. Non-ASCII text is
/// normalized/folded and transliterated when possible (for example
/// `"Café"` -> `"Cafe"`, `"Straße"` -> `"Strasse"`), while characters
/// with no feasible ASCII mapping are removed.
///
/// # Errors
/// This infallible wrapper does not return errors; construct a
/// [`TextCleaner`] and call [`TextCleaner::try_clean`] for fallible behavior.
///
/// # Panics
/// Panics when normalization is requested but the `unorm` feature is disabled.
pub fn clean(text: &str) -> CleaningResult<'_> {
    TextCleaner::new(CleaningOptions::default()).clean(text)
}

/// Convenience: clean with the humanize preset.
///
/// # Arguments
/// - `text`: Input text to clean.
///
/// # Returns
/// Cleaned output and statistics.
///
/// # Errors
/// This infallible wrapper does not return errors; construct a
/// [`TextCleaner`] and call [`TextCleaner::try_clean`] for error handling.
///
/// # Panics
/// Panics when normalization is requested but the `unorm` feature is disabled.
pub fn humanize(text: &str) -> CleaningResult<'_> {
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
        #[cfg(feature = "stats")]
        assert_eq!(out.stats.hidden_chars_removed, 1);
    }

    #[test]
    fn removes_mongolian_vowel_separator() {
        let c = TextCleaner::new(CleaningOptions::default());
        let out = c.clean("Hello\u{180E}World");
        assert_eq!(out.text, "HelloWorld");
        #[cfg(feature = "stats")]
        assert!(out.stats.hidden_chars_removed >= 1);
    }

    #[test]
    fn normalizes_spaces_and_dashes_quotes_and_ellipsis() {
        let c = TextCleaner::new(CleaningOptions::default());
        let out = c.clean("\u{201C}Hi\u{201D}\u{00A0}\u{2014} ok…");
        assert_eq!(out.text, "\"Hi\" - ok...");
        #[cfg(feature = "stats")]
        {
            assert!(out.stats.spaces_normalized >= 1);
            assert!(out.stats.dashes_normalized >= 1);
            assert!(out.stats.quotes_normalized >= 2);
            assert!(out.stats.other_normalized >= 1);
        }
    }

    #[test]
    fn trims_trailing_ws() {
        let c = TextCleaner::new(CleaningOptions {
            remove_trailing_whitespace: true,
            ..CleaningOptions::minimal()
        });
        let out = c.clean("a  \n b\t\t\n");
        assert_eq!(out.text, "a\n b\n");
        #[cfg(feature = "stats")]
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
        #[cfg(feature = "stats")]
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
        #[cfg(feature = "stats")]
        assert!(out.stats.line_endings_normalized >= 3);
    }

    #[test]
    fn normalizes_unicode_line_separators() {
        let mut options = CleaningOptions::minimal();
        options.normalize_line_endings = Some(LineEndingStyle::Lf);
        let c = TextCleaner::new(options);
        let out = c.clean("a\u{2028}b\u{2029}c");
        assert_eq!(out.text, "a\nb\nc");
        #[cfg(feature = "stats")]
        assert_eq!(out.stats.line_endings_normalized, 2);
    }

    #[test]
    fn restamping_counts_changes() {
        let mut options = CleaningOptions::minimal();
        options.normalize_line_endings = Some(LineEndingStyle::Crlf);
        let c = TextCleaner::new(options);
        let out = c.clean("a\nb\n");
        assert_eq!(out.text, "a\r\nb\r\n");
        #[cfg(feature = "stats")]
        assert_eq!(out.stats.line_endings_normalized, 2);
    }

    #[test]
    fn default_cleaning_matches_keyboard_equivalent() {
        let out = clean("“Hello—world…”\u{00A0}😀");
        assert_eq!(out.text, "\"Hello-world...\"");
        #[cfg(feature = "stats")]
        {
            assert_eq!(out.stats.quotes_normalized, 2);
            assert_eq!(out.stats.dashes_normalized, 1);
            assert_eq!(out.stats.other_normalized, 1);
            assert_eq!(out.stats.spaces_normalized, 1);
            assert_eq!(out.stats.emojis_dropped, 1);
        }
        assert_eq!(out.changes_made, 7);
    }

    #[test]
    fn keyboard_only_drops_non_ascii_and_emoji() {
        let cleaner = TextCleaner::new(CleaningOptions {
            keyboard_only: true,
            ..CleaningOptions::default()
        });
        let out = cleaner.clean("Ascii😀世界");
        assert_eq!(out.text, "Ascii");
        #[cfg(feature = "stats")]
        {
            assert_eq!(out.stats.emojis_dropped, 1);
            assert!(out.stats.non_keyboard_removed >= 2);
        }
    }

    #[test]
    fn keyboard_only_folds_latin_diacritics_to_ascii() {
        let cleaner = TextCleaner::new(CleaningOptions::default());
        let out = cleaner.clean("Caf\u{00E9} d\u{00E9}j\u{00E0} vu");
        assert_eq!(out.text, "Cafe deja vu");
        #[cfg(feature = "stats")]
        assert!(out.stats.non_keyboard_transliterated >= 3);
    }

    #[test]
    fn transliterates_non_decomposing_latin_letters() {
        let cleaner = TextCleaner::new(CleaningOptions::default());
        let out = cleaner.clean("Stra\u{00DF}e \u{00C6}sir \u{00F8}l \u{0153}uvre");
        assert_eq!(out.text, "Strasse AEsir ol oeuvre");
        #[cfg(feature = "stats")]
        assert!(out.stats.non_keyboard_transliterated >= 4);
    }

    #[test]
    fn non_ascii_policy_modes_control_behavior() {
        let drop = TextCleaner::new(
            CleaningOptions::builder()
                .non_ascii_policy(NonAsciiPolicy::Drop)
                .build(),
        )
        .clean("Stra\u{00DF}e \u{00BD} \u{2122}");
        assert_eq!(drop.text, "Strae");

        // ½ -> "1/2" and ™ -> "TM" come from NFKD, which needs `unorm`.
        #[cfg(feature = "unorm")]
        {
            let fold = TextCleaner::new(
                CleaningOptions::builder()
                    .non_ascii_policy(NonAsciiPolicy::Fold)
                    .build(),
            )
            .clean("Stra\u{00DF}e \u{00BD} \u{2122}");
            assert_eq!(fold.text, "Strae 1/2 TM");

            let transliterate = TextCleaner::new(
                CleaningOptions::builder()
                    .non_ascii_policy(NonAsciiPolicy::Transliterate)
                    .build(),
            )
            .clean("Stra\u{00DF}e \u{00BD} \u{2122}");
            assert_eq!(transliterate.text, "Strasse 1/2 TM");
            #[cfg(feature = "stats")]
            assert!(transliterate.stats.non_keyboard_transliterated >= 3);
        }
    }

    #[test]
    fn extended_keyboard_allowlist_can_preserve_curated_symbols() {
        let default = TextCleaner::new(
            CleaningOptions::builder()
                .non_ascii_policy(NonAsciiPolicy::Drop)
                .build(),
        )
        .clean("€ and ™");
        assert_eq!(default.text, "and");

        let extended = TextCleaner::new(
            CleaningOptions::builder()
                .extended_keyboard(true)
                .non_ascii_policy(NonAsciiPolicy::Drop)
                .build(),
        )
        .clean("€ and ™");
        assert_eq!(extended.text, "€ and");
    }

    #[test]
    fn preserve_joiners_toggle_controls_zwj_zwnj_retention() {
        let text = "می\u{200C}خواهم";
        let default =
            TextCleaner::new(CleaningOptions::builder().keyboard_only(false).build()).clean(text);
        assert!(!default.text.contains('\u{200C}'));

        let preserved = TextCleaner::new(
            CleaningOptions::builder()
                .keyboard_only(false)
                .preserve_joiners(true)
                .build(),
        )
        .clean(text);
        assert!(preserved.text.contains('\u{200C}'));
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
            keyboard_only: false,
            ..CleaningOptions::default()
        });
        let out = cleaner.clean(input);
        assert_eq!(out.text, "Hello\u{200B} World!");
        assert_eq!(out.changes_made, 3);

        let cleaner = TextCleaner::new(CleaningOptions {
            normalize_spaces: false,
            keyboard_only: false,
            ..CleaningOptions::default()
        });
        let out = cleaner.clean(input);
        assert_eq!(out.text, "Hello\u{00A0}World!");
        assert_eq!(out.changes_made, 3);
    }

    #[cfg(feature = "security")]
    #[test]
    fn strips_bidi_controls_when_enabled() {
        let options = CleaningOptions {
            strip_bidi_controls: true,
            ..CleaningOptions::default()
        };
        let cleaner = TextCleaner::new(options);
        let out = cleaner.clean("\u{202E}ab\u{202C}c");
        assert_eq!(out.text, "abc");
        assert!(out.changes_made >= 2);
        #[cfg(feature = "stats")]
        {
            assert!(out.stats.bidi_controls_removed >= 2);
        }
    }

    #[test]
    fn ts_dashes_case() {
        let cleaner = TextCleaner::new(CleaningOptions::default());
        let out = cleaner.clean("I — super — man – 💪");
        assert_eq!(out.text, "I - super - man -");
        #[cfg(feature = "stats")]
        {
            assert_eq!(out.stats.dashes_normalized, 3);
            assert_eq!(out.stats.emojis_dropped, 1);
        }
        assert_eq!(out.changes_made, 5);
    }

    #[test]
    fn ts_quotes_case() {
        let cleaner = TextCleaner::new(CleaningOptions::default());
        let out = cleaner.clean("Angular “quote” «marks» looks„ like Christmas «« tree");
        assert_eq!(
            out.text,
            "Angular \"quote\" \"marks\" looks\" like Christmas \"\" tree"
        );
        #[cfg(feature = "stats")]
        assert_eq!(out.stats.quotes_normalized, 7);
        assert_eq!(out.changes_made, 7);
    }

    #[test]
    fn maps_additional_quotes_and_primes() {
        let cleaner = TextCleaner::new(CleaningOptions::default());
        let out = cleaner.clean("‹left› ‟double‟ ′prime′ ″double″");
        assert_eq!(out.text, "'left' \"double\" 'prime' \"double\"");
        #[cfg(feature = "stats")]
        assert!(out.stats.quotes_normalized >= 6);
    }

    #[test]
    fn fraction_slash_maps_to_ascii() {
        let cleaner = TextCleaner::new(CleaningOptions::default());
        let out = cleaner.clean("1\u{2044}2");
        assert_eq!(out.text, "1/2");
        #[cfg(feature = "stats")]
        assert_eq!(out.stats.other_normalized, 1);
    }

    #[test]
    fn keeps_variation_selector_for_emoji() {
        let cleaner = TextCleaner::new(CleaningOptions {
            keyboard_only: false,
            ..CleaningOptions::default()
        });
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
        #[cfg(feature = "stats")]
        assert_eq!(out.stats.emojis_dropped, 1);
    }

    #[test]
    fn code_safe_preset_fields_match_cli_contract() {
        let options = CleaningOptions::code_safe();
        assert!(options.remove_hidden);
        assert!(options.remove_trailing_whitespace);
        assert!(options.normalize_spaces);
        assert!(!options.normalize_dashes);
        assert!(!options.normalize_quotes);
        assert!(!options.normalize_other);
        assert!(!options.keyboard_only);
        assert_eq!(options.emoji_policy, EmojiPolicy::Keep);
        assert_eq!(options.non_ascii_policy, NonAsciiPolicy::Transliterate);
        assert!(options.preserve_joiners);
        assert!(options.remove_control_chars);
        assert!(!options.collapse_whitespace);
        assert_eq!(options.normalize_line_endings, None);
        assert_eq!(
            options.unicode_normalization,
            UnicodeNormalizationMode::None
        );
    }

    // ---- F2: negation must never be inverted ----

    #[test]
    fn negated_operators_are_not_inverted() {
        let c = TextCleaner::new(CleaningOptions::default());
        // The three operators whose ASCII base survives NFKD and would otherwise
        // flip; output must carry the negation, never the bare base operator.
        assert_eq!(c.clean("a \u{2260} b").text, "a != b"); // was "a = b"
        assert_eq!(c.clean("a \u{226E} b").text, "a !< b"); // was "a < b"
        assert_eq!(c.clean("a \u{226F} b").text, "a !> b"); // was "a > b"
        assert_ne!(c.clean("\u{2260}").text, "=");
    }

    #[test]
    fn fold_mode_also_preserves_negation() {
        let fold = TextCleaner::new(
            CleaningOptions::builder()
                .non_ascii_policy(NonAsciiPolicy::Fold)
                .build(),
        );
        // compat_override runs before NFKD in Fold too, so ≠ stays "!=", not "=".
        assert_eq!(fold.clean("a \u{2260} b").text, "a != b");
    }

    // ---- F3: symbols transliterate instead of being deleted ----

    #[test]
    fn arrows_transliterate_to_ascii() {
        let c = TextCleaner::new(CleaningOptions::default());
        assert_eq!(c.clean("a \u{2192} b").text, "a -> b"); // →
        assert_eq!(c.clean("a \u{2190} b").text, "a <- b"); // ←
        assert_eq!(c.clean("a \u{2194} b").text, "a <-> b"); // ↔
        assert_eq!(c.clean("a \u{21D2} b").text, "a ==> b"); // ⇒
        assert_eq!(c.clean("a \u{27F6} b").text, "a --> b"); // ⟶ (long)
    }

    #[test]
    fn math_and_relational_operators_transliterate() {
        let c = TextCleaner::new(CleaningOptions::default());
        assert_eq!(c.clean("a \u{2264} b").text, "a <= b"); // ≤
        assert_eq!(c.clean("a \u{2265} b").text, "a >= b"); // ≥
        assert_eq!(c.clean("a \u{2248} b").text, "a ~= b"); // ≈
        assert_eq!(c.clean("5 \u{00B1} 1").text, "5 +/- 1"); // ±
    }

    #[test]
    fn bullets_geometric_and_marks_transliterate() {
        let c = TextCleaner::new(CleaningOptions::default());
        assert_eq!(c.clean("\u{2022} item").text, "- item"); // •
        assert_eq!(c.clean("\u{2605} star").text, "* star"); // ★
        assert_eq!(c.clean("\u{00A9} 2026").text, "(c) 2026"); // © (was dropped as "emoji")
        assert_eq!(c.clean("Acme\u{00AE}").text, "Acme(r)"); // ®
    }

    #[test]
    fn long_tail_symbols_use_deunicode_fallback() {
        // Box-drawing is not in the curated table; it reaches deunicode via the
        // widened symbol gate (─ -> "-", │ -> "|", ┌ -> "+").
        let c = TextCleaner::new(CleaningOptions::default());
        assert_eq!(c.clean("\u{2500}\u{2502}\u{250C}").text, "-|+");
    }

    #[test]
    fn scripts_still_drop_and_are_not_romanized() {
        // The symbol gate must not enable deunicode for letter scripts. Greek is
        // the one deliberate exception (spelled out; see greek_letters tests).
        let c = TextCleaner::new(CleaningOptions::default());
        assert_eq!(c.clean("ok \u{4E16}\u{754C}").text, "ok"); // 世界 dropped, not "Shi Jie"
        assert_eq!(c.clean("ok \u{0430}\u{0431}").text, "ok"); // Cyrillic dropped
        assert_eq!(c.clean("ok \u{0645}\u{0631}").text, "ok"); // Arabic dropped
    }

    // ---- Greek letters spell out to names (math-symbol usage) ----

    #[test]
    fn greek_letters_spell_out_to_names() {
        let c = TextCleaner::new(CleaningOptions::default());
        assert_eq!(c.clean("\u{03BB} = 0.5").text, "lambda = 0.5");
        assert_eq!(
            c.clean("\u{0394}x \u{2264} \u{03B5}").text,
            "Deltax <= epsilon"
        );
        assert_eq!(c.clean("\u{03C0} \u{2248} 3.14").text, "pi ~= 3.14");
        assert_eq!(c.clean("\u{03A3} over \u{03C3}").text, "Sigma over sigma");
        assert_eq!(c.clean("\u{03D5}").text, "phi"); // math phi symbol variant
        assert_eq!(c.clean("\u{03AD}").text, "epsilon"); // monotonic accented
    }

    // ---- Meaning-bearing emoji marks transliterate; pictures still drop ----

    #[test]
    fn semantic_emoji_marks_transliterate() {
        let c = TextCleaner::new(CleaningOptions::default());
        assert_eq!(c.clean("\u{2705} tests pass").text, "[x] tests pass");
        assert_eq!(c.clean("\u{274C} build fails").text, "[ ] build fails");
        assert_eq!(c.clean("\u{26A0}\u{FE0F} careful").text, "[!] careful"); // with VS16
        assert_eq!(c.clean("\u{2B50}\u{2B50}\u{2B50}").text, "***");
        assert_eq!(c.clean("done\u{2757}").text, "done!");
        // Pictorial emoji still drop.
        assert_eq!(c.clean("ok \u{1F44D}\u{2702}").text, "ok");
    }

    // ---- Latin-1 punctuation / currency (below the block gates) ----

    #[test]
    fn latin1_punctuation_and_currency_transliterate() {
        let c = TextCleaner::new(CleaningOptions::default());
        assert_eq!(c.clean("50\u{00A2}").text, "50c");
        assert_eq!(c.clean("\u{00A3}5").text, "GBP5");
        assert_eq!(c.clean("\u{00A5}100").text, "JPY100");
        assert_eq!(c.clean("a\u{00A6}b").text, "a|b");
        assert_eq!(c.clean("\u{00A7} 230").text, "S 230");
        assert_eq!(c.clean("\u{00B6} 4").text, "P 4");
    }

    // ---- Math extras where deunicode alone was wrong or absent ----

    #[test]
    fn math_extras_transliterate() {
        let c = TextCleaner::new(CleaningOptions::default());
        assert_eq!(c.clean("x \u{2254} 5").text, "x := 5"); // ≔ (deunicode: "=")
        assert_eq!(c.clean("A \u{21CC} B").text, "A <=> B"); // ⇌ (deunicode: "=")
        assert_eq!(c.clean("f \u{2218} g").text, "f o g"); // ∘ (deunicode: "*")
        assert_eq!(c.clean("a \u{226A} b").text, "a << b"); // ≪ via gate fallback
        assert_eq!(c.clean("\u{27E8}x, y\u{27E9}").text, "<x, y>"); // ⟨⟩ via gate
        assert_eq!(c.clean("5\u{2030}").text, "50/00"); // per mille
    }

    // ---- Phonetic Latin and modifier apostrophes ----

    #[test]
    fn phonetic_latin_and_modifier_apostrophes() {
        let c = TextCleaner::new(CleaningOptions::default());
        assert_eq!(c.clean("don\u{02BC}t").text, "don't"); // modifier apostrophe
        assert_eq!(c.clean("Hawai\u{02BB}i").text, "Hawai'i"); // okina, not a backtick
        assert_eq!(c.clean("37\u{02DA}C").text, "37degC"); // spacing ring as degree
        assert_eq!(c.clean("\u{0259}").text, "@"); // IPA schwa, SAMPA-style
    }

    // ---- F1: ASCII fast path must respect keyboard_only ----

    #[test]
    fn fast_path_respects_keyboard_only() {
        // minimal() leaves trim/collapse/control off, which is what makes the
        // all-ASCII fast path reachable; keyboard_only must still strip C0/DEL.
        let c = TextCleaner::new(CleaningOptions {
            keyboard_only: true,
            ..CleaningOptions::minimal()
        });
        let out = c.clean("a\u{0001}b\u{007F}c");
        assert_eq!(out.text, "abc");
    }

    // ---- F4: a dropped emoji is billed once, not also as hidden removals ----

    #[test]
    fn dropped_emoji_not_double_counted() {
        let c = TextCleaner::new(CleaningOptions {
            keyboard_only: true,
            emoji_policy: EmojiPolicy::Drop,
            ..CleaningOptions::default()
        });
        let out = c.clean("\u{1F44D}\u{FE0F}"); // 👍 + VS16
        assert_eq!(out.text, "");
        assert_eq!(out.changes_made, 1);
        #[cfg(feature = "stats")]
        {
            assert_eq!(out.stats.emojis_dropped, 1);
            assert_eq!(out.stats.hidden_chars_removed, 0); // VS16 not separately billed
        }

        let family = c.clean("\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}"); // ZWJ family
        assert_eq!(family.text, "");
        assert_eq!(family.changes_made, 1);
        #[cfg(feature = "stats")]
        {
            assert_eq!(family.stats.emojis_dropped, 1);
            assert_eq!(family.stats.hidden_chars_removed, 0); // two interior ZWJ not billed
        }
    }

    // ---- F7: U+2028/U+2029 fold to \n without normalize_line_endings ----

    #[test]
    fn line_separators_fold_to_newline_without_line_ending_option() {
        // Default preset (keyboard_only, normalize_line_endings = None).
        let out = clean("a\u{2028}b\u{2029}c");
        assert_eq!(out.text, "a\nb\nc");
        #[cfg(feature = "stats")]
        assert_eq!(out.stats.spaces_normalized, 2);

        // Non-keyboard mode: previously passed through unchanged.
        let c = TextCleaner::new(CleaningOptions::builder().keyboard_only(false).build());
        assert_eq!(c.clean("a\u{2028}b").text, "a\nb");

        // Trailing whitespace before the separator trims like a real newline.
        assert_eq!(clean("a \u{2028}b").text, "a\nb");

        // humanize collapses whitespace but keeps the folded newline. (Its
        // preset requests NFKC, so it needs `unorm` to run at all.)
        #[cfg(feature = "unorm")]
        assert_eq!(humanize("x\u{2028}y").text, "x\ny");
    }

    #[test]
    fn stream_cleaner_flushes_on_unicode_line_separators() {
        // Since batch cleaning treats U+2028/U+2029 as line breaks, streaming
        // must flush on them too: a stream delimited only by these separators
        // (no LF byte anywhere) has to emit per line, not buffer to finish().
        for sep in ['\u{2028}', '\u{2029}'] {
            let mut stream = StreamCleaner::new(CleaningOptions::default());
            let mut out = String::new();
            let input = format!("alpha{sep}beta");
            let flushed = stream
                .feed(&input, &mut out)
                .unwrap_or_else(|| panic!("U+{:04X} must be a flush boundary", sep as u32));
            assert_eq!(flushed.text, "alpha\n");
            let mut tail = String::new();
            let finished = stream.finish(&mut tail).expect("buffered remainder");
            assert_eq!(finished.text, "beta");
        }
    }

    // ---- idempotence: clean(clean(x)) == clean(x) on representative input ----

    #[test]
    fn cleaning_is_idempotent_on_symbol_samples() {
        let c = TextCleaner::new(CleaningOptions::default());
        for s in [
            "a \u{2192} b \u{2264} c",
            "\u{2260} \u{00A9} \u{2605} \u{2022}",
            "caf\u{00E9} \u{2014} \u{00BD}",
            "\u{1F44D}\u{FE0F} done",
            "\u{00B8}\u{00B4}\u{00A8}", // spacing diacritics: the F8 regression
            "\u{03BB} \u{03A3} \u{2705} \u{274C} \u{00A3} \u{00A7}",
            "\u{27E8}x\u{27E9} don\u{02BC}t \u{0259}",
        ] {
            let once = c.clean(s).text.into_owned();
            let twice = c.clean(&once).text.into_owned();
            assert_eq!(once, twice, "not idempotent for {s:?}");
        }
    }
}
