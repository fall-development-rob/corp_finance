#!/usr/bin/env bash
# Verify cfa-codegen Rust output matches the legacy Python scripts.
#
# Usage:
#   scripts/verify-codegen-parity.sh
#
# Exits non-zero if any *unexpected* divergence appears. The 5 currently
# documented divergences (see crates/cfa-codegen/README.md) are tolerated
# by name — they are bug fixes where the Rust parser correctly disambiguates
# Input structs that share a name across modules.
set -euo pipefail

REPO=$(cd "$(dirname "$0")/.." && pwd)
PY_OUT=/tmp/cfa-codegen-py-out
RS_OUT=/tmp/cfa-codegen-rs-out

# Documented divergences (Rust correctness fixes — see crates/cfa-codegen/README.md).
EXPECTED_EXAMPLE_DIFFS=(
  "analyze_breakeven.json"
  "calculate_waterfall.json"
  "comp_reconciliation.json"
  "run_black_litterman.json"
)

cd "$REPO"

rm -rf "$PY_OUT" "$RS_OUT"
mkdir -p "$PY_OUT/examples" "$RS_OUT/examples"

echo "=== Capturing Python baseline ==="
python3 scripts/gen-wasm-bindings.py >/dev/null
cp crates/corp-finance-wasm/src/lib.rs "$PY_OUT/wasm-lib.rs"
python3 scripts/gen-mcp-tools.py >/dev/null
cp plugins/cfa-core/mcp/src/tools.ts "$PY_OUT/tools.ts"
python3 scripts/gen-zod-schemas.py >/dev/null
cp plugins/cfa-core/mcp/src/schemas.ts "$PY_OUT/schemas.ts"
cp plugins/cfa-core/mcp/src/schemas.json "$PY_OUT/schemas.json"
python3 scripts/gen-examples.py >/dev/null
cp plugins/cfa-core/examples/*.json "$PY_OUT/examples/"
cp plugins/cfa-core/examples/README.md "$PY_OUT/examples-README.md"

echo "=== Building cfa-codegen ==="
cargo build -p cfa-codegen --quiet

echo "=== Running cfa-codegen all ==="
cargo run -p cfa-codegen --quiet -- all >/dev/null
cp crates/corp-finance-wasm/src/lib.rs "$RS_OUT/wasm-lib.rs"
cp plugins/cfa-core/mcp/src/tools.ts "$RS_OUT/tools.ts"
cp plugins/cfa-core/mcp/src/schemas.ts "$RS_OUT/schemas.ts"
cp plugins/cfa-core/mcp/src/schemas.json "$RS_OUT/schemas.json"
cp plugins/cfa-core/examples/*.json "$RS_OUT/examples/"
cp plugins/cfa-core/examples/README.md "$RS_OUT/examples-README.md"

echo
echo "=== Diffing artifacts ==="
fail=0

check_identical() {
    local label=$1
    local py=$2
    local rs=$3
    if diff -q "$py" "$rs" >/dev/null; then
        printf "  %-40s OK (byte-identical)\n" "$label"
    else
        printf "  %-40s DIVERGED (unexpected)\n" "$label"
        diff -u "$py" "$rs" | head -40
        fail=1
    fi
}

check_identical "wasm-lib.rs"        "$PY_OUT/wasm-lib.rs"        "$RS_OUT/wasm-lib.rs"
check_identical "tools.ts"           "$PY_OUT/tools.ts"           "$RS_OUT/tools.ts"
check_identical "schemas.json"       "$PY_OUT/schemas.json"       "$RS_OUT/schemas.json"
check_identical "examples/README.md" "$PY_OUT/examples-README.md" "$RS_OUT/examples-README.md"

# schemas.ts: tolerate the documented module-path correctness fix.
# Each schema is one z.object({...}) block — the tool name appears as a
# context (unchanged) line just above the changed lines. We extract it by
# walking the diff: for every line starting `@@`, look at the next context
# line that ends with `: z.object({`.
EXPECTED_TS_CHANGED=(
  "analyze_breakeven"
  "calculate_waterfall"
  "comp_reconciliation"
  "run_black_litterman"
)
echo
echo "=== schemas.ts (semantic check) ==="
if diff -q "$PY_OUT/schemas.ts" "$RS_OUT/schemas.ts" >/dev/null; then
    echo "  schemas.ts                              OK (byte-identical)"
else
    actual_changed=$(diff -u "$PY_OUT/schemas.ts" "$RS_OUT/schemas.ts" \
        | awk '
            /^@@/ { in_hunk=1; next }
            # Context line in unified diff has a leading space, then the
            # original indentation (2 spaces) and the tool name + ": z.object(".
            in_hunk && /^   [a-z_]+: z\.object/ {
                # Strip leading whitespace and trailing colon-onwards.
                sub(/^[[:space:]]+/, "", $0)
                sub(/:.*/, "", $0)
                print $0
                in_hunk=0
            }
        ' | sort -u || true)
    unexpected=""
    for tool in $actual_changed; do
        if ! printf '%s\n' "${EXPECTED_TS_CHANGED[@]}" | grep -qx "$tool"; then
            unexpected="$unexpected $tool"
        fi
    done
    if [ -z "$unexpected" ]; then
        n=$(echo "$actual_changed" | grep -c . || true)
        echo "  schemas.ts                              OK ($n documented divergence(s): $(echo $actual_changed | tr '\n' ' '))"
    else
        echo "  schemas.ts                              DIVERGED (unexpected:$unexpected)"
        diff -u "$PY_OUT/schemas.ts" "$RS_OUT/schemas.ts" | head -80
        fail=1
    fi
fi

# examples: tolerate the 4 documented divergences.
echo
echo "=== examples/*.json (per-file check) ==="
diverged_examples=$(diff -rq "$PY_OUT/examples/" "$RS_OUT/examples/" \
    | grep -oE '[a-z_]+\.json' | sort -u || true)

for diff_file in $diverged_examples; do
    if printf '%s\n' "${EXPECTED_EXAMPLE_DIFFS[@]}" | grep -qx "$diff_file"; then
        printf "  %-40s OK (documented divergence)\n" "$diff_file"
    else
        printf "  %-40s DIVERGED (unexpected)\n" "$diff_file"
        diff -u "$PY_OUT/examples/$diff_file" "$RS_OUT/examples/$diff_file" | head -40
        fail=1
    fi
done

echo
if [ "$fail" -eq 0 ]; then
    echo "PARITY: all checks passed (byte-identical for 4 artifacts; documented divergences for schemas.ts and 4 example files)."
    exit 0
else
    echo "PARITY: FAILED — see diffs above."
    exit 1
fi
