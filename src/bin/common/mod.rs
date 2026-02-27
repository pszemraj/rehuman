//! Shared CLI types and helpers for `rehuman` and `ishuman`.
//!
//! This module owns argument/value conversion, config I/O, input/output helpers,
//! streaming glue, and stats serialization used by both binaries.

use std::fs;
use std::io::{self, IsTerminal, Read, Write};
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use clap::{Args, ValueEnum};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

use rehuman::{
    CleaningOptions, CleaningResult, CleaningStats, EmojiPolicy, LineEndingStyle, NonAsciiPolicy,
    UnicodeNormalizationMode,
};

/// Maximum input size accepted by non-streaming paths.
pub const MAX_INPUT_BYTES: usize = 5 * 1024 * 1024;
/// Version identifier for on-disk config schema.
pub const CONFIG_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, ValueEnum)]
#[value(rename_all = "kebab_case")]
/// Named option presets for CLI workflows.
pub enum PresetArg {
    Minimal,
    Balanced,
    Humanize,
    Aggressive,
    CodeSafe,
}

/// Build options for a named preset.
///
/// # Arguments
/// - `preset`: Preset variant selected by the user.
///
/// # Returns
/// A full [`CleaningOptions`] value for the selected preset.
pub fn options_from_preset(preset: PresetArg) -> CleaningOptions {
    match preset {
        PresetArg::Minimal => CleaningOptions::minimal(),
        PresetArg::Balanced => CleaningOptions::balanced(),
        PresetArg::Humanize => CleaningOptions::humanize(),
        PresetArg::Aggressive => CleaningOptions::aggressive(),
        PresetArg::CodeSafe => CleaningOptions::code_safe(),
    }
}

#[derive(Clone, Copy, Debug, ValueEnum, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
/// CLI/config representation of emoji handling policy.
pub enum EmojiPolicyArg {
    Drop,
    Keep,
}

impl From<EmojiPolicyArg> for EmojiPolicy {
    fn from(value: EmojiPolicyArg) -> Self {
        match value {
            EmojiPolicyArg::Drop => EmojiPolicy::Drop,
            EmojiPolicyArg::Keep => EmojiPolicy::Keep,
        }
    }
}

impl From<EmojiPolicy> for EmojiPolicyArg {
    fn from(value: EmojiPolicy) -> Self {
        match value {
            EmojiPolicy::Drop => EmojiPolicyArg::Drop,
            EmojiPolicy::Keep => EmojiPolicyArg::Keep,
        }
    }
}

#[derive(Clone, Copy, Debug, ValueEnum, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
/// CLI/config representation of non-ASCII handling in keyboard-only mode.
pub enum NonAsciiPolicyArg {
    Drop,
    Fold,
    Transliterate,
}

impl From<NonAsciiPolicyArg> for NonAsciiPolicy {
    fn from(value: NonAsciiPolicyArg) -> Self {
        match value {
            NonAsciiPolicyArg::Drop => NonAsciiPolicy::Drop,
            NonAsciiPolicyArg::Fold => NonAsciiPolicy::Fold,
            NonAsciiPolicyArg::Transliterate => NonAsciiPolicy::Transliterate,
        }
    }
}

impl From<NonAsciiPolicy> for NonAsciiPolicyArg {
    fn from(value: NonAsciiPolicy) -> Self {
        match value {
            NonAsciiPolicy::Drop => NonAsciiPolicyArg::Drop,
            NonAsciiPolicy::Fold => NonAsciiPolicyArg::Fold,
            NonAsciiPolicy::Transliterate => NonAsciiPolicyArg::Transliterate,
        }
    }
}

#[derive(Clone, Copy, Debug, ValueEnum, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
/// CLI/config representation of line-ending normalization strategy.
pub enum LineEndingChoice {
    Auto,
    Lf,
    Crlf,
    Cr,
}

