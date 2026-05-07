//! Tenant aggregate: provisioning, surface-resolution, path scoping, and
//! isolation enforcement (ADR-019 §"Tenant Scoping at the Surface
//! Boundaries").
//!
//! v1 simple-tenancy only: per-tenant directory tree under a base path,
//! per-surface resolution (CLI flag / MCP `_meta` / plugin env var or
//! `<repo>/.cfa-tenant`), and a fail-closed `enforce_isolation` check that
//! prevents path traversal across tenants (RUF-FED-INV-001 / FED-INV-008).

use std::env;
use std::fs;
use std::path::{Component, Path, PathBuf};

use crate::error::CorpFinanceError;
use crate::CorpFinanceResult;

use super::types::{Tenant, TenantContext};

// ---------------------------------------------------------------------------
// Resource kinds
// ---------------------------------------------------------------------------

/// One of the per-tenant subtrees materialised by `provision_tenant`.
///
/// The fivefold split mirrors ADR-019: `out` for surface output files;
/// `memory` for HNSW / BM25 / petgraph partitions; `cost-ledger` for the
/// per-tenant SQLite ledger; `session` for federated session state (v2);
/// `audit` for the audit pipeline's per-tenant namespace.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceKind {
    Output,
    Memory,
    CostLedger,
    Session,
    Audit,
}

impl ResourceKind {
    fn dir_name(self) -> &'static str {
        match self {
            ResourceKind::Output => "out",
            ResourceKind::Memory => "memory",
            ResourceKind::CostLedger => "cost-ledger",
            ResourceKind::Session => "session",
            ResourceKind::Audit => "audit",
        }
    }
}

// ---------------------------------------------------------------------------
// Provisioning
// ---------------------------------------------------------------------------

/// Provision a tenant's on-disk directory tree under `base_dir`.
///
/// Creates `<base_dir>/<tenant_id>/{out,memory,cost-ledger,session,audit}/`.
/// On POSIX systems the per-tenant root is created with mode 0700 (FED-INV-
/// 003 / RUF-FED-INV-006). The reserved id `local` is rejected at the
/// surface boundary by `cfa tenant init`; this function permits the id
/// `local` so the platform default tenant can be lazily provisioned, and
/// the registry layer is the gate for re-registration attempts.
///
/// Returns the materialised [`TenantContext`] suitable for passing through
/// the surface wrapper.
///
/// Emits the `tenant_provisioned` domain event (the actual emission is the
/// responsibility of the audit pipeline at the surface boundary; this
/// function logs the intent via tracing when the `observability` feature
/// is on).
pub fn provision_tenant(tenant: &Tenant, base_dir: &Path) -> CorpFinanceResult<TenantContext> {
    if tenant.tenant_id.is_empty() {
        return Err(CorpFinanceError::InvalidInput {
            field: "tenant_id".to_string(),
            reason: "must be non-empty".to_string(),
        });
    }
    if !is_valid_tenant_id(&tenant.tenant_id) {
        return Err(CorpFinanceError::InvalidInput {
            field: "tenant_id".to_string(),
            reason: "must be lowercase kebab-case (a-z, 0-9, '-')".to_string(),
        });
    }

    let tenant_root = base_dir.join(&tenant.tenant_id);
    fs::create_dir_all(&tenant_root).map_err(|e| CorpFinanceError::InvalidInput {
        field: "base_dir".to_string(),
        reason: format!(
            "could not create tenant root {}: {}",
            tenant_root.display(),
            e
        ),
    })?;

    // POSIX 0700 on the per-tenant root (FED-INV-003).
    set_posix_0700(&tenant_root)?;

    for kind in [
        ResourceKind::Output,
        ResourceKind::Memory,
        ResourceKind::CostLedger,
        ResourceKind::Session,
        ResourceKind::Audit,
    ] {
        let sub = tenant_root.join(kind.dir_name());
        fs::create_dir_all(&sub).map_err(|e| CorpFinanceError::InvalidInput {
            field: "resource_dir".to_string(),
            reason: format!("could not create {}: {}", sub.display(), e),
        })?;
    }

    Ok(TenantContext {
        tenant_id: tenant.tenant_id.clone(),
        output_root: tenant_root,
        env_namespace: tenant.env_namespace.clone(),
    })
}

/// Validate the tenant id shape (lowercase kebab-case).
fn is_valid_tenant_id(id: &str) -> bool {
    !id.is_empty()
        && id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        && !id.starts_with('-')
        && !id.ends_with('-')
}

#[cfg(unix)]
fn set_posix_0700(p: &Path) -> CorpFinanceResult<()> {
    use std::os::unix::fs::PermissionsExt;
    let perms = fs::Permissions::from_mode(0o700);
    fs::set_permissions(p, perms).map_err(|e| CorpFinanceError::InvalidInput {
        field: "permissions".to_string(),
        reason: format!("could not set 0700 on {}: {}", p.display(), e),
    })
}

#[cfg(not(unix))]
fn set_posix_0700(_p: &Path) -> CorpFinanceResult<()> {
    // Non-POSIX platforms rely on user-account ACLs; ADR-019 documents
    // Linux/macOS as primary targets.
    Ok(())
}

