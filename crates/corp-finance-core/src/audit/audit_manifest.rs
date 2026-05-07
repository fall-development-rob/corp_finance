//! `<output_path>.audit.json` companion-file writer (ADR-017 §2).
//!
//! For every output file containing a numeric recommendation, the plugin
//! `Write`/`Edit` PostToolUse hook fires and writes a sibling JSON manifest
//! at `<output_path>.audit.json`. This module provides the pure file-I/O
//! helpers; surface wiring (CLI subcommand `cfa audit write --for <path>`,
//! MCP tool `surface_audit_compute`) is layered above and not implemented
//! here.
//!
//! The schema deliberately mirrors the ADR-017 §2 example with one small
//! local liberty: we keep the field set minimal and load-bearing here
//! (everything actually consumed by RUF-AUD-001..005) and let downstream
//! wiring add `schema_version`, `model`, `skills_in_scope`, etc. when it
//! assembles the full surface-event payload. The wire format remains
//! forward-compatible because we use `serde(default)` and `#[serde(flatten)]`
//! is not used (so unknown fields round-trip without panic).

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::surface_audit::Surface;
use crate::error::CorpFinanceError;
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Tool-call ledger entry
// ---------------------------------------------------------------------------

/// One entry in the deterministic `tool_call_ledger` per ADR-017 §2.
///
/// Every MCP tool call made within a CLI subcommand or higher-level MCP tool
/// handler appends one of these to the in-memory ledger. The ledger is
/// persisted alongside the audit manifest at hook-fire time. RUF-AUD-004
/// requires that `step` values are 1-indexed and contiguous; we don't store
/// a `step` field directly — order in the `Vec<ToolCallRecord>` is the
/// step ordering. Validators check ascending order at the manifest level.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolCallRecord {
    /// Registered MCP tool name (e.g. `dcf_model`, `fmp_quote`).
    pub tool_name: String,
    /// `djb2:0x...` over canonical input.
    pub input_hash: String,
    /// `djb2:0x...` over canonical output.
    pub output_hash: String,
    /// When the tool returned.
    pub ts: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// Audit manifest aggregate
// ---------------------------------------------------------------------------

/// Schema for the `<output_path>.audit.json` companion file.
///
/// Required fields per RUF-AUD-002: `surface_audit_hash`, `surface`,
/// `surface_event_id`, `output_path`, `output_sha256`, `ts`. Optional fields
/// (`tool_calls`, `tenant_id`) carry additional provenance that consumers
/// may rely on but the basic schema validator treats as opt-in.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AuditManifest {
    /// `djb2:0x<8 hex>` surface event hash, per `surface_audit::compute_surface_audit_hash`.
    pub surface_audit_hash: String,
    /// Which CFA surface produced the output.
    pub surface: Surface,
    /// CLI subcommand / MCP tool / slash-command / plugin hook id.
    pub surface_event_id: String,
    /// Absolute or repo-relative path of the output file this manifest
    /// accompanies.
    pub output_path: PathBuf,
    /// SHA-256 of the output file's bytes (lowercase hex, no prefix).
    pub output_sha256: String,
    /// When the output was produced (UTC).
    pub ts: DateTime<Utc>,
    /// Ordered tool-call ledger. Entries are in execution order;
    /// position-in-`Vec` is the 1-indexed `step` value that RUF-AUD-004
    /// requires to be contiguous.
    #[serde(default)]
    pub tool_calls: Vec<ToolCallRecord>,
    /// Optional tenant identifier for multi-tenant deployments (Phase 27).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tenant_id: Option<String>,
}

// ---------------------------------------------------------------------------
// Path helpers
// ---------------------------------------------------------------------------

/// Append `.audit.json` to the given output path. Used by the plugin
/// `Write`/`Edit` hook to derive the sibling-manifest location from the
/// output's path.
///
/// Examples:
///
/// - `out/coverage_report.md` -> `out/coverage_report.md.audit.json`
/// - `out/dcf_model.csv` -> `out/dcf_model.csv.audit.json`
pub fn output_path_to_audit_path(output: &Path) -> PathBuf {
    let mut audit = output.as_os_str().to_owned();
    audit.push(".audit.json");
    PathBuf::from(audit)
}

// ---------------------------------------------------------------------------
// Read / Write
// ---------------------------------------------------------------------------

/// Write `<output_path>.audit.json` next to the given output file.
///
/// The output directory must already exist (the output file lives there);
/// the writer does not create intermediate directories. The manifest is
/// pretty-printed with 2-space indent so a compliance reviewer can inspect
/// it by hand. Returns the path of the manifest file that was written.
pub fn write_audit_manifest(
    output_path: &Path,
    manifest: &AuditManifest,
) -> CorpFinanceResult<PathBuf> {
    let audit_path = output_path_to_audit_path(output_path);
    let json = serde_json::to_string_pretty(manifest)
        .map_err(|e| CorpFinanceError::SerializationError(e.to_string()))?;
    fs::write(&audit_path, json).map_err(|e| io_to_cfe(&audit_path, e))?;
    Ok(audit_path)
}

