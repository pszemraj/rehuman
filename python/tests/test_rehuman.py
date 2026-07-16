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


def test_stats_keys_preserve_declaration_order() -> None:
    """Stats dict keys keep CleaningStats declaration order, not alphabetical."""
    cleaner = rehuman.Cleaner()
    result = cleaner.clean("“Hi” — ok…")
    assert list(result.stats.keys())[:3] == [
        "hidden_chars_removed",
        "trailing_whitespace_removed",
        "spaces_normalized",
    ]


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


@pytest.mark.parametrize("line_endings", [None, "auto", "none"])
def test_line_ending_preservation_aliases(line_endings: str | None) -> None:
    """Both names for preserving line endings remain accepted."""
    options = rehuman.Options(line_endings=line_endings)
    result = rehuman.Cleaner(options).clean("a\r\nb\rc\n")
    assert result.text == "a\r\nb\rc\n"


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


def test_extended_keyboard_allowlist_is_configurable() -> None:
    """Extended keyboard mode keeps curated non-ASCII symbols."""
    default = rehuman.Cleaner(
        rehuman.Options(keyboard_only=True, non_ascii_policy="drop")
    ).clean("\u20ac and \u2122")
    extended = rehuman.Cleaner(
        rehuman.Options(
            keyboard_only=True, non_ascii_policy="drop", extended_keyboard=True
        )
    ).clean("\u20ac and \u2122")

    assert default.text == "and"
    assert extended.text == "\u20ac and"


def test_preserve_joiners_toggle() -> None:
    """Joiners can be preserved when hidden-character removal is enabled."""
    text = "\u0645\u06CC\u200C\u062E\u0648\u0627\u0647\u0645"  # Persian with ZWNJ
    default = rehuman.Cleaner(
        rehuman.Options(keyboard_only=False, remove_hidden=True)
    ).clean(text)
    preserved = rehuman.Cleaner(
        rehuman.Options(
            keyboard_only=False, remove_hidden=True, preserve_joiners=True
        )
    ).clean(text)

    assert "\u200c" not in default.text
    assert "\u200c" in preserved.text


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


def test_code_safe_preset_normalizes_quotes_and_dashes() -> None:
    """Code-safe preset rewrites typographic quotes/dashes but keeps glyphs."""
    code_safe = rehuman.Cleaner(rehuman.Options.code_safe_preset())
    # Keep a literal Rust escape token (`\\u{00A0}`), not the NBSP codepoint.
    source_like = 'let input = "“Hello — world…”\\u{00A0}😀";'
    result = code_safe.clean(source_like)
    assert result.text == 'let input = ""Hello - world…"\\u{00A0}😀";'
    assert result.changes_made == 3

    # Diagram glyphs, ellipsis, and emoji still pass through untouched.
    preserved = "├── src/ … 😀"
    assert code_safe.clean(preserved).text == preserved


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
    assert "extended_keyboard=false" in options_repr
    assert "preserve_joiners=false" in options_repr
    assert "line_endings='lf'" in options_repr
    assert "unicode_normalization='nfkc'" in options_repr

    cleaner = rehuman.Cleaner(options)
    assert repr(cleaner) == "Cleaner(keyboard_only=true, emoji_policy='keep')"
    left = cleaner.clean("e\u0301 👍")
    right = cleaner.clean("e\u0301 👍")
    assert left == right
    assert str(left) == left.text
    assert repr(left).startswith("CleaningResult(changes_made=")


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


def test_options_getters_mirror_constructor() -> None:
    """Field getters report the values passed to the constructor."""
    options = rehuman.Options(
        keyboard_only=False,
        keep_emoji=True,
        non_ascii_policy="fold",
        line_endings="lf",
        unicode_normalization="nfc",
    )
    assert options.keyboard_only is False
    assert options.keep_emoji is True
    assert options.non_ascii_policy == "fold"
    assert options.line_endings == "lf"
    assert options.unicode_normalization == "nfc"
    # Untouched fields keep constructor defaults.
    assert options.remove_hidden is True
    assert options.collapse_whitespace is False
    assert rehuman.Options().line_endings is None


def test_options_replace_derives_from_preset() -> None:
    """replace() copies options with named overrides, leaving the base intact."""
    base = rehuman.Options.code_safe_preset()
    derived = base.replace(normalize_other=True, unicode_normalization="nfc")
    assert derived.normalize_other is True
    assert derived.unicode_normalization == "nfc"
    # Preset fields not named in replace() carry over.
    assert derived.keyboard_only is False
    assert derived.preserve_joiners is True
    # The original preset object is unchanged, and no-arg replace is identity.
    assert base.normalize_other is False
    assert base.replace() == base
    assert derived != base

    with pytest.raises(TypeError):
        base.replace(not_an_option=True)  # type: ignore[call-arg]
    if not rehuman.HAS_SECURITY:
        with pytest.raises(TypeError):
            base.replace(strip_bidi_controls=True)  # type: ignore[call-arg]


def test_options_and_cleaner_pickle_roundtrip() -> None:
    """Options and Cleaner survive pickling (datasets.map / multiprocessing)."""
    import pickle

    options = rehuman.Options.code_safe_preset().replace(normalize_other=True)
    restored = pickle.loads(pickle.dumps(options))
    assert restored == options

    cleaner = rehuman.Cleaner(options)
    restored_cleaner = pickle.loads(pickle.dumps(cleaner))
    sample = "“quoted” — text…"
    assert restored_cleaner.clean(sample).text == cleaner.clean(sample).text
