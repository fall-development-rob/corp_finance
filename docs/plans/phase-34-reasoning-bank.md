# Phase 34 — Reasoning Bank (RuVector-backed memory)

**Status:** Planning
**Date:** 2026-05-10
**Owner:** Robert Fall
**Predecessors:** Phase 33 (skill-driven harness), Phase 31 Wave 4 (audit chain + session memory).

## Why Phase 34

Phase 31 BC4 ("Learning & Adaptation") was documented in the DDD doc but never built — at the time there was no implementation plan and no clear use case ("the chief has done 50 IC memos; find the 5 most similar to this Chaco deal" requires real production data we didn't have yet). After Phases 31–33 ship, three conditions converge:

1. **Audit chain is producing real data** — every dispatch writes a sha256-hashed `AuditRecord` with prompt + tool calls + result. The corpus grows organically with use.
2. **Session memory captures full conversations** — replayable end-to-end. Trajectories are addressable.
3. **Skill-driven specialists** (Phase 33) make routing decisions explicit — chief delegates to derivatives via `delegate_to_derivatives_analyst`. These delegation choices are the routing-pattern signal.

Phase 34 closes the loop: a **reasoning bank** that indexes past dispatches semantically, retrieves relevant prior work for new prompts, and over time learns which specialist routes best for which prompt shape.

## Why RuVector specifically

The `ruvnet/RuVector` library (Rust core, NAPI-RS Node bindings) is purpose-built for this:

| Capability | Use in Phase 34 |
|---|---|
| HNSW similarity search | "Find dispatches whose prompt embedding is nearest to this new one" |
| GNN-enhanced index (learns from query patterns) | Routing-decision learning improves with use |
| Hyperedge knowledge graph + Cypher queries | "Every deal the chief has analyzed in Argentina; every Argentine private placement; every convertible debenture" |
| Metadata filtering at search time | Constrain by `agent_id`, time window, deal type without scanning |
| Local LLM inference (`run_llm`) | Optional: re-embed prompts via the same model the harness uses |
| Both NAPI and WASM bindings | Same library serves the harness (Node) and any future browser dashboard |
| `.rvf` cognitive container format (portable) | Audit + memory + index ship as one file, not three databases |

The right level of fit. Audit + session storage doesn't need any of this (Phase 32 Wave 2's `FileJsonStore` is correct there). Reasoning recall does need all of this.

## Architecture

```
┌──────────────────────────────────────────────────────────────────┐
│  EXISTING (Phases 31-33)                                         │
├──────────────────────────────────────────────────────────────────┤
│                                                                  │
│  packages/harness/src/                                           │
│    persistence/file-json-store.ts  ← key-value, append-only     │
│    audit/chain.ts                  ← AuditSink (FileJsonStore)  │
│    memory/session.ts               ← SessionStore (FileJsonStore)│
│    skills/loader.ts                ← Phase 33 skill loader      │
│                                                                  │
└──────────────────────────────────────────────────────────────────┘
            │                                    │
            │ AuditRecord stream                 │ AgentDef
            ▼                                    │
┌──────────────────────────────────────────────────────────────────┐
│  NEW (Phase 34)                                                  │
├──────────────────────────────────────────────────────────────────┤
│                                                                  │
│  packages/harness/src/reasoning/                                 │
│    bank.ts                  ← ReasoningBank port + impl          │
│    embeddings.ts            ← prompt → vector helper             │
│    rv-index.ts              ← RuVector HNSW wrapper              │
│    rv-graph.ts              ← RuVector knowledge graph wrapper   │
│    indexer.ts               ← AuditRecord → ReasoningBank entry  │
│                                                                  │
│  packages/harness/src/agents/specialists/                        │
│    (chief skill body)       ← gets `recall_similar` virtual tool │
│                                                                  │
└──────────────────────────────────────────────────────────────────┘
```

