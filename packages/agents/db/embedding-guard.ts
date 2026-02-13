// Embedding quality guard — validates embedding vectors before storage/search
// Detects silent degradation to hash-based fallback vectors (ADR-002 Decision 2)

/**
 * Error thrown when an embedding fails quality validation.
 * Indicates the embedding model may have fallen back to hash-based pseudo-random vectors.
 */
export class EmbeddingQualityError extends Error {
  constructor(
    message: string,
    public readonly variance: number,
    public readonly l2Norm: number,
  ) {
    super(message);
    this.name = 'EmbeddingQualityError';
  }
}

/**
 * Validate an embedding vector for quality.
 * 
 * Checks:
 * 1. Variance > 0.001 — hash-based fallback produces low-variance vectors
 *    from the sin/cos generation pattern.
 * 2. L2 norm within [0.9, 1.1] — all-MiniLM-L6-v2 produces unit vectors.
 * 
 * @throws EmbeddingQualityError if validation fails
 */
export function validateEmbedding(embedding: Float32Array, text?: string): void {
  const n = embedding.length;
  if (n === 0) {
    throw new EmbeddingQualityError('Empty embedding vector', 0, 0);
  }

  // Compute mean
  let sum = 0;
  for (let i = 0; i < n; i++) sum += embedding[i];
  const mean = sum / n;

  // Compute variance
  let varianceSum = 0;
  for (let i = 0; i < n; i++) {
    const diff = embedding[i] - mean;
    varianceSum += diff * diff;
  }
  const variance = varianceSum / n;

  // Compute L2 norm
  let normSum = 0;
  for (let i = 0; i < n; i++) normSum += embedding[i] * embedding[i];
  const l2Norm = Math.sqrt(normSum);

  if (variance < 0.001) {
    const context = text ? ` for text "${text.slice(0, 50)}..."` : '';
    throw new EmbeddingQualityError(
      `Embedding variance too low (${variance.toFixed(6)})${context}. ` +
      `Model may have fallen back to hash-based vectors.`,
      variance,
      l2Norm,
    );
  }

  if (l2Norm < 0.9 || l2Norm > 1.1) {
    const context = text ? ` for text "${text.slice(0, 50)}..."` : '';
    throw new EmbeddingQualityError(
      `Embedding L2 norm out of range (${l2Norm.toFixed(4)}, expected [0.9, 1.1])${context}. ` +
      `Model output may be corrupted.`,
      variance,
      l2Norm,
    );
  }
}

/**
 * Compute embedding with quality validation.
 * Wraps the agentic-flow computeEmbedding and validates the result.
 * 
 * @throws EmbeddingQualityError if the embedding fails quality checks
 */
export async function computeValidatedEmbedding(
  computeFn: (text: string) => Promise<Float32Array>,
  text: string,
): Promise<Float32Array> {
  const embedding = await computeFn(text);
  validateEmbedding(embedding, text);
  return embedding;
}
