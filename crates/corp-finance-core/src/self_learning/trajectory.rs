//! Trajectory capture and retrieval at the surface boundary.
//!
//! Wires the four CFA surface wrappers (CLI, MCP, Skill, Plugin) into the
//! Self-Learning bounded context. Capture is a two-step protocol:
//!
//! 1. [`capture_trajectory_step`] — append one [`SurfaceEventRef`] to the
//!    in-memory record keyed on `(surface, surface_event_id)`.
//! 2. [`complete_trajectory`] — flush the in-memory record into a final
//!    immutable [`Trajectory`], assigning the [`EvalGrade`] and clearing
//!    the in-memory slot.
//!
//! Retrieval is one call:
//!
//! - [`retrieve_similar`] — returns trajectories with embedding cosine
//!   similarity above the implicit floor and matching the supplied
//!   [`TrajectoryFilter`].
//!
//! ## Storage
//!
//! Finalised trajectories live in a process-local store with two indexes:
//!
//! - `by_id` (`BTreeMap<Uuid, (Trajectory, Vec<f32>)>`) — canonical record
//!   for every trajectory, used for filter-only retrieval (zero-vector
//!   queries) and trajectory-id lookup.
//! - `hnsw` (`Option<TrajectoryHnswIndex>`) — `hnsw_rs` graph keyed on
//!   `data_id -> trajectory_id` for sub-linear nearest-neighbour retrieval
//!   when a non-zero query embedding is supplied. Built lazily on the
//!   first non-empty `persist_with_embedding` call so the dimensionality
//!   is discovered from real data.
//!
//! Per the ADR-020 separation, `Trajectory` is its own aggregate; we do
//! not reuse `crate::memory::HnswMemoryIndex` (which is keyed on
//! `RunSummary`). Co-locating the HNSW handle with the trajectory store
//! keeps both aggregates pure.

use std::collections::{BTreeMap, HashMap};
use std::sync::{Mutex, OnceLock};

use chrono::Utc;
use hnsw_rs::prelude::{DistL2, Hnsw};
use uuid::Uuid;

use crate::error::CorpFinanceError;
use crate::self_learning::types::{EvalGrade, SurfaceEventRef, Trajectory};
use crate::surface::Surface;
use crate::CorpFinanceResult;

/// Filter applied at trajectory retrieval time.
///
/// Per `RUF-LEARN-INV-009`, every read API in `self_learning` accepts an
/// optional `tenant_id`; production callers must supply one. The
/// `eval_grade_min` field implements the floor required by
/// `RUF-LEARN-INV-003`.
#[derive(Debug, Clone, Default)]
pub struct TrajectoryFilter {
    pub surface: Option<Surface>,
    pub eval_grade_min: Option<EvalGrade>,
    pub tenant_id: Option<String>,
}

impl TrajectoryFilter {
    /// Default filter: no surface scope, no grade floor, no tenant scope.
    /// Used by tests; production callers must populate `tenant_id`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Builder-style setter for the surface scope.
    pub fn with_surface(mut self, s: Surface) -> Self {
        self.surface = Some(s);
        self
    }

    /// Builder-style setter for the minimum eval grade.
    pub fn with_eval_grade_min(mut self, g: EvalGrade) -> Self {
        self.eval_grade_min = Some(g);
        self
    }

    /// Builder-style setter for the tenant scope.
    pub fn with_tenant_id(mut self, id: impl Into<String>) -> Self {
        self.tenant_id = Some(id.into());
        self
    }
}

// ---------------------------------------------------------------------------
// Process-local store
// ---------------------------------------------------------------------------

/// In-memory partial-trajectory state keyed on `(surface, surface_event_id)`.
struct PartialState {
    surface: Surface,
    surface_event_id: String,
    steps: Vec<SurfaceEventRef>,
    tenant_id: Option<String>,
    started_at: chrono::DateTime<Utc>,
}

/// HNSW index over trajectory embeddings, keyed by `trajectory_id`.
///
/// Lazily constructed on the first non-empty embedding insert (the dim
/// is discovered from the first vector). Subsequent inserts must match
/// the established dim.
struct TrajectoryHnswIndex {
    hnsw: Hnsw<'static, f32, DistL2>,
    /// `data_id` (the integer key inserted into HNSW) -> `trajectory_id`.
    id_to_uuid: Vec<Uuid>,
    embedding_dim: usize,
    ef_construction: usize,
}

