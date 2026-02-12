use clap::Args;
use serde_json::Value;

use corp_finance_core::earnings_quality::accrual_quality::{self, AccrualQualityInput};
use corp_finance_core::earnings_quality::beneish::{self, BeneishInput};
use corp_finance_core::earnings_quality::composite::{self, EarningsQualityCompositeInput};
use corp_finance_core::earnings_quality::piotroski::{self, PiotroskiInput};
use corp_finance_core::earnings_quality::revenue_quality::{self, RevenueQualityInput};

use crate::input;

#[derive(Args)]
pub struct BeneishArgs {
    #[arg(long)]
    pub input: Option<String>,
}

#[derive(Args)]
pub struct PiotroskiArgs {
    #[arg(long)]
    pub input: Option<String>,
}

#[derive(Args)]
pub struct AccrualQualityArgs {
    #[arg(long)]
    pub input: Option<String>,
}

#[derive(Args)]
pub struct RevenueQualityArgs {
    #[arg(long)]
    pub input: Option<String>,
}

#[derive(Args)]
pub struct EarningsQualityCompositeArgs {
    #[arg(long)]
    pub input: Option<String>,
}

pub fn run_beneish(args: BeneishArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let input_data: BeneishInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required".into());
    };
    let result = beneish::calculate_beneish_m_score(&input_data)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_piotroski(args: PiotroskiArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let input_data: PiotroskiInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required".into());
    };
    let result = piotroski::calculate_piotroski_f_score(&input_data)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_accrual_quality(args: AccrualQualityArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let input_data: AccrualQualityInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required".into());
    };
    let result = accrual_quality::calculate_accrual_quality(&input_data)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_revenue_quality(args: RevenueQualityArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let input_data: RevenueQualityInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required".into());
    };
    let result = revenue_quality::calculate_revenue_quality(&input_data)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_earnings_quality_composite(
    args: EarningsQualityCompositeArgs,
) -> Result<Value, Box<dyn std::error::Error>> {
    let input_data: EarningsQualityCompositeInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required".into());
    };
    let result = composite::calculate_earnings_quality_composite(&input_data)?;
    Ok(serde_json::to_value(result)?)
}
