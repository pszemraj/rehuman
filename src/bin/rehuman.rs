//! `rehuman` CLI entrypoint and argument routing.

mod common;

use std::borrow::Cow;
use std::fs;
use std::io::{self, BufRead, BufReader, BufWriter, IsTerminal, Write};
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use clap::{ArgAction, Parser};
use tempfile::NamedTempFile;

use common::{
    default_cli_options, default_config_path, load_config, options_from_preset, read_input,
    validate_emoji_policy_dependency, validate_extended_keyboard_dependency,
    validate_non_ascii_policy_dependency, write_stats, write_stats_json, ConfigFile,
    SerializableOptions, SharedCliOptions, StatsSummary, CONFIG_VERSION, MAX_INPUT_BYTES,
};
use rehuman::{CleaningResult, CleaningStats, StreamCleaner, TextCleaner};

fn main() -> Result<()> {
    let cli = Cli::parse();

    let config_path = cli.shared.config.clone().or_else(default_config_path);

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

#[derive(Debug)]
struct StreamOutcome {
    stats: CleaningStats,
    changes_made: u64,
}

fn save_config(path: &Path, options: &rehuman::CleaningOptions) -> Result<()> {
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

fn write_output(result: &CleaningResult<'_>) -> Result<()> {
    let mut stdout = io::stdout().lock();
    stdout
        .write_all(result.text.as_bytes())
        .context("failed to write to stdout")?;
    Ok(())
}

fn clean_stream<R, W>(
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

    #[command(flatten)]
    shared: SharedCliOptions,

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

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
    fn write_stats_json_emits_trailing_newline() {
        let stats = CleaningStats::default();
        let summary = StatsSummary {
            changed: false,
            changes_made: 0,
            stats: &stats,
        };
        let mut out = Vec::<u8>::new();
        write_stats_json(&mut out, &summary).expect("JSON stats should serialize");
        assert_eq!(out.last(), Some(&b'\n'));
    }

    struct FlushErrorWriter;

    impl Write for FlushErrorWriter {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Err(io::Error::new(io::ErrorKind::BrokenPipe, "flush failed"))
        }
    }

    #[test]
    fn write_stats_json_propagates_flush_errors() {
        let stats = CleaningStats::default();
        let summary = StatsSummary {
            changed: true,
            changes_made: 1,
            stats: &stats,
        };
        let mut out = FlushErrorWriter;
        let err = write_stats_json(&mut out, &summary).expect_err("flush errors should surface");
        assert!(
            err.to_string()
                .contains("failed to flush JSON stats output"),
            "unexpected error: {err}"
        );
    }
}
