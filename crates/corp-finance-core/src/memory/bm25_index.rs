//! BM25 keyword index for `RunSummary` records using `tantivy`.
//!
//! Schema:
//!
//! | Field             | Tantivy options       | Purpose                          |
//! |-------------------|-----------------------|----------------------------------|
//! | `run_id`          | `STRING \| STORED`    | Stable retrieval key (UUID v7).  |
//! | `surface_event_id`| `STRING \| STORED`    | Filter / display.                |
//! | `summary_text`    | `TEXT \| STORED`      | BM25 keyword search target.      |
//! | `surface`         | `STRING \| STORED`    | Filter / display.                |
//! | `ts`              | `I64 \| STORED`       | Unix-seconds timestamp filter.   |
//! | `tenant_id`       | `STRING \| STORED`    | Tenant scope filter.             |
//!
//! TODO(phase-27): add `MmapDirectory`-backed persistence so the BM25
//! index survives process restarts. Phase 26 ships an in-RAM index only;
//! restart rebuilds from the on-disk `RunSummary` records (see ADR-016
//! retention policy section).

use std::collections::HashMap;

use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::{Field, OwnedValue, Schema, Value, FAST, INDEXED, STORED, STRING, TEXT};
use tantivy::{doc, Index, IndexWriter, TantivyDocument};
use uuid::Uuid;

use crate::error::CorpFinanceError;
use crate::memory::types::{RunSummary, Surface};
use crate::CorpFinanceResult;

/// Default writer heap size — 50 MB is plenty for the v1 in-RAM index.
const WRITER_HEAP_BYTES: usize = 50_000_000;

/// A BM25 hit returned from [`BM25MemoryIndex::query`]: the canonical
/// `RunSummary` plus the tantivy-emitted BM25 relevance score for the
/// originating query.
///
/// The score is the unnormalised BM25 score returned by tantivy's
/// `TopDocs` collector; it is positive and increases with relevance, but
/// is not directly comparable across distinct queries (it depends on
/// query term IDF which differs per call).
#[derive(Debug, Clone, PartialEq)]
pub struct BM25Hit {
    pub run_summary: RunSummary,
    pub bm25_score: f32,
}

/// In-memory BM25 inverted index over `RunSummary` records.
pub struct BM25MemoryIndex {
    index: Index,
    f_run_id: Field,
    f_surface_event_id: Field,
    f_summary_text: Field,
    f_surface: Field,
    f_ts: Field,
    f_tenant_id: Field,
    /// Side map so query results can rebuild full `RunSummary` records
    /// (tantivy's stored values are flat and lossy for vector / chrono).
    summaries: HashMap<Uuid, RunSummary>,
}

impl BM25MemoryIndex {
    /// Build a new in-RAM BM25 index.
    pub fn new() -> CorpFinanceResult<Self> {
        let mut sb = Schema::builder();
        let f_run_id = sb.add_text_field("run_id", STRING | STORED);
        let f_surface_event_id = sb.add_text_field("surface_event_id", STRING | STORED);
        let f_summary_text = sb.add_text_field("summary_text", TEXT | STORED);
        let f_surface = sb.add_text_field("surface", STRING | STORED);
        let f_ts = sb.add_i64_field("ts", INDEXED | STORED | FAST);
        let f_tenant_id = sb.add_text_field("tenant_id", STRING | STORED);
        let schema = sb.build();
        let index = Index::create_in_ram(schema);
        Ok(Self {
            index,
            f_run_id,
            f_surface_event_id,
            f_summary_text,
            f_surface,
            f_ts,
            f_tenant_id,
            summaries: HashMap::new(),
        })
    }

    /// Number of records currently indexed (per side store).
    pub fn len(&self) -> usize {
        self.summaries.len()
    }

    /// Whether the index is empty.
    pub fn is_empty(&self) -> bool {
        self.summaries.is_empty()
    }

    /// Insert a `RunSummary` into the inverted index.
    pub fn ingest(&mut self, summary: &RunSummary) -> CorpFinanceResult<()> {
        let mut writer: IndexWriter = self
            .index
            .writer(WRITER_HEAP_BYTES)
            .map_err(tantivy_to_cf)?;
        let tenant = summary.tenant_id.clone().unwrap_or_default();
        writer
            .add_document(doc!(
                self.f_run_id => summary.run_id.to_string(),
                self.f_surface_event_id => summary.surface_event_id.clone(),
                self.f_summary_text => summary.summary_text.clone(),
                self.f_surface => summary.surface.as_str().to_string(),
                self.f_ts => summary.ts.timestamp(),
                self.f_tenant_id => tenant,
            ))
            .map_err(tantivy_to_cf)?;
        writer.commit().map_err(tantivy_to_cf)?;
        self.summaries.insert(summary.run_id, summary.clone());
        Ok(())
    }

