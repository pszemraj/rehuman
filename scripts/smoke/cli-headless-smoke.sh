#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "${ROOT_DIR}"

REHUMAN_BIN="${ROOT_DIR}/target/debug/rehuman"
ISHUMAN_BIN="${ROOT_DIR}/target/debug/ishuman"
SMOKE_DIR="${ROOT_DIR}/target/smoke/cli"

section() {
  printf "\n== %s ==\n" "$1"
}

section "Build smoke-test binaries"
cargo build --quiet --bin rehuman --bin ishuman

rm -rf "${SMOKE_DIR}"
mkdir -p "${SMOKE_DIR}"

DIAGRAM_INPUT="${SMOKE_DIR}/diagram.md"
PROSE_INPUT="${SMOKE_DIR}/prose.txt"

cat > "${DIAGRAM_INPUT}" <<'EOF'
rehuman/
├── src/
│   ├── lib.rs
│   └── bin/
└── docs/
EOF

cat > "${PROSE_INPUT}" <<'EOF'
“Smart quotes”—with…emoji 👍 and non-breaking spaces.
EOF

section "Scenario 1: default mode vs code-safe preset on diagram content"
"${REHUMAN_BIN}" "${DIAGRAM_INPUT}" > "${SMOKE_DIR}/diagram.default.out"
"${REHUMAN_BIN}" --preset code-safe "${DIAGRAM_INPUT}" > "${SMOKE_DIR}/diagram.code_safe.out"

"${REHUMAN_BIN}" "${DIAGRAM_INPUT}" --stats-json >/dev/null 2> "${SMOKE_DIR}/diagram.default.stats.json"
"${REHUMAN_BIN}" --preset code-safe "${DIAGRAM_INPUT}" --stats-json >/dev/null 2> "${SMOKE_DIR}/diagram.code_safe.stats.json"

echo "Input/default/code-safe byte sizes:"
wc -c "${DIAGRAM_INPUT}" "${SMOKE_DIR}/diagram.default.out" "${SMOKE_DIR}/diagram.code_safe.out"

echo
echo "Default stats JSON:"
cat "${SMOKE_DIR}/diagram.default.stats.json"
echo
echo "Code-safe stats JSON:"
cat "${SMOKE_DIR}/diagram.code_safe.stats.json"

echo
echo "Diff: input -> default (expected: diagram glyphs dropped)"
diff -u "${DIAGRAM_INPUT}" "${SMOKE_DIR}/diagram.default.out" | sed -n '1,80p' || true

echo
echo "Diff: input -> code-safe (expected: no diff)"
diff -u "${DIAGRAM_INPUT}" "${SMOKE_DIR}/diagram.code_safe.out" || true

if ! cmp -s "${DIAGRAM_INPUT}" "${SMOKE_DIR}/diagram.code_safe.out"; then
  echo "ERROR: --preset code-safe did not preserve diagram input."
  exit 1
fi
if cmp -s "${DIAGRAM_INPUT}" "${SMOKE_DIR}/diagram.default.out"; then
  echo "ERROR: default mode unexpectedly preserved diagram input."
  exit 1
fi

section "Scenario 2: preset override precedence"
"${REHUMAN_BIN}" --preset code-safe --keyboard-only true "${DIAGRAM_INPUT}" > "${SMOKE_DIR}/diagram.code_safe_overridden.out"
if cmp -s "${DIAGRAM_INPUT}" "${SMOKE_DIR}/diagram.code_safe_overridden.out"; then
  echo "ERROR: explicit --keyboard-only true did not override --preset code-safe."
  exit 1
fi
echo "PASS: explicit flags override preset baseline."

section "Scenario 3: prose cleanup behavior"
"${REHUMAN_BIN}" "${PROSE_INPUT}" > "${SMOKE_DIR}/prose.default.out"
"${REHUMAN_BIN}" --preset code-safe "${PROSE_INPUT}" > "${SMOKE_DIR}/prose.code_safe.out"
"${REHUMAN_BIN}" --preset humanize "${PROSE_INPUT}" > "${SMOKE_DIR}/prose.humanize.out"

echo "Input:"
cat "${PROSE_INPUT}"
echo
echo "Default output:"
cat "${SMOKE_DIR}/prose.default.out"
echo
echo "Code-safe output:"
cat "${SMOKE_DIR}/prose.code_safe.out"
echo
echo "Humanize output:"
cat "${SMOKE_DIR}/prose.humanize.out"

section "Scenario 4: ishuman exit-code sanity"
set +e
"${ISHUMAN_BIN}" "${DIAGRAM_INPUT}" >/dev/null 2>&1
DEFAULT_EXIT=$?
"${ISHUMAN_BIN}" --preset code-safe "${DIAGRAM_INPUT}" >/dev/null 2>&1
CODE_SAFE_EXIT=$?
set -e

echo "ishuman default exit code: ${DEFAULT_EXIT} (expected 1)"
echo "ishuman --preset code-safe exit code: ${CODE_SAFE_EXIT} (expected 0)"

if [[ "${DEFAULT_EXIT}" -ne 1 || "${CODE_SAFE_EXIT}" -ne 0 ]]; then
  echo "ERROR: ishuman exit codes do not match expected behavior."
  exit 1
fi

section "Smoke test summary"
echo "PASS: all smoke scenarios completed successfully."
echo "Artifacts kept at: ${SMOKE_DIR}"
