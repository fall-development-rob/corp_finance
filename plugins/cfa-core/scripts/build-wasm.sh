#!/usr/bin/env bash
# Build the corp-finance-wasm crate and copy artifacts into the plugin's mcp/wasm/ directory.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
PLUGIN_DIR="$REPO_ROOT/plugins/cfa-core"
CRATE_DIR="$REPO_ROOT/crates/corp-finance-wasm"
OUT_DIR="$PLUGIN_DIR/mcp/wasm"

if ! command -v wasm-pack >/dev/null 2>&1; then
    echo "error: wasm-pack not installed. install with:"
    echo "    curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh"
    exit 1
fi

echo "[cfa-core] building WASM (release, target=nodejs)..."
wasm-pack build "$CRATE_DIR" \
    --release \
    --target nodejs \
    --out-dir "$OUT_DIR" \
    --out-name corp_finance_wasm

echo "[cfa-core] artifact:"
ls -lh "$OUT_DIR"/*.wasm
echo "[cfa-core] done."
