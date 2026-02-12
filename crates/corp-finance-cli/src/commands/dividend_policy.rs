use clap::Args;
use serde_json::Value;

use corp_finance_core::dividend_policy::buyback::{self, BuybackInput};
use corp_finance_core::dividend_policy::h_model::{self, HModelInput};
use corp_finance_core::dividend_policy::multistage_ddm::{self, MultistageDdmInput};
use corp_finance_core::dividend_policy::payout_sustainability::{self, PayoutSustainabilityInput};
use corp_finance_core::dividend_policy::total_shareholder_return::{
    self, TotalShareholderReturnInput,
};

use crate::input;

#[derive(Args)]
pub struct HModelDdmArgs {
    #[arg(long)]
    pub input: Option<String>,
}

#[derive(Args)]
pub struct MultistageDdmArgs {
    #[arg(long)]
    pub input: Option<String>,
}

#[derive(Args)]
pub struct BuybackArgs {
    #[arg(long)]
    pub input: Option<String>,
}

#[derive(Args)]
pub struct PayoutSustainabilityArgs {
    #[arg(long)]
    pub input: Option<String>,
}

#[derive(Args)]
pub struct TotalShareholderReturnArgs {
    #[arg(long)]
    pub input: Option<String>,
}

pub fn run_h_model_ddm(args: HModelDdmArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let input_data: HModelInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required".into());
    };
    let result = h_model::calculate_h_model(&input_data)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_multistage_ddm(args: MultistageDdmArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let input_data: MultistageDdmInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required".into());
    };
    let result = multistage_ddm::calculate_multistage_ddm(&input_data)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_buyback(args: BuybackArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let input_data: BuybackInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required".into());
    };
    let result = buyback::calculate_buyback(&input_data)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_payout_sustainability(
    args: PayoutSustainabilityArgs,
) -> Result<Value, Box<dyn std::error::Error>> {
    let input_data: PayoutSustainabilityInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required".into());
    };
    let result = payout_sustainability::calculate_payout_sustainability(&input_data)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_total_shareholder_return(
    args: TotalShareholderReturnArgs,
) -> Result<Value, Box<dyn std::error::Error>> {
    let input_data: TotalShareholderReturnInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required".into());
    };
    let result = total_shareholder_return::calculate_total_shareholder_return(&input_data)?;
    Ok(serde_json::to_value(result)?)
}
