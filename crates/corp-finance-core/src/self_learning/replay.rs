//! Replay-driven contract tests over [`crate::self_learning::types::GoldenSet`]s.
//!
//! Per `RUF-LEARN-006` / `RUF-LEARN-007`, CI replays each frozen golden
//! input against the current surface event handler, hashes the output,
//! and compares against the manifest's `expected_digest`. Mismatches
//! produce [`ReplayFailure`] entries with a structural-diff report; the
//! deploy gate inspects the resulting [`ReplayResult`].
//!
//! The dispatcher abstraction (`impl Fn(&serde_json::Value) ->
//! Result<serde_json::Value>`) is the integration seam. Production
//! callers supply a closure that invokes the surface (CLI subcommand,
//! MCP tool, slash command, plugin hook) on the recorded input. This
//! crate does not dispatch surface events itself — that work happens in
//! the corp-finance-cli, packages/*-mcp-server, and plugins/cfa-core
//! integration code.

use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::self_learning::drift::detect_drift;
use crate::self_learning::types::{DriftReport, GoldenSet};
use crate::surface::Surface;
use crate::CorpFinanceResult;

/// One input that failed the replay byte-comparison.
///
/// `structural_delta` is populated when the dispatcher succeeded but the
/// output digest differs; the field is `None` when the dispatcher itself
/// returned an error (in which case `actual_digest` is the empty string).
#[derive(Debug, Clone)]
pub struct ReplayFailure {
    pub input_id: Uuid,
    pub expected_digest: String,
    pub actual_digest: String,
    pub structural_delta: Option<DriftReport>,
}

/// Aggregate result of running [`run_replay`] over one [`GoldenSet`].
#[derive(Debug, Clone)]
pub struct ReplayResult {
    pub passed: usize,
    pub failed: usize,
    pub failures: Vec<ReplayFailure>,
}

impl ReplayResult {
    /// Returns true when no inputs failed the replay.
    pub fn is_clean(&self) -> bool {
        self.failed == 0
    }
}

/// Canonicalise a JSON value: sort object keys recursively. Arrays
/// preserve order. Mirrors the canonicalisation used by
/// [`crate::audit::surface_audit::compute_surface_audit_hash`] so digests
/// are stable across object-key reorderings.
fn canonicalize(v: &serde_json::Value) -> serde_json::Value {
    match v {
        serde_json::Value::Object(map) => {
            let mut sorted: std::collections::BTreeMap<String, serde_json::Value> =
                std::collections::BTreeMap::new();
            for (k, val) in map {
                sorted.insert(k.clone(), canonicalize(val));
            }
            let mut out = serde_json::Map::new();
            for (k, val) in sorted {
                out.insert(k, val);
            }
            serde_json::Value::Object(out)
        }
        serde_json::Value::Array(arr) => {
            serde_json::Value::Array(arr.iter().map(canonicalize).collect())
        }
        other => other.clone(),
    }
}

/// SHA-256 hex digest of the canonicalised JSON form of `value`.
///
/// Used as the content-addressable identifier of a surface event's
/// output. Identical content produces an identical digest regardless of
/// object-key declaration order.
pub fn digest_output(value: &serde_json::Value) -> String {
    let canon = canonicalize(value);
    let bytes = serde_json::to_vec(&canon).unwrap_or_default();
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let h = hasher.finalize();
    let mut s = String::with_capacity(64);
    for b in h {
        use std::fmt::Write as _;
        let _ = write!(s, "{b:02x}");
    }
    s
}

/// Replay every input in `golden_set` against the supplied dispatcher.
///
/// The dispatcher is invoked once per input and is expected to:
///
/// 1. Decode the recorded `input_json`.
/// 2. Invoke the surface event under test.
/// 3. Return the surface event's output as `serde_json::Value`.
///
/// Dispatcher errors are recorded as [`ReplayFailure`] entries with an
/// empty `actual_digest`. Successful dispatcher returns whose digest
/// matches the golden `expected_digest` increment `passed`; mismatches
/// increment `failed` with a [`DriftReport`] from
/// [`crate::self_learning::drift::detect_drift`].
pub fn run_replay<F>(golden_set: &GoldenSet, mut dispatcher: F) -> CorpFinanceResult<ReplayResult>
where
    F: FnMut(&serde_json::Value) -> CorpFinanceResult<serde_json::Value>,
{
    let mut passed = 0usize;
    let mut failed = 0usize;
    let mut failures: Vec<ReplayFailure> = Vec::new();

    for input in &golden_set.inputs {
        match dispatcher(&input.input_json) {
            Ok(output) => {
                let actual = digest_output(&output);
                if actual == input.expected_digest {
                    passed += 1;
                } else {
                    let report = build_drift_report(
                        golden_set.surface,
                        &golden_set.surface_event_id,
                        &input.input_json,
                        &output,
                        &input.expected_digest,
                        &actual,
                    );
                    failures.push(ReplayFailure {
                        input_id: input.input_id,
                        expected_digest: input.expected_digest.clone(),
                        actual_digest: actual,
                        structural_delta: Some(report),
                    });
                    failed += 1;
                }
            }
            Err(_) => {
                failures.push(ReplayFailure {
                    input_id: input.input_id,
                    expected_digest: input.expected_digest.clone(),
                    actual_digest: String::new(),
                    structural_delta: None,
                });
                failed += 1;
            }
        }
    }

    Ok(ReplayResult {
        passed,
        failed,
        failures,
    })
}

