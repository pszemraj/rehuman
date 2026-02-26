"""Behavioral tests for the public `rehuman` Python bindings."""

import pytest

import rehuman


def test_clean_returns_text_only() -> None:
    """`clean` returns plain text, not a result object."""
    value = rehuman.clean("plain ascii")
    assert isinstance(value, str)
    assert value == "plain ascii"


def test_humanize_returns_text_only() -> None:
    """`humanize` returns normalized text only."""
    value = rehuman.humanize("\u201cHello\u201d\u2014world\u2026")
    assert value == '"Hello"-world...'


def test_clean_hidden_characters() -> None:
    """Hidden characters are removed by default cleaning."""
    assert rehuman.clean("Hello\u200bthere") == "Hellothere"


def test_clean_curly_quotes() -> None:
    """Curly quotes normalize to ASCII quotes."""
    assert rehuman.clean("\u201cQuote\u201d") == '"Quote"'


def test_clean_dash_and_ellipsis_and_nbsp() -> None:
    """Dash, ellipsis, and NBSP normalize to ASCII-friendly forms."""
    assert rehuman.clean("a\u2014b\u2026\u00a0z") == "a-b... z"


def test_clean_drops_emoji_by_default() -> None:
    """Default policy drops emoji in keyboard-only mode."""
    assert rehuman.clean("hello \U0001f44d") == "hello"


def test_cleaner_default_result_shape() -> None:
    """`Cleaner.clean` returns a result with text, changes, and stats."""
    cleaner = rehuman.Cleaner()
    result = cleaner.clean("Hello\u200bthere")
    assert result.text == "Hellothere"
    assert result.changes_made >= 1
    assert isinstance(result.stats, dict)
    assert bool(result) is True


def test_cleaner_result_no_change() -> None:
    """Result change count is zero when input needs no normalization."""
    cleaner = rehuman.Cleaner()
    result = cleaner.clean("plain ascii")
    assert result.text == "plain ascii"
    assert result.changes_made == 0
    assert bool(result) is False


def test_cleaner_with_custom_options_keep_emoji() -> None:
    """Explicit keep-emoji option preserves emoji output."""
    options = rehuman.Options(keyboard_only=True, keep_emoji=True)
    cleaner = rehuman.Cleaner(options)
    result = cleaner.clean("hello \U0001f44d world")
    assert result.text == "hello \U0001f44d world"


def test_stats_contains_expected_keys() -> None:
    """Stats dict exposes the expected stable keys."""
    cleaner = rehuman.Cleaner()
    result = cleaner.clean("\u201cHi\u201d\u00a0\u2014 ok\u2026")
    stats = result.stats
    keys = {
        "hidden_chars_removed",
        "trailing_whitespace_removed",
        "spaces_normalized",
        "dashes_normalized",
        "quotes_normalized",
        "other_normalized",
        "control_chars_removed",
        "line_endings_normalized",
        "non_keyboard_removed",
        "non_keyboard_transliterated",
        "emojis_dropped",
    }
    assert keys.issubset(stats.keys())
    if rehuman.HAS_SECURITY:
        assert "bidi_controls_removed" in stats


def test_invalid_normalization_raises_value_error() -> None:
    """Invalid normalization mode is rejected with `ValueError`."""
    with pytest.raises(ValueError, match="invalid normalization mode"):
        rehuman.Options(unicode_normalization="bogus")


def test_invalid_line_endings_raises_value_error() -> None:
    """Invalid line ending mode is rejected with `ValueError`."""
    with pytest.raises(ValueError, match="invalid line ending style"):
        rehuman.Options(line_endings="bogus")


def test_invalid_non_ascii_policy_raises_value_error() -> None:
    """Invalid non-ASCII policy is rejected with `ValueError`."""
    with pytest.raises(ValueError, match="invalid non-ASCII policy"):
        rehuman.Options(non_ascii_policy="bogus")


def test_line_endings_lf() -> None:
    """Line-ending normalization can force LF output."""
    options = rehuman.Options(line_endings="lf")
    result = rehuman.Cleaner(options).clean("a\r\nb\rc\u0085")
    assert result.text == "a\nb\nc\n"


def test_unorm_is_available_by_default() -> None:
    """Default bindings build includes `unorm` and composes decomposed chars."""
    options = rehuman.Options(keyboard_only=False, unicode_normalization="nfkc")
    result = rehuman.Cleaner(options).clean("e\u0301")
    assert result.text == "\u00e9"


