mod common;

use std::io;
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{ArgAction, Parser};

use common::{
    default_cli_options, default_config_path, load_config, parse_bool_flag, read_input,
    write_stats, write_stats_json, EmojiPolicyArg, LineEndingChoice, PartialOptions, StatsSummary,
    UnicodeNormalizationChoice, MAX_INPUT_BYTES,
};
use rehuman::TextCleaner;

fn main() -> Result<()> {
    let exit_code = run()?;
    std::process::exit(exit_code);
}

fn run() -> Result<i32> {
    let cli = Cli::parse();

    let config_path = cli.config.clone().or_else(default_config_path);

    let mut options = default_cli_options();

    if let Some(ref path) = config_path {
        if path.exists() {
            options = load_config(path)
                .with_context(|| format!("failed to read config at {}", path.display()))?;
        }
    }

    let overrides = cli.to_partial_options();
    overrides.apply_to(&mut options);

    let input = read_input(cli.input.as_deref(), MAX_INPUT_BYTES)?;

    let cleaner = TextCleaner::new(options);
    let result = cleaner.clean(&input);

    let is_clean = result.changes_made == 0;

    if cli.stats {
        write_stats(&result);
    }

    if cli.stats_json {
        let summary = StatsSummary {
            changed: !is_clean,
            changes_made: result.changes_made,
            stats: &result.stats,
        };
        let mut stdout = io::stdout().lock();
        write_stats_json(&mut stdout, &summary)?;
    }

    Ok(if is_clean { 0 } else { 1 })
}

#[derive(Parser, Debug)]
#[command(
    name = "ishuman",
    about = "Check whether text already complies with rehuman cleaning rules",
    version,
    author
)]
struct Cli {
    /// Path to the input file. Reads from STDIN when omitted.
    #[arg(value_name = "INPUT")]
    input: Option<PathBuf>,

    /// Override remove_hidden behavior (true/false, default true)
    #[arg(long, value_name = "BOOL", value_parser = parse_bool_flag, default_missing_value = "true", num_args = 0..=1)]
    remove_hidden: Option<bool>,

    /// Override remove_trailing_whitespace behavior (true/false, default true)
    #[arg(long, value_name = "BOOL", value_parser = parse_bool_flag, default_missing_value = "true", num_args = 0..=1)]
    remove_trailing_whitespace: Option<bool>,

    /// Override normalize_spaces behavior (true/false, default true)
    #[arg(long, value_name = "BOOL", value_parser = parse_bool_flag, default_missing_value = "true", num_args = 0..=1)]
    normalize_spaces: Option<bool>,

    /// Override normalize_dashes behavior (true/false, default true)
    #[arg(long, value_name = "BOOL", value_parser = parse_bool_flag, default_missing_value = "true", num_args = 0..=1)]
    normalize_dashes: Option<bool>,

    /// Override normalize_quotes behavior (true/false, default true)
    #[arg(long, value_name = "BOOL", value_parser = parse_bool_flag, default_missing_value = "true", num_args = 0..=1)]
    normalize_quotes: Option<bool>,

    /// Override normalize_other behavior (true/false, default true)
    #[arg(long, value_name = "BOOL", value_parser = parse_bool_flag, default_missing_value = "true", num_args = 0..=1)]
    normalize_other: Option<bool>,

    /// Override keyboard_only behavior (true/false, default true for CLI)
    #[arg(long, value_name = "BOOL", value_parser = parse_bool_flag, default_missing_value = "true", num_args = 0..=1)]
    keyboard_only: Option<bool>,

    /// Allow emoji to pass through even when keyboard_only is enabled
    #[arg(long, action = ArgAction::SetTrue, conflicts_with = "emoji_policy")]
    keep_emoji: bool,

    /// Explicit emoji policy (drop or keep)
    #[arg(long, value_enum)]
    emoji_policy: Option<EmojiPolicyArg>,

    /// Override remove_control_chars behavior (true/false, default true)
    #[arg(long, value_name = "BOOL", value_parser = parse_bool_flag, default_missing_value = "true", num_args = 0..=1)]
    remove_control_chars: Option<bool>,

    /// Override collapse_whitespace behavior (true/false, default false)
    #[arg(long, value_name = "BOOL", value_parser = parse_bool_flag, default_missing_value = "true", num_args = 0..=1)]
    collapse_whitespace: Option<bool>,

    /// Line ending normalization strategy (auto = preserve input)
    #[arg(long, value_enum)]
    line_endings: Option<LineEndingChoice>,

    /// Unicode normalization mode (none/NFD/NFC/NFKD/NFKC)
    #[arg(long, value_enum)]
    unicode_normalization: Option<UnicodeNormalizationChoice>,

    /// Strip bidi control characters (true/false, default false)
    #[cfg(feature = "security")]
    #[arg(long, value_name = "BOOL", value_parser = parse_bool_flag, default_missing_value = "true", num_args = 0..=1)]
    strip_bidi_controls: Option<bool>,

    /// Path to config file. Defaults to platform config directory.
    #[arg(long, value_name = "PATH")]
    config: Option<PathBuf>,

    /// Print a summary of potential transformations to stderr.
    #[arg(long, short = 's', action = ArgAction::SetTrue)]
    stats: bool,

    /// Emit a JSON summary of potential transformations to stdout.
    #[arg(long = "json", action = ArgAction::SetTrue)]
    stats_json: bool,

    /// Deprecated: exit status now reflects cleanliness by default (flag is ignored).
    #[arg(long, action = ArgAction::SetTrue)]
    exit_code: bool,
}

impl Cli {
    fn to_partial_options(&self) -> PartialOptions {
        let mut partial = PartialOptions {
            remove_hidden: self.remove_hidden,
            remove_trailing_whitespace: self.remove_trailing_whitespace,
            normalize_spaces: self.normalize_spaces,
            normalize_dashes: self.normalize_dashes,
            normalize_quotes: self.normalize_quotes,
            normalize_other: self.normalize_other,
            keyboard_only: self.keyboard_only,
            emoji_policy: None,
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
}
