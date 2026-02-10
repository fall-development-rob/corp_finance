use clap::Args;
use serde_json::Value;

use corp_finance_core::real_assets::project_finance::{self, ProjectFinanceInput};
use corp_finance_core::real_assets::real_estate::{self, PropertyValuationInput};

use crate::input;

/// Arguments for property valuation
#[derive(Args)]
pub struct PropertyValuationArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

/// Arguments for project finance modelling
#[derive(Args)]
pub struct ProjectFinanceArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

pub fn run_property_valuation(
    args: PropertyValuationArgs,
) -> Result<Value, Box<dyn std::error::Error>> {
    let prop_input: PropertyValuationInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for property valuation".into());
    };
    let result = real_estate::value_property(&prop_input)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_project_finance(args: ProjectFinanceArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let pf_input: ProjectFinanceInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for project finance model".into());
    };
    let result = project_finance::model_project_finance(&pf_input)?;
    Ok(serde_json::to_value(result)?)
}
