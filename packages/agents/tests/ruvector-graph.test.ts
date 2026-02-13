import { describe, it, expect, vi, beforeEach } from 'vitest';

const mockQuery = vi.fn();

vi.mock('../db/pg-client.js', () => ({
  queryWithRetry: vi.fn((...args: unknown[]) => mockQuery(...args)),
}));

describe('ruvector-graph', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('buildPatternEdges', () => {
    it('returns similarity edges above threshold', async () => {
      mockQuery.mockResolvedValueOnce({
        rows: [
          { source_id: 'a', target_id: 'b', similarity: 0.85 },
          { source_id: 'a', target_id: 'c', similarity: 0.42 },
        ],
      });

      const { buildPatternEdges } = await import('../db/ruvector-graph.js');
      const edges = await buildPatternEdges('cfa-valuation', 0.3);

      expect(edges).toHaveLength(2);
      expect(edges[0].sourceId).toBe('a');
      expect(edges[0].similarity).toBe(0.85);

      const [sql, params] = mockQuery.mock.calls[0];
      expect(sql).toContain('build_pattern_edges');
      expect(params[0]).toBe('cfa-valuation');
      expect(params[1]).toBe(0.3);
    });

    it('returns empty array when no edges exist', async () => {
      mockQuery.mockResolvedValueOnce({ rows: [] });

      const { buildPatternEdges } = await import('../db/ruvector-graph.js');
      const edges = await buildPatternEdges('empty-domain');
      expect(edges).toEqual([]);
    });
  });

  describe('computeMincut', () => {
    it('returns mincut partitions', async () => {
      mockQuery.mockResolvedValueOnce({
        rows: [{
          cut_value: 0.35,
          partition_a: ['id-1', 'id-2'],
          partition_b: ['id-3', 'id-4'],
        }],
      });

      const { computeMincut } = await import('../db/ruvector-graph.js');
      const result = await computeMincut('cfa-valuation');

      expect(result.cutValue).toBe(0.35);
      expect(result.partitionA).toEqual(['id-1', 'id-2']);
      expect(result.partitionB).toEqual(['id-3', 'id-4']);
    });

    it('returns empty result when no patterns exist', async () => {
      mockQuery.mockResolvedValueOnce({ rows: [] });

      const { computeMincut } = await import('../db/ruvector-graph.js');
      const result = await computeMincut('empty-domain');

      expect(result.cutValue).toBe(0);
      expect(result.partitionA).toEqual([]);
    });
  });

  describe('partitionPatterns', () => {
    it('creates two partitions when cut is below threshold', async () => {
      // computeMincut call
      mockQuery.mockResolvedValueOnce({
        rows: [{
          cut_value: 0.2,
          partition_a: ['id-1', 'id-2'],
          partition_b: ['id-3'],
        }],
      });
      // Two assignPartition UPDATE calls
      mockQuery.mockResolvedValue({ rowCount: 1 });

      const { partitionPatterns } = await import('../db/ruvector-graph.js');
      const clusters = await partitionPatterns('cfa-valuation');

      expect(clusters).toHaveLength(2);
      expect(clusters[0].clusterId).toBe(0);
      expect(clusters[0].patternIds).toEqual(['id-1', 'id-2']);
      expect(clusters[1].clusterId).toBe(1);
      expect(clusters[1].patternIds).toEqual(['id-3']);
    });

    it('returns single cluster when cut is above threshold', async () => {
      mockQuery.mockResolvedValueOnce({
        rows: [{
          cut_value: 0.8,
          partition_a: ['id-1', 'id-2', 'id-3'],
          partition_b: [],
        }],
      });
      mockQuery.mockResolvedValue({ rowCount: 1 });

      const { partitionPatterns } = await import('../db/ruvector-graph.js');
      const clusters = await partitionPatterns('cfa-valuation');

      expect(clusters).toHaveLength(1);
      expect(clusters[0].patternIds).toEqual(['id-1', 'id-2', 'id-3']);
    });
  });

  describe('detectNovelPattern', () => {
    it('detects novel pattern below similarity threshold', async () => {
      mockQuery.mockResolvedValueOnce({
        rows: [{ partition: 0, max_sim: 0.15 }],
      });

      const { detectNovelPattern } = await import('../db/ruvector-graph.js');
      const score = await detectNovelPattern('new-id', 'cfa-valuation');

      expect(score.isNovel).toBe(true);
      expect(score.maxSimilarityToCluster).toBe(0.15);
      expect(score.nearestClusterId).toBe(0);
    });

    it('marks high-similarity pattern as not novel', async () => {
      mockQuery.mockResolvedValueOnce({
        rows: [{ partition: 1, max_sim: 0.75 }],
      });

      const { detectNovelPattern } = await import('../db/ruvector-graph.js');
      const score = await detectNovelPattern('known-id', 'cfa-valuation');

      expect(score.isNovel).toBe(false);
      expect(score.maxSimilarityToCluster).toBe(0.75);
    });

    it('marks pattern as novel when no clusters exist', async () => {
      mockQuery.mockResolvedValueOnce({ rows: [] });

      const { detectNovelPattern } = await import('../db/ruvector-graph.js');
      const score = await detectNovelPattern('orphan-id', 'cfa-valuation');

      expect(score.isNovel).toBe(true);
      expect(score.nearestClusterId).toBeNull();
    });
  });

  describe('computePatternPageRank', () => {
    it('returns importance scores', async () => {
      mockQuery.mockResolvedValueOnce({
        rows: [
          { id: 'p-1', importance: 0.42 },
          { id: 'p-2', importance: 0.18 },
        ],
      });

      const { computePatternPageRank } = await import('../db/ruvector-graph.js');
      const ranks = await computePatternPageRank('cfa-valuation');

      expect(ranks).toHaveLength(2);
      expect(ranks[0].patternId).toBe('p-1');
      expect(ranks[0].importance).toBe(0.42);
    });
  });
});
