//! CLI subcommands for `cfa cost <verb>` (Phase 26).
//!
//! Three verbs:
//! - `cfa cost summary` — aggregate `cost_events` by surface / tier / tenant.
//! - `cfa cost budget set` — insert-or-update a budget row.
//! - `cfa cost budget get` — read the budget for a key + check threshold.
//!
//! All money is integer cents (i64). The ledger is `rusqlite`-backed and
//! defaults to `<repo>/var/observability/cost-ledger.sqlite`.

use std::path::PathBuf;

use chrono::{DateTime, Utc};
use clap::{Args, Subcommand};
use serde_json::{json, Value};

use corp_finance_core::cost::budget::{check_threshold, get_budget, set_budget, BudgetFilter};
use corp_finance_core::cost::ledger::{CostFilter, CostLedger};
use corp_finance_core::cost::reporter::{summary, GroupBy};
use corp_finance_core::cost::types::{BudgetPeriod, CostBudget, Surface, TierTag};

/// Top-level arg group for the `cost` subcommand group.
#[derive(Args)]
pub struct CostArgs {
    #[command(subcommand)]
    pub command: CostCommands,
}

#[derive(Subcommand)]
pub enum CostCommands {
    /// Aggregate `cost_events` over a time window.
    Summary(CostSummaryArgs),
    /// Manage budgets (set / get).
    #[command(subcommand)]
    Budget(CostBudgetCommands),
}

#[derive(Subcommand)]
pub enum CostBudgetCommands {
    /// Insert-or-update a budget row.
    Set(CostBudgetSetArgs),
    /// Read the budget for a key and report threshold status.
    Get(CostBudgetGetArgs),
}

// ---------------------------------------------------------------------------
// Default ledger path
// ---------------------------------------------------------------------------

fn default_ledger_path() -> PathBuf {
    let mut p = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    p.push("var");
    p.push("observability");
    p.push("cost-ledger.sqlite");
    p
}

fn resolve_ledger_path(opt: Option<&str>) -> PathBuf {
    match opt {
        Some(s) => PathBuf::from(s),
        None => default_ledger_path(),
    }
}

