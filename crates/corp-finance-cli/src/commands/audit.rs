//! CLI subcommands for `cfa audit <verb>` (Phase 26).
//!
//! Single verb in this slice:
//! - `cfa audit write --for <output>` — given an output file path, compute
//!   its SHA-256, build an `AuditManifest` from the active surface event
//!   context (env vars / file), and write `<output>.audit.json`.

use std::path::PathBuf;

use chrono::Utc;
use clap::{Args, Subcommand};
use serde_json::{json, Value};

use corp_finance_core::audit::audit_manifest::{
    sha256_file, write_audit_manifest, AuditManifest, ToolCallRecord,
};
use corp_finance_core::audit::surface_audit::{
    compute_surface_audit_hash, Surface, SurfaceManifest,
};

/// Top-level arg group for the `audit` subcommand group.
#[derive(Args)]
pub struct AuditArgs {
    #[command(subcommand)]
    pub command: AuditCommands,
}

#[derive(Subcommand)]
pub enum AuditCommands {
    /// Write an `<output>.audit.json` companion manifest.
    Write(AuditWriteArgs),
}

// ---------------------------------------------------------------------------
// AuditWriteArgs
// ---------------------------------------------------------------------------

#[derive(Args)]
pub struct AuditWriteArgs {
    /// Output file the audit manifest is companion to.
    #[arg(long = "for")]
    pub for_path: String,

    /// Optional path to a pre-built `AuditManifest` JSON; when supplied, the
    /// manifest is written verbatim (after recomputing `output_sha256`).
    #[arg(long)]
    pub manifest_file: Option<String>,
}

fn surface_from_env() -> Surface {
    match std::env::var("CFA_SURFACE")
        .ok()
        .as_deref()
        .map(str::to_ascii_lowercase)
    {
        Some(s) if s == "mcp" => Surface::Mcp,
        Some(s) if s == "skill" => Surface::Skill,
        Some(s) if s == "plugin" => Surface::Plugin,
        _ => Surface::Cli,
    }
}

pub fn run_write(args: AuditWriteArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let output_path = PathBuf::from(&args.for_path);
    if !output_path.exists() {
        return Err(format!("output file does not exist: {}", output_path.display()).into());
    }

    let output_sha256 = sha256_file(&output_path)?;

    let manifest: AuditManifest = if let Some(ref mfp) = args.manifest_file {
        let bytes = std::fs::read(mfp)?;
        let mut m: AuditManifest = serde_json::from_slice(&bytes)?;
        // Always recompute the sha so a stale manifest_file doesn't lie.
        m.output_path = output_path.clone();
        m.output_sha256 = output_sha256.clone();
        m
    } else {
        // Synthesise a manifest from environment context.
        let surface = surface_from_env();

        // surface_event_id: prefer env var; else derive from the program
        // argv[0] subcommand name.  Falls back to "audit.write".
        let surface_event_id = std::env::var("CFA_SURFACE_EVENT_ID").unwrap_or_else(|_| {
            std::env::args()
                .nth(1)
                .unwrap_or_else(|| "audit.write".to_string())
        });

        // tenant from env (optional).
        let tenant_id = std::env::var("CFA_TENANT_ID").ok();

        // Compute surface_audit_hash from a minimal SurfaceManifest derived
        // from the env / argv context.
        let command_args: Value = std::env::var("CFA_SURFACE_COMMAND_ARGS")
            .ok()
            .and_then(|s| serde_json::from_str::<Value>(&s).ok())
            .unwrap_or(Value::Null);

        let surface_manifest = SurfaceManifest {
            surface,
            surface_event_id: surface_event_id.clone(),
            command_args,
            output_paths: vec![output_path.display().to_string()],
        };
        let surface_audit_hash = compute_surface_audit_hash(&surface_manifest);

        AuditManifest {
            surface_audit_hash,
            surface,
            surface_event_id,
            output_path: output_path.clone(),
            output_sha256: output_sha256.clone(),
            ts: Utc::now(),
            tool_calls: Vec::<ToolCallRecord>::new(),
            tenant_id,
        }
    };

    let written_path = write_audit_manifest(&output_path, &manifest)?;

    Ok(json!({
        "status": "ok",
        "audit_path": written_path.display().to_string(),
        "output_sha256": output_sha256,
        "surface_audit_hash": manifest.surface_audit_hash,
        "surface_event_id": manifest.surface_event_id,
    }))
}
