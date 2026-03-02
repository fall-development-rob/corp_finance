# ADR-012: Gap Analysis Remediation

## Status: Accepted

## Context

A systematic gap analysis of the CFA agent codebase identified 6 priority items spanning security, dependency hygiene, skill routing completeness, developer experience, and test coverage:

| # | Gap | Severity | Category |
|---|-----|----------|----------|
| 1 | Command injection vulnerability in `pipeline-fmp.ts` using `exec()` with user input | Critical | Security |
| 2 | 6 high-severity npm vulnerabilities via transitive `tar` dependency | High | Dependencies |
| 3 | `AGENT_SKILLS` missing data/geopolitical/vendor skill wiring | Medium | Routing |
| 4 | 3 missing domain slash commands (credit-analysis, bond-analysis, derivatives-valuation) | Medium | Developer UX |
| 5 | No MCP tool smoke tests | Medium | Test coverage |
| 6 | No ESLint/Prettier configuration | Low | Code quality |

Item 1 is an OWASP A03:2021 (Injection) risk -- `child_process.exec()` passes the entire command string through a shell, enabling arbitrary command execution if any argument originates from user input (e.g., ticker symbols, company names). Items 2-6 are hygiene and completeness gaps that reduce confidence in the platform's production readiness.

## Decision

Address all 6 items in a single remediation pass:

### 1. Command Injection Fix (SEC-001)

Replace `child_process.exec()` with `child_process.execFile()` in `pipeline-fmp.ts` and any other agent code that invokes external processes. `execFile()` passes arguments as an array and does **not** spawn a shell, preventing injection via metacharacters (`; && | $()` etc.).

Before:
```typescript
exec(`npx fmp-tool ${ticker} --format json`, callback);
```

After:
```typescript
execFile('npx', ['fmp-tool', ticker, '--format', 'json'], callback);
```

### 2. Dependency Vulnerability Fix (DEP-001)

Add an `overrides` block to root `package.json` pinning the transitive `tar` dependency to `>=7.5.8` (the patched version). Run `npm audit fix` and verify zero high/critical findings.

```json
{
  "overrides": {
    "tar": ">=7.5.8"
  }
}
```

### 3. AGENT_SKILLS Wiring (ROUTE-001)

Wire all 15 data, geopolitical, and vendor skills into `AGENT_SKILLS` in `packages/agents/src/pipeline.ts` based on domain relevance:

| Agent | Added Skills |
|-------|-------------|
| `cfa-chief-analyst` | +data-fred, +data-edgar, +data-wb, +geopolitical-conflict, +geopolitical-environment, +geopolitical-trade, +geopolitical-alternative, +vendor-sp-global, +vendor-moodys, +vendor-factset |
| `cfa-equity-analyst` | +data-edgar, +data-yf, +geopolitical-conflict, +geopolitical-trade, +vendor-factset, +vendor-sp-global, +vendor-pitchbook |
| `cfa-credit-analyst` | +data-edgar, +data-fred, +geopolitical-conflict, +vendor-moodys, +vendor-sp-global |
| `cfa-fixed-income-analyst` | +data-fred, +data-edgar, +vendor-moodys, +vendor-factset |
| `cfa-derivatives-analyst` | +data-yf, +data-figi, +vendor-factset |
| `cfa-macro-analyst` | +data-fred, +data-wb, +geopolitical-conflict, +geopolitical-trade, +geopolitical-alternative |
| `cfa-esg-analyst` | +data-wb, +geopolitical-environment, +vendor-morningstar, +vendor-sp-global |
| `cfa-private-markets-analyst` | +data-edgar, +vendor-pitchbook, +vendor-sp-global |
| `cfa-quant-risk-analyst` | +data-fred, +data-yf, +geopolitical-alternative, +vendor-factset, +vendor-morningstar |

This brings each agent from ~10 skills to ~15 skills on average, ensuring comprehensive data retrieval and geopolitical awareness.

