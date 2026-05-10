/**
 * ReasoningBank — port + RuVector-backed implementation.
 *
 * The bank wraps an `RVIndex` with the embedding step factored out: callers
 * pass a `ReasoningBankOptions.embed` function and the bank produces vectors
 * lazily on `index()` and `recallSimilar()`. This keeps embedding choice
 * (OpenAI / deterministic / future Voyage / future local) orthogonal to
 * vector storage.
 *
 * Wave 4 adds `recallByGraph` — structured-metadata queries over the indexed
 * entries. The published `ruvector` 0.2.25 npm release does not ship the
 * Cypher / hyperedge surface in its README roadmap, so the implementation
 * scans the index via `RVIndex.scan` (a JSONL-sidecar mirror — see rv-index.ts
 * for the rationale) and applies the filter in JS. The public surface is
 * stable: when ruvector ships native graph queries we can swap the
 * implementation without changing this interface, the virtual tool, or the
 * chief skill prose.
 */
import type { EmbeddingFn } from "./embeddings.js";
import { openRVIndex, type RVIndex } from "./rv-index.js";

/**
 * One indexed dispatch trajectory. Mirrors the shape in
 * docs/plans/phase-34-reasoning-bank.md (lines 70-82).
 */
export interface ReasoningEntry {
  /** Back-reference to the AuditRecord this entry summarizes. */
  audit_id: string;
  /** Which agent produced the trajectory (chief, equity-analyst, etc.). */
  agent_id: string;
  /** sha256 of the original prompt — already in the audit record. */
  prompt_hash: string;
  /** ≤ 200 char human-readable prompt summary. */
  prompt_summary: string;
  /** Embedding vector of the prompt summary (or whatever queryText was passed). */
  embedding: number[];
  /** Aggregated counts of tool invocations during the dispatch. */
  tool_calls: { name: string; count: number }[];
  /** IDs of specialists the chief routed to. */
  delegations: string[];
  /** ≤ 500 char result summary. */
  result_excerpt: string;
  /** Free-form structured metadata (issuer, jurisdiction, instrument…). */
  metadata: Record<string, unknown>;
  /** ISO 8601 timestamp; used for `since` filtering. */
  timestamp: string;
}

export interface ReasoningBankOptions {
  /** Storage directory for the underlying RuVector `.rvf` file. */
  dir: string;
  /** Embedding function — e.g. `createDeterministicEmbedder()` for tests. */
  embed: EmbeddingFn;
  /**
   * Vector dimension. Should match the embedder; defaults to 384
   * (deterministic embedder). Use 1536 for OpenAI text-embedding-3-small.
   */
  dimensions?: number;
}

export interface RecallOptions {
  /** Top-k results, default 5. */
  k?: number;
  filter?: Partial<{
    agent_id: string;
    since: Date;
    metadata: Record<string, unknown>;
  }>;
}

/**
 * Wave 4 — structured graph-style query over the indexed entries.
 *
 * Every field is optional. Provided fields are ANDed together (entry must
 * satisfy every supplied predicate). Within `hasTools` and `hasDelegations`
 * matching is OR-style — an entry passes if it overlaps ANY of the supplied
 * names. Within `metadata` matching is equality on each provided key.
 *
 * An empty / missing filter returns the most recent `limit` entries globally
 * — this is intentional, mirroring the recall-by-recency pattern many
 * reasoning-bank consumers want when no other signal is available.
 */
export interface GraphRecallQuery {
  /** Filter to entries whose metadata matches each provided key. Equality only. */
  metadata?: Record<string, unknown>;
  /** Filter to entries that invoked at least one of these tool names. */
  hasTools?: string[];
  /** Filter to entries that delegated to at least one of these specialist ids. */
  hasDelegations?: string[];
  /** Filter to entries from this agent. */
  agent_id?: string;
  /** Filter to entries timestamped on or after this datetime. */
  since?: Date;
  /** Filter to entries timestamped on or before this datetime. */
  until?: Date;
  /** Maximum entries to return. Default 50, hard cap 500. */
  limit?: number;
}

/** Default and maximum sizes for `recallByGraph` result sets. */
export const GRAPH_RECALL_DEFAULT_LIMIT = 50;
export const GRAPH_RECALL_MAX_LIMIT = 500;

