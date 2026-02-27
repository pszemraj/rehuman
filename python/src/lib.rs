//! PyO3 bindings exposing `rehuman` as the `rehuman._rehuman` extension.

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use rehuman::{
    CleaningOptions, CleaningStats, EmojiPolicy, LineEndingStyle, NonAsciiPolicy, TextCleaner,
    UnicodeNormalizationMode,
};

fn format_emoji_policy(policy: EmojiPolicy) -> &'static str {
    match policy {
        EmojiPolicy::Drop => "drop",
        EmojiPolicy::Keep => "keep",
    }
}

fn format_non_ascii_policy(policy: NonAsciiPolicy) -> &'static str {
    match policy {
        NonAsciiPolicy::Drop => "drop",
        NonAsciiPolicy::Fold => "fold",
        NonAsciiPolicy::Transliterate => "transliterate",
    }
}

fn format_line_endings(style: Option<LineEndingStyle>) -> &'static str {
    match style {
        None => "auto",
        Some(LineEndingStyle::Lf) => "lf",
        Some(LineEndingStyle::Crlf) => "crlf",
        Some(LineEndingStyle::Cr) => "cr",
    }
}

fn format_unicode_normalization(mode: UnicodeNormalizationMode) -> &'static str {
    match mode {
        UnicodeNormalizationMode::None => "none",
        UnicodeNormalizationMode::NFD => "nfd",
        UnicodeNormalizationMode::NFC => "nfc",
        UnicodeNormalizationMode::NFKD => "nfkd",
        UnicodeNormalizationMode::NFKC => "nfkc",
    }
}

fn parse_unicode_normalization(value: &str) -> PyResult<UnicodeNormalizationMode> {
    match value.to_ascii_lowercase().as_str() {
        "none" => Ok(UnicodeNormalizationMode::None),
        "nfd" => Ok(UnicodeNormalizationMode::NFD),
        "nfc" => Ok(UnicodeNormalizationMode::NFC),
        "nfkd" => Ok(UnicodeNormalizationMode::NFKD),
        "nfkc" => Ok(UnicodeNormalizationMode::NFKC),
        other => Err(PyValueError::new_err(format!(
            "invalid normalization mode: {other:?} (expected none/nfd/nfc/nfkd/nfkc)"
        ))),
    }
}

fn parse_non_ascii_policy(value: &str) -> PyResult<NonAsciiPolicy> {
    match value.to_ascii_lowercase().as_str() {
        "drop" => Ok(NonAsciiPolicy::Drop),
        "fold" => Ok(NonAsciiPolicy::Fold),
        "transliterate" => Ok(NonAsciiPolicy::Transliterate),
        other => Err(PyValueError::new_err(format!(
            "invalid non-ASCII policy: {other:?} (expected drop/fold/transliterate)"
        ))),
    }
}

fn parse_line_endings(value: Option<&str>) -> PyResult<Option<LineEndingStyle>> {
    match value.map(str::to_ascii_lowercase) {
        None => Ok(None),
        Some(mode) => match mode.as_str() {
            "auto" | "none" => Ok(None),
            "lf" => Ok(Some(LineEndingStyle::Lf)),
            "crlf" => Ok(Some(LineEndingStyle::Crlf)),
            "cr" => Ok(Some(LineEndingStyle::Cr)),
            other => Err(PyValueError::new_err(format!(
                "invalid line ending style: {other:?} (expected auto/none/lf/crlf/cr)"
            ))),
        },
    }
}

