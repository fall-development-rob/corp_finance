//! Self-Learning bounded context (Phase 28).
//!
//! Implements ADR-020 (native self-learning loop over CLI / MCP / skill /
//! plugin surface events) and the `feature_self_learning.yml` contracts
//! (`RUF-LEARN-001..013` plus the `RUF-LEARN-INV-001..012` invariants).
//!
//! The bounded context consumes Phase 26 (memory store: HNSW + BM25 +
//! petgraph) for trajectory persistence and Phase 27 (multi_agent: GOAP
//! planner + entity graph) for the planner action space. It produces:
//!
//! - Trajectory capture and retrieval at the four CFA surface boundaries.
//! - K-means trajectory clustering (the SONA-equivalent) for planning bias.
//! - Replay-driven contract tests with byte-diff + structural-diff drift
//!   detection.
//! - Golden-set freeze + restore with ed25519-signed manifests.
//! - First-run keypair management for the manifest-signing pipeline.
//!
//! ## Module layout
//!
//! - [`types`] — domain types ([`Trajectory`], [`TrajectoryCluster`],
//!   [`GoldenSet`], [`GoldenInput`], [`SignedManifest`], [`DriftReport`],
//!   [`StructuralDelta`], [`SurfaceEventRef`], [`SurfaceEventKind`],
//!   [`EvalGrade`], [`DeltaKind`], [`DriftVerdict`]).
//! - [`trajectory`] — surface-boundary capture and retrieval
//!   ([`capture_trajectory_step`], [`complete_trajectory`],
//!   [`retrieve_similar`], [`TrajectoryFilter`]).
//! - [`sona`] — k-means clustering and best-trajectory lookup
//!   ([`cluster_trajectories`], [`find_best_trajectory_for_input`]).
//! - [`replay`] — replay-driven contract tests ([`run_replay`],
//!   [`digest_output`], [`ReplayResult`], [`ReplayFailure`]).
//! - [`drift`] — byte-diff + structural-diff drift detection
//!   ([`detect_drift`], [`diff_json`], [`block_deploy_on_drift`]).
//! - [`golden_set`] — freeze + restore for golden sets on disk
//!   ([`freeze_golden_set`], [`restore_golden_set`],
//!   [`list_golden_sets`]).
//! - [`signing`] — ed25519 keypair management and manifest signing
//!   ([`generate_keypair`], [`sign_manifest`], [`verify_manifest`],
//!   [`load_signing_key`], [`save_signing_key`], [`ensure_keypair`]).
//!
//! ## Feature gating
//!
//! The whole module is gated behind the `self_learning` cargo feature at
//! the crate root (`lib.rs`). Internal files do not repeat the gate.

pub mod drift;
pub mod golden_set;
pub mod replay;
pub mod signing;
pub mod sona;
pub mod trajectory;
pub mod types;

#[cfg(test)]
mod tests;

pub use drift::{block_deploy_on_drift, detect_drift, diff_json};
pub use golden_set::{freeze_golden_set, list_golden_sets, restore_golden_set};
pub use replay::{digest_output, run_replay, ReplayFailure, ReplayResult};
pub use signing::{
    ensure_keypair, generate_keypair, load_signing_key, save_signing_key, save_verifying_key,
    sign_manifest, verify_manifest,
};
pub use sona::{cluster_trajectories, find_best_trajectory_for_input, CLUSTER_TRAINING_FLOOR};
pub use trajectory::{
    attach_tenant, capture_trajectory_step, complete_trajectory, persist_with_embedding,
    retrieve_similar, TrajectoryFilter, MAX_TRAJECTORY_STEPS,
};
pub use types::{
    DeltaKind, DriftReport, DriftVerdict, EvalGrade, GoldenInput, GoldenSet, SignedManifest,
    StructuralDelta, SurfaceEventKind, SurfaceEventRef, Trajectory, TrajectoryCluster,
};
