# rehuman

Unicode-safe text cleaning & normalization for Rust.

Strip invisible characters, normalize typography, and enforce consistent formatting for text sourced from web scraping, user input, or [LLMs](https://archive.fo/PrRYl).

> This crate is a Rust rewrite and expansion of [humanize-ai-lib](https://github.com/Nordth/humanize-ai-lib) by [Nordth](https://github.com/Nordth).

## Install

Add the Rust library crate:

```toml
[dependencies]
rehuman = "0.1.1" # replace with the latest published version
```

Install CLI binaries (`rehuman`, `ishuman`):

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

```rust
use rehuman::{clean, humanize};

let cleaned = clean("Hello\u{200B}there"); // -> "Hellothere"
let humanized = humanize("“Quote”—and…more"); // -> "\"Quote\"-and...more"
```

```rust
use rehuman::clean;

// Default behavior removes emoji
let cleaned = clean("Thanks 👍"); // -> "Thanks"
```

By default, keyboard-only mode drops non-ASCII characters rather than transliterating them.
For detailed semantics and option behavior, use the API reference links below.

## Documentation

Primary docs by concern:

- Rust API semantics (defaults, options, presets, stats, errors):
  [docs/api.md](docs/api.md)
- CLI flags, modes, config, and exit behavior:
  [docs/cli.md](docs/cli.md)
- Usage recipes:
  [docs/examples.md](docs/examples.md)
- Python bindings (`import rehuman`):
  [python/docs/index.md](python/docs/index.md)
- Roadmap and development notes:
  [docs/development.md](docs/development.md)

For CLI help at runtime: `rehuman --help` and `ishuman --help`.

## License

MIT
