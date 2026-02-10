use clap::Args;
use serde_json::Value;

use corp_finance_core::monte_carlo::simulation::{self, McDcfInput, MonteCarloInput};

use crate::input;

/// Arguments for generic Monte Carlo simulation
#[derive(Args)]
pub struct MonteCarloArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

/// Arguments for Monte Carlo DCF valuation
#[derive(Args)]
pub struct McDcfArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

pub fn run_monte_carlo(args: MonteCarloArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let mc_input: MonteCarloInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for Monte Carlo simulation".into());
    };
    let result = simulation::run_monte_carlo_simulation(&mc_input)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_mc_dcf(args: McDcfArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let mc_input: McDcfInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for Monte Carlo DCF".into());
    };
    let result = simulation::run_monte_carlo_dcf(&mc_input)?;
    Ok(serde_json::to_value(result)?)
}
