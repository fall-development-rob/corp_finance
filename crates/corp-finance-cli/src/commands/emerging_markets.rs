use clap::Args;
use serde_json::Value;

use corp_finance_core::emerging_markets::capital_controls::{self, CapitalControlsInput};
use corp_finance_core::emerging_markets::country_risk_premium::{self, CountryRiskPremiumInput};
use corp_finance_core::emerging_markets::em_bond_analysis::{self, EmBondAnalysisInput};
use corp_finance_core::emerging_markets::em_equity_premium::{self, EmEquityPremiumInput};
use corp_finance_core::emerging_markets::political_risk::{self, PoliticalRiskInput};

use crate::input;

#[derive(Args)]
pub struct CountryRiskPremiumArgs {
    #[arg(long)]
    pub input: Option<String>,
}

#[derive(Args)]
pub struct PoliticalRiskArgs {
    #[arg(long)]
    pub input: Option<String>,
}

#[derive(Args)]
pub struct CapitalControlsArgs {
    #[arg(long)]
    pub input: Option<String>,
}

#[derive(Args)]
pub struct EmBondAnalysisArgs {
    #[arg(long)]
    pub input: Option<String>,
}

#[derive(Args)]
pub struct EmEquityPremiumArgs {
    #[arg(long)]
    pub input: Option<String>,
}

pub fn run_country_risk_premium(
    args: CountryRiskPremiumArgs,
) -> Result<Value, Box<dyn std::error::Error>> {
    let input_data: CountryRiskPremiumInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required".into());
    };
    let result = country_risk_premium::calculate_country_risk_premium(&input_data)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_political_risk(args: PoliticalRiskArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let input_data: PoliticalRiskInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required".into());
    };
    let result = political_risk::assess_political_risk(&input_data)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_capital_controls(
    args: CapitalControlsArgs,
) -> Result<Value, Box<dyn std::error::Error>> {
    let input_data: CapitalControlsInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required".into());
    };
    let result = capital_controls::analyse_capital_controls(&input_data)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_em_bond_analysis(args: EmBondAnalysisArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let input_data: EmBondAnalysisInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required".into());
    };
    let result = em_bond_analysis::analyse_em_bonds(&input_data)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_em_equity_premium(
    args: EmEquityPremiumArgs,
) -> Result<Value, Box<dyn std::error::Error>> {
    let input_data: EmEquityPremiumInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required".into());
    };
    let result = em_equity_premium::calculate_em_equity_premium(&input_data)?;
    Ok(serde_json::to_value(result)?)
}