fn ensure_parent_dir(path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            std::fs::create_dir_all(parent)?;
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// CostSummaryArgs
// ---------------------------------------------------------------------------

#[derive(Args)]
pub struct CostSummaryArgs {
    /// Path to the cost ledger SQLite file. Defaults to
    /// `<repo>/var/observability/cost-ledger.sqlite`.
    #[arg(long)]
    pub ledger: Option<String>,

    /// Lower bound on event timestamp (RFC3339, inclusive).
    #[arg(long)]
    pub since: Option<String>,

    /// Upper bound on event timestamp (RFC3339, inclusive).
    #[arg(long)]
    pub until: Option<String>,

    /// Filter by surface: cli | mcp | skill | plugin.
    #[arg(long)]
    pub surface: Option<String>,

    /// Filter by tier tag (snake_case from `TierTag::as_str()`).
    #[arg(long)]
    pub tier: Option<String>,

    /// Filter by tenant id.
    #[arg(long)]
    pub tenant: Option<String>,

    /// Bucket dimension: surface | tier | tenant | surface-and-tier.
    #[arg(long, default_value = "surface")]
    pub by: String,
}

fn parse_dt(s: &str, field: &str) -> Result<DateTime<Utc>, Box<dyn std::error::Error>> {
    let dt = DateTime::parse_from_rfc3339(s)
        .map_err(|e| format!("invalid {} timestamp '{}': {}", field, s, e))?;
    Ok(dt.with_timezone(&Utc))
}

fn parse_surface(s: &str) -> Result<Surface, Box<dyn std::error::Error>> {
    Surface::parse(s).ok_or_else(|| {
        format!(
            "unknown surface '{}'; expected cli | mcp | skill | plugin",
            s
        )
        .into()
    })
}

fn parse_tier(s: &str) -> Result<TierTag, Box<dyn std::error::Error>> {
    TierTag::parse(s).ok_or_else(|| {
        format!(
            "unknown tier '{}'; expected one of cookbook_core_only | cookbook_freemium | \
             cookbook_paid_vendor | mcp_free_native | mcp_free_public_with_api_key | \
             mcp_freemium | mcp_paid_vendor | unknown",
            s
        )
        .into()
    })
}

fn parse_period(s: &str) -> Result<BudgetPeriod, Box<dyn std::error::Error>> {
    BudgetPeriod::parse(s).ok_or_else(|| {
        format!(
            "unknown period '{}'; expected daily | weekly | monthly | total",
            s
        )
        .into()
    })
}

fn parse_group_by(s: &str) -> Result<GroupBy, Box<dyn std::error::Error>> {
    match s {
        "surface" => Ok(GroupBy::Surface),
        "tier" => Ok(GroupBy::Tier),
        "tenant" => Ok(GroupBy::Tenant),
        "surface-and-tier" | "surface_and_tier" => Ok(GroupBy::SurfaceAndTier),
        other => Err(format!(
            "unknown --by value '{}'; expected surface | tier | tenant | surface-and-tier",
            other
        )
        .into()),
    }
}

pub fn run_summary(args: CostSummaryArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let path = resolve_ledger_path(args.ledger.as_deref());
    ensure_parent_dir(&path)?;

    let ledger = CostLedger::open(&path)?;

    let filter = CostFilter {
        surface: args.surface.as_deref().map(parse_surface).transpose()?,
        tier: args.tier.as_deref().map(parse_tier).transpose()?,
        tenant_id: args.tenant.clone(),
        since: args
            .since
            .as_deref()
            .map(|s| parse_dt(s, "--since"))
            .transpose()?,
        until: args
            .until
            .as_deref()
            .map(|s| parse_dt(s, "--until"))
            .transpose()?,
    };

    let group_by = parse_group_by(&args.by)?;
    let result = summary(&ledger, &filter, group_by)?;
    Ok(serde_json::to_value(result)?)
}

// ---------------------------------------------------------------------------
// CostBudgetSetArgs
// ---------------------------------------------------------------------------

#[derive(Args)]
pub struct CostBudgetSetArgs {
    /// Path to the cost ledger SQLite file.
    #[arg(long)]
    pub ledger: Option<String>,

    /// Budget period: daily | weekly | monthly | total.
    #[arg(long)]
    pub period: String,

    /// Hard cap in integer cents.
    #[arg(long)]
    pub limit_cents: i64,

    /// Warning threshold percentage (0..=100).
    #[arg(long)]
    pub threshold_pct: u8,

    /// Optional surface filter.
    #[arg(long)]
    pub surface: Option<String>,

    /// Optional tier filter.
    #[arg(long)]
    pub tier: Option<String>,
}

pub fn run_budget_set(args: CostBudgetSetArgs) -> Result<Value, Box<dyn std::error::Error>> {
    if args.threshold_pct > 100 {
        return Err(format!(
            "--threshold-pct must be between 0 and 100, got {}",
            args.threshold_pct
        )
        .into());
    }

    let path = resolve_ledger_path(args.ledger.as_deref());
    ensure_parent_dir(&path)?;

    let ledger = CostLedger::open(&path)?;
    let budget = CostBudget {
        surface_filter: args.surface.as_deref().map(parse_surface).transpose()?,
        tier_filter: args.tier.as_deref().map(parse_tier).transpose()?,
        period: parse_period(&args.period)?,
        limit_cents: args.limit_cents,
        threshold_pct: args.threshold_pct,
    };

    set_budget(&ledger, &budget)?;
    Ok(serde_json::to_value(json!({
        "status": "ok",
        "budget": budget,
    }))?)
}

// ---------------------------------------------------------------------------
// CostBudgetGetArgs
// ---------------------------------------------------------------------------

#[derive(Args)]
pub struct CostBudgetGetArgs {
    /// Path to the cost ledger SQLite file.
    #[arg(long)]
    pub ledger: Option<String>,

    /// Budget period filter (defaults to `monthly`).
    #[arg(long)]
    pub period: Option<String>,

    /// Optional surface filter.
    #[arg(long)]
    pub surface: Option<String>,

    /// Optional tier filter.
    #[arg(long)]
    pub tier: Option<String>,
}

pub fn run_budget_get(args: CostBudgetGetArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let path = resolve_ledger_path(args.ledger.as_deref());
    ensure_parent_dir(&path)?;

    let ledger = CostLedger::open(&path)?;

    let filter = BudgetFilter {
        surface_filter: args.surface.as_deref().map(parse_surface).transpose()?,
        tier_filter: args.tier.as_deref().map(parse_tier).transpose()?,
        period: args.period.as_deref().map(parse_period).transpose()?,
    };

    let budget = get_budget(&ledger, &filter)?;
    let status = match &budget {
        Some(b) => Some(check_threshold(&ledger, b)?),
        None => None,
    };

    Ok(serde_json::to_value(json!({
        "budget": budget,
        "status": status,
    }))?)
}
