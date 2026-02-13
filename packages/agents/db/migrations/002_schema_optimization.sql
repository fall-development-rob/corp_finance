-- 002_schema_optimization.sql
-- ADR-002: Schema optimization — drop unused HNSW indexes, add partial index,
-- fingerprint constraint, and updated_at trigger

-- a. Drop unused HNSW indexes — saves memory and eliminates insert overhead
DROP INDEX IF EXISTS idx_pe_embedding_hnsw;
DROP INDEX IF EXISTS idx_am_embedding_hnsw;

-- b. Add partial index for pattern lookups by domain + recency
CREATE INDEX IF NOT EXISTS idx_rm_patterns
  ON reasoning_memories (domain, created_at DESC)
  WHERE type = 'reasoning_memory';

-- c. Add updated_at trigger
CREATE OR REPLACE FUNCTION update_updated_at()
RETURNS TRIGGER AS $$
BEGIN NEW.updated_at = now(); RETURN NEW; END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_rm_updated_at ON reasoning_memories;
CREATE TRIGGER trg_rm_updated_at
  BEFORE UPDATE ON reasoning_memories
  FOR EACH ROW EXECUTE FUNCTION update_updated_at();

-- d. Add fingerprint column and unique partial index for pattern deduplication
ALTER TABLE reasoning_memories
  ADD COLUMN IF NOT EXISTS fingerprint TEXT;

CREATE UNIQUE INDEX IF NOT EXISTS idx_rm_fingerprint
  ON reasoning_memories (fingerprint)
  WHERE fingerprint IS NOT NULL;

-- Record migration
INSERT INTO schema_migrations (version) VALUES ('002_schema_optimization')
  ON CONFLICT (version) DO NOTHING;
