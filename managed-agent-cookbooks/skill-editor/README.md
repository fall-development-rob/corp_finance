# skill-editor cookbook

> **Tier:** `Free` — no vendor API required (reads local/S3 reasoning bank only)

Phase 41 closed learning loop: analyses the reasoning bank for recurring dispatch
outliers and drafts skill prose update proposals for human review.

## What it does

Runs three subagents in sequence:

1. **outlier-detector** — queries the reasoning bank for four cluster patterns:
   novel dispatches (cosine distance > 0.7), validation failures, tool-thrashing
   (tool_uses > p95), and delegation-mismatches. Returns a structured OutlierReport.

2. **pattern-analyst** — reads each cluster and the current content of the
   affected skill. Identifies what prose is missing that would have prevented the
   failure pattern. Returns a structured GapAnalysis.

3. **proposal-writer** — writes two files per actionable cluster to
   `docs/proposed-skill-updates/`: a unified diff (`*.diff`) and a JSON metadata
   sidecar (`*.json`). Never mutates skill files directly.

Every proposal requires ≥3 motivating audit_ids. Singleton outliers are dropped
with `recommended_action: no-action`.

## Subagents

| Role | Model | Toolset |
|------|-------|---------|
| `skill-editor-outlier-detector` | Sonnet | read (bank queries only) |
| `skill-editor-pattern-analyst` | Sonnet | read (skill files) |
| `skill-editor-proposal-writer` | Sonnet | read + write (`docs/proposed-skill-updates/` only) |

## Invocation

```bash
# Manual run — analyse last 7 days
cfa-harness cookbook run skill-editor \
  --prompt "analyse last 7 days of dispatches for skill-prose gaps" \
  --cookbooks-root ./managed-agent-cookbooks \
  --audit-dir ./audit \
  --output ./skill-editor-run.json

# Manual run — focus on validation failures only
cfa-harness cookbook run skill-editor \
  --prompt "analyse last 14 days, focus on validation-failure clusters" \
  --cookbooks-root ./managed-agent-cookbooks \
  --audit-dir ./audit

# CI workflow_dispatch trigger (GitHub UI)
gh workflow run skill-editor-cron.yml
```

Weekly cron: `.github/workflows/skill-editor-cron.yml` (Monday 06:00 UTC).
Opens a PR automatically when proposals are generated.

## Applying a proposal

```bash
# Review the diff
cat docs/proposed-skill-updates/2026-05-12-060031-workflow-er-initiating-coverage.diff

# Apply after review (never auto-applied by the cookbook)
git apply docs/proposed-skill-updates/2026-05-12-060031-workflow-er-initiating-coverage.diff

# Archive applied proposal
./scripts/archive-skill-proposal.sh 2026-05-12-060031-workflow-er-initiating-coverage
```

## Required environment variables

| Variable | Description |
|----------|-------------|
| `ANTHROPIC_API_KEY` | Anthropic API key |
| `CFA_CORE_MCP_URL` | URL of the cfa-core MCP server |
| `CFA_BANK_BACKEND` | `local` (default) or `s3` |
| `CFA_BANK_S3_BUCKET` | S3 bucket name (only when `CFA_BANK_BACKEND=s3`) |

## Security

- outlier-detector and pattern-analyst are fully read-only
- proposal-writer has write access scoped to `docs/proposed-skill-updates/` only
- PathGuard defensive layer rejects any write path outside the permitted prefix
- Anti-injection wording in every subagent system prompt
- `auto_merge_eligible: true` is informational only — no patch is ever auto-applied