/// HNSW defaults — match institutional vector-search practice and the
/// Phase 26 `memory::HnswMemoryIndex` constants so trajectory and
/// `RunSummary` retrieval behave consistently.
const HNSW_M: usize = 16;
const HNSW_EF_CONSTRUCTION: usize = 200;
const HNSW_MAX_LAYER: usize = 16;
const HNSW_MAX_ELEMENTS: usize = 100_000;

impl TrajectoryHnswIndex {
    fn new(embedding_dim: usize) -> Self {
        let hnsw = Hnsw::<f32, DistL2>::new(
            HNSW_M,
            HNSW_MAX_ELEMENTS,
            HNSW_MAX_LAYER,
            HNSW_EF_CONSTRUCTION,
            DistL2 {},
        );
        Self {
            hnsw,
            id_to_uuid: Vec::new(),
            embedding_dim,
            ef_construction: HNSW_EF_CONSTRUCTION,
        }
    }

    /// Insert one trajectory embedding into the graph. Errors on dim
    /// mismatch — caller controls the embedding pipeline and should
    /// supply consistent dims.
    fn insert(&mut self, trajectory_id: Uuid, embedding: &[f32]) -> CorpFinanceResult<()> {
        if embedding.len() != self.embedding_dim {
            return Err(CorpFinanceError::InvalidInput {
                field: "embedding".into(),
                reason: format!(
                    "trajectory hnsw expects dim {}, got {}",
                    self.embedding_dim,
                    embedding.len()
                ),
            });
        }
        let data_id = self.id_to_uuid.len();
        self.hnsw.insert((embedding, data_id));
        self.id_to_uuid.push(trajectory_id);
        Ok(())
    }

    /// Return the top-`k` candidate trajectory ids ordered ascending by
    /// L2 distance. The caller re-scores with cosine similarity to match
    /// pre-HNSW ordering semantics.
    fn query_top_k(&self, query: &[f32], k: usize) -> Vec<Uuid> {
        if query.len() != self.embedding_dim || self.id_to_uuid.is_empty() {
            return Vec::new();
        }
        let knbn = k.max(1);
        let ef_search = self.ef_construction.max(knbn);
        let neighbours = self.hnsw.search(query, knbn, ef_search);
        neighbours
            .into_iter()
            .filter_map(|n| self.id_to_uuid.get(n.d_id).copied())
            .collect()
    }
}

struct Store {
    /// In-flight trajectories, keyed on `(surface as_str, event_id)`.
    in_flight: HashMap<(String, String), PartialState>,
    /// Canonical record per finalised trajectory: every completed trajectory
    /// lives here regardless of whether an embedding was supplied. The
    /// `BTreeMap` keying gives deterministic iteration order (UUID v7 is
    /// time-sortable, so this is also start-time order).
    by_id: BTreeMap<Uuid, (Trajectory, Vec<f32>)>,
    /// Lazily-built HNSW index over non-empty embeddings. `None` until the
    /// first non-empty embedding insert establishes the dim.
    hnsw: Option<TrajectoryHnswIndex>,
}

fn store() -> &'static Mutex<Store> {
    static STORE: OnceLock<Mutex<Store>> = OnceLock::new();
    STORE.get_or_init(|| {
        Mutex::new(Store {
            in_flight: HashMap::new(),
            by_id: BTreeMap::new(),
            hnsw: None,
        })
    })
}

