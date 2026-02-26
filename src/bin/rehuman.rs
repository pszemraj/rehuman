//! `rehuman` CLI entrypoint and argument routing.

mod common;

use std::borrow::Cow;
use std::fs;
use std::io::{self, BufReader, BufWriter, IsTerminal};
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use clap::{ArgAction, Parser};
use tempfile::NamedTempFile;

use common::{
    clean_stream, default_cli_options, default_config_path, load_config, options_from_preset,
    parse_bool_flag, read_input, save_config, validate_emoji_policy_dependency,
    validate_extended_keyboard_dependency, validate_non_ascii_policy_dependency, write_output,
    write_stats, write_stats_json, ConfigFile, EmojiPolicyArg, LineEndingChoice, NonAsciiPolicyArg,
    PartialOptions, PresetArg, SerializableOptions, StatsSummary, UnicodeNormalizationChoice,
    CONFIG_VERSION, MAX_INPUT_BYTES,
};
use rehuman::{CleaningResult, TextCleaner};

fn main() -> Result<()> {
    let cli = Cli::parse();

    let config_path = cli.config.clone().or_else(default_config_path);

    if cli.reset_config {
        if let Some(ref path) = config_path {
            if path.exists() {
                fs::remove_file(path)
                    .with_context(|| format!("failed to remove config at {}", path.display()))?;
            }
        } else {
            bail!(
                "unable to determine config path; specify '--config <path>' when using '--reset-config'"
            );
        }
    }

    let mut options = default_cli_options();

    if let Some(ref path) = config_path {
        if path.exists() {
            options = load_config(path)
                .with_context(|| format!("failed to read config at {}", path.display()))?;
        }
    }

    if let Some(preset) = cli.preset {
        options = options_from_preset(preset);
    }

    let overrides = cli.to_partial_options();
    overrides.apply_to(&mut options);
    validate_emoji_policy_dependency(&options, cli.keep_emoji || cli.emoji_policy.is_some())?;
    validate_non_ascii_policy_dependency(&options, cli.non_ascii_policy.is_some())?;
    validate_extended_keyboard_dependency(&options, cli.extended_keyboard.is_some())?;

    if cli.save_config {
        if let Some(ref path) = config_path {
            save_config(path, &options)
                .with_context(|| format!("failed to write config to {}", path.display()))?;
        } else {
            bail!(
                "unable to determine config path; specify '--config <path>' when using '--save-config'"
            );
        }
    }

    if cli.print_config {
        let snapshot = ConfigFile {
            version: CONFIG_VERSION,
            options: SerializableOptions::from_cleaning_options(&options),
        };
        let toml = toml::to_string_pretty(&snapshot)?;
        println!("{toml}");
        return Ok(());
    }

    let stdin_is_terminal = std::io::stdin().is_terminal();
    if cli.input.is_none() && stdin_is_terminal {
        if cli.save_config || cli.reset_config {
            return Ok(());
        }
        bail!("no input provided; pass a file path or pipe data into stdin");
    }

    if cli.inplace && cli.input.is_none() {
        bail!("'--inplace' requires an explicit file path input");
    }

    let cleaner = TextCleaner::new(options.clone());

    let (aggregate_stats, changes_made) = if cli.inplace {
        let input_path = cli
            .input
            .as_ref()
            .expect("checked above that inplace requires an input path");
        let file = fs::File::open(input_path)
            .with_context(|| format!("failed to open {}", input_path.display()))?;
        let metadata = file
            .metadata()
            .with_context(|| format!("failed to read metadata for {}", input_path.display()))?;
        let parent = input_path.parent().unwrap_or_else(|| Path::new("."));

        let mut reader = BufReader::new(file);
        let mut temp = NamedTempFile::new_in(parent)
            .with_context(|| format!("failed to create temporary file in {}", parent.display()))?;

        let outcome = {
            let mut writer = BufWriter::new(temp.as_file_mut());
            clean_stream(&mut reader, &mut writer, &cleaner)?
        };

        if outcome.changes_made > 0 {
            let permissions = metadata.permissions();
            temp.persist(input_path)
                .with_context(|| format!("failed to replace {}", input_path.display()))?;
            fs::set_permissions(input_path, permissions).with_context(|| {
                format!("failed to restore permissions for {}", input_path.display())
            })?;
        } else {
            temp.close()
                .context("failed to remove temporary file after no-op inplace run")?;
        }
        (outcome.stats, outcome.changes_made)
    } else if cli.stream {
        let outcome = if let Some(ref path) = cli.input {
            let file = fs::File::open(path)
                .with_context(|| format!("failed to open {}", path.display()))?;
            let mut reader = BufReader::new(file);
            let stdout = io::stdout();
            let mut handle = stdout.lock();
            let mut writer = BufWriter::new(&mut handle);
            clean_stream(&mut reader, &mut writer, &cleaner)?
        } else {
            let stdin = io::stdin();
            let handle = stdin.lock();
            let mut reader = BufReader::new(handle);
            let stdout = io::stdout();
            let mut handle = stdout.lock();
            let mut writer = BufWriter::new(&mut handle);
            clean_stream(&mut reader, &mut writer, &cleaner)?
        };

        (outcome.stats, outcome.changes_made)
    } else {
        let input = read_input(cli.input.as_deref(), MAX_INPUT_BYTES)?;
        let result = cleaner.clean(&input);
        write_output(&result)?;
        (result.stats.clone(), result.changes_made)
    };

    if cli.stats {
        let stats_result = CleaningResult {
            text: Cow::Owned(String::new()),
            changes_made,
            stats: aggregate_stats.clone(),
        };
        write_stats(&stats_result);
    }

    if cli.stats_json {
        let summary = StatsSummary {
            changed: changes_made > 0,
            changes_made,
            stats: &aggregate_stats,
        };
        let mut stderr = io::stderr().lock();
        write_stats_json(&mut stderr, &summary)?;
    }

    if cli.exit_code {
        std::process::exit(if changes_made == 0 { 0 } else { 1 });
    }

    Ok(())
}

