# Python API Reference

Behavior reference for the `rehuman` Python package.

## Module

```python
import rehuman
```

Top-level metadata/constants:

- `rehuman.__version__: str`
- `rehuman.HAS_STATS: bool`
- `rehuman.HAS_SECURITY: bool`

## `clean` vs `humanize`

Both helpers return `str`. They differ in policy:

| Helper | Preset Basis | Keyboard-Only | Whitespace Collapse | Unicode Normalization |
| --- | --- | --- | --- | --- |
| `clean(text)` | `CleaningOptions::default()` | `true` | `false` | `none` |
| `humanize(text)` | `CleaningOptions::humanize()` | `false` | `true` | `nfkc` |

Example:

```python
import rehuman

text = "A   B 👍 Café"
assert rehuman.clean(text) == "A   B Cafe"
assert rehuman.humanize(text) == "A B 👍 Café"
```

## Functions

### `clean(text: str) -> str`

Runs the default cleaner and returns cleaned text only.

- Default behavior is keyboard-safe output (`keyboard_only=True`).
- In keyboard-only mode, text is normalized then transliterated to ASCII when feasible (`"Café"` -> `"Cafe"`, `"Straße"` -> `"Strasse"`), then remaining non-ASCII characters are removed.
- Use `non_ascii_policy="drop"|"fold"|"transliterate"` and `extended_keyboard=True` to control this behavior.
- Whitespace is not collapsed unless you configure it via `Options` + `Cleaner`.

```python
import rehuman

assert rehuman.clean("Hello\u200bthere") == "Hellothere"
assert rehuman.clean("Thanks 👍") == "Thanks"
```

### `humanize(text: str) -> str`

Runs the Rust `humanize` preset and returns cleaned text only.

- Intended for normalized, human-readable Unicode output.
- Collapses repeated whitespace.
- Applies NFKC normalization.

```python
import rehuman

assert rehuman.humanize("“Quote”—and…more") == '"Quote"-and...more'
```

## Classes

### `Options`

Configuration object for `Cleaner`.

Constructor keyword arguments:

- `remove_hidden: bool = True`
- `remove_trailing_whitespace: bool = True`
- `normalize_spaces: bool = True`
- `normalize_dashes: bool = True`
- `normalize_quotes: bool = True`
- `normalize_other: bool = True`
- `keyboard_only: bool = True`
- `extended_keyboard: bool = False`
- `keep_emoji: bool = False`
- `non_ascii_policy: str = "transliterate"` (`"drop"` / `"fold"` / `"transliterate"`)
- `preserve_joiners: bool = False`
- `remove_control_chars: bool = True`
- `collapse_whitespace: bool = False`
- `line_endings: str | None = None` (`None` / `"auto"` / `"none"` / `"lf"` / `"crlf"` / `"cr"`)
- `unicode_normalization: str = "none"` (`"none"` / `"nfd"` / `"nfc"` / `"nfkd"` / `"nfkc"`)
- `strip_bidi_controls: bool = False` (only when `rehuman.HAS_SECURITY` is `True`)

Presets:

- `Options.minimal_preset()`
- `Options.balanced_preset()`
- `Options.humanize_preset()`
- `Options.aggressive_preset()`
- `Options.code_safe_preset()`: preserves source/docs text semantics by disabling quote/dash/ellipsis rewrites and turning off keyboard-only dropping.

### `Cleaner`

Reusable cleaner instance.

- `Cleaner(options: Options | None = None)`
- `clean(text: str) -> CleaningResult`

Use `Cleaner` when you need counters/stats, not just cleaned text.

### `CleaningResult`

Returned by `Cleaner.clean`.

- `text: str`
- `changes_made: int`
- `stats: dict[str, int]`

Stats keys:

- `hidden_chars_removed`
- `trailing_whitespace_removed`
- `spaces_normalized`
- `dashes_normalized`
- `quotes_normalized`
- `other_normalized`
- `control_chars_removed`
- `line_endings_normalized`
- `non_keyboard_removed`
- `non_keyboard_transliterated`
- `emojis_dropped`
- `bidi_controls_removed` (when `rehuman.HAS_SECURITY` is `True`)

## Errors

- Invalid option strings (for `line_endings` / `unicode_normalization` / `non_ascii_policy`) raise `ValueError`.
- Cleaner runtime errors from unavailable normalization features are surfaced as `ValueError`.

## Docstrings & Typing

- Runtime docstrings are available via `help(rehuman)`, `help(rehuman.Options)`, etc.
- Type hints are shipped via `rehuman/__init__.pyi` and `rehuman/py.typed`.
