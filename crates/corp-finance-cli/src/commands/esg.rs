use clap::Args;
use serde_json::Value;

use corp_finance_core::esg::climate::{self, CarbonFootprintInput, GreenBondInput, SllInput};
use corp_finance_core::esg::scoring::{self, EsgScoreInput};

use crate::input;

/// Arguments for ESG scoring
#[derive(Args)]
pub struct EsgScoreArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

/// Arguments for carbon footprint analysis
#[derive(Args)]
pub struct CarbonFootprintArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

/// Arguments for green bond analysis
#[derive(Args)]
pub struct GreenBondArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

/// Arguments for sustainability-linked loan covenant testing
#[derive(Args)]
pub struct SllArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

pub fn run_esg_score(args: EsgScoreArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let esg_input: EsgScoreInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for ESG scoring".into());
    };
    let result = scoring::calculate_esg_score(&esg_input)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_carbon_footprint(
    args: CarbonFootprintArgs,
) -> Result<Value, Box<dyn std::error::Error>> {
    let cf_input: CarbonFootprintInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for carbon footprint".into());
    };
    let result = climate::analyze_carbon_footprint(&cf_input)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_green_bond(args: GreenBondArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let gb_input: GreenBondInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for green bond analysis".into());
    };
    let result = climate::analyze_green_bond(&gb_input)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_sll(args: SllArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let sll_input: SllInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for SLL covenant testing".into());
    };
    let result = climate::test_sll_covenants(&sll_input)?;
    Ok(serde_json::to_value(result)?)
}