fn key(surface: Surface, surface_event_id: &str) -> (String, String) {
    (surface.as_str().to_string(), surface_event_id.to_string())
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Maximum step count permitted on a single trajectory before the API
/// rejects further appends. Per `RUF-LEARN-INV` step-count cap. Set high
/// enough that no realistic CFA session reaches it; protects against
/// runaway capture loops.
pub const MAX_TRAJECTORY_STEPS: usize = 1024;

/// Append one surface event to the trajectory keyed on
/// `(surface, surface_event_id)`.
///
/// If no in-flight trajectory exists for the key, one is created and the
/// step is the first.
///
/// # Errors
///
/// - Returns [`CorpFinanceError::InvalidInput`] if `surface_event_id` is
///   empty (per `RUF-LEARN-008`, a trajectory step requires a non-empty
///   event id).
/// - Returns [`CorpFinanceError::FinancialImpossibility`] (used as the
///   domain error envelope) if the in-flight trajectory already has
///   [`MAX_TRAJECTORY_STEPS`] steps.
pub fn capture_trajectory_step(
    surface: Surface,
    surface_event_id: &str,
    step: SurfaceEventRef,
) -> CorpFinanceResult<()> {
    if surface_event_id.is_empty() {
        return Err(CorpFinanceError::InvalidInput {
            field: "surface_event_id".into(),
            reason: "trajectory step requires non-empty surface_event_id".into(),
        });
    }
    let mut s = store().lock().expect("self_learning store poisoned");
    let entry = s
        .in_flight
        .entry(key(surface, surface_event_id))
        .or_insert_with(|| PartialState {
            surface,
            surface_event_id: surface_event_id.to_string(),
            steps: Vec::new(),
            tenant_id: None,
            started_at: Utc::now(),
        });
    if entry.steps.len() >= MAX_TRAJECTORY_STEPS {
        return Err(CorpFinanceError::FinancialImpossibility(format!(
            "trajectory step cap {MAX_TRAJECTORY_STEPS} exceeded for {}/{}",
            surface.as_str(),
            surface_event_id
        )));
    }
    entry.steps.push(step);
    Ok(())
}

/// Attach a tenant id to the in-flight trajectory keyed on
/// `(surface, surface_event_id)`. No-op if no in-flight record exists.
///
/// Per `RUF-LEARN-013`, every trajectory carries a tenant id; the
/// production wrappers populate it as soon as the surface event's tenant
/// scope is resolved.
pub fn attach_tenant(
    surface: Surface,
    surface_event_id: &str,
    tenant_id: impl Into<String>,
) -> CorpFinanceResult<()> {
    let mut s = store().lock().expect("self_learning store poisoned");
    if let Some(entry) = s.in_flight.get_mut(&key(surface, surface_event_id)) {
        entry.tenant_id = Some(tenant_id.into());
    }
    Ok(())
}

/// Finalise the in-flight trajectory keyed on `(surface, surface_event_id)`.
///
/// Returns the resulting immutable [`Trajectory`] with the supplied
/// `eval_grade`. The in-flight slot is cleared on success.
///
/// # Errors
///
/// - Returns [`CorpFinanceError::InsufficientData`] if no in-flight
///   trajectory exists for the key (no steps were captured).
/// - Returns [`CorpFinanceError::InvalidInput`] if the in-flight record
///   has zero steps.
pub fn complete_trajectory(
    surface: Surface,
    surface_event_id: &str,
    eval_grade: Option<EvalGrade>,
) -> CorpFinanceResult<Trajectory> {
    let mut s = store().lock().expect("self_learning store poisoned");
    let partial = s.in_flight.remove(&key(surface, surface_event_id)).ok_or(
        CorpFinanceError::InsufficientData(format!(
            "no in-flight trajectory for {}/{}",
            surface.as_str(),
            surface_event_id
        )),
    )?;
    if partial.steps.is_empty() {
        return Err(CorpFinanceError::InvalidInput {
            field: "steps".into(),
            reason: "trajectory must have at least one step".into(),
        });
    }
    let trajectory = Trajectory {
        trajectory_id: Uuid::now_v7(),
        surface: partial.surface,
        surface_event_id: partial.surface_event_id,
        steps: partial.steps,
        eval_grade,
        tenant_id: partial.tenant_id,
        ts: partial.started_at,
    };
    s.by_id
        .insert(trajectory.trajectory_id, (trajectory.clone(), Vec::new()));
    Ok(trajectory)
}

/// Persist a previously-finalised trajectory together with its embedding.
///
/// Called by integration code that has its own embedding pipeline; the
/// trajectory itself is not mutated. The canonical record always lands
/// in `by_id`; non-empty embeddings additionally land in the HNSW
/// graph for sub-linear nearest-neighbour retrieval.
///
/// # Errors
///
/// - Returns [`CorpFinanceError::InvalidInput`] if `embedding` is non-empty
///   and its dimensionality differs from the dim established by the first
///   embedding insert into the process-local index.
pub fn persist_with_embedding(
    trajectory: Trajectory,
    embedding: Vec<f32>,
) -> CorpFinanceResult<()> {
    let mut s = store().lock().expect("self_learning store poisoned");
    let trajectory_id = trajectory.trajectory_id;
    s.by_id
        .insert(trajectory_id, (trajectory, embedding.clone()));
    if !embedding.is_empty() {
        if s.hnsw.is_none() {
            s.hnsw = Some(TrajectoryHnswIndex::new(embedding.len()));
        }
        let idx = s
            .hnsw
            .as_mut()
            .expect("hnsw index just initialised in branch above");
        idx.insert(trajectory_id, &embedding)?;
    }
    Ok(())
}

/// Cosine similarity between two equal-length f32 vectors.
///
/// Returns `0.0` when either vector has zero norm or dimensions
/// disagree — the retrieval path treats these candidates as non-matches.
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let mut dot = 0.0f32;
    let mut na = 0.0f32;
    let mut nb = 0.0f32;
    for i in 0..a.len() {
        dot += a[i] * b[i];
        na += a[i] * a[i];
        nb += b[i] * b[i];
    }
    if na == 0.0 || nb == 0.0 {
        return 0.0;
    }
    dot / (na.sqrt() * nb.sqrt())
}

