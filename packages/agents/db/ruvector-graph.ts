// ruvector-graph — Mincut-based pattern clustering via ruvector graph primitives
// Implements Stoer-Wagner mincut for partitioning reasoning patterns into coherent clusters
// Uses ruvector's graph_edge_similarity() and compute_mincut() SQL functions

import { queryWithRetry } from './pg-client.js';

export interface PatternEdge {
  sourceId: string;
  targetId: string;
  similarity: number;
}

export interface MincutResult {
  cutValue: number;
  partitionA: string[];
  partitionB: string[];
}

export interface PatternCluster {
  clusterId: number;
  patternIds: string[];
  coherenceScore: number;
}

export interface NoveltyScore {
  patternId: string;
  maxSimilarityToCluster: number;
  isNovel: boolean;
  nearestClusterId: number | null;
}

/**
 * Build similarity edges between patterns in a domain.
 * Uses ruvector's graph_edge_similarity(real[], real[]) primitive.
 */
export async function buildPatternEdges(
  domain: string,
  threshold = 0.3,
): Promise<PatternEdge[]> {
  const { rows } = await queryWithRetry<{
    source_id: string;
    target_id: string;
    similarity: number;
  }>(
    'SELECT * FROM build_pattern_edges($1, $2)',
    [domain, threshold],
  );

  return rows.map(r => ({
    sourceId: r.source_id,
    targetId: r.target_id,
    similarity: r.similarity,
  }));
}

/**
 * Compute the minimum cut of the pattern similarity graph.
 * Uses the Stoer-Wagner implementation in PL/pgSQL.
 */
export async function computeMincut(
  domain: string,
  threshold = 0.3,
): Promise<MincutResult> {
  const { rows } = await queryWithRetry<{
    cut_value: number;
    partition_a: string[];
    partition_b: string[];
  }>(
    'SELECT * FROM compute_mincut($1, $2)',
    [domain, threshold],
  );

  if (rows.length === 0) {
    return { cutValue: 0, partitionA: [], partitionB: [] };
  }

  return {
    cutValue: rows[0].cut_value,
    partitionA: rows[0].partition_a ?? [],
    partitionB: rows[0].partition_b ?? [],
  };
}

/**
 * Recursively partition patterns into clusters using repeated mincut.
 * Stops when the cut value exceeds the threshold (cluster is cohesive).
 */
export async function partitionPatterns(
  domain: string,
  options: {
    similarityThreshold?: number;
    maxClusters?: number;
    minCutThreshold?: number;
  } = {},
): Promise<PatternCluster[]> {
  const {
    similarityThreshold = 0.3,
    maxClusters = 10,
    minCutThreshold = 0.5,
  } = options;

  // Get initial mincut
  const result = await computeMincut(domain, similarityThreshold);

  if (result.partitionA.length === 0 && result.partitionB.length === 0) {
    return [];
  }

  // If cut value is high, the graph is cohesive — single cluster
  if (result.cutValue >= minCutThreshold || result.partitionB.length === 0) {
    const allIds = [...result.partitionA, ...result.partitionB];
    await assignPartition(allIds, 0, result.cutValue);
    return [{
      clusterId: 0,
      patternIds: allIds,
      coherenceScore: result.cutValue,
    }];
  }

  // Assign the two partitions
  const clusters: PatternCluster[] = [
    { clusterId: 0, patternIds: result.partitionA, coherenceScore: result.cutValue },
    { clusterId: 1, patternIds: result.partitionB, coherenceScore: result.cutValue },
  ];

  // Persist partition assignments
  await Promise.all([
    assignPartition(result.partitionA, 0, result.cutValue),
    assignPartition(result.partitionB, 1, result.cutValue),
  ]);

  return clusters;
}

/**
 * Persist partition assignment to reasoning_memories.
 */
async function assignPartition(
  patternIds: string[],
  partitionId: number,
  coherenceScore: number,
): Promise<void> {
  if (patternIds.length === 0) return;

  await queryWithRetry(
    `UPDATE reasoning_memories
     SET mincut_partition = $1, coherence_score = $2
     WHERE id = ANY($3)`,
    [partitionId, coherenceScore, patternIds],
  );
}

/**
 * Check if a new pattern is novel (doesn't fit existing clusters).
 * Uses graph_edge_similarity to compare against cluster centroids.
 */
export async function detectNovelPattern(
  patternId: string,
  domain: string,
  noveltyThreshold = 0.4,
): Promise<NoveltyScore> {
  // Find max similarity to any existing pattern in each partition
  const { rows } = await queryWithRetry<{
    partition: number;
    max_sim: number;
  }>(
    `SELECT rm.mincut_partition AS partition,
            max(graph_edge_similarity(
              ruvector_to_real_array((SELECT embedding FROM reasoning_memories WHERE id = $1)),
              ruvector_to_real_array(rm.embedding)
            )) AS max_sim
     FROM reasoning_memories rm
     WHERE rm.domain = $2
       AND rm.embedding IS NOT NULL
       AND rm.mincut_partition IS NOT NULL
       AND rm.id != $1
     GROUP BY rm.mincut_partition
     ORDER BY max_sim DESC`,
    [patternId, domain],
  );

  if (rows.length === 0) {
    return {
      patternId,
      maxSimilarityToCluster: 0,
      isNovel: true,
      nearestClusterId: null,
    };
  }

  const best = rows[0];
  return {
    patternId,
    maxSimilarityToCluster: best.max_sim,
    isNovel: best.max_sim < noveltyThreshold,
    nearestClusterId: best.partition,
  };
}

/**
 * Get PageRank-based importance scores for patterns in a domain.
 * Uses ruvector's graph_pagerank_contribution() primitive.
 */
export async function computePatternPageRank(
  domain: string,
  damping = 0.85,
): Promise<Array<{ patternId: string; importance: number }>> {
  const { rows } = await queryWithRetry<{
    id: string;
    importance: number;
  }>(
    `WITH pattern_counts AS (
       SELECT count(*) AS n FROM reasoning_memories
       WHERE domain = $1 AND embedding IS NOT NULL
     ),
     base_rank AS (
       SELECT graph_pagerank_base(n::integer, $2) AS base FROM pattern_counts
     ),
     neighbor_counts AS (
       SELECT pl.source_id, count(*) AS n_neighbors
       FROM pattern_links pl
       JOIN reasoning_memories rm ON rm.id = pl.source_id
       WHERE rm.domain = $1
       GROUP BY pl.source_id
     )
     SELECT
       rm.id,
       coalesce(
         graph_pagerank_contribution(
           rm.confidence,
           coalesce(nc.n_neighbors, 1)::integer,
           $2
         ),
         (SELECT base FROM base_rank)
       ) AS importance
     FROM reasoning_memories rm
     LEFT JOIN neighbor_counts nc ON nc.source_id = rm.id
     WHERE rm.domain = $1 AND rm.embedding IS NOT NULL
     ORDER BY importance DESC`,
    [domain, damping],
  );

  return rows.map(r => ({ patternId: r.id, importance: r.importance }));
}
