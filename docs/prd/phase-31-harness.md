# Product Requirements Document: Phase 31 — CFA Harness

**Status:** Planning
**Date:** 2026-05-09
**Owner:** Robert Fall
**Version:** 0.1

---

## 1. Overview

Phase 31 delivers the CFA Harness: a custom TypeScript dispatch loop that replaces the broken Claude Code subagent path with a direct `@anthropic-ai/sdk` Messages-API loop capable of reliably invoking MCP tools. It introduces six bounded contexts (Agent Runtime, MCP Transport, Provider Abstraction, Agent Registry, Audit Chain, Session Memory) documented in `docs/ddd/phase-31-harness.md`. The harness ships as a CLI (`cfa-harness run`) and a programmatic TypeScript API.

---

## 2. Owner / Stakeholders

| Role | Person / System | Interest |
|------|----------------|---------|
| Owner | Robert Fall | Delivery and architecture |
| Platform consumer | Regfin integration | Programmatic API (F8) |
| CI/CD pipeline | GitHub Actions | Acceptance test automation (F6, J2) |
| Institutional analyst | End user | Memo and research workflows (J1) |
| Pricing desk | End user | Warrant grid and option analytics |
| IC member | End user | Investment Committee memo consumption |

---

## 3. Problem Statement

The cfa-* subagent class in Claude Code's runtime returns `tool_uses: 0` on every non-trivial prompt. After Phase 29 (compute extraction) and Phase 30 (script-to-tool migration) the compute and tool surface is correct — 4 plugin MCP servers connected, 623 tools registered, 9 specialist agent definitions updated with plugin-namespaced tool references — but subagent dispatches still emit `tool_use` JSON as literal text rather than invoking the tools. A general-purpose subagent given the same Chaco Minerals IC memo prompt returns `tool_uses: 22` and produces the canonical memo with rust_decimal-precision warrant pricing. The gap is intrinsic to the cfa-* subagent class in Claude Code's runtime and is not fixable via allowlist or body rewrites (see memory note `feedback_cfa_agents_cannot_invoke_tools.md`).

The harness is the durable answer: it owns the dispatch loop end-to-end, calls `@anthropic-ai/sdk` directly, routes `tool_use` blocks to MCP servers via JSON-RPC, and injects `tool_result` blocks back. The existing MCP servers, compute library, and audit modules are reused without modification.

---

## 4. Users / Personas

### P1 — Institutional Analyst

Works within Claude Code as their editor. Runs `cfa-harness run` from the terminal to produce memos, comps tables, IC summaries, and pricing grids. Needs correct numerical outputs with full provenance. Not interested in harness internals; cares about memo quality and run time.

### P2 — IC Member

Receives the output memo (e.g., `chaco-memo.md`) and the companion audit log (`chaco-memo.audit.json`). Does not run the harness directly. Needs confidence that numerical results (warrant prices, country risk premiums, IRRs) are traceable to specific tool invocations with logged inputs.

### P3 — Pricing Desk Operator

Runs warrant-grid pricing scenarios at scale. May invoke the harness programmatically (F8) or via the CLI in a loop. Needs deterministic results over the same inputs (NFR1) and predictable latency (NFR2).

### P4 — Automation Pipeline (CI/CD)

Runs the canonical acceptance test (`tests/chaco-acceptance.test.ts`) on every pull request. Needs the harness to be invocable headlessly, with machine-readable exit codes and structured JSON output. Needs portability — no machine-specific paths in any committed file (NFR5).

---

## 5. Functional Requirements

### F1 — Chief-Analyst Dispatch

The harness dispatches a prompt to the chief-analyst agent via the `@anthropic-ai/sdk` Messages API, runs the full async generator loop, collects all `tool_use` blocks, routes them to MCP servers via JSON-RPC, injects `tool_result` blocks, and iterates until `stop_reason: end_turn` or the turn budget is exhausted.

