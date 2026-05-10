/**
 * Thin wrapper over the `ruvector` npm package (NAPI-RS native bindings).
 *
 * Exposes only what the Reasoning Bank needs: insert, search with optional
 * metadata + time + agent filtering, and close. RuVector internals
 * (Float32Array conversion, JSON-string metadata, distance metric naming) are
 * hidden so the bank doesn't bind to library-specific quirks.
 *
 * RuVector quirks adapted to:
 *   - The `ruvector` package is CJS (`require()`-based); we import via the
 *     default-interop ESM bridge that TypeScript's bundler resolution gives
 *     us.
 *   - The native `VectorDb` wrapper accepts `storagePath` (file, not
 *     directory) — we pass `<dir>/store.rvf`.
 *   - Cosine "score" is a distance: 0 = identical, 1 = orthogonal. Lower is
 *     more similar. We pass results through verbatim and let the bank /
 *     callers reason about ordering (results are already returned ascending).
 *   - Metadata `filter` only supports equality on flat keys. `since` and any
 *     other range-based filter is applied post-search by reading entries
 *     back via `metadata.timestamp`. We over-fetch by a small factor when a
 *     non-equality filter is in play.
 *   - There is no `close()` on the underlying VectorDb. Persistence is via
 *     `storagePath`; closing is implicit when references drop. Our `close()`
 *     is a no-op, retained for symmetry with `AuditSink` / `SessionStore`.
 *
 * Wave 4 addition — JSONL scan sidecar. The published `ruvector` 0.2.25
 * VectorDB wrapper exposes `insert`, `search`, `get(id)`, `delete`, and
 * `len` only — there is NO API to enumerate the entries already in the
 * store, and no Cypher/graph surface yet (the GitHub README documents both
 * as roadmap items). To support `recallByGraph`'s structured-metadata
 * queries we mirror every `insert` to an append-only JSONL file at
 * `<dir>/scan.jsonl`. `scan()` reads the JSONL and reconstructs entries.
 * Search is unchanged (still served by VectorDB). The sidecar is a derived
 * artefact — if it is deleted, `recallByGraph` returns empty until new
 * entries are indexed, but `recallSimilar` still works because the vector
 * store is intact. Operators can rebuild the sidecar from the audit chain
 * via a future `--rebuild-reasoning` CLI command (see ADR-040).
 */
import { appendFile, mkdir, readFile } from "node:fs/promises";
import { join } from "node:path";
import { VectorDB } from "ruvector";
import type { ReasoningEntry } from "./bank.js";

/** The dimension we expect throughout Wave 1 — derived from the deterministic embedder default. */
const DEFAULT_DIMENSIONS = 384;

export interface RVIndexOptions {
  /** Directory the `.rvf` store lives in. Created if missing. */
  dir: string;
  /**
   * Vector dimension. Must match the embedder's output. Defaults to 384
   * (deterministic embedder). For OpenAI `text-embedding-3-small`, pass 1536.
   */
  dimensions?: number;
}

export interface RVSearchFilter {
  agent_id?: string;
  since?: Date;
  metadata?: Record<string, unknown>;
}

/**
 * Filter accepted by `RVIndex.scan`. Only flat top-level predicates appear
 * here — richer filters (metadata equality, has-tool, has-delegation) live
 * one layer up in `ReasoningBank.recallByGraph` so the storage shim stays
 * narrow.
 */
export interface RVScanFilter {
  agent_id?: string;
  since?: Date;
  until?: Date;
}

export interface RVIndex {
  insert(entry: ReasoningEntry): Promise<void>;
  search(
    queryVector: number[],
    k: number,
    filter?: RVSearchFilter,
  ): Promise<ReasoningEntry[]>;
  /**
   * Wave 4 — vector-free scan over the indexed entries. Backed by an
   * append-only JSONL sidecar (see file header). Returns up to `limit`
   * entries that match `filter`, in insertion order. Caller handles further
   * sorting / filtering.
   */
  scan(filter?: RVScanFilter, limit?: number): Promise<ReasoningEntry[]>;
  close(): Promise<void>;
}

