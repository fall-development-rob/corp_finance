use clap::Args;
use serde_json::Value;

use corp_finance_core::clo_analytics::coverage_tests::{self, CoverageTestInput};
use corp_finance_core::clo_analytics::reinvestment::{self, ReinvestmentInput};
use corp_finance_core::clo_analytics::scenario::{self, CloScenarioInput};
use corp_finance_core::clo_analytics::tranche_analytics::{self, TrancheAnalyticsInput};
use corp_finance_core::clo_analytics::waterfall::{self, WaterfallInput};

use crate::input;

#[derive(Args)]
pub struct CloWaterfallArgs {
    #[arg(long)]
    pub input: Option<String>,
}

#[derive(Args)]
pub struct CloCoverageArgs {
    #[arg(long)]
    pub input: Option<String>,
}

#[derive(Args)]
pub struct CloReinvestmentArgs {
    #[arg(long)]
    pub input: Option<String>,
}

#[derive(Args)]
pub struct CloTrancheArgs {
    #[arg(long)]
    pub input: Option<String>,
}

#[derive(Args)]
pub struct CloScenarioArgs {
    #[arg(long)]
    pub input: Option<String>,
}

pub fn run_clo_waterfall(args: CloWaterfallArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let input_data: WaterfallInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required".into());
    };
    let result = waterfall::calculate_waterfall(&input_data)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_clo_coverage(args: CloCoverageArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let input_data: CoverageTestInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required".into());
    };
    let result = coverage_tests::calculate_coverage_tests(&input_data)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_clo_reinvestment(
    args: CloReinvestmentArgs,
) -> Result<Value, Box<dyn std::error::Error>> {
    let input_data: ReinvestmentInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required".into());
    };
    let result = reinvestment::calculate_reinvestment(&input_data)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_clo_tranche(args: CloTrancheArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let input_data: TrancheAnalyticsInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required".into());
    };
    let result = tranche_analytics::calculate_tranche_analytics(&input_data)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_clo_scenario(args: CloScenarioArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let input_data: CloScenarioInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required".into());
    };
    let result = scenario::calculate_clo_scenario(&input_data)?;
    Ok(serde_json::to_value(result)?)
}
