//! PyO3 bindings exposing `rehuman` as the `rehuman._rehuman` extension.

use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyTuple, PyType};
use rehuman::{
    CleaningOptions, CleaningStats, EmojiPolicy, LineEndingStyle, NonAsciiPolicy, TextCleaner,
    UnicodeNormalizationMode,
};

const EMOJI_POLICY_NAMES: &[(&str, EmojiPolicy)] =
    &[("drop", EmojiPolicy::Drop), ("keep", EmojiPolicy::Keep)];
const NON_ASCII_POLICY_NAMES: &[(&str, NonAsciiPolicy)] = &[
    ("drop", NonAsciiPolicy::Drop),
    ("fold", NonAsciiPolicy::Fold),
    ("transliterate", NonAsciiPolicy::Transliterate),
];
const LINE_ENDING_NAMES: &[(&str, Option<LineEndingStyle>)] = &[
    ("auto", None),
    ("none", None),
    ("lf", Some(LineEndingStyle::Lf)),
    ("crlf", Some(LineEndingStyle::Crlf)),
    ("cr", Some(LineEndingStyle::Cr)),
];
const UNICODE_NORMALIZATION_NAMES: &[(&str, UnicodeNormalizationMode)] = &[
    ("none", UnicodeNormalizationMode::None),
    ("nfd", UnicodeNormalizationMode::NFD),
    ("nfc", UnicodeNormalizationMode::NFC),
    ("nfkd", UnicodeNormalizationMode::NFKD),
    ("nfkc", UnicodeNormalizationMode::NFKC),
];

fn format_choice<T: Copy + Eq>(value: T, names: &'static [(&'static str, T)]) -> &'static str {
    names
        .iter()
        .find_map(|(name, candidate)| (*candidate == value).then_some(*name))
        .expect("every Python-facing enum value must have a canonical name")
}

fn parse_choice<T: Copy>(
    value: &str,
    names: &[(&str, T)],
    label: &str,
    expected: &str,
) -> PyResult<T> {
    let normalized = value.to_ascii_lowercase();
    names
        .iter()
        .find_map(|(name, candidate)| (*name == normalized).then_some(*candidate))
        .ok_or_else(|| {
            PyValueError::new_err(format!(
                "invalid {label}: {normalized:?} (expected {expected})"
            ))
        })
}

fn format_emoji_policy(policy: EmojiPolicy) -> &'static str {
    format_choice(policy, EMOJI_POLICY_NAMES)
}

fn format_non_ascii_policy(policy: NonAsciiPolicy) -> &'static str {
    format_choice(policy, NON_ASCII_POLICY_NAMES)
}

fn format_line_endings(style: Option<LineEndingStyle>) -> &'static str {
    format_choice(style, LINE_ENDING_NAMES)
}

fn format_unicode_normalization(mode: UnicodeNormalizationMode) -> &'static str {
    format_choice(mode, UNICODE_NORMALIZATION_NAMES)
}

fn parse_unicode_normalization(value: &str) -> PyResult<UnicodeNormalizationMode> {
    parse_choice(
        value,
        UNICODE_NORMALIZATION_NAMES,
        "normalization mode",
        "none/nfd/nfc/nfkd/nfkc",
    )
}

fn parse_non_ascii_policy(value: &str) -> PyResult<NonAsciiPolicy> {
    parse_choice(
        value,
        NON_ASCII_POLICY_NAMES,
        "non-ASCII policy",
        "drop/fold/transliterate",
    )
}

fn parse_line_endings(value: Option<&str>) -> PyResult<Option<LineEndingStyle>> {
    value.map_or(Ok(None), |value| {
        parse_choice(
            value,
            LINE_ENDING_NAMES,
            "line ending style",
            "auto/none/lf/crlf/cr",
        )
    })
}

