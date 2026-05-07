//! CLI subcommands for `cfa attest <verb>` (Phase 29 Wave 4).
//!
//! Five verbs covering the federation v2 trust attestation surface:
//! - `cfa attest issue`  — issue a new signed attestation and persist it.
//! - `cfa attest verify` — load and crypto-verify an attestation by id.
//! - `cfa attest list`   — list all attestations for a subject tenant.
//! - `cfa attest revoke` — record a revocation tombstone.
//! - `cfa attest check`  — check whether a subject holds a live capability.
//!
//! Each verb prints one JSON object to stdout and exits 0 on success.

use std::path::{Path, PathBuf};

use chrono::Utc;
use clap::{Args, Subcommand};
use serde_json::{json, Value};
use uuid::Uuid;

use corp_finance_core::federation::attestation::{
    check_capability, init_attestation_schema, issue_attestation, list_attestations_by_subject,
    load_attestation, revoke_attestation, save_attestation, status_with_revocations,
};
use corp_finance_core::self_learning::signing::ensure_keypair;

// ---------------------------------------------------------------------------
// Top-level arg group
// ---------------------------------------------------------------------------

/// Top-level arg group for the `attest` subcommand group.
#[derive(Args)]
pub struct AttestArgs {
    #[command(subcommand)]
    pub command: AttestSubcommand,
}

#[derive(Subcommand)]
pub enum AttestSubcommand {
    /// Issue a new trust attestation from issuer to subject.
    Issue(AttestIssueArgs),
    /// Verify the cryptographic integrity and status of an attestation.
    Verify(AttestVerifyArgs),
    /// List all attestations for a given subject tenant.
    List(AttestListArgs),
    /// Record a revocation tombstone for an attestation.
    Revoke(AttestRevokeArgs),
    /// Check whether a subject tenant currently holds a named capability.
    Check(AttestCheckArgs),
}

// ---------------------------------------------------------------------------
// Per-verb arg structs
// ---------------------------------------------------------------------------

#[derive(Args)]
pub struct AttestIssueArgs {
    /// Issuer tenant id.
    #[arg(long)]
    pub issuer: String,

    /// Subject tenant id.
    #[arg(long)]
    pub subject: String,

    /// Capability to grant. Repeatable: --capability A --capability B.
    #[arg(long = "capability")]
    pub capabilities: Vec<String>,

    /// Validity window in hours (default 24).
    #[arg(long = "valid-for-hours", default_value = "24")]
    pub valid_for_hours: i64,

    /// Path to the SQLite trust store.
    #[arg(long)]
    pub db: Option<PathBuf>,

    /// Directory for ed25519 signing key storage.
    #[arg(long = "keys-dir")]
    pub keys_dir: Option<PathBuf>,
}

#[derive(Args)]
pub struct AttestVerifyArgs {
    /// UUID of the attestation to verify.
    #[arg(long = "attestation-id")]
    pub attestation_id: String,

    /// Expected issuer tenant id.
    #[arg(long = "expected-issuer")]
    pub expected_issuer: String,

    /// Path to the SQLite trust store.
    #[arg(long)]
    pub db: Option<PathBuf>,
}

#[derive(Args)]
pub struct AttestListArgs {
    /// Subject tenant id to list attestations for.
    #[arg(long)]
    pub subject: String,

    /// Path to the SQLite trust store.
    #[arg(long)]
    pub db: Option<PathBuf>,
}

#[derive(Args)]
pub struct AttestRevokeArgs {
    /// UUID of the attestation to revoke.
    #[arg(long = "attestation-id")]
    pub attestation_id: String,

    /// Identity revoking the attestation.
    #[arg(long = "revoked-by")]
    pub revoked_by: String,

    /// Human-readable revocation reason.
    #[arg(long)]
    pub reason: String,

    /// Path to the SQLite trust store.
    #[arg(long)]
    pub db: Option<PathBuf>,
}

#[derive(Args)]
pub struct AttestCheckArgs {
    /// Subject tenant id.
    #[arg(long)]
    pub subject: String,

    /// Capability name to check.
    #[arg(long = "capability")]
    pub capability: String,

    /// Path to the SQLite trust store.
    #[arg(long)]
    pub db: Option<PathBuf>,
}

// ---------------------------------------------------------------------------
// Path helpers
// ---------------------------------------------------------------------------

fn resolve_db(arg: Option<PathBuf>) -> Result<PathBuf, Box<dyn std::error::Error>> {
    match arg {
        Some(p) => Ok(p),
        None => {
            let cwd = std::env::current_dir()?;
            Ok(cwd.join("var").join("federation").join("trust.db"))
        }
    }
}