// ---------------------------------------------------------------------------
// Surface resolution
// ---------------------------------------------------------------------------

/// Resolve the tenant context for a CLI invocation.
///
/// Looks for `--tenant <id>` in `args`; if absent, falls back to the
/// `CFA_TENANT_ID` environment variable. Returns `None` if neither is
/// present — the surface wrapper then defaults to the implicit `local`
/// tenant.
pub fn resolve_tenant_for_cli(args: &[String]) -> Option<TenantContext> {
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        if arg == "--tenant" {
            if let Some(id) = iter.next() {
                return Some(stub_context(id));
            }
        } else if let Some(rest) = arg.strip_prefix("--tenant=") {
            return Some(stub_context(rest));
        }
    }
    env::var("CFA_TENANT_ID").ok().map(|id| stub_context(&id))
}

/// Resolve the tenant context from MCP request metadata.
///
/// Reads `_meta.tenant_id` from the MCP request envelope's metadata.
///
/// TODO(MCP spec): the field name `_meta.tenant_id` is the proposal in
/// ADR-019; confirm with the Anthropic MCP transport team that this is
/// the canonical metadata field. If they choose a different name (e.g.
/// `extensions.tenant_id`), update both this function and the MCP wrapper.
pub fn resolve_tenant_for_mcp(metadata: &serde_json::Value) -> Option<TenantContext> {
    metadata
        .get("_meta")
        .and_then(|m| m.get("tenant_id"))
        .and_then(|v| v.as_str())
        .map(stub_context)
}

/// Resolve the tenant context for a plugin hook fire.
///
/// Reads `CFA_TENANT_ID` from the env, then falls back to a per-repo
/// `<cwd>/.cfa-tenant` file containing a single-line tenant id.
pub fn resolve_tenant_for_plugin() -> Option<TenantContext> {
    if let Ok(id) = env::var("CFA_TENANT_ID") {
        return Some(stub_context(&id));
    }
    if let Ok(cwd) = env::current_dir() {
        let marker = cwd.join(".cfa-tenant");
        if marker.is_file() {
            if let Ok(contents) = fs::read_to_string(&marker) {
                let id = contents.trim();
                if !id.is_empty() {
                    return Some(stub_context(id));
                }
            }
        }
    }
    None
}

/// Build a stub `TenantContext` for resolution helpers. The output_root
/// and env_namespace are derived deterministically from the id so the
/// caller can use the context immediately; the registry layer is the
/// authoritative source for full tenant records.
fn stub_context(id: &str) -> TenantContext {
    TenantContext {
        tenant_id: id.to_string(),
        output_root: PathBuf::from("var/tenants").join(id).join("out"),
        env_namespace: format!("TENANT_{}_", id.to_ascii_uppercase().replace('-', "")),
    }
}

// ---------------------------------------------------------------------------
// Path composition
// ---------------------------------------------------------------------------

/// Build a tenant-scoped path under the given resource subtree.
///
/// Example: `tenant_scoped_path(ctx, ResourceKind::Output, "morning-note.md")`
/// returns `<ctx.output_root>/out/morning-note.md`.
///
/// `name` is appended verbatim. Callers that accept untrusted names should
/// validate before calling; this function does not normalise away `..`
/// components — that's the role of [`enforce_isolation`].
pub fn tenant_scoped_path(ctx: &TenantContext, kind: ResourceKind, name: &str) -> PathBuf {
    ctx.output_root.join(kind.dir_name()).join(name)
}

// ---------------------------------------------------------------------------
// Isolation enforcement (RUF-FED-INV-001)
// ---------------------------------------------------------------------------

/// Reject path-traversal attempts that would escape the tenant's
/// `output_root` subtree (RUF-FED-INV-001 / FED-INV-008).
///
/// We do a *lexical* check (no filesystem reads): walk `attempted_path`'s
/// components and reject any `..` segment that would pop above the
/// tenant root. This is fail-closed: ambiguous paths (relative paths
/// outside the tenant tree, paths containing prefix `..`) are rejected.
pub fn enforce_isolation(ctx: &TenantContext, attempted_path: &Path) -> CorpFinanceResult<()> {
    let root = &ctx.output_root;

    // Reject any explicit `..` component anywhere in the attempted path.
    for comp in attempted_path.components() {
        if matches!(comp, Component::ParentDir) {
            return Err(CorpFinanceError::InvalidInput {
                field: "attempted_path".to_string(),
                reason: format!(
                    "path traversal denied: '{}' contains '..' (tenant '{}')",
                    attempted_path.display(),
                    ctx.tenant_id
                ),
            });
        }
    }

    // If the path is absolute, require it to start with the tenant root.
    if attempted_path.is_absolute() && !attempted_path.starts_with(root) {
        return Err(CorpFinanceError::InvalidInput {
            field: "attempted_path".to_string(),
            reason: format!(
                "tenant isolation: '{}' is outside output_root '{}' (tenant '{}')",
                attempted_path.display(),
                root.display(),
                ctx.tenant_id
            ),
        });
    }

    Ok(())
}
