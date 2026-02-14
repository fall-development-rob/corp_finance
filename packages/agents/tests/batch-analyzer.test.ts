// Tests for ADR-006 BatchAnalyzer — batch portfolio analysis

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { BatchAnalyzer, type BatchProgress } from '../orchestrator/batch-analyzer.js';

// Mock the Orchestrator module
vi.mock('./coordinator.js', () => {
  return {
    Orchestrator: vi.fn().mockImplementation(() => ({
      analyze: vi.fn(),
    })),
  };
});

describe('BatchAnalyzer', () => {
  const mockCallTool = vi.fn().mockResolvedValue({});

  describe('constructor', () => {
    it('creates instance with config', () => {
      const analyzer = new BatchAnalyzer({ callTool: mockCallTool });
      expect(analyzer).toBeDefined();
    });
  });

  describe('analyze', () => {
    it('processes companies and produces comparative output', async () => {
      // Create a mock orchestrator that returns canned results
      const mockAnalyze = vi.fn().mockImplementation(async (query: string) => {
        const company = query.includes('Apple') ? 'Apple' : 'Microsoft';
        return {
          request: { requestId: `req-${company}` },
          report: `Analysis report for ${company}: Revenue is strong.`,
          results: [{
            resultId: `res-${company}`,
            agentId: 'agent-1',
            agentType: 'credit-analyst',
            assignmentId: 'assign-1',
            findings: [{
              statement: `${company} has strong credit profile`,
              supportingData: { interest_coverage: company === 'Apple' ? 49 : 30 },
              confidence: 0.9,
              methodology: 'credit-scoring',
              citations: [],
            }],
            summary: `${company} credit analysis`,
            confidence: 0.85,
            toolInvocations: [],
            completedAt: new Date(),
          }],
        };
      });

      // Override the orchestrator's analyze method
      const analyzer = new BatchAnalyzer({ callTool: mockCallTool });
      (analyzer as any).orchestrator = { analyze: mockAnalyze };

      const progressUpdates: BatchProgress[] = [];
      const result = await analyzer.analyze(
        ['Apple', 'Microsoft'],
        'Compare credit risk',
        {
          concurrency: 2,
          onProgress: (p) => progressUpdates.push({ ...p }),
        },
      );

      // Individual results
      expect(result.companies).toHaveLength(2);
      expect(result.companies[0].company).toBe('Apple');
      expect(result.companies[1].company).toBe('Microsoft');
      expect(result.companies[0].report).toContain('Apple');
      expect(result.companies[1].report).toContain('Microsoft');

      // Comparative summary
      expect(result.comparative).toContain('Comparative');
      expect(result.comparative).toContain('Apple');
      expect(result.comparative).toContain('Microsoft');

      // Duration
      expect(result.totalDurationMs).toBeGreaterThan(0);

      // Progress callbacks
      expect(progressUpdates.length).toBeGreaterThan(0);
    });

    it('handles failed analyses gracefully', async () => {
      const mockAnalyze = vi.fn()
        .mockResolvedValueOnce({
          request: { requestId: 'req-1' },
          report: 'Apple report',
          results: [],
        })
        .mockRejectedValueOnce(new Error('API rate limit'));

      const analyzer = new BatchAnalyzer({ callTool: mockCallTool });
      (analyzer as any).orchestrator = { analyze: mockAnalyze };

      const result = await analyzer.analyze(
        ['Apple', 'FailCo'],
        'Credit analysis',
        { concurrency: 2 },
      );

      expect(result.companies).toHaveLength(2);
      expect(result.companies[0].error).toBeUndefined();
      expect(result.companies[1].error).toContain('rate limit');
      expect(result.comparative).toContain('Failed');
    });

    it('respects concurrency limit', async () => {
      let concurrent = 0;
      let maxConcurrent = 0;

      const mockAnalyze = vi.fn().mockImplementation(async (query: string) => {
        concurrent++;
        maxConcurrent = Math.max(maxConcurrent, concurrent);
        await new Promise(r => setTimeout(r, 50));
        concurrent--;
        return {
          request: { requestId: 'req' },
          report: `Report for ${query}`,
          results: [],
        };
      });

      const analyzer = new BatchAnalyzer({ callTool: mockCallTool });
      (analyzer as any).orchestrator = { analyze: mockAnalyze };

      await analyzer.analyze(
        ['A', 'B', 'C', 'D', 'E'],
        'Analyze',
        { concurrency: 2 },
      );

      // With concurrency=2, max concurrent should be 2
      expect(maxConcurrent).toBeLessThanOrEqual(2);
    });

    it('builds query with {company} placeholder', async () => {
      const mockAnalyze = vi.fn().mockResolvedValue({
        request: { requestId: 'req' },
        report: 'report',
        results: [],
      });

      const analyzer = new BatchAnalyzer({ callTool: mockCallTool });
      (analyzer as any).orchestrator = { analyze: mockAnalyze };

      await analyzer.analyze(
        ['Apple'],
        'Analyze credit risk for {company} Inc.',
      );

      expect(mockAnalyze).toHaveBeenCalledWith(
        'Analyze credit risk for Apple Inc.',
        'STANDARD',
      );
    });

    it('appends company name when no placeholder', async () => {
      const mockAnalyze = vi.fn().mockResolvedValue({
        request: { requestId: 'req' },
        report: 'report',
        results: [],
      });

      const analyzer = new BatchAnalyzer({ callTool: mockCallTool });
      (analyzer as any).orchestrator = { analyze: mockAnalyze };

      await analyzer.analyze(
        ['Apple'],
        'Compare credit risk',
      );

      expect(mockAnalyze).toHaveBeenCalledWith(
        'Compare credit risk for Apple',
        'STANDARD',
      );
    });
  });
});
