# rehuman (Python)

Python bindings for `rehuman` built with PyO3 + maturin.

## Install (Development)

```bash
cd python
python -m venv .venv
source .venv/bin/activate
pip install -U pip maturin pytest
maturin develop
```

## Documentation

See [python/docs/index.md](docs/index.md) for the Python docs map.

## Quickstart

`clean()` and `humanize()` both return `str`, but they target different outputs:

- `clean(text)`: ASCII/keyboard-safe text (drops emoji by default, transliterates when feasible)
- `humanize(text)`: human-readable Unicode text (keeps Unicode, collapses repeated whitespace)

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

For exact behavior differences and presets, see:

- [clean vs humanize](docs/api.md#clean-vs-humanize)
- [Options](docs/api.md#options)

## Tests

```bash
cd python
source .venv/bin/activate
pytest -q
```