impl LineEndingChoice {
    /// Convert the CLI/config choice into the library line-ending option.
    ///
    /// # Returns
    /// `None` when line endings should be left unchanged (`auto`), otherwise
    /// the target [`LineEndingStyle`] to enforce.
    pub fn into_option(self) -> Option<LineEndingStyle> {
        match self {
            LineEndingChoice::Auto => None,
            LineEndingChoice::Lf => Some(LineEndingStyle::Lf),
            LineEndingChoice::Crlf => Some(LineEndingStyle::Crlf),
            LineEndingChoice::Cr => Some(LineEndingStyle::Cr),
        }
    }
}

impl From<Option<LineEndingStyle>> for LineEndingChoice {
    fn from(value: Option<LineEndingStyle>) -> Self {
        match value {
            Some(LineEndingStyle::Lf) => LineEndingChoice::Lf,
            Some(LineEndingStyle::Crlf) => LineEndingChoice::Crlf,
            Some(LineEndingStyle::Cr) => LineEndingChoice::Cr,
            None => LineEndingChoice::Auto,
        }
    }
}

#[derive(Clone, Copy, Debug, ValueEnum, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
/// CLI/config representation of Unicode normalization mode.
pub enum UnicodeNormalizationChoice {
    None,
    Nfd,
    Nfc,
    Nfkd,
    Nfkc,
}

impl From<UnicodeNormalizationChoice> for UnicodeNormalizationMode {
    fn from(value: UnicodeNormalizationChoice) -> Self {
        match value {
            UnicodeNormalizationChoice::None => UnicodeNormalizationMode::None,
            UnicodeNormalizationChoice::Nfd => UnicodeNormalizationMode::NFD,
            UnicodeNormalizationChoice::Nfc => UnicodeNormalizationMode::NFC,
            UnicodeNormalizationChoice::Nfkd => UnicodeNormalizationMode::NFKD,
            UnicodeNormalizationChoice::Nfkc => UnicodeNormalizationMode::NFKC,
        }
    }
}

impl From<UnicodeNormalizationMode> for UnicodeNormalizationChoice {
    fn from(value: UnicodeNormalizationMode) -> Self {
        match value {
            UnicodeNormalizationMode::None => UnicodeNormalizationChoice::None,
            UnicodeNormalizationMode::NFD => UnicodeNormalizationChoice::Nfd,
            UnicodeNormalizationMode::NFC => UnicodeNormalizationChoice::Nfc,
            UnicodeNormalizationMode::NFKD => UnicodeNormalizationChoice::Nfkd,
            UnicodeNormalizationMode::NFKC => UnicodeNormalizationChoice::Nfkc,
        }
    }
}

#[derive(Default)]
/// Sparse option overrides from CLI flags.
pub struct PartialOptions {
    pub remove_hidden: Option<bool>,
    pub remove_trailing_whitespace: Option<bool>,
    pub normalize_spaces: Option<bool>,
    pub normalize_dashes: Option<bool>,
    pub normalize_quotes: Option<bool>,
    pub normalize_other: Option<bool>,
    pub keyboard_only: Option<bool>,
    pub extended_keyboard: Option<bool>,
    pub emoji_policy: Option<EmojiPolicyArg>,
    pub non_ascii_policy: Option<NonAsciiPolicyArg>,
    pub preserve_joiners: Option<bool>,
    pub remove_control_chars: Option<bool>,
    pub collapse_whitespace: Option<bool>,
    pub line_endings: Option<LineEndingChoice>,
    pub unicode_normalization: Option<UnicodeNormalizationChoice>,
    #[cfg(feature = "security")]
    pub strip_bidi_controls: Option<bool>,
}

