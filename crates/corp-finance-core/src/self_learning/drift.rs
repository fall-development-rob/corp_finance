//! Byte-diff + structural-diff drift detection.
//!
//! Implements [`detect_drift`] and [`block_deploy_on_drift`] per
//! `RUF-LEARN-005` / `RUF-LEARN-006` / `RUF-LEARN-007`. The byte-diff
//! component compares serialised JSON sizes; the structural-diff
//! component walks the two `serde_json::Value` trees recursively and
//! emits one [`StructuralDelta`] per differing path (JSON-pointer
//! syntax).
//!
//! ## Verdict thresholds
//!
//! - `NoDrift` when both byte-diff and structural-diff are empty.
//! - `WithinTolerance { pct }` when the byte-diff is below the supplied
//!   tolerance and no `TypeChanged` deltas exist.
//! - `BeyondTolerance { pct }` when the byte-diff is at or above the
//!   tolerance.
//! - `Critical { reason }` when any structural delta is `TypeChanged`
//!   (the load-bearing case — a numeric field becoming a string, etc.).

use crate::self_learning::types::{DeltaKind, DriftReport, DriftVerdict, StructuralDelta};
use crate::surface::Surface;

/// Drift report verdict for a (`baseline`, `current`) pair.
///
/// The `surface` and `surface_event_id` fields on the returned report are
/// stub values (`Surface::Cli`, empty string); callers in
/// [`crate::self_learning::replay`] populate them after the fact. This
/// keeps the function pure for unit testing.
pub fn detect_drift(
    baseline: &serde_json::Value,
    current: &serde_json::Value,
    tolerance_pct: f64,
) -> DriftReport {
    let baseline_bytes = serde_json::to_string(baseline)
        .unwrap_or_default()
        .into_bytes();
    let current_bytes = serde_json::to_string(current)
        .unwrap_or_default()
        .into_bytes();
    let byte_diff_pct = if baseline_bytes.is_empty() {
        if current_bytes.is_empty() {
            0.0
        } else {
            1.0
        }
    } else {
        let diff = (current_bytes.len() as i64 - baseline_bytes.len() as i64).abs() as f64;
        diff / (baseline_bytes.len() as f64)
    };

    let mut deltas: Vec<StructuralDelta> = Vec::new();
    diff_json(baseline, current, "", &mut deltas);

    let has_type_change = deltas.iter().any(|d| d.kind == DeltaKind::TypeChanged);
    let verdict = if byte_diff_pct == 0.0 && deltas.is_empty() {
        DriftVerdict::NoDrift
    } else if has_type_change {
        DriftVerdict::Critical {
            reason: "structural type change in load-bearing field".into(),
        }
    } else if byte_diff_pct < tolerance_pct {
        DriftVerdict::WithinTolerance { pct: byte_diff_pct }
    } else {
        DriftVerdict::BeyondTolerance { pct: byte_diff_pct }
    };

    DriftReport {
        surface: Surface::Cli,
        surface_event_id: String::new(),
        baseline_digest: String::new(),
        current_digest: String::new(),
        byte_diff_pct,
        structural_diff: deltas,
        verdict,
    }
}

