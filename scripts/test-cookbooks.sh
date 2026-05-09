#!/usr/bin/env bash
# Smoke test every managed-agent cookbook: validate manifest + dry-run deploy.
#
# Replaces the upstream `python3 scripts/test-cookbooks.py` harness with our
# deterministic Rust CLI plus a thin bash wrapper. No network calls.
#
# Usage:
#   scripts/test-cookbooks.sh              # test every cookbook
#   scripts/test-cookbooks.sh <slug> ...   # test specific cookbooks only
#
# Exit codes:
#   0 — all tested cookbooks passed validation and dry-run deploy
#   1 — at least one cookbook failed (full output printed)

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
COOKBOOKS_DIR="${REPO_ROOT}/managed-agent-cookbooks"

die() { echo "ERROR: $*" >&2; exit 1; }
info() { echo "[test-cookbooks] $*"; }

[[ -d "$COOKBOOKS_DIR" ]] || die "Cookbooks directory not found: $COOKBOOKS_DIR"

discover_slugs() {
  find "$COOKBOOKS_DIR" -mindepth 2 -maxdepth 2 -name agent.json -printf '%h\n' \
    | xargs -I{} basename {} \
    | sort
}

if [[ $# -gt 0 ]]; then
  SLUGS=("$@")
else
  mapfile -t SLUGS < <(discover_slugs)
fi

[[ ${#SLUGS[@]} -gt 0 ]] || die "No cookbooks discovered under $COOKBOOKS_DIR"

info "Testing ${#SLUGS[@]} cookbook(s): ${SLUGS[*]}"

# Phase 29 Wave 18: corp-finance-cli was extracted to
# github.com/rob-otix-ai/corp-finance-core. Use the `cfa` binary from $PATH —
# install it once via:
#   cargo install --git https://github.com/rob-otix-ai/corp-finance-core \
#     --tag v1.1.0 corp-finance-cli --locked
if ! CLI_BIN="$(command -v cfa)"; then
  die "cfa CLI not found in PATH. Install with: cargo install --git https://github.com/rob-otix-ai/corp-finance-core --tag v1.1.0 corp-finance-cli --locked"
fi
info "Using cfa CLI at: $CLI_BIN"

PASS=0
FAIL=0
FAILED_SLUGS=()

for slug in "${SLUGS[@]}"; do
  info "── ${slug} ──────────────────────────────────────"

  # 1. Schema/reference validation (no network).
  if ! "$CLI_BIN" managed-agent validate "$slug" \
       --cookbooks-root "$COOKBOOKS_DIR" \
       --skills-root "${REPO_ROOT}/.claude/skills" \
       --agents-root "${REPO_ROOT}/.claude/agents/cfa" \
       > /tmp/cfa-test-validate-"$slug".json
  then
    info "  validate: FAIL"
    FAIL=$((FAIL+1))
    FAILED_SLUGS+=("$slug:validate")
    continue
  fi

  # `ok: true` is the contract — false means at least one check failed.
  # Match both compact (`"ok":true`) and pretty (`"ok": true`) JSON output.
  if ! grep -Eq '"ok":[[:space:]]*true' /tmp/cfa-test-validate-"$slug".json; then
    info "  validate: FAIL (ok=false). Failed checks:"
    grep -E '"passed":[[:space:]]*false' -B 1 -A 1 /tmp/cfa-test-validate-"$slug".json || true
    FAIL=$((FAIL+1))
    FAILED_SLUGS+=("$slug:validate")
    continue
  fi
  info "  validate: ok"

  # 2. Dry-run deploy payload assembly via the Rust CLI (no network, no jq dependency).
  #    `cfa managed-agent deploy` returns the same JSON the upstream Python tooling
  #    would have produced — orchestrator + subagent + skill payloads — but performs
  #    the assembly via corp-finance-core::managed_agent::deploy::build_deploy_payload.
  if "$CLI_BIN" managed-agent deploy "$slug" \
       --cookbooks-root "$COOKBOOKS_DIR" \
       --skills-root "${REPO_ROOT}/.claude/skills" \
       --agents-root "${REPO_ROOT}/.claude/agents/cfa" \
       > /tmp/cfa-test-deploy-"$slug".json 2>/tmp/cfa-test-deploy-"$slug".err
  then
    info "  deploy (dry-run payload): ok"
    PASS=$((PASS+1))
  else
    info "  deploy (dry-run payload): FAIL"
    tail -20 /tmp/cfa-test-deploy-"$slug".err
    FAIL=$((FAIL+1))
    FAILED_SLUGS+=("$slug:deploy")
  fi
done

info "═════════════════════════════════════════════════"
info "Result: ${PASS} passed, ${FAIL} failed (of ${#SLUGS[@]} tested)"

if [[ $FAIL -gt 0 ]]; then
  info "Failed: ${FAILED_SLUGS[*]}"
  exit 1
fi
