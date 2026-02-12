use clap::Args;
use serde_json::Value;

use corp_finance_core::carbon_markets::carbon_pricing::{self, CarbonPricingInput};
use corp_finance_core::carbon_markets::cbam::{self, CbamInput};
use corp_finance_core::carbon_markets::ets_compliance::{self, EtsComplianceInput};
use corp_finance_core::carbon_markets::offset_valuation::{self, OffsetValuationInput};
use corp_finance_core::carbon_markets::shadow_carbon::{self, ShadowCarbonInput};

use crate::input;

#[derive(Args)]
pub struct CarbonPricingArgs {
    #[arg(long)]
    pub input: Option<String>,
}

#[derive(Args)]
pub struct EtsComplianceArgs {
    #[arg(long)]
    pub input: Option<String>,
}

#[derive(Args)]
pub struct CbamArgs {
    #[arg(long)]
    pub input: Option<String>,
}

#[derive(Args)]
pub struct OffsetValuationArgs {
    #[arg(long)]
    pub input: Option<String>,
}

#[derive(Args)]
pub struct ShadowCarbonArgs {
    #[arg(long)]
    pub input: Option<String>,
}

pub fn run_carbon_pricing(args: CarbonPricingArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let input_data: CarbonPricingInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required".into());
    };
    let result = carbon_pricing::calculate_carbon_pricing(&input_data)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_ets_compliance(args: EtsComplianceArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let input_data: EtsComplianceInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required".into());
    };
    let result = ets_compliance::calculate_ets_compliance(&input_data)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_cbam(args: CbamArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let input_data: CbamInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required".into());
    };
    let result = cbam::calculate_cbam(&input_data)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_offset_valuation(
    args: OffsetValuationArgs,
) -> Result<Value, Box<dyn std::error::Error>> {
    let input_data: OffsetValuationInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required".into());
    };
    let result = offset_valuation::calculate_offset_valuation(&input_data)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_shadow_carbon(args: ShadowCarbonArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let input_data: ShadowCarbonInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required".into());
    };
    let result = shadow_carbon::calculate_shadow_carbon(&input_data)?;
    Ok(serde_json::to_value(result)?)
}
