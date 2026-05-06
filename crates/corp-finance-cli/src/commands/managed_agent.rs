//! CLI subcommands for managed-agent deployment tooling.
//!
//! Three subcommands exposed under `cfa managed-agent`:
//! - `validate <slug>` — schema-validate a cookbook directory
//! - `deploy <slug>`   — assemble POST-ready JSON payload (prints to stdout)
//! - `orchestrate <slug> [--allowlist ...]` — stdin event-loop router
//!
//! Each is a thin wrapper calling `corp_finance_core::managed_agent::*`.
//! Follows the pattern in `crates/corp-finance-cli/src/commands/workflows.rs`.

use clap::{Args, Subcommand};
use serde_json::Value;
use std::collections::HashMap;
use std::io::{self, BufRead};

use corp_finance_core::managed_agent::deploy;
use corp_finance_core::managed_agent::orchestrate;
use corp_finance_core::managed_agent::types::{
    CheckAllInput, DeployInput, OrchestrateInput, ValidateInput,
};
use corp_finance_core::managed_agent::validate;

/// Top-level arg group for the `managed-agent` subcommand group.
#[derive(Args)]
pub struct ManagedAgentArgs {
    #[command(subcommand)]
    pub command: ManagedAgentCommands,
}

#[derive(Subcommand)]
pub enum ManagedAgentCommands {
    /// Validate a managed-agent cookbook (no network)
    Validate(ValidateArgs),
    /// Validate every cookbook found under `--cookbooks-root` (no network)
    CheckAll(CheckAllArgs),
    /// Assemble the deployment JSON payload (no network — pipe to curl to deploy)
    Deploy(DeployArgs),
    /// Stdin event-loop router: validates handoff_request events and prints routing decisions
    Orchestrate(OrchestrateArgs),
}

// ---------------------------------------------------------------------------
// ValidateArgs
// ---------------------------------------------------------------------------

#[derive(Args)]
pub struct ValidateArgs {
    /// Agent slug to validate (e.g. equity-analyst)
    pub slug: String,

    /// Path to the managed-agent-cookbooks directory
    #[arg(long, default_value = "managed-agent-cookbooks")]
    pub cookbooks_root: String,

    /// Path to the skills directory
    #[arg(long, default_value = ".claude/skills")]
    pub skills_root: String,

    /// Path to the agents directory
    #[arg(long, default_value = ".claude/agents/cfa")]
    pub agents_root: String,
}

// ---------------------------------------------------------------------------
// CheckAllArgs
// ---------------------------------------------------------------------------

#[derive(Args)]
pub struct CheckAllArgs {
    /// Path to the managed-agent-cookbooks directory
    #[arg(long, default_value = "managed-agent-cookbooks")]
    pub cookbooks_root: String,

    /// Path to the skills directory
    #[arg(long, default_value = ".claude/skills")]
    pub skills_root: String,

    /// Path to the agents directory
    #[arg(long, default_value = ".claude/agents/cfa")]
    pub agents_root: String,
}

// ---------------------------------------------------------------------------
// DeployArgs
// ---------------------------------------------------------------------------

#[derive(Args)]
pub struct DeployArgs {
    /// Agent slug to assemble payload for (e.g. equity-analyst)
    pub slug: String,

    /// Path to the managed-agent-cookbooks directory
    #[arg(long, default_value = "managed-agent-cookbooks")]
    pub cookbooks_root: String,

    /// Path to the skills directory
    #[arg(long, default_value = ".claude/skills")]
    pub skills_root: String,

    /// Path to the agents directory
    #[arg(long, default_value = ".claude/agents/cfa")]
    pub agents_root: String,
}

// ---------------------------------------------------------------------------
// OrchestrateArgs
// ---------------------------------------------------------------------------

#[derive(Args)]
pub struct OrchestrateArgs {
    /// Agent slug (used as the expected orchestrator identity)
    pub slug: String,

    /// Comma-separated list of allowed target slugs.
    /// Defaults to all known slugs when omitted.
    #[arg(long)]
    pub allowlist: Option<String>,
}

// ---------------------------------------------------------------------------
// Run functions
// ---------------------------------------------------------------------------

/// Resolve a relative path arg to an absolute path using the current working dir.
fn abs_path(path: &str) -> String {
    let p = std::path::Path::new(path);
    if p.is_absolute() {
        path.to_string()
    } else {
        std::env::current_dir()
            .map(|cwd| cwd.join(p).display().to_string())
            .unwrap_or_else(|_| path.to_string())
    }
}

pub fn run_check_all(args: CheckAllArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let input = CheckAllInput {
        cookbooks_root: abs_path(&args.cookbooks_root),
        skills_root: abs_path(&args.skills_root),
        agents_root: abs_path(&args.agents_root),
    };
    let result = validate::validate_all(&input)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_validate(args: ValidateArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let input = ValidateInput {
        slug: args.slug,
        cookbooks_root: abs_path(&args.cookbooks_root),
        skills_root: abs_path(&args.skills_root),
        agents_root: abs_path(&args.agents_root),
    };
    let result = validate::validate_manifest(&input)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_deploy(args: DeployArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let env_vars: HashMap<String, String> = std::env::vars().collect();
    let input = DeployInput {
        slug: args.slug,
        cookbooks_root: abs_path(&args.cookbooks_root),
        skills_root: abs_path(&args.skills_root),
        agents_root: abs_path(&args.agents_root),
        env_vars,
    };
    let result = deploy::build_deploy_payload(&input)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_orchestrate(args: OrchestrateArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let allowlist: Option<Vec<String>> = args
        .allowlist
        .as_deref()
        .map(|s| s.split(',').map(|p| p.trim().to_string()).collect());

    let stdin = io::stdin();
    let mut outputs: Vec<Value> = Vec::new();

    for line in stdin.lock().lines() {
        let line = line?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let event: corp_finance_core::managed_agent::types::OrchestrateEvent =
            serde_json::from_str(trimmed)
                .map_err(|e| format!("Failed to parse event JSON: {}", e))?;

        let input = OrchestrateInput {
            event,
            allowlist: allowlist.clone(),
        };
        let result = orchestrate::route_event(&input)?;
        outputs.push(serde_json::to_value(result)?);
    }

    Ok(Value::Array(outputs))
}
