# Domain-Driven Design Supplement: Gap Analysis Remediation

## Overview

This supplement documents how the 6 remediation items from ADR-012 affect the bounded contexts defined in the core DDD document.

---

## Affected Bounded Contexts

### 1. Specialist Analysts (Agent Routing)

The `AGENT_SKILLS` expansion is the most significant domain-level change. Each of the 9 specialist agents gains access to data retrieval, geopolitical awareness, and vendor intelligence skills:

- **Before**: ~10 skills per agent (corp-finance computation + workflow skills)
- **After**: ~15 skills per agent (+data-*, +geopolitical-*, +vendor-*)

This does not change the aggregate structure or domain events within the Specialist Analysts context. It expands the **capability set** of each agent entity, enabling richer analysis without requiring inter-agent delegation for data retrieval tasks.

The 3 new slash commands (`credit-analysis`, `bond-analysis`, `derivatives-valuation`) add new entry points to the Analysis Orchestration context, each routed to the appropriate specialist via the existing `SemanticRouter`.

### 2. Hosted MCP Gateway

The new MCP smoke tests (`mcp-smoke.test.ts`) enforce registration invariants on the gateway:

| Invariant | Rule |
|-----------|------|
| Tool count | >= 200 registered tools |
| Naming | All tool names match `snake_case` |
| Uniqueness | No duplicate tool names |
| Schema | Every tool has a Zod input schema |

These tests operate as **anti-corruption layer contracts** -- they guard the boundary between the gateway and consumers (agents, CLI) by ensuring the tool registry remains consistent after any change.

### 3. Cross-Cutting Infrastructure

Two items affect infrastructure that spans all bounded contexts:

- **Security (SEC-001)**: The `exec()` to `execFile()` migration is a cross-cutting security constraint enforced at the process boundary. It does not affect domain logic but hardens the runtime environment.
- **Code quality (LINT-001)**: ESLint and Prettier operate outside the domain model entirely. They enforce syntactic consistency across all TypeScript packages via the Turbo build pipeline.

---

## Domain Model Impact Summary

| Bounded Context | Change Type | Impact |
|----------------|-------------|--------|
| Specialist Analysts | Entity capability expansion | Medium -- broader skill sets, same aggregate boundaries |
| Analysis Orchestration | New entry points (3 commands) | Low -- uses existing SemanticRouter |
| Hosted MCP Gateway | Smoke test contracts | Low -- validation only, no schema changes |
| Cross-cutting | Security + quality infrastructure | None on domain model |
