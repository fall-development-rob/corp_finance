use clap::Args;
use serde_json::Value;

use corp_finance_core::equity_research::sotp::{self, SotpInput};
use corp_finance_core::equity_research::target_price::{self, TargetPriceInput};

use crate::input;

/// Arguments for sum-of-the-parts valuation
#[derive(Args)]
pub struct SotpArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

/// Arguments for target price calculation
#[derive(Args)]
pub struct TargetPriceArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

pub fn run_sotp(args: SotpArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let sotp_input: SotpInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for SOTP valuation".into());
    };
    let result = sotp::calculate_sotp(&sotp_input)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_target_price(args: TargetPriceArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let tp_input: TargetPriceInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for target price calculation".into());
    };
    let result = target_price::calculate_target_price(&tp_input)?;
    Ok(serde_json::to_value(result)?)
}
