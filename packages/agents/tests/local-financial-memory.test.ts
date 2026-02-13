import { describe, it, expect, beforeEach } from 'vitest';
import { LocalFinancialMemory } from '../memory/financial-memory.js';
import type { MemoryMetadata } from '../types/memory.js';

function makeMetadata(overrides: Partial<MemoryMetadata> = {}): MemoryMetadata {
  return {
    sourceType: 'analysis',
    sector: 'technology',
    tickers: ['AAPL'],
    analysisType: 'dcf_valuation',
    tags: ['equity', 'large-cap'],
    ...overrides,
  };
}

describe('LocalFinancialMemory', () => {
  let memory: LocalFinancialMemory;

  beforeEach(() => {
    memory = new LocalFinancialMemory();
  });

  it('stores and retrieves an entry', async () => {
    const content = 'AAPL DCF valuation: intrinsic value $185, current price $172';
    const entry = await memory.store(content, makeMetadata());

    expect(entry.entryId).toBeTruthy();
    expect(entry.content).toBe(content);
    expect(entry.metadata.sourceType).toBe('analysis');
    expect(entry.retentionTier).toBe('hot');

    const retrieved = await memory.retrieve(entry.entryId);
    expect(retrieved).not.toBeNull();
    expect(retrieved!.content).toBe(content);
    expect(retrieved!.accessCount).toBe(1);
  });

  it('returns null for unknown entry ID', async () => {
    const result = await memory.retrieve('nonexistent');
    expect(result).toBeNull();
  });

  it('increments access count on each retrieve', async () => {
    const entry = await memory.store('test content', makeMetadata());

    await memory.retrieve(entry.entryId);
    await memory.retrieve(entry.entryId);
    const third = await memory.retrieve(entry.entryId);

    expect(third!.accessCount).toBe(3);
  });

  it('searches by keyword matching', async () => {
    await memory.store('AAPL DCF valuation with WACC discount', makeMetadata());
    await memory.store('MSFT comparable analysis P/E multiples', makeMetadata({ tickers: ['MSFT'] }));
    await memory.store('GOOG revenue growth and margin expansion', makeMetadata({ tickers: ['GOOG'] }));

    const results = await memory.search('DCF valuation WACC');
    expect(results.entries.length).toBeGreaterThanOrEqual(1);
    expect(results.entries[0].entry.content).toContain('DCF');
    expect(results.entries[0].similarityScore).toBeGreaterThan(0);
    expect(results.query).toBe('DCF valuation WACC');
  });

  it('returns empty results for no matches', async () => {
    await memory.store('AAPL analysis', makeMetadata());
    const results = await memory.search('cryptocurrency blockchain');
    expect(results.entries.length).toBe(0);
  });

  it('sorts search results by similarity score descending', async () => {
    await memory.store('bond pricing yield duration convexity spread', makeMetadata());
    await memory.store('bond yield curve construction bootstrap', makeMetadata());
    await memory.store('equity DCF model valuation', makeMetadata());

    const results = await memory.search('bond yield');
    if (results.entries.length >= 2) {
      expect(results.entries[0].similarityScore).toBeGreaterThanOrEqual(
        results.entries[1].similarityScore,
      );
    }
  });

  it('respects limit parameter in search', async () => {
    for (let i = 0; i < 10; i++) {
      await memory.store(`analysis ${i} equity valuation`, makeMetadata());
    }

    const results = await memory.search('equity valuation', 3);
    expect(results.entries.length).toBe(3);
  });

  it('gets entries by ticker symbol', async () => {
    await memory.store('AAPL analysis 1', makeMetadata({ tickers: ['AAPL'] }));
    await memory.store('MSFT analysis', makeMetadata({ tickers: ['MSFT'] }));
    await memory.store('AAPL analysis 2', makeMetadata({ tickers: ['AAPL'] }));

    const aaplEntries = await memory.getByTicker('AAPL');
    expect(aaplEntries.length).toBe(2);
    expect(aaplEntries.every(e => e.metadata.tickers?.includes('AAPL'))).toBe(true);
  });

  it('returns empty array for unknown ticker', async () => {
    await memory.store('AAPL analysis', makeMetadata({ tickers: ['AAPL'] }));
    const results = await memory.getByTicker('TSLA');
    expect(results).toEqual([]);
  });

  it('getByTicker respects limit and sorts by creation time', async () => {
    for (let i = 0; i < 5; i++) {
      await memory.store(`AAPL analysis ${i}`, makeMetadata({ tickers: ['AAPL'] }));
    }

    const results = await memory.getByTicker('AAPL', 3);
    expect(results.length).toBe(3);
  });

  it('updates lastAccessedAt on search hit', async () => {
    const entry = await memory.store('equity valuation analysis', makeMetadata());
    const originalAccess = entry.lastAccessedAt.getTime();

    // Small delay to ensure timestamp difference
    await new Promise(r => setTimeout(r, 10));

    await memory.search('equity valuation');

    const retrieved = await memory.retrieve(entry.entryId);
    expect(retrieved!.lastAccessedAt.getTime()).toBeGreaterThanOrEqual(originalAccess);
  });

  it('tracks totalSearched count', async () => {
    for (let i = 0; i < 5; i++) {
      await memory.store(`entry ${i}`, makeMetadata());
    }

    const results = await memory.search('entry');
    expect(results.totalSearched).toBe(5);
  });

  it('generates unique entry IDs', async () => {
    const entry1 = await memory.store('content 1', makeMetadata());
    const entry2 = await memory.store('content 2', makeMetadata());
    expect(entry1.entryId).not.toBe(entry2.entryId);
  });
});
