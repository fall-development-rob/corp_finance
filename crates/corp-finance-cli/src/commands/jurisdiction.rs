use clap::Args;
use serde_json::Value;

use corp_finance_core::jurisdiction::fund_fees::{self, FundFeeInput};

use crate::input;

/// Arguments for fund fee modelling
#[derive(Args)]
pub struct FundFeesArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

pub fn run_fund_fees(args: FundFeesArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let fee_input: FundFeeInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for fund fees".into());
    };
    let result = fund_fees::calculate_fund_fees(&fee_input)?;
    Ok(serde_json::to_value(result)?)
}