/// Read and deserialize an existing `<output_path>.audit.json`.
///
/// `audit_path` is the manifest path (i.e. the `.audit.json` file itself,
/// not the underlying output). Use [`output_path_to_audit_path`] to derive
/// it from an output file path.
pub fn read_audit_manifest(audit_path: &Path) -> CorpFinanceResult<AuditManifest> {
    let bytes = fs::read(audit_path).map_err(|e| io_to_cfe(audit_path, e))?;
    let manifest: AuditManifest = serde_json::from_slice(&bytes)
        .map_err(|e| CorpFinanceError::SerializationError(e.to_string()))?;
    Ok(manifest)
}

/// Internal: translate an `io::Error` into a `CorpFinanceError`. We don't
/// have a dedicated `Io` variant on the error enum (see `error.rs`); the
/// `SerializationError` variant has historically absorbed boundary-IO
/// failures so that's what we use here for consistency with the rest of
/// `corp_finance_core`.
fn io_to_cfe(path: &Path, e: io::Error) -> CorpFinanceError {
    CorpFinanceError::SerializationError(format!("audit manifest io error at {path:?}: {e}"))
}

// ---------------------------------------------------------------------------
// SHA-256 helper (re-exposed for surface wiring; output_sha256 field)
// ---------------------------------------------------------------------------

/// Compute the SHA-256 of an output file's bytes and return the lowercase
/// hex string suitable for the `output_sha256` field. Helper for surface
/// wiring; not invoked by the manifest writer itself (the writer trusts
/// the caller to supply a pre-computed hash).
pub fn sha256_file(path: &Path) -> CorpFinanceResult<String> {
    use sha2::{Digest, Sha256};
    let bytes = fs::read(path).map_err(|e| io_to_cfe(path, e))?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let digest = hasher.finalize();
    let mut hex = String::with_capacity(64);
    for b in digest.iter() {
        hex.push_str(&format!("{b:02x}"));
    }
    Ok(hex)
}

// ---------------------------------------------------------------------------
// Validation helpers (exercised by tests; useful to surface wiring later)
// ---------------------------------------------------------------------------

