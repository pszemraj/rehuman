//! `ishuman` CLI entrypoint and argument routing.

mod common;

use std::io;
use std::path::PathBuf;

use anyhow::Result;
use clap::{ArgAction, Parser};

use common::{
    default_config_path, read_input, resolve_options, write_stats, write_stats_json,
    SharedCliOptions, StatsSummary, MAX_INPUT_BYTES,
};
use rehuman::TextCleaner;

fn main() -> Result<()> {
    let exit_code = run()?;
    std::process::exit(exit_code);
}

fn run() -> Result<i32> {
    let cli = Cli::parse();

    let config_path = cli.shared.config.clone().or_else(default_config_path);

    let options = resolve_options(&cli.shared, config_path.as_deref())?;

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
