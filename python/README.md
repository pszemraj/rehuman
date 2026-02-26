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

`clean()` and `humanize()` both return `str`, but they use different defaults:

| Helper | Default behavior |
| --- | --- |
| `clean(text)` | Keyboard-safe output (`keyboard_only=True`), transliterates non-ASCII when feasible |
| `humanize(text)` | Human-readable Unicode output (`keyboard_only=False`), collapses whitespace |

```python
import rehuman

text = "A   B 👍 Café"
assert rehuman.clean(text) == "A   B Cafe"
assert rehuman.humanize(text) == "A B 👍 Café"

# Use Cleaner for change counts and stats
cleaner = rehuman.Cleaner()
result = cleaner.clean("Hi\u200bthere \U0001f44d")
print(result.text)         # "Hithere"
print(result.changes_made) # e.g. 3
print(result.stats)        # dict with per-operation counters
```

For custom options (`non_ascii_policy`, `extended_keyboard`, `preserve_joiners`,
presets), see [python/docs/api.md](docs/api.md#options).

## Tests

```bash
cd python
source .venv/bin/activate
pytest -q
```