#[derive(Parser, Debug)]
#[command(
    name = "rehuman",
    about = "Normalize text into keyboard-friendly characters",
    version,
    author
)]
struct Cli {
    /// Path to the input file. Reads from STDIN when omitted.
    #[arg(value_name = "INPUT")]
    input: Option<PathBuf>,

    /// Apply a named preset (for example `code-safe` for docs/source text).
    #[arg(long, value_enum)]
    preset: Option<PresetArg>,

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

    /// Allow a curated non-ASCII keyboard allowlist in keyboard-only mode.
    #[arg(long, value_name = "BOOL", value_parser = parse_bool_flag, default_missing_value = "true", num_args = 0..=1)]
    extended_keyboard: Option<bool>,

    /// Allow emoji to pass through even when keyboard_only is enabled
    #[arg(long, action = ArgAction::SetTrue, conflicts_with = "emoji_policy")]
    keep_emoji: bool,

    /// Explicit emoji policy (drop or keep)
    #[arg(long, value_enum)]
    emoji_policy: Option<EmojiPolicyArg>,

    /// Non-ASCII handling in keyboard-only mode (drop/fold/transliterate).
    #[arg(long, value_enum)]
    non_ascii_policy: Option<NonAsciiPolicyArg>,

    /// Preserve ZWJ/ZWNJ joiners even when hidden characters are removed.
    #[arg(long, value_name = "BOOL", value_parser = parse_bool_flag, default_missing_value = "true", num_args = 0..=1)]
    preserve_joiners: Option<bool>,

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

    /// Persist the resolved configuration back to the config file.
    #[arg(long, action = ArgAction::SetTrue)]
    save_config: bool,

    /// Print the resolved configuration (TOML) and exit.
    #[arg(
        long,
        action = ArgAction::SetTrue,
        conflicts_with_all = [
            "save_config",
            "reset_config",
            "stats",
            "stats_json",
            "exit_code",
            "stream",
            "inplace",
            "input",
        ]
    )]
    print_config: bool,

    /// Remove the stored config file before applying overrides.
    #[arg(long, action = ArgAction::SetTrue)]
    reset_config: bool,

    /// Print a summary of applied transformations to stderr.
    #[arg(long, short = 's', action = ArgAction::SetTrue)]
    stats: bool,

    /// Emit a JSON summary of changes to stderr.
    #[arg(long = "stats-json", action = ArgAction::SetTrue)]
    stats_json: bool,

    /// Set the process exit code to 1 when changes are made.
    #[arg(long, action = ArgAction::SetTrue)]
    exit_code: bool,

    /// Process the input in a streaming fashion (line by line).
    #[arg(long, action = ArgAction::SetTrue, conflicts_with = "inplace")]
    stream: bool,

    /// Apply the transformation directly to the input file.
    #[arg(long = "inplace", action = ArgAction::SetTrue, conflicts_with = "stream")]
    inplace: bool,
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clap_rejects_stream_and_inplace_together() {
        let parsed = Cli::try_parse_from(["rehuman", "--stream", "--inplace", "input.txt"]);
        assert!(parsed.is_err(), "expected clap conflict error");
    }

    #[test]
    fn clap_rejects_print_config_with_processing_flags() {
        let parsed = Cli::try_parse_from(["rehuman", "--print-config", "--stats"]);
        assert!(parsed.is_err(), "expected clap conflict error");
    }

    #[test]
    fn emoji_policy_requires_keyboard_mode_when_explicit() {
        let cli = Cli::try_parse_from([
            "rehuman",
            "--keyboard-only",
            "false",
            "--emoji-policy",
            "drop",
            "input.txt",
        ])
        .expect("args should parse");
        let mut options = default_cli_options();
        cli.to_partial_options().apply_to(&mut options);
        let check = validate_emoji_policy_dependency(
            &options,
            cli.keep_emoji || cli.emoji_policy.is_some(),
        );
        assert!(check.is_err(), "dependency check should fail");
    }

    #[test]
    fn non_ascii_policy_requires_keyboard_mode_when_explicit() {
        let cli = Cli::try_parse_from([
            "rehuman",
            "--keyboard-only",
            "false",
            "--non-ascii-policy",
            "transliterate",
            "input.txt",
        ])
        .expect("args should parse");
        let mut options = default_cli_options();
        cli.to_partial_options().apply_to(&mut options);
        let check = validate_non_ascii_policy_dependency(&options, cli.non_ascii_policy.is_some());
        assert!(check.is_err(), "dependency check should fail");
    }

    #[test]
    fn extended_keyboard_requires_keyboard_mode_when_explicit() {
        let cli = Cli::try_parse_from([
            "rehuman",
            "--keyboard-only",
            "false",
            "--extended-keyboard",
            "true",
            "input.txt",
        ])
        .expect("args should parse");
        let mut options = default_cli_options();
        cli.to_partial_options().apply_to(&mut options);
        let check =
            validate_extended_keyboard_dependency(&options, cli.extended_keyboard.is_some());
        assert!(check.is_err(), "dependency check should fail");
    }
}