export interface ReasoningBank {
  /**
   * Embed `queryText` (or `entry.prompt_summary` if omitted) and persist the
   * full entry for later recall. The supplied `entry.embedding` is ignored —
   * the bank computes the vector itself for consistency.
   */
  index(
    entry: Omit<ReasoningEntry, "embedding">,
    queryText?: string,
  ): Promise<void>;
  recallSimilar(query: string, opts?: RecallOptions): Promise<ReasoningEntry[]>;
  /**
   * Structured-metadata recall. Filters across `metadata`, `hasTools`,
   * `hasDelegations`, `agent_id`, `since`, `until`, with `limit` clamping.
   * Returns entries sorted by `timestamp` descending (most recent first).
   */
  recallByGraph(query: GraphRecallQuery): Promise<ReasoningEntry[]>;
  close(): Promise<void>;
}

/**
 * Factory: open a RuVector-backed reasoning bank rooted at `opts.dir`.
 *
 * The directory is created on demand. Repeated calls against the same
 * directory share the same on-disk store (RuVector itself handles concurrent
 * access internally).
 */
export async function createRuVectorBank(
  opts: ReasoningBankOptions,
): Promise<ReasoningBank> {
  const dimensions = opts.dimensions ?? 384;
  const index: RVIndex = await openRVIndex({
    dir: opts.dir,
    dimensions,
  });

  return {
    async index(
      entry: Omit<ReasoningEntry, "embedding">,
      queryText?: string,
    ): Promise<void> {
      const text = queryText ?? entry.prompt_summary;
      const embedding = await opts.embed(text);
      const full: ReasoningEntry = { ...entry, embedding };
      await index.insert(full);
    },

    async recallSimilar(
      query: string,
      recallOpts?: RecallOptions,
    ): Promise<ReasoningEntry[]> {
      const k = recallOpts?.k ?? 5;
      const vec = await opts.embed(query);
      return index.search(vec, k, recallOpts?.filter);
    },

    async recallByGraph(
      query: GraphRecallQuery,
    ): Promise<ReasoningEntry[]> {
      const limit = Math.min(
        GRAPH_RECALL_MAX_LIMIT,
        Math.max(1, Math.floor(query.limit ?? GRAPH_RECALL_DEFAULT_LIMIT)),
      );

      // Push the cheap top-level filters (`agent_id`, `since`, `until`) into
      // the index scan so we don't materialize the whole corpus when callers
      // narrow by them. The richer filters (metadata equality, hasTools,
      // hasDelegations) are applied here in JS — RVIndex's storage layer is a
      // flat key/value blob and doesn't index those fields.
      const scanFilter: { agent_id?: string; since?: Date; until?: Date } = {};
      if (query.agent_id !== undefined) scanFilter.agent_id = query.agent_id;
      if (query.since !== undefined) scanFilter.since = query.since;
      if (query.until !== undefined) scanFilter.until = query.until;

      // Over-fetch when post-filters are present so we don't return short
      // results just because the first N scan hits failed the metadata filter.
      const hasPostFilter =
        (query.metadata !== undefined &&
          Object.keys(query.metadata).length > 0) ||
        (query.hasTools !== undefined && query.hasTools.length > 0) ||
        (query.hasDelegations !== undefined && query.hasDelegations.length > 0);
      const fetchLimit = hasPostFilter
        ? Math.min(GRAPH_RECALL_MAX_LIMIT * 4, limit * 8)
        : limit;

      const candidates = await index.scan(scanFilter, fetchLimit);

      const out: ReasoningEntry[] = [];
      for (const entry of candidates) {
        if (query.metadata !== undefined) {
          let metaOk = true;
          for (const [mk, mv] of Object.entries(query.metadata)) {
            if (entry.metadata[mk] !== mv) {
              metaOk = false;
              break;
            }
          }
          if (!metaOk) continue;
        }
        if (query.hasTools !== undefined && query.hasTools.length > 0) {
          const want = new Set(query.hasTools);
          const hit = entry.tool_calls.some((tc) => want.has(tc.name));
          if (!hit) continue;
        }
        if (
          query.hasDelegations !== undefined &&
          query.hasDelegations.length > 0
        ) {
          const want = new Set(query.hasDelegations);
          const hit = entry.delegations.some((d) => want.has(d));
          if (!hit) continue;
        }
        out.push(entry);
        if (out.length >= limit) break;
      }

      // Sort by timestamp descending; ties broken by audit_id for determinism.
      out.sort((a, b) => {
        if (a.timestamp !== b.timestamp) {
          return a.timestamp < b.timestamp ? 1 : -1;
        }
        return a.audit_id < b.audit_id ? 1 : -1;
      });

      return out;
    },

    async close(): Promise<void> {
      await index.close();
    },
  };
}
