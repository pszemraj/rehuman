# Development

## Roadmap

> [!TIP]
> Interested in seeing a feature sooner? Open a request at [GitHub Issues](https://github.com/pszemraj/rehuman/issues/new/choose).

- [x] Expand transliteration coverage beyond Latin-focused defaults (curated symbol table + scoped `deunicode` fallback for arrows, math operators, bullets, shapes, and letterlike marks)
- [ ] Tune script-specific mappings (letter scripts currently drop by design; script-aware opt-in transliteration is unexplored)
- [ ] Extend `extended_keyboard` coverage with script-specific opt-in profiles
- [ ] Tune `preserve_joiners` preset defaults for script-sensitive contexts beyond `code-safe` (which already preserves joiners)
- [ ] Handle additional ellipsis/compatibility punctuation variants
- [ ] Automated Unicode data refresh (script + CI)
- [ ] Benchmark suite (contributions welcome)

## Implementation Notes

- `rehuman` is a Rust rewrite and expansion of
  [humanize-ai-lib](https://github.com/Nordth/humanize-ai-lib) by
  [Nordth](https://github.com/Nordth).
- Unicode-derived tables are generated at build time; no network traffic occurs at runtime.

## Documentation

- Rust docs index: [README](../README.md#documentation)
- Python package docs: [python/README.md](../python/README.md)
