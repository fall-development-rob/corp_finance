//! Domain types for the Self-Learning bounded context (Phase 28).
//!
//! Mirrors `docs/ddd/domain-self-learning.md`:
//!
//! - [`Trajectory`] is the aggregate root: an ordered sequence of
//!   [`SurfaceEventRef`] plus an optional [`EvalGrade`], persisted into the
//!   Phase 26 native memory store.
//! - [`TrajectoryCluster`] is the k-means cluster aggregate over trajectory
//!   embeddings, used to bias future planning toward high-grade tool paths.
//! - [`GoldenSet`] / [`GoldenInput`] / [`SignedManifest`] capture the
//!   replay-driven contract surface (ed25519-signed; immutable
//!   post-acceptance).
//! - [`DriftReport`] / [`StructuralDelta`] / [`DriftVerdict`] hold the
//!   replay-time byte-diff + structural-diff verdict.
//!
//! All types are `serde::{Serialize, Deserialize}` so they round-trip
//! through the NAPI / MCP boundary unchanged. Per ADR-020, the public
//! surface is domain types only — no `serde_json::Value` shapes leak out
//! beyond the dispatcher closure (which is an explicit caller-supplied
//! abstraction in [`crate::self_learning::replay`]).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::surface::Surface;

// ---------------------------------------------------------------------------
// SurfaceEventKind + SurfaceEventRef
// ---------------------------------------------------------------------------

/// What kind of surface event a [`SurfaceEventRef`] represents.
///
/// One trajectory step is exactly one of these kinds. The four kinds map
/// directly onto the four CFA runtime surfaces enumerated in
/// [`crate::surface::Surface`] but at finer granularity (e.g. an MCP server
/// hosts many MCP tools).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SurfaceEventKind {
    /// `server.tool(...)` registration in `packages/*-mcp-server/src/`.
    McpTool,
    /// `cfa <subcommand>` dispatched from the CLI binary.
    CliSubcommand,
    /// Slash command (`.claude/commands/cfa/*.md`) or `.claude/skills/*`
    /// invocation, recorded via the MCP wrapper.
    SkillInvocation,
    /// Plugin hook fire from `plugins/cfa-core/hooks/hooks.json`.
    PluginHook,
}

impl SurfaceEventKind {
    /// Lower-case display token used in serialisation.
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::McpTool => "mcp_tool",
            Self::CliSubcommand => "cli_subcommand",
            Self::SkillInvocation => "skill_invocation",
            Self::PluginHook => "plugin_hook",
        }
    }
}

/// One step in a [`Trajectory`].
///
/// Each step is a single surface-event invocation: an MCP tool call, a CLI
/// subcommand, a skill invocation, or a plugin hook fire. The
/// `input_hash` and `output_hash` are content-addressed (SHA-256 hex)
/// per `RUF-LEARN-001`; `output_hash` is `None` if the event failed before
/// producing output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SurfaceEventRef {
    pub kind: SurfaceEventKind,
    /// Surface-specific identifier — MCP tool name, CLI subcommand path,
    /// slash command id, or plugin hook id.
    pub name: String,
    /// SHA-256 hex of the canonical JSON of the event input.
    pub input_hash: String,
    /// SHA-256 hex of the canonical JSON of the event output, or `None`
    /// for failed steps.
    pub output_hash: Option<String>,
    /// Wall-clock duration of the surface event in milliseconds.
    pub duration_ms: u64,
}

// ---------------------------------------------------------------------------
// EvalGrade
// ---------------------------------------------------------------------------

/// Quality grade for a completed [`Trajectory`].
///
/// Five-bucket grade per the DDD model. Used to filter clusters
/// (`RUF-LEARN-INV-003` floors training data at `Acceptable` or higher)
/// and to rank candidates returned by [`crate::self_learning::trajectory::retrieve_similar`].
///
/// The `Ord` derivation orders the variants from worst (`Failed`, lowest
/// rank) to best (`Excellent`, highest rank); the trajectory retrieval
/// path can therefore use `>=` comparisons against a floor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvalGrade {
    Failed,
    Poor,
    Acceptable,
    Good,
    Excellent,
}