/** What we serialize into RuVector metadata. */
interface StoredMetadata {
  audit_id: string;
  agent_id: string;
  prompt_hash: string;
  prompt_summary: string;
  tool_calls: { name: string; count: number }[];
  delegations: string[];
  result_excerpt: string;
  metadata: Record<string, unknown>;
  timestamp: string;
}

/** Reconstruct a ReasoningEntry from a search result. */
function reconstruct(
  meta: StoredMetadata | undefined,
  vector: Float32Array | number[] | undefined,
): ReasoningEntry | null {
  if (!meta) return null;
  let embedding: number[];
  if (vector instanceof Float32Array) {
    embedding = Array.from(vector);
  } else if (Array.isArray(vector)) {
    embedding = vector.slice();
  } else {
    // RuVector returned an object-shaped array (numeric-keyed) — flatten.
    embedding = Object.values((vector ?? {}) as Record<string, number>);
  }
  return {
    audit_id: meta.audit_id,
    agent_id: meta.agent_id,
    prompt_hash: meta.prompt_hash,
    prompt_summary: meta.prompt_summary,
    embedding,
    tool_calls: meta.tool_calls ?? [],
    delegations: meta.delegations ?? [],
    result_excerpt: meta.result_excerpt ?? "",
    metadata: meta.metadata ?? {},
    timestamp: meta.timestamp,
  };
}

/**
 * Open (or create) a RuVector-backed index in `<dir>/store.rvf`.
 *
 * The native NAPI binary loads lazily on first call; if it fails we surface
 * the error rather than falling back to the stub implementation.
 */
