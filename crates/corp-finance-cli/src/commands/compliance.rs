use clap::Args;
use serde_json::Value;

use corp_finance_core::compliance::best_execution::{self, BestExecutionInput};
use corp_finance_core::compliance::reporting::{self, GipsInput};

use crate::input;

/// Arguments for best execution analysis
#[derive(Args)]
pub struct BestExecutionArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

/// Arguments for GIPS performance reporting
#[derive(Args)]
pub struct GipsReportArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

pub fn run_best_execution(args: BestExecutionArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let be_input: BestExecutionInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for best execution analysis".into());
    };
    let result = best_execution::analyze_best_execution(&be_input)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_gips_report(args: GipsReportArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let gips_input: GipsInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for GIPS reporting".into());
    };
    let result = reporting::generate_gips_report(&gips_input)?;
    Ok(serde_json::to_value(result)?)
}
