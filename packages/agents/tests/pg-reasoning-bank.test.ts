import { describe, it, expect, beforeEach, vi } from 'vitest';
import type { ReasoningTrace, QualityFeedback } from '../types/learning.js';

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
  queryWithRetry: vi.fn((...args: unknown[]) => mockQuery(...args)),
}));

vi.mock('agentic-flow/reasoningbank', () => ({
  computeEmbedding: vi.fn(() => Promise.resolve(new Float32Array(384).fill(0.1))),
}));

function makeTrace(overrides: Partial<ReasoningTrace> = {}): ReasoningTrace {
  return {
    traceId: 'trace-1',
    agentType: 'equity-analyst',
    requestId: 'req-1',
    steps: [
      { phase: 'observe', content: 'Reading financials', timestamp: new Date() },
      { phase: 'think', content: 'Analysing margins', timestamp: new Date() },
      {
        phase: 'act',
        content: 'Running DCF',
        toolCalls: ['wacc_calculator', 'dcf_model'],
        timestamp: new Date(),
      },
      { phase: 'reflect', content: 'Valuation complete', timestamp: new Date() },
    ],
    outcome: 'success',
    createdAt: new Date(),
    ...overrides,
  };
}

describe('PgReasoningBank', () => {
  let bank: InstanceType<typeof import('../learning/pg-reasoning-bank.js').PgReasoningBank>;

  beforeEach(async () => {
    vi.clearAllMocks();
    const { PgReasoningBank } = await import('../learning/pg-reasoning-bank.js');
    bank = new PgReasoningBank();
  });

  it('starts with empty stats', async () => {
    const stats = await bank.getStats();
    expect(stats.totalPatterns).toBe(0);
    expect(stats.totalTraces).toBe(0);
    expect(stats.avgReward).toBe(0);
  });

  it('recordTrace() inserts trajectory and pattern for successful traces', async () => {
    mockQuery.mockResolvedValue({ rows: [], rowCount: 1 });

    await bank.recordTrace(makeTrace());

    const stats = await bank.getStats();
    expect(stats.totalTraces).toBe(1);
    expect(stats.totalPatterns).toBe(1);

    // Calls: trajectory insert, pattern upsert, getStats query (falls back to in-memory)
    expect(mockQuery).toHaveBeenCalledTimes(3);

    const [trajSql] = mockQuery.mock.calls[0];
    expect(trajSql).toContain('INSERT INTO task_trajectories');

    const [patternSql, patternParams] = mockQuery.mock.calls[1];
    expect(patternSql).toContain('INSERT INTO reasoning_memories');
    expect(patternSql).toContain('ON CONFLICT (fingerprint)');
    expect(patternParams[1]).toBe('valuation-pattern');
    expect(patternParams[8]).toBeTruthy(); // fingerprint param
  });

  it('recordTrace() inserts trajectory but no pattern for failed traces', async () => {
    mockQuery.mockResolvedValue({ rows: [], rowCount: 1 });

    await bank.recordTrace(makeTrace({ outcome: 'failure' }));

    const stats = await bank.getStats();
    expect(stats.totalTraces).toBe(1);
    expect(stats.totalPatterns).toBe(0);

    // Calls: trajectory insert + getStats query (falls back to in-memory)
    expect(mockQuery).toHaveBeenCalledTimes(2);
    const [sql] = mockQuery.mock.calls[0];
    expect(sql).toContain('INSERT INTO task_trajectories');
  });

  it('recordTrace() skips pattern for traces without tool calls', async () => {
    mockQuery.mockResolvedValue({ rows: [], rowCount: 1 });

    await bank.recordTrace(makeTrace({
      steps: [
        { phase: 'observe', content: 'Looking around', timestamp: new Date() },
        { phase: 'think', content: 'Thinking', timestamp: new Date() },
      ],
    }));

    expect((await bank.getStats()).totalPatterns).toBe(0);
    // Calls: trajectory insert + getStats query (falls back to in-memory)
    expect(mockQuery).toHaveBeenCalledTimes(2);
  });

  it('recordFeedback() inserts feedback as reasoning memory', async () => {
    mockQuery.mockResolvedValue({ rows: [], rowCount: 1 });

    const feedback: QualityFeedback = {
      feedbackId: 'fb-1',
      requestId: 'req-1',
      score: 0.85,
      automated: false,
      createdAt: new Date(),
    };

    await bank.recordFeedback(feedback);

    expect(mockQuery).toHaveBeenCalledOnce();
    const [sql, params] = mockQuery.mock.calls[0];
    expect(sql).toContain('INSERT INTO reasoning_memories');
    expect(params[4]).toBe(0.85); // confidence = score
  });

  it('searchPatterns() queries with embedding and returns parsed patterns', async () => {
    mockQuery.mockResolvedValueOnce({
      rows: [{
        id: 'p-1',
        content: JSON.stringify({
          patternId: 'p-1',
          taskType: 'valuation',
          toolSequence: ['wacc_calculator', 'dcf_model'],
          agentTypes: ['equity-analyst'],
          rewardScore: 0.7,
          fingerprint: 'abc123',
        }),
        confidence: 0.7,
        usage_count: 5,
        similarity: 0.88,
      }],
    });

    const patterns = await bank.searchPatterns('valuation', 5);

    expect(patterns).toHaveLength(1);
    expect(patterns[0].patternId).toBe('p-1');
    expect(patterns[0].taskType).toBe('valuation');
    expect(patterns[0].toolSequence).toContain('dcf_model');
    expect(patterns[0].rewardScore).toBe(0.7);
    expect(patterns[0].usageCount).toBe(5);

    const [sql, params] = mockQuery.mock.calls[0];
    expect(sql).toContain('search_reasoning_memories');
    expect(params[1]).toBe('cfa-valuation');
    expect(params[2]).toBe(5);
  });

  it('searchPatterns() returns empty for no matches', async () => {
    mockQuery.mockResolvedValueOnce({ rows: [] });

    const patterns = await bank.searchPatterns('macro_research');
    expect(patterns).toEqual([]);
  });

  it('searchPatterns() skips rows with invalid JSON', async () => {
    mockQuery.mockResolvedValueOnce({
      rows: [
        { id: 'p-1', content: 'not json', confidence: 0.5, usage_count: 0, similarity: 0.9 },
        {
          id: 'p-2',
          content: JSON.stringify({
            patternId: 'p-2',
            taskType: 'valuation',
            toolSequence: ['dcf_model'],
            agentTypes: ['equity-analyst'],
            fingerprint: 'def456',
          }),
          confidence: 0.6,
          usage_count: 2,
          similarity: 0.85,
        },
      ],
    });

    const patterns = await bank.searchPatterns('valuation');
    expect(patterns).toHaveLength(1);
    expect(patterns[0].patternId).toBe('p-2');
  });

  it('getPattern() returns pattern by ID', async () => {
    mockQuery.mockResolvedValueOnce({
      rows: [{
        id: 'p-1',
        content: JSON.stringify({
          patternId: 'p-1',
          taskType: 'valuation',
          toolSequence: ['wacc_calculator'],
          agentTypes: ['equity-analyst'],
          fingerprint: 'abc123',
        }),
        confidence: 0.8,
        usage_count: 10,
        created_at: new Date('2024-01-01'),
        last_used_at: new Date('2024-06-01'),
      }],
    });

    const pattern = await bank.getPattern('p-1');

    expect(pattern).not.toBeNull();
    expect(pattern!.patternId).toBe('p-1');
    expect(pattern!.toolSequence).toContain('wacc_calculator');
    expect(pattern!.rewardScore).toBe(0.8);
    expect(pattern!.usageCount).toBe(10);
  });

  it('getPattern() returns null for nonexistent ID', async () => {
    mockQuery.mockResolvedValueOnce({ rows: [] });

    const pattern = await bank.getPattern('nonexistent');
    expect(pattern).toBeNull();
  });

  it('infers correct task types from agent type strings', async () => {
    mockQuery.mockResolvedValue({ rows: [], rowCount: 1 });

    const mappings: [string, string][] = [
      ['equity-analyst', 'cfa-valuation'],
      ['credit-analyst', 'cfa-credit-assessment'],
      ['quant-risk-analyst', 'cfa-risk-analysis'],
      ['macro-analyst', 'cfa-macro-research'],
      ['esg-analyst', 'cfa-esg-review'],
      ['private-markets-analyst', 'cfa-deal-analysis'],
    ];

    for (const [agentType, expectedDomain] of mappings) {
      vi.clearAllMocks();
      mockQuery.mockResolvedValue({ rows: [], rowCount: 1 });

      await bank.recordTrace(makeTrace({
        traceId: `trace-${agentType}`,
        agentType,
      }));

      // Pattern insert is the second call — check the domain param
      const patternCall = mockQuery.mock.calls[1];
      if (patternCall) {
        expect(patternCall[1][4]).toBe(expectedDomain); // domain param
      }
    }
  });

  it('tracks cumulative stats across multiple traces', async () => {
    mockQuery.mockResolvedValue({ rows: [], rowCount: 1 });

    await bank.recordTrace(makeTrace({ traceId: 't1' }));
    await bank.recordTrace(makeTrace({ traceId: 't2' }));
    await bank.recordTrace(makeTrace({ traceId: 't3', outcome: 'failure' }));

    const stats = await bank.getStats();
    expect(stats.totalTraces).toBe(3);
    expect(stats.totalPatterns).toBe(2);
    expect(stats.avgReward).toBe(0.5); // 0.5 * 2 / 2
  });
});
