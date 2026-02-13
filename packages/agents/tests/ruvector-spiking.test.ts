import { describe, it, expect, vi, beforeEach } from 'vitest';

const mockQuery = vi.fn();

vi.mock('../db/pg-client.js', () => ({
  queryWithRetry: vi.fn((...args: unknown[]) => mockQuery(...args)),
}));

describe('ruvector-spiking', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('fireSpike', () => {
    it('fires a spike and propagates to connected patterns', async () => {
      mockQuery.mockResolvedValueOnce({
        rows: [
          { fired_pattern: 'source-1', new_potential: 0.0, did_fire: true },
          { fired_pattern: 'target-1', new_potential: 0.45, did_fire: false },
          { fired_pattern: 'target-2', new_potential: 0.0, did_fire: true },
        ],
      });

      const { fireSpike } = await import('../db/ruvector-spiking.js');
      const events = await fireSpike('source-1');

      expect(events).toHaveLength(3);
      expect(events[0].firedPattern).toBe('source-1');
      expect(events[0].didFire).toBe(true);
      expect(events[0].newPotential).toBe(0.0);
      expect(events[1].didFire).toBe(false);
      expect(events[1].newPotential).toBe(0.45);
      expect(events[2].didFire).toBe(true);

      const [sql] = mockQuery.mock.calls[0];
      expect(sql).toContain('process_spike_event');
    });

    it('returns only source event when no connections exist', async () => {
      mockQuery.mockResolvedValueOnce({
        rows: [{ fired_pattern: 'isolated', new_potential: 0.0, did_fire: true }],
      });

      const { fireSpike } = await import('../db/ruvector-spiking.js');
      const events = await fireSpike('isolated');

      expect(events).toHaveLength(1);
      expect(events[0].firedPattern).toBe('isolated');
    });
  });

  describe('detectAnomalies', () => {
    it('returns patterns with abnormal spike rates', async () => {
      mockQuery.mockResolvedValueOnce({
        rows: [{
          pattern_id: 'hot-pattern',
          spike_rate: 15.0,
          avg_rate: 3.2,
          stddev_rate: 2.1,
          anomaly_score: 5.62,
        }],
      });

      const { detectAnomalies } = await import('../db/ruvector-spiking.js');
      const anomalies = await detectAnomalies('cfa-valuation', 3600, 2.0);

      expect(anomalies).toHaveLength(1);
      expect(anomalies[0].patternId).toBe('hot-pattern');
      expect(anomalies[0].anomalyScore).toBeGreaterThan(2.0);

      const [sql, params] = mockQuery.mock.calls[0];
      expect(sql).toContain('detect_spike_anomalies');
      expect(params[0]).toBe('cfa-valuation');
      expect(params[1]).toBe(3600);
    });

    it('returns empty when all rates are normal', async () => {
      mockQuery.mockResolvedValueOnce({ rows: [] });

      const { detectAnomalies } = await import('../db/ruvector-spiking.js');
      const anomalies = await detectAnomalies('cfa-valuation');

      expect(anomalies).toEqual([]);
    });
  });

  describe('getNetworkState', () => {
    it('returns network summary statistics', async () => {
      mockQuery
        .mockResolvedValueOnce({
          rows: [{
            total_neurons: '50',
            active_neurons: '12',
            avg_potential: 0.23,
            recent_spikes: '8',
          }],
        })
        .mockResolvedValueOnce({
          rows: [
            { id: 'p-1', spike_potential: 0.95, last_spike_at: new Date('2024-01-01') },
            { id: 'p-2', spike_potential: 0.82, last_spike_at: null },
          ],
        });

      const { getNetworkState } = await import('../db/ruvector-spiking.js');
      const state = await getNetworkState('cfa-valuation', 5);

      expect(state.totalNeurons).toBe(50);
      expect(state.activeNeurons).toBe(12);
      expect(state.avgPotential).toBe(0.23);
      expect(state.recentSpikes).toBe(8);
      expect(state.topFiringPatterns).toHaveLength(2);
      expect(state.topFiringPatterns[0].patternId).toBe('p-1');
    });
  });

  describe('computeSpikeAttention', () => {
    it('returns attention-weighted pattern scores', async () => {
      mockQuery.mockResolvedValueOnce({
        rows: [
          { id: 'p-1', raw_score: 0.92, normalized_weight: 0.65 },
          { id: 'p-2', raw_score: 0.48, normalized_weight: 0.35 },
        ],
      });

      const { computeSpikeAttention } = await import('../db/ruvector-spiking.js');
      const weights = await computeSpikeAttention('[0.1,0.2]', 'cfa-valuation', 5);

      expect(weights).toHaveLength(2);
      expect(weights[0].patternId).toBe('p-1');
      expect(weights[0].normalizedWeight).toBe(0.65);

      const [sql] = mockQuery.mock.calls[0];
      expect(sql).toContain('attention_score');
      expect(sql).toContain('attention_softmax');
    });
  });

  describe('resetNetwork', () => {
    it('resets all spike potentials', async () => {
      mockQuery.mockResolvedValueOnce({ rowCount: 25 });

      const { resetNetwork } = await import('../db/ruvector-spiking.js');
      const count = await resetNetwork('cfa-valuation');

      expect(count).toBe(25);
      const [sql] = mockQuery.mock.calls[0];
      expect(sql).toContain('spike_potential = 0.0');
    });
  });

  describe('buildLinksFromTrajectories', () => {
    it('creates pattern links from co-occurring patterns', async () => {
      mockQuery.mockResolvedValueOnce({ rowCount: 8 });

      const { buildLinksFromTrajectories } = await import('../db/ruvector-spiking.js');
      const count = await buildLinksFromTrajectories('cfa-valuation');

      expect(count).toBe(8);
      const [sql] = mockQuery.mock.calls[0];
      expect(sql).toContain('INSERT INTO pattern_links');
      expect(sql).toContain('graph_edge_similarity');
      expect(sql).toContain('graph_is_connected');
    });
  });
});
