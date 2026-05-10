# ADR-040: RuVector for the Reasoning Bank

## Status: Accepted

## Date: 2026-05-10

## Deciders

- Robert Fall
- CFA Agent platform engineering

## Tags

`harness`, `reasoning-bank`, `vector-search`, `ruvector`, `semantic-recall`,
`graph-recall`, `phase-31-bc4`

## Context

Phase 31's Bounded Context 4 ("Learning & Adaptation") was documented in
the DDD doc but never built. At Phase 31's authoring time there was no
implementation plan, no clear corpus to operate over, and the risk of a
premature semantic-memory layer outweighed the speculative gain. After
Phases 31-33 ship, three preconditions converge:

1. **Audit chain produces real data** — every dispatch writes a sha256-
   hashed `AuditRecord` with prompt + tool calls + result. The corpus
   grows organically with use.
2. **Session memory captures full conversations** — replayable end-to-end.
   Trajectories are addressable.
3. **Skill-driven specialists (Phase 33, ADR-031)** make routing decisions
   explicit — chief delegates to derivatives via
   `delegate_to_derivatives_analyst`. These delegation choices are the
   routing-pattern signal.

Phase 34 closes the loop with semantic memory: a reasoning bank that
indexes past dispatches semantically, retrieves relevant prior work for
new prompts, and (over time) lets the chief learn which specialist routes
best for which prompt shape.

## Decision

### 1. Adopt `ruvector` as the bank's vector backend

`ruvector` 0.2.25 (Rust core + NAPI-RS Node bindings, MIT-licensed,
published to npm) is the harness's vector storage. The bank port wraps
the published `VectorDB` API behind `RVIndex` (rv-index.ts) so library-
specific quirks (Float32Array marshalling, JSON metadata, distance-metric
naming) don't leak. The `.rvf` file format is portable and persists
without an external server. Mounted as a separate concern from the audit
chain and session store (both file-based via `FileJsonStore`) — different
problem shapes, different optimal storage, both coexist.

### 2. The ReasoningBank port is the boundary

`ReasoningBank` (bank.ts) is the outbound port:

```typescript
interface ReasoningBank {
  index(entry, queryText?): Promise<void>;
  recallSimilar(query, opts?): Promise<ReasoningEntry[]>;
  recallByGraph(query: GraphRecallQuery): Promise<ReasoningEntry[]>;
  close(): Promise<void>;
}
```

The agent loop only ever talks to this interface. RuVector specifics live
under `RVIndex`. Agents see the surface as two virtual tools —
`recall_similar` (k-NN over prompt embeddings) and `recall_by_graph`
(structured-metadata filter) — never the underlying store.

### 3. Embedding choice is pluggable

`EmbeddingFn` is a function type. The harness ships
`createOpenAIEmbedder()` (production) and `createDeterministicEmbedder()`
(tests + offline fallback). Voyage, Anthropic-future, or a local
`run_llm`-backed embedder can be added without touching `bank.ts` or
`rv-index.ts`.

### 4. Graph queries are scan-and-filter today

`ruvector` 0.2.25's npm release does NOT ship the Cypher / hyperedge
graph surface that the GitHub README documents as a roadmap item — only
the vector-DB wrapper is published. Wave 4 implements `recallByGraph` as
a structured-metadata query: each `RVIndex.insert` mirrors the entry to
an append-only JSONL sidecar (`<dir>/scan.jsonl`); `RVIndex.scan` reads
the JSONL and applies flat filters; `ReasoningBank.recallByGraph`
applies the richer predicates (metadata equality, hasTools,
hasDelegations) in JS.

The `recall_by_graph` virtual tool spec and the `ReasoningBank.recallByGraph`
public interface are stable. When ruvector publishes a Cypher / graph
surface, we swap the implementation under the port without changing the
public surface or the chief skill prose.

## Why ruvector specifically