/// Recursively walk `baseline` and `current`, appending a
/// [`StructuralDelta`] to `deltas` for every differing JSON path.
///
/// `path` is JSON-pointer syntax: object members are appended as
/// `/<key>`; array elements as `/<index>`. The root path is the empty
/// string.
pub fn diff_json(
    baseline: &serde_json::Value,
    current: &serde_json::Value,
    path: &str,
    deltas: &mut Vec<StructuralDelta>,
) {
    use serde_json::Value;
    match (baseline, current) {
        (Value::Null, Value::Null) => {}
        (Value::Bool(a), Value::Bool(b)) => {
            if a != b {
                deltas.push(StructuralDelta {
                    json_path: path.to_string(),
                    kind: DeltaKind::Changed,
                    baseline: baseline.clone(),
                    current: current.clone(),
                });
            }
        }
        (Value::Number(a), Value::Number(b)) => {
            if a != b {
                deltas.push(StructuralDelta {
                    json_path: path.to_string(),
                    kind: DeltaKind::Changed,
                    baseline: baseline.clone(),
                    current: current.clone(),
                });
            }
        }
        (Value::String(a), Value::String(b)) => {
            if a != b {
                deltas.push(StructuralDelta {
                    json_path: path.to_string(),
                    kind: DeltaKind::Changed,
                    baseline: baseline.clone(),
                    current: current.clone(),
                });
            }
        }
        (Value::Array(a), Value::Array(b)) => {
            let max_len = a.len().max(b.len());
            for i in 0..max_len {
                let child_path = format!("{path}/{i}");
                match (a.get(i), b.get(i)) {
                    (Some(av), Some(bv)) => diff_json(av, bv, &child_path, deltas),
                    (Some(av), None) => deltas.push(StructuralDelta {
                        json_path: child_path,
                        kind: DeltaKind::Removed,
                        baseline: av.clone(),
                        current: Value::Null,
                    }),
                    (None, Some(bv)) => deltas.push(StructuralDelta {
                        json_path: child_path,
                        kind: DeltaKind::Added,
                        baseline: Value::Null,
                        current: bv.clone(),
                    }),
                    (None, None) => {}
                }
            }
        }
        (Value::Object(a), Value::Object(b)) => {
            let mut keys: std::collections::BTreeSet<&String> = a.keys().collect();
            keys.extend(b.keys());
            for k in keys {
                let child_path = format!("{path}/{k}");
                match (a.get(k), b.get(k)) {
                    (Some(av), Some(bv)) => diff_json(av, bv, &child_path, deltas),
                    (Some(av), None) => deltas.push(StructuralDelta {
                        json_path: child_path,
                        kind: DeltaKind::Removed,
                        baseline: av.clone(),
                        current: Value::Null,
                    }),
                    (None, Some(bv)) => deltas.push(StructuralDelta {
                        json_path: child_path,
                        kind: DeltaKind::Added,
                        baseline: Value::Null,
                        current: bv.clone(),
                    }),
                    (None, None) => {}
                }
            }
        }
        // Type changes — the load-bearing case for the Critical verdict.
        (a, b) => {
            // Same path exists on both sides but with different scalar
            // types (e.g. number vs string, scalar vs container).
            deltas.push(StructuralDelta {
                json_path: path.to_string(),
                kind: DeltaKind::TypeChanged,
                baseline: a.clone(),
                current: b.clone(),
            });
        }
    }
}

/// Decide whether a [`DriftReport`] should block deploy.
///
/// Returns `true` (block) when the verdict is `BeyondTolerance` or
/// `Critical`. `WithinTolerance` and `NoDrift` are non-blocking. The
/// `max_pct` argument is the configurable per-surface override; when
/// supplied below the default 5%, it tightens the gate.
pub fn block_deploy_on_drift(report: &DriftReport, max_pct: f64) -> bool {
    match &report.verdict {
        DriftVerdict::NoDrift => false,
        DriftVerdict::WithinTolerance { pct } => *pct >= max_pct,
        DriftVerdict::BeyondTolerance { .. } | DriftVerdict::Critical { .. } => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn identical_values_produce_no_drift() {
        let v = json!({"a": 1, "b": [1, 2, 3]});
        let report = detect_drift(&v, &v, 0.05);
        assert!(matches!(report.verdict, DriftVerdict::NoDrift));
        assert!(report.structural_diff.is_empty());
    }

    #[test]
    fn small_change_is_within_tolerance() {
        let a = json!({"x": "hello world"});
        let b = json!({"x": "hello world!"});
        let report = detect_drift(&a, &b, 0.50);
        match report.verdict {
            DriftVerdict::WithinTolerance { .. } => {}
            other => panic!("expected within_tolerance, got {other:?}"),
        }
    }

    #[test]
    fn type_change_is_critical() {
        let a = json!({"x": 1});
        let b = json!({"x": "1"});
        let report = detect_drift(&a, &b, 0.05);
        match report.verdict {
            DriftVerdict::Critical { .. } => {}
            other => panic!("expected critical, got {other:?}"),
        }
    }

    #[test]
    fn block_deploy_on_critical_returns_true() {
        let report = DriftReport {
            surface: Surface::Cli,
            surface_event_id: "x".into(),
            baseline_digest: "a".into(),
            current_digest: "b".into(),
            byte_diff_pct: 0.0,
            structural_diff: vec![],
            verdict: DriftVerdict::Critical {
                reason: "type changed".into(),
            },
        };
        assert!(block_deploy_on_drift(&report, 0.05));
    }

    #[test]
    fn diff_json_records_added_and_removed() {
        let a = json!({"a": 1, "b": 2});
        let b = json!({"a": 1, "c": 3});
        let mut deltas = Vec::new();
        diff_json(&a, &b, "", &mut deltas);
        let kinds: Vec<DeltaKind> = deltas.iter().map(|d| d.kind).collect();
        assert!(kinds.contains(&DeltaKind::Added));
        assert!(kinds.contains(&DeltaKind::Removed));
    }
}
