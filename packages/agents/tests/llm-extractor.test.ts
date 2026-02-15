import { describe, it, expect } from 'vitest';
import { applyEntities } from '../utils/llm-extractor.js';
import type { ExtractedMetrics } from '../utils/financial-parser.js';

describe('llm-extractor', () => {
  describe('applyEntities', () => {
    it('fills in missing company name', () => {
      const metrics: ExtractedMetrics = { _raw: 'test' };
      applyEntities(metrics, { company: 'Apple', ticker: 'AAPL' });
      expect(metrics._company).toBe('Apple');
      expect(metrics._symbol).toBe('AAPL');
    });

    it('does not overwrite existing company name', () => {
      const metrics: ExtractedMetrics = { _raw: 'test', _company: 'Microsoft' };
      applyEntities(metrics, { company: 'Apple' });
      expect(metrics._company).toBe('Microsoft');
    });

    it('does not overwrite existing symbol', () => {
      const metrics: ExtractedMetrics = { _raw: 'test', _symbol: 'MSFT' };
      applyEntities(metrics, { ticker: 'AAPL' });
      expect(metrics._symbol).toBe('MSFT');
    });

    it('fills sector when missing', () => {
      const metrics: ExtractedMetrics = { _raw: 'test' };
      applyEntities(metrics, { sector: 'Technology' });
      expect(metrics._sector).toBe('Technology');
    });

    it('handles empty entities gracefully', () => {
      const metrics: ExtractedMetrics = { _raw: 'test' };
      applyEntities(metrics, {});
      expect(metrics._company).toBeUndefined();
      expect(metrics._symbol).toBeUndefined();
    });
  });

  describe('createEntityExtractor', () => {
    it('returns null when ANTHROPIC_API_KEY is not set', async () => {
      const originalKey = process.env.ANTHROPIC_API_KEY;
      delete process.env.ANTHROPIC_API_KEY;
      try {
        // Re-import to get fresh evaluation
        const { createEntityExtractor } = await import('../utils/llm-extractor.js');
        const extractor = createEntityExtractor();
        expect(extractor).toBeNull();
      } finally {
        if (originalKey) process.env.ANTHROPIC_API_KEY = originalKey;
      }
    });
  });
});
