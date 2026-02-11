use clap::Args;
use serde_json::Value;

use corp_finance_core::commodity_trading::spreads::{self, CommoditySpreadInput};
use corp_finance_core::commodity_trading::storage::{self, StorageEconomicsInput};

use crate::input;

/// Arguments for commodity spread analysis
#[derive(Args)]
pub struct CommoditySpreadArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

/// Arguments for storage economics analysis
#[derive(Args)]
pub struct StorageEconomicsArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

pub fn run_commodity_spread(
    args: CommoditySpreadArgs,
) -> Result<Value, Box<dyn std::error::Error>> {
    let spread_input: CommoditySpreadInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for commodity spread analysis".into());
    };
    let result = spreads::analyze_commodity_spread(&spread_input)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_storage_economics(
    args: StorageEconomicsArgs,
) -> Result<Value, Box<dyn std::error::Error>> {
    let storage_input: StorageEconomicsInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for storage economics analysis".into());
    };
    let result = storage::analyze_storage_economics(&storage_input)?;
    Ok(serde_json::to_value(result)?)
}
