# Phase 31 — CFA Harness

**Status:** Planning
**Date:** 2026-05-09
**Owner:** Robert Fall
**Predecessors:** Phase 29 (compute extraction to crates.io 1.1.0), Phase 30 (script-to-tool MCP migration)

## Problem statement

Claude Code's subagent dispatch is the runtime path the cfa-* agents use to act on user prompts. After Phase 29/30 the entire compute and tool surface is correct: 4 plugin MCP servers ✓ Connected, 623 tools registered, agent frontmatter allowlists updated to plugin-namespaced wildcards (commit `6c4bd78`), agent body documentation rewritten with prefixed names + invocation conventions across all 9 cfa specialists (commit `d548509`, 308 plugin-prefix references inserted).

Despite this, **cfa-* subagent dispatches return `tool_uses: 0` on minimal probes**. The agent emits `tool_use` JSON as text but never invokes. A general-purpose subagent given the same prompt returns `tool_uses: 22` and produces the canonical Chaco Minerals memo with rust_decimal-precision warrant pricing.

The gap is intrinsic to the cfa-* subagent class in Claude Code's runtime. Memory note `feedback_cfa_agents_cannot_invoke_tools.md` records the negative result of every plugin-install / allowlist / body-rewrite fix attempted.

The path forward is a custom harness that owns the dispatch loop end-to-end.

## Goals

1. **End-to-end natural-language → tool-grounded result** for the chief-analyst → specialist → MCP-tool flow, with `tool_uses ≥ 21` on the Chaco acceptance test.
2. **Provider-neutral dispatch loop** — Anthropic now, OpenAI/Gemini/Bedrock as additive provider adapters.
3. **Reuse existing investments** — the four plugin MCP servers (cfa-core 227, cfa-data 129, fmp-market-data 180, vendor 87), the corp-finance-core compute library on crates.io 1.1.0, and the audit/memory modules already shipped in Phase 26.
4. **Auditable** — every tool call recorded, every agent invocation hashed, every numerical result traceable to a tool with logged inputs and outputs (the Phase 26 audit chain pattern).
5. **Subagent-correct dispatch** — chief decomposes prompt → specialists invoke tools → chief aggregates. Each level is a real Messages-API loop, not a string of text-as-pretend-tool-calls.

## Non-goals (Phase 31 explicitly defers)

- Federation across machines (Phase 32+ if needed; the existing `corp_finance_core::federation` module covers the primitives).
- Container sandboxing per agent (agentbox-style; deferred to Phase 32 wave).
- Replacing the Claude Code IDE — the harness exposes a CLI (`cfa-harness run`) and a programmatic API; users keep using Claude Code as their editor.
- Self-modifying agents / SONA-style adaptive learning.

## Inspirations

| Repo | Adopted | Rejected / deferred |
|---|---|---|
| **ruvnet/open-claude-code** | Async generator agent loop pattern. Four MCP transports (stdio/SSE/HTTP/WS). Provider abstraction for Anthropic/OpenAI/Google/Bedrock/Vertex. Permission modes for tool-call approval. | Reverse-engineered code is not a dependency — only the architectural pattern is borrowed. |
| **ruvnet/ruflo** | Hierarchical Queen-led dispatch (chief → specialists). Plugin-as-agent pattern. Smart routing via shared memory. The 27-hook event model. | We do not adopt the federation/Byzantine consensus layer for Phase 31 — single-machine dispatch is sufficient. The SONA self-learning loop is also deferred. |
| **DreamLab-AI/agentbox** | Per-agent API-key scoping. Privacy-filter sidecar pattern for log redaction. | Nix-reproducible OCI containers and `did:nostr` identity mesh are deferred — the harness is a TypeScript library first; sandbox-as-deployment is a Phase 32 concern. |

## Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  cfa-harness (TypeScript, ~2000 LOC across packages)                        │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌──────────────────────┐    ┌──────────────────────────────┐              │
│  │  packages/cli        │    │  packages/agents             │              │
│  │  cfa-harness run     │───▶│  registry, chief-analyst,    │              │
│  │  --agent chief       │    │  8 specialists               │              │
│  │  --prompt @file      │    │  delegate.ts (sub-dispatch)  │              │
│  └──────────────────────┘    └──────────────┬───────────────┘              │
│                                              │                              │
│  ┌──────────────────────────────────────────▼──────────────────────────┐  │
│  │  packages/core (the dispatch loop)                                  │  │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────────┐  │  │
│  │  │ agent-loop   │  │ tool-router  │  │ providers/               │  │  │
│  │  │ async gen    │──│ tool_use →   │──│   anthropic.ts (now)     │  │  │
│  │  │ Messages API │  │ MCP dispatch │  │   openai.ts (W3)         │  │  │
│  │  └──────────────┘  └──────┬───────┘  │   gemini.ts (W3)         │  │  │
│  │                            │          │   bedrock.ts (W4)        │  │  │
│  │                            │          └──────────────────────────┘  │  │
│  │                            ▼                                         │  │
│  │  ┌─────────────────────────────────────────────────────────────┐   │  │
│  │  │ packages/mcp-client (4 transports)                          │   │  │
│  │  │  stdio (now) ── SSE (W3) ── HTTP (W3) ── WebSocket (W4)     │   │  │
│  │  └─────────────────────────────────────────────────────────────┘   │  │
│  └──────────────────────────────────────────────┬──────────────────────┘  │
│                                                  │                          │
│  ┌──────────────────────┐    ┌──────────────────▼──────────────────────┐  │
│  │  packages/audit      │    │  packages/memory                        │  │
│  │  hash chain over     │    │  session state, agent inbox,            │  │
│  │  invocations         │    │  durable across runs                    │  │
│  └──────────────────────┘    └─────────────────────────────────────────┘  │
└──────────────────────────────────────┬──────────────────────────────────────┘
                                        │
                                        ▼
              ┌──────────────────────────────────────────────────┐
              │  EXISTING — reused without modification          │
              ├──────────────────────────────────────────────────┤
              │  plugin:cfa-core:cfa-core      (227 tools, WASM) │
              │  plugin:cfa-data:data           (129 tools)      │
              │  plugin:cfa-pro:fmp-market-data (180 tools)      │
              │  plugin:cfa-pro:vendor          (87 tools)       │
              │  ───────────────────────────────────────────     │
              │  corp-finance-core 1.1.0 on crates.io            │
              │  (the compute library beneath the WASM plugin)   │
              └──────────────────────────────────────────────────┘
