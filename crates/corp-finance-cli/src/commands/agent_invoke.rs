//! CLI subcommands for `cfa agent-invoke <verb>` (Phase 29 Wave 2).
//!
//! Two verbs:
//! - `cfa agent-invoke record` — validate and persist an
//!   `agent_invocation_started` event + `.audit.json` companion under
//!   `manifest_root`. Calls
//!   [`corp_finance_core::multi_agent::record_invocation_with_audit`].
//! - `cfa agent-invoke complete` — persist an `agent_invocation_completed`
//!   event + `.audit.json` companion. Calls
//!   [`corp_finance_core::multi_agent::complete_invocation_with_audit`].
//!
//! Both commands print a single JSON object to stdout:
//! ```json
//! {
//!   "event_path": "/abs/path/to/<id>.<phase>.json",
//!   "audit_path": "/abs/path/to/<id>.<phase>.json.audit.json",
//!   "surface_audit_hash": "djb2:0xabcdef12"
//! }
//! ```
//! and exit 0 on success.

use std::path::PathBuf;

use chrono::Utc;
use clap::{Args, Subcommand};
use serde_json::Value;
use uuid::Uuid;

use corp_finance_core::multi_agent::types::{AgentInvocation, EntityRef, InvocationStatus};
use corp_finance_core::multi_agent::{
    complete_invocation_with_audit, record_invocation_with_audit, InvocationAuditPaths,
};

// ---------------------------------------------------------------------------
// Top-level arg group
// ---------------------------------------------------------------------------

/// Top-level arg group for the `agent-invoke` subcommand group.
#[derive(Args)]
pub struct AgentInvokeArgs {
    #[command(subcommand)]
    pub command: AgentInvokeCommands,
}

#[derive(Subcommand)]
pub enum AgentInvokeCommands {
    /// Record an agent_invocation_started event and write the audit companion.
    Record(AgentInvokeRecordArgs),
    /// Record an agent_invocation_completed event and write the audit companion.
    Complete(AgentInvokeCompleteArgs),
}

// ---------------------------------------------------------------------------
// Record args
// ---------------------------------------------------------------------------

#[derive(Args)]
pub struct AgentInvokeRecordArgs {
    /// Unique identifier for this invocation (UUID v7 recommended).
    #[arg(long = "invocation-id")]
    pub invocation_id: String,

    /// Target specialist slug (must be in REGISTERED_CFA_AGENTS).
    #[arg(long)]
    pub target: String,

    /// Short description of the input handed to the specialist.
    #[arg(long = "input-summary")]
    pub input_summary: String,

    /// Parent invocation id, if this is a sub-call inside an existing chain.
    #[arg(long)]
    pub parent: Option<String>,

    /// Federation tenant scope.
    #[arg(long)]
    pub tenant: Option<String>,

    /// Directory under which event and audit files are written.
    /// Defaults to `<cwd>/var/agent-invocations`.
    #[arg(long = "manifest-root")]
    pub manifest_root: Option<PathBuf>,
}

// ---------------------------------------------------------------------------
// Complete args
// ---------------------------------------------------------------------------

#[derive(Args)]
pub struct AgentInvokeCompleteArgs {
    /// Invocation id matching the earlier `record` call.
    #[arg(long = "invocation-id")]
    pub invocation_id: String,

    /// Short description of the specialist output.
    #[arg(long = "output-summary")]
    pub output_summary: String,

    /// JSON array of extracted entities, e.g.
    /// `[{"kind":"ticker","value":"AAPL","source_invocation":null}]`.
    #[arg(long)]
    pub entities: Option<String>,

    /// Federation tenant scope.
    #[arg(long)]
    pub tenant: Option<String>,

