use clap::Args;
use serde_json::Value;

use corp_finance_core::regulatory::alm::{self, AlmInput};
use corp_finance_core::regulatory::capital::{self, RegulatoryCapitalInput};
use corp_finance_core::regulatory::liquidity::{self, LcrInput, NsfrInput};

use crate::input;

/// Arguments for regulatory capital calculation
#[derive(Args)]
pub struct RegulatoryCapitalArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

/// Arguments for LCR calculation
#[derive(Args)]
pub struct LcrArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

/// Arguments for NSFR calculation
#[derive(Args)]
pub struct NsfrArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

/// Arguments for ALM / IRRBB analysis
#[derive(Args)]
pub struct AlmArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

pub fn run_regulatory_capital(
    args: RegulatoryCapitalArgs,
) -> Result<Value, Box<dyn std::error::Error>> {
    let rc_input: RegulatoryCapitalInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for regulatory capital".into());
    };
    let result = capital::calculate_regulatory_capital(&rc_input)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_lcr(args: LcrArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let lcr_input: LcrInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for LCR calculation".into());
    };
    let result = liquidity::calculate_lcr(&lcr_input)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_nsfr(args: NsfrArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let nsfr_input: NsfrInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for NSFR calculation".into());
    };
    let result = liquidity::calculate_nsfr(&nsfr_input)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_alm(args: AlmArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let alm_input: AlmInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for ALM analysis".into());
    };
    let result = alm::analyze_alm(&alm_input)?;
    Ok(serde_json::to_value(result)?)
}
