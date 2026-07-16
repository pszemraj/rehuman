# Python API Reference

Behavior reference for the `rehuman` Python package
([PyPI](https://pypi.org/project/rehuman/)). Installation, wheel coverage, and
quickstart live in the [package README](../README.md); the underlying option
semantics are defined by the Rust core in [docs/api.md](../../docs/api.md).

## Module

```python
import rehuman
```

Top-level metadata/constants:

- `rehuman.__version__: str`
- `rehuman.HAS_STATS: bool`: whether the native module was built with per-operation counters (`True` on published wheels; `False` means `stats` values stay `0` while `changes_made` remains accurate).
- `rehuman.HAS_SECURITY: bool`: whether bidi-control stripping is available (`False` on published wheels; `strip_bidi_controls` and the `bidi_controls_removed` stat require a custom build with the `security` feature).

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

- Default behavior: keyboard-safe output (`keyboard_only=True`).
- For the exact policy differences versus `humanize`, see [clean vs humanize](#clean-vs-humanize).
- For option-level control (`non_ascii_policy`, `extended_keyboard`, `preserve_joiners`), use [`Options`](#options) with [`Cleaner`](#cleaner).

```python
import rehuman

assert rehuman.clean("Hello\u200bthere") == "Hellothere"
assert rehuman.clean("Thanks 👍") == "Thanks"
```

### `humanize(text: str) -> str`

Runs the Rust `humanize` preset and returns cleaned text only.

- Default behavior: human-readable Unicode output (`keyboard_only=False`).
- Collapses repeated whitespace and applies NFKC normalization.
- For the exact policy differences versus `clean`, see [clean vs humanize](#clean-vs-humanize).

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
- `non_ascii_policy: str = "transliterate"` (`"drop"` / `"fold"` / `"transliterate"`; resolution order, symbol/Greek mappings, and what still drops are specified in [Keyboard-Only Behavior](../../docs/api.md#keyboard-only-behavior))
- `preserve_joiners: bool = False`
- `remove_control_chars: bool = True`
- `collapse_whitespace: bool = False`
- `line_endings: str | None = None` (`None` / `"auto"` / `"none"` / `"lf"` / `"crlf"` / `"cr"`)
- `unicode_normalization: str = "none"` (`"none"` / `"nfd"` / `"nfc"` / `"nfkd"` / `"nfkc"`)
- `strip_bidi_controls: bool = False` (only when `rehuman.HAS_SECURITY` is `True`; the shipped type stubs match the published wheels and do not declare this keyword)

Presets (each returns an `Options` matching the same-named Rust preset; field
values are defined in the [Rust builder docs](../../docs/api.md#builder-api)):

- `Options.minimal_preset()`
- `Options.balanced_preset()`
- `Options.humanize_preset()`
- `Options.aggressive_preset()`
- `Options.code_safe_preset()`: for source/docs text; keeps non-ASCII glyphs, ellipses, emoji, and joiners (no keyboard-only dropping) while still normalizing typographic quotes and dashes to ASCII.

Deriving and inspecting options:

- `replace(**kwargs) -> Options`: returns a copy with the named fields
  replaced; accepts the same keyword arguments as the constructor. This is
  how you derive from a preset without restating it:

  ```python
  options = rehuman.Options.code_safe_preset().replace(normalize_other=True)
  ```

- Every constructor keyword is also a read-only attribute
  (`options.keyboard_only`, `options.non_ascii_policy`, ...) reporting the
  resolved value. `strip_bidi_controls` is readable only on security builds.
- `Options` compares by value (`==`) and pickles; unknown keywords in
  `replace()` raise `TypeError`, matching the constructor.

`repr(options)` uses the same lowercase Python-facing names accepted by the
constructor (for example `emoji_policy='keep'`).

### `Cleaner`

Reusable cleaner instance.

- `Cleaner(options: Options | None = None)`
- `clean(text: str) -> CleaningResult`

`repr(cleaner)` reports `keyboard_only` and the lowercase emoji policy.

Use `Cleaner` when you need counters/stats, not just cleaned text.

`Cleaner` pickles (it reconstructs as `Cleaner(options)`), so instances work
with `multiprocessing` and libraries that fingerprint their inputs by
serializing them. `clean` releases the GIL while the Rust pipeline runs, so
thread pools scale across cores.

### `CleaningResult`

Returned by `Cleaner.clean`.

- `text: str`
- `changes_made: int`
- `stats: dict[str, int]`

`CleaningResult` compares by value, converts to its cleaned text with `str()`,
and is truthy when `changes_made > 0`.

Stats keys, in dict order (matching the Rust `CleaningStats` field order):

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
- Passing `strip_bidi_controls` when `rehuman.HAS_SECURITY` is `False` raises `TypeError`: the constructor on non-security builds does not accept the keyword at all.

## Docstrings & Typing

- Runtime docstrings are available via `help(rehuman)`, `help(rehuman.Options)`, etc.
- Type hints are shipped via `rehuman/__init__.pyi` and `rehuman/py.typed`.