/// Build a [`DriftReport`] for one replay mismatch.
///
/// Compares `baseline_output` (decoded from the golden set's input fixture
/// when available) against `current_output`. v1 uses the input as the
/// baseline canonical form when no separate baseline is supplied; this
/// keeps the structural diff focused on output drift rather than input
/// reconstruction.
fn build_drift_report(
    surface: Surface,
    surface_event_id: &str,
    _input: &serde_json::Value,
    current_output: &serde_json::Value,
    baseline_digest: &str,
    current_digest: &str,
) -> DriftReport {
    // We don't have the original output payload (only the digest) on the
    // replay path; the structural diff is a no-op until the manifest
    // carries the full output for narrative cases. v1 emits an empty
    // structural-diff with a non-zero byte_diff_pct when digests differ.
    let baseline_value = serde_json::Value::Null;
    detect_drift_with_id(
        surface,
        surface_event_id,
        &baseline_value,
        current_output,
        baseline_digest,
        current_digest,
        0.05,
    )
}

/// Variant of [`crate::self_learning::drift::detect_drift`] that takes the
/// surface metadata directly (used internally so callers don't reconstruct
/// digest fields).
fn detect_drift_with_id(
    surface: Surface,
    surface_event_id: &str,
    baseline: &serde_json::Value,
    current: &serde_json::Value,
    baseline_digest: &str,
    current_digest: &str,
    tolerance_pct: f64,
) -> DriftReport {
    let mut report = detect_drift(baseline, current, tolerance_pct);
    report.surface = surface;
    report.surface_event_id = surface_event_id.to_string();
    report.baseline_digest = baseline_digest.to_string();
    report.current_digest = current_digest.to_string();
    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::self_learning::types::{GoldenInput, SignedManifest};
    use chrono::Utc;
    use serde_json::json;

    fn make_golden_set(inputs: Vec<GoldenInput>) -> GoldenSet {
        GoldenSet {
            surface: Surface::Mcp,
            surface_event_id: "test_tool".into(),
            inputs,
            expected_output_digest: "stub".into(),
            signed_manifest: SignedManifest {
                content_hash: "h".into(),
                signature: "s".into(),
                public_key: "k".into(),
                signed_at: Utc::now(),
            },
        }
    }

    #[test]
    fn digest_output_is_stable_across_key_order() {
        let a = json!({"x": 1, "y": 2});
        let b = json!({"y": 2, "x": 1});
        assert_eq!(digest_output(&a), digest_output(&b));
    }

    #[test]
    fn replay_passes_when_dispatcher_matches_expected() {
        let out = json!({"result": 42});
        let digest = digest_output(&out);
        let gs = make_golden_set(vec![GoldenInput {
            input_id: Uuid::now_v7(),
            input_json: json!({"in": 1}),
            expected_digest: digest,
        }]);
        let result = run_replay(&gs, |_| Ok(out.clone())).unwrap();
        assert_eq!(result.passed, 1);
        assert_eq!(result.failed, 0);
    }

    #[test]
    fn replay_records_failure_on_dispatcher_error() {
        let gs = make_golden_set(vec![GoldenInput {
            input_id: Uuid::now_v7(),
            input_json: json!({"in": 1}),
            expected_digest: "x".into(),
        }]);
        let result = run_replay(&gs, |_| {
            Err(crate::error::CorpFinanceError::InsufficientData(
                "boom".into(),
            ))
        })
        .unwrap();
        assert_eq!(result.failed, 1);
        assert_eq!(result.failures.len(), 1);
        assert!(result.failures[0].structural_delta.is_none());
    }
}
