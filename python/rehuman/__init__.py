"""Python bindings for rehuman text cleaning.

This package re-exports the native extension API from ``rehuman._rehuman``.
Top-level ``clean``/``humanize`` helpers return text only; use ``Cleaner``
for detailed change counts and stats.
"""

from ._rehuman import (
    HAS_SECURITY,
    HAS_STATS,
    __version__,
    Cleaner,
    CleaningResult,
    Options,
    clean,
    humanize,
)

__all__ = [
    "__version__",
    "HAS_SECURITY",
    "HAS_STATS",
    "clean",
    "humanize",
    "Options",
    "Cleaner",
    "CleaningResult",
]
