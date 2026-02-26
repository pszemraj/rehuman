use std::fs;
use std::io::{self, BufRead, IsTerminal, Read, Write};
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use clap::ValueEnum;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

use rehuman::{
    CleaningOptions, CleaningResult, CleaningStats, EmojiPolicy, LineEndingStyle, StreamCleaner,
    TextCleaner, UnicodeNormalizationMode,
};

pub const MAX_INPUT_BYTES: usize = 5 * 1024 * 1024;
pub const CONFIG_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, ValueEnum, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
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
#[serde(rename_all = "lowercase")]
pub enum LineEndingChoice {
    Auto,
    Lf,
    Crlf,
    Cr,
}

impl LineEndingChoice {
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
pub struct PartialOptions {
    pub remove_hidden: Option<bool>,
    pub remove_trailing_whitespace: Option<bool>,
    pub normalize_spaces: Option<bool>,
    pub normalize_dashes: Option<bool>,
    pub normalize_quotes: Option<bool>,
    pub normalize_other: Option<bool>,
    pub keyboard_only: Option<bool>,
    pub emoji_policy: Option<EmojiPolicyArg>,
    pub remove_control_chars: Option<bool>,
    pub collapse_whitespace: Option<bool>,
    pub line_endings: Option<LineEndingChoice>,
    pub unicode_normalization: Option<UnicodeNormalizationChoice>,
    #[cfg(feature = "security")]
    pub strip_bidi_controls: Option<bool>,
}

impl PartialOptions {
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
        if let Some(policy) = self.emoji_policy {
            options.emoji_policy = policy.into();
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(default)]
pub struct SerializableOptions {
    pub remove_hidden: bool,
    pub remove_trailing_whitespace: bool,
    pub normalize_spaces: bool,
    pub normalize_dashes: bool,
    pub normalize_quotes: bool,
    pub normalize_other: bool,
    pub keyboard_only: bool,
    pub emoji_policy: EmojiPolicyArg,
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
    pub fn to_cleaning_options(&self) -> CleaningOptions {
        let builder = CleaningOptions::builder()
            .remove_hidden(self.remove_hidden)
            .remove_trailing_whitespace(self.remove_trailing_whitespace)
            .normalize_spaces(self.normalize_spaces)
            .normalize_dashes(self.normalize_dashes)
            .normalize_quotes(self.normalize_quotes)
            .normalize_other(self.normalize_other)
            .keyboard_only(self.keyboard_only)
            .emoji_policy(self.emoji_policy.into())
            .remove_control_chars(self.remove_control_chars)
            .collapse_whitespace(self.collapse_whitespace)
            .normalize_line_endings(self.line_endings.into_option())
            .unicode_normalization(self.unicode_normalization.into());
        #[cfg(feature = "security")]
        let builder = builder.strip_bidi_controls(self.strip_bidi_controls);
        builder.build()
    }

