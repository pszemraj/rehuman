# CLI Guide

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

### Output Options

| Flag                             | Description                                                      |
| -------------------------------- | ---------------------------------------------------------------- |
| `--keyboard-only=<bool>`         | Restrict output to ASCII keyboard chars (default `true` for CLI) |
| `--keep-emoji`                   | Keep emoji even when keyboard-only is active                     |
| `--unicode-normalization <mode>` | One of `none`, `nfd`, `nfc`, `nfkd`, `nfkc`                      |
| `--line-endings <style>`         | `lf`, `crlf`, `cr`, or `auto` (preserve input)                   |
| `--stats`                        | Human-readable statistics to stderr                              |
| `--stats-json`                   | JSON summary to stderr                                           |
| `--exit-code`                    | Exit with status `1` if changes were made                        |

### Processing Modes

- `--stream`: process the input line-by-line (lower memory).
- `--inplace`: rewrite the input file atomically (uses a temp file).

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
keyboard_only = true
emoji_policy = "drop"
normalize_spaces = true
normalize_quotes = true
unicode_normalization = "nfkc"
```

### File Size Limit

`rehuman` reads entire inputs into memory by default and rejects files over **5 MiB**. Use `--stream` for larger files.

## ishuman

Determines if text would change when cleaned. Returns `1` when no changes are needed, `0` otherwise.

```bash
# Basic check
ishuman notes.txt

# Stats and JSON output
ishuman --stats notes.txt
ishuman --json notes.txt

# Use exit codes for scripting
if ishuman --exit-code notes.txt; then
    echo "Text is clean"
else
    echo "Text needs cleaning"
fi
```

`ishuman` accepts the same configuration flags as `rehuman` (normalization modes, keyboard-only, etc.).
