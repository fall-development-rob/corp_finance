-- 003_graph_spiking.sql
-- ADR-002 extension: Graph mincut partitioning + neural spiking (LIF/STDP)
-- Built on ruvector-postgres graph and attention SQL primitives

-- ============================================================
-- Schema additions for mincut partitioning
-- ============================================================

-- Partition assignment and coherence score on reasoning_memories
ALTER TABLE reasoning_memories
  ADD COLUMN IF NOT EXISTS mincut_partition INTEGER,
  ADD COLUMN IF NOT EXISTS coherence_score REAL DEFAULT 0.0;

-- Index for partition-scoped queries
CREATE INDEX IF NOT EXISTS idx_rm_partition
  ON reasoning_memories (mincut_partition, coherence_score DESC)
  WHERE mincut_partition IS NOT NULL;

-- ============================================================
-- Schema additions for neural spiking (LIF + STDP)
-- ============================================================

-- Spiking state on reasoning_memories (membrane potential, spike timing)
ALTER TABLE reasoning_memories
  ADD COLUMN IF NOT EXISTS spike_potential REAL DEFAULT 0.0,
  ADD COLUMN IF NOT EXISTS last_spike_at TIMESTAMPTZ;

-- Plasticity columns on pattern_links (STDP weight modulation)
ALTER TABLE pattern_links
  ADD COLUMN IF NOT EXISTS spike_count INTEGER DEFAULT 0,
  ADD COLUMN IF NOT EXISTS last_activation TIMESTAMPTZ,
  ADD COLUMN IF NOT EXISTS plasticity_weight REAL DEFAULT 1.0;

-- Index for temporal pattern queries (STDP window lookups)
CREATE INDEX IF NOT EXISTS idx_pl_activation
  ON pattern_links (last_activation DESC)
  WHERE spike_count > 0;

-- Index for spike event detection
CREATE INDEX IF NOT EXISTS idx_rm_spike
  ON reasoning_memories (last_spike_at DESC)
  WHERE last_spike_at IS NOT NULL;

-- ============================================================
-- Helper: convert ruvector column to real[] for graph primitives
-- ruvector has no direct cast to real[], so we go through text
-- ============================================================
CREATE OR REPLACE FUNCTION ruvector_to_real_array(v ruvector)
RETURNS real[]
LANGUAGE sql IMMUTABLE STRICT AS $$
  SELECT string_to_array(trim(both '[]' from v::text), ',')::real[];
$$;

-- ============================================================
-- Mincut helper: build similarity edges between pattern embeddings
-- Uses ruvector's graph_edge_similarity(real[], real[]) primitive
-- ============================================================
CREATE OR REPLACE FUNCTION build_pattern_edges(
  p_domain TEXT,
  p_threshold REAL DEFAULT 0.3
)
RETURNS TABLE (
  source_id UUID,
  target_id UUID,
  similarity REAL
)
LANGUAGE sql STABLE AS $$
  SELECT
    a.id AS source_id,
    b.id AS target_id,
    graph_edge_similarity(ruvector_to_real_array(a.embedding), ruvector_to_real_array(b.embedding)) AS similarity
  FROM reasoning_memories a
  CROSS JOIN reasoning_memories b
  WHERE a.id < b.id
    AND a.embedding IS NOT NULL
    AND b.embedding IS NOT NULL
    AND a.domain = p_domain
    AND b.domain = p_domain
    AND graph_edge_similarity(ruvector_to_real_array(a.embedding), ruvector_to_real_array(b.embedding)) >= p_threshold;
$$;

-- ============================================================
-- Stoer-Wagner mincut implementation in PL/pgSQL
-- Operates on a weighted adjacency list stored as temp table
-- Returns the minimum cut value (weight of edges crossing the cut)
-- ============================================================
CREATE OR REPLACE FUNCTION compute_mincut(
  p_domain TEXT,
  p_threshold REAL DEFAULT 0.3
)
RETURNS TABLE (
  cut_value REAL,
  partition_a UUID[],
  partition_b UUID[]
)
LANGUAGE plpgsql AS $$
DECLARE
  n INTEGER;
  best_cut REAL := 'Infinity';
  best_partition_a UUID[];
  best_partition_b UUID[];
  merged_into UUID;
  last_added UUID;
  second_last UUID;
  phase_cut REAL;
  rec RECORD;
