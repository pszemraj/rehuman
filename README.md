# rehuman

Unicode-safe text cleaning & normalization for Rust.

Strip invisible characters, normalize typography, and enforce consistent formatting-ideal for text sourced from web scraping, user input, or LLMs.

## Why rehuman?

Untrusted text often contains:

- Zero-width spaces and control characters that break parsers
- Mixed quote styles that defeat string matching
- Non-breaking spaces that masquerade as regular spaces
- Inconsistent Unicode normalization that produces duplicate keys

**rehuman fixes this** in a single pass with predictable, measurable output.

## Installation

```toml
[dependencies]
rehuman = "0.1.0" # replace with the latest published version
```

## Quick Start

```rust
use rehuman::{clean, humanize};

// Basic preset: remove hidden chars, normalize spaces
let cleaned = clean("Hi\u{200B}there"); // -> "Hi there"

// Humanize preset: adds typographic fixes
let humanized = humanize("“Quote”—and…more"); // -> "\"Quote\"-and...more"
```

## Features

- **Invisible character removal**: ZWSP, BOM, bidi isolates, control characters
- **Space normalization**: NBSP, figure space, ideographic space → ASCII space
- **Typography fixes**: curly quotes → ASCII, em/en dash → hyphen, ellipsis → three dots
- **Unicode normalization**: NFC/NFD/NFKC/NFKD (`unorm` feature, enabled by default)
- **Whitespace controls**: optional collapsing, trimming, and line-ending normalization
- **Keyboard-only enforcement**: ASCII output with configurable emoji policy
- **Detailed stats**: every cleaning run reports what changed
- **CLI tooling**: `rehuman` (cleaner) and `ishuman` (detector) with streaming & in-place modes

## Documentation

- [API Reference](docs/api.md) - all functions, options, and statistics
- [CLI Guide](docs/cli.md) - usage of `rehuman` and `ishuman`
- [Examples](docs/examples.md) - recipes for common workloads
- [Development Notes](docs/development.md) - roadmap & implementation details

## License

MIT