fn stats_to_dict<'py>(py: Python<'py>, stats: &CleaningStats) -> PyResult<Bound<'py, PyDict>> {
    let dict = PyDict::new(py);
    dict.set_item("hidden_chars_removed", stats.hidden_chars_removed)?;
    dict.set_item(
        "trailing_whitespace_removed",
        stats.trailing_whitespace_removed,
    )?;
    dict.set_item("spaces_normalized", stats.spaces_normalized)?;
    dict.set_item("dashes_normalized", stats.dashes_normalized)?;
    dict.set_item("quotes_normalized", stats.quotes_normalized)?;
    dict.set_item("other_normalized", stats.other_normalized)?;
    dict.set_item("control_chars_removed", stats.control_chars_removed)?;
    dict.set_item("line_endings_normalized", stats.line_endings_normalized)?;
    dict.set_item("non_keyboard_removed", stats.non_keyboard_removed)?;
    dict.set_item(
        "non_keyboard_transliterated",
        stats.non_keyboard_transliterated,
    )?;
    dict.set_item("emojis_dropped", stats.emojis_dropped)?;
    #[cfg(feature = "security")]
    dict.set_item("bidi_controls_removed", stats.bidi_controls_removed)?;
    Ok(dict)
}

#[pyfunction]
/// Clean text with the default `rehuman` policy and return cleaned text only.
///
/// Keyboard-only mode normalizes and transliterates non-ASCII text to ASCII
/// where feasible (`"Café"` -> `"Cafe"`, `"Straße"` -> `"Strasse"`), then
/// drops remaining non-keyboard glyphs.
///
/// Use `Cleaner` when you need `changes_made` and per-operation stats.
fn clean(text: &str) -> String {
    rehuman::clean(text).text.into_owned()
}

#[pyfunction]
/// Clean text with the "humanize" preset and return cleaned text only.
///
/// This preset applies typographic normalization and whitespace collapsing.
fn humanize(text: &str) -> String {
    rehuman::humanize(text).text.into_owned()
}

#[pyclass(skip_from_py_object)]
#[derive(Clone)]
/// Result returned by `Cleaner.clean`.
///
/// Attributes:
/// - `text`: cleaned text output
/// - `changes_made`: total number of transformations
/// - `stats`: dict of per-operation counters
struct CleaningResult {
    #[pyo3(get)]
    text: String,
    #[pyo3(get)]
    changes_made: u64,
    stats_inner: CleaningStats,
}

#[pymethods]
impl CleaningResult {
    fn __eq__(&self, other: PyRef<'_, CleaningResult>) -> bool {
        self.text == other.text
            && self.changes_made == other.changes_made
            && self.stats_inner == other.stats_inner
    }

    #[getter]
    /// Per-operation counters as `dict[str, int]`.
    fn stats<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        stats_to_dict(py, &self.stats_inner)
    }

    fn __repr__(&self) -> String {
        let mut preview: String = self.text.chars().take(60).collect();
        if self.text.chars().count() > 60 {
            preview.push_str("...");
        }
        format!(
            "CleaningResult(changes_made={}, text={preview:?})",
            self.changes_made
        )
    }

    fn __str__(&self) -> &str {
        &self.text
    }

    fn __bool__(&self) -> bool {
        self.changes_made > 0
    }
}

#[pyclass(skip_from_py_object)]
#[derive(Clone)]
/// Cleaning options used by `Cleaner`.
///
/// Most callers should start with a preset and then override specific fields.
struct Options {
    inner: CleaningOptions,
}

