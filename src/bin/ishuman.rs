//! `ishuman` CLI entrypoint and argument routing.

mod common;

use std::io;
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{ArgAction, Parser};

use common::{
    default_cli_options, default_config_path, load_config, options_from_preset, read_input,
    validate_emoji_policy_dependency, validate_extended_keyboard_dependency,
    validate_non_ascii_policy_dependency, write_stats, write_stats_json, SharedCliOptions,
    StatsSummary, MAX_INPUT_BYTES,
};
use rehuman::TextCleaner;

fn main() -> Result<()> {
    let exit_code = run()?;
    std::process::exit(exit_code);
}

fn run() -> Result<i32> {
    let cli = Cli::parse();

    let config_path = cli.shared.config.clone().or_else(default_config_path);

    let mut options = default_cli_options();

    if let Some(ref path) = config_path {
        if path.exists() {
            options = load_config(path)
                .with_context(|| format!("failed to read config at {}", path.display()))?;
        }
    }

    if let Some(preset) = cli.shared.preset {
        options = options_from_preset(preset);
    }

    let overrides = cli.shared.to_partial_options();
    overrides.apply_to(&mut options);
    validate_emoji_policy_dependency(&options, cli.shared.emoji_policy_specified_by_user())?;
    validate_non_ascii_policy_dependency(
        &options,
        cli.shared.non_ascii_policy_specified_by_user(),
    )?;
    validate_extended_keyboard_dependency(
        &options,
        cli.shared.extended_keyboard_specified_by_user(),
    )?;

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

    #[command(flatten)]
    shared: SharedCliOptions,

    /// Print a summary of potential transformations to stderr.
    #[arg(long, short = 's', action = ArgAction::SetTrue)]
    stats: bool,

    /// Emit a JSON summary of potential transformations to stdout.
    #[arg(long = "json", action = ArgAction::SetTrue)]
    stats_json: bool,
}
