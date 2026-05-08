#!/usr/bin/env bash
# Phase 29 Wave 18 — Delegate WASM build to the extracted corp-finance-core repo,
# then copy artefacts into this plugin's wasm/ directory.
# corp-finance-core is the source-of-truth for WASM builds; cfa_agent only consumes output.
set -euo pipefail

COREREPO="/home/robert/corp-finance-core"
PLUGIN_WASM="/home/robert/cfa_agent/plugins/cfa-core/mcp/wasm"

if [ ! -d "$COREREPO" ]; then
    echo "error: corp-finance-core repo not found at $COREREPO"
    exit 1
fi

echo "[cfa-core] delegating WASM build to $COREREPO ..."
cd "$COREREPO" && bash scripts/build-wasm.sh

mkdir -p "$PLUGIN_WASM"
cp -f "$COREREPO/dist/wasm/corp_finance_wasm.js" "$PLUGIN_WASM/"
cp -f "$COREREPO/dist/wasm/corp_finance_wasm_bg.wasm" "$PLUGIN_WASM/"
cp -f "$COREREPO/dist/wasm/corp_finance_wasm_bg.wasm.d.ts" "$PLUGIN_WASM/"
cp -f "$COREREPO/dist/wasm/corp_finance_wasm.d.ts" "$PLUGIN_WASM/"

echo "[cfa-core] artifact:"
ls -lh "$PLUGIN_WASM/corp_finance_wasm_bg.wasm"
echo "[cfa-core] done."
