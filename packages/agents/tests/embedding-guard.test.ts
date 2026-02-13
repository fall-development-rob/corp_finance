import { describe, it, expect } from 'vitest';
import { validateEmbedding, computeValidatedEmbedding, EmbeddingQualityError } from '../db/embedding-guard.js';

/** Create a realistic unit vector with varied components */
function makeUnitVector(dim: number): Float32Array {
  const vec = new Float32Array(dim);
  for (let i = 0; i < dim; i++) {
    vec[i] = Math.sin(i * 0.37) * Math.cos(i * 0.13);
  }
  // Normalize to unit length
  let norm = 0;
  for (let i = 0; i < dim; i++) norm += vec[i] * vec[i];
  norm = Math.sqrt(norm);
  for (let i = 0; i < dim; i++) vec[i] /= norm;
  return vec;
}

describe('embedding-guard', () => {
  describe('validateEmbedding', () => {
    it('accepts a valid unit vector', () => {
      const vec = makeUnitVector(384);
      expect(() => validateEmbedding(vec)).not.toThrow();
    });

    it('rejects an empty vector', () => {
      expect(() => validateEmbedding(new Float32Array(0))).toThrow(EmbeddingQualityError);
    });

    it('rejects a constant (zero-variance) vector', () => {
      const vec = new Float32Array(384).fill(0.1);
      expect(() => validateEmbedding(vec)).toThrow(EmbeddingQualityError);
      expect(() => validateEmbedding(vec)).toThrow(/variance too low/);
    });

    it('rejects a vector with L2 norm far from 1.0', () => {
      const vec = makeUnitVector(384);
      // Scale to norm ~5.0
      for (let i = 0; i < vec.length; i++) vec[i] *= 5;
      expect(() => validateEmbedding(vec)).toThrow(EmbeddingQualityError);
      expect(() => validateEmbedding(vec)).toThrow(/L2 norm out of range/);
    });

    it('exposes variance and l2Norm on the error', () => {
      const vec = new Float32Array(384).fill(0.1);
      try {
        validateEmbedding(vec);
        expect.unreachable('should have thrown');
      } catch (err) {
        expect(err).toBeInstanceOf(EmbeddingQualityError);
        const e = err as EmbeddingQualityError;
        expect(e.variance).toBeLessThan(0.001);
        expect(e.l2Norm).toBeGreaterThan(1.1);
      }
    });

    it('includes text context in error message', () => {
      const vec = new Float32Array(384).fill(0.1);
      expect(() => validateEmbedding(vec, 'AAPL quarterly earnings'))
        .toThrow(/AAPL quarterly earnings/);
    });
  });

  describe('computeValidatedEmbedding', () => {
    it('returns the embedding when valid', async () => {
      const vec = makeUnitVector(384);
      const computeFn = async () => vec;
      const result = await computeValidatedEmbedding(computeFn, 'test');
      expect(result).toBe(vec);
    });

    it('throws when embedding is invalid', async () => {
      const badVec = new Float32Array(384).fill(0.1);
      const computeFn = async () => badVec;
      await expect(computeValidatedEmbedding(computeFn, 'test'))
        .rejects.toThrow(EmbeddingQualityError);
    });
  });
});