    /// BM25 query with optional tenant filter.
    ///
    /// Returns up to `limit` matching [`BM25Hit`] records, each carrying
    /// the canonical `RunSummary` plus the tantivy-emitted BM25 score.
    /// Hits are ordered by descending `bm25_score` (most relevant first).
    /// The tenant filter is applied post-retrieval to keep the query
    /// parser simple.
    ///
    // TODO(Phase-28-Wave-2): adjust callers for the `BM25Hit` shape.
    // Phase 28 cleanup widened the return type from `Vec<RunSummary>` to
    // `Vec<BM25Hit>` so the tantivy BM25 score is no longer dropped on
    // the floor (Phase 26 NAPI agent's TODO). Known downstream callers
    // that need updating in the Wave 2 NAPI / CLI pass:
    //   - `packages/bindings/src/lib.rs::surface_memory_find` (~line 2836)
    //     — currently iterates `for s in results` expecting `RunSummary`;
    //     replace with `for hit in results` and read `hit.run_summary` /
    //     `hit.bm25_score`. The placeholder `score: 0.0` can become the
    //     real BM25 score now.
    //   - `crates/corp-finance-cli/src/commands/memory.rs::run_find`
    //     (~line 182) — the `raw` iteration needs `.into_iter().map(|h|
    //     h.run_summary)` to recover the existing filter pipeline shape.
    // Both are out of scope for the Phase 28 cleanup pass per the task
    // contract; the Wave 2 agent owns the caller wiring.
    pub fn query(
        &self,
        query_text: &str,
        limit: usize,
        filter_tenant: Option<&str>,
    ) -> CorpFinanceResult<Vec<BM25Hit>> {
        if query_text.trim().is_empty() {
            return Ok(Vec::new());
        }
        let reader = self.index.reader().map_err(tantivy_to_cf)?;
        let searcher = reader.searcher();
        let qp = QueryParser::for_index(&self.index, vec![self.f_summary_text]);
        let q = qp
            .parse_query(query_text)
            .map_err(|e| CorpFinanceError::InvalidInput {
                field: "query_text".into(),
                reason: format!("parse: {e}"),
            })?;
        let over = limit.saturating_mul(4).max(limit).max(1);
        let hits = searcher
            .search(&q, &TopDocs::with_limit(over))
            .map_err(tantivy_to_cf)?;

        let mut out: Vec<BM25Hit> = Vec::with_capacity(limit);
        for (score, addr) in hits {
            let stored: TantivyDocument = searcher.doc(addr).map_err(tantivy_to_cf)?;
            let run_id_str = match stored.get_first(self.f_run_id) {
                Some(v) => owned_str(v).unwrap_or_default(),
                None => continue,
            };
            let run_id = match Uuid::parse_str(&run_id_str) {
                Ok(u) => u,
                Err(_) => continue,
            };
            if let Some(summary) = self.summaries.get(&run_id) {
                if let Some(want) = filter_tenant {
                    if summary.tenant_id.as_deref() != Some(want) {
                        continue;
                    }
                }
                out.push(BM25Hit {
                    run_summary: summary.clone(),
                    bm25_score: score,
                });
                if out.len() >= limit {
                    break;
                }
            }
        }
        Ok(out)
    }

    /// Filter helper for surface scoping — used by the hybrid retriever.
    pub fn matches_surface(summary: &RunSummary, want: Option<Surface>) -> bool {
        match want {
            Some(s) => summary.surface == s,
            None => true,
        }
    }
}

fn tantivy_to_cf(e: tantivy::TantivyError) -> CorpFinanceError {
    CorpFinanceError::SerializationError(format!("tantivy: {e}"))
}

/// Extract a string from a stored `OwnedValue` reference, if it is text.
fn owned_str(v: &OwnedValue) -> Option<String> {
    v.as_str().map(|s| s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mk_summary(text: &str, surface_event_id: &str) -> RunSummary {
        RunSummary::new(
            Surface::Mcp,
            surface_event_id,
            "djb2:0xaaaa",
            text,
            vec![0.0, 0.0, 0.0],
        )
    }

    #[test]
    fn ingest_then_query_finds_doc() {
        let mut idx = BM25MemoryIndex::new().unwrap();
        let s = mk_summary("Apple revenue beat consensus", "earnings");
        idx.ingest(&s).unwrap();
        let res = idx.query("apple revenue", 5, None).unwrap();
        assert!(!res.is_empty());
        assert_eq!(res[0].run_summary.run_id, s.run_id);
    }

    #[test]
    fn bm25_score_is_nonzero_for_relevant_hits() {
        let mut idx = BM25MemoryIndex::new().unwrap();
        idx.ingest(&mk_summary(
            "AAPL Q3 earnings beat consensus on iPhone revenue",
            "earnings",
        ))
        .unwrap();
        let hits = idx.query("earnings", 10, None).unwrap();
        assert!(!hits.is_empty());
        // tantivy emits a positive BM25 score for any matching document;
        // a zero score would mean the score plumbing was lost.
        assert!(
            hits[0].bm25_score > 0.0,
            "expected positive BM25 score, got {}",
            hits[0].bm25_score
        );
    }
}
