//! Surface-event audit hashing (djb2) per ADR-017 §1.
//!
//! Computes a deterministic djb2 fingerprint over the canonical JSON
//! representation of a surface-event manifest. The algorithm is byte-identical
//! to [`crate::workflows::audit::djb2_hash`] (ADR-009) so workflow-level audit
//! hashes and surface-level audit hashes share one algorithm.
//!
//! ## `Surface` enum
//!
//! Re-exported from the shared kernel `corp_finance_core::surface::Surface`
//! (Phase 27 cleanup). The previous local duplicate was collapsed into the
//! shared kernel; the wire form is byte-identical
//! (`serde(rename_all = "snake_case")` over variants `Cli`, `Mcp`, `Skill`,
//! `Plugin`) so existing audit hashes remain stable.
//!
//! ## Hash format
//!
//! Output of [`compute_surface_audit_hash`] is `djb2:0x<8-hex-digits>`,
//! which matches `RUF-AUD-INV-002` (regex `^djb2:0x[0-9a-f]{8}$`).

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Surface — re-exported from the shared kernel (Phase 27 cleanup).
// ---------------------------------------------------------------------------

pub use crate::surface::Surface;

// ---------------------------------------------------------------------------
// Surface manifest (input to the hash)
// ---------------------------------------------------------------------------

/// Canonical manifest of a single surface event, hashed to produce
/// `surface_audit_hash`.
///
/// Per ADR-017 §1, the hash domain is the surface event's static manifest:
/// the `surface` enum value, an opaque identifier for the event, the command
/// arguments / tool input value, and the set of output paths the event
/// produced. The `serde_json::Value` for `command_args` is canonicalised
/// (keys sorted, no insignificant whitespace) before hashing — see
/// [`compute_surface_audit_hash`] for the canonicalisation step.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SurfaceManifest {
    /// Which of the four CFA surfaces produced this event.
    pub surface: Surface,
    /// Surface-specific event identifier — CLI subcommand name (e.g.
    /// `workflow.audit`), MCP tool name (e.g. `dcf_model`), slash-command id,
    /// or plugin hook id (e.g. `pre_tool_use`).
    pub surface_event_id: String,
    /// Canonical JSON of the command arguments / tool input. The hasher
    /// canonicalises this further (sorts keys, strips whitespace) before
    /// folding into the hash.
    pub command_args: serde_json::Value,
    /// Output file paths produced by the event. Sorted lexicographically
    /// inside the hasher so insertion order does not affect the hash.
    pub output_paths: Vec<String>,
}

// ---------------------------------------------------------------------------
// djb2 hash — byte-identical to workflows::audit::djb2_hash
// ---------------------------------------------------------------------------

/// djb2 string hash (Bernstein). Byte-identical to the workflow audit djb2
/// in `crate::workflows::audit::djb2_hash`. Returns the raw u64 so callers
/// can format it however they wish; [`compute_surface_audit_hash`] formats
/// to `djb2:0x<8-hex-digits>` per RUF-AUD-INV-002.
fn djb2_u64(data: &str) -> u64 {
    let mut hash: u64 = 5381;
    for byte in data.bytes() {
        hash = hash.wrapping_mul(33).wrapping_add(byte as u64);
    }
    hash
}

// ---------------------------------------------------------------------------
// Canonicalisation
// ---------------------------------------------------------------------------

/// Recursively canonicalise a `serde_json::Value`: sort object keys, leave
/// arrays in declared order, leave scalars as-is. The result serialises to a
/// single deterministic string via `serde_json::to_string`.
fn canonicalize_value(v: &serde_json::Value) -> serde_json::Value {
    match v {
        serde_json::Value::Object(map) => {
            let mut sorted: std::collections::BTreeMap<String, serde_json::Value> =
                std::collections::BTreeMap::new();
            for (k, val) in map {
                sorted.insert(k.clone(), canonicalize_value(val));
            }
            // Convert BTreeMap back to a serde_json::Map preserving sorted order.
            let mut out = serde_json::Map::new();
            for (k, val) in sorted {
                out.insert(k, val);
            }
            serde_json::Value::Object(out)
        }
        serde_json::Value::Array(arr) => {
            serde_json::Value::Array(arr.iter().map(canonicalize_value).collect())
        }
        other => other.clone(),
    }
}

