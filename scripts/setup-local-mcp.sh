#!/bin/bash
#
# setup-local-mcp.sh — Helper script to verify and guide local MCP setup for CFA subagents.
#
# This script:
#   1. Checks for the pre-built NAPI binary
#   2. Builds packages/mcp-server if dist/index.js is missing
#   3. Prints the exact JSON snippet for ~/.claude.json
#   4. Does NOT modify ~/.claude.json directly (user must paste manually)
#
# Usage:
#   scripts/setup-local-mcp.sh
#

set -e

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BINDINGS_DIR="${REPO_ROOT}/packages/bindings"
MCP_SERVER_DIR="${REPO_ROOT}/packages/mcp-server"
NAPI_BINARY="${BINDINGS_DIR}/corp-finance.linux-x64-gnu.node"
MCP_DIST="${MCP_SERVER_DIR}/dist/index.js"

echo "=== Local MCP Setup Helper ==="
echo ""

# Check for NAPI binary
echo "[1/3] Checking NAPI binary..."
if [ -f "$NAPI_BINARY" ]; then
  BINARY_SIZE=$(stat -c%s "$NAPI_BINARY" 2>/dev/null || stat -f%z "$NAPI_BINARY" 2>/dev/null || echo "unknown")
  echo "  ✓ Found: $NAPI_BINARY ($BINARY_SIZE bytes)"
else
  echo "  ✗ Missing: $NAPI_BINARY"
  echo "  Building NAPI binary..."
  cd "$BINDINGS_DIR"
  npm run build
  cd "$REPO_ROOT"
  echo "  ✓ NAPI binary built"
fi

echo ""

# Check for MCP server dist
echo "[2/3] Checking MCP server build..."
if [ -f "$MCP_DIST" ]; then
  echo "  ✓ Found: $MCP_DIST"
else
  echo "  ✗ Missing: $MCP_DIST"
  echo "  Building MCP server..."
  cd "$MCP_SERVER_DIR"
  npm run build
  cd "$REPO_ROOT"
  echo "  ✓ MCP server built"
fi

echo ""

# Print the JSON snippet
echo "[3/3] JSON snippet for ~/.claude.json"
echo ""
echo "Add the following to your ~/.claude.json under projects[\"$REPO_ROOT\"].mcpServers:"
echo ""
cat <<'EOF'
{
  "projects": {
    "/home/robert/cfa_agent": {
      "mcpServers": {
        "cfa-core": {
          "command": "node",
          "args": ["/home/robert/cfa_agent/plugins/cfa-core/mcp/dist/index.js"]
        },
        "corp-finance-mcp": {
          "command": "node",
          "args": ["/home/robert/cfa_agent/packages/mcp-server/dist/index.js"]
        }
      }
    }
  }
}
EOF

echo ""
echo "(Replace /home/robert/cfa_agent with your absolute repo path if different.)"
echo ""
echo "Next steps:"
echo "  1. Copy the JSON snippet above"
echo "  2. Edit ~/.claude.json and add/update the 'projects' section"
echo "  3. Update .claude/agents/cfa/*.md to add 'corp-finance-mcp' to the 'tools:' list"
echo "  4. Restart Claude Code"
echo "  5. Spawn a CFA subagent and verify with 'ListTools'"
echo ""
echo "See docs/local-mcp-setup.md for detailed instructions."
