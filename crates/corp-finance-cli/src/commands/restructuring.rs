use clap::Args;
use serde_json::Value;

use corp_finance_core::restructuring::distressed_debt::{self, DistressedDebtInput};
use corp_finance_core::restructuring::recovery::{self, RecoveryAnalysisInput};

use crate::input;

/// Arguments for restructuring recovery analysis
#[derive(Args)]
pub struct RecoveryArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

/// Arguments for distressed debt analysis
#[derive(Args)]
pub struct DistressedDebtArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

pub fn run_recovery(args: RecoveryArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let rec_input: RecoveryAnalysisInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for recovery analysis".into());
    };
    let result = recovery::analyze_recovery(&rec_input)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_distressed_debt(args: DistressedDebtArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let dd_input: DistressedDebtInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for distressed debt analysis".into());
    };
    let result = distressed_debt::analyze_distressed_debt(&dd_input)?;
    Ok(serde_json::to_value(result)?)
}