/// Over-fetch factor when consulting HNSW. We retrieve `4×limit`
/// candidates and re-rank with cosine similarity + eval-grade so a
/// single graph pass still yields `limit` results after filter and
/// re-sort.
const HNSW_OVERFETCH: usize = 4;

/// Retrieve the top-`limit` trajectories most similar to `query_embedding`,
/// filtered by `filter`.
///
/// Sorting key is `(eval_grade DESC, cosine_similarity DESC)` so the
/// highest-graded matches surface first. Returns up to `limit` results.
///
/// # Retrieval path
///
/// - When `query_embedding` is non-empty and an HNSW index has been
///   built (i.e. at least one [`persist_with_embedding`] call supplied
///   a non-empty embedding) **and** the query dim matches the index dim,
///   the function pulls `4×limit` candidates from HNSW (`O(log N)`
///   expected), re-scores each with cosine similarity, applies the
///   filter, and sorts by `(grade, similarity)`.
/// - Otherwise (zero-vector query, no graph, or dim mismatch) it falls
///   back to a linear scan over `by_id` so callers without an embedding
///   pipeline still get filter-only retrieval.
pub fn retrieve_similar(
    query_embedding: &[f32],
    filter: &TrajectoryFilter,
    limit: usize,
) -> CorpFinanceResult<Vec<Trajectory>> {
    let s = store().lock().expect("self_learning store poisoned");

    // Decide: HNSW path (sub-linear) or linear scan.
    let use_hnsw = !query_embedding.is_empty()
        && s.hnsw
            .as_ref()
            .map(|idx| idx.embedding_dim == query_embedding.len() && !idx.id_to_uuid.is_empty())
            .unwrap_or(false);

    let candidates: Vec<(&Trajectory, f32)> = if use_hnsw {
        let idx = s.hnsw.as_ref().expect("hnsw existence checked above");
        let knbn = limit.saturating_mul(HNSW_OVERFETCH).max(limit).max(1);
        let ids = idx.query_top_k(query_embedding, knbn);
        ids.into_iter()
            .filter_map(|tid| s.by_id.get(&tid))
            .filter(|(t, _)| filter_matches(t, filter))
            .map(|(t, e)| (t, cosine_similarity(query_embedding, e)))
            .collect()
    } else {
        s.by_id
            .values()
            .filter(|(t, _)| filter_matches(t, filter))
            .map(|(t, e)| (t, cosine_similarity(query_embedding, e)))
            .collect()
    };

    let mut scored: Vec<(f32, &Trajectory)> =
        candidates.into_iter().map(|(t, sim)| (sim, t)).collect();
    // Stable sort by (eval_grade DESC, similarity DESC).
    scored.sort_by(|a, b| {
        let ag = a.1.eval_grade.unwrap_or(EvalGrade::Failed);
        let bg = b.1.eval_grade.unwrap_or(EvalGrade::Failed);
        bg.cmp(&ag)
            .then(b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal))
    });
    Ok(scored
        .into_iter()
        .take(limit)
        .map(|(_, t)| t.clone())
        .collect())
}

