#!/usr/bin/env bash
#
# DEPRECATED — DO NOT USE.
#
# This script reads `managed-agent-cookbooks/<slug>/agent.json`, but the
# cookbooks migrated to YAML in Phase 36. Run against any current cookbook
# this script fails with "Manifest not found: …agent.json".
#
# Replacement (Phase 28 D1):
#
#   npx tsx scripts/deploy-cookbook.ts <slug>           # dry-run JSON to stdout
#   npx tsx scripts/deploy-cookbook.ts <slug> --apply   # actual deploy
#
# The replacement reads agent.yaml, substitutes ${VAR} env placeholders,
# inlines skills + subagents, and POSTs in the correct order. Snapshot
# baselines for every cookbook live at data/cookbook-deploy-payloads/.

cat <<'NOTE' >&2
[deploy-managed-agent.sh] DEPRECATED — use scripts/deploy-cookbook.ts.

  npx tsx scripts/deploy-cookbook.ts <slug>           # dry-run
  npx tsx scripts/deploy-cookbook.ts <slug> --apply   # actual deploy

NOTE
exit 1

# --- Original implementation preserved below for reference ---
# Adapted from: anthropics/financial-services scripts/deploy-managed-agent.sh
#
# Deploy a CFA managed agent to the Anthropic Managed Agents API (/v1/agents).
# Resolves manifest conveniences before posting:
#   - Inlines system prompts from .claude/agents/cfa/<agent>.md files
#   - Uploads skill files from .claude/skills/ via /v1/skills
#   - Creates callable subagents first, then the orchestrator
#
# Usage:
#   scripts/deploy-managed-agent.sh <slug> [--dry-run | --apply]
#
# Examples:
#   scripts/deploy-managed-agent.sh equity-analyst --dry-run
#   scripts/deploy-managed-agent.sh equity-analyst --apply
#
# Requirements:
#   - ANTHROPIC_API_KEY environment variable (skipped in --dry-run)
#   - jq >= 1.6, envsubst (gettext)
#   - Agent manifest at managed-agent-cookbooks/<slug>/agent.json

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
COOKBOOKS_DIR="${REPO_ROOT}/managed-agent-cookbooks"
AGENTS_DIR="${REPO_ROOT}/.claude/agents/cfa"
SKILLS_DIR="${REPO_ROOT}/.claude/skills"
API_BASE="https://api.anthropic.com"
API_VERSION="2023-06-01"
BETA_HEADER="managed-agents-2026-05-06"

die() { echo "ERROR: $*" >&2; exit 1; }
info() { echo "[deploy] $*"; }

require_cmd() {
  command -v "$1" >/dev/null 2>&1 || die "Required command not found: $1"
}

require_cmd jq
require_cmd envsubst
require_cmd curl

# Read a JSON manifest with envvar substitution applied.
read_manifest() {
  local path="$1"
  [[ -f "$path" ]] || die "Manifest not found: $path"
  envsubst < "$path"
}

SLUG=""
MODE="--dry-run"

for arg in "$@"; do
  case "$arg" in
    --dry-run) MODE="--dry-run" ;;
    --apply)   MODE="--apply" ;;
    -*)        die "Unknown flag: $arg" ;;
    *)
      [[ -z "$SLUG" ]] || die "Extra positional argument: $arg"
      SLUG="$arg"
      ;;
  esac
done

[[ -n "$SLUG" ]] || die "Usage: deploy-managed-agent.sh <slug> [--dry-run | --apply]"

MANIFEST="${COOKBOOKS_DIR}/${SLUG}/agent.json"
[[ -f "$MANIFEST" ]] || die "Manifest not found: ${MANIFEST}"

if [[ "$MODE" == "--apply" ]]; then
  [[ -n "${ANTHROPIC_API_KEY:-}" ]] || die "ANTHROPIC_API_KEY environment variable is required for --apply"
fi

info "Slug:     ${SLUG}"
info "Mode:     ${MODE}"
info "Manifest: ${MANIFEST}"

MANIFEST_JSON="$(read_manifest "$MANIFEST" | jq .)"
info "Parsed manifest OK"

# Inline system prompt
SYSTEM_FILE_REF="$(jq -r '.system.file // empty' <<< "$MANIFEST_JSON")"
if [[ -n "$SYSTEM_FILE_REF" ]]; then
  SYSTEM_FILE="${COOKBOOKS_DIR}/${SLUG}/${SYSTEM_FILE_REF}"
  [[ -f "$SYSTEM_FILE" ]] || SYSTEM_FILE="${AGENTS_DIR}/$(basename "${SYSTEM_FILE_REF}")"
  [[ -f "$SYSTEM_FILE" ]] || die "System prompt file not found: ${SYSTEM_FILE}"
  SYSTEM_TEXT="$(cat "$SYSTEM_FILE")"
  APPEND_TEXT="$(jq -r '.system.append // empty' <<< "$MANIFEST_JSON")"
  if [[ -n "$APPEND_TEXT" ]]; then
    SYSTEM_TEXT="${SYSTEM_TEXT}