`ReasoningBank` is a new outbound port in `cfa-application` (per Phase 32 Wave 3 hex layering, if it's landed by then) with this shape:

```typescript
export interface ReasoningEntry {
  audit_id: string;             // back-reference to AuditRecord
  agent_id: string;             // who produced this trajectory
  prompt_hash: string;          // sha256 (already in AuditRecord)
  prompt_summary: string;       // ≤ 200 char human-readable summary
  embedding: number[];          // prompt embedding (768- or 1024-dim)
  tool_calls: { name: string; count: number }[];
  delegations: string[];        // specialist agents the chief routed to
  result_excerpt: string;       // ≤ 500 char summary of finalText
  metadata: Record<string, unknown>; // issuer, jurisdiction, instrument, etc.
  timestamp: string;
}

export interface ReasoningBank {
  index(record: AuditRecord, prompt: string, finalText: string): Promise<void>;
  recallSimilar(prompt: string, opts?: {
    k?: number;                // top-k results, default 5
    filter?: Partial<{
      agent_id: string;
      since: Date;
      metadata: Record<string, unknown>;
    }>;
  }): Promise<ReasoningEntry[]>;
  recallByGraph(query: string): Promise<ReasoningEntry[]>; // Cypher
  close(): Promise<void>;
}
```

## What Phase 34 changes

1. **New `packages/harness/src/reasoning/`** — `ReasoningBank` port + RuVector-backed implementation.
2. **`agent-loop.ts`** gains an optional `reasoning?: ReasoningBank` field on `DispatchOptions`. When provided, after a successful dispatch the loop calls `reasoning.index(auditRecord, prompt, finalText)` — non-blocking, never on the hot path.
3. **Chief specialist skill** gains a virtual `recall_similar(prompt: string, k?: number)` tool. When the chief invokes it, the harness intercepts (same pattern as `delegate_to_*` from Wave 2) and calls `reasoning.recallSimilar(...)`. The chief uses the returned `ReasoningEntry[]` to inform its routing decisions and as priors for the new analysis.
4. **CLI gains `--reasoning-dir <dir>` flag** mirroring `--audit-dir` and `--session-dir`. When set, the harness wires up a `RuVectorReasoningBank` rooted at that directory.

## What Phase 34 does NOT change

- **Audit chain** — stays file-based via `FileJsonStore`. The reasoning bank READS audit records but doesn't replace them.
- **Session memory** — stays file-based.
- **MCP plugins** — unchanged (RuVector is in-process, not an MCP server).
- **Skills** — unchanged in format. The chief's skill body gains one paragraph documenting the new `recall_similar` tool.
- **The 4 plugin MCP servers** — RuVector lives in the harness, not as a plugin; agents access reasoning recall through the harness's virtual tool surface.

## Wave plan

### Wave 1 — RuVector wrapper + bank port (3 days)

1. New `packages/harness/src/reasoning/rv-index.ts` — thin wrapper over `ruvector` npm package (NAPI). Open / insert / search / close.
2. New `embeddings.ts` — uses Anthropic's embedding API (or OpenAI's if Anthropic doesn't ship one) to convert prompt strings to vectors. Cache embeddings keyed by `sha256(prompt)`.
3. New `bank.ts` — `ReasoningBank` interface + `createRuVectorBank({ dir })` factory.
4. Unit tests with a temp `.rvf` directory; index a handful of AuditRecords; assert `recallSimilar` returns expected ordering.

### Wave 2 — Indexer + agent-loop hook (2 days)

5. New `indexer.ts` — `indexAuditRecord(bank, record, prompt, finalText)`. Computes embedding, builds metadata, delegates to bank.
6. `agent-loop.ts` integration: after the audit write, if `reasoning` is in `DispatchOptions`, fire-and-forget `indexer.indexAuditRecord(...)`. Indexing failure does NOT fail the dispatch — log and continue.
7. End-to-end test: run a Chaco-style dispatch with both `audit` and `reasoning` configured; assert the audit record AND the reasoning entry both land; assert `recallSimilar` on the same prompt returns the just-indexed entry.

### Wave 3 — Recall as a virtual tool (2 days)

8. Extend the chief specialist's skill body (in `corp-finance-analyst-chief/SKILL.md`) with a paragraph documenting `recall_similar`.
9. Wire `recall_similar` as a virtual tool in `agent-loop.ts` (same pattern as `delegate_to_*` — handle in the dispatch loop, not as an MCP tool).
10. Live acceptance: index 10 sample dispatches; run an 11th similar to one of them; assert the chief calls `recall_similar` and the response includes the most-similar prior.

### Wave 4 — Knowledge-graph queries + cleanup (2 days)

11. Add `recallByGraph(query)` for Cypher queries — surface in chief skill.
12. Update Phase 31 DDD: add the Reasoning Bank context (or formalize Phase 31 BC4); add the `recall_similar` virtual tool to the Agent Runtime context's tool catalog.
13. Write ADR-040 (RuVector for reasoning bank) and ADR-041 (Indexer hook in agent-loop).

**Total Phase 34: ~9 days, ~600 LOC added (reasoning module), 0 LOC removed.**

## Success metrics

| Metric | Pre-Phase-34 | Target |
|---|---|---|
| Dispatches indexed for similarity recall | 0 | every dispatch (when `reasoning` enabled) |
| `recall_similar` virtual tool available to chief | no | yes |
| Cross-session learning signal | none | routing patterns visible in chief's call sequence |
| Phase 31 BC4 ("Learning & Adaptation") | unbuilt | implemented |
| RuVector NAPI integration in harness | absent | working with `.rvf` persistence |

## Risk register

| Risk | Mitigation |
|---|---|
| Embedding API cost at scale | Cache embeddings by `sha256(prompt)` (we already hash prompts for audit). Re-use across dispatches. |
| RuVector NAPI binding breaks on a new Node version | Pin to a specific RuVector version; track upstream releases via Dependabot. |
| Indexing on the dispatch hot path adds latency | Fire-and-forget after audit write; never block the dispatch return. Failed index is a log entry, not an error. |
| Vector index file corruption | RuVector handles this internally (writes are atomic); add a `--rebuild-reasoning` CLI command to re-index from the audit chain if needed. |
| Recall returns irrelevant priors and confuses the chief | Top-k is configurable; chief skill body explicitly tells the chief to ignore priors with similarity < 0.7 (configurable threshold). |

## Open questions

1. **Embedding provider** — Anthropic doesn't currently ship a public embedding API. Options: (a) OpenAI embeddings (well-tested, $0.02/1M tokens for `text-embedding-3-small`), (b) RuVector's own embeddings (requires local ruvllm — adds installation complexity), (c) Voyage AI (used by Anthropic in some demos). Recommend (a) for Phase 34 Wave 1; revisit if cost or vendor risk surfaces.
2. **Index directory portability** — `.rvf` files are RuVector-specific; if RuVector goes away, audit records remain (the index is a derived artefact). Consider a `--reset-reasoning` command that wipes and rebuilds.
3. **Multi-tenant isolation** — does each user / agent class get its own index, or one shared index with `agent_id` as a metadata filter? Recommend one shared index for Phase 34; per-tenant isolation is a Phase 35+ concern if it becomes relevant.
4. **Privacy** — audit records contain prompts which may include PII. The reasoning bank stores embeddings (irreversible) plus metadata (selective). Embeddings of PII are still potentially leaky in similarity search. Consider a redaction step before indexing if PII surfaces.

## Cross-references

- Phase 31 DDD `docs/ddd/phase-31-harness.md` — BC4 "Learning & Adaptation" is the formal home of this work.
- Phase 31 Wave 4 audit + memory — Phase 34 reads from these but does not modify them.
- Phase 32 Wave 2B (`FileJsonStore<T>`) — distinct concern (key-value persistence) from Phase 34's similarity search; both coexist.
- Phase 33 (skill-driven harness) — chief skill body gains a paragraph on `recall_similar`; no other interaction.
- RuVector (https://github.com/ruvnet/RuVector) — Rust core + NAPI-RS bindings. `npm install ruvector` from the workspace root.
- ruflo-agentdb plugin (already installed in the user's Claude Code) — uses the same RuVector under the hood; Phase 34 uses RuVector directly rather than via the AgentDB plugin (one less indirection).

## Recommendation

**Land Phase 33 first** (skill-driven harness — Wave 1 alone proves the loader pattern). Phase 34 sits on top: it depends on the chief skill body being editable as a `.md` skill file (which is what Phase 33 enables). Trying to add `recall_similar` to a TypeScript-defined chief specialist would require a code release for every prose tweak — exactly the friction Phase 33 removes.

Sequence: Phase 32 Wave 2 (PR #41) → merge → Phase 33 Waves 1-4 → Phase 34 Waves 1-4. Total: ~16 working days; Phases 33+34 together net ~−1,500 LOC (Phase 33's specialist deletion dwarfs Phase 34's reasoning addition).
