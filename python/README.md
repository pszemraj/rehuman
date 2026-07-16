# rehuman (Python)

Unicode-safe text cleaning & normalization for Python, backed by the native
Rust `rehuman` core (PyO3 + maturin). Strip invisible characters, normalize
typography, and enforce ASCII/keyboard-safe output for text sourced from web
scraping, user input, or LLMs.

## Install

The package is published on PyPI as [`rehuman`](https://pypi.org/project/rehuman/):

```bash
pip install rehuman
```

- Requires Python 3.9+ (abi3 wheels: one wheel per platform covers every supported CPython).
- Prebuilt wheels: Linux (x86_64, aarch64), macOS (arm64), Windows (x86_64).
- Other platforms install from the sdist, which compiles the native module and needs a [Rust toolchain](https://rustup.rs/). Wheel coverage details live in [Release Automation](docs/release.md#artifact-matrix).
- Published wheels are built with the default Rust features: `rehuman.HAS_STATS` is `True` and `rehuman.HAS_SECURITY` is `False`.

## Quickstart

`clean()` and `humanize()` both return `str` but target different outputs:
`clean` produces ASCII/keyboard-safe text, `humanize` keeps Unicode and tidies
typography and whitespace ([exact policy differences](docs/api.md#clean-vs-humanize)).

```python
import rehuman

text = "A   B 👍 Café"
cleaned = rehuman.clean(text)       # "A   B Cafe"
humanized = rehuman.humanize(text)  # "A B 👍 Café"

assert cleaned == "A   B Cafe"
assert humanized == "A B 👍 Café"

# Use Cleaner for change counts and stats
cleaner = rehuman.Cleaner()
result = cleaner.clean("Hi\u200bthere \U0001f44d")
print(result.text)         # "Hithere"
print(result.changes_made) # e.g. 3
print(result.stats)        # dict with per-operation counters
```

Option-level control (`non_ascii_policy`, presets, line endings, ...) goes
through [`Options`](docs/api.md#options) and [`Cleaner`](docs/api.md#cleaner).

## Documentation

- [Python API Reference](docs/api.md): functions, classes, presets, constants, and errors.
- [Rust core semantics](../docs/api.md): what each option and preset actually does; the bindings map onto these one-to-one.
- [Release Automation](docs/release.md): wheel/sdist build and PyPI publish flows.

## Development

Build the extension into a local venv and run the test suite:

```bash
cd python
python -m venv .venv
source .venv/bin/activate
pip install -U pip maturin pytest
maturin develop
pytest -q
```
