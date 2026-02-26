# Smoke Tests

Manual, headless smoke tests complement unit/integration tests by exercising
practical CLI flows and printing reviewable terminal output.

## Run

From repo root:

```bash
./scripts/smoke/cli-headless-smoke.sh
```

The script builds `rehuman` and `ishuman`, runs scenario-style checks, and
writes artifacts to:

```text
target/smoke/cli/
```

## What It Covers

- Default CLI behavior versus `--preset code-safe` for Unicode diagram content
- Preset override precedence (explicit flags override preset baseline)
- Practical prose cleanup outputs across default / `code-safe` / `humanize`
- `ishuman` exit-code behavior for default versus `code-safe` content checks

## Maintainer Guidance

- Run this script manually before committing changes that affect:
  - CLI defaults, presets, or routing
  - text transformation behavior
  - output/exit-code semantics
- Treat the script output as a human-review gate, not a replacement for tests.
