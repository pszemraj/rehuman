# Development

## Roadmap

> [!TIP]
> Interested in seeing a feature sooner? Open a request at [GitHub Issues](https://github.com/pszemraj/rehuman/issues/new/choose).

- [ ] International keyboard mode with a curated non-ASCII allowlist (€, £, §, …)
- [ ] Optional transliteration when dropping non-ASCII characters in keyboard-only mode
- [ ] Toggle to preserve join controls and handle additional ellipsis variants
- [ ] Automated Unicode data refresh (script + CI)
- [ ] Benchmark suite (contributions welcome)

## Implementation Notes

- `rehuman` is a Rust rewrite and expansion of
  [humanize-ai-lib](https://github.com/Nordth/humanize-ai-lib) by
  [Nordth](https://github.com/Nordth).
- Unicode-derived tables are generated at build time; no network traffic occurs at runtime.

## Documentation Ownership

- Rust library semantics: [API Reference](api.md)
- CLI behavior and flags: [CLI Guide](cli.md)
- Rust usage recipes: [Examples](examples.md)
- Python package docs: [python/docs/index.md](../python/docs/index.md)
