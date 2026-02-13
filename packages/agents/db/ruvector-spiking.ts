// ruvector-spiking — Neural spiking (LIF + STDP) via ruvector attention primitives
// Implements leaky integrate-and-fire neurons for pattern activation tracking
// and spike-timing dependent plasticity for link weight modulation.
// Uses ruvector's attention_score(), attention_softmax(), lif_neuron_step(),
// process_spike_event(), and detect_spike_anomalies() SQL functions.

import { queryWithRetry } from './pg-client.js';

export interface SpikeEvent {
  firedPattern: string;
  newPotential: number;
  didFire: boolean;
}

export interface SpikeAnomaly {
  patternId: string;
  spikeRate: number;
  avgRate: number;
  stddevRate: number;
  anomalyScore: number;
}

export interface NetworkState {
  totalNeurons: number;
  activeNeurons: number;
  avgPotential: number;
  recentSpikes: number;
  topFiringPatterns: Array<{ patternId: string; potential: number; lastSpike: Date | null }>;
}

export interface AttentionWeights {
  patternId: string;
  attentionScore: number;
  normalizedWeight: number;
}

/**
 * Fire a spike event for a pattern, propagating activation
 * through pattern_links with STDP weight modulation.
 * Uses the process_spike_event() PL/pgSQL function which internally
 * calls attention_score() for input current and lif_neuron_step() for updates.
 */
export async function fireSpike(patternId: string): Promise<SpikeEvent[]> {
  const { rows } = await queryWithRetry<{
    fired_pattern: string;
    new_potential: number;
    did_fire: boolean;
  }>(
    'SELECT * FROM process_spike_event($1)',
    [patternId],
  );

  return rows.map(r => ({
    firedPattern: r.fired_pattern,
    newPotential: r.new_potential,
    didFire: r.did_fire,
  }));
}

/**
 * Detect patterns with abnormal firing rates (spike anomalies).
 * Returns patterns whose spike rate deviates beyond the threshold
 * from the population mean.
 */
export async function detectAnomalies(
  domain: string,
  windowSeconds = 3600,
  thresholdStddev = 2.0,
): Promise<SpikeAnomaly[]> {
  const { rows } = await queryWithRetry<{
    pattern_id: string;
    spike_rate: number;
    avg_rate: number;
    stddev_rate: number;
    anomaly_score: number;
  }>(
    'SELECT * FROM detect_spike_anomalies($1, $2, $3)',
    [domain, windowSeconds, thresholdStddev],
  );

  return rows.map(r => ({
    patternId: r.pattern_id,
    spikeRate: r.spike_rate,
    avgRate: r.avg_rate,
    stddevRate: r.stddev_rate,
    anomalyScore: r.anomaly_score,
  }));
}

/**
 * Get the current spiking network state for a domain.
 */
export async function getNetworkState(
  domain: string,
  topN = 10,
): Promise<NetworkState> {
  const { rows } = await queryWithRetry<{
    total_neurons: string;
    active_neurons: string;
    avg_potential: number;
    recent_spikes: string;
  }>(
    `SELECT
       count(*) AS total_neurons,
       count(*) FILTER (WHERE spike_potential > 0.1) AS active_neurons,
       coalesce(avg(spike_potential), 0) AS avg_potential,
       count(*) FILTER (WHERE last_spike_at > now() - interval '1 hour') AS recent_spikes
     FROM reasoning_memories
     WHERE domain = $1 AND embedding IS NOT NULL`,
    [domain],
  );

  const { rows: topFiring } = await queryWithRetry<{
    id: string;
    spike_potential: number;
    last_spike_at: Date | null;
  }>(
    `SELECT id, spike_potential, last_spike_at
     FROM reasoning_memories
     WHERE domain = $1 AND embedding IS NOT NULL
     ORDER BY spike_potential DESC
     LIMIT $2`,
    [domain, topN],
  );

  const r = rows[0];
  return {
    totalNeurons: Number(r.total_neurons),
    activeNeurons: Number(r.active_neurons),
    avgPotential: r.avg_potential,
    recentSpikes: Number(r.recent_spikes),
    topFiringPatterns: topFiring.map(p => ({
      patternId: p.id,
      potential: p.spike_potential,
      lastSpike: p.last_spike_at,
    })),
  };
}

/**
 * Compute attention-weighted pattern scores for a query embedding.
 * Uses ruvector's attention_score() and attention_softmax() primitives
 * to produce attention weights across patterns, modulated by spike potential.
 */
export async function computeSpikeAttention(
  queryEmbedding: string,
  domain: string,
  limit = 10,
): Promise<AttentionWeights[]> {
  const { rows } = await queryWithRetry<{
    id: string;
    raw_score: number;
    normalized_weight: number;
  }>(
    `WITH scored AS (
       SELECT
         rm.id,
         attention_score($1::real[], ruvector_to_real_array(rm.embedding))
           * (1.0 + rm.spike_potential) AS raw_score
       FROM reasoning_memories rm
       WHERE rm.domain = $2
         AND rm.embedding IS NOT NULL
       ORDER BY raw_score DESC
       LIMIT $3
     ),
     weights AS (
       SELECT id, raw_score,
              unnest(attention_softmax(array_agg(raw_score) OVER ())) AS normalized_weight
       FROM scored
     )
     SELECT id, raw_score, normalized_weight
     FROM weights
     ORDER BY normalized_weight DESC`,
    [queryEmbedding, domain, limit],
  );

  return rows.map(r => ({
    patternId: r.id,
    attentionScore: r.raw_score,
    normalizedWeight: r.normalized_weight,
  }));
}

/**
 * Reset all spike potentials in a domain (quiescent state).
 */
export async function resetNetwork(domain: string): Promise<number> {
  const { rowCount } = await queryWithRetry(
    `UPDATE reasoning_memories
     SET spike_potential = 0.0
     WHERE domain = $1 AND spike_potential > 0`,
    [domain],
  );
  return rowCount ?? 0;
}

/**
 * Create pattern links from co-occurring patterns in task trajectories.
 * Patterns that appear in the same trajectory are linked with 'related' type.
 * Uses existing pattern_links table for STDP weight modulation.
 */
export async function buildLinksFromTrajectories(
  domain: string,
): Promise<number> {
  const { rowCount } = await queryWithRetry(
    `INSERT INTO pattern_links (source_id, target_id, link_type, weight)
     SELECT DISTINCT a.id, b.id, 'related', 
       graph_edge_similarity(ruvector_to_real_array(a.embedding), ruvector_to_real_array(b.embedding))
     FROM reasoning_memories a
     JOIN reasoning_memories b ON a.id < b.id
       AND a.domain = $1 AND b.domain = $1
       AND a.embedding IS NOT NULL AND b.embedding IS NOT NULL
       AND a.fingerprint IS NOT NULL AND b.fingerprint IS NOT NULL
     WHERE graph_is_connected(ruvector_to_real_array(a.embedding), ruvector_to_real_array(b.embedding), 0.3)
     ON CONFLICT (source_id, target_id, link_type) DO UPDATE
       SET weight = EXCLUDED.weight`,
    [domain],
  );
  return rowCount ?? 0;
}