**Maps to:** DDD Agent Runtime context — `AgentSession` aggregate, Dispatch Loop.

Acceptance: `tool_uses ≥ 21` on the Chaco acceptance prompt.

### F2 — Specialist Routing (Wave 2)

The chief-analyst decomposes a multi-domain prompt and delegates sub-goals to named specialists (derivatives, equity, credit, fixed-income, quant-risk, macro, private-markets, esg-regulatory) via nested `AgentSession` calls. Delegation depth is bounded at 1: a specialist may not sub-dispatch further. Each specialist operates against a filtered tool subset derived from its Tool Allowlist in the Agent Registry.

**Maps to:** DDD Agent Runtime (sub-dispatch), DDD Agent Registry (Tool Allowlist, DelegationRule).

Acceptance: `tests/specialist-routing.test.ts` — chief routes warrant grid to derivatives specialist; country risk sub-goal to macro specialist.

### F3 — MCP Tool Invocation

Every `tool_use` block in a provider response is translated to a JSON-RPC `tools/call` request on the correct MCP server connection (cfa-core, cfa-data, fmp-market-data, or vendor). The result content is returned as a `tool_result` block in the next turn. Failed tool calls are recorded in the Audit Chain and do not crash the session; the error content is injected as a `tool_result` with `is_error: true`.

**Maps to:** DDD MCP Transport context — `McpServerConnection`, `ToolRouterTable`.

Acceptance: all 20 `option_pricer` tool calls and 1 `country_risk_premium` call return valid results on the Chaco prompt.

### F4 — Multi-Provider Support (Wave 3)

The harness supports Anthropic (Wave 1), OpenAI Chat Completions (Wave 3), Google Gemini Generate Content (Wave 3), and Amazon Bedrock Converse (Wave 4) as provider backends. The active provider is selected via `--provider` CLI flag or the `provider` programmatic API option. Tool schemas are stored in provider-neutral form and translated at the wire boundary by the provider adapter.

**Maps to:** DDD Provider Abstraction context — `ProviderAdapter`, ACL translation functions.

Acceptance: `tests/cross-provider-determinism.test.ts` — same Chaco prompt across Anthropic and OpenAI adapters produces the same set of tool call names (inputs may differ in formatting).

### F5 — Audit Chain

Every session, turn, tool invocation, and provider call is recorded in an append-only SHA-256-linked JSON Lines file written to `<output>.audit.json`. The chain is verifiable: re-walking the log and recomputing hashes confirms integrity. Each numerical result is traceable to a specific `tool_invocation_succeeded` entry with logged tool name, input hash, and output hash. The audit format mirrors `corp_finance_core::audit` from Phase 26.

**Maps to:** DDD Audit Chain context — `AuditLog`, `AuditEntry`, chain hash invariants.

Acceptance: `out/chaco-memo.audit.json` exists, is valid JSON Lines, passes chain verification, contains at least 21 `tool_invocation_succeeded` entries.

### F6 — Session Replay (Wave 4)

Completed sessions are persisted as gzip-compressed JSON archives in `.cfa-sessions/`. The CLI command `cfa-harness replay --session <session_id>` restores the Turn History and re-runs the dispatch loop from the last completed turn. Replay is useful for debugging mid-session failures and for CI determinism checks.

**Maps to:** DDD Session Memory context — `SessionArchive`, `SessionMemoryRepository`.

Acceptance: a session interrupted at turn 15 can be restored and continued to completion with no tool re-invocations for turns 1–14.

### F7 — CLI Surface

The harness exposes:

```
cfa-harness run   --agent <agent_id>
                  --prompt <file_or_string>
                  [--output <path>]
                  [--provider <provider_id>]
                  [--max-turns <n>]
                  [--audit]

cfa-harness replay --session <session_id>

cfa-harness list-sessions

cfa-harness verify-audit --log <path>
```

All flags are machine-readable. Exit code 0 on success, non-zero on failure. `--output` defaults to stdout for the memo and `<output_basename>.audit.json` for the audit log.