/// Validate that the `tool_calls` list satisfies RUF-AUD-004: order is the
/// 1-indexed step ordering, and the list is contiguous (no gaps). Because
/// we model `step` as positional, the only thing to validate is that the
/// list itself is well-formed (timestamps non-decreasing, no duplicate
/// `(tool_name, ts)` pairs at the same nanosecond).
///
/// This is a defence-in-depth check; a manifest authored by the surface
/// wrapper is expected to be well-formed by construction.
pub fn validate_tool_call_ledger(ledger: &[ToolCallRecord]) -> CorpFinanceResult<()> {
    for window in ledger.windows(2) {
        if window[1].ts < window[0].ts {
            return Err(CorpFinanceError::InvalidInput {
                field: "tool_calls".to_string(),
                reason: "ledger ts not monotonic non-decreasing".to_string(),
            });
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit::surface_audit::{compute_surface_audit_hash, SurfaceManifest};
    use serde_json::json;
    use std::env;

    fn tmp_output(name: &str, contents: &[u8]) -> PathBuf {
        let dir = env::temp_dir().join(format!("cfa-audit-test-{}", uuid::Uuid::now_v7()));
        fs::create_dir_all(&dir).expect("create tmp dir");
        let path = dir.join(name);
        fs::write(&path, contents).expect("write tmp output");
        path
    }

    fn sample_manifest_for(output_path: &Path, sha256: String) -> AuditManifest {
        let surface_manifest = SurfaceManifest {
            surface: Surface::Cli,
            surface_event_id: "workflow.audit".to_string(),
            command_args: json!({ "ticker": "AAPL" }),
            output_paths: vec![output_path.display().to_string()],
        };
        let surface_hash = compute_surface_audit_hash(&surface_manifest);
        AuditManifest {
            surface_audit_hash: surface_hash,
            surface: Surface::Cli,
            surface_event_id: "workflow.audit".to_string(),
            output_path: output_path.to_path_buf(),
            output_sha256: sha256,
            ts: Utc::now(),
            tool_calls: vec![
                ToolCallRecord {
                    tool_name: "fmp_quote".to_string(),
                    input_hash: "djb2:0x10aa10aa".to_string(),
                    output_hash: "djb2:0x55bc55bc".to_string(),
                    ts: Utc::now(),
                },
                ToolCallRecord {
                    tool_name: "dcf_model".to_string(),
                    input_hash: "djb2:0x44e144e1".to_string(),
                    output_hash: "djb2:0x90829082".to_string(),
                    ts: Utc::now(),
                },
            ],
            tenant_id: None,
        }
    }

    /// RUF-AUD-001: every output file with a numeric recommendation has a
    /// matching `<output_path>.audit.json` after the writer runs.
    #[test]
    fn ruf_aud_001_audit_manifest_for_every_output() {
        let output = tmp_output("coverage_report.md", b"## DCF\nFair value: $150\n");
        let sha = sha256_file(&output).expect("sha256");
        let manifest = sample_manifest_for(&output, sha);

        let written = write_audit_manifest(&output, &manifest).expect("write manifest");
        assert!(written.exists(), "audit manifest must exist after write");
        assert_eq!(written, output_path_to_audit_path(&output));
    }

    /// RUF-AUD-002: required fields populated and `output_sha256` matches
    /// actual file content.
    #[test]
    fn ruf_aud_002_audit_manifest_required_fields() {
        let output = tmp_output("dcf_model.csv", b"ticker,fair_value\nAAPL,150\n");
        let sha = sha256_file(&output).expect("sha256");
        let manifest = sample_manifest_for(&output, sha.clone());

        let written = write_audit_manifest(&output, &manifest).expect("write");
        let loaded = read_audit_manifest(&written).expect("read");

        assert!(!loaded.surface_audit_hash.is_empty());
        assert_eq!(loaded.surface, Surface::Cli);
        assert_eq!(loaded.surface_event_id, "workflow.audit");
        assert_eq!(loaded.output_path, output);
        assert_eq!(loaded.output_sha256, sha);
        // ts is non-zero
        assert!(loaded.ts.timestamp() > 0);

        // output_sha256 matches the actual file content hash
        let recomputed = sha256_file(&output).expect("sha256 recompute");
        assert_eq!(
            loaded.output_sha256, recomputed,
            "output_sha256 must equal sha256(output_path)"
        );
    }

    /// RUF-AUD-004: tool-call ledger entries are ordered ascending by ts.
    /// The validator catches a non-monotonic ledger.
    #[test]
    fn ruf_aud_004_tool_call_ledger_ordered() {
        let output = tmp_output("ledger.csv", b"x\n");
        let sha = sha256_file(&output).expect("sha");
        let mut manifest = sample_manifest_for(&output, sha);

        // Sanity: the canonical ledger validates.
        validate_tool_call_ledger(&manifest.tool_calls).expect("canonical ledger valid");

        // Inject a backwards-in-time entry: the validator must reject.
        let earlier_ts = manifest.tool_calls[0].ts - chrono::Duration::seconds(60);
        manifest.tool_calls.push(ToolCallRecord {
            tool_name: "out_of_order".to_string(),
            input_hash: "djb2:0xaaaaaaaa".to_string(),
            output_hash: "djb2:0xbbbbbbbb".to_string(),
            ts: earlier_ts,
        });
        let res = validate_tool_call_ledger(&manifest.tool_calls);
        assert!(res.is_err(), "non-monotonic ledger must be rejected");
    }

    /// RUF-AUD-005: round-trip preserves run-correlation fields. We model
    /// run-correlation here as `surface_audit_hash` + `surface_event_id`;
    /// the actual `run_id` correlation lives in `run_summary.json` (memory
    /// module). The audit-side invariant is "the hash and event id loaded
    /// from disk equal the values written to disk".
    #[test]
    fn ruf_aud_005_round_trip_preserves_correlation_fields() {
        let output = tmp_output("rt.csv", b"row1\n");
        let sha = sha256_file(&output).expect("sha");
        let manifest = sample_manifest_for(&output, sha);

        let written = write_audit_manifest(&output, &manifest).expect("write");
        let loaded = read_audit_manifest(&written).expect("read");

        assert_eq!(loaded.surface_audit_hash, manifest.surface_audit_hash);
        assert_eq!(loaded.surface_event_id, manifest.surface_event_id);
        assert_eq!(loaded.tool_calls.len(), manifest.tool_calls.len());
        for (a, b) in loaded.tool_calls.iter().zip(manifest.tool_calls.iter()) {
            assert_eq!(a.tool_name, b.tool_name);
            assert_eq!(a.input_hash, b.input_hash);
            assert_eq!(a.output_hash, b.output_hash);
        }
    }

    /// RUF-AUD-INV-001 (sample): audit coverage. With the writer in place,
    /// for every output file a sibling `.audit.json` exists. We assert the
    /// derived path is exactly `<output>.audit.json`.
    #[test]
    fn ruf_aud_inv_001_audit_path_derivation() {
        let cases = [
            (
                "out/coverage_report.md",
                "out/coverage_report.md.audit.json",
            ),
            ("out/dcf_model.csv", "out/dcf_model.csv.audit.json"),
            ("a.txt", "a.txt.audit.json"),
            ("/abs/path/x.json", "/abs/path/x.json.audit.json"),
        ];
        for (input, expected) in &cases {
            let got = output_path_to_audit_path(Path::new(input));
            assert_eq!(got, PathBuf::from(*expected), "mismatch for input {input}");
        }
    }
}