fn resolve_keys_dir(arg: Option<PathBuf>) -> Result<PathBuf, Box<dyn std::error::Error>> {
    match arg {
        Some(p) => Ok(p),
        None => {
            let cwd = std::env::current_dir()?;
            Ok(cwd.join("var").join("federation").join("keys"))
        }
    }
}

fn open_db(path: &Path) -> Result<rusqlite::Connection, Box<dyn std::error::Error>> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("failed to create DB parent dir {}: {}", parent.display(), e))?;
    }
    let conn = rusqlite::Connection::open(path)
        .map_err(|e| format!("failed to open trust DB {}: {}", path.display(), e))?;
    init_attestation_schema(&conn)?;
    Ok(conn)
}

fn attestation_to_value(att: &corp_finance_core::federation::types::TrustAttestation) -> Value {
    json!({
        "attestation_id": att.attestation_id.to_string(),
        "issuer_tenant": att.issuer_tenant,
        "subject_tenant": att.subject_tenant,
        "capabilities": att.capabilities,
        "issued_at": att.issued_at.to_rfc3339(),
        "expires_at": att.expires_at.to_rfc3339(),
        "signature": att.signature,
        "public_key": att.public_key,
    })
}

// ---------------------------------------------------------------------------
// run_issue
// ---------------------------------------------------------------------------

pub fn run_issue(args: AttestIssueArgs) -> Result<Value, Box<dyn std::error::Error>> {
    if args.capabilities.is_empty() {
        return Err("at least one --capability is required".into());
    }
    if args.valid_for_hours <= 0 {
        return Err("--valid-for-hours must be positive".into());
    }

    let keys_dir = resolve_keys_dir(args.keys_dir)?;
    let (signing_key, _) = ensure_keypair(&keys_dir)?;

    let duration = chrono::Duration::hours(args.valid_for_hours);
    let attestation = issue_attestation(
        &args.issuer,
        &args.subject,
        args.capabilities,
        duration,
        &signing_key,
    )?;

    let db_path = resolve_db(args.db)?;
    let conn = open_db(&db_path)?;
    save_attestation(&conn, &attestation)?;

    Ok(attestation_to_value(&attestation))
}

// ---------------------------------------------------------------------------
// run_verify
// ---------------------------------------------------------------------------

pub fn run_verify(args: AttestVerifyArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let attestation_id = Uuid::parse_str(&args.attestation_id)
        .map_err(|e| format!("invalid --attestation-id: {}", e))?;

    let db_path = resolve_db(args.db)?;
    let conn = open_db(&db_path)?;

    let attestation = load_attestation(&conn, attestation_id)?
        .ok_or_else(|| format!("attestation {} not found", attestation_id))?;

    let status = status_with_revocations(&conn, &attestation, &args.expected_issuer)?;
    let status_str = serde_json::to_value(status)?
        .as_str()
        .unwrap_or("unknown")
        .to_string();

    Ok(json!({
        "attestation_id": attestation_id.to_string(),
        "expected_issuer": args.expected_issuer,
        "status": status_str,
    }))
}

// ---------------------------------------------------------------------------
// run_list
// ---------------------------------------------------------------------------

pub fn run_list(args: AttestListArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let db_path = resolve_db(args.db)?;
    let conn = open_db(&db_path)?;

    let attestations = list_attestations_by_subject(&conn, &args.subject)?;
    let count = attestations.len();
    let items: Vec<Value> = attestations.iter().map(attestation_to_value).collect();

    Ok(json!({
        "subject": args.subject,
        "count": count,
        "attestations": items,
    }))
}

// ---------------------------------------------------------------------------
// run_revoke
// ---------------------------------------------------------------------------

pub fn run_revoke(args: AttestRevokeArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let attestation_id = Uuid::parse_str(&args.attestation_id)
        .map_err(|e| format!("invalid --attestation-id: {}", e))?;

    let db_path = resolve_db(args.db)?;
    let conn = open_db(&db_path)?;

    let revoked_at = Utc::now();
    revoke_attestation(&conn, attestation_id, &args.revoked_by, &args.reason)?;

    Ok(json!({
        "attestation_id": attestation_id.to_string(),
        "revoked_by": args.revoked_by,
        "reason": args.reason,
        "revoked_at": revoked_at.to_rfc3339(),
    }))
}

// ---------------------------------------------------------------------------
// run_check
// ---------------------------------------------------------------------------

