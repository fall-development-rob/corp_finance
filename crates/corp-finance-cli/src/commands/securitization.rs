use clap::Args;
use serde_json::Value;

use corp_finance_core::securitization::abs_mbs::{self, AbsMbsInput};
use corp_finance_core::securitization::tranching::{self, TranchingInput};

use crate::input;

/// Arguments for ABS/MBS cash flow modelling
#[derive(Args)]
pub struct AbsMbsArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

/// Arguments for CDO/CLO tranching analysis
#[derive(Args)]
pub struct TranchingArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

pub fn run_abs_mbs(args: AbsMbsArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let abs_input: AbsMbsInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for ABS/MBS modelling".into());
    };
    let result = abs_mbs::model_abs_cashflows(&abs_input)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_tranching(args: TranchingArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let tr_input: TranchingInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for tranching analysis".into());
    };
    let result = tranching::analyze_tranching(&tr_input)?;
    Ok(serde_json::to_value(result)?)
}