```

## Bounded contexts (full DDD in `docs/ddd/phase-31-harness.md`)

1. **Agent Runtime** — the async generator dispatch loop, Messages-API turn management, end_turn detection, error retry.
2. **MCP Transport** — stdio/SSE/HTTP/WS transports, MCP handshake, tools/list, tools/call routing.
3. **Provider Abstraction** — Anthropic Messages API, OpenAI function-calling, Gemini tool-calls, Bedrock invocation. Tool-schema translation across providers.
4. **Agent Registry** — code-defined agent specs (system prompt + tool subset + delegation rules), dispatch table, capability map.
5. **Audit Chain** — sha2-hashed invocation log, deterministic across same input, exportable to corp-finance-core's audit module.
6. **Session Memory** — per-conversation state (messages, tool results, sub-dispatch tree), restorable from disk for replay.

## Wave plan

### Wave 1 — Core dispatch + chief-analyst (acceptance: Chaco memo, `tool_uses ≥ 21`)

Scope:
- `packages/core/agent-loop.ts` — async generator wrapping `@anthropic-ai/sdk` Messages API
- `packages/core/tool-router.ts` — translates `tool_use` blocks → MCP `tools/call`
- `packages/mcp-client/stdio.ts` — JSON-RPC over stdio to the four cfa plugin servers
- `packages/agents/chief-analyst.ts` — system prompt + access to all 623 tools
- `packages/cli/run.ts` — `cfa-harness run --agent chief-analyst --prompt @file`
- `tests/chaco-acceptance.test.ts` — runs the canonical memo, asserts `tool_uses ≥ 21`

Estimated: ~600 LOC, 2–3 days.

### Wave 2 — Specialists + recursive dispatch

Scope:
- `packages/agents/specialists/` — derivatives, equity, credit, fixed-income, quant-risk, macro, private-markets, esg-regulatory
- `packages/agents/delegate.ts` — chief delegates to specialists via nested Messages-API calls; specialists return structured outputs to chief; chief aggregates
- Each specialist's tool subset constrained to their domain (e.g., derivatives only sees `option_pricer`, `implied_volatility`, etc.)
- `tests/specialist-routing.test.ts` — assert chief routes warrant grid → derivatives, country risk → macro

Estimated: ~500 LOC, 2 days.

### Wave 3 — Multi-provider + remote MCP transports

Scope:
- `packages/core/providers/openai.ts` — Messages-API ↔ OpenAI Chat Completions translator (tool format + role conventions)
- `packages/core/providers/gemini.ts` — Anthropic ↔ Gemini Generate Content
- `packages/mcp-client/sse.ts`, `http.ts` — remote MCP server support
- `tests/cross-provider-determinism.test.ts` — same prompt across providers produces same tool calls

Estimated: ~600 LOC, 3 days.

### Wave 4 — Audit, memory, sandbox (optional)

Scope:
- `packages/audit/chain.ts` — sha2 hash chain over `(agent_id, prompt_hash, tool_calls[], result_hash)`
- `packages/memory/session.ts` — durable session state, replay support
- (Optional) agentbox-style API-key scoping — only inject `FMP_API_KEY` into agents that allowlist FMP tools

Estimated: ~400 LOC, 2 days.

**Total Phase 31: ~2100 LOC TypeScript across 4 waves, ~10 working days end-to-end.**

## Tech stack

| Concern | Choice | ADR |
|---|---|---|
| Language | TypeScript (strict) | ADR-032 |
| Runtime | Node.js ≥ 20 | ADR-032 |
| Anthropic SDK | `@anthropic-ai/sdk` ≥ 0.30 | ADR-033 |
| MCP client | `@modelcontextprotocol/sdk` (re-using the SDK shipped with Claude Code) | ADR-034 |
| Tests | vitest | (project-wide convention) |
| Audit hash | sha2 (mirrors corp-finance-core::audit) | ADR-036 |
| Build | tsc + npm workspaces (project-wide) | (existing) |

## Acceptance test

The canonical Chaco Minerals memo:

```bash
cfa-harness run \
  --agent chief-analyst \
  --prompt docs/examples/chaco-minerals-deal.md \
  --output out/chaco-memo.md
```

Must produce:
- `tool_uses ≥ 21` (20 × `option_pricer` + 1 × `country_risk_premium`)
- Numerical results matching the canonical values from /tmp/mcp-chaco-demo.mjs (e.g., per-warrant price at C$0.10/80% = C$0.0208)
- Memo with all 9 sections from the standard IC memo template
- Tool invocation log written to `out/chaco-memo.audit.json` for traceability

## Risks

| Risk | Mitigation |
|---|---|
| `@anthropic-ai/sdk` API drift | Pin to `^0.30.0`; track release notes in `feedback_anthropic_sdk.md`. |
| Tool-schema translation losses across providers | Wave 3 only; keep tool schemas Anthropic-flavored and translate at the wire boundary; provider-neutral tool definitions live in `packages/core/tool-schema.ts`. |
| API rate limits during recursive dispatch | Per-agent token budget caps. Specialist dispatches are bounded (max depth 1 in W2). |
| Re-introducing the Claude Code subagent issue via SDK quirks | Acceptance test is the canary — if `tool_uses` is ever 0 on a non-trivial dispatch, the harness has a bug; ship it as a release-blocker. |

## Cross-references

- **ADRs:** `docs/adr/ADR-031-custom-harness.md` through `ADR-037-agent-registry-as-code.md`
- **DDD:** `docs/ddd/phase-31-harness.md` (six bounded contexts)
- **PRD:** `docs/prd/phase-31-harness.md`
- **Specflow contracts:** `docs/contracts/feature_harness_*.yml` (six contracts)
- **Memory note:** `~/.claude/projects/-home-robert-cfa-agent/memory/feedback_cfa_agents_cannot_invoke_tools.md`
- **Phase 30 acceptance evidence:** general-purpose subagent dispatch with `tool_uses: 22` on the Chaco prompt (this session, 2026-05-09)