/// Apply constructor-style keyword arguments onto existing options.
///
/// Shared by `Options.replace` and pickle state restoration; key names and
/// accepted values mirror the `Options` constructor exactly, including
/// rejecting `strip_bidi_controls` on non-`security` builds.
fn apply_option_kwargs(options: &mut CleaningOptions, kwargs: &Bound<'_, PyDict>) -> PyResult<()> {
    for (key, value) in kwargs.iter() {
        let key: String = key.extract()?;
        match key.as_str() {
            "remove_hidden" => options.remove_hidden = value.extract()?,
            "remove_trailing_whitespace" => options.remove_trailing_whitespace = value.extract()?,
            "normalize_spaces" => options.normalize_spaces = value.extract()?,
            "normalize_dashes" => options.normalize_dashes = value.extract()?,
            "normalize_quotes" => options.normalize_quotes = value.extract()?,
            "normalize_other" => options.normalize_other = value.extract()?,
            "keyboard_only" => options.keyboard_only = value.extract()?,
            "extended_keyboard" => options.extended_keyboard = value.extract()?,
            "keep_emoji" => {
                options.emoji_policy = if value.extract()? {
                    EmojiPolicy::Keep
                } else {
                    EmojiPolicy::Drop
                }
            }
            "non_ascii_policy" => {
                options.non_ascii_policy = parse_non_ascii_policy(&value.extract::<String>()?)?
            }
            "preserve_joiners" => options.preserve_joiners = value.extract()?,
            "remove_control_chars" => options.remove_control_chars = value.extract()?,
            "collapse_whitespace" => options.collapse_whitespace = value.extract()?,
            "line_endings" => {
                options.normalize_line_endings =
                    parse_line_endings(value.extract::<Option<String>>()?.as_deref())?
            }
            "unicode_normalization" => {
                options.unicode_normalization =
                    parse_unicode_normalization(&value.extract::<String>()?)?
            }
            #[cfg(feature = "security")]
            "strip_bidi_controls" => options.strip_bidi_controls = value.extract()?,
            _ => {
                return Err(PyTypeError::new_err(format!(
                    "unexpected keyword argument {key:?}"
                )))
            }
        }
    }
    Ok(())
}

fn stats_to_dict<'py>(py: Python<'py>, stats: &CleaningStats) -> PyResult<Bound<'py, PyDict>> {
    let dict = PyDict::new(py);
    let serialized = serde_json::to_value(stats)
        .map_err(|error| PyValueError::new_err(format!("failed to serialize stats: {error}")))?;
    let fields = serialized
        .as_object()
        .ok_or_else(|| PyValueError::new_err("CleaningStats did not serialize as an object"))?;
    for (name, value) in fields {
        let count = value.as_u64().ok_or_else(|| {
            PyValueError::new_err(format!("stats field {name:?} did not serialize as u64"))
        })?;
        dict.set_item(name, count)?;
    }
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
///
/// Releases the GIL while the Rust pipeline runs.
fn clean(py: Python<'_>, text: &str) -> String {
    py.detach(|| rehuman::clean(text).text.into_owned())
}

#[pyfunction]
/// Clean text with the "humanize" preset and return cleaned text only.
///
/// This preset applies typographic normalization and whitespace collapsing.
///
/// Releases the GIL while the Rust pipeline runs.
fn humanize(py: Python<'_>, text: &str) -> String {
    py.detach(|| rehuman::humanize(text).text.into_owned())
}

#[pyclass(module = "rehuman._rehuman", skip_from_py_object)]
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
    // PyO3's rich-comparison slot converts a failed `PyRef` extraction to
    // `NotImplemented`, allowing Python to try reflected comparison. Keep the
    // typed parameter so value equality remains limited to this class.
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

#[pyclass(module = "rehuman._rehuman", skip_from_py_object)]
#[derive(Clone)]
/// Cleaning options used by `Cleaner`.
///
/// Most callers should start with a preset and derive from it with
/// `replace`, for example `Options.code_safe_preset().replace(keep_emoji=True)`.
struct Options {
    inner: CleaningOptions,
}