export async function openRVIndex(opts: RVIndexOptions): Promise<RVIndex> {
  await mkdir(opts.dir, { recursive: true });
  const storagePath = join(opts.dir, "store.rvf");
  const scanSidecarPath = join(opts.dir, "scan.jsonl");
  const dimensions = opts.dimensions ?? DEFAULT_DIMENSIONS;

  const db = new VectorDB({
    dimensions,
    storagePath,
    distanceMetric: "cosine",
  });

  let closed = false;

  return {
    async insert(entry: ReasoningEntry): Promise<void> {
      if (closed) {
        throw new Error("RVIndex: insert called after close()");
      }
      if (entry.embedding.length !== dimensions) {
        throw new Error(
          `RVIndex.insert: embedding length ${entry.embedding.length} ≠ index dim ${dimensions}`,
        );
      }
      const stored: StoredMetadata = {
        audit_id: entry.audit_id,
        agent_id: entry.agent_id,
        prompt_hash: entry.prompt_hash,
        prompt_summary: entry.prompt_summary,
        tool_calls: entry.tool_calls,
        delegations: entry.delegations,
        result_excerpt: entry.result_excerpt,
        metadata: entry.metadata,
        timestamp: entry.timestamp,
      };
      await db.insert({
        id: entry.audit_id,
        vector: entry.embedding,
        metadata: stored as unknown as Record<string, unknown>,
      });
      // Wave 4: mirror the entry to the JSONL scan sidecar so
      // `recallByGraph` can iterate without a query vector. We persist the
      // SAME `StoredMetadata` shape as the vector store, plus an `embedding`
      // field, so reconstruction is symmetric. Append is atomic for a
      // single-process scenario (Node fs.appendFile uses O_APPEND).
      const sidecarLine =
        JSON.stringify({
          ...stored,
          embedding: entry.embedding,
        }) + "\n";
      await appendFile(scanSidecarPath, sidecarLine, { encoding: "utf8" });
    },

    async search(
      queryVector: number[],
      k: number,
      filter?: RVSearchFilter,
    ): Promise<ReasoningEntry[]> {
      if (closed) {
        throw new Error("RVIndex: search called after close()");
      }
      if (queryVector.length !== dimensions) {
        throw new Error(
          `RVIndex.search: query length ${queryVector.length} ≠ index dim ${dimensions}`,
        );
      }

      // RuVector filter only supports flat equality. We forward `agent_id`
      // (very common) and any caller-supplied flat metadata keys; `since` and
      // anything else is applied post-search.
      const rvFilter: Record<string, unknown> = {};
      if (filter?.agent_id !== undefined) {
        rvFilter.agent_id = filter.agent_id;
      }
      // Flat metadata equality keys go through the native filter directly,
      // BUT only at the StoredMetadata top level. Since our user metadata is
      // nested under `metadata.*` we cannot delegate that to the native
      // filter; apply post-fetch instead.
      const hasEqualityFilter = Object.keys(rvFilter).length > 0;
      const hasPostFilter =
        filter?.since !== undefined ||
        (filter?.metadata !== undefined && Object.keys(filter.metadata).length > 0);

      // Over-fetch when post-filtering so we don't return short results.
      const fetchK = hasPostFilter ? Math.max(k * 4, k + 16) : k;

      const results = await db.search({
        vector: queryVector,
        k: fetchK,
        ...(hasEqualityFilter ? { filter: rvFilter } : {}),
      });

      const entries: ReasoningEntry[] = [];
      for (const r of results) {
        const entry = reconstruct(r.metadata as StoredMetadata | undefined, r.vector);
        if (!entry) continue;

        if (filter?.since !== undefined) {
          const ts = new Date(entry.timestamp).getTime();
          if (Number.isNaN(ts) || ts < filter.since.getTime()) continue;
        }
        if (filter?.metadata !== undefined) {
          let metaOk = true;
          for (const [mk, mv] of Object.entries(filter.metadata)) {
            if (entry.metadata[mk] !== mv) {
              metaOk = false;
              break;
            }
          }
          if (!metaOk) continue;
        }

        entries.push(entry);
        if (entries.length >= k) break;
      }
      return entries;
    },

    async scan(
      filter?: RVScanFilter,
      limit?: number,
    ): Promise<ReasoningEntry[]> {
      if (closed) {
        throw new Error("RVIndex: scan called after close()");
      }
      // No sidecar yet → empty index from the scan perspective.
      let raw: string;
      try {
        raw = await readFile(scanSidecarPath, "utf8");
      } catch (err) {
        if (
          (err as NodeJS.ErrnoException).code === "ENOENT" ||
          (err as { code?: string }).code === "ENOENT"
        ) {
          return [];
        }
        throw err;
      }

      const cap =
        typeof limit === "number" && limit > 0
          ? Math.floor(limit)
          : Number.POSITIVE_INFINITY;
      const out: ReasoningEntry[] = [];
      const sinceMs = filter?.since?.getTime();
      const untilMs = filter?.until?.getTime();

      // Iterate lines newest-first by reversing — JSONL append order is
      // chronological, callers typically want recency. (Caller can sort
      // again; we just bias the early-exit toward the recent end so a tight
      // `limit` returns the most recent matches.)
      const lines = raw.split("\n");
      for (let i = lines.length - 1; i >= 0; i--) {
        const line = lines[i];
        if (!line || line.length === 0) continue;
        let parsed: (StoredMetadata & { embedding: number[] }) | null = null;
        try {
          parsed = JSON.parse(line) as StoredMetadata & { embedding: number[] };
        } catch {
          // Tolerate a truncated tail line — sidecar is best-effort.
          continue;
        }
        if (!parsed) continue;

        if (filter?.agent_id !== undefined && parsed.agent_id !== filter.agent_id) {
          continue;
        }
        if (sinceMs !== undefined) {
          const ts = new Date(parsed.timestamp).getTime();
          if (Number.isNaN(ts) || ts < sinceMs) continue;
        }
        if (untilMs !== undefined) {
          const ts = new Date(parsed.timestamp).getTime();
          if (Number.isNaN(ts) || ts > untilMs) continue;
        }

        const entry = reconstruct(parsed, parsed.embedding);
        if (!entry) continue;
        out.push(entry);
        if (out.length >= cap) break;
      }

      return out;
    },

    async close(): Promise<void> {
      // RuVector has no explicit close; flag locally so subsequent ops fail loud.
      closed = true;
    },
  };
}