impl PartialOptions {
    /// Apply only explicitly provided option values onto a full options struct.
    pub fn apply_to(self, options: &mut CleaningOptions) {
        if let Some(val) = self.remove_hidden {
            options.remove_hidden = val;
        }
        if let Some(val) = self.remove_trailing_whitespace {
            options.remove_trailing_whitespace = val;
        }
        if let Some(val) = self.normalize_spaces {
            options.normalize_spaces = val;
        }
        if let Some(val) = self.normalize_dashes {
            options.normalize_dashes = val;
        }
        if let Some(val) = self.normalize_quotes {
            options.normalize_quotes = val;
        }
        if let Some(val) = self.normalize_other {
            options.normalize_other = val;
        }
        if let Some(val) = self.keyboard_only {
            options.keyboard_only = val;
        }
        if let Some(val) = self.extended_keyboard {
            options.extended_keyboard = val;
        }
        if let Some(policy) = self.emoji_policy {
            options.emoji_policy = policy.into();
        }
        if let Some(policy) = self.non_ascii_policy {
            options.non_ascii_policy = policy.into();
        }
        if let Some(val) = self.preserve_joiners {
            options.preserve_joiners = val;
        }
        if let Some(val) = self.remove_control_chars {
            options.remove_control_chars = val;
        }
        if let Some(val) = self.collapse_whitespace {
            options.collapse_whitespace = val;
        }
        if let Some(choice) = self.line_endings {
            options.normalize_line_endings = choice.into_option();
        }
        if let Some(choice) = self.unicode_normalization {
            options.unicode_normalization = choice.into();
        }
        #[cfg(feature = "security")]
        if let Some(val) = self.strip_bidi_controls {
            options.strip_bidi_controls = val;
        }
    }
}

/// Shared CLI argument surface used by both `rehuman` and `ishuman`.
#[derive(Args, Debug)]
pub struct SharedCliOptions {
    /// Apply a named preset (for example `code-safe` for docs/source text).
    #[arg(long, value_enum)]
    pub preset: Option<PresetArg>,

    /// Override remove_hidden behavior (true/false, default true)
    #[arg(long, value_name = "BOOL", value_parser = parse_bool_flag, default_missing_value = "true", num_args = 0..=1)]
    pub remove_hidden: Option<bool>,

    /// Override remove_trailing_whitespace behavior (true/false, default true)
    #[arg(long, value_name = "BOOL", value_parser = parse_bool_flag, default_missing_value = "true", num_args = 0..=1)]
    pub remove_trailing_whitespace: Option<bool>,

    /// Override normalize_spaces behavior (true/false, default true)
    #[arg(long, value_name = "BOOL", value_parser = parse_bool_flag, default_missing_value = "true", num_args = 0..=1)]
    pub normalize_spaces: Option<bool>,

    /// Override normalize_dashes behavior (true/false, default true)
    #[arg(long, value_name = "BOOL", value_parser = parse_bool_flag, default_missing_value = "true", num_args = 0..=1)]
    pub normalize_dashes: Option<bool>,

    /// Override normalize_quotes behavior (true/false, default true)
    #[arg(long, value_name = "BOOL", value_parser = parse_bool_flag, default_missing_value = "true", num_args = 0..=1)]
    pub normalize_quotes: Option<bool>,

    /// Override normalize_other behavior (true/false, default true)
    #[arg(long, value_name = "BOOL", value_parser = parse_bool_flag, default_missing_value = "true", num_args = 0..=1)]
    pub normalize_other: Option<bool>,

    /// Override keyboard_only behavior (true/false, default true for CLI)
    #[arg(long, value_name = "BOOL", value_parser = parse_bool_flag, default_missing_value = "true", num_args = 0..=1)]
    pub keyboard_only: Option<bool>,

    /// Allow a curated non-ASCII keyboard allowlist in keyboard-only mode.
    #[arg(long, value_name = "BOOL", value_parser = parse_bool_flag, default_missing_value = "true", num_args = 0..=1)]
    pub extended_keyboard: Option<bool>,

    /// Allow emoji to pass through even when keyboard_only is enabled
    #[arg(long, action = clap::ArgAction::SetTrue, conflicts_with = "emoji_policy")]
    pub keep_emoji: bool,

    /// Explicit emoji policy (drop or keep)
    #[arg(long, value_enum)]
    pub emoji_policy: Option<EmojiPolicyArg>,

