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
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
    def __bool__(self) -> bool: ...
    def __eq__(self, other: CleaningResult, /) -> bool: ...

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
    def replace(
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
    ) -> Options: ...
    @property
    def remove_hidden(self) -> bool: ...
    @property
    def remove_trailing_whitespace(self) -> bool: ...
    @property
    def normalize_spaces(self) -> bool: ...
    @property
    def normalize_dashes(self) -> bool: ...
    @property
    def normalize_quotes(self) -> bool: ...
    @property
    def normalize_other(self) -> bool: ...
    @property
    def keyboard_only(self) -> bool: ...
    @property
    def extended_keyboard(self) -> bool: ...
    @property
    def keep_emoji(self) -> bool: ...
    @property
    def non_ascii_policy(self) -> str: ...
    @property
    def preserve_joiners(self) -> bool: ...
    @property
    def remove_control_chars(self) -> bool: ...
    @property
    def collapse_whitespace(self) -> bool: ...
    @property
    def line_endings(self) -> str | None: ...
    @property
    def unicode_normalization(self) -> str: ...
    def __eq__(self, other: Options, /) -> bool: ...
    def __repr__(self) -> str: ...

class Cleaner:
    """Reusable cleaner that returns ``CleaningResult`` objects."""

    def __init__(self, options: Options | None = ...) -> None: ...
    def clean(self, text: str) -> CleaningResult: ...
    def __repr__(self) -> str: ...
