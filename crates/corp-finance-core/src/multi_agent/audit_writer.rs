//! Per-invocation `.audit.json` companion-file writer for the multi-agent
//! coordination surface (Phase 29 Wave 2).
//!
//! Persists `<invocation_id>.<phase>.json` event files and their sibling
//! `<...>.json.audit.json` companions under a caller-supplied manifest root
//! directory. Phase is either `"started"` or `"completed"`.
//!
//! Surface wrappers (CLI / MCP) call [`write_invocation_started`] and
//! [`write_invocation_completed`] after the in-process `record_invocation` /
//! `complete_invocation` variants have validated inputs. The returned
//! [`InvocationAuditPaths`] lets callers include the event and audit paths
//! in their own log lines and surface envelopes.
//!
//! Feature gate: available only when the `audit` cargo feature is enabled
//! (which is guaranteed transitively by the `multi_agent` feature since
//! Phase 29 Wave 2 made `audit` a hard dep of `multi_agent`).

use std::fs;
use std::path::{Path, PathBuf};

use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::audit::audit_manifest::{write_audit_manifest, AuditManifest};
use crate::audit::surface_audit::{compute_surface_audit_hash, SurfaceManifest};
use crate::multi_agent::types::{AgentInvocation, EntityRef};
use crate::surface::Surface;
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Per-invocation paths returned to surface wrappers so they can include
/// them in their own audit envelopes / log lines.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InvocationAuditPaths {
    pub event_path: PathBuf,
    pub audit_path: PathBuf,
    pub surface_audit_hash: String,
}

// ---------------------------------------------------------------------------
// Path helper
// ---------------------------------------------------------------------------

/// Standard layout: `<manifest_root>/<invocation_id>.<phase>.json`.
/// Phase is `"started"` or `"completed"`. Pure path construction — no I/O.
pub fn invocation_event_path(manifest_root: &Path, invocation_id: Uuid, phase: &str) -> PathBuf {
    manifest_root.join(format!("{}.{}.json", invocation_id, phase))
}

// ---------------------------------------------------------------------------
// Write helpers
// ---------------------------------------------------------------------------

/// Write the `agent_invocation_started` event file and companion audit
/// manifest under `manifest_root`.
///
/// 1. Creates `manifest_root` (and any intermediate directories) if absent.
/// 2. Writes `<id>.started.json` — the event payload (pretty-printed JSON).
/// 3. Computes `output_sha256` from the written file.
/// 4. Builds and writes `<id>.started.json.audit.json` via
///    [`crate::audit::audit_manifest::write_audit_manifest`].
/// 5. Returns the paths + surface audit hash.
pub fn write_invocation_started(
    manifest_root: &Path,
    invocation: &AgentInvocation,
) -> CorpFinanceResult<InvocationAuditPaths> {
    fs::create_dir_all(manifest_root).map_err(|e| {
        crate::error::CorpFinanceError::SerializationError(format!(
            "audit_writer: cannot create manifest root {manifest_root:?}: {e}"
        ))
    })?;

    let event_path = invocation_event_path(manifest_root, invocation.invocation_id, "started");

    let payload = serde_json::json!({
        "event": "agent_invocation_started",
        "invocation_id": invocation.invocation_id.to_string(),
        "target_agent": invocation.target_agent,
        "input_summary": invocation.input_summary,
        "tenant_id": invocation.tenant_id,
        "parent_invocation_id": invocation.parent_invocation_id.map(|u| u.to_string()),
        "ts": Utc::now().to_rfc3339(),
    });

    write_json_event(&event_path, &payload)?;
    build_and_write_audit(
        &event_path,
        invocation.invocation_id,
        &payload,
        invocation.tenant_id.as_deref(),
    )
}

