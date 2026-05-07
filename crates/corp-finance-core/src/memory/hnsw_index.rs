//! HNSW vector store for `RunSummary` embeddings.
//!
//! Wraps [`hnsw_rs::hnsw::Hnsw`] with a side `HashMap<Uuid, RunSummary>`
//! holding the canonical record for each indexed point. The HNSW graph is
//! treated as a derived view: the side map is the ground truth, the graph
//! is rebuilt from scratch on `load_from`. This keeps the on-disk format
//! crate-version-agnostic across `hnsw_rs` minor bumps.
//!
//! Defaults match institutional vector-search practice:
//!
//! - `M = 16` — number of bidirectional links per element.
//! - `ef_construction = 200` — search width during construction.
//! - `max_layer = 16` — clamped by `hnsw_rs` to its internal `NB_LAYER_MAX`.
//! - `max_elements = 100_000` — sizing hint, not a hard cap.

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::Path;

use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use hnsw_rs::prelude::{DistL2, Hnsw};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::CorpFinanceError;
use crate::memory::types::RunSummary;
use crate::CorpFinanceResult;

/// Default M (bidirectional links per element) — institutional-grade.
pub const DEFAULT_M: usize = 16;

/// Default ef_construction — institutional-grade.
pub const DEFAULT_EF_CONSTRUCTION: usize = 200;

/// Default sizing hint for the HNSW allocator.
pub const DEFAULT_MAX_ELEMENTS: usize = 100_000;

/// Default max layer value (`hnsw_rs` clamps to its own ceiling internally).
pub const DEFAULT_MAX_LAYER: usize = 16;

/// On-disk envelope version for `save_to` / `load_from`.
const ENVELOPE_VERSION: u32 = 1;

/// Parameters baked into the on-disk envelope so `load_from` can reconstruct
/// the index without external configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct HnswParams {
    m: usize,
    ef_construction: usize,
    embedding_dim: usize,
}

#[derive(Debug, Serialize, Deserialize)]
struct HnswEnvelope {
    version: u32,
    params: HnswParams,
    summaries: Vec<RunSummary>,
}

/// Vector store over `RunSummary` embeddings using HNSW (`hnsw_rs`).
///
/// `query()` returns `(RunSummary, distance)` pairs sorted ascending by L2
/// distance. The `filter` callback handles tenant and surface scoping
/// at the recall boundary so the underlying graph stays single-namespace.
pub struct HnswMemoryIndex {
    hnsw: Hnsw<'static, f32, DistL2>,
    /// `data_id` (the integer key inserted into HNSW) -> `run_id`.
    id_to_uuid: Vec<Uuid>,
    /// `run_id` -> canonical `RunSummary` record.
    summaries: HashMap<Uuid, RunSummary>,
    embedding_dim: usize,
    m: usize,
    ef_construction: usize,
}

impl HnswMemoryIndex {
    /// Build a new in-memory HNSW index with institutional-grade defaults.
    pub fn new(embedding_dim: usize) -> Self {
        Self::with_params(
            embedding_dim,
            DEFAULT_M,
            DEFAULT_EF_CONSTRUCTION,
            DEFAULT_MAX_ELEMENTS,
        )
    }

    /// Build a new in-memory HNSW index with caller-supplied parameters.
    pub fn with_params(
        embedding_dim: usize,
        m: usize,
        ef_construction: usize,
        max_elements: usize,
    ) -> Self {
        let hnsw = Hnsw::<f32, DistL2>::new(
            m,
            max_elements,
            DEFAULT_MAX_LAYER,
            ef_construction,
            DistL2 {},
        );
        Self {
            hnsw,
            id_to_uuid: Vec::new(),
            summaries: HashMap::new(),
            embedding_dim,
            m,
            ef_construction,
        }
    }

    /// Number of records currently indexed.
    pub fn len(&self) -> usize {
        self.summaries.len()
    }

    /// Whether the index is empty.
    pub fn is_empty(&self) -> bool {
        self.summaries.is_empty()
    }

    /// Configured embedding dimensionality.
    pub fn embedding_dim(&self) -> usize {
        self.embedding_dim
    }

    /// Insert a `RunSummary` into the HNSW graph and the side store.
    ///
    /// Validates dimension and rejects duplicate `run_id`s (RUF-MEM-INV-001
    /// "no duplicate run ids" invariant).
    pub fn ingest(&mut self, summary: &RunSummary) -> CorpFinanceResult<()> {
        if summary.embedding.len() != self.embedding_dim {
            return Err(CorpFinanceError::InvalidInput {
                field: "embedding".into(),
                reason: format!(
                    "expected dim {}, got {}",
                    self.embedding_dim,
                    summary.embedding.len()
                ),
            });
        }
        if self.summaries.contains_key(&summary.run_id) {
            return Err(CorpFinanceError::InvalidInput {
                field: "run_id".into(),
                reason: format!("duplicate run_id {}", summary.run_id),
            });
        }
        let data_id = self.id_to_uuid.len();
        self.hnsw.insert((&summary.embedding, data_id));
        self.id_to_uuid.push(summary.run_id);
        self.summaries.insert(summary.run_id, summary.clone());
        Ok(())
    }

