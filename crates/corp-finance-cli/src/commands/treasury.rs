use clap::Args;
use serde_json::Value;

use corp_finance_core::treasury::cash_management::{self, CashManagementInput};
use corp_finance_core::treasury::hedging::{self, HedgingInput};

use crate::input;

/// Arguments for corporate cash management analysis
#[derive(Args)]
pub struct CashManagementArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

/// Arguments for hedge effectiveness analysis
#[derive(Args)]
pub struct HedgingArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

pub fn run_cash_management(args: CashManagementArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let cash_input: CashManagementInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for cash management analysis".into());
    };
    let result = cash_management::analyze_cash_management(&cash_input)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_hedging(args: HedgingArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let hedging_input: HedgingInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err(
            "--input <file.json> or stdin required for hedge effectiveness analysis".into(),
        );
    };
    let result = hedging::analyze_hedging(&hedging_input)?;
    Ok(serde_json::to_value(result)?)
}
