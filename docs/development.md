# Development

## Roadmap

> [!TIP]
> Interested in seeing a feature sooner? Open a request at [GitHub Issues](https://github.com/pszemraj/rehuman/issues/new/choose).

- [ ] Expand transliteration coverage and tune script-specific mappings beyond Latin-focused defaults
- [ ] Extend `extended_keyboard` coverage with script-specific opt-in profiles
- [ ] Tune preset-level defaults for `preserve_joiners` in script-sensitive contexts
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
- Python docs index: [python/docs/index.md](../python/docs/index.md)
