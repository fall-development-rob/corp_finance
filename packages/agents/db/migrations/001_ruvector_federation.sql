-- 001_ruvector_federation.sql
-- Schema for CFA agents using ruvector extension (pgvector-compatible, 384-dim all-MiniLM-L6-v2)

-- Enable ruvector extension (superset of pgvector)
CREATE EXTENSION IF NOT EXISTS ruvector;

-- ============================================================
-- Reasoning memories — primary knowledge store
-- ============================================================
CREATE TABLE IF NOT EXISTS reasoning_memories (
  id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  type          TEXT NOT NULL DEFAULT 'reasoning_memory'
                  CHECK (type IN ('reasoning_memory', 'episodic', 'semantic', 'pattern')),
  title         TEXT NOT NULL,
  description   TEXT,
  content       TEXT NOT NULL,
  domain        TEXT NOT NULL DEFAULT 'cfa-analysis',
  tags          TEXT[] DEFAULT '{}',
  source_json   JSONB DEFAULT '{}',
  confidence    REAL DEFAULT 0.5 CHECK (confidence >= 0 AND confidence <= 1),
  usage_count   INTEGER DEFAULT 0,
  embedding     ruvector(384),
  created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
  last_used_at  TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_rm_domain ON reasoning_memories (domain);
CREATE INDEX IF NOT EXISTS idx_rm_type ON reasoning_memories (type);
CREATE INDEX IF NOT EXISTS idx_rm_tags ON reasoning_memories USING GIN (tags);
CREATE INDEX IF NOT EXISTS idx_rm_created ON reasoning_memories (created_at DESC);

-- HNSW index for fast approximate nearest-neighbour search
CREATE INDEX IF NOT EXISTS idx_rm_embedding_hnsw
  ON reasoning_memories
  USING hnsw (embedding ruvector_cosine_ops)
  WITH (m = 16, ef_construction = 200);

-- ============================================================
-- Pattern embeddings — dedicated embedding store for patterns
-- ============================================================
CREATE TABLE IF NOT EXISTS pattern_embeddings (
  id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  memory_id     UUID NOT NULL REFERENCES reasoning_memories(id) ON DELETE CASCADE,
  model         TEXT NOT NULL DEFAULT 'all-MiniLM-L6-v2',
  dims          INTEGER NOT NULL DEFAULT 384,
  embedding     ruvector(384),
  created_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_pe_memory ON pattern_embeddings (memory_id);
CREATE INDEX IF NOT EXISTS idx_pe_embedding_hnsw
  ON pattern_embeddings
  USING hnsw (embedding ruvector_cosine_ops)
  WITH (m = 16, ef_construction = 200);

-- ============================================================
-- Pattern links — relationships between patterns
-- ============================================================
CREATE TABLE IF NOT EXISTS pattern_links (
  id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  source_id     UUID NOT NULL REFERENCES reasoning_memories(id) ON DELETE CASCADE,
  target_id     UUID NOT NULL REFERENCES reasoning_memories(id) ON DELETE CASCADE,
  link_type     TEXT NOT NULL DEFAULT 'related'
                  CHECK (link_type IN ('related', 'derived', 'contradicts', 'supersedes')),
  weight        REAL DEFAULT 1.0,
  created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
  UNIQUE (source_id, target_id, link_type)
);

CREATE INDEX IF NOT EXISTS idx_pl_source ON pattern_links (source_id);
CREATE INDEX IF NOT EXISTS idx_pl_target ON pattern_links (target_id);

-- ============================================================
-- Task trajectories — SONA learning traces
-- ============================================================
CREATE TABLE IF NOT EXISTS task_trajectories (
  id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  task_id         TEXT NOT NULL,
  agent_id        TEXT NOT NULL,
  query           TEXT,
  trajectory_json JSONB NOT NULL,
  judge_label     TEXT CHECK (judge_label IN ('Success', 'Failure', 'Partial')),
  judge_conf      REAL CHECK (judge_conf >= 0 AND judge_conf <= 1),
  started_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
  ended_at        TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_tt_task ON task_trajectories (task_id);
CREATE INDEX IF NOT EXISTS idx_tt_agent ON task_trajectories (agent_id);
CREATE INDEX IF NOT EXISTS idx_tt_started ON task_trajectories (started_at DESC);

-- ============================================================
-- Agent sessions — tracks active agent sessions
-- ============================================================
CREATE TABLE IF NOT EXISTS agent_sessions (
  id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  agent_id      TEXT NOT NULL,
  agent_type    TEXT NOT NULL,
  started_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
  ended_at      TIMESTAMPTZ,
  metadata      JSONB DEFAULT '{}'
);

CREATE INDEX IF NOT EXISTS idx_as_agent ON agent_sessions (agent_id);

-- ============================================================
-- Agent memories — per-agent working memory
-- ============================================================
CREATE TABLE IF NOT EXISTS agent_memories (
  id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  session_id    UUID REFERENCES agent_sessions(id) ON DELETE CASCADE,
  agent_id      TEXT NOT NULL,
  key           TEXT NOT NULL,
  value         TEXT NOT NULL,
  embedding     ruvector(384),
  created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
  expires_at    TIMESTAMPTZ,
  UNIQUE (agent_id, key)
);

CREATE INDEX IF NOT EXISTS idx_am_agent ON agent_memories (agent_id);
CREATE INDEX IF NOT EXISTS idx_am_session ON agent_memories (session_id);
CREATE INDEX IF NOT EXISTS idx_am_embedding_hnsw
  ON agent_memories
  USING hnsw (embedding ruvector_cosine_ops)
  WITH (m = 16, ef_construction = 200);

-- ============================================================
-- Agent tasks — task assignment tracking
-- ============================================================
CREATE TABLE IF NOT EXISTS agent_tasks (
  id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  task_id       TEXT NOT NULL,
  agent_id      TEXT NOT NULL,
  status        TEXT NOT NULL DEFAULT 'pending'
                  CHECK (status IN ('pending', 'in_progress', 'completed', 'failed', 'skipped')),
  input_json    JSONB DEFAULT '{}',
  output_json   JSONB,
  created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
  completed_at  TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_at_task ON agent_tasks (task_id);
CREATE INDEX IF NOT EXISTS idx_at_agent ON agent_tasks (agent_id);
CREATE INDEX IF NOT EXISTS idx_at_status ON agent_tasks (status);

-- ============================================================
-- Agent events — domain event log
-- ============================================================
CREATE TABLE IF NOT EXISTS agent_events (
  id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  event_type    TEXT NOT NULL,
  agent_id      TEXT,
  payload       JSONB NOT NULL DEFAULT '{}',
  created_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_ae_type ON agent_events (event_type);
CREATE INDEX IF NOT EXISTS idx_ae_agent ON agent_events (agent_id);
CREATE INDEX IF NOT EXISTS idx_ae_created ON agent_events (created_at DESC);

-- ============================================================
-- Migration tracking
-- ============================================================
CREATE TABLE IF NOT EXISTS schema_migrations (
  version       TEXT PRIMARY KEY,
  applied_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

INSERT INTO schema_migrations (version) VALUES ('001_ruvector_federation')
  ON CONFLICT (version) DO NOTHING;

-- ============================================================
-- Vector similarity search function
-- ============================================================
CREATE OR REPLACE FUNCTION search_reasoning_memories(
  query_embedding ruvector(384),
  match_domain    TEXT DEFAULT NULL,
  match_limit     INTEGER DEFAULT 10,
  min_similarity  REAL DEFAULT 0.0
)
RETURNS TABLE (
  id          UUID,
  title       TEXT,
  content     TEXT,
  domain      TEXT,
  tags        TEXT[],
  confidence  REAL,
  usage_count INTEGER,
  similarity  REAL
)
LANGUAGE sql STABLE AS $$
  SELECT
    rm.id,
    rm.title,
    rm.content,
    rm.domain,
    rm.tags,
    rm.confidence,
    rm.usage_count,
    1 - (rm.embedding <=> query_embedding) AS similarity
  FROM reasoning_memories rm
  WHERE rm.embedding IS NOT NULL
    AND (match_domain IS NULL OR rm.domain = match_domain)
    AND 1 - (rm.embedding <=> query_embedding) >= min_similarity
  ORDER BY rm.embedding <=> query_embedding
  LIMIT match_limit;
$$;