    /// Top-K vector query with a caller-supplied filter callback.
    ///
    /// Returns `(RunSummary, distance)` pairs sorted ascending by distance.
    /// The filter is applied post-retrieval so the underlying graph stays
    /// single-namespace; tenant scoping and surface filters live here.
    pub fn query<F>(&self, embedding: &[f32], limit: usize, filter: F) -> Vec<(RunSummary, f32)>
    where
        F: Fn(&RunSummary) -> bool,
    {
        if embedding.len() != self.embedding_dim || self.summaries.is_empty() {
            return Vec::new();
        }
        // Over-fetch so that filtering can still produce `limit` records.
        let knbn = limit.saturating_mul(4).max(limit).max(1);
        let ef_search = self.ef_construction.max(knbn);
        let neighbours = self.hnsw.search(embedding, knbn, ef_search);

        let mut out: Vec<(RunSummary, f32)> = Vec::with_capacity(limit);
        for n in neighbours {
            if let Some(uuid) = self.id_to_uuid.get(n.d_id) {
                if let Some(summary) = self.summaries.get(uuid) {
                    if filter(summary) {
                        out.push((summary.clone(), n.distance));
                        if out.len() >= limit {
                            break;
                        }
                    }
                }
            }
        }
        out
    }

    /// Persist the side store + HNSW parameters to a gzip+JSON envelope.
    ///
    /// The HNSW graph itself is not serialised — it is rebuilt from the
    /// side store on `load_from`. See module docs for the rationale.
    pub fn save_to(&self, path: &Path) -> CorpFinanceResult<()> {
        let envelope = HnswEnvelope {
            version: ENVELOPE_VERSION,
            params: HnswParams {
                m: self.m,
                ef_construction: self.ef_construction,
                embedding_dim: self.embedding_dim,
            },
            summaries: self
                .id_to_uuid
                .iter()
                .filter_map(|u| self.summaries.get(u).cloned())
                .collect(),
        };
        let json = serde_json::to_vec(&envelope)?;
        let file = File::create(path).map_err(io_to_cf)?;
        let mut encoder = GzEncoder::new(BufWriter::new(file), Compression::default());
        encoder.write_all(&json).map_err(io_to_cf)?;
        encoder.finish().map_err(io_to_cf)?;
        Ok(())
    }

    /// Restore an index from a gzip+JSON envelope produced by `save_to`.
    pub fn load_from(path: &Path) -> CorpFinanceResult<Self> {
        let file = File::open(path).map_err(io_to_cf)?;
        let mut decoder = GzDecoder::new(BufReader::new(file));
        let mut buf = Vec::new();
        decoder.read_to_end(&mut buf).map_err(io_to_cf)?;
        let envelope: HnswEnvelope = serde_json::from_slice(&buf)?;
        if envelope.version != ENVELOPE_VERSION {
            return Err(CorpFinanceError::InvalidInput {
                field: "envelope.version".into(),
                reason: format!(
                    "unsupported envelope version {} (expected {})",
                    envelope.version, ENVELOPE_VERSION
                ),
            });
        }
        let mut idx = Self::with_params(
            envelope.params.embedding_dim,
            envelope.params.m,
            envelope.params.ef_construction,
            DEFAULT_MAX_ELEMENTS.max(envelope.summaries.len()),
        );
        for s in envelope.summaries {
            idx.ingest(&s)?;
        }
        Ok(idx)
    }
}

fn io_to_cf(e: std::io::Error) -> CorpFinanceError {
    CorpFinanceError::SerializationError(format!("io: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::types::Surface;

    fn mk_summary(text: &str, emb: Vec<f32>) -> RunSummary {
        RunSummary::new(Surface::Mcp, "dcf_calc", "djb2:0xaaaa", text, emb)
    }

    #[test]
    fn ingest_then_query_returns_inserted() {
        let mut idx = HnswMemoryIndex::new(3);
        let s = mk_summary("hello", vec![1.0, 0.0, 0.0]);
        idx.ingest(&s).unwrap();
        let res = idx.query(&[1.0, 0.0, 0.0], 1, |_| true);
        assert_eq!(res.len(), 1);
        assert_eq!(res[0].0.run_id, s.run_id);
    }

    #[test]
    fn dim_mismatch_rejected() {
        let mut idx = HnswMemoryIndex::new(3);
        let s = mk_summary("x", vec![1.0, 0.0]);
        assert!(idx.ingest(&s).is_err());
    }
}
