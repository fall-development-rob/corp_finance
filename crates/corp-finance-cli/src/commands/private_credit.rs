use clap::Args;
use serde_json::Value;

use corp_finance_core::private_credit::direct_lending::{self, DirectLoanInput, SyndicationInput};
use corp_finance_core::private_credit::unitranche::{self, UnitrancheInput};

use crate::input;

/// Arguments for unitranche pricing
#[derive(Args)]
pub struct UnitrancheArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

/// Arguments for direct loan modelling
#[derive(Args)]
pub struct DirectLoanArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

/// Arguments for syndication analysis
#[derive(Args)]
pub struct SyndicationArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

pub fn run_unitranche(args: UnitrancheArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let ut_input: UnitrancheInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for unitranche pricing".into());
    };
    let result = unitranche::price_unitranche(&ut_input)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_direct_loan(args: DirectLoanArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let dl_input: DirectLoanInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for direct loan model".into());
    };
    let result = direct_lending::model_direct_loan(&dl_input)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_syndication(args: SyndicationArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let synd_input: SyndicationInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for syndication analysis".into());
    };
    let result = direct_lending::analyze_syndication(&synd_input)?;
    Ok(serde_json::to_value(result)?)
}