**Maps to:** CLI entry point in `packages/cli/run.ts`, consulting Agent Registry for `--agent` validation.

Acceptance: `cfa-harness run --agent chief-analyst --prompt docs/examples/chaco-minerals-deal.md --output out/chaco-memo.md` completes with exit 0 and produces both output files.

### F8 — Programmatic API

The harness exposes a TypeScript module API for embedding in applications (e.g., the regfin platform):

```typescript
import { createHarness } from "@cfa/harness";

const harness = createHarness({ provider: "anthropic" });
const result = await harness.run({
  agent: "chief-analyst",
  prompt: "...",
  auditOutput: "./memo.audit.json",
});
// result.text, result.toolUseCount, result.sessionId
```

The API is promise-based, fully typed, and does not write to stdout unless `verbose: true` is set.

**Maps to:** DDD Agent Runtime (AgentSession), DDD Agent Registry (resolve).

Acceptance: a TypeScript unit test instantiates the harness, calls `run()`, and asserts `result.toolUseCount >= 21`.

---

## 6. Non-Functional Requirements

### NFR1 — Determinism Over Same Input

Given the same agent spec, the same initial prompt, the same provider model, and the same MCP server versions, the harness produces the same sequence of tool call names on every run. Numerical outputs produced by `corp_finance_core` tools are deterministic by construction (rust_decimal, no f64 in financial math). Provider sampling temperature defaults to 0.0 for all production runs to maximise reproducibility.

**Maps to:** DDD Provider Abstraction (temperature), DDD Audit Chain (chain hash verifiability).

### NFR2 — Latency Budget

- Single chief-analyst session (Wave 1): completes in under 120 s on the Chaco acceptance prompt (21 tool calls, one specialist delegation in Wave 2).
- Individual MCP tool call round-trip: under 5 s for cfa-core stdio transport.
- Provider API call (excluding tool calls): under 30 s per turn.

Latency is measured end-to-end from `cfa-harness run` invocation to first byte of output.

### NFR3 — Cost Budget

- Per-session token cap: 200,000 input tokens and 10,000 output tokens for a chief-analyst session (configurable via `--max-input-tokens` and `--max-output-tokens`).
- Specialist sub-dispatch (Wave 2): each specialist session is capped at 50,000 input / 4,000 output tokens.
- Exceeding the cap terminates the session with `status: budget_exhausted` and a non-zero exit code; partial output is written if `--partial-output` is set.

Open question: Should cost budget enforcement track cumulative cost across a sub-dispatch tree (parent + all specialists) or per session independently? See §9.

### NFR4 — Observability

Every tool invocation records tool name, input hash, output hash, and duration in the audit log. Provider calls record input and output token counts. The harness emits structured JSON to stderr when `--verbose` is set (one line per event). No plaintext `console.log` in production paths.

**Maps to:** DDD Audit Chain (AuditLog, AuditEntry).

### NFR5 — Portability

No committed file may contain machine-specific absolute paths, user-specific home directories, or environment-specific configuration values. All paths in source files use relative references or environment variables. MCP server endpoints in test fixtures use `process.env` lookups. CI passes on a clean checkout on a different machine from the developer's workstation.

This requirement directly addresses the finding that `feedback_mcp_registration_check.md` paths baked into agent configs caused CI failures in Phase 30.

### NFR6 — License Compliance

`@anthropic-ai/sdk` is used under its published license. The harness does not reverse-engineer Claude Code's internal subagent dispatch mechanism; it uses only the published SDK. The `@modelcontextprotocol/sdk` package is used for the MCP client transport, consistent with its Apache 2.0 license. No vendor-specific SDKs (OpenAI, Google, AWS) are bundled in Wave 1; they are optional peer dependencies added in Wave 3/4.

---

## 7. User Journeys

### J1 — Analyst Runs Chaco IC Memo from CLI

**Actor:** P1 (Institutional Analyst)