#[pymethods]
impl Options {
    #[cfg(feature = "security")]
    #[new]
    #[pyo3(signature = (
        remove_hidden = true,
        remove_trailing_whitespace = true,
        normalize_spaces = true,
        normalize_dashes = true,
        normalize_quotes = true,
        normalize_other = true,
        keyboard_only = true,
        extended_keyboard = false,
        keep_emoji = false,
        non_ascii_policy = "transliterate",
        preserve_joiners = false,
        remove_control_chars = true,
        collapse_whitespace = false,
        line_endings = None,
        unicode_normalization = "none",
        strip_bidi_controls = false,
    ))]
    #[allow(clippy::too_many_arguments)]
    fn new(
        remove_hidden: bool,
        remove_trailing_whitespace: bool,
        normalize_spaces: bool,
        normalize_dashes: bool,
        normalize_quotes: bool,
        normalize_other: bool,
        keyboard_only: bool,
        extended_keyboard: bool,
        keep_emoji: bool,
        non_ascii_policy: &str,
        preserve_joiners: bool,
        remove_control_chars: bool,
        collapse_whitespace: bool,
        line_endings: Option<&str>,
        unicode_normalization: &str,
        strip_bidi_controls: bool,
    ) -> PyResult<Self> {
        let emoji_policy = if keep_emoji {
            EmojiPolicy::Keep
        } else {
            EmojiPolicy::Drop
        };
        let normalize_line_endings = parse_line_endings(line_endings)?;
        let unicode_normalization = parse_unicode_normalization(unicode_normalization)?;
        let non_ascii_policy = parse_non_ascii_policy(non_ascii_policy)?;

        let inner = CleaningOptions::builder()
            .remove_hidden(remove_hidden)
            .remove_trailing_whitespace(remove_trailing_whitespace)
            .normalize_spaces(normalize_spaces)
            .normalize_dashes(normalize_dashes)
            .normalize_quotes(normalize_quotes)
            .normalize_other(normalize_other)
            .keyboard_only(keyboard_only)
            .extended_keyboard(extended_keyboard)
            .emoji_policy(emoji_policy)
            .non_ascii_policy(non_ascii_policy)
            .preserve_joiners(preserve_joiners)
            .remove_control_chars(remove_control_chars)
            .collapse_whitespace(collapse_whitespace)
            .normalize_line_endings(normalize_line_endings)
            .unicode_normalization(unicode_normalization)
            .strip_bidi_controls(strip_bidi_controls)
            .build();

        Ok(Self { inner })
    }

    #[cfg(not(feature = "security"))]
    #[new]
    #[pyo3(signature = (
        remove_hidden = true,
        remove_trailing_whitespace = true,
        normalize_spaces = true,
        normalize_dashes = true,
        normalize_quotes = true,
        normalize_other = true,
        keyboard_only = true,
        extended_keyboard = false,
        keep_emoji = false,
        non_ascii_policy = "transliterate",
        preserve_joiners = false,
        remove_control_chars = true,
        collapse_whitespace = false,
        line_endings = None,
        unicode_normalization = "none",
    ))]
    #[allow(clippy::too_many_arguments)]
    fn new(
        remove_hidden: bool,
        remove_trailing_whitespace: bool,
        normalize_spaces: bool,
        normalize_dashes: bool,
        normalize_quotes: bool,
        normalize_other: bool,
        keyboard_only: bool,
        extended_keyboard: bool,
        keep_emoji: bool,
        non_ascii_policy: &str,
        preserve_joiners: bool,
        remove_control_chars: bool,
        collapse_whitespace: bool,
        line_endings: Option<&str>,
        unicode_normalization: &str,
    ) -> PyResult<Self> {
        let emoji_policy = if keep_emoji {
            EmojiPolicy::Keep
        } else {
            EmojiPolicy::Drop
        };
        let normalize_line_endings = parse_line_endings(line_endings)?;
        let unicode_normalization = parse_unicode_normalization(unicode_normalization)?;
        let non_ascii_policy = parse_non_ascii_policy(non_ascii_policy)?;

        let inner = CleaningOptions::builder()
            .remove_hidden(remove_hidden)
            .remove_trailing_whitespace(remove_trailing_whitespace)
            .normalize_spaces(normalize_spaces)
            .normalize_dashes(normalize_dashes)
            .normalize_quotes(normalize_quotes)
            .normalize_other(normalize_other)
            .keyboard_only(keyboard_only)
            .extended_keyboard(extended_keyboard)
            .emoji_policy(emoji_policy)
            .non_ascii_policy(non_ascii_policy)
            .preserve_joiners(preserve_joiners)
            .remove_control_chars(remove_control_chars)
            .collapse_whitespace(collapse_whitespace)
            .normalize_line_endings(normalize_line_endings)
            .unicode_normalization(unicode_normalization)
            .build();

        Ok(Self { inner })
    }

    #[staticmethod]
    /// Minimal preset: removes hidden characters only.
    fn minimal_preset() -> Self {
        Self {
            inner: CleaningOptions::minimal(),
        }
    }

    #[staticmethod]
    /// Balanced preset for typical human-authored text.
    fn balanced_preset() -> Self {
        Self {
            inner: CleaningOptions::balanced(),
        }
    }

    #[staticmethod]
    /// Humanize preset for AI-generated text normalization.
    fn humanize_preset() -> Self {
        Self {
            inner: CleaningOptions::humanize(),
        }
    }

    #[staticmethod]
    /// Aggressive preset: maximum cleanup, keyboard-only output.
    fn aggressive_preset() -> Self {
        Self {
            inner: CleaningOptions::aggressive(),
        }
    }

    #[staticmethod]
    /// Code-safe preset for docs/source text.
    ///
    /// Keeps emoji and non-ASCII characters, and avoids quote/dash/ellipsis
    /// rewrites so string literals and examples are not semantically altered.
    fn code_safe_preset() -> Self {
        Self {
            inner: CleaningOptions::code_safe(),
        }
    }

    fn __repr__(&self) -> String {
        let o = &self.inner;
        #[cfg(feature = "security")]
        let security = format!(", strip_bidi_controls={}", o.strip_bidi_controls);
        #[cfg(not(feature = "security"))]
        let security = String::new();

        format!(
            concat!(
                "Options(",
                "remove_hidden={}, remove_trailing_whitespace={}, normalize_spaces={}, ",
                "normalize_dashes={}, normalize_quotes={}, normalize_other={}, ",
                "keyboard_only={}, extended_keyboard={}, emoji_policy='{}', non_ascii_policy='{}', preserve_joiners={}, remove_control_chars={}, ",
                "collapse_whitespace={}, line_endings='{}', unicode_normalization='{}'",
                "{})"
            ),
            o.remove_hidden,
            o.remove_trailing_whitespace,
            o.normalize_spaces,
            o.normalize_dashes,
            o.normalize_quotes,
            o.normalize_other,
            o.keyboard_only,
            o.extended_keyboard,
            format_emoji_policy(o.emoji_policy),
            format_non_ascii_policy(o.non_ascii_policy),
            o.preserve_joiners,
            o.remove_control_chars,
            o.collapse_whitespace,
            format_line_endings(o.normalize_line_endings),
            format_unicode_normalization(o.unicode_normalization),
            security
        )
    }
}

