# Development

## Roadmap

> [!TIP]
> Interested in seeing a feature sooner? See [form here](https://github.com/pszemraj/rehuman/compare)

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
- The CLI defaults to keyboard-only output and drops emoji unless explicitly told otherwise.
