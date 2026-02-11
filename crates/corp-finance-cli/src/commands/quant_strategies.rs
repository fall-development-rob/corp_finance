use clap::Args;
use serde_json::Value;

use corp_finance_core::quant_strategies::momentum::{self, MomentumInput};
use corp_finance_core::quant_strategies::pairs_trading::{self, PairsTradingInput};

use crate::input;

/// Arguments for pairs trading analysis
#[derive(Args)]
pub struct PairsTradingArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

/// Arguments for momentum factor analysis
#[derive(Args)]
pub struct MomentumArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

pub fn run_pairs_trading(args: PairsTradingArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let pairs_input: PairsTradingInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for pairs trading analysis".into());
    };
    let result = pairs_trading::analyze_pairs_trading(&pairs_input)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_momentum(args: MomentumArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let momentum_input: MomentumInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for momentum analysis".into());
    };
    let result = momentum::analyze_momentum(&momentum_input)?;
    Ok(serde_json::to_value(result)?)
}
