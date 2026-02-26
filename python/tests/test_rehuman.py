import pytest

import rehuman


def test_clean_returns_text_only():
    value = rehuman.clean("plain ascii")
    assert isinstance(value, str)
    assert value == "plain ascii"


def test_humanize_returns_text_only():
    value = rehuman.humanize("\u201cHello\u201d\u2014world\u2026")
    assert value == '"Hello"-world...'


def test_clean_hidden_characters():
    assert rehuman.clean("Hello\u200bthere") == "Hellothere"


def test_clean_curly_quotes():
    assert rehuman.clean("\u201cQuote\u201d") == '"Quote"'


def test_clean_dash_and_ellipsis_and_nbsp():
    assert rehuman.clean("a\u2014b\u2026\u00a0z") == "a-b... z"


def test_clean_drops_emoji_by_default():
    assert rehuman.clean("hello \U0001f44d") == "hello"


def test_cleaner_default_result_shape():
    cleaner = rehuman.Cleaner()
    result = cleaner.clean("Hello\u200bthere")
    assert result.text == "Hellothere"
    assert result.changes_made >= 1
    assert isinstance(result.stats, dict)
    assert bool(result) is True


def test_cleaner_result_no_change():
    cleaner = rehuman.Cleaner()
    result = cleaner.clean("plain ascii")
    assert result.text == "plain ascii"
    assert result.changes_made == 0
    assert bool(result) is False


def test_cleaner_with_custom_options_keep_emoji():
    options = rehuman.Options(keyboard_only=True, keep_emoji=True)
    cleaner = rehuman.Cleaner(options)
    result = cleaner.clean("hello \U0001f44d world")
    assert result.text == "hello \U0001f44d world"


def test_stats_contains_expected_keys():
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
        "emojis_dropped",
    }
    assert keys.issubset(stats.keys())
    if rehuman.HAS_SECURITY:
        assert "bidi_controls_removed" in stats


def test_invalid_normalization_raises_value_error():
    with pytest.raises(ValueError, match="invalid normalization mode"):
        rehuman.Options(unicode_normalization="bogus")


def test_invalid_line_endings_raises_value_error():
    with pytest.raises(ValueError, match="invalid line ending style"):
        rehuman.Options(line_endings="bogus")


def test_line_endings_lf():
    options = rehuman.Options(line_endings="lf")
    result = rehuman.Cleaner(options).clean("a\r\nb\rc\u0085")
    assert result.text == "a\nb\nc\n"


def test_unicode_normalization_nfkc():
    options = rehuman.Options(keyboard_only=False, unicode_normalization="nfkc")
    result = rehuman.Cleaner(options).clean("e\u0301")
    assert result.text == "\u00e9"


def test_presets_minimal_balanced_humanize_aggressive():
    minimal = rehuman.Cleaner(rehuman.Options.minimal_preset())
    balanced = rehuman.Cleaner(rehuman.Options.balanced_preset())
    humanize = rehuman.Cleaner(rehuman.Options.humanize_preset())
    aggressive = rehuman.Cleaner(rehuman.Options.aggressive_preset())

    sample = "\u201ctest\u201d  \U0001f44d\r\n"
    assert "\u201c" in minimal.clean(sample).text
    assert balanced.clean(sample).text.startswith('"test"')
    assert "  " not in humanize.clean("a   b").text
    assert aggressive.clean("Caf\u00e9").text == "Caf"


def test_code_safe_preset_preserves_source_like_text():
    code_safe = rehuman.Cleaner(rehuman.Options.code_safe_preset())
    source_like = 'let input = "“Hello — world…”\\u{00A0}😀";'
    result = code_safe.clean(source_like)
    assert result.text == source_like
    assert result.changes_made == 0


def test_code_safe_preset_removes_hidden_and_control_chars():
    code_safe = rehuman.Cleaner(rehuman.Options.code_safe_preset())
    result = code_safe.clean("a\u200bb\x01")
    assert result.text == "ab"
    assert result.stats["hidden_chars_removed"] >= 1
    assert result.stats["control_chars_removed"] >= 1


def test_security_option_is_conditional():
    if rehuman.HAS_SECURITY:
        options = rehuman.Options(strip_bidi_controls=True)  # type: ignore[call-arg]
        result = rehuman.Cleaner(options).clean("\u202eab\u202cc")
        assert result.text == "abc"
        assert result.stats.get("bidi_controls_removed", 0) >= 2
    else:
        with pytest.raises(TypeError):
            rehuman.Options(strip_bidi_controls=True)  # type: ignore[call-arg]


def test_module_constants_present():
    assert isinstance(rehuman.HAS_STATS, bool)
    assert isinstance(rehuman.HAS_SECURITY, bool)
    assert isinstance(rehuman.__version__, str)
