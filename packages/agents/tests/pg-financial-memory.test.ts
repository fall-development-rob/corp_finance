import { describe, it, expect, beforeEach, vi } from 'vitest';
import type { MemoryMetadata } from '../types/memory.js';

// Mock pg module
const mockQuery = vi.fn();
const mockPool = { query: mockQuery };

vi.mock('../db/pg-client.js', () => ({
  getPool: vi.fn(() => Promise.resolve(mockPool)),
  float32ToVectorLiteral: vi.fn((vec: Float32Array) => {
    const parts: string[] = [];
    for (let i = 0; i < vec.length; i++) parts.push(String(vec[i]));
    return `[${parts.join(',')}]`;
  }),
}));

vi.mock('agentic-flow/reasoningbank', () => ({
  computeEmbedding: vi.fn(() => Promise.resolve(new Float32Array(384).fill(0.1))),
}));

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

describe('PgFinancialMemory', () => {
  let memory: InstanceType<typeof import('../memory/pg-financial-memory.js').PgFinancialMemory>;

  beforeEach(async () => {
    vi.clearAllMocks();
    const { PgFinancialMemory } = await import('../memory/pg-financial-memory.js');
    memory = new PgFinancialMemory('cfa-analysis');
  });

  it('store() inserts into reasoning_memories and returns a MemoryEntry', async () => {
    mockQuery.mockResolvedValueOnce({ rows: [], rowCount: 1 });

    const entry = await memory.store('AAPL DCF valuation: $185', makeMetadata());

    expect(entry.entryId).toBeTruthy();
    expect(entry.content).toBe('AAPL DCF valuation: $185');
    expect(entry.metadata.sourceType).toBe('analysis');
    expect(entry.embeddingModel).toBe('all-MiniLM-L6-v2');
    expect(entry.retentionTier).toBe('hot');
    expect(entry.accessCount).toBe(0);

    expect(mockQuery).toHaveBeenCalledOnce();
    const [sql, params] = mockQuery.mock.calls[0];
    expect(sql).toContain('INSERT INTO reasoning_memories');
    expect(params[3]).toBe('AAPL DCF valuation: $185'); // content
    expect(params[4]).toBe('cfa-analysis'); // domain
  });

  it('store() builds tags from metadata fields', async () => {
    mockQuery.mockResolvedValueOnce({ rows: [], rowCount: 1 });

    await memory.store('test', makeMetadata({
      sourceType: 'filing',
      tickers: ['MSFT', 'GOOG'],
      tags: ['tech'],
      sector: 'software',
      analysisType: 'comps',
    }));

    const tags = mockQuery.mock.calls[0][1][5]; // tags param
    expect(tags).toContain('filing');
    expect(tags).toContain('MSFT');
    expect(tags).toContain('GOOG');
    expect(tags).toContain('tech');
    expect(tags).toContain('software');
    expect(tags).toContain('comps');
  });

  it('search() calls search_reasoning_memories with embedding', async () => {
    mockQuery.mockResolvedValueOnce({
      rows: [
        {
          id: 'row-1',
          title: 'test',
          content: 'found content',
          domain: 'cfa-analysis',
          tags: ['equity'],
          confidence: 0.8,
          usage_count: 3,
          similarity: 0.92,
        },
      ],
    });

    const result = await memory.search('DCF valuation', 5);

    expect(result.query).toBe('DCF valuation');
    expect(result.entries).toHaveLength(1);
    expect(result.entries[0].similarityScore).toBe(0.92);
    expect(result.entries[0].entry.content).toBe('found content');
    expect(result.entries[0].entry.accessCount).toBe(3);

    const [sql, params] = mockQuery.mock.calls[0];
    expect(sql).toContain('search_reasoning_memories');
    expect(params[1]).toBe('cfa-analysis');
    expect(params[2]).toBe(5);
  });

  it('search() returns empty when no rows match', async () => {
    mockQuery.mockResolvedValueOnce({ rows: [] });

    const result = await memory.search('crypto blockchain');
    expect(result.entries).toHaveLength(0);
    expect(result.totalSearched).toBe(0);
  });

  it('retrieve() updates usage_count and returns the entry', async () => {
    mockQuery.mockResolvedValueOnce({
      rows: [{
        id: 'entry-1',
        content: 'stored analysis',
        domain: 'cfa-analysis',
        tags: ['equity'],
        confidence: 0.8,
        usage_count: 5,
        created_at: new Date('2024-01-01'),
      }],
    });

    const entry = await memory.retrieve('entry-1');

    expect(entry).not.toBeNull();
    expect(entry!.entryId).toBe('entry-1');
    expect(entry!.content).toBe('stored analysis');
    expect(entry!.accessCount).toBe(5);

    const [sql] = mockQuery.mock.calls[0];
    expect(sql).toContain('UPDATE reasoning_memories');
    expect(sql).toContain('usage_count = usage_count + 1');
  });

  it('retrieve() returns null for nonexistent entry', async () => {
    mockQuery.mockResolvedValueOnce({ rows: [] });

    const entry = await memory.retrieve('nonexistent');
    expect(entry).toBeNull();
  });

  it('getByTicker() uses embedding search with domain filter', async () => {
    mockQuery.mockResolvedValueOnce({
      rows: [
        {
          id: 'r1',
          content: 'AAPL analysis',
          domain: 'cfa-analysis',
          tags: ['equity'],
          confidence: 0.8,
          usage_count: 1,
          similarity: 0.85,
        },
      ],
    });

    const entries = await memory.getByTicker('AAPL', 5);

    expect(entries).toHaveLength(1);
    expect(entries[0].content).toBe('AAPL analysis');
    expect(entries[0].metadata.tickers).toContain('AAPL');

    const [sql, params] = mockQuery.mock.calls[0];
    expect(sql).toContain('search_reasoning_memories');
    expect(params[1]).toBe('cfa-analysis');
    expect(params[2]).toBe(5);
  });

  it('getByTicker() returns empty for no matches', async () => {
    mockQuery.mockResolvedValueOnce({ rows: [] });

    const entries = await memory.getByTicker('TSLA');
    expect(entries).toEqual([]);
  });

  it('generates unique entry IDs across stores', async () => {
    mockQuery.mockResolvedValue({ rows: [], rowCount: 1 });

    const e1 = await memory.store('content 1', makeMetadata());
    const e2 = await memory.store('content 2', makeMetadata());

    expect(e1.entryId).not.toBe(e2.entryId);
  });
});