    /// Non-ASCII handling in keyboard-only mode (drop/fold/transliterate).
    #[arg(long, value_enum)]
    pub non_ascii_policy: Option<NonAsciiPolicyArg>,

    /// Preserve ZWJ/ZWNJ joiners even when hidden characters are removed.
    #[arg(long, value_name = "BOOL", value_parser = parse_bool_flag, default_missing_value = "true", num_args = 0..=1)]
    pub preserve_joiners: Option<bool>,

    /// Override remove_control_chars behavior (true/false, default true)
    #[arg(long, value_name = "BOOL", value_parser = parse_bool_flag, default_missing_value = "true", num_args = 0..=1)]
    pub remove_control_chars: Option<bool>,

    /// Override collapse_whitespace behavior (true/false, default false)
    #[arg(long, value_name = "BOOL", value_parser = parse_bool_flag, default_missing_value = "true", num_args = 0..=1)]
    pub collapse_whitespace: Option<bool>,

    /// Line ending normalization strategy (auto = preserve input)
    #[arg(long, value_enum)]
    pub line_endings: Option<LineEndingChoice>,

    /// Unicode normalization mode (none/NFD/NFC/NFKD/NFKC)
    #[arg(long, value_enum)]
    pub unicode_normalization: Option<UnicodeNormalizationChoice>,

    /// Strip bidi control characters (true/false, default false)
    #[cfg(feature = "security")]
    #[arg(long, value_name = "BOOL", value_parser = parse_bool_flag, default_missing_value = "true", num_args = 0..=1)]
    pub strip_bidi_controls: Option<bool>,

    /// Path to config file. Defaults to platform config directory.
    #[arg(long, value_name = "PATH")]
    pub config: Option<PathBuf>,
}

impl SharedCliOptions {
    /// Convert shared CLI flags into sparse option overrides.
    ///
    /// # Returns
    /// A [`PartialOptions`] value containing only user-provided overrides.
    pub fn to_partial_options(&self) -> PartialOptions {
        let mut partial = PartialOptions {
            remove_hidden: self.remove_hidden,
            remove_trailing_whitespace: self.remove_trailing_whitespace,
            normalize_spaces: self.normalize_spaces,
            normalize_dashes: self.normalize_dashes,
            normalize_quotes: self.normalize_quotes,
            normalize_other: self.normalize_other,
            keyboard_only: self.keyboard_only,
            extended_keyboard: self.extended_keyboard,
            emoji_policy: None,
            non_ascii_policy: self.non_ascii_policy,
            preserve_joiners: self.preserve_joiners,
            remove_control_chars: self.remove_control_chars,
            collapse_whitespace: self.collapse_whitespace,
            line_endings: self.line_endings,
            unicode_normalization: self.unicode_normalization,
            #[cfg(feature = "security")]
            strip_bidi_controls: self.strip_bidi_controls,
        };

        if self.keep_emoji {
            partial.emoji_policy = Some(EmojiPolicyArg::Keep);
        } else if let Some(policy) = self.emoji_policy {
            partial.emoji_policy = Some(policy);
        }

        partial
    }

    /// Whether emoji policy controls were explicitly specified.
    ///
    /// # Returns
    /// `true` if `--keep-emoji` or `--emoji-policy` was provided.
    pub fn emoji_policy_specified_by_user(&self) -> bool {
        self.keep_emoji || self.emoji_policy.is_some()
    }

    /// Whether non-ASCII policy was explicitly specified.
    ///
    /// # Returns
    /// `true` if `--non-ascii-policy` was provided.
    pub fn non_ascii_policy_specified_by_user(&self) -> bool {
        self.non_ascii_policy.is_some()
    }

