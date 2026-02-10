use clap::Args;
use serde_json::Value;

use corp_finance_core::wealth::retirement::{self, RetirementInput};
use corp_finance_core::wealth::tax_estate::{self, EstatePlanInput, TlhInput};

use crate::input;

/// Arguments for retirement planning
#[derive(Args)]
pub struct RetirementArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

/// Arguments for tax-loss harvesting simulation
#[derive(Args)]
pub struct TlhArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

/// Arguments for estate planning
#[derive(Args)]
pub struct EstatePlanArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

pub fn run_retirement(args: RetirementArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let ret_input: RetirementInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for retirement planning".into());
    };
    let result = retirement::plan_retirement(&ret_input)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_tlh(args: TlhArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let tlh_input: TlhInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for TLH simulation".into());
    };
    let result = tax_estate::simulate_tax_loss_harvesting(&tlh_input)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_estate_plan(args: EstatePlanArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let ep_input: EstatePlanInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for estate planning".into());
    };
    let result = tax_estate::plan_estate(&ep_input)?;
    Ok(serde_json::to_value(result)?)
}
