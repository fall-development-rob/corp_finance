#!/usr/bin/env bash
# register-data-mcp.sh
#
# Documents and (optionally) executes the `claude mcp add` commands for the
# Daloopa and Aiera MCP servers defined in the Anthropic financial-services plugin:
#   plugins/vertical-plugins/financial-analysis/.mcp.json (anthropics/financial-services)
#
# PREREQUISITES
# -------------
# Daloopa API key:
#   Sign up or request access at https://www.daloopa.com
#   Set:  export DALOOPA_API_KEY="your-key-here"
#
# Aiera API key:
#   Sign up via https://dashboard.aiera.com or email support@aiera.com
#   Set:  export AIERA_API_KEY="your-key-here"
#
# Both servers use HTTP transport with bearer-token authentication.
# The claude MCP client forwards an Authorization header using the env var value.
#
# USAGE
# -----
#   Dry-run (default -- prints commands, does not execute):
#     ./scripts/register-data-mcp.sh
#
#   Apply (actually register the MCP servers):
#     ./scripts/register-data-mcp.sh --apply
#
# NOTES
# -----
#   - Run `claude mcp list` to verify registration after applying.
#   - Run `claude mcp remove daloopa` or `claude mcp remove aiera` to deregister.
#   - These servers are project-scoped (registered in .claude/mcp.json for this repo).
#     Pass --scope user to register globally instead.

set -euo pipefail

APPLY=false
SCOPE="project"

for arg in "$@"; do
  case "$arg" in
    --apply) APPLY=true ;;
    --scope=*) SCOPE="${arg#--scope=}" ;;
    --help|-h)
      grep '^#' "$0" | sed 's/^# \{0,1\}//'
      exit 0
      ;;
  esac
done

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

check_env_var() {
  local varname="$1"
  if [[ -z "${!varname:-}" ]]; then
    echo "[WARN] \$$varname is not set. The server will be registered but requests will fail until the key is exported."
  else
    echo "[OK]   \$$varname is set (${#!varname} chars)."
  fi
}

run_or_print() {
  local label="$1"
  shift
  if [[ "$APPLY" == "true" ]]; then
    echo "[RUN]  $label"
    "$@"
  else
    echo "[DRY]  $label"
    echo "       Command: $*"
  fi
}

# ---------------------------------------------------------------------------
# Preflight checks
# ---------------------------------------------------------------------------

echo ""
echo "=== register-data-mcp.sh ==="
echo "Mode:  $([ "$APPLY" = true ] && echo 'APPLY (will register servers)' || echo 'DRY RUN (pass --apply to execute)')"
echo "Scope: $SCOPE"
echo ""

echo "--- Checking environment variables ---"
check_env_var "DALOOPA_API_KEY"
check_env_var "AIERA_API_KEY"
echo ""

# ---------------------------------------------------------------------------
# Daloopa MCP server
# URL confirmed from: plugins/vertical-plugins/financial-analysis/.mcp.json
# Transport: http (streamable HTTP)
# Auth: bearer token via DALOOPA_API_KEY
# ---------------------------------------------------------------------------

DALOOPA_URL="https://mcp.daloopa.com/server/mcp"

echo "--- Daloopa ---"
echo "URL:    $DALOOPA_URL"
echo "EnvVar: DALOOPA_API_KEY"
echo ""

run_or_print "Register daloopa MCP server (scope: $SCOPE)" \
  claude mcp add daloopa \
    --transport http \
    --url "$DALOOPA_URL" \
    --header "Authorization: Bearer \${DALOOPA_API_KEY}" \
    --scope "$SCOPE"

echo ""

# ---------------------------------------------------------------------------
# Aiera MCP server
# URL confirmed from: plugins/vertical-plugins/financial-analysis/.mcp.json
# Transport: http (streamable HTTP)
# Auth: bearer token via AIERA_API_KEY
# ---------------------------------------------------------------------------

AIERA_URL="https://mcp-pub.aiera.com"

echo "--- Aiera ---"
echo "URL:    $AIERA_URL"
echo "EnvVar: AIERA_API_KEY"
echo ""

run_or_print "Register aiera MCP server (scope: $SCOPE)" \
  claude mcp add aiera \
    --transport http \
    --url "$AIERA_URL" \
    --header "Authorization: Bearer \${AIERA_API_KEY}" \
    --scope "$SCOPE"

echo ""

# ---------------------------------------------------------------------------
# Post-registration verification (apply mode only)
# ---------------------------------------------------------------------------

if [[ "$APPLY" == "true" ]]; then
  echo "--- Verifying registration ---"
  claude mcp list | grep -E "daloopa|aiera" || echo "[WARN] Neither server appeared in 'claude mcp list'. Check for errors above."
  echo ""
  echo "Next steps:"
  echo "  1. Confirm tools are discoverable: claude mcp list-tools daloopa"
  echo "  2. Confirm tools are discoverable: claude mcp list-tools aiera"
  echo "  3. Update .claude/skills/data-daloopa/SKILL.md and data-aiera/SKILL.md"
  echo "     with actual tool names returned by the above commands."
else
  echo "Dry run complete. Re-run with --apply to register the servers."
  echo ""
  echo "Tip: ensure DALOOPA_API_KEY and AIERA_API_KEY are exported before applying."
fi
