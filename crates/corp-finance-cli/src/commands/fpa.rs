use clap::Args;
use serde_json::Value;

use corp_finance_core::fpa::variance::{self, BreakevenInput, VarianceInput};
use corp_finance_core::fpa::working_capital::{self, RollingForecastInput, WorkingCapitalInput};

use crate::input;

/// Arguments for budget-vs-actual variance analysis
#[derive(Args)]
pub struct VarianceArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

/// Arguments for break-even analysis
#[derive(Args)]
pub struct BreakevenArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

/// Arguments for working capital analysis
#[derive(Args)]
pub struct WorkingCapitalArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

/// Arguments for rolling forecast
#[derive(Args)]
pub struct RollingForecastArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

pub fn run_variance(args: VarianceArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let var_input: VarianceInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for variance analysis".into());
    };
    let result = variance::analyze_variance(&var_input)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_breakeven(args: BreakevenArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let be_input: BreakevenInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for break-even analysis".into());
    };
    let result = variance::analyze_breakeven(&be_input)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_working_capital(args: WorkingCapitalArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let wc_input: WorkingCapitalInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for working capital analysis".into());
    };
    let result = working_capital::analyze_working_capital(&wc_input)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_rolling_forecast(
    args: RollingForecastArgs,
) -> Result<Value, Box<dyn std::error::Error>> {
    let rf_input: RollingForecastInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for rolling forecast".into());
    };
    let result = working_capital::build_rolling_forecast(&rf_input)?;
    Ok(serde_json::to_value(result)?)
}
