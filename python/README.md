# rehuman (Python)

Python bindings for `rehuman` built with PyO3 + maturin.

## Development Install

```bash
cd python
python -m venv .venv
source .venv/bin/activate
pip install -U pip maturin pytest
maturin develop
```

## Quickstart

```python
import rehuman

# Top-level helpers return cleaned text only
assert rehuman.clean("\u201cHello\u201d") == '"Hello"'
assert rehuman.humanize("a   b") == "a b"

# Use Cleaner for change counts and stats
cleaner = rehuman.Cleaner()
result = cleaner.clean("Hi\u200bthere \U0001f44d")
print(result.text)         # "Hithere"
print(result.changes_made) # e.g. 3
print(result.stats)        # dict with per-operation counters
```

## API

- `clean(text: str) -> str`
- `humanize(text: str) -> str`
- `Options(...)`
- `Cleaner(options: Options | None = None)`
- `CleaningResult`
- `Options.code_safe_preset()` for docs/source-safe normalization defaults

Module constants:

- `HAS_STATS`: `bool`
- `HAS_SECURITY`: `bool`
- `__version__`: `str`

## Features

- Default Python build enables Rust features: `unorm`, `stats`.
- Optional `security` feature is supported; when enabled:
  - `Options(..., strip_bidi_controls=...)` is available.
  - `CleaningResult.stats` may include `bidi_controls_removed`.

## Tests

```bash
cd python
source .venv/bin/activate
pytest -q
```
