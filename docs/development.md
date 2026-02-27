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

## Documentation Map

- Rust library semantics: [API Reference](api.md)
- CLI behavior and flags: [CLI Guide](cli.md)
- Rust usage recipes: [Examples](examples.md)
- Python package docs: [python/docs/index.md](../python/docs/index.md)
- Python release automation + PyPI flow: [python/docs/release.md](../python/docs/release.md)