| Alternative | Reason rejected |
|---|---|
| `faiss-node` | Heavyweight; no graph roadmap; LGPL licensing complications |
| `hnswlib-node` | Pure HNSW, no metadata filtering, no persistence layer |
| In-memory + JSON serialization | Doesn't index incrementally; no sublinear search |
| AgentDB plugin (already installed in user's Claude Code) | Adds an MCP indirection layer when in-process is simpler; and uses ruvector itself under the hood |
| Pinecone / Weaviate / Qdrant | External service; HTTP latency on the dispatch hot path; vendor-lock |

`ruvector` matches the harness's existing native-binding pattern (we
already have NAPI bindings in `packages/bindings/`), is MIT-licensed,
ships a single `.rvf` portable file, has a published roadmap toward the
graph surface we want, and is designed for local AI-agent workloads
exactly like ours.

## Consequences

### Positive

- Phase 31 BC4 is implemented.
- Chief gains semantic priors (`recall_similar`) and structured priors
  (`recall_by_graph`) — both observable via tool_call audit records.
- Routing decisions improve with use: prior `delegations` arrays are
  visible to the chief through both recall surfaces.
- Vector store is a single `.rvf` file; portable, backup-friendly.
- Embedding API cost is bounded by the sha256-keyed cache.

### Negative

- Adds a NAPI native dependency (binary download at install).
- Embedding API cost (when production embedder is used). Mitigated by
  cache + provider-pluggable design.
- The published 0.2.25 wrapper has no enumerate API, requiring the
  JSONL scan sidecar (ADR-041 details the agent-loop implication).
- Native-binding flakiness under high test parallelism — observed once
  during Wave 4 development as a SIGSEGV when many VectorDB instances
  spin up simultaneously. Vitest's default file parallelism handles this
  in practice; the segfault has not reproduced. If it does, the workaround
  is `vitest run --no-file-parallelism` (which the CI already uses for the
  acceptance suite).

### Neutral

- Failure mode if ruvector publishes a breaking change → version pin in
  `packages/harness/package.json`; Dependabot tracks upstream.

## Alternatives Considered

**Defer Phase 34 entirely** — Rejected. Phases 31-33 made the
preconditions real; without the bank, the chief's routing decisions stay
amnesiac across dispatches.

**Use the `ruflo-agentdb` plugin already installed in user-scope Claude
Code** — Rejected. The plugin uses ruvector under the hood anyway; mounting
the bank in-process at the harness layer removes one MCP indirection and
gives us direct access to the JSONL scan sidecar pattern.

**Wait for ruvector graph queries before implementing `recallByGraph`** —
Rejected. The chief-facing surface is identical either way; structured-
metadata recall is genuinely useful today (filter every Argentine private
placement; every dispatch that delegated to derivatives). When the
upstream graph queries land we swap the implementation, not the
interface.

**Implement our own Cypher parser** — Rejected. Out of scope. The
roadmap on ruvector's side is the right place for that work.

## Future work

- When ruvector ships graph queries, replace the JSONL-sidecar scan
  with the native graph surface. Public interface stays the same.
- Add `cfa-harness --rebuild-reasoning` CLI flag to re-derive the bank
  from the audit chain (useful for index recovery, embedding-model
  upgrades, or sidecar corruption).
- Per-tenant reasoning isolation. Phase 34 ships a single shared bank;
  multi-tenant isolation deferred to Phase 35+ if it becomes relevant.
- PII redaction on the indexing path. Embeddings are irreversible but
  similarity search across PII-laden prompts is still potentially
  leaky; redact-before-index is a future hardening step.

## Links

- Plan: `docs/plans/phase-34-reasoning-bank.md`
- Implementation:
  - `packages/harness/src/reasoning/bank.ts` — port + factory
  - `packages/harness/src/reasoning/rv-index.ts` — RuVector wrapper +
    JSONL scan sidecar
  - `packages/harness/src/reasoning/embeddings.ts` — pluggable embedders
  - `packages/harness/src/reasoning/recall-tool.ts` — `recall_similar`
    virtual tool
  - `packages/harness/src/reasoning/recall-graph-tool.ts` —
    `recall_by_graph` virtual tool
- DDD: `docs/ddd/domain-orchestration.md` — BC4 Reasoning Bank section
- Companion ADR: ADR-041 (Indexer hook in the agent loop)
- Depends on: ADR-031 (skill-driven specialists), ADR-024 (MCP tool-call
  audit)
- Upstream: https://github.com/ruvnet/RuVector — Rust core + NAPI-RS
  bindings; npm `ruvector@^0.2.25`
