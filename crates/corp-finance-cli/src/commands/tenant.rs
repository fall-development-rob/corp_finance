//! CLI subcommands for `cfa tenant <verb>` (Phase 27).
//!
//! Two verbs:
//! - `cfa tenant list [--base-dir PATH]` — list tenants under `--base-dir`
//!   (default `<cwd>/var/tenants`). Each tenant directory is expected to
//!   contain a `tenant.json` manifest (otherwise the directory is
//!   silently skipped).
//! - `cfa tenant scope --tenant-id <id> [--base-dir PATH]
//!   [--resource ...] [--name ...]` — resolve a tenant-scoped path under
//!   one of the five `ResourceKind` subtrees. With no `--resource`/`--name`,
//!   returns the active `TenantContext`.
//!
//! See `corp_finance_core::federation::tenant` for the underlying
//! semantics (`FED-INV-008` isolation, `RUF-FED-INV-001` provisioning).

use std::path::PathBuf;

use clap::{Args, Subcommand};
use serde_json::{json, Value};

use corp_finance_core::federation::tenant::{tenant_scoped_path, ResourceKind};
use corp_finance_core::federation::types::{Tenant, TenantContext};

/// Top-level arg group for the `tenant` subcommand group.
#[derive(Args)]
pub struct TenantArgs {
    #[command(subcommand)]
    pub command: TenantCommands,
}

#[derive(Subcommand)]
pub enum TenantCommands {
    /// List tenants found under `--base-dir`.
    List(TenantListArgs),
    /// Resolve a tenant-scoped path, or return the active TenantContext.
    Scope(TenantScopeArgs),
}

// ---------------------------------------------------------------------------
// Default base dir
// ---------------------------------------------------------------------------

fn default_base_dir() -> PathBuf {
    let mut p = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    p.push("var");
    p.push("tenants");
    p
}

fn resolve_base_dir(opt: Option<&str>) -> PathBuf {
    match opt {
        Some(s) => PathBuf::from(s),
        None => default_base_dir(),
    }
}

fn parse_resource_kind(s: &str) -> Result<ResourceKind, Box<dyn std::error::Error>> {
    match s.to_ascii_lowercase().as_str() {
        "output" | "out" => Ok(ResourceKind::Output),
        "memory" => Ok(ResourceKind::Memory),
        "cost-ledger" | "cost_ledger" | "costledger" => Ok(ResourceKind::CostLedger),
        "session" => Ok(ResourceKind::Session),
        "audit" => Ok(ResourceKind::Audit),
        other => Err(format!(
            "unknown --resource '{}'; expected output | memory | cost-ledger | session | audit",
            other
        )
        .into()),
    }
}

// ---------------------------------------------------------------------------
// TenantListArgs
// ---------------------------------------------------------------------------

#[derive(Args)]
pub struct TenantListArgs {
    /// Base directory under which each tenant has its own subdirectory.
    /// Defaults to `<cwd>/var/tenants`.
    #[arg(long)]
    pub base_dir: Option<String>,
}

pub fn run_list(args: TenantListArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let base = resolve_base_dir(args.base_dir.as_deref());

    let mut tenants: Vec<Tenant> = Vec::new();
    if !base.exists() {
        return Ok(json!({
            "base_dir": base.display().to_string(),
            "tenants": tenants,
            "note": "base directory does not exist",
        }));
    }

    let read_dir = std::fs::read_dir(&base)
        .map_err(|e| format!("could not read --base-dir '{}': {}", base.display(), e))?;

    for entry in read_dir.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let manifest = path.join("tenant.json");
        if !manifest.is_file() {
            continue;
        }
        let bytes = match std::fs::read(&manifest) {
            Ok(b) => b,
            Err(e) => {
                eprintln!(
                    "[cfa] tenant list: skipping {} (read error: {})",
                    manifest.display(),
                    e
                );
                continue;
            }
        };
        match serde_json::from_slice::<Tenant>(&bytes) {
            Ok(t) => tenants.push(t),
            Err(e) => {
                eprintln!(
                    "[cfa] tenant list: skipping {} (parse error: {})",
                    manifest.display(),
                    e
                );
            }
        }
    }

    // Stable order: sort by tenant_id for deterministic output.
    tenants.sort_by(|a, b| a.tenant_id.cmp(&b.tenant_id));

    Ok(json!({
        "base_dir": base.display().to_string(),
        "count": tenants.len(),
        "tenants": tenants,
    }))
}

// ---------------------------------------------------------------------------
// TenantScopeArgs
// ---------------------------------------------------------------------------

#[derive(Args)]
pub struct TenantScopeArgs {
    /// Tenant id (lowercase kebab-case).
    #[arg(long)]
    pub tenant_id: String,

    /// Base directory under which the tenant's subtree lives. Defaults
    /// to `<cwd>/var/tenants`.
    #[arg(long)]
    pub base_dir: Option<String>,

    /// Resource subtree: output | memory | cost-ledger | session | audit.
    #[arg(long)]
    pub resource: Option<String>,

    /// Leaf name appended to the resource subtree path.
    #[arg(long)]
    pub name: Option<String>,
}

pub fn run_scope(args: TenantScopeArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let base = resolve_base_dir(args.base_dir.as_deref());
    let tenant_root = base.join(&args.tenant_id);
    let env_namespace = format!(
        "TENANT_{}_",
        args.tenant_id.to_ascii_uppercase().replace('-', "")
    );
    let ctx = TenantContext {
        tenant_id: args.tenant_id.clone(),
        output_root: tenant_root.clone(),
        env_namespace,
    };

    match (args.resource.as_deref(), args.name.as_deref()) {
        (None, None) => Ok(json!({
            "tenant_context": ctx,
            "tenant_root": tenant_root.display().to_string(),
        })),
        (Some(r), Some(n)) => {
            let kind = parse_resource_kind(r)?;
            let path = tenant_scoped_path(&ctx, kind, n);
            Ok(json!({
                "tenant_id": args.tenant_id,
                "resource": r,
                "name": n,
                "path": path.display().to_string(),
            }))
        }
        (Some(_), None) => Err("--resource requires --name".into()),
        (None, Some(_)) => Err("--name requires --resource".into()),
    }
}