def test_keyboard_only_transliteration_policy_modes() -> None:
    """Keyboard-only mode supports drop, fold, and transliterate policies."""
    drop = rehuman.Cleaner(
        rehuman.Options(keyboard_only=True, non_ascii_policy="drop")
    ).clean("Stra\u00dfe \u00bd \u2122")
    fold = rehuman.Cleaner(
        rehuman.Options(keyboard_only=True, non_ascii_policy="fold")
    ).clean("Stra\u00dfe \u00bd \u2122")
    transliterate = rehuman.Cleaner(
        rehuman.Options(keyboard_only=True, non_ascii_policy="transliterate")
    ).clean("Stra\u00dfe \u00bd \u2122")

    assert drop.text == "Strae"
    assert fold.text == "Strae 1/2 TM"
    assert transliterate.text == "Strasse 1/2 TM"
    assert transliterate.stats["non_keyboard_transliterated"] >= 3


def test_presets_minimal_balanced_humanize_aggressive() -> None:
    """Built-in presets map to distinct cleaning behaviors."""
    minimal = rehuman.Cleaner(rehuman.Options.minimal_preset())
    balanced = rehuman.Cleaner(rehuman.Options.balanced_preset())
    humanize = rehuman.Cleaner(rehuman.Options.humanize_preset())
    aggressive = rehuman.Cleaner(rehuman.Options.aggressive_preset())

    sample = "\u201ctest\u201d  \U0001f44d\r\n"
    assert "\u201c" in minimal.clean(sample).text
    assert balanced.clean(sample).text.startswith('"test"')
    assert "  " not in humanize.clean("a   b").text
    assert aggressive.clean("Caf\u00e9").text == "Cafe"


def test_code_safe_preset_preserves_source_like_text() -> None:
    """Code-safe preset avoids semantic text rewrites."""
    code_safe = rehuman.Cleaner(rehuman.Options.code_safe_preset())
    # Keep a literal Rust escape token (`\\u{00A0}`), not the NBSP codepoint.
    source_like = 'let input = "“Hello — world…”\\u{00A0}😀";'
    result = code_safe.clean(source_like)
    assert result.text == source_like
    assert result.changes_made == 0


def test_options_repr_and_result_equality_are_value_based() -> None:
    """Options repr and CleaningResult equality use Python-facing value semantics."""
    options = rehuman.Options(
        keep_emoji=True,
        keyboard_only=True,
        unicode_normalization="nfkc",
        line_endings="lf",
    )
    options_repr = repr(options)
    assert "emoji_policy='keep'" in options_repr
    assert "non_ascii_policy='transliterate'" in options_repr
    assert "line_endings='lf'" in options_repr
    assert "unicode_normalization='nfkc'" in options_repr

    cleaner = rehuman.Cleaner(options)
    left = cleaner.clean("e\u0301 👍")
    right = cleaner.clean("e\u0301 👍")
    assert left == right


def test_code_safe_preset_removes_hidden_and_control_chars() -> None:
    """Code-safe preset still strips hidden and control characters."""
    code_safe = rehuman.Cleaner(rehuman.Options.code_safe_preset())
    result = code_safe.clean("a\u200bb\x01")
    assert result.text == "ab"
    assert result.stats["hidden_chars_removed"] >= 1
    assert result.stats["control_chars_removed"] >= 1


def test_security_option_is_conditional() -> None:
    """Security-only option behavior matches compiled feature set."""
    if rehuman.HAS_SECURITY:
        options = rehuman.Options(strip_bidi_controls=True)  # type: ignore[call-arg]
        result = rehuman.Cleaner(options).clean("\u202eab\u202cc")
        assert result.text == "abc"
        assert result.stats.get("bidi_controls_removed", 0) >= 2
    else:
        with pytest.raises(TypeError):
            rehuman.Options(strip_bidi_controls=True)  # type: ignore[call-arg]


def test_module_constants_present() -> None:
    """Feature/version constants are exported with stable types."""
    assert isinstance(rehuman.HAS_STATS, bool)
    assert isinstance(rehuman.HAS_SECURITY, bool)
    assert isinstance(rehuman.__version__, str)


def test_public_docstrings_present() -> None:
    """Primary API symbols expose non-empty docstrings."""
    assert rehuman.__doc__
    assert rehuman.clean.__doc__
    assert rehuman.humanize.__doc__
    assert rehuman.Options.__doc__
    assert rehuman.Cleaner.__doc__
    assert rehuman.CleaningResult.__doc__