/// Build the canonical input string for hashing a [`SurfaceManifest`]. Format:
///
/// ```text
/// surface=<snake_case>|event=<id>|args=<canonical_json>|outputs=<sorted_csv>
/// ```
fn canonical_manifest_string(manifest: &SurfaceManifest) -> String {
    let surface_token = match manifest.surface {
        Surface::Cli => "cli",
        Surface::Mcp => "mcp",
        Surface::Skill => "skill",
        Surface::Plugin => "plugin",
    };
    let canonical_args = canonicalize_value(&manifest.command_args);
    let args_json = serde_json::to_string(&canonical_args).unwrap_or_else(|_| "null".to_string());
    let mut sorted_outputs = manifest.output_paths.clone();
    sorted_outputs.sort();
    let outputs_csv = sorted_outputs.join(",");
    format!(
        "surface={}|event={}|args={}|outputs={}",
        surface_token, manifest.surface_event_id, args_json, outputs_csv
    )
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Compute the surface audit hash for the given manifest.
///
/// Returns a string of the form `djb2:0x<8-hex-digits>` (lowercase). Per
/// ADR-017 §1, identical manifest content always produces the identical hash;
/// any change to the manifest content (surface, event id, args, or outputs)
/// changes the hash. Insertion order of object keys in `command_args` and
/// of paths in `output_paths` does not affect the hash.
pub fn compute_surface_audit_hash(manifest: &SurfaceManifest) -> String {
    let canonical = canonical_manifest_string(manifest);
    let h = djb2_u64(&canonical);
    // Truncate to the low 32 bits for an 8-hex-digit display, matching
    // RUF-AUD-INV-002 regex `^djb2:0x[0-9a-f]{8}$`.
    let truncated = (h & 0xFFFF_FFFF) as u32;
    format!("djb2:0x{truncated:08x}")
}

/// Verify a previously computed surface audit hash against a manifest.
///
/// Returns `true` iff `compute_surface_audit_hash(manifest) == expected_hash`
/// (byte-equal comparison). Useful at audit-replay time when reconstructing
/// `.audit.json` provenance.
pub fn verify_audit_hash(manifest: &SurfaceManifest, expected_hash: &str) -> bool {
    compute_surface_audit_hash(manifest) == expected_hash
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn sample_manifest() -> SurfaceManifest {
        SurfaceManifest {
            surface: Surface::Cli,
            surface_event_id: "workflow.audit".to_string(),
            command_args: json!({ "ticker": "AAPL", "horizon_years": 5 }),
            output_paths: vec!["out/dcf.csv".to_string(), "out/dcf.md".to_string()],
        }
    }

    /// RUF-AUD-002: djb2 hash deterministic across calls.
    #[test]
    fn ruf_aud_002_djb2_hash_deterministic() {
        let m = sample_manifest();
        let h1 = compute_surface_audit_hash(&m);
        let h2 = compute_surface_audit_hash(&m);
        assert_eq!(h1, h2, "djb2 hash must be deterministic");
    }

    /// RUF-AUD-INV-002: hash format matches `^djb2:0x[0-9a-f]{8}$`.
    #[test]
    fn ruf_aud_inv_002_hash_format() {
        let m = sample_manifest();
        let h = compute_surface_audit_hash(&m);
        let re = regex_lite_format_check(&h);
        assert!(re, "expected djb2:0x[0-9a-f]{{8}}, got {h}");
    }

    /// Tiny inline format check (no regex crate dep): exactly
    /// `djb2:0x` (7 chars) + 8 lowercase hex digits, total length 15.
    fn regex_lite_format_check(s: &str) -> bool {
        if s.len() != 15 {
            return false;
        }
        if !s.starts_with("djb2:0x") {
            return false;
        }
        s[7..].chars().all(|c| matches!(c, '0'..='9' | 'a'..='f'))
    }

    /// RUF-AUD-003: hash is stable under cosmetic reordering (object keys,
    /// output_paths declared order). Per the contract, two constructions of
    /// the same surface entry-point spec with different field ordering must
    /// produce the same djb2 string.
    #[test]
    fn ruf_aud_003_surface_hash_content_stable() {
        let m1 = SurfaceManifest {
            surface: Surface::Mcp,
            surface_event_id: "dcf_model".to_string(),
            command_args: json!({ "alpha": 1, "beta": 2, "gamma": 3 }),
            output_paths: vec!["out/a.csv".to_string(), "out/b.csv".to_string()],
        };
        let m2 = SurfaceManifest {
            surface: Surface::Mcp,
            surface_event_id: "dcf_model".to_string(),
            // Same content, different declared order.
            command_args: json!({ "gamma": 3, "alpha": 1, "beta": 2 }),
            output_paths: vec!["out/b.csv".to_string(), "out/a.csv".to_string()],
        };
        assert_eq!(
            compute_surface_audit_hash(&m1),
            compute_surface_audit_hash(&m2),
            "hash must be invariant under cosmetic reordering"
        );
    }

    /// RUF-AUD-003 (negative direction): any change to manifest content
    /// changes the hash.
    #[test]
    fn ruf_aud_003_surface_hash_changes_on_content_change() {
        let m1 = sample_manifest();
        let mut m2 = sample_manifest();
        m2.surface_event_id = "workflow.audit.v2".to_string();
        assert_ne!(
            compute_surface_audit_hash(&m1),
            compute_surface_audit_hash(&m2),
            "hash must change when surface_event_id changes"
        );

        let mut m3 = sample_manifest();
        m3.command_args = json!({ "ticker": "MSFT", "horizon_years": 5 });
        assert_ne!(
            compute_surface_audit_hash(&m1),
            compute_surface_audit_hash(&m3),
            "hash must change when args change"
        );

        let mut m4 = sample_manifest();
        m4.surface = Surface::Mcp;
        assert_ne!(
            compute_surface_audit_hash(&m1),
            compute_surface_audit_hash(&m4),
            "hash must change when surface changes"
        );
    }

    /// `verify_audit_hash` returns true for the canonical hash and false for
    /// any tampered hash.
    #[test]
    fn verify_audit_hash_round_trip() {
        let m = sample_manifest();
        let h = compute_surface_audit_hash(&m);
        assert!(verify_audit_hash(&m, &h));
        assert!(!verify_audit_hash(&m, "djb2:0xdeadbeef"));
    }

    /// djb2_u64 is byte-identical to workflows::audit::djb2_hash. We exercise
    /// it on a small fixed input and assert the same arithmetic that the
    /// workflows hasher uses (same seed 5381, same multiplier 33).
    #[test]
    fn djb2_byte_identical_to_workflow_djb2() {
        // Reference: implement the exact algorithm inline and compare.
        fn reference(data: &str) -> u64 {
            let mut hash: u64 = 5381;
            for byte in data.bytes() {
                hash = hash.wrapping_mul(33).wrapping_add(byte as u64);
            }
            hash
        }
        let inputs = ["", "a", "abc", "the quick brown fox", "djb2 test 123"];
        for s in &inputs {
            assert_eq!(
                djb2_u64(s),
                reference(s),
                "djb2 implementation drift on input {s:?}"
            );
        }
    }
}
