//! Memory bounded context for the CFA agent platform (Phase 26).
//!
//! Implements ADR-016 (memory architecture) and the
//! `feature_memory.yml` contracts (RUF-MEM-001..010 + RUF-MEM-INV-001..005).
//!
//! The bounded context owns the canonical record of CFA runtime activity at
//! the four runtime surfaces declared in ADR-015 — `Cli`, `Mcp`, `Skill`,
//! and `Plugin` — plus the indexing and retrieval substrate that makes
//! cross-session retrieval ("most similar past run") possible.
//!
//! ## Module layout
//!
//! - [`types`] — domain types (`RunSummary`, `Surface`, `EntityRef`,
//!   `EntityKind`, `CfaSession`, `MemoryQuery`).
//! - [`hnsw_index`] — HNSW vector store wrapping [`hnsw_rs`] with a
//!   side `HashMap<Uuid, RunSummary>` for record retrieval.
//! - [`bm25_index`] — BM25 keyword inverted index wrapping [`tantivy`].
//! - [`cfa_session`] — portable session archive (JSON + `flate2` gzip).
//!
//! Entity graph and tag-based entity extraction live in the multi-agent
//! coordination context at [`crate::multi_agent::entity_graph`] (Phase 27);
//! the Phase 26 placeholder stub that previously lived here was removed in
//! Phase 28 cleanup. Memory tests that exercise entity extraction now reach
//! into the multi-agent module directly under `cfg(feature = "multi_agent")`.
//!
//! ## On-disk persistence format (HNSW + summary side-store)
//!
//! `HnswMemoryIndex::save_to(path)` writes a single gzip-compressed JSON
//! envelope containing:
//!
//! - `version: u32` — currently `1`.
//! - `params: { m, ef_construction, embedding_dim }`.
//! - `summaries: Vec<RunSummary>` — one entry per ingested record.
//!
//! On `load_from(path)` the HNSW graph is rebuilt by re-inserting every
//! summary's embedding into a fresh in-memory index. The graph itself is
//! not serialised; the summary side-store is the ground truth and the
//! graph is treated as a derived view that can always be rebuilt — this
//! keeps the format crate-version-agnostic across `hnsw_rs` minor bumps.
//!
//! ## Feature gating
//!
//! The whole module is gated behind the `memory` cargo feature at the
//! crate root (`lib.rs`). Internal files do not repeat the gate.

pub mod bm25_index;
pub mod cfa_session;
pub mod hnsw_index;
pub mod types;

#[cfg(test)]
mod tests;

pub use bm25_index::{BM25Hit, BM25MemoryIndex};
pub use cfa_session::{restore as restore_session, round_trip_test_helper, save as save_session};
pub use hnsw_index::HnswMemoryIndex;
pub use types::{CfaSession, EntityKind, EntityRef, MemoryQuery, RunSummary, Surface};