impl Options {
    #[allow(clippy::too_many_arguments)]
    fn from_parameters(
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

    /// Full field state as a constructor-kwargs dict (pickle payload).
    fn state_dict<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let o = &self.inner;
        let dict = PyDict::new(py);
        dict.set_item("remove_hidden", o.remove_hidden)?;
        dict.set_item("remove_trailing_whitespace", o.remove_trailing_whitespace)?;
        dict.set_item("normalize_spaces", o.normalize_spaces)?;
        dict.set_item("normalize_dashes", o.normalize_dashes)?;
        dict.set_item("normalize_quotes", o.normalize_quotes)?;
        dict.set_item("normalize_other", o.normalize_other)?;
        dict.set_item("keyboard_only", o.keyboard_only)?;
        dict.set_item("extended_keyboard", o.extended_keyboard)?;
        dict.set_item("keep_emoji", o.emoji_policy == EmojiPolicy::Keep)?;
        dict.set_item(
            "non_ascii_policy",
            format_non_ascii_policy(o.non_ascii_policy),
        )?;
        dict.set_item("preserve_joiners", o.preserve_joiners)?;
        dict.set_item("remove_control_chars", o.remove_control_chars)?;
        dict.set_item("collapse_whitespace", o.collapse_whitespace)?;
        dict.set_item(
            "line_endings",
            o.normalize_line_endings
                .map(|style| format_line_endings(Some(style))),
        )?;
        dict.set_item(
            "unicode_normalization",
            format_unicode_normalization(o.unicode_normalization),
        )?;
        #[cfg(feature = "security")]
        dict.set_item("strip_bidi_controls", o.strip_bidi_controls)?;
        Ok(dict)
    }
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
        Self::from_parameters(
            remove_hidden,
            remove_trailing_whitespace,
            normalize_spaces,
            normalize_dashes,
            normalize_quotes,
            normalize_other,
            keyboard_only,
            extended_keyboard,
            keep_emoji,
            non_ascii_policy,
            preserve_joiners,
            remove_control_chars,
            collapse_whitespace,
            line_endings,
            unicode_normalization,
            strip_bidi_controls,
        )
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
        Self::from_parameters(
            remove_hidden,
            remove_trailing_whitespace,
            normalize_spaces,
            normalize_dashes,
            normalize_quotes,
            normalize_other,
            keyboard_only,
            extended_keyboard,
            keep_emoji,
            non_ascii_policy,
            preserve_joiners,
            remove_control_chars,
            collapse_whitespace,
            line_endings,
            unicode_normalization,
            false,
        )
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
    /// Keeps emoji, non-ASCII characters, and ellipsis-like punctuation,
    /// while normalizing typographic quotes and dashes to ASCII.
    fn code_safe_preset() -> Self {
        Self {
            inner: CleaningOptions::code_safe(),
        }
    }

    #[pyo3(signature = (**kwargs))]
    /// Return a copy of these options with the named fields replaced.
    ///
    /// Accepts the same keyword arguments as the constructor; unnamed fields
    /// keep their current values. Use this to derive from a preset, for
    /// example `Options.code_safe_preset().replace(normalize_other=True)`.
    fn replace(&self, kwargs: Option<&Bound<'_, PyDict>>) -> PyResult<Self> {
        let mut inner = self.inner.clone();
        if let Some(kwargs) = kwargs {
            apply_option_kwargs(&mut inner, kwargs)?;
        }
        Ok(Self { inner })
    }

    // PyO3's rich-comparison slot converts a failed `PyRef` extraction to
    // `NotImplemented`, allowing Python to try reflected comparison. Keep the
    // typed parameter so value equality remains limited to this class.
    fn __eq__(&self, other: PyRef<'_, Options>) -> bool {
        self.inner == other.inner
    }

