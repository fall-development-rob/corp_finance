/**
 * ReasoningBank — port + RuVector-backed implementation.
 *
 * The bank wraps an `RVIndex` with the embedding step factored out: callers
 * pass a `ReasoningBankOptions.embed` function and the bank produces vectors
 * lazily on `index()` and `recallSimilar()`. This keeps embedding choice
 * (OpenAI / deterministic / future Voyage / future local) orthogonal to
 * vector storage.
 *
 * `recallByGraph` is a Wave 4 surface; in Wave 1 it throws.
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
  /** Cypher-style graph queries. Wave 4 — currently throws. */
  recallByGraph(query: string): Promise<ReasoningEntry[]>;
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

    async recallByGraph(_query: string): Promise<ReasoningEntry[]> {
      throw new Error(
        "ReasoningBank.recallByGraph: not implemented in Wave 1 (planned for Wave 4)",
      );
    },

    async close(): Promise<void> {
      await index.close();
    },
  };
}