/// Write the `agent_invocation_completed` event file and companion audit
/// manifest under `manifest_root`.
///
/// Identical structure to [`write_invocation_started`] but phase is
/// `"completed"` and the payload carries `output_summary` + `entities`.
pub fn write_invocation_completed(
    manifest_root: &Path,
    invocation_id: Uuid,
    output_summary: &str,
    entities: &[EntityRef],
    tenant_id: Option<&str>,
) -> CorpFinanceResult<InvocationAuditPaths> {
    fs::create_dir_all(manifest_root).map_err(|e| {
        crate::error::CorpFinanceError::SerializationError(format!(
            "audit_writer: cannot create manifest root {manifest_root:?}: {e}"
        ))
    })?;

    let event_path = invocation_event_path(manifest_root, invocation_id, "completed");

    let entity_summary: Vec<serde_json::Value> = entities
        .iter()
        .map(|e| {
            serde_json::json!({
                "kind": format!("{:?}", e.kind).to_lowercase(),
                "value": e.value,
            })
        })
        .collect();

    let payload = serde_json::json!({
        "event": "agent_invocation_completed",
        "invocation_id": invocation_id.to_string(),
        "output_summary": output_summary,
        "entities": entity_summary,
        "tenant_id": tenant_id,
        "ts": Utc::now().to_rfc3339(),
    });

    write_json_event(&event_path, &payload)?;
    build_and_write_audit(&event_path, invocation_id, &payload, tenant_id)
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

fn write_json_event(path: &Path, payload: &serde_json::Value) -> CorpFinanceResult<()> {
    let json = serde_json::to_string_pretty(payload).map_err(|e| {
        crate::error::CorpFinanceError::SerializationError(format!(
            "audit_writer: json serialize failed: {e}"
        ))
    })?;
    fs::write(path, json).map_err(|e| {
        crate::error::CorpFinanceError::SerializationError(format!(
            "audit_writer: cannot write event file {path:?}: {e}"
        ))
    })
}

fn build_and_write_audit(
    event_path: &Path,
    invocation_id: Uuid,
    payload: &serde_json::Value,
    tenant_id: Option<&str>,
) -> CorpFinanceResult<InvocationAuditPaths> {
    let output_sha256 = crate::audit::audit_manifest::sha256_file(event_path)?;

    let surface_manifest = SurfaceManifest {
        surface: Surface::Mcp,
        surface_event_id: invocation_id.to_string(),
        command_args: payload.clone(),
        output_paths: vec![event_path.display().to_string()],
    };

    let surface_audit_hash = compute_surface_audit_hash(&surface_manifest);

    let audit_manifest = AuditManifest {
        surface_audit_hash: surface_audit_hash.clone(),
        surface: Surface::Mcp,
        surface_event_id: invocation_id.to_string(),
        output_path: event_path.to_path_buf(),
        output_sha256,
        ts: Utc::now(),
        tool_calls: Vec::new(),
        tenant_id: tenant_id.map(str::to_owned),
    };

    let audit_path = write_audit_manifest(event_path, &audit_manifest)?;

    Ok(InvocationAuditPaths {
        event_path: event_path.to_path_buf(),
        audit_path,
        surface_audit_hash,
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit::audit_manifest::{output_path_to_audit_path, read_audit_manifest};
    use crate::audit::surface_audit::compute_surface_audit_hash;
    use crate::multi_agent::types::{EntityKind, EntityRef, InvocationStatus};

    fn mk_invocation() -> AgentInvocation {
        AgentInvocation {
            invocation_id: Uuid::now_v7(),
            target_agent: "cfa-equity-analyst".into(),
            parent_invocation_id: None,
            input_summary: "value AAPL".into(),
            tenant_id: Some("tenant-abc".into()),
            ts: Utc::now(),
            status: InvocationStatus::Pending,
        }
    }

    /// Test 1: both event file and audit companion exist after `write_started`.
    #[test]
    fn write_started_creates_event_and_audit_files() {
        let dir = tempfile::tempdir().unwrap();
        let inv = mk_invocation();
        let paths = write_invocation_started(dir.path(), &inv).unwrap();

        let expected_event = invocation_event_path(dir.path(), inv.invocation_id, "started");
        let expected_audit = output_path_to_audit_path(&expected_event);

        assert_eq!(paths.event_path, expected_event);
        assert_eq!(paths.audit_path, expected_audit);
        assert!(paths.event_path.exists(), "event file must exist");
        assert!(paths.audit_path.exists(), "audit companion must exist");
    }

    /// Test 2: `surface_audit_hash` returned equals independently recomputed
    /// hash — pins the contract that the writer doesn't drift from `record_invocation`.
    #[test]
    fn write_started_audit_hash_matches_compute_surface_audit_hash() {
        let dir = tempfile::tempdir().unwrap();
        let inv = mk_invocation();
        let paths = write_invocation_started(dir.path(), &inv).unwrap();

        // Read the event payload back to reconstruct the manifest exactly as
        // the writer built it.
        let raw = fs::read_to_string(&paths.event_path).unwrap();
        let payload: serde_json::Value = serde_json::from_str(&raw).unwrap();

        let reconstructed = SurfaceManifest {
            surface: Surface::Mcp,
            surface_event_id: inv.invocation_id.to_string(),
            command_args: payload,
            output_paths: vec![paths.event_path.display().to_string()],
        };
        let expected_hash = compute_surface_audit_hash(&reconstructed);
        assert_eq!(paths.surface_audit_hash, expected_hash);
    }

    /// Test 3: entity `kind` field in the completed event payload is lowercase.
    #[test]
    fn write_completed_event_payload_includes_entity_kinds_lowercase() {
        let dir = tempfile::tempdir().unwrap();
        let inv = mk_invocation();
        let entities = vec![EntityRef {
            kind: EntityKind::Issuer,
            value: "Apple Inc".into(),
            source_invocation: None,
        }];

        let paths =
            write_invocation_completed(dir.path(), inv.invocation_id, "done", &entities, None)
                .unwrap();

        let raw = fs::read_to_string(&paths.event_path).unwrap();
        let payload: serde_json::Value = serde_json::from_str(&raw).unwrap();
        let kind = payload["entities"][0]["kind"].as_str().unwrap();
        assert_eq!(kind, "issuer");
    }

    /// Test 4: started + completed for the same invocation_id produce two
    /// distinct event files and both audit companions.
    #[test]
    fn write_started_then_completed_creates_distinct_files() {
        let dir = tempfile::tempdir().unwrap();
        let inv = mk_invocation();

        let started = write_invocation_started(dir.path(), &inv).unwrap();
        let completed = write_invocation_completed(
            dir.path(),
            inv.invocation_id,
            "summary",
            &[],
            inv.tenant_id.as_deref(),
        )
        .unwrap();

        assert_ne!(started.event_path, completed.event_path);
        let started_str = started.event_path.to_string_lossy();
        let completed_str = completed.event_path.to_string_lossy();
        assert!(started_str.contains(".started."), "started path infix");
        assert!(
            completed_str.contains(".completed."),
            "completed path infix"
        );
        assert!(started.event_path.exists());
        assert!(started.audit_path.exists());
        assert!(completed.event_path.exists());
        assert!(completed.audit_path.exists());
    }

    /// Test 5: intermediate directories are created on demand.
    #[test]
    fn write_creates_intermediate_dirs() {
        let base = tempfile::tempdir().unwrap();
        let nested = base.path().join("a").join("b").join("c");
        assert!(!nested.exists(), "nested dir must not pre-exist");

        let inv = mk_invocation();
        let paths = write_invocation_started(&nested, &inv).unwrap();
        assert!(
            paths.event_path.exists(),
            "event file must exist after dir creation"
        );
    }

    /// Test 6: round-trip — read .audit.json back, assert `surface_event_id`
    /// and `tenant_id` preserved.
    #[test]
    fn audit_manifest_round_trip_preserves_surface_event_id() {
        let dir = tempfile::tempdir().unwrap();
        let inv = mk_invocation();
        let paths = write_invocation_started(dir.path(), &inv).unwrap();

        let manifest = read_audit_manifest(&paths.audit_path).unwrap();
        assert_eq!(manifest.surface_event_id, inv.invocation_id.to_string());
        assert_eq!(manifest.tenant_id.as_deref(), inv.tenant_id.as_deref());
    }
}
