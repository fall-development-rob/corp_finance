use clap::Args;
use serde_json::Value;

use corp_finance_core::sovereign::country_risk::{self, CountryRiskInput};
use corp_finance_core::sovereign::sovereign_bonds::{self, SovereignBondInput};

use crate::input;

/// Arguments for sovereign bond analysis
#[derive(Args)]
pub struct SovereignBondArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

/// Arguments for country risk assessment
#[derive(Args)]
pub struct CountryRiskArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

pub fn run_sovereign_bond(args: SovereignBondArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let bond_input: SovereignBondInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for sovereign bond analysis".into());
    };
    let result = sovereign_bonds::analyze_sovereign_bond(&bond_input)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_country_risk(args: CountryRiskArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let risk_input: CountryRiskInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for country risk assessment".into());
    };
    let result = country_risk::assess_country_risk(&risk_input)?;
    Ok(serde_json::to_value(result)?)
}