### 4. Missing Slash Commands (CMD-001)

Add 3 new slash commands to `.claude/commands/cfa/`:

- `credit-analysis.md` -- Routed to `cfa-credit-analyst` for credit metrics, Altman Z-score, covenant analysis
- `bond-analysis.md` -- Routed to `cfa-fixed-income-analyst` for yield, duration, spread, and relative value
- `derivatives-valuation.md` -- Routed to `cfa-derivatives-analyst` for options pricing, Greeks, strategies

This brings the total from 16 (Phase 20) + 4 (geopolitical, Phase 21) = 20 to 23 CFA slash commands.

### 5. MCP Smoke Tests (TEST-001)

Add `packages/agents/tests/contracts/mcp-smoke.test.ts` with contract tests validating:

- Total tool count matches expected registration (195 core + 5 workflow = 200 minimum)
- All tool names follow `snake_case` convention
- No duplicate tool names across all registrations
- Every tool has a Zod input schema defined

### 6. ESLint and Prettier (LINT-001)

Add root-level configuration files:

- `eslint.config.mjs` -- Flat config format with `typescript-eslint` parser and recommended rules
- `.prettierrc` -- Standard formatting (2-space indent, single quotes, trailing commas, 100 char print width)

Integrate into the Turbo pipeline:

```json
{
  "lint": {
    "dependsOn": ["^build"],
    "inputs": ["src/**/*.ts"]
  },
  "format:check": {
    "inputs": ["src/**/*.ts"]
  }
}
```

## Consequences

### Positive

- Eliminates OWASP A03:2021 injection risk -- all external process invocations use safe argument arrays
- Zero high/critical npm audit vulnerabilities
- Full skill routing coverage -- all 9 agents have access to relevant data, geopolitical, and vendor skills
- 23 CFA slash commands covering all major analyst workflows
- MCP smoke tests catch tool registration regressions in CI
- ESLint and Prettier enforce consistent code quality across all TypeScript packages
- Turbo pipeline integration means linting and formatting run in parallel with builds

### Negative

- ESLint flat config may surface existing warnings across the codebase that require gradual fixing (recommend `--max-warnings` threshold)
- `tar` override in `package.json` creates a maintenance burden -- must be reviewed and removed once the upstream dependency chain ships a clean version
- Adding ~5 skills per agent increases system prompt size by ~50-100 lines per agent, increasing context window pressure

## Options Considered

### Option 1: Incremental remediation across multiple PRs (Rejected)

- **Pros**: Smaller PRs, easier review
- **Cons**: Security fix (item 1) should not wait; items are independent and non-conflicting; single PR is cleaner for changelog

### Option 2: Use `spawn()` instead of `execFile()` for the injection fix (Considered)

- **Pros**: More control over stdio streams
- **Cons**: `execFile()` is the direct drop-in replacement for `exec()` with the same callback API; `spawn()` requires stream handling refactoring

### Option 3: Biome instead of ESLint + Prettier (Rejected)

- **Pros**: Single tool, faster execution
- **Cons**: `typescript-eslint` has deeper TypeScript integration; team familiarity with ESLint ecosystem; Biome rule coverage still catching up for TypeScript-specific patterns

## Related Decisions

- ADR-008: Financial Services Workflow Integration (workflow skills and slash commands)
- ADR-010: Multi-Source Financial Data Integration (data skill definitions)
- ADR-011: Geopolitical and Alternative Data Integration (geopolitical skill definitions)

## References

- [OWASP A03:2021 - Injection](https://owasp.org/Top10/A03_2021-Injection/)
- [Node.js child_process.execFile](https://nodejs.org/api/child_process.html#child_processexecfilefile-args-options-callback)
- [npm overrides documentation](https://docs.npmjs.com/cli/v10/configuring-npm/package-json#overrides)
- [typescript-eslint flat config](https://typescript-eslint.io/getting-started/)
- [Prettier configuration](https://prettier.io/docs/en/configuration.html)
