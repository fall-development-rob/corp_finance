#!/usr/bin/env bash
# Self-consistency check for cfa-codegen.
#
# Usage:
#   scripts/verify-codegen-parity.sh
#
# This previously diff'd the Rust output against the legacy Python codegen
# scripts. Those scripts were removed in v0.3 — Rust is now the single
# source of truth. The check now snapshots the four generated artifacts,
# regenerates them via `cargo run -p cfa-codegen -- all`, and confirms the
# regeneration is idempotent (running it twice yields the same bytes).
#
# Exits non-zero if regeneration drifts from what's checked in or if the
# second run drifts from the first.
set -euo pipefail

REPO=$(cd "$(dirname "$0")/.." && pwd)
SNAPSHOT=/tmp/cfa-codegen-snapshot
RERUN=/tmp/cfa-codegen-rerun

# All paths the Rust emitters write to. Self-consistency means the
# committed copy of each must match what `cfa-codegen all` produces.
GENERATED_PATHS=(
    "crates/corp-finance-wasm/src/lib.rs"
    "plugins/cfa-core/mcp/src/tools.ts"
    "plugins/cfa-core/mcp/src/schemas.ts"
    "plugins/cfa-core/mcp/src/schemas.json"
    "plugins/cfa-core/examples"
)

cd "$REPO"

rm -rf "$SNAPSHOT" "$RERUN"
mkdir -p "$SNAPSHOT" "$RERUN"

snapshot_to() {
    local target=$1
    for path in "${GENERATED_PATHS[@]}"; do
        local dest="$target/${path//\//__}"
        if [ -d "$path" ]; then
            # `cp -r src dest` behaves differently depending on whether
            # dest exists, so always start from a clean slate then copy
            # the contents (not the dir itself) to make the snapshot
            # path deterministic.
            rm -rf "$dest"
            mkdir -p "$dest"
            cp -r "$path"/. "$dest"/
        else
            cp "$path" "$dest"
        fi
    done
}

echo "=== Snapshotting committed generated artifacts ==="
snapshot_to "$SNAPSHOT"

echo "=== Building cfa-codegen ==="
cargo build -p cfa-codegen --quiet

echo "=== Run 1: cfa-codegen all ==="
cargo run -p cfa-codegen --quiet -- all >/dev/null

echo "=== Diffing against committed snapshot ==="
fail=0
check_dirs_match() {
    local label=$1
    local before=$2
    local after=$3
    local pass_msg=${4:-OK}
    if diff -rq "$before" "$after" >/dev/null 2>&1; then
        printf "  %-50s %s\n" "$label" "$pass_msg"
    else
        printf "  %-50s DRIFTED\n" "$label"
        diff -ru "$before" "$after" 2>/dev/null | head -40 || true
        fail=1
    fi
}

for path in "${GENERATED_PATHS[@]}"; do
    snapped="$SNAPSHOT/${path//\//__}"
    check_dirs_match "$path" "$snapped" "$path" "OK (matches committed)"
done

echo
echo "=== Run 2: cfa-codegen all (idempotency check) ==="
snapshot_to "$RERUN"
cargo run -p cfa-codegen --quiet -- all >/dev/null

echo "=== Diffing run 2 against run 1 ==="
for path in "${GENERATED_PATHS[@]}"; do
    rerun_snap="$RERUN/${path//\//__}"
    check_dirs_match "$path (idempotent)" "$rerun_snap" "$path" "OK (run 2 == run 1)"
done

echo
if [ "$fail" -eq 0 ]; then
    echo "PARITY: all checks passed (committed artifacts match cfa-codegen output, regeneration is idempotent)."
    exit 0
else
    echo "PARITY: FAILED — see diffs above."
    echo
    echo "Hint: if the drift is intentional (you changed an Input struct or"
    echo "the codegen logic itself), re-run 'cargo run -p cfa-codegen -- all'"
    echo "and commit the regenerated artifacts."
    exit 1
fi