    pub fn from_cleaning_options(options: &CleaningOptions) -> Self {
        Self {
            remove_hidden: options.remove_hidden,
            remove_trailing_whitespace: options.remove_trailing_whitespace,
            normalize_spaces: options.normalize_spaces,
            normalize_dashes: options.normalize_dashes,
            normalize_quotes: options.normalize_quotes,
            normalize_other: options.normalize_other,
            keyboard_only: options.keyboard_only,
            emoji_policy: options.emoji_policy.into(),
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

pub fn default_cli_options() -> CleaningOptions {
    CleaningOptions::builder()
        .keyboard_only(true)
        .emoji_policy(EmojiPolicy::Drop)
        .build()
}

pub fn default_config_path() -> Option<PathBuf> {
    ProjectDirs::from("com", "rehuman", "rehuman").map(|dirs| dirs.config_dir().join("config.toml"))
}

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

pub fn validate_emoji_policy_dependency(
    options: &CleaningOptions,
    emoji_policy_specified_by_user: bool,
) -> Result<()> {
    if emoji_policy_specified_by_user && !options.keyboard_only {
        bail!(
            "'--keep-emoji'/'--emoji-policy' require keyboard-only mode; set '--keyboard-only true' or remove emoji policy flags"
        );
    }
    Ok(())
}

#[allow(dead_code)]
pub fn save_config(path: &Path, options: &CleaningOptions) -> Result<()> {
    let cfg = ConfigFile {
        version: CONFIG_VERSION,
        options: SerializableOptions::from_cleaning_options(options),
    };
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create config directory {}", parent.display()))?;
    }
    let data = toml::to_string_pretty(&cfg)?;
    fs::write(path, data)?;
    Ok(())
}

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

#[allow(dead_code)]
pub fn write_output(result: &CleaningResult<'_>) -> Result<()> {
    let mut stdout = io::stdout().lock();
    stdout
        .write_all(result.text.as_bytes())
        .context("failed to write to stdout")?;
    Ok(())
}

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
    eprintln!("  emojis_dropped: {}", stats.emojis_dropped);
}

pub fn parse_bool_flag(value: &str) -> std::result::Result<bool, String> {
    match value.trim().to_ascii_lowercase().as_str() {
        "true" | "t" | "1" | "yes" | "y" | "on" => Ok(true),
        "false" | "f" | "0" | "no" | "n" | "off" => Ok(false),
        other => Err(format!("invalid boolean value '{other}'")),
    }
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct StreamOutcome {
    pub stats: CleaningStats,
    pub changes_made: u64,
}

#[derive(Serialize)]
pub struct StatsSummary<'a> {
    pub changed: bool,
    pub changes_made: u64,
    pub stats: &'a CleaningStats,
}

#[allow(dead_code)]
pub fn clean_stream<R, W>(
    reader: &mut R,
    writer: &mut W,
    cleaner: &TextCleaner,
) -> Result<StreamOutcome>
where
    R: BufRead,
    W: Write,
{
    let mut stream = StreamCleaner::new(cleaner.options().clone());
    let mut buffer = String::new();
    let mut chunk_output = String::new();

    loop {
        buffer.clear();
        let bytes_read = reader
            .read_line(&mut buffer)
            .context("failed to read input stream")?;
        if bytes_read == 0 {
            break;
        }
        if let Some(result) = stream.feed(&buffer, &mut chunk_output) {
            writer
                .write_all(result.text.as_bytes())
                .context("failed to write stream chunk")?;
            chunk_output.clear();
        }
    }

    if let Some(result) = stream.finish(&mut chunk_output) {
        writer
            .write_all(result.text.as_bytes())
            .context("failed to write stream chunk")?;
        chunk_output.clear();
    }

    writer.flush().context("failed to flush output stream")?;

    let summary = stream.summary();
    Ok(StreamOutcome {
        stats: summary.stats,
        changes_made: summary.changes_made,
    })
}

pub fn write_stats_json<W: Write>(writer: &mut W, summary: &StatsSummary) -> Result<()> {
    serde_json::to_writer_pretty(&mut *writer, summary)
        .context("failed to serialize JSON stats")?;
    writer.write_all(b"\n").ok();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_defaults_match_library_defaults() {
        let cli_defaults = default_cli_options();
        let library_defaults = CleaningOptions::default();
        assert_eq!(
            cli_defaults, library_defaults,
            "CLI default options should mirror library defaults"
        );
    }

    #[test]
    fn config_rejects_unknown_option_fields() {
        let bad = r#"
version = 1
[options]
keyboard_only = true
normalise_spaces = false
"#;
        let err = toml::from_str::<ConfigFile>(bad).expect_err("unknown fields should fail");
        assert!(
            err.to_string().contains("unknown field"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn emoji_policy_dependency_requires_keyboard_mode_when_explicit() {
        let mut options = default_cli_options();
        options.keyboard_only = false;
        let err = validate_emoji_policy_dependency(&options, true)
            .expect_err("explicit emoji policy must require keyboard mode");
        assert!(
            err.to_string().contains("require keyboard-only mode"),
            "unexpected error: {err}"
        );
    }
}
