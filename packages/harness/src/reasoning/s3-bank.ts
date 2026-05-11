/**
 * S3ReasoningBank — Phase 41 Wave 0.
 *
 * An implementation of the `ReasoningBank` interface backed by S3-compatible
 * object storage. Works with AWS S3 in production and MinIO locally (same
 * `@aws-sdk/client-s3` client, different `endpoint` config).
 *
 * Object layout:
 *   <keyPrefix>/index.jsonl           — append-only newline-delimited JSON of all entries
 *   <keyPrefix>/<agent_id>/<ts>-<audit_id>.json — canonical full entry per dispatch
 *
 * Concurrency model: optimistic ETag-based concurrency on `index.jsonl`.
 *   - GET index.jsonl (capture ETag)
 *   - Append new index record
 *   - PUT with `IfMatch: <ETag>` (412 on conflict → retry)
 *   - First write uses `IfNoneMatch: "*"` (create-if-absent)
 *
 * The index record is a compact subset of ReasoningEntry (all fields except
 * the full payload are included — embedding IS stored for recallSimilar).
 *
 * This file does NOT modify the existing RuVector bank (rv-index.ts / bank.ts).
 * S3 is opt-in via CFA_BANK_BACKEND=s3.
 */
import {
  GetObjectCommand,
  PutObjectCommand,
  S3Client,
  type S3ClientConfig,
} from "@aws-sdk/client-s3";
import type {
  GraphRecallQuery,
  ReasoningBank,
  ReasoningEntry,
  RecallOptions,
} from "./bank.js";
import {
  GRAPH_RECALL_DEFAULT_LIMIT,
  GRAPH_RECALL_MAX_LIMIT,
} from "./bank.js";