**Preconditions:** Harness is installed (`npm install`). `ANTHROPIC_API_KEY` is set. The four MCP plugin servers are available on the local machine. The Chaco Minerals deal brief exists at `docs/examples/chaco-minerals-deal.md`.

**Steps:**

1. Analyst opens a terminal in the `cfa_agent` workspace.
2. Runs: `cfa-harness run --agent chief-analyst --prompt docs/examples/chaco-minerals-deal.md --output out/chaco-memo.md`
3. Harness resolves `chief-analyst` from the Agent Registry (F7, F1).
4. Harness establishes stdio connections to cfa-core, cfa-data, fmp-market-data, and vendor MCP servers (F3).
5. Harness sends the initial prompt to the Anthropic Messages API (F1).
6. Provider responds with `tool_use` blocks: 20 × `option_pricer`, 1 × `country_risk_premium`.
7. Harness routes each block to the cfa-core MCP server via JSON-RPC (F3); results injected as `tool_result` blocks.
8. Provider responds with `stop_reason: end_turn` and the full IC memo text.
9. Memo is written to `out/chaco-memo.md`; audit log to `out/chaco-memo.audit.json` (F5).
10. Harness exits with code 0.

**Outcome:** Analyst opens `out/chaco-memo.md` and sees all 9 IC memo sections with warrant prices at rust_decimal precision. Passes to IC member (P2) alongside the audit log.

**Error path:** If the Anthropic API returns `429`, the harness applies exponential back-off (up to 60 s) and retries. The delay is recorded in the audit log. If back-off is exhausted, the harness exits with code 1 and a human-readable error on stderr.

---

### J2 — CI Pipeline Runs the Canonical Acceptance Test on Every PR

**Actor:** P4 (CI/CD Pipeline)

**Preconditions:** GitHub Actions runner has `ANTHROPIC_API_KEY` as a repository secret. The four MCP plugin servers are started as job services (or the harness uses a stubbed transport in the test environment). `vitest` is available.

**Steps:**

1. PR is opened or pushed to `phase-31-*` branch.
2. GitHub Actions triggers the `harness-ci.yml` workflow.
3. Workflow runs `npm run build` to compile TypeScript.
4. Workflow runs `npx vitest tests/chaco-acceptance.test.ts`.
5. Test instantiates the harness programmatically (F8): `harness.run({ agent: "chief-analyst", prompt: chacoPrompt })`.
6. Test asserts: `result.toolUseCount >= 21` (F1, F3).
7. Test asserts: warrant price at C$0.10/80% equals `C$0.0208` ± 0.0001 (rust_decimal precision).
8. Test asserts: `result.text` contains all 9 IC memo section headings.
9. Test asserts: audit log at `auditOutput` passes `harness.verifyAudit()` (F5).
10. If all assertions pass, workflow reports green; PR is eligible to merge.

**Outcome:** Every PR that touches harness code is automatically gated on the full Chaco acceptance criteria. `tool_uses: 0` regressions are caught within minutes.

**NFR5 relevance:** No absolute paths appear in `chaco-acceptance.test.ts`; the Chaco prompt is loaded from `docs/examples/chaco-minerals-deal.md` via a relative import. MCP server commands reference `process.env.MCP_STDIO_CMD_CFA_CORE` so the test passes on any CI runner.

---

### J3 — Programmatic Dispatch from a TypeScript Caller (Regfin Platform)

**Actor:** P3 (Pricing Desk Operator) via the Regfin platform.

**Preconditions:** Regfin has `@cfa/harness` installed as a dependency. `ANTHROPIC_API_KEY` is available in the platform's secret store (injected as an environment variable at runtime, never committed). Regfin constructs a structured warrant-grid prompt from its own deal data model.

**Steps:**

1. Regfin's deal service builds a prompt string from a `DealSpec` object: strike prices, expiry dates, underlying volatilities, country code.
2. Calls the harness API (F8):

