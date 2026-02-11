use clap::Args;
use serde_json::Value;

use corp_finance_core::onshore_structures::uk_eu_funds::{self, UkEuFundInput};
use corp_finance_core::onshore_structures::us_funds::{self, UsFundInput};

use crate::input;

/// Arguments for US onshore fund structure analysis
#[derive(Args)]
pub struct UsFundArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

/// Arguments for UK/EU onshore fund structure analysis
#[derive(Args)]
pub struct UkEuFundArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

pub fn run_us_fund(args: UsFundArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let fund_input: UsFundInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for US fund analysis".into());
    };
    let result = us_funds::analyze_us_fund_structure(&fund_input)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_uk_eu_fund(args: UkEuFundArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let fund_input: UkEuFundInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for UK/EU fund analysis".into());
    };
    let result = uk_eu_funds::analyze_uk_eu_fund(&fund_input)?;
    Ok(serde_json::to_value(result)?)
}
