// Tests for ADR-006 InsightBus — cross-specialist collaboration

import { describe, it, expect, beforeEach, vi } from 'vitest';
import { InsightBus } from '../collaboration/insight-bus.js';
import type { AgentInsight } from '../types/collaboration.js';

describe('InsightBus', () => {
  let bus: InsightBus;

  beforeEach(() => {
    bus = new InsightBus();
  });

  describe('broadcast', () => {
    it('stores insight and assigns id + timestamp', () => {
      const insight = bus.broadcast({
        sourceAgent: 'credit-analyst',
        sourceAgentId: 'agent-1',
        insightType: 'finding',
        content: 'Interest coverage is 49x — exceptionally strong',
        data: { interest_coverage: 49 },
        confidence: 0.92,
      });

      expect(insight.id).toBeDefined();
      expect(insight.timestamp).toBeInstanceOf(Date);
      expect(insight.content).toContain('49x');
      expect(bus.size).toBe(1);
    });

    it('delivers to subscribers except source agent', () => {
      const handler1 = vi.fn();
      const handler2 = vi.fn();

      bus.subscribe('agent-1', handler1);
      bus.subscribe('agent-2', handler2);

      bus.broadcast({
        sourceAgent: 'credit-analyst',
        sourceAgentId: 'agent-1',
        insightType: 'finding',
        content: 'High leverage detected',
        data: {},
        confidence: 0.8,
      });

      // agent-1 is the source — should NOT receive
      expect(handler1).not.toHaveBeenCalled();
      // agent-2 should receive
      expect(handler2).toHaveBeenCalledTimes(1);
      expect(handler2.mock.calls[0][0].content).toBe('High leverage detected');
    });

    it('does not crash if subscriber handler throws', () => {
      bus.subscribe('agent-2', () => { throw new Error('boom'); });

      expect(() => {
        bus.broadcast({
          sourceAgent: 'equity-analyst',
          sourceAgentId: 'agent-1',
          insightType: 'metric',
          content: 'PE ratio is 25x',
          data: { pe_ratio: 25 },
          confidence: 0.85,
        });
      }).not.toThrow();

      expect(bus.size).toBe(1);
    });
  });

  describe('subscribe / unsubscribe', () => {
    it('tracks subscriber count', () => {
      expect(bus.subscriberCount).toBe(0);

      bus.subscribe('a1', () => {});
      bus.subscribe('a2', () => {});
      expect(bus.subscriberCount).toBe(2);

      bus.unsubscribe('a1');
      expect(bus.subscriberCount).toBe(1);
    });
  });

  describe('getInsights', () => {
    beforeEach(() => {
      bus.broadcast({
        sourceAgent: 'credit-analyst',
        sourceAgentId: 'agent-1',
        insightType: 'finding',
        content: 'Strong coverage',
        data: {},
        confidence: 0.9,
      });
      bus.broadcast({
        sourceAgent: 'equity-analyst',
        sourceAgentId: 'agent-2',
        insightType: 'risk',
        content: 'Overvalued on PE basis',
        data: {},
        confidence: 0.6,
      });
      bus.broadcast({
        sourceAgent: 'credit-analyst',
        sourceAgentId: 'agent-1',
        insightType: 'metric',
        content: 'D/E ratio 0.3',
        data: { de_ratio: 0.3 },
        confidence: 0.95,
      });
    });

    it('returns all insights when no filter', () => {
      expect(bus.getInsights()).toHaveLength(3);
    });

    it('filters by sourceAgent', () => {
      const results = bus.getInsights({ sourceAgent: 'credit-analyst' });
      expect(results).toHaveLength(2);
    });

    it('filters by insightType', () => {
      const results = bus.getInsights({ insightType: 'risk' });
      expect(results).toHaveLength(1);
      expect(results[0].content).toBe('Overvalued on PE basis');
    });

    it('filters by minConfidence', () => {
      const results = bus.getInsights({ minConfidence: 0.85 });
      expect(results).toHaveLength(2);
    });
  });

  describe('getPeerInsights', () => {
    it('excludes the requesting agent', () => {
      bus.broadcast({
        sourceAgent: 'credit-analyst',
        sourceAgentId: 'agent-1',
        insightType: 'finding',
        content: 'My own finding',
        data: {},
        confidence: 0.9,
      });
      bus.broadcast({
        sourceAgent: 'equity-analyst',
        sourceAgentId: 'agent-2',
        insightType: 'finding',
        content: 'Peer finding',
        data: {},
        confidence: 0.8,
      });

      const peers = bus.getPeerInsights('agent-1');
      expect(peers).toHaveLength(1);
      expect(peers[0].content).toBe('Peer finding');
    });

    it('respects minConfidence', () => {
      bus.broadcast({
        sourceAgent: 'equity-analyst',
        sourceAgentId: 'agent-2',
        insightType: 'finding',
        content: 'Low conf',
        data: {},
        confidence: 0.3,
      });

      const peers = bus.getPeerInsights('agent-1', 0.5);
      expect(peers).toHaveLength(0);
    });
  });

  describe('formatPeerContext', () => {
    it('returns empty string when no peers', () => {
      expect(bus.formatPeerContext('agent-1')).toBe('');
    });

    it('formats peer insights as readable text', () => {
      bus.broadcast({
        sourceAgent: 'credit-analyst',
        sourceAgentId: 'agent-1',
        insightType: 'finding',
        content: 'Interest coverage 49x',
        data: {},
        confidence: 0.9,
      });

      const context = bus.formatPeerContext('agent-2');
      expect(context).toContain('[credit-analyst]');
      expect(context).toContain('finding');
      expect(context).toContain('Interest coverage 49x');
      expect(context).toContain('0.90');
    });
  });

  describe('clear', () => {
    it('removes all insights', () => {
      bus.broadcast({
        sourceAgent: 'credit-analyst',
        sourceAgentId: 'agent-1',
        insightType: 'finding',
        content: 'test',
        data: {},
        confidence: 0.5,
      });
      expect(bus.size).toBe(1);

      bus.clear();
      expect(bus.size).toBe(0);
    });
  });
});