/// Predicate factored out so the HNSW and linear paths share filter logic.
fn filter_matches(t: &Trajectory, filter: &TrajectoryFilter) -> bool {
    if let Some(surf) = filter.surface {
        if t.surface != surf {
            return false;
        }
    }
    if let Some(min) = filter.eval_grade_min {
        match t.eval_grade {
            Some(g) if g >= min => {}
            _ => return false,
        }
    }
    if let Some(ref tid) = filter.tenant_id {
        if t.tenant_id.as_deref() != Some(tid.as_str()) {
            return false;
        }
    }
    true
}

/// Test-only: clear the process-local store. Tests use this to isolate
/// state between cases.
#[cfg(test)]
pub(crate) fn reset_store_for_tests() {
    let mut s = store().lock().expect("self_learning store poisoned");
    s.in_flight.clear();
    s.by_id.clear();
    s.hnsw = None;
}

/// Test-only: serialising guard for the process-local trajectory store.
///
/// Returns a mutex guard that callers hold for the lifetime of one test.
/// Because the store is global (a `OnceLock<Mutex<Store>>`), two tests
/// running concurrently can otherwise interleave each other's
/// `reset_store_for_tests` + capture + complete cycles. Every test that
/// touches the store should hold this guard up front:
///
/// ```ignore
/// let _g = trajectory::lock_test_store();
/// reset_store_for_tests();
/// // ... test body ...
/// ```
///
/// Recovers from poisoning so a panic in one test does not cascade-fail
/// every following test.
#[cfg(test)]
pub(crate) fn lock_test_store() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();
    let l = LOCK.get_or_init(|| std::sync::Mutex::new(()));
    match l.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::self_learning::types::SurfaceEventKind;

    /// Local alias for the module-level test serialiser; lets each test
    /// acquire the guard with `let _guard = lock_for_test();`.
    fn lock_for_test() -> std::sync::MutexGuard<'static, ()> {
        super::lock_test_store()
    }

    fn step(name: &str) -> SurfaceEventRef {
        SurfaceEventRef {
            kind: SurfaceEventKind::McpTool,
            name: name.into(),
            input_hash: "h".into(),
            output_hash: Some("o".into()),
            duration_ms: 10,
        }
    }

    #[test]
    fn capture_then_complete_returns_trajectory_with_steps() {
        let _guard = lock_for_test();
        reset_store_for_tests();
        capture_trajectory_step(Surface::Cli, "test1", step("a")).unwrap();
        capture_trajectory_step(Surface::Cli, "test1", step("b")).unwrap();
        let t = complete_trajectory(Surface::Cli, "test1", Some(EvalGrade::Good)).unwrap();
        assert_eq!(t.steps.len(), 2);
        assert_eq!(t.eval_grade, Some(EvalGrade::Good));
    }

    #[test]
    fn capture_with_empty_event_id_errors() {
        let _guard = lock_for_test();
        reset_store_for_tests();
        let err = capture_trajectory_step(Surface::Cli, "", step("a")).unwrap_err();
        assert!(matches!(err, CorpFinanceError::InvalidInput { .. }));
    }

    #[test]
    fn complete_without_capture_errors() {
        let _guard = lock_for_test();
        reset_store_for_tests();
        let err = complete_trajectory(Surface::Cli, "missing", None).unwrap_err();
        assert!(matches!(err, CorpFinanceError::InsufficientData(_)));
    }

    fn finalised(name: &str, grade: EvalGrade) -> Trajectory {
        capture_trajectory_step(Surface::Cli, name, step("a")).unwrap();
        complete_trajectory(Surface::Cli, name, Some(grade)).unwrap()
    }

    #[test]
    fn persist_with_empty_embedding_does_not_build_hnsw() {
        let _guard = lock_for_test();
        reset_store_for_tests();
        let t = finalised("empty-emb", EvalGrade::Good);
        persist_with_embedding(t.clone(), Vec::new()).unwrap();
        // Linear path used (zero query embedding) — match still returned.
        let hits = retrieve_similar(&[], &TrajectoryFilter::new(), 5).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].trajectory_id, t.trajectory_id);
    }

    #[test]
    fn persist_with_non_empty_embedding_builds_hnsw_lazily() {
        let _guard = lock_for_test();
        reset_store_for_tests();
        let t1 = finalised("emb-1", EvalGrade::Good);
        persist_with_embedding(t1.clone(), vec![1.0, 0.0, 0.0]).unwrap();
        let t2 = finalised("emb-2", EvalGrade::Excellent);
        persist_with_embedding(t2.clone(), vec![0.0, 1.0, 0.0]).unwrap();

        // HNSW path: query parallel to t1 — t1 should beat t2 on cosine.
        let hits = retrieve_similar(&[1.0, 0.0, 0.0], &TrajectoryFilter::new(), 2).unwrap();
        assert_eq!(hits.len(), 2);
        // Excellent (t2) outranks Good (t1) on grade-DESC sort.
        assert_eq!(hits[0].trajectory_id, t2.trajectory_id);
        assert_eq!(hits[1].trajectory_id, t1.trajectory_id);
    }

    #[test]
    fn hnsw_dim_mismatch_on_insert_errors() {
        let _guard = lock_for_test();
        reset_store_for_tests();
        let t1 = finalised("dim-1", EvalGrade::Good);
        persist_with_embedding(t1, vec![1.0, 0.0, 0.0]).unwrap();
        let t2 = finalised("dim-2", EvalGrade::Good);
        let err = persist_with_embedding(t2, vec![1.0, 0.0]).unwrap_err();
        assert!(matches!(err, CorpFinanceError::InvalidInput { .. }));
    }

    #[test]
    fn retrieve_dim_mismatch_falls_back_to_linear() {
        let _guard = lock_for_test();
        reset_store_for_tests();
        let t1 = finalised("falls-back-1", EvalGrade::Good);
        persist_with_embedding(t1.clone(), vec![1.0, 0.0, 0.0]).unwrap();
        // Query with a 4-dim vector — index dim is 3 — falls back to linear,
        // which still returns the filter-matching record.
        let hits = retrieve_similar(&[1.0, 0.0, 0.0, 0.0], &TrajectoryFilter::new(), 5).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].trajectory_id, t1.trajectory_id);
    }

    #[test]
    fn retrieve_filter_by_grade_min_excludes_below() {
        let _guard = lock_for_test();
        reset_store_for_tests();
        let bad = finalised("low-grade", EvalGrade::Poor);
        persist_with_embedding(bad.clone(), vec![1.0, 0.0, 0.0]).unwrap();
        let good = finalised("good-grade", EvalGrade::Good);
        persist_with_embedding(good.clone(), vec![0.9, 0.1, 0.0]).unwrap();

        let filter = TrajectoryFilter::new().with_eval_grade_min(EvalGrade::Acceptable);
        let hits = retrieve_similar(&[1.0, 0.0, 0.0], &filter, 5).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].trajectory_id, good.trajectory_id);
    }

    #[test]
    fn retrieve_filter_by_tenant_excludes_other_tenants() {
        let _guard = lock_for_test();
        reset_store_for_tests();
        // Build two trajectories with distinct tenant ids.
        capture_trajectory_step(Surface::Cli, "tenant-a-traj", step("a")).unwrap();
        attach_tenant(Surface::Cli, "tenant-a-traj", "tenant-a").unwrap();
        let a = complete_trajectory(Surface::Cli, "tenant-a-traj", Some(EvalGrade::Good)).unwrap();
        persist_with_embedding(a.clone(), vec![1.0, 0.0, 0.0]).unwrap();

        capture_trajectory_step(Surface::Cli, "tenant-b-traj", step("b")).unwrap();
        attach_tenant(Surface::Cli, "tenant-b-traj", "tenant-b").unwrap();
        let b = complete_trajectory(Surface::Cli, "tenant-b-traj", Some(EvalGrade::Good)).unwrap();
        persist_with_embedding(b.clone(), vec![1.0, 0.0, 0.0]).unwrap();

        let filter = TrajectoryFilter::new().with_tenant_id("tenant-a");
        let hits = retrieve_similar(&[1.0, 0.0, 0.0], &filter, 5).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].trajectory_id, a.trajectory_id);
    }
}
