use clap::Args;
use serde_json::Value;

use corp_finance_core::market_microstructure::optimal_execution::{self, OptimalExecutionInput};
use corp_finance_core::market_microstructure::spread_analysis::{self, SpreadAnalysisInput};

use crate::input;

#[derive(Args)]
pub struct SpreadAnalysisArgs {
    #[arg(long)]
    pub input: Option<String>,
}

#[derive(Args)]
pub struct OptimalExecutionArgs {
    #[arg(long)]
    pub input: Option<String>,
}

pub fn run_spread_analysis(args: SpreadAnalysisArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let sa_input: SpreadAnalysisInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for spread analysis".into());
    };
    let result = spread_analysis::analyze_spreads(&sa_input)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_optimal_execution(
    args: OptimalExecutionArgs,
) -> Result<Value, Box<dyn std::error::Error>> {
    let oe_input: OptimalExecutionInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for optimal execution".into());
    };
    let result = optimal_execution::optimize_execution(&oe_input)?;
    Ok(serde_json::to_value(result)?)
}
