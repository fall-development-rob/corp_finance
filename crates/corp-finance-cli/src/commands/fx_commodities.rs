use clap::Args;
use serde_json::Value;

use corp_finance_core::fx_commodities::commodities::{
    self, CommodityCurveInput, CommodityForwardInput,
};
use corp_finance_core::fx_commodities::fx::{self, CrossRateInput, FxForwardInput};

use crate::input;

/// Arguments for FX forward pricing
#[derive(Args)]
pub struct FxForwardArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

/// Arguments for cross rate calculation
#[derive(Args)]
pub struct CrossRateArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

/// Arguments for commodity forward pricing
#[derive(Args)]
pub struct CommodityForwardArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

/// Arguments for commodity term structure analysis
#[derive(Args)]
pub struct CommodityCurveArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

pub fn run_fx_forward(args: FxForwardArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let fx_input: FxForwardInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for FX forward pricing".into());
    };
    let result = fx::price_fx_forward(&fx_input)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_cross_rate(args: CrossRateArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let cr_input: CrossRateInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for cross rate calculation".into());
    };
    let result = fx::calculate_cross_rate(&cr_input)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_commodity_forward(
    args: CommodityForwardArgs,
) -> Result<Value, Box<dyn std::error::Error>> {
    let cf_input: CommodityForwardInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for commodity forward pricing".into());
    };
    let result = commodities::price_commodity_forward(&cf_input)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_commodity_curve(args: CommodityCurveArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let cc_input: CommodityCurveInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for commodity curve analysis".into());
    };
    let result = commodities::analyze_commodity_curve(&cc_input)?;
    Ok(serde_json::to_value(result)?)
}
