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

Primary Python docs live in this package directory:

- [python/docs/index.md](docs/index.md)
- [python/docs/api.md](docs/api.md) (canonical Python API semantics)
- [python/docs/release.md](docs/release.md) (release assets + PyPI automation)

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

## Tests

```bash
cd python
source .venv/bin/activate
pytest -q
```