    /// Whether extended keyboard mode was explicitly specified.
    ///
    /// # Returns
    /// `true` if `--extended-keyboard` was provided.
    pub fn extended_keyboard_specified_by_user(&self) -> bool {
        self.extended_keyboard.is_some()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(default)]
/// Serializable options payload used by the on-disk config file.
pub struct SerializableOptions {
    pub remove_hidden: bool,
    pub remove_trailing_whitespace: bool,
    pub normalize_spaces: bool,
    pub normalize_dashes: bool,
    pub normalize_quotes: bool,
    pub normalize_other: bool,
    pub keyboard_only: bool,
    pub extended_keyboard: bool,
    pub emoji_policy: EmojiPolicyArg,
    pub non_ascii_policy: NonAsciiPolicyArg,
    pub preserve_joiners: bool,
    pub remove_control_chars: bool,
    pub collapse_whitespace: bool,
    pub line_endings: LineEndingChoice,
    pub unicode_normalization: UnicodeNormalizationChoice,
    #[cfg(feature = "security")]
    pub strip_bidi_controls: bool,
}

impl Default for SerializableOptions {
    fn default() -> Self {
        Self::from_cleaning_options(&default_cli_options())
    }
}

impl SerializableOptions {
    /// Convert serializable config values into runtime cleaning options.
    ///
    /// # Returns
    /// A fully materialized [`CleaningOptions`] value for runtime cleaning.
    pub fn to_cleaning_options(&self) -> CleaningOptions {
        let builder = CleaningOptions::builder()
            .remove_hidden(self.remove_hidden)
            .remove_trailing_whitespace(self.remove_trailing_whitespace)
            .normalize_spaces(self.normalize_spaces)
            .normalize_dashes(self.normalize_dashes)
            .normalize_quotes(self.normalize_quotes)
            .normalize_other(self.normalize_other)
            .keyboard_only(self.keyboard_only)
            .extended_keyboard(self.extended_keyboard)
            .emoji_policy(self.emoji_policy.into())
            .non_ascii_policy(self.non_ascii_policy.into())
            .preserve_joiners(self.preserve_joiners)
            .remove_control_chars(self.remove_control_chars)
            .collapse_whitespace(self.collapse_whitespace)
            .normalize_line_endings(self.line_endings.into_option())
            .unicode_normalization(self.unicode_normalization.into());
        #[cfg(feature = "security")]
        let builder = builder.strip_bidi_controls(self.strip_bidi_controls);
        builder.build()
    }

