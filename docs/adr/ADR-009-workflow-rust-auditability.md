# ADR-009: Workflow Definitions in Rust for Auditability

## Status
Accepted

## Context
Phase 20 (ADR-008) added 6 workflow skills as markdown SKILL.md files for agent system prompt injection. While effective for guiding agent behaviour, markdown-only workflows lack:
- Fixed auditability — no deterministic hash of workflow execution
- Programmatic validation — cannot validate inputs before execution
- Quality enforcement — quality gates are advisory, not code-enforced
- CLI/MCP access — workflows only available through agent prompts

## Decision
Implement all workflow definitions as Rust compile-time constants in `corp-finance-core::workflows`. Expose via:
1. **5 MCP tools**: `workflow_list`, `workflow_describe`, `workflow_validate`, `workflow_quality_check`, `workflow_audit`
2. **5 CLI commands**: `cfa workflow-list`, `cfa workflow-describe`, `cfa workflow-validate`, `cfa workflow-quality-check`, `cfa workflow-audit`
3. **NAPI bindings**: JSON string boundary pattern (existing)

Workflow definitions are `static` constants — no runtime allocation, fully auditable at compile time.

## Consequences
### Positive
- Every workflow is a typed, versioned Rust struct — changes require code review
- Deterministic audit hashes via djb2 fingerprinting of execution state
- Input validation catches missing required fields before workflow starts
- Quality gates enforced in code, not just prompt guidance
- CLI enables automation and scripting of workflow operations
- MCP tools enable agents to query workflow specs programmatically

### Negative
- Dual maintenance: Rust definitions must stay in sync with markdown skills
- 68th feature flag in corp-finance-core (was 67)
- 200 MCP tools (was 195) — DRIFT-001 count needs updating

### Mitigations
- Contract WORKFLOW-008 enforces Rust workflow count matches markdown workflow count
- CI test validates all workflow IDs in Rust exist as referenced in SKILL.md files
