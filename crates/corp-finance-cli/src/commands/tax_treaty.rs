use clap::Args;
use serde_json::Value;

use corp_finance_core::tax_treaty::optimization::{self, TreatyOptInput};
use corp_finance_core::tax_treaty::treaty_network::{self, TreatyNetworkInput};

use crate::input;

/// Arguments for tax treaty network analysis
#[derive(Args)]
pub struct TreatyNetworkArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

/// Arguments for multi-jurisdiction holding structure optimization
#[derive(Args)]
pub struct TreatyOptArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

pub fn run_treaty_network(args: TreatyNetworkArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let treaty_input: TreatyNetworkInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for treaty network analysis".into());
    };
    let result = treaty_network::analyze_treaty_network(&treaty_input)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_treaty_optimization(args: TreatyOptArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let opt_input: TreatyOptInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err(
            "--input <file.json> or stdin required for treaty structure optimization".into(),
        );
    };
    let result = optimization::optimize_treaty_structure(&opt_input)?;
    Ok(serde_json::to_value(result)?)
}
