use clap::Args;
use serde_json::Value;

use corp_finance_core::insurance::pricing::{
    self, CombinedRatioInput, PremiumPricingInput, ScrInput,
};
use corp_finance_core::insurance::reserving::{self, ReservingInput};

use crate::input;

/// Arguments for loss reserve estimation
#[derive(Args)]
pub struct ReservingArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

/// Arguments for premium pricing
#[derive(Args)]
pub struct PremiumPricingArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

/// Arguments for combined ratio analysis
#[derive(Args)]
pub struct CombinedRatioArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

/// Arguments for Solvency II SCR calculation
#[derive(Args)]
pub struct ScrArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

pub fn run_reserving(args: ReservingArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let res_input: ReservingInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for reserving".into());
    };
    let result = reserving::estimate_reserves(&res_input)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_premium_pricing(args: PremiumPricingArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let pp_input: PremiumPricingInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for premium pricing".into());
    };
    let result = pricing::price_premium(&pp_input)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_combined_ratio(args: CombinedRatioArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let cr_input: CombinedRatioInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for combined ratio analysis".into());
    };
    let result = pricing::analyze_combined_ratio(&cr_input)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_scr(args: ScrArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let scr_input: ScrInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for SCR calculation".into());
    };
    let result = pricing::calculate_scr(&scr_input)?;
    Ok(serde_json::to_value(result)?)
}