    /// Build a serializable snapshot from runtime cleaning options.
    ///
    /// # Returns
    /// A config-ready representation of `options`.
    pub fn from_cleaning_options(options: &CleaningOptions) -> Self {
        Self {
            remove_hidden: options.remove_hidden,
            remove_trailing_whitespace: options.remove_trailing_whitespace,
            normalize_spaces: options.normalize_spaces,
            normalize_dashes: options.normalize_dashes,
            normalize_quotes: options.normalize_quotes,
            normalize_other: options.normalize_other,
            keyboard_only: options.keyboard_only,
            extended_keyboard: options.extended_keyboard,
            emoji_policy: options.emoji_policy.into(),
            non_ascii_policy: options.non_ascii_policy.into(),
            preserve_joiners: options.preserve_joiners,
            remove_control_chars: options.remove_control_chars,
            collapse_whitespace: options.collapse_whitespace,
            line_endings: options.normalize_line_endings.into(),
            unicode_normalization: options.unicode_normalization.into(),
            #[cfg(feature = "security")]
            strip_bidi_controls: options.strip_bidi_controls,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Top-level on-disk config schema.
pub struct ConfigFile {
    pub version: u32,
    #[serde(default)]
    pub options: SerializableOptions,
}

impl Default for ConfigFile {
    fn default() -> Self {
        Self {
            version: CONFIG_VERSION,
            options: SerializableOptions::default(),
        }
    }
}

/// Default options used by CLI binaries when no config/overrides are provided.
///
/// # Returns
/// The baseline CLI configuration.
pub fn default_cli_options() -> CleaningOptions {
    CleaningOptions::default()
}

/// Resolve the platform-specific default config path.
///
/// # Returns
/// The default config file path when platform directories are available.
pub fn default_config_path() -> Option<PathBuf> {
    ProjectDirs::from("com", "rehuman", "rehuman").map(|dirs| dirs.config_dir().join("config.toml"))
}

/// Load and validate config file contents.
///
/// # Arguments
/// - `path`: File to read as TOML config.
///
/// # Returns
/// Parsed and validated [`CleaningOptions`].
///
/// # Errors
/// Returns an error if the file cannot be read, parsed, or has an unsupported
/// schema version.
pub fn load_config(path: &Path) -> Result<CleaningOptions> {
    let contents = fs::read_to_string(path)?;
    let config: ConfigFile = toml::from_str(&contents)?;
    if config.version != CONFIG_VERSION {
        bail!(
            "unsupported config version {} (expected {})",
            config.version,
            CONFIG_VERSION
        );
    }
    Ok(config.options.to_cleaning_options())
}

/// Validate that explicitly requested emoji policy is meaningful.
///
/// Emoji policy is only effective when keyboard-only mode is enabled.
///
/// # Arguments
/// - `options`: Fully resolved options after config + CLI overrides.
/// - `emoji_policy_specified_by_user`: Whether the user explicitly set an
///   emoji policy flag on this invocation.
///
/// # Returns
/// `Ok(())` when the combination is coherent.
///
/// # Errors
/// Returns an error when emoji policy was set explicitly while
/// `keyboard_only` is disabled.
pub fn validate_emoji_policy_dependency(
    options: &CleaningOptions,
    emoji_policy_specified_by_user: bool,
) -> Result<()> {
    validate_keyboard_only_dependency(
        options,
        emoji_policy_specified_by_user,
        "'--keep-emoji'/'--emoji-policy'",
    )
}

/// Validate that explicit non-ASCII handling is meaningful.
///
/// Non-ASCII policy is only effective when keyboard-only mode is enabled.
///
/// # Arguments
/// - `options`: Fully resolved options after config + CLI overrides.
/// - `non_ascii_policy_specified_by_user`: Whether the user explicitly set
///   non-ASCII handling on this invocation.
///
/// # Returns
/// `Ok(())` when the combination is coherent.
///
/// # Errors
/// Returns an error when non-ASCII policy was set explicitly while
/// `keyboard_only` is disabled.
pub fn validate_non_ascii_policy_dependency(
    options: &CleaningOptions,
    non_ascii_policy_specified_by_user: bool,
) -> Result<()> {
    validate_keyboard_only_dependency(
        options,
        non_ascii_policy_specified_by_user,
        "'--non-ascii-policy'",
    )
}

/// Validate that explicit extended keyboard mode is meaningful.
///
/// Extended keyboard mode is only effective when keyboard-only mode is enabled.
///
/// # Arguments
/// - `options`: Fully resolved options after config + CLI overrides.
/// - `extended_keyboard_specified_by_user`: Whether the user explicitly set
///   extended keyboard mode on this invocation.
///
/// # Returns
/// `Ok(())` when the combination is coherent.
///
/// # Errors
/// Returns an error when extended keyboard mode was set explicitly while
/// `keyboard_only` is disabled.
pub fn validate_extended_keyboard_dependency(
    options: &CleaningOptions,
    extended_keyboard_specified_by_user: bool,
) -> Result<()> {
    validate_keyboard_only_dependency(
        options,
        extended_keyboard_specified_by_user,
        "'--extended-keyboard'",
    )
}

/// Read input text from a path or stdin with size checks.
///
/// # Arguments
/// - `input_path`: Optional input file path; when `None`, reads stdin.
/// - `max_bytes`: Maximum allowed payload size.
///
/// # Returns
/// The full input text.
///
/// # Errors
/// Returns an error if no stdin data is present, input exceeds `max_bytes`, or
/// file/stdin reads fail.
pub fn read_input(input_path: Option<&Path>, max_bytes: usize) -> Result<String> {
    if let Some(path) = input_path {
        let metadata =
            fs::metadata(path).with_context(|| format!("failed to access {}", path.display()))?;
        if metadata.len() as usize > max_bytes {
            bail!(
                "input file {} exceeds maximum supported size of {} bytes",
                path.display(),
                max_bytes
            );
        }
        let content = fs::read_to_string(path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        Ok(content)
    } else {
        let stdin = io::stdin();
        if stdin.is_terminal() {
            bail!("no input provided; pass a file path or pipe data into stdin");
        }
        let mut buffer = String::new();
        stdin
            .lock()
            .read_to_string(&mut buffer)
            .context("failed to read from stdin")?;
        if buffer.len() > max_bytes {
            bail!(
                "stdin stream exceeds maximum supported size of {} bytes",
                max_bytes
            );
        }
        Ok(buffer)
    }
}

/// Emit human-readable stats to stderr.
pub fn write_stats(result: &CleaningResult<'_>) {
    let stats = &result.stats;
    eprintln!("changes_made: {}", result.changes_made);
    eprintln!("  hidden_chars_removed: {}", stats.hidden_chars_removed);
    eprintln!(
        "  trailing_whitespace_removed: {}",
        stats.trailing_whitespace_removed
    );
    eprintln!("  spaces_normalized: {}", stats.spaces_normalized);
    eprintln!("  dashes_normalized: {}", stats.dashes_normalized);
    eprintln!("  quotes_normalized: {}", stats.quotes_normalized);
    eprintln!("  other_normalized: {}", stats.other_normalized);
    eprintln!("  control_chars_removed: {}", stats.control_chars_removed);
    eprintln!(
        "  line_endings_normalized: {}",
        stats.line_endings_normalized
    );
    eprintln!("  non_keyboard_removed: {}", stats.non_keyboard_removed);
    eprintln!(
        "  non_keyboard_transliterated: {}",
        stats.non_keyboard_transliterated
    );
    eprintln!("  emojis_dropped: {}", stats.emojis_dropped);
}

/// Parse a flexible boolean flag value.
///
/// # Returns
/// Parsed boolean value for common truthy/falsey spellings.
///
/// # Errors
/// Returns `Err(String)` for unsupported values.
pub fn parse_bool_flag(value: &str) -> std::result::Result<bool, String> {
    match value.trim().to_ascii_lowercase().as_str() {
        "true" | "t" | "1" | "yes" | "y" | "on" => Ok(true),
        "false" | "f" | "0" | "no" | "n" | "off" => Ok(false),
        other => Err(format!("invalid boolean value '{other}'")),
    }
}

#[derive(Serialize)]
/// JSON-serializable summary payload for stats output.
pub struct StatsSummary<'a> {
    pub changed: bool,
    pub changes_made: u64,
    pub stats: &'a CleaningStats,
}

fn validate_keyboard_only_dependency(
    options: &CleaningOptions,
    dependent_flag_specified_by_user: bool,
    flag_label: &str,
) -> Result<()> {
    if dependent_flag_specified_by_user && !options.keyboard_only {
        bail!(
            "{flag_label} requires keyboard-only mode; set '--keyboard-only true' or remove the {flag_label} flag"
        );
    }
    Ok(())
}

/// Serialize JSON stats payload and append a trailing newline.
///
/// # Arguments
/// - `writer`: Destination stream.
/// - `summary`: Stats payload to serialize.
///
/// # Returns
/// `Ok(())` once JSON payload is written.
///
/// # Errors
/// Returns an error if serialization or writing fails.
pub fn write_stats_json<W: Write>(writer: &mut W, summary: &StatsSummary) -> Result<()> {
    serde_json::to_writer_pretty(&mut *writer, summary)
        .context("failed to serialize JSON stats")?;
    writer
        .write_all(b"\n")
        .context("failed to write JSON stats newline")?;
    writer
        .flush()
        .context("failed to flush JSON stats output")?;
    Ok(())
}
