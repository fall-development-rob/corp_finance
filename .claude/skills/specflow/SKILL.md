---
name: specflow
description: Spec-driven development with executable contracts for CFA agent drift detection
version: 1.0.0
author: Hulupeep
source: https://github.com/Hulupeep/Specflow
---

# Specflow Skill

Specs that enforce themselves. Turn requirements into contracts that break the build when violated.

## Core Loop

```
Spec --> Contract --> Test --> Code --> Verify
```

1. Write requirements with IDs (ARCH-001 MUST, MCP-001, DRIFT-001)
2. Generate contract YAML with forbidden/required patterns
3. Generate tests that scan source code for violations
4. Implement code that satisfies contracts
5. Violations = build fails = PR blocked

## When Activated

When this skill is active, Claude Code MUST:

1. **Before modifying any file**: Check if it falls under a contract scope in `docs/contracts/*.yml`. If yes, read the contract and respect all `non_negotiable` rules.
2. **Before closing any work**: Run contract tests (`cd packages/agents && npx vitest run tests/contracts/`) and verify. Work is not done if tests fail.
3. **When creating new features**: Generate the spec (with REQ IDs), contract YAML, and test files BEFORE implementing code.
4. **When a contract violation is reported**: Read the contract rule, understand why it exists, fix the code to comply. Never work around the test.
5. **Never modify `non_negotiable` rules** unless the user explicitly says `override_contract: <contract_id>`.

## CFA Project Contracts

### Active Contracts

| Contract | Rules | Scope |
|----------|-------|-------|
| `feature_architecture.yml` | ARCH-001..006 | No direct math in agents, no secrets, 500-line limit, no DB writes, rust_decimal only |
| `feature_mcp_tools.yml` | MCP-001..003 | Zod schemas required, inputSchema on tools, no empty catches |
| `feature_agent_routing.yml` | ROUTE-001..003 | System prompts required, no hardcoded routing, all domains covered |

### Drift Detection

| Check | Catches |
|-------|---------|
| DRIFT-001 | Stale tool/module counts in docs (must be 195/67) |
| DRIFT-002 | Missing specialist agent files or prompts |
| DRIFT-003 | Chief analyst not referencing all specialists |
| DRIFT-004 | ADR numbering gaps |
| DRIFT-005 | Rust feature count drift vs docs |
| DRIFT-006 | Missing MCP schema files |

### Running Tests

```bash
# Contract + drift tests (fast gate, no build needed)
cd packages/agents && npx vitest run tests/contracts/

# Specific contract
cd packages/agents && npx vitest run tests/contracts/architecture.test.ts
cd packages/agents && npx vitest run tests/contracts/drift.test.ts
```

### Override Protocol

Only humans can override non-negotiable rules. User must say:
```
override_contract: <contract_id>
```

## Contract YAML Structure

```yaml
contract_meta:
  id: feature_name
  version: 1
  covers_reqs: [ARCH-001, ARCH-002]

rules:
  non_negotiable:
    - id: ARCH-001
      title: "Rule description"
      scope: ["src/**/*.ts"]
      behavior:
        forbidden_patterns:
          - pattern: /regex_pattern/
            message: "Why this is forbidden"
        required_patterns:
          - pattern: /regex_pattern/
            message: "Why this is required"
```

## Invocation

```
/specflow              Full loop: verify all contracts, report violations
/specflow verify       Contract validation only
/specflow status       Show contract coverage dashboard
```
