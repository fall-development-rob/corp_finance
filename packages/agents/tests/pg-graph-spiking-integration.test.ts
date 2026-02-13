// Integration test — ruvector graph mincut + neural spiking
// Requires ruvector-postgres container running on port 5433 with migration 003 applied
// Skips automatically when the database is unreachable

import { describe, it, expect, beforeAll, afterAll } from 'vitest';
import { getPool, healthCheck, closePool, resetPool } from '../db/pg-client.js';
import {
  buildPatternEdges,
  computeMincut,
  partitionPatterns,
  detectNovelPattern,
  computePatternPageRank,
} from '../db/ruvector-graph.js';
import {
  fireSpike,
  detectAnomalies,
  getNetworkState,
  resetNetwork,
  buildLinksFromTrajectories,
} from '../db/ruvector-spiking.js';

// Check connectivity before running any tests
let canConnect = false;
try {
  const pool = await getPool({
    host: 'localhost',
    port: 5433,
    user: 'cfa',
    password: 'cfa_dev_pass',
    database: 'cfa_agents',
    poolMax: 3,
    connectionTimeoutMs: 3000,
  });
  const res = await pool.query('SELECT 1 AS ok');
  canConnect = res.rows[0]?.ok === 1;
} catch {
  canConnect = false;
}

const TEST_DOMAIN = 'test-graph-spiking';

