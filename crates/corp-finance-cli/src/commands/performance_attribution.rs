use clap::Args;
use serde_json::Value;

use corp_finance_core::performance_attribution::brinson::{self, BrinsonInput};
use corp_finance_core::performance_attribution::factor_attribution::{
    self, FactorAttributionInput,
};

use crate::input;

/// Arguments for Brinson-Fachler performance attribution
#[derive(Args)]
pub struct BrinsonArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

/// Arguments for factor-based return attribution
#[derive(Args)]
pub struct FactorAttributionArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

pub fn run_brinson(args: BrinsonArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let brinson_input: BrinsonInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for brinson attribution".into());
    };
    let result = brinson::brinson_attribution(&brinson_input)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_factor_attribution(
    args: FactorAttributionArgs,
) -> Result<Value, Box<dyn std::error::Error>> {
    let fa_input: FactorAttributionInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for factor attribution".into());
    };
    let result = factor_attribution::factor_attribution(&fa_input)?;
    Ok(serde_json::to_value(result)?)
}