export interface S3ReasoningBankOptions {
  /** S3 bucket name. */
  bucket: string;
  /**
   * Override endpoint for MinIO (e.g., http://localhost:9000).
   * Defaults to standard AWS endpoint resolution.
   */
  endpoint?: string;
  /** AWS region. Defaults to us-east-1 (MinIO ignores this). */
  region?: string;
  /** Object key prefix. Defaults to "cfa-bank". */
  keyPrefix?: string;
  /** Max retries on ETag conflicts when appending to index.jsonl. Default 5. */
  maxConcurrencyRetries?: number;
  /** Force path-style addressing (required for MinIO; harmless for AWS). Default true. */
  forcePathStyle?: boolean;
  /** Optional pre-built S3 client (for tests). */
  client?: S3Client;
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/** Read the current index.jsonl body + ETag. Returns null body when absent. */
async function getIndex(
  client: S3Client,
  bucket: string,
  key: string,
): Promise<{ body: string; etag: string } | null> {
  try {
    const resp = await client.send(
      new GetObjectCommand({ Bucket: bucket, Key: key }),
    );
    const etag = resp.ETag ?? "";
    const body = resp.Body
      ? await resp.Body.transformToString("utf-8")
      : "";
    return { body, etag };
  } catch (err) {
    // NoSuchKey — bucket exists but index hasn't been created yet.
    const code = (err as { name?: string; Code?: string }).name ??
      (err as { Code?: string }).Code ?? "";
    if (code === "NoSuchKey" || code === "NotFound") {
      return null;
    }
    throw err;
  }
}

/** Write the full entry as a standalone object for canonical record. */
async function putEntryObject(
  client: S3Client,
  bucket: string,
  key: string,
  entry: ReasoningEntry,
): Promise<void> {
  await client.send(
    new PutObjectCommand({
      Bucket: bucket,
      Key: key,
      Body: JSON.stringify(entry),
      ContentType: "application/json",
    }),
  );
}

/**
 * Conditional PUT of `newBody` to `key`.
 *   - `etag` provided → IfMatch (update existing)
 *   - `etag` is null → IfNoneMatch "*" (create-if-absent)
 * Returns true on success, false on 412 / PreconditionFailed.
 */
async function conditionalPutIndex(
  client: S3Client,
  bucket: string,
  key: string,
  newBody: string,
  etag: string | null,
): Promise<boolean> {
  try {
    const cmd = new PutObjectCommand({
      Bucket: bucket,
      Key: key,
      Body: newBody,
      ContentType: "application/x-ndjson",
      ...(etag !== null
        ? { IfMatch: etag }
        : { IfNoneMatch: "*" }),
    });
    await client.send(cmd);
    return true;
  } catch (err) {
    const code = (err as { name?: string; $metadata?: { httpStatusCode?: number } }).name ?? "";
    const status = (err as { $metadata?: { httpStatusCode?: number } }).$metadata?.httpStatusCode;
    if (
      code === "PreconditionFailed" ||
      code === "ConditionalRequestConflict" ||
      status === 412
    ) {
      return false;
    }
    throw err;
  }
}

/** Parse an index.jsonl body into ReasoningEntry[]. Tolerates malformed lines. */
function parseIndexBody(body: string): ReasoningEntry[] {
  const entries: ReasoningEntry[] = [];
  for (const line of body.split("\n")) {
    const trimmed = line.trim();
    if (!trimmed) continue;
    try {
      entries.push(JSON.parse(trimmed) as ReasoningEntry);
    } catch {
      // Tolerate truncated/corrupt tail lines — best-effort.
    }
  }
  return entries;
}

/** Cosine similarity between two vectors. Returns 0 when either has zero norm. */
function cosineSimilarity(a: number[], b: number[]): number {
  if (a.length !== b.length || a.length === 0) return 0;
  let dot = 0, normA = 0, normB = 0;
  for (let i = 0; i < a.length; i++) {
    // safe: i is always in bounds
    const ai = a[i] as number;
    const bi = b[i] as number;
    dot += ai * bi;
    normA += ai * ai;
    normB += bi * bi;
  }
  const denom = Math.sqrt(normA) * Math.sqrt(normB);
  return denom === 0 ? 0 : dot / denom;
}

// ---------------------------------------------------------------------------
// Factory
// ---------------------------------------------------------------------------

/**
 * Create an S3-backed `ReasoningBank`. Compatible with AWS S3 and MinIO.
 *
 * The returned object satisfies the `ReasoningBank` interface. It is
 * stateless (no in-memory cache) — every `recallSimilar` / `recallByGraph`
 * reads the full `index.jsonl` from S3.
 */
export function createS3ReasoningBank(opts: S3ReasoningBankOptions): ReasoningBank {
  const {
    bucket,
    keyPrefix = "cfa-bank",
    maxConcurrencyRetries = 5,
    forcePathStyle = true,
  } = opts;

  const clientConfig: S3ClientConfig = {
    region: opts.region ?? "us-east-1",
    forcePathStyle,
  };
  if (opts.endpoint !== undefined) {
    clientConfig.endpoint = opts.endpoint;
  }

  const client: S3Client = opts.client ?? new S3Client(clientConfig);
  const indexKey = `${keyPrefix}/index.jsonl`;

  return {
    async index(
      entry: Omit<ReasoningEntry, "embedding">,
      _queryText?: string,
    ): Promise<void> {
      // The embed fn is baked into the bank at the ReasoningBank layer above us.
      // For the S3 bank the caller must pass the embedding in the entry OR we
      // require an embed function. However, looking at bank.ts the interface
      // says the bank ignores entry.embedding and computes it itself. The S3 bank
      // doesn't hold an embed function reference in its opts — callers provide
      // an embed fn at the ReasoningBank factory level, which wraps this.
      //
      // For the S3 bank, we mirror the RuVector bank's approach: the `index()`
      // signature accepts `Omit<ReasoningEntry, "embedding">` and an optional
      // `queryText`. The embedding must already be computed and injected before
      // this call is made OR the caller passes a full `ReasoningEntry`. We align
      // with the factory pattern used in bank.ts where createRuVectorBank wraps
      // an embed fn. For S3, `createS3ReasoningBank` is a lower-level factory
      // and the embed step is done externally (see indexer.ts caller pattern).
      //
      // In practice, the indexer calls bank.index(entryWithoutEmbedding, text)
      // and the bank impl computes the embedding. Since S3Bank doesn't have an
      // embed fn, we model it as: embedding is provided by the wrapper. Looking
      // at usage, the caller is expected to pass a full entry including embedding
      // when using the S3 bank directly. We store what we receive.
      //
      // Deviation from spec: The spec says "bank computes embedding" but the S3
      // bank factory has no embed fn parameter. We follow the same pattern as
      // createRuVectorBank — the embedding is provided as a side-channel via
      // the full entry record. The S3Bank wraps createS3ReasoningBank with an
      // embed fn at a higher level (see createS3ReasoningBankWithEmbed below).
      //
      // For the direct factory, we store the entry as-is with a zero embedding
      // placeholder if no embedding field is provided, preserving interface compat.
      const withEmbedding = entry as ReasoningEntry;

      // Validate embedding dimension consistency.
      // If embedding is present and dimensioned, we trust it. If absent, we
      // use empty array (callers should use createS3ReasoningBankWithEmbed).
      if (!Array.isArray(withEmbedding.embedding)) {
        (withEmbedding as { embedding: number[] }).embedding = [];
      }

      // 1. Write canonical entry object.
      const ts = withEmbedding.timestamp.replace(/[:.]/g, "-");
      const entryKey = `${keyPrefix}/${withEmbedding.agent_id}/${ts}-${withEmbedding.audit_id}.json`;
      await putEntryObject(client, bucket, entryKey, withEmbedding);

      // 2. Append to index.jsonl with ETag-based optimistic concurrency.
      const indexLine = JSON.stringify(withEmbedding);
      let retries = 0;

      while (true) {
        const current = await getIndex(client, bucket, indexKey);
        const etag = current?.etag ?? null;
        const currentBody = current?.body ?? "";
        const newBody = currentBody
          ? currentBody.trimEnd() + "\n" + indexLine + "\n"
          : indexLine + "\n";

        const ok = await conditionalPutIndex(client, bucket, indexKey, newBody, etag);
        if (ok) return;

        retries++;
        if (retries >= maxConcurrencyRetries) {
          throw new Error(
            `S3ReasoningBank.index: ETag conflict after ${retries} retries. ` +
              `Key: ${indexKey}, audit_id: ${withEmbedding.audit_id}`,
          );
        }
      }
    },

    async recallSimilar(query: string, opts?: RecallOptions): Promise<ReasoningEntry[]> {
      // query is a string — the embed step should have been done by the wrapper.
      // For the raw S3 bank, we cannot embed the query without an embed fn.
      // This method is intended to be called via the embed-wrapping factory.
      // When called directly (e.g., in tests with mock embeddings), callers
      // pass the query string. We need a queryVector — without an embed fn we
      // cannot compute one. Returning empty array is wrong; throwing is better.
      //
      // Resolution: the query is a string here matching the ReasoningBank
      // interface. The S3 bank stores full embeddings in index.jsonl. Without
      // an embed fn we cannot compute the query vector. The spec says the bank
      // computes the vector itself — but createS3ReasoningBank doesn't accept
      // an embed fn. This is the same limitation as noted above.
      //
      // For tests we use createS3ReasoningBankWithEmbed (below) which wraps
      // this and does the embedding before delegating to recallSimilar.
      // This path should not be called without an embed fn in production.
      void query; // suppress unused param warning
      void opts;
      throw new Error(
        "S3ReasoningBank.recallSimilar: use createS3ReasoningBankWithEmbed " +
          "to get a bank that can embed query strings. Direct recallSimilar " +
          "on createS3ReasoningBank is not supported.",
      );
    },

    async recallByGraph(query: GraphRecallQuery): Promise<ReasoningEntry[]> {
      const current = await getIndex(client, bucket, indexKey);
      if (!current) return [];
      const entries = parseIndexBody(current.body);
      return filterByGraph(entries, query);
    },

    async close(): Promise<void> {
      // S3Client has no close method; nothing to do.
    },
  };
}

// ---------------------------------------------------------------------------
// Embed-wrapping factory (full ReasoningBank with embed fn)
// ---------------------------------------------------------------------------

import type { EmbeddingFn } from "./embeddings.js";

export interface S3ReasoningBankWithEmbedOptions extends S3ReasoningBankOptions {
  /** Embedding function — same as ReasoningBankOptions.embed. */
  embed: EmbeddingFn;
  /** Vector dimension — for validation. Defaults to 384. */
  dimensions?: number;
}

/**
 * Full S3 ReasoningBank factory that includes the embedding function.
 * This satisfies the complete `ReasoningBank` contract including `recallSimilar`.
 */
export function createS3ReasoningBankWithEmbed(
  opts: S3ReasoningBankWithEmbedOptions,
): ReasoningBank {
  const {
    bucket,
    keyPrefix = "cfa-bank",
    maxConcurrencyRetries = 5,
    forcePathStyle = true,
    embed,
    dimensions = 384,
  } = opts;

  const clientConfig: S3ClientConfig = {
    region: opts.region ?? "us-east-1",
    forcePathStyle,
  };
  if (opts.endpoint !== undefined) {
    clientConfig.endpoint = opts.endpoint;
  }

  const client: S3Client = opts.client ?? new S3Client(clientConfig);
  const indexKey = `${keyPrefix}/index.jsonl`;

  return {
    async index(
      entry: Omit<ReasoningEntry, "embedding">,
      queryText?: string,
    ): Promise<void> {
      const text = queryText ?? entry.prompt_summary;
      const embedding = await embed(text);

      if (embedding.length !== dimensions) {
        throw new Error(
          `S3ReasoningBank.index: embedding length ${embedding.length} ≠ expected ${dimensions}`,
        );
      }

      const full: ReasoningEntry = { ...entry, embedding };
      const ts = full.timestamp.replace(/[:.]/g, "-");
      const entryKey = `${keyPrefix}/${full.agent_id}/${ts}-${full.audit_id}.json`;
      await putEntryObject(client, bucket, entryKey, full);

      const indexLine = JSON.stringify(full);
      let retries = 0;

      while (true) {
        const current = await getIndex(client, bucket, indexKey);
        const etag = current?.etag ?? null;
        const currentBody = current?.body ?? "";
        const newBody = currentBody
          ? currentBody.trimEnd() + "\n" + indexLine + "\n"
          : indexLine + "\n";

        const ok = await conditionalPutIndex(client, bucket, indexKey, newBody, etag);
        if (ok) return;

        retries++;
        if (retries >= maxConcurrencyRetries) {
          throw new Error(
            `S3ReasoningBank.index: ETag conflict after ${retries} retries for audit_id: ${full.audit_id}`,
          );
        }
      }
    },

    async recallSimilar(query: string, recallOpts?: RecallOptions): Promise<ReasoningEntry[]> {
      const k = recallOpts?.k ?? 5;
      const queryVector = await embed(query);
      const current = await getIndex(client, bucket, indexKey);
      if (!current) return [];

      const entries = parseIndexBody(current.body);

      // Apply pre-filter (agent_id, since) before ranking.
      const filter = recallOpts?.filter;
      const candidates = entries.filter((e) => {
        if (filter?.agent_id !== undefined && e.agent_id !== filter.agent_id) {
          return false;
        }
        if (filter?.since !== undefined) {
          const ts = new Date(e.timestamp).getTime();
          if (Number.isNaN(ts) || ts < filter.since.getTime()) return false;
        }
        if (filter?.metadata !== undefined) {
          for (const [mk, mv] of Object.entries(filter.metadata)) {
            if (e.metadata[mk] !== mv) return false;
          }
        }
        return true;
      });

      // Score by cosine similarity and return top-k.
      const scored = candidates
        .map((e) => ({ entry: e, score: cosineSimilarity(queryVector, e.embedding) }))
        .sort((a, b) => b.score - a.score);

      return scored.slice(0, k).map((s) => s.entry);
    },

    async recallByGraph(query: GraphRecallQuery): Promise<ReasoningEntry[]> {
      const current = await getIndex(client, bucket, indexKey);
      if (!current) return [];
      const entries = parseIndexBody(current.body);
      return filterByGraph(entries, query);
    },

    async close(): Promise<void> {
      // S3Client has no close; nothing to do.
    },
  };
}

// ---------------------------------------------------------------------------
// Shared graph filter
// ---------------------------------------------------------------------------

function filterByGraph(
  entries: ReasoningEntry[],
  query: GraphRecallQuery,
): ReasoningEntry[] {
  const limit = Math.min(
    GRAPH_RECALL_MAX_LIMIT,
    Math.max(1, Math.floor(query.limit ?? GRAPH_RECALL_DEFAULT_LIMIT)),
  );
  const sinceMs = query.since?.getTime();
  const untilMs = query.until?.getTime();

  const out: ReasoningEntry[] = [];
  for (const e of entries) {
    if (query.agent_id !== undefined && e.agent_id !== query.agent_id) continue;
    if (sinceMs !== undefined) {
      const ts = new Date(e.timestamp).getTime();
      if (Number.isNaN(ts) || ts < sinceMs) continue;
    }
    if (untilMs !== undefined) {
      const ts = new Date(e.timestamp).getTime();
      if (Number.isNaN(ts) || ts > untilMs) continue;
    }
    if (query.metadata !== undefined) {
      let ok = true;
      for (const [mk, mv] of Object.entries(query.metadata)) {
        if (e.metadata[mk] !== mv) { ok = false; break; }
      }
      if (!ok) continue;
    }
    if (query.hasTools !== undefined && query.hasTools.length > 0) {
      const want = new Set(query.hasTools);
      if (!e.tool_calls.some((tc) => want.has(tc.name))) continue;
    }
    if (query.hasDelegations !== undefined && query.hasDelegations.length > 0) {
      const want = new Set(query.hasDelegations);
      if (!e.delegations.some((d) => want.has(d))) continue;
    }
    out.push(e);
    if (out.length >= limit) break;
  }

  // Sort by timestamp descending; ties broken by audit_id for determinism.
  out.sort((a, b) => {
    if (a.timestamp !== b.timestamp) return a.timestamp < b.timestamp ? 1 : -1;
    return a.audit_id < b.audit_id ? 1 : -1;
  });

  return out;
}