```typescript
const harness = createHarness({
  provider: "anthropic",
  agentRegistryPath: "./config/agents",
  sessionStore: "./data/cfa-sessions",
});
const result = await harness.run({
  agent: "derivatives",          // specialist directly, bypassing chief
  prompt: warrantsPrompt,
  auditOutput: `./audits/${dealId}.audit.json`,
  maxTurns: 30,
});
```

3. Harness resolves the `derivatives` specialist spec from the Registry (F1, Agent Registry).
4. Harness connects to cfa-core MCP server and calls `option_pricer` 20 times (F3).
5. Result is returned as a typed object: `{ text, toolUseCount, sessionId, resultHash }`.
6. Regfin extracts warrant prices from `result.text` using its own parser.
7. Regfin stores `result.sessionId` and the audit log path in its deal database for IC traceability.

**Outcome:** Regfin's pricing desk gets deterministic (NFR1) warrant prices with full audit provenance (F5). The harness session archive (F6) can be replayed if the pricing desk questions a result.

---

## 8. Acceptance Criteria

The harness is considered complete for Wave 1 when all of the following pass in CI:

| ID | Criterion | Test |
|----|-----------|------|
| AC-01 | `cfa-harness run --agent chief-analyst --prompt docs/examples/chaco-minerals-deal.md` exits with code 0 | `tests/chaco-acceptance.test.ts` |
| AC-02 | `result.toolUseCount >= 21` (20 × `option_pricer` + 1 × `country_risk_premium`) | `tests/chaco-acceptance.test.ts` |
| AC-03 | Per-warrant price at C$0.10 strike / 80% volatility equals `C$0.0208` ± 0.0001 | `tests/chaco-acceptance.test.ts` |
| AC-04 | Output memo contains all 9 IC memo section headings from the standard template | `tests/chaco-acceptance.test.ts` |
| AC-05 | `out/chaco-memo.audit.json` is valid JSON Lines and passes `harness.verifyAudit()` | `tests/chaco-acceptance.test.ts` |
| AC-06 | Audit log contains at least 21 `tool_invocation_succeeded` entries | `tests/chaco-acceptance.test.ts` |
| AC-07 | `cfa-harness verify-audit --log out/chaco-memo.audit.json` exits with code 0 | CLI smoke test |
| AC-08 | No absolute paths appear in any committed source file or test fixture | `grep -r "/home/" src/ tests/` in CI |
| AC-09 | `npm run build` exits with code 0 on TypeScript strict mode | CI build step |
| AC-10 | `npx vitest --run` exits with code 0 (all unit + acceptance tests pass) | CI test step |

Wave 2 additional acceptance criterion:

| AC-11 | `tests/specialist-routing.test.ts` confirms chief routes warrant sub-goal to derivatives specialist and country risk sub-goal to macro specialist | Wave 2 CI |

Wave 4 additional acceptance criterion:

| AC-12 | A session interrupted at turn 15 can be replayed to completion with `cfa-harness replay` | `tests/session-replay.test.ts` |

---

## 9. Out of Scope

The following are explicitly deferred to Phase 32 or later:

- **Federation across machines** — `corp_finance_core::federation` covers the primitives; cross-machine dispatch is a Phase 32 concern.
- **Container sandboxing per agent** — agentbox-style per-agent OCI container isolation is deferred. Phase 31 runs all agents in the same Node.js process.
- **IDE replacement** — the harness exposes a CLI and a programmatic API; it does not replace Claude Code as the developer's editor.
- **Self-modifying agents / adaptive learning** — SONA-style loops are deferred; the harness is a static dispatch engine.
- **`did:nostr` identity mesh** — the agentbox identity primitives are not adopted in Phase 31.
- **Byzantine consensus / federation audit** — single-machine dispatch; no multi-node consensus required.

---

## 10. Open Questions

