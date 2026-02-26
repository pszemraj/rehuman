# Python Docs

This directory is the source of truth for Python package documentation (`import rehuman`).

## Docs Map

- [API Reference](api.md): functions, classes, presets, constants, and errors.

## Quick Start

```python
import rehuman

print(rehuman.clean("“Hello”—world…"))  # '"Hello"-world...'

cleaner = rehuman.Cleaner(rehuman.Options.code_safe_preset())
result = cleaner.clean('let input = "“Hello — world…”";')
print(result.text)
print(result.changes_made)
print(result.stats)
```

## Local Build & Test

```bash
cd python
python -m venv .venv
source .venv/bin/activate
pip install -U pip maturin pytest
maturin develop
pytest -q
```