impl EvalGrade {
    /// Lower-case display token.
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Failed => "failed",
            Self::Poor => "poor",
            Self::Acceptable => "acceptable",
            Self::Good => "good",
            Self::Excellent => "excellent",
        }
    }
}

// ---------------------------------------------------------------------------
// Trajectory aggregate
// ---------------------------------------------------------------------------

/// The complete record of one user-facing CFA session.
///
/// Aggregate root of the Self-Learning bounded context. Immutable once
/// captured; tenant-scoped per ADR-019. The `surface` and
/// `surface_event_id` fields identify the top-level entry point that
/// initiated the trajectory; `steps` records every downstream surface
/// event in order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Trajectory {
    /// Unique trajectory id (UUID v7 — sortable by time).
    pub trajectory_id: Uuid,
    /// Top-level surface that initiated this session.
    pub surface: Surface,
    /// Top-level event identifier on `surface` (CLI subcommand, MCP tool,
    /// slash command, or plugin hook).
    pub surface_event_id: String,
    /// Ordered surface events that comprise this trajectory.
    pub steps: Vec<SurfaceEventRef>,
    /// Quality grade assigned at session completion. `None` while the
    /// trajectory is still in flight; required for cluster training.
    pub eval_grade: Option<EvalGrade>,
    /// Federation tenant scope (per `RUF-LEARN-013`).
    pub tenant_id: Option<String>,
    /// Trajectory start timestamp.
    pub ts: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// TrajectoryCluster aggregate
// ---------------------------------------------------------------------------

/// A cluster centroid produced by k-means over trajectory embeddings.
///
/// Per `RUF-LEARN-INV-003`, only trajectories at `EvalGrade::Acceptable` or
/// higher feed the k-means worker; the `dominant_eval_grade` field is the
/// mode of the member trajectories' grades.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrajectoryCluster {
    /// Unique cluster id.
    pub cluster_id: Uuid,
    /// Centroid embedding (mean of member embeddings).
    pub centroid_embedding: Vec<f32>,
    /// Trajectory ids that belong to this cluster.
    pub member_ids: Vec<Uuid>,
    /// Mode of `eval_grade` across `member_ids`.
    pub dominant_eval_grade: EvalGrade,
    /// Member count snapshot (== `member_ids.len()`).
    pub sample_size: usize,
}

// ---------------------------------------------------------------------------
// GoldenSet aggregate
// ---------------------------------------------------------------------------

/// One frozen replay input for a [`GoldenSet`].
///
/// Each `GoldenInput` is a `(input_id, input_json, expected_digest)` tuple
/// captured at acceptance. The `expected_digest` is the SHA-256 hex of the
/// canonical JSON of the surface event's expected output for this input.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoldenInput {
    pub input_id: Uuid,
    pub input_json: serde_json::Value,
    pub expected_digest: String,
}

/// ed25519 manifest signature attached to a [`GoldenSet`].
///
/// The `content_hash` is the SHA-256 hex of the canonical manifest body;
/// `signature` is the hex-encoded ed25519 signature over those bytes;
/// `public_key` is the hex-encoded ed25519 verifying-key bytes (32 bytes
/// per the ed25519 specification).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignedManifest {
    pub content_hash: String,
    pub signature: String,
    pub public_key: String,
    pub signed_at: DateTime<Utc>,
}

/// Frozen replay-driven contract bundle for one (`surface`,
/// `surface_event_id`) pair.
///
/// Immutable post-acceptance; signature mismatch fails CI per
/// `RUF-LEARN-009`. The on-disk layout is
/// `<root>/<surface>/<surface_event_id>/manifest.json` for the manifest
/// and `<root>/<surface>/<surface_event_id>/inputs/<input_id>.json` for
/// each input fixture.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoldenSet {
    pub surface: Surface,
    pub surface_event_id: String,
    pub inputs: Vec<GoldenInput>,
    /// SHA-256 hex over the concatenated `expected_digest` fields of
    /// `inputs` in declared order.
    pub expected_output_digest: String,
    pub signed_manifest: SignedManifest,
}

// ---------------------------------------------------------------------------
// DriftReport aggregate
// ---------------------------------------------------------------------------

