use clap::Args;
use serde_json::Value;

use corp_finance_core::inflation_linked::inflation_derivatives::{self, InflationDerivativeInput};
use corp_finance_core::inflation_linked::tips_pricing::{self, TipsAnalyticsInput};

use crate::input;

#[derive(Args)]
pub struct TipsAnalyticsArgs {
    #[arg(long)]
    pub input: Option<String>,
}

#[derive(Args)]
pub struct InflationDerivativeArgs {
    #[arg(long)]
    pub input: Option<String>,
}

pub fn run_tips_analytics(args: TipsAnalyticsArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let tips_input: TipsAnalyticsInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for TIPS analytics".into());
    };
    let result = tips_pricing::analyze_tips(&tips_input)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_inflation_derivatives(
    args: InflationDerivativeArgs,
) -> Result<Value, Box<dyn std::error::Error>> {
    let id_input: InflationDerivativeInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for inflation derivatives".into());
    };
    let result = inflation_derivatives::analyze_inflation_derivatives(&id_input)?;
    Ok(serde_json::to_value(result)?)
}