    #[getter]
    fn remove_hidden(&self) -> bool {
        self.inner.remove_hidden
    }

    #[getter]
    fn remove_trailing_whitespace(&self) -> bool {
        self.inner.remove_trailing_whitespace
    }

    #[getter]
    fn normalize_spaces(&self) -> bool {
        self.inner.normalize_spaces
    }

    #[getter]
    fn normalize_dashes(&self) -> bool {
        self.inner.normalize_dashes
    }

    #[getter]
    fn normalize_quotes(&self) -> bool {
        self.inner.normalize_quotes
    }

    #[getter]
    fn normalize_other(&self) -> bool {
        self.inner.normalize_other
    }

    #[getter]
    fn keyboard_only(&self) -> bool {
        self.inner.keyboard_only
    }

    #[getter]
    fn extended_keyboard(&self) -> bool {
        self.inner.extended_keyboard
    }

    #[getter]
    fn keep_emoji(&self) -> bool {
        self.inner.emoji_policy == EmojiPolicy::Keep
    }

    #[getter]
    fn non_ascii_policy(&self) -> &'static str {
        format_non_ascii_policy(self.inner.non_ascii_policy)
    }

    #[getter]
    fn preserve_joiners(&self) -> bool {
        self.inner.preserve_joiners
    }

    #[getter]
    fn remove_control_chars(&self) -> bool {
        self.inner.remove_control_chars
    }

    #[getter]
    fn collapse_whitespace(&self) -> bool {
        self.inner.collapse_whitespace
    }

    #[getter]
    fn line_endings(&self) -> Option<&'static str> {
        self.inner
            .normalize_line_endings
            .map(|style| format_line_endings(Some(style)))
    }

    #[getter]
    fn unicode_normalization(&self) -> &'static str {
        format_unicode_normalization(self.inner.unicode_normalization)
    }

    #[cfg(feature = "security")]
    #[getter]
    fn strip_bidi_controls(&self) -> bool {
        self.inner.strip_bidi_controls
    }

    /// Pickle support: reconstruct via `Options()` + `__setstate__`.
    fn __reduce__<'py>(
        &self,
        py: Python<'py>,
    ) -> PyResult<(Bound<'py, PyType>, Bound<'py, PyTuple>, Bound<'py, PyDict>)> {
        Ok((
            py.get_type::<Options>(),
            PyTuple::empty(py),
            self.state_dict(py)?,
        ))
    }

    fn __setstate__(&mut self, state: &Bound<'_, PyDict>) -> PyResult<()> {
        let mut inner = CleaningOptions::default();
        apply_option_kwargs(&mut inner, state)?;
        self.inner = inner;
        Ok(())
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

#[pyclass(module = "rehuman._rehuman")]
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
    /// Releases the GIL while the Rust pipeline runs. Raises `ValueError` if
    /// requested normalization is unavailable in the current build
    /// configuration.
    fn clean(&self, py: Python<'_>, text: &str) -> PyResult<CleaningResult> {
        let (text, changes_made, stats) = py
            .detach(|| {
                self.inner
                    .try_clean(text)
                    .map(|result| (result.text.into_owned(), result.changes_made, result.stats))
            })
            .map_err(|error| PyValueError::new_err(error.to_string()))?;
        Ok(CleaningResult {
            text,
            changes_made,
            stats_inner: stats,
        })
    }

    /// Pickle support: reconstruct as `Cleaner(options)`.
    fn __reduce__<'py>(&self, py: Python<'py>) -> (Bound<'py, PyType>, (Options,)) {
        let options = Options {
            inner: self.inner.options().clone(),
        };
        (py.get_type::<Cleaner>(), (options,))
    }

    fn __repr__(&self) -> String {
        let options = self.inner.options();
        format!(
            "Cleaner(keyboard_only={}, emoji_policy='{}')",
            options.keyboard_only,
            format_emoji_policy(options.emoji_policy)
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
