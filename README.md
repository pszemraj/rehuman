# rehuman

Unicode-safe text cleaning & normalization for Rust.

Strip invisible characters, normalize typography, and enforce consistent formatting-ideal for text sourced from web scraping, user input, or [LLMs](https://archive.fo/PrRYl).

> This crate is a Rust rewrite and expansion of [humanize-ai-lib](https://github.com/Nordth/humanize-ai-lib) by [Nordth](https://github.com/Nordth).

## Why rehuman?

Untrusted text often contains:

- Zero-width spaces and control characters that break parsers
- Mixed quote styles that defeat string matching
- Non-breaking spaces that masquerade as regular spaces
- Inconsistent Unicode normalization that produces duplicate keys

**rehuman fixes this** in a single pass with predictable, measurable output.

## Installation

**Library crate**: add `rehuman` to your project with `cargo add rehuman` or edit `Cargo.toml`:

```toml
[dependencies]
rehuman = "0.1.0" # replace with the latest published version
```

**CLI binaries**: install the published release (installs both `rehuman` and `ishuman`):

```bash
cargo install rehuman
```

<details>
<summary><b>Click to Expand:</b> Build from Source</summary>

For the latest version(s), clone this repo and run `cargo install --path .`:

```bash
git clone https://github.com/pszemraj/rehuman.git
cd rehuman
cargo install --path .
```

Binaries will be installed to `~/.cargo/bin` by default.[^1]

[^1]: You may need to add `~/.cargo/bin` to your `PATH` if it is not already there; add `export PATH="$HOME/.cargo/bin:$PATH"` to your shell profile (`.bashrc`, `.zshrc`, etc.).

</details>

## Quick Start

> [!WARNING]
> This is an **early release** focused on correctness. Performance optimizations are in progress. Use `--stream` or `StreamCleaner` to stream large files.

### Library

```rust
use rehuman::{clean, humanize};

let cleaned = clean("Hello\u{200B}there"); // -> "Hello there"
let humanized = humanize("“Quote”—and…more"); // -> "\"Quote\"-and...more"
```

> [!IMPORTANT]
> By default `rehuman::clean` removes emoji to guarantee ASCII-only output[^2].

[^2]: This is a deliberate design choice given the propensity of today's LLMs to spam emoji in their outputs.

```rust
use rehuman::clean;

// Default behavior removes emoji
let cleaned = clean("Thanks 👍"); // -> "Thanks "
```

To keep emoji, construct a cleaner with `CleaningOptions::builder().keyboard_only(false)` (or pass `--keep-emoji` on the CLI).

### CLI

`rehuman` reads the input and emits cleaned text to STDOUT-your source file stays untouched unless you pass `--inplace`:

```bash
# Stream-clean to STDOUT and capture stats
rehuman notes.txt --stream --stats > notes.cleaned.txt

# Overwrite the original file in place
rehuman notes.txt --inplace
```

> [!TIP]
> Both CLI tools act as filters, so you can drop them into pipelines

```bash
cat notes.txt | rehuman --stream | tee notes.cleaned.txt
curl https://example.com/raw.txt | rehuman --stream --stats-json >/tmp/clean.txt
```

Use `ishuman` when you only need detection:

```bash
# Exit status 0 when clean, 1 when changes would be made (no stdout by default)
ishuman notes.txt

# Add --stats or --json to explain what would change
ishuman notes.txt --stats
```

Run `rehuman --help` or `ishuman --help` for the full list of flags (_emoji policy, line endings, configs, streaming, etc._).

## Documentation

More details are available in the [`docs/`](docs/) folder:

- [API Reference](docs/api.md) - all functions, options, and statistics
- [CLI Guide](docs/cli.md) - usage of `rehuman` and `ishuman`
- [Examples](docs/examples.md) - recipes for common workloads
- [Development Notes](docs/development.md) - roadmap & implementation details

## Detailed Features

- **Invisible character removal**: ZWSP, BOM, bidi isolates, control characters
- **Space normalization**: NBSP, figure space, ideographic space → ASCII space
- **Typography fixes**: curly quotes → ASCII, em/en dash → hyphen, ellipsis → three dots
- **Unicode normalization**: NFC/NFD/NFKC/NFKD (`unorm` feature, enabled by default)
- **Whitespace controls**: optional collapsing, trimming, and line-ending normalization
- **Keyboard-only enforcement**: ASCII output with configurable emoji policy
- **Detailed stats**: every cleaning run reports what changed
- **CLI tooling**: `rehuman` (cleaner) and `ishuman` (detector) with streaming & in-place modes

## License

MIT
