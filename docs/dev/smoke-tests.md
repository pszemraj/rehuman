# Smoke Tests

Manual, headless smoke tests complement unit/integration tests by exercising
practical CLI flows and printing reviewable terminal output.

## Run

From repo root, run this command block directly (or put your own local wrapper
under `local-scratch/`):

```bash
set -euo pipefail
cargo build --quiet --bin rehuman --bin ishuman

SMOKE_DIR="target/smoke/cli"
rm -rf "${SMOKE_DIR}"
mkdir -p "${SMOKE_DIR}"

printf '%s\n' \
  'rehuman/' \
  $'\u251c\u2500\u2500 src/' \
  $'\u2502   \u251c\u2500\u2500 lib.rs' \
  $'\u2502   \u2514\u2500\u2500 bin/' \
  $'\u2514\u2500\u2500 docs/' \
  > "${SMOKE_DIR}/diagram.md"

printf '%s\n' \
  $'\u201cSmart quotes\u201d\u2014with\u2026emoji \U0001F44D and non-breaking spaces.' \
  > "${SMOKE_DIR}/prose.txt"

target/debug/rehuman "${SMOKE_DIR}/diagram.md" > "${SMOKE_DIR}/diagram.default.out"
target/debug/rehuman --preset code-safe "${SMOKE_DIR}/diagram.md" > "${SMOKE_DIR}/diagram.code_safe.out"
target/debug/rehuman "${SMOKE_DIR}/diagram.md" --stats-json >/dev/null 2> "${SMOKE_DIR}/diagram.default.stats.json"
target/debug/rehuman --preset code-safe "${SMOKE_DIR}/diagram.md" --stats-json >/dev/null 2> "${SMOKE_DIR}/diagram.code_safe.stats.json"

target/debug/rehuman --preset code-safe --keyboard-only true "${SMOKE_DIR}/diagram.md" > "${SMOKE_DIR}/diagram.code_safe_overridden.out"

target/debug/rehuman "${SMOKE_DIR}/prose.txt" > "${SMOKE_DIR}/prose.default.out"
target/debug/rehuman --preset code-safe "${SMOKE_DIR}/prose.txt" > "${SMOKE_DIR}/prose.code_safe.out"
target/debug/rehuman --preset humanize "${SMOKE_DIR}/prose.txt" > "${SMOKE_DIR}/prose.humanize.out"

target/debug/ishuman "${SMOKE_DIR}/diagram.md" >/dev/null 2>&1; DEFAULT_EXIT=$?
target/debug/ishuman --preset code-safe "${SMOKE_DIR}/diagram.md" >/dev/null 2>&1; CODE_SAFE_EXIT=$?
echo "ishuman default exit code: ${DEFAULT_EXIT} (expected 1)"
echo "ishuman --preset code-safe exit code: ${CODE_SAFE_EXIT} (expected 0)"
```

Artifacts are written to:

```text
target/smoke/cli/
```

## What It Covers

- Default CLI behavior versus `--preset code-safe` for Unicode diagram content
- Preset override precedence (explicit flags override preset baseline)
- Practical prose cleanup outputs across default / `code-safe` / `humanize`
- `ishuman` exit-code behavior for default versus `code-safe` content checks

## Maintainer Guidance

- Run this flow manually before committing changes that affect:
  - CLI defaults, presets, or routing
  - text transformation behavior
  - output/exit-code semantics
- Treat this output as a human-review gate, not a replacement for tests.
