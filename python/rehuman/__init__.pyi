from __future__ import annotations

__version__: str
HAS_STATS: bool
HAS_SECURITY: bool

def clean(text: str) -> str: ...
def humanize(text: str) -> str: ...

class CleaningResult:
    @property
    def text(self) -> str: ...
    @property
    def changes_made(self) -> int: ...
    @property
    def stats(self) -> dict[str, int]: ...

class Options:
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
        keep_emoji: bool = ...,
        remove_control_chars: bool = ...,
        collapse_whitespace: bool = ...,
        line_endings: str | None = ...,
        unicode_normalization: str = ...,
        strip_bidi_controls: bool = ...,
    ) -> None: ...
    @staticmethod
    def minimal_preset() -> Options: ...
    @staticmethod
    def balanced_preset() -> Options: ...
    @staticmethod
    def humanize_preset() -> Options: ...
    @staticmethod
    def aggressive_preset() -> Options: ...

class Cleaner:
    def __init__(self, options: Options | None = ...) -> None: ...
    def clean(self, text: str) -> CleaningResult: ...
