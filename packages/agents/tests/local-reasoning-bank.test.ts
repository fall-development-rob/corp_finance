import { describe, it, expect, beforeEach } from 'vitest';
import { LocalReasoningBank } from '../learning/reasoning-bank.js';
import type { ReasoningTrace, QualityFeedback } from '../types/learning.js';

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

describe('LocalReasoningBank', () => {
  let bank: LocalReasoningBank;

  beforeEach(() => {
    bank = new LocalReasoningBank();
  });

  it('starts with empty stats', () => {
    const stats = bank.getStats();
    expect(stats.totalPatterns).toBe(0);
    expect(stats.totalTraces).toBe(0);
    expect(stats.avgReward).toBe(0);
  });

  it('records a successful trace and creates a pattern', async () => {
    await bank.recordTrace(makeTrace());

    const stats = bank.getStats();
    expect(stats.totalTraces).toBe(1);
    expect(stats.totalPatterns).toBe(1);
  });

  it('does not create a pattern for failed traces', async () => {
    await bank.recordTrace(makeTrace({ outcome: 'failure' }));

    const stats = bank.getStats();
    expect(stats.totalTraces).toBe(1);
    expect(stats.totalPatterns).toBe(0);
  });

  it('does not create a pattern for traces without tool calls', async () => {
    const trace = makeTrace({
      steps: [
        { phase: 'observe', content: 'Looking around', timestamp: new Date() },
        { phase: 'think', content: 'Thinking', timestamp: new Date() },
      ],
    });

    await bank.recordTrace(trace);
    expect(bank.getStats().totalPatterns).toBe(0);
  });

  it('deduplicates patterns with the same tool fingerprint', async () => {
    await bank.recordTrace(makeTrace({ traceId: 'trace-1' }));
    await bank.recordTrace(makeTrace({ traceId: 'trace-2' }));

    const stats = bank.getStats();
    expect(stats.totalTraces).toBe(2);
    expect(stats.totalPatterns).toBe(1);
  });

  it('searches patterns by task type', async () => {
    await bank.recordTrace(makeTrace({ agentType: 'equity-analyst' }));
    await bank.recordTrace(
      makeTrace({
        traceId: 'trace-credit',
        agentType: 'credit-analyst',
        steps: [
          {
            phase: 'act',
            content: 'Running credit metrics',
            toolCalls: ['credit_metrics', 'altman_zscore'],
            timestamp: new Date(),
          },
        ],
      }),
    );

    const valuationPatterns = await bank.searchPatterns('valuation');
    expect(valuationPatterns.length).toBe(1);
    expect(valuationPatterns[0].toolSequence).toContain('dcf_model');

    const creditPatterns = await bank.searchPatterns('credit_assessment');
    expect(creditPatterns.length).toBe(1);
    expect(creditPatterns[0].toolSequence).toContain('credit_metrics');
  });

  it('returns empty array for unknown task type', async () => {
    await bank.recordTrace(makeTrace());
    const patterns = await bank.searchPatterns('macro_research');
    expect(patterns).toEqual([]);
  });

  it('retrieves a pattern by ID', async () => {
    await bank.recordTrace(makeTrace());
    const patterns = await bank.searchPatterns('valuation');
    const pattern = await bank.getPattern(patterns[0].patternId);

    expect(pattern).not.toBeNull();
    expect(pattern!.toolSequence).toContain('wacc_calculator');
    expect(pattern!.toolSequence).toContain('dcf_model');
  });

  it('returns null for unknown pattern ID', async () => {
    const pattern = await bank.getPattern('nonexistent');
    expect(pattern).toBeNull();
  });

  it('updates reward score via feedback (EMA blend)', async () => {
    await bank.recordTrace(makeTrace());
    const patternsBefore = await bank.searchPatterns('valuation');
    const initialReward = patternsBefore[0].rewardScore;

    const feedback: QualityFeedback = {
      feedbackId: 'fb-1',
      requestId: 'req-1',
      score: 1.0,
      automated: false,
      createdAt: new Date(),
    };
    await bank.recordFeedback(feedback);

    const patternsAfter = await bank.searchPatterns('valuation');
    expect(patternsAfter[0].rewardScore).toBeGreaterThan(initialReward);
  });

  it('sorts search results by reward score descending', async () => {
    // Create two different patterns
    await bank.recordTrace(makeTrace({ traceId: 'trace-a', requestId: 'req-a' }));
    await bank.recordTrace(
      makeTrace({
        traceId: 'trace-b',
        requestId: 'req-b',
        steps: [
          {
            phase: 'act',
            content: 'Alt approach',
            toolCalls: ['comps_analysis', 'sensitivity_matrix'],
            timestamp: new Date(),
          },
        ],
      }),
    );

    // Boost second pattern's reward
    await bank.recordFeedback({
      feedbackId: 'fb-2',
      requestId: 'req-b',
      score: 1.0,
      automated: false,
      createdAt: new Date(),
    });

    const patterns = await bank.searchPatterns('valuation');
    expect(patterns.length).toBe(2);
    expect(patterns[0].rewardScore).toBeGreaterThanOrEqual(patterns[1].rewardScore);
  });

  it('infers task type from agent type string', async () => {
    const agentMappings: Array<[string, string]> = [
      ['equity-analyst', 'valuation'],
      ['credit-analyst', 'credit_assessment'],
      ['risk-analyst', 'risk_analysis'],
      ['quant-risk-analyst', 'risk_analysis'],
      ['macro-analyst', 'macro_research'],
      ['esg-analyst', 'esg_review'],
      ['private-markets-analyst', 'deal_analysis'],
      ['pe-analyst', 'deal_analysis'],
      ['portfolio-analyst', 'portfolio_construction'],
      ['regulatory-analyst', 'regulatory_check'],
    ];

    for (const [agentType, expectedTaskType] of agentMappings) {
      const localBank = new LocalReasoningBank();
      await localBank.recordTrace(
        makeTrace({
          traceId: `trace-${agentType}`,
          agentType,
          steps: [
            {
              phase: 'act',
              content: 'tool call',
              toolCalls: [`tool_for_${agentType}`],
              timestamp: new Date(),
            },
          ],
        }),
      );

      const patterns = await localBank.searchPatterns(expectedTaskType as any);
      expect(patterns.length).toBe(1);
    }
  });

  it('respects limit parameter in searchPatterns', async () => {
    // Create 5 distinct patterns
    for (let i = 0; i < 5; i++) {
      await bank.recordTrace(
        makeTrace({
          traceId: `trace-${i}`,
          requestId: `req-${i}`,
          steps: [
            {
              phase: 'act',
              content: `tool ${i}`,
              toolCalls: [`unique_tool_${i}`],
              timestamp: new Date(),
            },
          ],
        }),
      );
    }

    const limited = await bank.searchPatterns('valuation', 3);
    expect(limited.length).toBe(3);
  });
});
