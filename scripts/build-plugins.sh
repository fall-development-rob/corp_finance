#!/usr/bin/env bash
# Build all three cfa-* plugins for distribution.
#
# For each companion plugin (cfa-data, cfa-pro), this:
# 1. Builds the underlying MCP server in packages/
# 2. Copies the built dist/ + package.json + node_modules into the plugin's
#    own mcp/ folder so the plugin manifest can use a self-contained relative
#    path instead of ../../packages/...
#
# After this script runs, each plugin folder is self-contained and can be
# zipped, copied, or git-shared without bringing the whole monorepo along.
#
# cfa-core is already self-contained (it builds its own WASM artifact via
# plugins/cfa-core/scripts/build-wasm.sh).

set -euo pipefail

REPO="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO"

bundle_plugin() {
    local plugin="$1"
    local source_pkg="$2"
    local plugin_dir="$REPO/plugins/$plugin"
    local source_dir="$REPO/packages/$source_pkg"

    echo ""
    echo "=== Building $plugin (source: $source_pkg) ==="

    # 1. Ensure the source MCP server is built.
    (cd "$source_dir" && npm run build > /dev/null)

    # 2. Mirror dist/ and package.json into the plugin folder.
    local plugin_mcp="$plugin_dir/mcp"
    mkdir -p "$plugin_mcp"
    cp -r "$source_dir/dist" "$plugin_mcp/"
    cp "$source_dir/package.json" "$plugin_mcp/"
    [ -f "$source_dir/package-lock.json" ] && cp "$source_dir/package-lock.json" "$plugin_mcp/"

    # 3. Production install for runtime deps only (no dev deps needed at runtime).
    (cd "$plugin_mcp" && npm install --omit=dev --silent --no-audit --no-fund 2>&1 | tail -3 || true)

    echo "  $plugin: bundled $(du -sh "$plugin_mcp" | cut -f1)"
}

# cfa-core already builds its own WASM artifact and TS server.
echo "=== Building cfa-core (WASM + TS) ==="
"$REPO/plugins/cfa-core/scripts/build-wasm.sh" 2>&1 | tail -2
(cd "$REPO/plugins/cfa-core/mcp" && npm run build > /dev/null)
echo "  cfa-core: $(du -sh "$REPO/plugins/cfa-core" | cut -f1)"

# cfa-data wraps the data-mcp-server.
bundle_plugin "cfa-data" "data-mcp-server"

# cfa-pro wraps two MCP servers (FMP + vendors). Both bundled into the same
# plugin folder; the plugin.json registers them as separate mcpServers entries.
bundle_plugin "cfa-pro" "fmp-mcp-server"
# vendor-mcp-server bundle target lives alongside fmp-mcp-server inside cfa-pro/mcp/
echo ""
echo "=== Bundling vendor-mcp-server into cfa-pro ==="
mkdir -p "$REPO/plugins/cfa-pro/mcp/vendors"
(cd "$REPO/packages/vendor-mcp-server" && npm run build > /dev/null)
cp -r "$REPO/packages/vendor-mcp-server/dist" "$REPO/plugins/cfa-pro/mcp/vendors/"
cp "$REPO/packages/vendor-mcp-server/package.json" "$REPO/plugins/cfa-pro/mcp/vendors/"
echo "  vendors bundled into cfa-pro/mcp/vendors"

echo ""
echo "=== Done. Plugin sizes ==="
du -sh "$REPO/plugins"/*/ | sort -h