    /// Directory under which event and audit files are written.
    /// Defaults to `<cwd>/var/agent-invocations`.
    #[arg(long = "manifest-root")]
    pub manifest_root: Option<PathBuf>,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn resolve_manifest_root(arg: Option<PathBuf>) -> Result<PathBuf, Box<dyn std::error::Error>> {
    match arg {
        Some(p) => Ok(p),
        None => {
            let cwd = std::env::current_dir()?;
            Ok(cwd.join("var").join("agent-invocations"))
        }
    }
}

fn paths_to_json(paths: &InvocationAuditPaths) -> Value {
    serde_json::json!({
        "event_path": paths.event_path.display().to_string(),
        "audit_path": paths.audit_path.display().to_string(),
        "surface_audit_hash": paths.surface_audit_hash,
    })
}

fn parse_entities(s: &str) -> Result<Vec<EntityRef>, Box<dyn std::error::Error>> {
    let raw: Vec<serde_json::Value> =
        serde_json::from_str(s).map_err(|e| format!("--entities is not valid JSON: {}", e))?;
    raw.into_iter()
        .map(serde_json::from_value)
        .collect::<Result<Vec<EntityRef>, _>>()
        .map_err(|e| -> Box<dyn std::error::Error> {
            format!("--entities element parse failed: {}", e).into()
        })
}

// ---------------------------------------------------------------------------
// run_record
// ---------------------------------------------------------------------------

pub fn run_record(args: AgentInvokeRecordArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let invocation_id = Uuid::parse_str(&args.invocation_id)
        .map_err(|e| format!("invalid --invocation-id: {}", e))?;

    let parent_invocation_id = args
        .parent
        .as_deref()
        .map(Uuid::parse_str)
        .transpose()
        .map_err(|e| format!("invalid --parent: {}", e))?;

    let invocation = AgentInvocation {
        invocation_id,
        target_agent: args.target,
        parent_invocation_id,
        input_summary: args.input_summary,
        tenant_id: args.tenant,
        ts: Utc::now(),
        status: InvocationStatus::Pending,
    };

    let manifest_root = resolve_manifest_root(args.manifest_root)?;
    let paths = record_invocation_with_audit(&invocation, &manifest_root)?;
    Ok(paths_to_json(&paths))
}

// ---------------------------------------------------------------------------
// run_complete
// ---------------------------------------------------------------------------

pub fn run_complete(args: AgentInvokeCompleteArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let invocation_id = Uuid::parse_str(&args.invocation_id)
        .map_err(|e| format!("invalid --invocation-id: {}", e))?;

    let entities = match args.entities.as_deref() {
        None => Vec::new(),
        Some(s) => parse_entities(s)?,
    };

    let manifest_root = resolve_manifest_root(args.manifest_root)?;
    let paths = complete_invocation_with_audit(
        invocation_id,
        &args.output_summary,
        &entities,
        args.tenant.as_deref(),
        &manifest_root,
    )?;
    Ok(paths_to_json(&paths))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use corp_finance_core::multi_agent::audit_writer::invocation_event_path;

    fn new_id() -> String {
        Uuid::now_v7().to_string()
    }

    /// Test 1: record writes both event and audit files; returned paths exist.
    #[test]
    fn record_writes_started_event_and_audit_files() {
        let dir = tempfile::tempdir().unwrap();
        let id = new_id();
        let args = AgentInvokeRecordArgs {
            invocation_id: id.clone(),
            target: "cfa-equity-analyst".into(),
            input_summary: "Initiate AAPL coverage".into(),
            parent: None,
            tenant: Some("local".into()),
            manifest_root: Some(dir.path().to_path_buf()),
        };
        let value = run_record(args).unwrap();

        let event_path = PathBuf::from(value["event_path"].as_str().unwrap());
        let audit_path = PathBuf::from(value["audit_path"].as_str().unwrap());

        assert!(event_path.exists(), "event file must exist");
        assert!(audit_path.exists(), "audit file must exist");
        assert!(
            value["surface_audit_hash"]
                .as_str()
                .unwrap()
                .starts_with("djb2:"),
            "hash must carry djb2: prefix"
        );
    }

    /// Test 2: complete writes the completed event and audit files.
    #[test]
    fn complete_writes_completed_event() {
        let dir = tempfile::tempdir().unwrap();
        let id = new_id();

        // Record first.
        let record_args = AgentInvokeRecordArgs {
            invocation_id: id.clone(),
            target: "cfa-credit-analyst".into(),
            input_summary: "credit check XYZ".into(),
            parent: None,
            tenant: None,
            manifest_root: Some(dir.path().to_path_buf()),
        };
        run_record(record_args).unwrap();

        let complete_args = AgentInvokeCompleteArgs {
            invocation_id: id.clone(),
            output_summary: "BB+ / stable".into(),
            entities: Some(
                r#"[{"kind":"issuer","value":"XYZ Corp","source_invocation":null}]"#.into(),
            ),
            tenant: None,
            manifest_root: Some(dir.path().to_path_buf()),
        };
        let value = run_complete(complete_args).unwrap();

        let event_path = PathBuf::from(value["event_path"].as_str().unwrap());
        assert!(event_path.exists(), "completed event file must exist");
        let name = event_path.file_name().unwrap().to_string_lossy();
        assert!(
            name.contains(".completed."),
            "filename must contain .completed. infix"
        );
    }

    /// Test 3: unknown agent slug surfaces the error; no I/O attempted.
    #[test]
    fn record_rejects_unknown_agent_slug() {
        let dir = tempfile::tempdir().unwrap();
        let id = new_id();
        let args = AgentInvokeRecordArgs {
            invocation_id: id.clone(),
            target: "not-a-real-analyst".into(),
            input_summary: "test".into(),
            parent: None,
            tenant: None,
            manifest_root: Some(dir.path().to_path_buf()),
        };
        let result = run_record(args);
        assert!(result.is_err(), "unknown slug must return Err");

        // Confirm no event file was created with the given id.
        let uuid = Uuid::parse_str(&id).unwrap();
        let would_be = invocation_event_path(dir.path(), uuid, "started");
        assert!(
            !would_be.exists(),
            "no event file must be written for an unknown slug"
        );
    }
}