/// Kind of structural change detected by the recursive `serde_json::Value`
/// walker in [`crate::self_learning::drift::diff_json`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeltaKind {
    /// A field present in `current` but absent in `baseline`.
    Added,
    /// A field present in `baseline` but absent in `current`.
    Removed,
    /// Same JSON path, same scalar type, different value.
    Changed,
    /// Same JSON path, different scalar type (load-bearing).
    TypeChanged,
}

/// One structural delta in a [`DriftReport`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StructuralDelta {
    /// JSON-pointer-style path (e.g. `/foo/bar/0`).
    pub json_path: String,
    pub kind: DeltaKind,
    pub baseline: serde_json::Value,
    pub current: serde_json::Value,
}

/// Final verdict produced by [`crate::self_learning::drift::detect_drift`].
///
/// `WithinTolerance` is non-blocking; `BeyondTolerance` and `Critical`
/// block deploy in CI per `RUF-LEARN-007`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "verdict", rename_all = "snake_case")]
pub enum DriftVerdict {
    NoDrift,
    WithinTolerance { pct: f64 },
    BeyondTolerance { pct: f64 },
    Critical { reason: String },
}

/// Drift detection verdict for one (`surface`, `surface_event_id`) pair.
///
/// Combines a byte-diff percentage with a recursive structural diff.
/// Default tolerance is 5% per `RUF-LEARN-INV-008`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DriftReport {
    pub surface: Surface,
    pub surface_event_id: String,
    pub baseline_digest: String,
    pub current_digest: String,
    pub byte_diff_pct: f64,
    pub structural_diff: Vec<StructuralDelta>,
    pub verdict: DriftVerdict,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn surface_event_kind_round_trips() {
        for k in [
            SurfaceEventKind::McpTool,
            SurfaceEventKind::CliSubcommand,
            SurfaceEventKind::SkillInvocation,
            SurfaceEventKind::PluginHook,
        ] {
            let s = serde_json::to_string(&k).unwrap();
            let back: SurfaceEventKind = serde_json::from_str(&s).unwrap();
            assert_eq!(back, k);
        }
    }

    #[test]
    fn eval_grade_orders_failed_lowest_excellent_highest() {
        assert!(EvalGrade::Failed < EvalGrade::Poor);
        assert!(EvalGrade::Poor < EvalGrade::Acceptable);
        assert!(EvalGrade::Acceptable < EvalGrade::Good);
        assert!(EvalGrade::Good < EvalGrade::Excellent);
    }

    #[test]
    fn delta_kind_round_trips() {
        let d = DeltaKind::TypeChanged;
        let s = serde_json::to_string(&d).unwrap();
        assert_eq!(s, "\"type_changed\"");
    }

    #[test]
    fn drift_verdict_serializes_with_tag() {
        let v = DriftVerdict::WithinTolerance { pct: 0.03 };
        let s = serde_json::to_string(&v).unwrap();
        assert!(s.contains("\"verdict\":\"within_tolerance\""));
        assert!(s.contains("\"pct\":0.03"));
    }

    #[test]
    fn trajectory_round_trips_via_json() {
        let t = Trajectory {
            trajectory_id: Uuid::now_v7(),
            surface: Surface::Cli,
            surface_event_id: "initiate-coverage".into(),
            steps: vec![SurfaceEventRef {
                kind: SurfaceEventKind::McpTool,
                name: "dcf_model".into(),
                input_hash: "abc123".into(),
                output_hash: Some("def456".into()),
                duration_ms: 150,
            }],
            eval_grade: Some(EvalGrade::Good),
            tenant_id: Some("tenant-a".into()),
            ts: Utc::now(),
        };
        let v = serde_json::to_value(&t).unwrap();
        let back: Trajectory = serde_json::from_value(v).unwrap();
        assert_eq!(back, t);
    }

    #[test]
    fn golden_input_round_trips() {
        let g = GoldenInput {
            input_id: Uuid::now_v7(),
            input_json: json!({"ticker": "AAPL"}),
            expected_digest: "deadbeef".into(),
        };
        let s = serde_json::to_string(&g).unwrap();
        let back: GoldenInput = serde_json::from_str(&s).unwrap();
        assert_eq!(back, g);
    }
}
