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
//! ## Phase 26 storage integration (TODO)
//!
//! v1 keeps the in-memory map and a backing `Vec<Trajectory>` so the
//! capture/finalise contract is testable end-to-end. The `retrieve_similar`
//! path consults the same backing vector; production wiring into
//! [`crate::memory::HnswMemoryIndex`] needs a `summaries_iter()` accessor
//! which the Phase 26 cleanup pass will add. Until then, the function
//! returns matches from the local `STORE`.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use chrono::Utc;
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

struct Store {
    /// In-flight trajectories, keyed on `(surface as_str, event_id)`.
    in_flight: HashMap<(String, String), PartialState>,
    /// Completed trajectories. v1 stores them locally; Phase 26 cleanup
    /// will replace this with an HnswMemoryIndex handle.
    completed: Vec<(Trajectory, Vec<f32>)>,
}

fn store() -> &'static Mutex<Store> {
    static STORE: OnceLock<Mutex<Store>> = OnceLock::new();
    STORE.get_or_init(|| {
        Mutex::new(Store {
            in_flight: HashMap::new(),
            completed: Vec::new(),
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
    s.completed.push((trajectory.clone(), Vec::new()));
    Ok(trajectory)
}

/// Persist a previously-finalised trajectory together with its embedding.
///
/// Called by integration code that has its own embedding pipeline; the
/// trajectory itself is not mutated. v1 stores everything in the local
/// `STORE`; production wiring will route through Phase 26 HNSW.
pub fn persist_with_embedding(
    trajectory: Trajectory,
    embedding: Vec<f32>,
) -> CorpFinanceResult<()> {
    let mut s = store().lock().expect("self_learning store poisoned");
    s.completed.push((trajectory, embedding));
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

/// Retrieve the top-`limit` trajectories most similar to `query_embedding`,
/// filtered by `filter`.
///
/// Sorting key is `(eval_grade DESC, cosine_similarity DESC)` so the
/// highest-graded matches surface first. Returns up to `limit` results.
///
/// # TODO (Phase 26 cleanup)
///
/// When [`crate::memory::HnswMemoryIndex`] gains a `summaries_iter()`
/// accessor, route the candidate list through the HNSW graph for
/// sub-linear retrieval. v1 walks the local `STORE` linearly because the
/// alternative — committing trajectories to HNSW under the
/// `RunSummary` aggregate — would conflate two domain models.
pub fn retrieve_similar(
    query_embedding: &[f32],
    filter: &TrajectoryFilter,
    limit: usize,
) -> CorpFinanceResult<Vec<Trajectory>> {
    let s = store().lock().expect("self_learning store poisoned");
    let mut scored: Vec<(f32, &Trajectory)> = s
        .completed
        .iter()
        .filter(|(t, _)| {
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
        })
        .map(|(t, e)| (cosine_similarity(query_embedding, e), t))
        .collect();
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

/// Test-only: clear the process-local store. Tests use this to isolate
/// state between cases.
#[cfg(test)]
pub(crate) fn reset_store_for_tests() {
    let mut s = store().lock().expect("self_learning store poisoned");
    s.in_flight.clear();
    s.completed.clear();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::self_learning::types::SurfaceEventKind;

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
        reset_store_for_tests();
        capture_trajectory_step(Surface::Cli, "test1", step("a")).unwrap();
        capture_trajectory_step(Surface::Cli, "test1", step("b")).unwrap();
        let t = complete_trajectory(Surface::Cli, "test1", Some(EvalGrade::Good)).unwrap();
        assert_eq!(t.steps.len(), 2);
        assert_eq!(t.eval_grade, Some(EvalGrade::Good));
    }

    #[test]
    fn capture_with_empty_event_id_errors() {
        reset_store_for_tests();
        let err = capture_trajectory_step(Surface::Cli, "", step("a")).unwrap_err();
        assert!(matches!(err, CorpFinanceError::InvalidInput { .. }));
    }

    #[test]
    fn complete_without_capture_errors() {
        reset_store_for_tests();
        let err = complete_trajectory(Surface::Cli, "missing", None).unwrap_err();
        assert!(matches!(err, CorpFinanceError::InsufficientData(_)));
    }
}