pub fn run_check(args: AttestCheckArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let db_path = resolve_db(args.db)?;
    let conn = open_db(&db_path)?;

    let granted = check_capability(&conn, &args.subject, &args.capability)?;

    Ok(json!({
        "subject": args.subject,
        "capability": args.capability,
        "granted": granted,
    }))
}

// ---------------------------------------------------------------------------
// Top-level dispatcher
// ---------------------------------------------------------------------------

pub fn run_attest(args: AttestArgs) -> Result<Value, Box<dyn std::error::Error>> {
    match args.command {
        AttestSubcommand::Issue(a) => run_issue(a),
        AttestSubcommand::Verify(a) => run_verify(a),
        AttestSubcommand::List(a) => run_list(a),
        AttestSubcommand::Revoke(a) => run_revoke(a),
        AttestSubcommand::Check(a) => run_check(a),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn temp_db_path(dir: &TempDir) -> PathBuf {
        dir.path().join("trust.db")
    }

    fn temp_keys_dir(dir: &TempDir) -> PathBuf {
        dir.path().join("keys")
    }

    /// Test 1: issue then verify in a second call returns status "valid".
    #[test]
    fn issue_then_verify_returns_valid() {
        let dir = TempDir::new().unwrap();
        let db = temp_db_path(&dir);
        let keys = temp_keys_dir(&dir);

        let issue_result = run_issue(AttestIssueArgs {
            issuer: "tenant-a".into(),
            subject: "tenant-b".into(),
            capabilities: vec!["read:portfolio".into()],
            valid_for_hours: 24,
            db: Some(db.clone()),
            keys_dir: Some(keys.clone()),
        })
        .unwrap();

        let att_id = issue_result["attestation_id"].as_str().unwrap().to_string();

        let verify_result = run_verify(AttestVerifyArgs {
            attestation_id: att_id.clone(),
            expected_issuer: "tenant-a".into(),
            db: Some(db.clone()),
        })
        .unwrap();

        assert_eq!(verify_result["status"], "valid");
        assert_eq!(verify_result["attestation_id"], att_id);
    }

    /// Test 2: issue capability, check returns true; revoke; check returns false.
    #[test]
    fn revoke_then_check_returns_false() {
        let dir = TempDir::new().unwrap();
        let db = temp_db_path(&dir);
        let keys = temp_keys_dir(&dir);

        let issue_result = run_issue(AttestIssueArgs {
            issuer: "tenant-a".into(),
            subject: "tenant-b".into(),
            capabilities: vec!["read:portfolio".into()],
            valid_for_hours: 24,
            db: Some(db.clone()),
            keys_dir: Some(keys.clone()),
        })
        .unwrap();

        let att_id = issue_result["attestation_id"].as_str().unwrap().to_string();

        let check_before = run_check(AttestCheckArgs {
            subject: "tenant-b".into(),
            capability: "read:portfolio".into(),
            db: Some(db.clone()),
        })
        .unwrap();
        assert_eq!(check_before["granted"], true);

        run_revoke(AttestRevokeArgs {
            attestation_id: att_id.clone(),
            revoked_by: "tenant-a".into(),
            reason: "key compromise".into(),
            db: Some(db.clone()),
        })
        .unwrap();

        let check_after = run_check(AttestCheckArgs {
            subject: "tenant-b".into(),
            capability: "read:portfolio".into(),
            db: Some(db.clone()),
        })
        .unwrap();
        assert_eq!(check_after["granted"], false);
    }

    /// Test 3: fresh tempdir DB returns count 0 for list.
    #[test]
    fn list_returns_empty_when_no_attestations() {
        let dir = TempDir::new().unwrap();
        let db = temp_db_path(&dir);

        let result = run_list(AttestListArgs {
            subject: "nobody".into(),
            db: Some(db),
        })
        .unwrap();

        assert_eq!(result["count"], 0);
        assert!(result["attestations"].as_array().unwrap().is_empty());
    }

    /// Test 4: issue with cap "A", check for cap "B" returns false.
    #[test]
    fn check_capability_returns_false_for_unknown_capability() {
        let dir = TempDir::new().unwrap();
        let db = temp_db_path(&dir);
        let keys = temp_keys_dir(&dir);

        run_issue(AttestIssueArgs {
            issuer: "tenant-a".into(),
            subject: "tenant-b".into(),
            capabilities: vec!["cap:A".into()],
            valid_for_hours: 24,
            db: Some(db.clone()),
            keys_dir: Some(keys),
        })
        .unwrap();

        let result = run_check(AttestCheckArgs {
            subject: "tenant-b".into(),
            capability: "cap:B".into(),
            db: Some(db),
        })
        .unwrap();

        assert_eq!(result["granted"], false);
    }
}