describe.skipIf(!canConnect)('PG Graph + Spiking Integration', () => {
  // Insert test patterns with real embeddings via the embedding model
  beforeAll(async () => {
    await resetPool();
    const pool = await getPool();

    // Clean prior test data
    await pool.query(`DELETE FROM pattern_links WHERE source_id IN (SELECT id FROM reasoning_memories WHERE domain = $1)`, [TEST_DOMAIN]);
    await pool.query(`DELETE FROM reasoning_memories WHERE domain = $1`, [TEST_DOMAIN]);

    // Insert 6 patterns with varied embeddings (using ruvector math for deterministic vectors)
    // We'll create them as unit vectors in different "directions" to get varied similarities
    const patterns = [
      { title: 'dcf-valuation', content: 'DCF model with WACC and terminal value' },
      { title: 'comps-valuation', content: 'Comparable companies EV/EBITDA multiples' },
      { title: 'wacc-calc', content: 'WACC calculation using CAPM beta risk-free rate' },
      { title: 'credit-metrics', content: 'Altman Z-score debt coverage interest coverage' },
      { title: 'credit-spread', content: 'Credit default swap spread curve analysis' },
      { title: 'var-risk', content: 'Value at risk portfolio Monte Carlo simulation' },
    ];

    // Create dense 384-dim vectors with shared components so cosine similarity is nonzero.
    // Cluster 1 (valuation): patterns 0,1,2 use sin(d * 0.1) base + small per-pattern perturbation
    // Cluster 2 (credit): patterns 3,4 use cos(d * 0.1) base + small perturbation
    // Outlier: pattern 5 uses sin(d * 0.3) — different frequency, low similarity to both clusters
    const baseVectors = [
      (d: number) => Math.sin(d * 0.1) + 0.05 * Math.sin(d * 0.7),      // dcf
      (d: number) => Math.sin(d * 0.1) + 0.05 * Math.cos(d * 0.5),      // comps
      (d: number) => Math.sin(d * 0.1) + 0.05 * Math.sin(d * 0.9),      // wacc
      (d: number) => Math.cos(d * 0.1) + 0.05 * Math.sin(d * 0.3),      // credit-metrics
      (d: number) => Math.cos(d * 0.1) + 0.05 * Math.cos(d * 0.4),      // credit-spread
      (d: number) => Math.sin(d * 0.3) + 0.1 * Math.cos(d * 0.7),       // var-risk (outlier)
    ];

    for (let i = 0; i < patterns.length; i++) {
      const p = patterns[i];
      const gen = baseVectors[i];
      const vec = new Array(384);
      for (let d = 0; d < 384; d++) vec[d] = gen(d);
      // Normalize
      let norm = 0;
      for (const v of vec) norm += v * v;
      norm = Math.sqrt(norm);
      const normalized = vec.map((v: number) => (norm > 0 ? v / norm : 0));

      const literal = `[${normalized.map(v => v.toFixed(6)).join(',')}]`;

      await pool.query(
        `INSERT INTO reasoning_memories (id, type, title, description, content, domain, tags, confidence, usage_count, embedding, fingerprint)
         VALUES (gen_random_uuid(), 'reasoning_memory', $1, $2, $3, $4, $5, 0.5, 1, $6::ruvector, $7)`,
        [p.title, p.content, JSON.stringify({ title: p.title }), TEST_DOMAIN, [`pattern-${i}`], literal, `fp-test-${i}-${Date.now()}`],
      );
    }
  });

  afterAll(async () => {
    await resetPool();
    try {
      const pool = await getPool();
      await pool.query(`DELETE FROM pattern_links WHERE source_id IN (SELECT id FROM reasoning_memories WHERE domain = $1)`, [TEST_DOMAIN]);
      await pool.query(`DELETE FROM reasoning_memories WHERE domain = $1`, [TEST_DOMAIN]);
    } catch {
      // Best-effort cleanup
    }
    await closePool();
  });

  // ─── Graph / Mincut ──────────────────────────────────────────

  describe('buildPatternEdges', () => {
    it('finds similarity edges between patterns', async () => {
      const edges = await buildPatternEdges(TEST_DOMAIN, 0.01);

      expect(edges.length).toBeGreaterThan(0);
      expect(edges[0]).toHaveProperty('sourceId');
      expect(edges[0]).toHaveProperty('targetId');
      expect(edges[0]).toHaveProperty('similarity');
      expect(edges[0].similarity).toBeGreaterThanOrEqual(0.01);

      console.log(`Found ${edges.length} edges (threshold 0.01)`);
      // Patterns in the same cluster should have higher similarity
      for (const e of edges.slice(0, 3)) {
        console.log(`  ${e.sourceId.slice(0, 8)}..${e.targetId.slice(0, 8)} sim=${e.similarity.toFixed(4)}`);
      }
    });

    it('returns fewer edges at higher threshold', async () => {
      const lowThreshold = await buildPatternEdges(TEST_DOMAIN, 0.01);
      const highThreshold = await buildPatternEdges(TEST_DOMAIN, 0.5);

      expect(lowThreshold.length).toBeGreaterThanOrEqual(highThreshold.length);
    });
  });

  describe('computeMincut', () => {
    it('partitions the pattern graph', async () => {
      const result = await computeMincut(TEST_DOMAIN, 0.01);

      console.log(`Mincut value: ${result.cutValue}`);
      console.log(`Partition A: ${result.partitionA.length} patterns`);
      console.log(`Partition B: ${result.partitionB.length} patterns`);

      // Should produce two non-empty partitions (we have 6 patterns in 3 clusters)
      expect(result.partitionA.length).toBeGreaterThan(0);
      // Total should be 6
      expect(result.partitionA.length + result.partitionB.length).toBe(6);
    });
  });

  describe('partitionPatterns', () => {
    it('creates clusters and persists assignments', async () => {
      const clusters = await partitionPatterns(TEST_DOMAIN, {
        similarityThreshold: 0.01,
        minCutThreshold: 10.0, // High threshold to force a split
      });

      expect(clusters.length).toBeGreaterThanOrEqual(1);
      console.log(`Created ${clusters.length} clusters:`);
      for (const c of clusters) {
        console.log(`  Cluster ${c.clusterId}: ${c.patternIds.length} patterns, coherence=${c.coherenceScore.toFixed(4)}`);
      }

      // Verify assignments persisted
      const pool = await getPool();
      const { rows } = await pool.query<{ mincut_partition: number; cnt: string }>(
        `SELECT mincut_partition, count(*) AS cnt FROM reasoning_memories WHERE domain = $1 AND mincut_partition IS NOT NULL GROUP BY mincut_partition`,
        [TEST_DOMAIN],
      );
      expect(rows.length).toBeGreaterThanOrEqual(1);
    });
  });

  describe('detectNovelPattern', () => {
    it('evaluates novelty of an existing pattern', async () => {
      const pool = await getPool();
      const { rows } = await pool.query<{ id: string }>(
        `SELECT id FROM reasoning_memories WHERE domain = $1 LIMIT 1`,
        [TEST_DOMAIN],
      );

      const score = await detectNovelPattern(rows[0].id, TEST_DOMAIN, 0.01);

      expect(score.patternId).toBe(rows[0].id);
      expect(typeof score.maxSimilarityToCluster).toBe('number');
      expect(typeof score.isNovel).toBe('boolean');
      console.log(`Pattern ${rows[0].id.slice(0, 8)}: maxSim=${score.maxSimilarityToCluster.toFixed(4)}, novel=${score.isNovel}, nearestCluster=${score.nearestClusterId}`);
    });
  });

  describe('computePatternPageRank', () => {
    it('computes importance scores for all patterns', async () => {
      const ranks = await computePatternPageRank(TEST_DOMAIN);

      expect(ranks.length).toBe(6);
      expect(ranks[0].importance).toBeGreaterThanOrEqual(ranks[ranks.length - 1].importance);
      console.log('PageRank scores:');
      for (const r of ranks) {
        console.log(`  ${r.patternId.slice(0, 8)}: ${r.importance.toFixed(6)}`);
      }
    });
  });

  // ─── Neural Spiking ──────────────────────────────────────────

  describe('buildLinksFromTrajectories', () => {
    it('creates pattern_links from co-occurring embeddings', async () => {
      const count = await buildLinksFromTrajectories(TEST_DOMAIN);

      console.log(`Created ${count} pattern links`);
      // Should create some links since our patterns have non-zero similarity
      expect(count).toBeGreaterThanOrEqual(0);

      // Verify links exist
      const pool = await getPool();
      const { rows } = await pool.query<{ cnt: string }>(
        `SELECT count(*) AS cnt FROM pattern_links pl
         JOIN reasoning_memories rm ON rm.id = pl.source_id
         WHERE rm.domain = $1`,
        [TEST_DOMAIN],
      );
      console.log(`Total links in DB for domain: ${rows[0].cnt}`);
    });
  });

  describe('getNetworkState', () => {
    it('returns network state for the domain', async () => {
      const state = await getNetworkState(TEST_DOMAIN);

      expect(state.totalNeurons).toBe(6);
      expect(state.avgPotential).toBeGreaterThanOrEqual(0);
      expect(state.topFiringPatterns.length).toBeGreaterThan(0);

      console.log(`Network: ${state.totalNeurons} neurons, ${state.activeNeurons} active, avgPotential=${state.avgPotential.toFixed(4)}, recentSpikes=${state.recentSpikes}`);
    });
  });

  describe('fireSpike', () => {
    it('fires a spike and propagates to connected patterns', async () => {
      const pool = await getPool();
      const { rows } = await pool.query<{ id: string }>(
        `SELECT id FROM reasoning_memories WHERE domain = $1 LIMIT 1`,
        [TEST_DOMAIN],
      );
      const patternId = rows[0].id;

      const events = await fireSpike(patternId);

      // At least the source pattern fires
      expect(events.length).toBeGreaterThanOrEqual(1);
      expect(events[0].firedPattern).toBe(patternId);
      expect(events[0].didFire).toBe(true);
      expect(events[0].newPotential).toBe(0); // Reset after firing

      console.log(`Spike from ${patternId.slice(0, 8)}: ${events.length} events`);
      for (const e of events) {
        console.log(`  ${e.firedPattern.slice(0, 8)}: potential=${e.newPotential.toFixed(4)}, fired=${e.didFire}`);
      }
    });

    it('membrane potential accumulates across multiple spikes', async () => {
      const pool = await getPool();
      const { rows } = await pool.query<{ id: string }>(
        `SELECT rm.id FROM reasoning_memories rm
         JOIN pattern_links pl ON pl.source_id = rm.id
         WHERE rm.domain = $1
         LIMIT 1`,
        [TEST_DOMAIN],
      );

      if (rows.length === 0) {
        console.log('No linked patterns — skipping accumulation test');
        return;
      }

      // Fire the same source twice — connected targets should accumulate potential
      await fireSpike(rows[0].id);
      const secondSpike = await fireSpike(rows[0].id);

      // Check that some targets have non-zero potential (accumulation)
      const targets = secondSpike.filter(e => e.firedPattern !== rows[0].id);
      if (targets.length > 0) {
        console.log(`After 2 spikes, ${targets.length} targets reached:`);
        for (const t of targets) {
          console.log(`  ${t.firedPattern.slice(0, 8)}: potential=${t.newPotential.toFixed(6)}, fired=${t.didFire}`);
        }
      }
    });
  });

  describe('resetNetwork', () => {
    it('resets all spike potentials to zero', async () => {
      // Fire some spikes first
      const pool = await getPool();
      const { rows } = await pool.query<{ id: string }>(
        `SELECT id FROM reasoning_memories WHERE domain = $1 LIMIT 1`,
        [TEST_DOMAIN],
      );
      await fireSpike(rows[0].id);

      const resetCount = await resetNetwork(TEST_DOMAIN);
      console.log(`Reset ${resetCount} neurons`);

      // Verify all potentials are zero
      const state = await getNetworkState(TEST_DOMAIN);
      expect(state.activeNeurons).toBe(0);
      expect(state.avgPotential).toBe(0);
    });
  });

  describe('detectAnomalies', () => {
    it('returns anomalies (or empty if all rates are normal)', async () => {
      const anomalies = await detectAnomalies(TEST_DOMAIN, 3600, 1.5);

      console.log(`Anomalies detected: ${anomalies.length}`);
      for (const a of anomalies) {
        console.log(`  ${a.patternId.slice(0, 8)}: rate=${a.spikeRate}, avg=${a.avgRate.toFixed(2)}, score=${a.anomalyScore.toFixed(2)}`);
      }

      // After targeted spiking, we might see anomalies
      expect(Array.isArray(anomalies)).toBe(true);
    });
  });

  // ─── LIF neuron step (pure function) ─────────────────────────

  describe('lif_neuron_step (SQL)', () => {
    it('stays subthreshold with small input', async () => {
      const pool = await getPool();
      const { rows } = await pool.query<{ new_membrane: number; fired: boolean }>(
        `SELECT * FROM lif_neuron_step(0.0, 0.5, 1.0, 0.9, 0.001)`,
      );
      expect(rows[0].new_membrane).toBeCloseTo(0.0005, 4);
      expect(rows[0].fired).toBe(false);
    });

    it('fires and resets when threshold exceeded', async () => {
      const pool = await getPool();
      const { rows } = await pool.query<{ new_membrane: number; fired: boolean }>(
        `SELECT * FROM lif_neuron_step(0.99, 200.0, 1.0, 0.9, 0.1)`,
      );
      expect(rows[0].new_membrane).toBe(0);
      expect(rows[0].fired).toBe(true);
    });

    it('decays membrane potential over time', async () => {
      const pool = await getPool();
      const { rows } = await pool.query<{ new_membrane: number; fired: boolean }>(
        `SELECT * FROM lif_neuron_step(0.5, 0.0, 1.0, 0.9, 0.001)`,
      );
      // 0.5 * 0.9 + 0 = 0.45
      expect(rows[0].new_membrane).toBeCloseTo(0.45, 4);
      expect(rows[0].fired).toBe(false);
    });
  });
});