BEGIN
  -- Build adjacency into temp table
  CREATE TEMP TABLE IF NOT EXISTS _mc_nodes (
    node_id UUID PRIMARY KEY,
    merged_group UUID[] DEFAULT '{}'
  ) ON COMMIT DROP;

  CREATE TEMP TABLE IF NOT EXISTS _mc_edges (
    src UUID,
    dst UUID,
    w REAL,
    PRIMARY KEY (src, dst)
  ) ON COMMIT DROP;

  TRUNCATE _mc_nodes, _mc_edges;

  -- Populate nodes
  INSERT INTO _mc_nodes (node_id, merged_group)
  SELECT id, ARRAY[id]
  FROM reasoning_memories
  WHERE domain = p_domain AND embedding IS NOT NULL;

  GET DIAGNOSTICS n = ROW_COUNT;
  IF n < 2 THEN
    cut_value := 0;
    partition_a := ARRAY(SELECT node_id FROM _mc_nodes);
    partition_b := '{}';
    RETURN NEXT;
    RETURN;
  END IF;

  -- Populate edges using graph_edge_similarity (bidirectional)
  INSERT INTO _mc_edges (src, dst, w)
  SELECT source_id, target_id, similarity
  FROM build_pattern_edges(p_domain, p_threshold)
  UNION ALL
  SELECT target_id, source_id, similarity
  FROM build_pattern_edges(p_domain, p_threshold)
  ON CONFLICT (src, dst) DO UPDATE SET w = EXCLUDED.w;

  -- Stoer-Wagner: n-1 phases
  FOR phase IN 1..n-1 LOOP
    -- Maximum adjacency ordering (greedy)
    CREATE TEMP TABLE IF NOT EXISTS _mc_order (
      step INTEGER,
      node_id UUID
    ) ON COMMIT DROP;
    TRUNCATE _mc_order;

    -- Start with arbitrary node
    INSERT INTO _mc_order (step, node_id)
    SELECT 1, node_id FROM _mc_nodes LIMIT 1;

    FOR i IN 2..(SELECT count(*) FROM _mc_nodes) LOOP
      INSERT INTO _mc_order (step, node_id)
      SELECT i, e.dst
      FROM _mc_edges e
      JOIN _mc_order o ON e.src = o.node_id
      WHERE e.dst NOT IN (SELECT node_id FROM _mc_order)
      GROUP BY e.dst
      ORDER BY sum(e.w) DESC
      LIMIT 1;

      -- If no connected node found, pick any remaining
      IF NOT FOUND THEN
        INSERT INTO _mc_order (step, node_id)
        SELECT i, n2.node_id
        FROM _mc_nodes n2
        WHERE n2.node_id NOT IN (SELECT node_id FROM _mc_order)
        LIMIT 1;
      END IF;
    END LOOP;

    -- Last two added
    SELECT node_id INTO last_added FROM _mc_order ORDER BY step DESC LIMIT 1;
    SELECT node_id INTO second_last FROM _mc_order ORDER BY step DESC OFFSET 1 LIMIT 1;

    IF last_added IS NULL OR second_last IS NULL THEN EXIT; END IF;

    -- Cut-of-the-phase: sum of edges to last_added
    SELECT coalesce(sum(w), 0) INTO phase_cut
    FROM _mc_edges WHERE dst = last_added;

    IF phase_cut < best_cut THEN
      best_cut := phase_cut;
      -- Partition: merged_group of last_added vs everything else
      SELECT merged_group INTO best_partition_a
      FROM _mc_nodes WHERE node_id = last_added;
      best_partition_b := ARRAY(
        SELECT unnest(merged_group) FROM _mc_nodes WHERE node_id != last_added
      );
    END IF;

    -- Merge last_added into second_last
    UPDATE _mc_nodes
    SET merged_group = merged_group || (SELECT merged_group FROM _mc_nodes WHERE node_id = last_added)
    WHERE node_id = second_last;

    -- Merge parallel edges: sum weights where redirect would cause PK conflict, then redirect
    -- First, sum weights for edges that would become duplicates after redirect
    UPDATE _mc_edges e1
    SET w = e1.w + e2.w
    FROM _mc_edges e2
    WHERE e1.dst = second_last AND e2.dst = last_added
      AND e1.src = e2.src AND e1.src != second_last;

    UPDATE _mc_edges e1
    SET w = e1.w + e2.w
    FROM _mc_edges e2
    WHERE e1.src = second_last AND e2.src = last_added
      AND e1.dst = e2.dst AND e1.dst != second_last;

    -- Delete edges that would conflict (they've been merged into existing edges above)
    DELETE FROM _mc_edges WHERE dst = last_added
      AND src IN (SELECT src FROM _mc_edges WHERE dst = second_last);
    DELETE FROM _mc_edges WHERE src = last_added
      AND dst IN (SELECT dst FROM _mc_edges WHERE src = second_last);

    -- Now redirect remaining edges safely (no conflicts)
    UPDATE _mc_edges SET dst = second_last WHERE dst = last_added AND src != second_last;
    UPDATE _mc_edges SET src = second_last WHERE src = last_added AND dst != second_last;

    -- Remove self-loops and the merged node
    DELETE FROM _mc_edges WHERE src = dst;
    DELETE FROM _mc_edges WHERE src = last_added OR dst = last_added;
    DELETE FROM _mc_nodes WHERE node_id = last_added;
  END LOOP;

  cut_value := best_cut;
  partition_a := best_partition_a;
  partition_b := best_partition_b;
  RETURN NEXT;
END;
$$;

-- ============================================================
-- LIF neuron step: leaky integrate-and-fire membrane update
-- Uses attention_score for input current computation
-- ============================================================
CREATE OR REPLACE FUNCTION lif_neuron_step(
  p_membrane REAL,
  p_input_current REAL,
  p_threshold REAL DEFAULT 1.0,
  p_decay REAL DEFAULT 0.9,
  p_dt REAL DEFAULT 0.001
)
RETURNS TABLE (
  new_membrane REAL,
  fired BOOLEAN
)
LANGUAGE sql IMMUTABLE AS $$
  SELECT
    CASE
      WHEN (p_membrane * p_decay + p_input_current * p_dt) >= p_threshold THEN 0.0
      ELSE (p_membrane * p_decay + p_input_current * p_dt)
    END AS new_membrane,
    (p_membrane * p_decay + p_input_current * p_dt) >= p_threshold AS fired;
$$;

-- ============================================================
-- Process spike event: fire a spike for a pattern, propagate
-- to connected patterns via pattern_links with STDP update
-- ============================================================
CREATE OR REPLACE FUNCTION process_spike_event(
  p_pattern_id UUID,
  p_current_time TIMESTAMPTZ DEFAULT now()
)
RETURNS TABLE (
  fired_pattern UUID,
  new_potential REAL,
  did_fire BOOLEAN
)
LANGUAGE plpgsql AS $$
DECLARE
  rec RECORD;
  input_current REAL;
  step_result RECORD;
  stdp_window INTERVAL := interval '100 milliseconds';
  delta_t REAL;
  weight_delta REAL;
BEGIN
  -- Mark the source pattern as spiked
  UPDATE reasoning_memories
  SET spike_potential = 0.0, last_spike_at = p_current_time
  WHERE id = p_pattern_id;

  fired_pattern := p_pattern_id;
  new_potential := 0.0;
  did_fire := true;
  RETURN NEXT;

  -- Propagate to connected patterns
  FOR rec IN
    SELECT pl.target_id, pl.weight * pl.plasticity_weight AS effective_weight,
           rm.spike_potential AS membrane, rm.embedding, rm.last_spike_at
    FROM pattern_links pl
    JOIN reasoning_memories rm ON rm.id = pl.target_id
    WHERE pl.source_id = p_pattern_id
      AND pl.link_type IN ('related', 'derived')
  LOOP
    -- Input current from attention_score between source and target embeddings
    IF rec.embedding IS NOT NULL THEN
      SELECT attention_score(
        ruvector_to_real_array((SELECT embedding FROM reasoning_memories WHERE id = p_pattern_id)),
        ruvector_to_real_array(rec.embedding)
      ) * rec.effective_weight INTO input_current;
    ELSE
      input_current := rec.effective_weight * 0.1;
    END IF;

    -- LIF update
    SELECT * INTO step_result FROM lif_neuron_step(
      coalesce(rec.membrane, 0.0), input_current
    );

    -- Update target membrane potential
    UPDATE reasoning_memories
    SET spike_potential = step_result.new_membrane,
        last_spike_at = CASE WHEN step_result.fired THEN p_current_time ELSE last_spike_at END
    WHERE id = rec.target_id;

    -- STDP: update plasticity weight based on spike timing
    IF rec.last_spike_at IS NOT NULL THEN
      delta_t := EXTRACT(EPOCH FROM (p_current_time - rec.last_spike_at));
      IF delta_t > 0 AND delta_t < 0.1 THEN
        -- Pre before post: potentiation (strengthen)
        weight_delta := 0.01 * exp(-delta_t / 0.02);
      ELSIF delta_t < 0 AND delta_t > -0.1 THEN
        -- Post before pre: depression (weaken)
        weight_delta := -0.005 * exp(delta_t / 0.02);
      ELSE
        weight_delta := 0.0;
      END IF;

      UPDATE pattern_links
      SET plasticity_weight = greatest(0.1, least(5.0, plasticity_weight + weight_delta)),
          spike_count = spike_count + 1,
          last_activation = p_current_time
      WHERE source_id = p_pattern_id AND target_id = rec.target_id;
    END IF;

    fired_pattern := rec.target_id;
    new_potential := step_result.new_membrane;
    did_fire := step_result.fired;
    RETURN NEXT;
  END LOOP;
END;
$$;

-- ============================================================
-- Detect spike anomalies: patterns with abnormal firing rates
-- ============================================================
CREATE OR REPLACE FUNCTION detect_spike_anomalies(
  p_domain TEXT,
  p_window_seconds INTEGER DEFAULT 3600,
  p_threshold_stddev REAL DEFAULT 2.0
)
RETURNS TABLE (
  pattern_id UUID,
  spike_rate REAL,
  avg_rate REAL,
  stddev_rate REAL,
  anomaly_score REAL
)
LANGUAGE sql STABLE AS $$
  WITH spike_rates AS (
    SELECT
      rm.id,
      coalesce(
        (SELECT count(*)::real FROM pattern_links pl
         WHERE pl.target_id = rm.id
           AND pl.last_activation > now() - (p_window_seconds || ' seconds')::interval
        ), 0
      ) AS rate
    FROM reasoning_memories rm
    WHERE rm.domain = p_domain AND rm.embedding IS NOT NULL
  ),
  stats AS (
    SELECT avg(rate) AS avg_r, stddev_pop(rate) AS std_r FROM spike_rates
  )
  SELECT
    sr.id AS pattern_id,
    sr.rate AS spike_rate,
    s.avg_r AS avg_rate,
    s.std_r AS stddev_rate,
    CASE WHEN s.std_r > 0 THEN (sr.rate - s.avg_r) / s.std_r ELSE 0 END AS anomaly_score
  FROM spike_rates sr, stats s
  WHERE s.std_r > 0 AND abs(sr.rate - s.avg_r) / s.std_r > p_threshold_stddev
  ORDER BY abs(sr.rate - s.avg_r) / s.std_r DESC;
$$;

-- Record migration
INSERT INTO schema_migrations (version) VALUES ('003_graph_spiking')
  ON CONFLICT (version) DO NOTHING;