| ID | Question | Impact | Owner |
|----|----------|--------|-------|
| OQ-01 | Should the token cost budget (NFR3) track cumulative usage across the full sub-dispatch tree (parent + all specialist sessions) or enforce independently per session? | Affects budget enforcement logic in Wave 2 `delegate.ts`. Tree-level tracking is safer but requires a shared budget context across nested `AgentSession` instances. | Robert Fall |
| OQ-02 | What is the maximum permitted recursion depth for `delegate.ts` in Wave 2? The plan says depth 1 (specialists cannot sub-dispatch), but should the architecture allow depth 2 for future waves without a breaking change? | Affects `AR-INV-002` invariant in the DDD and the `parent_session_id` chain validation logic. | Robert Fall |
| OQ-03 | Should `cfa-harness replay` re-invoke tool calls that were previously successful, or should it cache-replay the stored `ToolResult` from the Session Archive? Re-invoking costs tokens and may produce different results if MCP server data has changed. Cache-replay is cheaper and deterministic but means the replay is not a live re-run. | Affects Session Memory `restore` semantics and NFR1 (determinism). | Robert Fall |
| OQ-04 | Should the harness support streaming (`@anthropic-ai/sdk` streaming messages) in Wave 1, or deliver the full response as a single block? Streaming reduces time-to-first-output for long memos but complicates the Audit Chain (partial entries). | Affects Agent Runtime dispatch loop design and Audit Chain entry timing. | Robert Fall |

---

## 11. References

| Document | Path |
|----------|------|
| Master plan | `docs/plans/phase-31-harness.md` |
| DDD (six bounded contexts) | `docs/ddd/phase-31-harness.md` |
| ADR-031: Custom harness decision | `docs/adr/ADR-031-custom-harness.md` |
| ADR-032: TypeScript / Node.js ≥ 20 | `docs/adr/ADR-032-typescript-node.md` |
| ADR-033: @anthropic-ai/sdk ≥ 0.30 | `docs/adr/ADR-033-anthropic-sdk.md` |
| ADR-034: @modelcontextprotocol/sdk | `docs/adr/ADR-034-mcp-sdk.md` |
| ADR-035: Stdio MCP transport (Wave 1) | `docs/adr/ADR-035-mcp-stdio-transport.md` |
| ADR-036: SHA-256 audit hash chain | `docs/adr/ADR-036-audit-sha256.md` |
| ADR-037: Agent Registry as code | `docs/adr/ADR-037-agent-registry-as-code.md` |
| Specflow contracts | `docs/contracts/feature_harness_*.yml` |
| Phase 26 memory/audit DDD | `docs/ddd/domain-audit-observability.md`, `docs/ddd/domain-memory.md` |
| Memory note (subagent tool-use failure) | `~/.claude/projects/-home-robert-cfa-agent/memory/feedback_cfa_agents_cannot_invoke_tools.md` |

---

## 12. Requirement Traceability Matrix

| Functional Req | DDD Context | NFR | ADR |
|----------------|-------------|-----|-----|
| F1 Chief-Analyst Dispatch | Agent Runtime (AgentSession) | NFR1, NFR2, NFR3 | ADR-031, ADR-033 |
| F2 Specialist Routing | Agent Runtime (sub-dispatch), Agent Registry | NFR1, NFR2 | ADR-037 |
| F3 MCP Tool Invocation | MCP Transport (McpServerConnection, ToolRouterTable) | NFR2, NFR4 | ADR-034, ADR-035 |
| F4 Multi-Provider | Provider Abstraction (ProviderAdapter) | NFR1, NFR6 | ADR-033 |
| F5 Audit Chain | Audit Chain (AuditLog, AuditEntry) | NFR1, NFR4 | ADR-036 |
| F6 Session Replay | Session Memory (SessionArchive) | NFR1 | ADR-031 |
| F7 CLI Surface | Agent Registry (resolve), Agent Runtime | NFR5 | ADR-032 |
| F8 Programmatic API | Agent Runtime, Agent Registry | NFR5, NFR6 | ADR-032 |
