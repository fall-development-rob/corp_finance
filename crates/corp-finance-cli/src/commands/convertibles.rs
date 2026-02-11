use clap::Args;
use serde_json::Value;

use corp_finance_core::convertibles::analysis::{self, ConvertibleAnalysisInput};
use corp_finance_core::convertibles::pricing::{self, ConvertibleBondInput};

use crate::input;

/// Arguments for convertible bond pricing
#[derive(Args)]
pub struct ConvertiblePricingArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

/// Arguments for convertible bond analysis
#[derive(Args)]
pub struct ConvertibleAnalysisArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

pub fn run_convertible_pricing(
    args: ConvertiblePricingArgs,
) -> Result<Value, Box<dyn std::error::Error>> {
    let cb_input: ConvertibleBondInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for convertible pricing".into());
    };
    let result = pricing::price_convertible(&cb_input)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_convertible_analysis(
    args: ConvertibleAnalysisArgs,
) -> Result<Value, Box<dyn std::error::Error>> {
    let ca_input: ConvertibleAnalysisInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for convertible analysis".into());
    };
    let result = analysis::analyze_convertible(&ca_input)?;
    Ok(serde_json::to_value(result)?)
}
