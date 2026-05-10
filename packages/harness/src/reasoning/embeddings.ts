/**
 * Pluggable embedding functions for the Reasoning Bank.
 *
 * Phase 34 Wave 1 ships two implementations:
 *
 *  1. `createOpenAIEmbedder` — real OpenAI `text-embedding-3-small` calls
 *     gated behind an explicit API key, with sha256-keyed in-memory cache.
 *
 *  2. `createDeterministicEmbedder` — sha256-derived stable projection used
 *     by unit tests. No network, no API key, identical output for identical
 *     input. Different inputs yield different unit vectors with overwhelming
 *     probability (collision space ≈ 2^256).
 *
 * The OpenAI embedder MUST NOT be invoked from any unit test that runs by
 * default. Tests that exercise the cache use an injectable `fetchImpl`.
 */
import { createHash } from "node:crypto";

/**
 * The single function shape the Reasoning Bank consumes. All future
 * embedding providers (Voyage, Anthropic if/when shipped, local ONNX) plug
 * in here.
 */
export type EmbeddingFn = (text: string) => Promise<number[]>;

// ---------------------------------------------------------------------------
// OpenAI embeddings — real, network-bound, cached.
// ---------------------------------------------------------------------------

export interface OpenAIEmbedderOptions {
  apiKey: string;
  /** Defaults to `"text-embedding-3-small"` (1536 dims). */
  model?: string;
  /**
   * Optional shared cache. Keyed by `sha256(text)`. Pass an external Map to
   * survive across embedder instances; omit to use a fresh per-instance cache.
   */
  cache?: Map<string, number[]>;
  /**
   * Fetch implementation override (for testing). Defaults to the global
   * `fetch`. Tests inject a mock to assert HTTP-call cardinality.
   */
  fetchImpl?: typeof fetch;
}

/** sha256 hex of utf-8 bytes; stable across Node versions and platforms. */
function sha256Hex(text: string): string {
  return createHash("sha256").update(text, "utf-8").digest("hex");
}

/**
 * Real embedder hitting `https://api.openai.com/v1/embeddings`.
 *
 * Each `(text)` invocation:
 *   1. Hashes the input.
 *   2. Returns the cached vector if hit.
 *   3. Otherwise POSTs `{ input, model }` to OpenAI, caches the result, returns it.
 *
 * Single text per request; batching is intentionally out of scope for Wave 1.
 */
export function createOpenAIEmbedder(opts: OpenAIEmbedderOptions): EmbeddingFn {
  if (!opts.apiKey) {
    throw new Error("createOpenAIEmbedder: apiKey is required");
  }
  const model = opts.model ?? "text-embedding-3-small";
  const cache = opts.cache ?? new Map<string, number[]>();
  const fetchImpl = opts.fetchImpl ?? globalThis.fetch;
  if (typeof fetchImpl !== "function") {
    throw new Error(
      "createOpenAIEmbedder: global fetch is unavailable; pass fetchImpl explicitly",
    );
  }

  return async function embed(text: string): Promise<number[]> {
    const key = sha256Hex(text);
    const cached = cache.get(key);
    if (cached !== undefined) {
      return cached;
    }

    const response = await fetchImpl("https://api.openai.com/v1/embeddings", {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        Authorization: `Bearer ${opts.apiKey}`,
      },
      body: JSON.stringify({ input: text, model }),
    });

    if (!response.ok) {
      const body = await response.text().catch(() => "<unreadable>");
      throw new Error(
        `OpenAI embeddings API failed: ${response.status} ${response.statusText}: ${body}`,
      );
    }

    const json = (await response.json()) as {
      data?: { embedding?: number[] }[];
    };
    const vec = json.data?.[0]?.embedding;
    if (!Array.isArray(vec)) {
      throw new Error("OpenAI embeddings API: malformed response (no data[0].embedding)");
    }

    cache.set(key, vec);
    return vec;
  };
}

// ---------------------------------------------------------------------------
// Deterministic embedder — hash-based projection, test-only.
// ---------------------------------------------------------------------------

export interface DeterministicEmbedderOptions {
  /** Defaults to 384. Any positive integer is permitted. */
  dim?: number;
}

/**
 * Stable, no-network embedder for tests and offline development.
 *
 * Algorithm:
 *   1. Take sha256(text) — 32 bytes, fully determined by input.
 *   2. Stretch to `dim` floats by re-hashing `sha256(text + ":" + i)` per
 *      8-byte block until the buffer is filled. Each byte maps to a
 *      centred float `(b - 127.5) / 127.5` in [-1, 1).
 *   3. Normalize to unit length so cosine distance behaves predictably.
 *
 * Identical input → identical output. Different inputs collide only if
 * sha256 collides, which we treat as never.
 */
export function createDeterministicEmbedder(
  opts: DeterministicEmbedderOptions = {},
): EmbeddingFn {
  const dim = opts.dim ?? 384;
  if (!Number.isInteger(dim) || dim <= 0) {
    throw new Error(`createDeterministicEmbedder: dim must be positive integer, got ${dim}`);
  }

  return async function embed(text: string): Promise<number[]> {
    const out = new Array<number>(dim);
    let cursor = 0;
    let counter = 0;
    while (cursor < dim) {
      const block = createHash("sha256")
        .update(`${text}:${counter}`, "utf-8")
        .digest();
      for (let i = 0; i < block.length && cursor < dim; i++) {
        const byte = block[i] ?? 0;
        out[cursor++] = (byte - 127.5) / 127.5;
      }
      counter++;
    }
    // L2 normalize to unit length.
    let sumSq = 0;
    for (let i = 0; i < dim; i++) {
      const v = out[i] ?? 0;
      sumSq += v * v;
    }
    const norm = Math.sqrt(sumSq) || 1;
    for (let i = 0; i < dim; i++) {
      out[i] = (out[i] ?? 0) / norm;
    }
    return out;
  };
}
