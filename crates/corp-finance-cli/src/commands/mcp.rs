//! CLI subcommand for the MCP-server tier registry.
//!
//! Single subcommand exposed under `cfa mcp`:
//! - `list [--tier=<tier>]` — enumerate MCP servers grouped by cost tier
//!
//! Tier strings accepted: `free-native`, `free-public-with-api-key`,
//! `freemium`, `paid-vendor` (snake_case variants also accepted).
//!
//! Pattern mirrors `crates/corp-finance-cli/src/commands/managed_agent.rs`.

use clap::{Args, Subcommand};
use serde_json::Value;

use corp_finance_core::mcp_servers::list;
use corp_finance_core::mcp_servers::types::{McpListInput, McpServerTier};

/// Top-level arg group for the `mcp` subcommand group.
#[derive(Args)]
pub struct McpArgs {
    #[command(subcommand)]
    pub command: McpCommands,
}

#[derive(Subcommand)]
pub enum McpCommands {
    /// List MCP servers grouped by cost tier (no I/O — registry-only)
    List(McpListArgs),
}

// ---------------------------------------------------------------------------
// McpListArgs
// ---------------------------------------------------------------------------

#[derive(Args)]
pub struct McpListArgs {
    /// Filter by cost tier: `free-native`, `free-public-with-api-key`, `freemium`, or `paid-vendor`.
    /// Omit to show all servers.
    #[arg(long)]
    pub tier: Option<String>,
}

// ---------------------------------------------------------------------------
// Run functions
// ---------------------------------------------------------------------------

pub fn run_list(args: McpListArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let tier = match args.tier.as_deref() {
        None => None,
        Some("free-native") | Some("free_native") => Some(McpServerTier::FreeNative),
        Some("free-public-with-api-key") | Some("free_public_with_api_key") => {
            Some(McpServerTier::FreePublicWithApiKey)
        }
        Some("freemium") => Some(McpServerTier::Freemium),
        Some("paid-vendor") | Some("paid_vendor") => Some(McpServerTier::PaidVendor),
        Some(other) => {
            return Err(format!(
                "unknown tier '{}'; expected free-native | free-public-with-api-key | freemium | paid-vendor",
                other
            )
            .into())
        }
    };
    let result = list::list_servers(&McpListInput { tier })?;
    Ok(serde_json::to_value(result)?)
}
