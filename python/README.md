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

## Advanced usage: cleaning a text corpus

A concrete case study: normalizing model-generated text in a training corpus
(SFT conversations, synthetic data, scraped documents). Such text tends to
carry invisible codepoints (zero-width spaces, soft hyphens), typographic
quotes/dashes/ellipses, and exotic whitespace. Normalizing it removes
stylistic tells and collapses spurious token variants without touching
content.

Two decisions matter up front:

- **Do not use default `clean()` on corpora you care about.** It targets
  ASCII/keyboard-safe output and deletes what it cannot transliterate, so
  non-Latin scripts vanish (`"日本語テキスト hello"` becomes `"hello"`).
- **Derive from the `code_safe` preset instead.** It keeps every script,
  emoji, and diagram glyph, strips hidden/control characters, and normalizes
  typographic quotes and dashes. Use `replace` to adjust it:

```python
import rehuman

OPTIONS = rehuman.Options.code_safe_preset().replace(
    normalize_other=True,        # also fold ellipsis -> "..." etc.
    unicode_normalization="nfc", # canonical form, no visual change
)

def clean_batch(batch):
    cleaner = rehuman.Cleaner(OPTIONS)  # construction is ~1 microsecond
    results = [cleaner.clean(t) for t in batch["text"]]
    return {
        "text": [r.text for r in results],
        "rehuman_changes": [r.changes_made for r in results],
    }
```

This shape drops straight into map-style pipelines, for example Hugging Face
datasets: `ds.map(clean_batch, batched=True, num_proc=8)`. `Options` and
`Cleaner` pickle, so worker processes and cache fingerprinting work; `clean`
releases the GIL, so thread pools scale too. Throughput is roughly a
thousand few-KB documents per second per core, linear in processes.

Keeping `changes_made` as a column is deliberate: sort by it descending and
eyeball the most-rewritten samples before training on the output.

One caveat: transforms apply to the whole string with no markdown or code
awareness. A fenced code block whose literals contain typographic characters
gets rewritten like any other text (`text.replace("“", '"')` breaks). If your
corpus is code-heavy, split out fenced blocks and clean only the prose.

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