#[pyclass]
/// Reusable text cleaner.
///
/// Construct once and call `clean` repeatedly.
struct Cleaner {
    inner: TextCleaner,
}

#[pymethods]
impl Cleaner {
    #[new]
    #[pyo3(signature = (options = None))]
    /// Build a cleaner with optional `Options`.
    fn new(options: Option<PyRef<'_, Options>>) -> Self {
        let inner_options = options
            .map(|options| options.inner.clone())
            .unwrap_or_default();
        Self {
            inner: TextCleaner::new(inner_options),
        }
    }

    /// Clean input text and return a `CleaningResult`.
    ///
    /// Raises `ValueError` if requested normalization is unavailable in the
    /// current build configuration.
    fn clean(&self, text: &str) -> PyResult<CleaningResult> {
        let result = self
            .inner
            .try_clean(text)
            .map_err(|error| PyValueError::new_err(error.to_string()))?;
        Ok(CleaningResult {
            text: result.text.into_owned(),
            changes_made: result.changes_made,
            stats_inner: result.stats,
        })
    }

    fn __repr__(&self) -> String {
        let options = self.inner.options();
        format!(
            "Cleaner(keyboard_only={}, emoji_policy={:?})",
            options.keyboard_only, options.emoji_policy
        )
    }
}

#[pymodule]
/// Native extension module backing the public `rehuman` Python package.
fn _rehuman(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add("__version__", env!("CARGO_PKG_VERSION"))?;
    module.add("HAS_STATS", cfg!(feature = "stats"))?;
    module.add("HAS_SECURITY", cfg!(feature = "security"))?;

    module.add_function(wrap_pyfunction!(clean, module)?)?;
    module.add_function(wrap_pyfunction!(humanize, module)?)?;
    module.add_class::<Cleaner>()?;
    module.add_class::<Options>()?;
    module.add_class::<CleaningResult>()?;
    Ok(())
}
