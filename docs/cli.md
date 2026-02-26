# CLI Guide

This document describes CLI behavior.
For transformation semantics and option meanings at the library level, see [API Reference](api.md).

rehuman ships two binaries:

- `rehuman` - cleans input text
- `ishuman` - reports whether text would change

Both tools share the same configuration options and support stdin, files, and configuration files.

---

- [CLI Guide](#cli-guide)
  - [rehuman](#rehuman)
    - [Output Options](#output-options)
    - [Processing Modes](#processing-modes)
    - [Configuration](#configuration)
    - [File Size Limit](#file-size-limit)
  - [ishuman](#ishuman)

---

## rehuman

Cleans text from stdin or a file and writes the normalized output to stdout.

```bash
# Clean a file, write to stdout
rehuman notes.txt

# Save to a different file
rehuman notes.txt > notes.clean.txt

# Pipe data
curl https://example.com | rehuman --stats

# Rewrite a file in place (safe temporary swap)
rehuman --inplace notes.txt

# Stream mode for large inputs (line by line)
rehuman --stream < huge.log > huge.clean.log
```

> [!NOTE]
> The CLI shares its defaults with the `rehuman::clean` helper: keyboard-only output with emoji dropped and non-ASCII transliterated when feasible.

### Output Options

| Flag                             | Description                                                      |
| -------------------------------- | ---------------------------------------------------------------- |
| `--preset <name>`                | Apply named baseline options: `minimal`, `balanced`, `humanize`, `aggressive`, `code-safe` |
| `--keyboard-only=<bool>`         | Restrict output to ASCII keyboard chars (default `true` for CLI) |
| `--extended-keyboard=<bool>`     | Allow curated non-ASCII keyboard symbols when keyboard-only is active |
| `--keep-emoji`                   | Keep emoji even when keyboard-only is active                     |
| `--non-ascii-policy <mode>`      | `drop`, `fold`, or `transliterate` for non-ASCII handling in keyboard-only mode |
| `--preserve-joiners=<bool>`      | Preserve ZWJ/ZWNJ when hidden-character removal is enabled        |
| `--unicode-normalization <mode>` | One of `none`, `nfd`, `nfc`, `nfkd`, `nfkc`                      |
| `--line-endings <style>`         | `lf`, `crlf`, `cr`, or `auto` (preserve input)                   |
| `--stats`                        | Human-readable statistics to stderr                              |
| `--stats-json`                   | JSON summary to stderr                                           |
| `--exit-code`                    | Exit with status `1` if changes were made                        |

Additional boolean overrides accepted by both tools:

- `--remove-hidden`
- `--remove-trailing-whitespace`
- `--normalize-spaces`
- `--normalize-dashes`
- `--normalize-quotes`
- `--normalize-other`
- `--remove-control-chars`
- `--collapse-whitespace`

Each also accepts explicit values (`true/false`, `1/0`, `yes/no`, `on/off`).

### Presets

Available preset names:

- `minimal`
- `balanced`
- `humanize`
- `aggressive`
- `code-safe`

`code-safe` is intended for docs/source-like text where non-ASCII glyphs and
literal punctuation should be preserved (for example Unicode box-drawing diagrams).

Preset precedence:

- Config is loaded first.
- `--preset` replaces the baseline options.
- Explicit option flags (for example `--keyboard-only false`) apply last.

For bulk cleanup of Markdown/code/docs files:

- preferred: `--preset code-safe`
- fallback: `--keyboard-only false`

### Processing Modes

- `--stream`: process the input line-by-line (lower memory).
- `--inplace`: rewrite the input file atomically (uses a temp file).
- `--stream` and `--inplace` are mutually exclusive.

### Configuration

| Flag              | Description                                |
| ----------------- | ------------------------------------------ |
| `--save-config`   | Persist current options to the config file |
| `--print-config`  | Print resolved configuration in TOML       |
| `--reset-config`  | Remove stored configuration before running |
| `--config <path>` | Use a specific config file                 |

Configuration files are stored under the platform config directory:

- Linux: `~/.config/rehuman/config.toml`
- macOS: `~/Library/Application Support/rehuman/config.toml`
- Windows: `%APPDATA%\rehuman\config.toml`

Example `config.toml`:

```toml
version = 1

[options]
keyboard_only = true
extended_keyboard = false
emoji_policy = "drop"
non_ascii_policy = "transliterate"
preserve_joiners = false
normalize_spaces = true
normalize_quotes = true
unicode_normalization = "nfkc"
```

> [!TIP]
> A ready-to-copy template lives in [`config.example.toml`](../config.example.toml) at the project root.

> [!IMPORTANT]
> Unknown config keys are rejected. Typos are errors, not ignored.

### Option Dependency Notes

- `--keep-emoji` / `--emoji-policy` require keyboard-only mode (`--keyboard-only true`).
- `--non-ascii-policy` requires keyboard-only mode (`--keyboard-only true`).
- `--extended-keyboard` requires keyboard-only mode (`--keyboard-only true`).
- `--print-config` is a standalone mode and conflicts with processing/output flags.

### File Size Limit

`rehuman` reads entire inputs into memory by default and rejects files over **5 MiB**. Use `--stream` for larger files.

## ishuman

Determines if text would change when cleaned. Exits with status `0` when no changes are needed and `1` when the input would be modified. By default no output is printed; add `--stats` or `--json` to learn what would change.

```bash
# Basic check (inspect exit status)
ishuman notes.txt
echo $?  # 0 when clean, 1 when changes are required

# Stats and JSON output
ishuman --stats notes.txt
ishuman --json notes.txt

# Use exit codes for scripting
if ishuman notes.txt; then
    echo "Text is clean"
else
    echo "Text needs cleaning"
fi
```

`ishuman` accepts the same configuration flags as `rehuman` (normalization modes, keyboard-only, etc.).
