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

```python
import rehuman

# Top-level helpers return cleaned text only.
# clean(): keyboard-safe default output
# humanize(): normalized, human-readable Unicode output
text = "A   B 👍 Café"
assert rehuman.clean(text) == "A   B Caf"
assert rehuman.humanize(text) == "A B 👍 Café"

# Use Cleaner for change counts and stats
cleaner = rehuman.Cleaner()
result = cleaner.clean("Hi\u200bthere \U0001f44d")
print(result.text)         # "Hithere"
print(result.changes_made) # e.g. 3
print(result.stats)        # dict with per-operation counters
```

## Tests

```bash
cd python
source .venv/bin/activate
pytest -q
```
