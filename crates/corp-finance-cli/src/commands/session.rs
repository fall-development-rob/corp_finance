//! CLI subcommands for `cfa session <verb>` (Phase 26).
//!
//! Two verbs:
//! - `cfa session save` — persist a `CfaSession` JSON file to the portable
//!   `.cfa-session` (gzip+JSON) archive format.
//! - `cfa session restore` — restore a `.cfa-session` archive to a JSON
//!   payload printed to stdout.

use std::path::PathBuf;

use clap::{Args, Subcommand};
use serde_json::Value;

use corp_finance_core::memory::cfa_session;
use corp_finance_core::memory::types::CfaSession;

/// Top-level arg group for the `session` subcommand group.
#[derive(Args)]
pub struct SessionArgs {
    #[command(subcommand)]
    pub command: SessionCommands,
}

#[derive(Subcommand)]
pub enum SessionCommands {
    /// Persist a `CfaSession` JSON file to a `.cfa-session` archive.
    Save(SessionSaveArgs),
    /// Restore a `.cfa-session` archive to a JSON payload (stdout).
    Restore(SessionRestoreArgs),
}

// ---------------------------------------------------------------------------
// SessionSaveArgs
// ---------------------------------------------------------------------------

#[derive(Args)]
pub struct SessionSaveArgs {
    /// Path to a JSON file containing the `CfaSession` payload.
    #[arg(long)]
    pub session_file: String,

    /// Destination path for the `.cfa-session` archive.
    #[arg(long)]
    pub to: String,
}

pub fn run_save(args: SessionSaveArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let bytes = std::fs::read(&args.session_file)?;
    let session: CfaSession = serde_json::from_slice(&bytes)?;
    let dest = PathBuf::from(&args.to);
    if let Some(parent) = dest.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            std::fs::create_dir_all(parent)?;
        }
    }
    cfa_session::save(&session, &dest)?;
    Ok(serde_json::json!({
        "status": "ok",
        "session_id": session.session_id,
        "path": dest.display().to_string(),
        "run_summary_count": session.run_summaries.len(),
    }))
}

// ---------------------------------------------------------------------------
// SessionRestoreArgs
// ---------------------------------------------------------------------------

#[derive(Args)]
pub struct SessionRestoreArgs {
    /// Path to the `.cfa-session` archive to restore.
    #[arg(long)]
    pub from: String,
}

pub fn run_restore(args: SessionRestoreArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let path = PathBuf::from(&args.from);
    let session = cfa_session::restore(&path)?;
    Ok(serde_json::to_value(session)?)
}
