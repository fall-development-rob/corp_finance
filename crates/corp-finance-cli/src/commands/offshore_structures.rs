use clap::Args;
use serde_json::Value;

use corp_finance_core::offshore_structures::cayman::{self, CaymanFundInput};
use corp_finance_core::offshore_structures::luxembourg::{self, LuxFundInput};

use crate::input;

/// Arguments for Cayman/BVI offshore fund structure analysis
#[derive(Args)]
pub struct CaymanFundArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

/// Arguments for Luxembourg/Ireland fund structure analysis
#[derive(Args)]
pub struct LuxFundArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

pub fn run_cayman_fund(args: CaymanFundArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let fund_input: CaymanFundInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for Cayman fund analysis".into());
    };
    let result = cayman::analyze_cayman_structure(&fund_input)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_lux_fund(args: LuxFundArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let fund_input: LuxFundInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for Luxembourg fund analysis".into());
    };
    let result = luxembourg::analyze_lux_structure(&fund_input)?;
    Ok(serde_json::to_value(result)?)
}
