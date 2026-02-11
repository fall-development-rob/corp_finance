use clap::Args;
use serde_json::Value;

use corp_finance_core::crypto::defi::{self, DefiYieldInput};
use corp_finance_core::crypto::valuation::{self, TokenValuationInput};

use crate::input;

/// Arguments for token valuation
#[derive(Args)]
pub struct TokenValuationArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

/// Arguments for DeFi yield analysis
#[derive(Args)]
pub struct DefiAnalysisArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

pub fn run_token_valuation(args: TokenValuationArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let tv_input: TokenValuationInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for token valuation".into());
    };
    let result = valuation::value_token(&tv_input)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_defi_analysis(args: DefiAnalysisArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let defi_input: DefiYieldInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for DeFi analysis".into());
    };
    let result = defi::analyze_defi(&defi_input)?;
    Ok(serde_json::to_value(result)?)
}
