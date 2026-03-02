# Product Requirements Document: Gap Analysis Remediation

**Product**: Autonomous CFA Analyst Platform
**Package**: @robotixai/corp-finance-mcp
**Version**: 1.0
**Date**: 2026-03-02
**Author**: RobotixAI Engineering

---

## 1. Overview

A systematic gap analysis identified 6 priority items affecting security posture, dependency hygiene, skill routing completeness, developer experience, test coverage, and code quality. This PRD specifies the acceptance criteria for each remediation item.

---

## 2. Requirements

### SEC-001: No Shell Injection

**Priority**: P0 (Critical)
**Description**: Agent code must not use `child_process.exec()` with template strings containing user input. All external process invocations must use `execFile()` or `spawn()` with argument arrays.
**Acceptance Criteria**:
- Zero occurrences of `exec()` in `packages/agents/src/**/*.ts`
- `execFile()` used with arguments passed as arrays, not interpolated strings
- Existing FMP pipeline functionality preserved (same outputs, same error handling)

### DEP-001: Zero High-Severity npm Vulnerabilities

**Priority**: P0 (High)
**Description**: `npm audit` must report zero high or critical severity vulnerabilities.
**Acceptance Criteria**:
- `npm audit --audit-level=high` exits with code 0
- `tar` override pinned to `>=7.5.8` in root `package.json`
- No new dependencies introduced

### ROUTE-001: Full Skill Routing Coverage

**Priority**: P1 (Medium)
**Description**: All 15 data, geopolitical, and vendor skills must be wired to appropriate agents in `AGENT_SKILLS`.
**Acceptance Criteria**:
- At least 5 of 9 agents reference at least one `data-*` skill
- At least 4 of 9 agents reference at least one `geopolitical-*` skill
- At least 5 of 9 agents reference at least one `vendor-*` skill
- Each skill is assigned to at least one agent

### CMD-001: Slash Command Coverage

**Priority**: P1 (Medium)
**Description**: At least 23 CFA slash commands available in `.claude/commands/cfa/`.
**Acceptance Criteria**:
- `credit-analysis.md`, `bond-analysis.md`, `derivatives-valuation.md` exist
- Each command file specifies the target agent and required MCP tools
- Total count of `.claude/commands/cfa/*.md` files >= 23

### TEST-001: MCP Smoke Tests

**Priority**: P1 (Medium)
**Description**: Contract tests validate MCP tool registration invariants.
**Acceptance Criteria**:
- `mcp-smoke.test.ts` exists in `packages/agents/tests/contracts/`
- Tests validate: tool count >= 200, snake_case naming, uniqueness, schema presence
- Tests pass in CI (`npm test` exits 0)

### LINT-001: ESLint and Prettier Configuration

**Priority**: P2 (Low)
**Description**: ESLint and Prettier configured at root level with Turbo pipeline integration.
**Acceptance Criteria**:
- `eslint.config.mjs` exists at repository root with `typescript-eslint` parser
- `.prettierrc` exists at repository root
- `npm run lint` executes ESLint across TypeScript packages
- `npm run format:check` executes Prettier in check mode

---

## 3. Out of Scope

- Fixing existing ESLint warnings surfaced by the new configuration (tracked separately)
- Migrating from ESLint + Prettier to Biome
- Adding new MCP tools or Rust computation modules
- Modifying any Rust source code

---

## 4. Success Metrics

| Metric | Before | After |
|--------|--------|-------|
| `exec()` occurrences in agent code | >= 1 | 0 |
| High/critical npm audit findings | 6 | 0 |
| Skills wired in AGENT_SKILLS | ~10/agent | ~15/agent |
| CFA slash commands | 20 | 23 |
| MCP smoke test assertions | 0 | 4+ |
| Lint/format config files | 0 | 2 |