${APPEND_TEXT}"
  fi
  MANIFEST_JSON="$(jq --arg t "$SYSTEM_TEXT" '.system = {text: $t}' <<< "$MANIFEST_JSON")"
  info "Inlined system prompt from: ${SYSTEM_FILE}"
fi

# Upload skills
declare -A SKILL_ID_MAP

upload_skill() {
  local skill_slug="$1"
  local skill_path="${SKILLS_DIR}/${skill_slug}/SKILL.md"
  [[ -f "$skill_path" ]] || { info "WARN: skill file not found: ${skill_path}"; return; }

  if [[ "$MODE" == "--dry-run" ]]; then
    info "  [dry-run] Would upload skill: ${skill_slug}"
    SKILL_ID_MAP["$skill_slug"]="skill_dry_run_${skill_slug}"
    return
  fi

  info "  Uploading skill: ${skill_slug}"
  local payload response
  payload="$(jq -n --arg name "$skill_slug" --rawfile content "$skill_path" '{name: $name, content: $content}')"
  response="$(curl -sf \
    -H "x-api-key: ${ANTHROPIC_API_KEY}" \
    -H "anthropic-version: ${API_VERSION}" \
    -H "anthropic-beta: ${BETA_HEADER}" \
    -H "Content-Type: application/json" \
    -X POST "${API_BASE}/v1/skills" \
    -d "$payload")"
  local skill_id
  skill_id="$(jq -r '.id' <<< "$response")"
  SKILL_ID_MAP["$skill_slug"]="$skill_id"
  info "  Skill uploaded: ${skill_id}"
}

while IFS= read -r skill_slug; do
  [[ -n "$skill_slug" ]] && upload_skill "$skill_slug"
done < <(jq -r '.skills[]?.from_skill // empty' <<< "$MANIFEST_JSON")

# Create subagents
declare -A SUBAGENT_ID_MAP

create_agent_from_manifest() {
  local manifest_path="$1"
  local agent_json agent_name
  agent_json="$(read_manifest "$manifest_path" | jq .)"
  agent_name="$(jq -r '.name' <<< "$agent_json")"

  if [[ "$MODE" == "--dry-run" ]]; then
    info "  [dry-run] Would create subagent: ${agent_name}"
    SUBAGENT_ID_MAP["$agent_name"]="agent_dry_run_${agent_name}"
    return
  fi

  info "  Creating subagent: ${agent_name}"
  local response
  response="$(curl -sf \
    -H "x-api-key: ${ANTHROPIC_API_KEY}" \
    -H "anthropic-version: ${API_VERSION}" \
    -H "anthropic-beta: ${BETA_HEADER}" \
    -H "Content-Type: application/json" \
    -X POST "${API_BASE}/v1/agents" \
    -d "$agent_json")"
  local agent_id
  agent_id="$(jq -r '.id' <<< "$response")"
  SUBAGENT_ID_MAP["$agent_name"]="$agent_id"
  info "  Subagent created: ${agent_id} (${agent_name})"
}

while IFS= read -r sub_manifest_ref; do
  [[ -z "$sub_manifest_ref" ]] && continue
  SUB_PATH="${COOKBOOKS_DIR}/${SLUG}/${sub_manifest_ref}"
  [[ -f "$SUB_PATH" ]] || die "Subagent manifest not found: ${SUB_PATH}"
  create_agent_from_manifest "$SUB_PATH"
done < <(jq -r '.callable_agents[]?.manifest // empty' <<< "$MANIFEST_JSON")

# Build orchestrator payload with subagent IDs
FINAL_PAYLOAD="$MANIFEST_JSON"
for agent_name in "${!SUBAGENT_ID_MAP[@]}"; do
  agent_id="${SUBAGENT_ID_MAP[$agent_name]}"
  FINAL_PAYLOAD="$(jq \
    --arg name "$agent_name" \
    --arg id   "$agent_id" \
    '(.callable_agents[]? | select(.name == $name)) |= . + {agent_id: $id}' \
    <<< "$FINAL_PAYLOAD")"
done

if [[ "$MODE" == "--dry-run" ]]; then
  info "[dry-run] Final orchestrator payload:"
  jq . <<< "$FINAL_PAYLOAD"
  info "[dry-run] No API calls made. Re-run with --apply to deploy."
  exit 0
fi

info "Creating orchestrator agent: ${SLUG}"
RESPONSE="$(curl -sf \
  -H "x-api-key: ${ANTHROPIC_API_KEY}" \
  -H "anthropic-version: ${API_VERSION}" \
  -H "anthropic-beta: ${BETA_HEADER}" \
  -H "Content-Type: application/json" \
  -X POST "${API_BASE}/v1/agents" \
  -d "$FINAL_PAYLOAD")"

AGENT_ID="$(jq -r '.id' <<< "$RESPONSE")"
info "Deployed: ${AGENT_ID}"
info "Console:  https://console.anthropic.com/agents/${AGENT_ID}"
