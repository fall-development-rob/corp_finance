//! CLI subcommands for `cfa plan <verb>` (Phase 27).
//!
//! Two verbs:
//! - `cfa plan show --goal "<text>"` — build an A* `GoapPlan` over the
//!   registered MCP-tool + slash-command action catalogue and print it
//!   as JSON or a tabular tree.
//! - `cfa plan replan --plan-file <path> --failed-step <uuid>
//!   --reason "<text>"` — read a persisted plan, mark the failed step
//!   (and downstream) and bump `replan_count`, then write the plan back.
//!
//! See `corp_finance_core::multi_agent::planner` for the underlying
//! semantics (`MAC-INV-006` replan budget, `MAC-INV-007` deterministic
//! plan-hash).

use std::path::PathBuf;

use clap::{Args, Subcommand};
use serde_json::{json, Value};
use uuid::Uuid;

use corp_finance_core::multi_agent::goap_adapter::load_action_catalogue;
use corp_finance_core::multi_agent::planner;
use corp_finance_core::multi_agent::types::{GoapPlan, PlanAction};

/// Top-level arg group for the `plan` subcommand group.
#[derive(Args)]
pub struct PlanArgs {
    #[command(subcommand)]
    pub command: PlanCommands,
}

#[derive(Subcommand)]
pub enum PlanCommands {
    /// Build and print an A* plan for a free-form goal.
    Show(PlanShowArgs),
    /// Replan from a failed step in an existing plan file.
    Replan(PlanReplanArgs),
}

// ---------------------------------------------------------------------------
// PlanShowArgs
// ---------------------------------------------------------------------------

#[derive(Args)]
pub struct PlanShowArgs {
    /// Free-form goal text. Drives the keyword-based decomposition in
    /// `multi_agent::planner::decompose_goal`.
    #[arg(long)]
    pub goal: String,

    /// Output format: `json` (default) or `table` (one row per step).
    #[arg(long, default_value = "json")]
    pub format: String,
}

pub fn run_show(args: PlanShowArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let catalogue = load_action_catalogue();
    let plan =
        planner::plan(&args.goal, &catalogue).map_err(|e| format!("planner error: {}", e))?;

    let format = args.format.to_ascii_lowercase();
    match format.as_str() {
        "json" => Ok(serde_json::to_value(&plan)?),
        "table" => {
            let rows: Vec<Value> = plan
                .steps
                .iter()
                .enumerate()
                .map(|(i, s)| {
                    let (kind, name) = match &s.action {
                        PlanAction::McpTool { name, .. } => ("mcp_tool", name.clone()),
                        PlanAction::SlashCommand { name, .. } => ("slash_command", name.clone()),
                        PlanAction::SpecialistAgent { slug, .. } => {
                            ("specialist_agent", slug.clone())
                        }
                    };
                    json!({
                        "index": i,
                        "step_id": s.step_id.to_string(),
                        "kind": kind,
                        "name": name,
                        "depends_on": s.depends_on.iter().map(|u| u.to_string()).collect::<Vec<_>>(),
                        "status": s.status,
                    })
                })
                .collect();
            Ok(json!({
                "plan_id": plan.plan_id.to_string(),
                "goal": plan.goal,
                "plan_hash": plan.plan_hash,
                "replan_count": plan.replan_count,
                "max_replans": plan.max_replans,
                "steps": rows,
            }))
        }
        other => Err(format!("unknown --format '{}'; expected json | table", other).into()),
    }
}

// ---------------------------------------------------------------------------
// PlanReplanArgs
// ---------------------------------------------------------------------------

#[derive(Args)]
pub struct PlanReplanArgs {
    /// Path to a JSON file containing a `GoapPlan`. The file is read,
    /// updated in place, and written back.
    #[arg(long)]
    pub plan_file: String,

    /// UUID of the step that failed (must be a step_id present in the
    /// plan).
    #[arg(long)]
    pub failed_step: String,

    /// Free-form reason recorded on the failed step's `result_summary`.
    #[arg(long)]
    pub reason: String,
}

pub fn run_replan(args: PlanReplanArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let path = PathBuf::from(&args.plan_file);
    let bytes = std::fs::read(&path)
        .map_err(|e| format!("could not read plan file '{}': {}", path.display(), e))?;
    let mut plan: GoapPlan = serde_json::from_slice(&bytes)
        .map_err(|e| format!("could not parse plan file '{}': {}", path.display(), e))?;

    let failed_step = Uuid::parse_str(&args.failed_step)
        .map_err(|e| format!("invalid --failed-step uuid '{}': {}", args.failed_step, e))?;

    planner::replan(&mut plan, failed_step, &args.reason)
        .map_err(|e| format!("replan error: {}", e))?;

    let updated = serde_json::to_vec_pretty(&plan)?;
    std::fs::write(&path, &updated)
        .map_err(|e| format!("could not write plan file '{}': {}", path.display(), e))?;

    Ok(json!({
        "status": "ok",
        "plan_file": path.display().to_string(),
        "plan": plan,
    }))
}
