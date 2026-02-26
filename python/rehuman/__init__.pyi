from __future__ import annotations

"""Typing stubs for the public ``rehuman`` Python API."""

__version__: str
HAS_STATS: bool
HAS_SECURITY: bool

def clean(text: str) -> str:
    """Clean text with default settings and return cleaned text only."""
    ...

def humanize(text: str) -> str:
    """Clean text with the humanize preset and return cleaned text only."""
    ...

class CleaningResult:
    """Result returned by ``Cleaner.clean``."""

    @property
    def text(self) -> str: ...
    @property
    def changes_made(self) -> int: ...
    @property
    def stats(self) -> dict[str, int]: ...

class Options:
    """Configuration object for ``Cleaner``.

    ``strip_bidi_controls`` is conditionally available at runtime only when
    ``rehuman.HAS_SECURITY`` is ``True``.
    """

    def __init__(
        self,
        *,
        remove_hidden: bool = ...,
        remove_trailing_whitespace: bool = ...,
        normalize_spaces: bool = ...,
        normalize_dashes: bool = ...,
        normalize_quotes: bool = ...,
        normalize_other: bool = ...,
        keyboard_only: bool = ...,
        extended_keyboard: bool = ...,
        keep_emoji: bool = ...,
        non_ascii_policy: str = ...,
        preserve_joiners: bool = ...,
        remove_control_chars: bool = ...,
        collapse_whitespace: bool = ...,
        line_endings: str | None = ...,
        unicode_normalization: str = ...,
    ) -> None: ...
    @staticmethod
    def minimal_preset() -> Options: ...
    @staticmethod
    def balanced_preset() -> Options: ...
    @staticmethod
    def humanize_preset() -> Options: ...
    @staticmethod
    def aggressive_preset() -> Options: ...
    @staticmethod
    def code_safe_preset() -> Options: ...

class Cleaner:
    """Reusable cleaner that returns ``CleaningResult`` objects."""

    def __init__(self, options: Options | None = ...) -> None: ...
    def clean(self, text: str) -> CleaningResult: ...
