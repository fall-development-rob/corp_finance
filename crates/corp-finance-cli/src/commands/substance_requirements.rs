use clap::Args;
use serde_json::Value;

use corp_finance_core::substance_requirements::economic_substance::{self, EconomicSubstanceInput};
use corp_finance_core::substance_requirements::jurisdiction_tests::{self, JurisdictionTestInput};

use crate::input;

/// Arguments for economic substance analysis
#[derive(Args)]
pub struct EconomicSubstanceArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

/// Arguments for jurisdiction substance test
#[derive(Args)]
pub struct JurisdictionSubstanceTestArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

pub fn run_economic_substance(
    args: EconomicSubstanceArgs,
) -> Result<Value, Box<dyn std::error::Error>> {
    let substance_input: EconomicSubstanceInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for economic substance analysis".into());
    };
    let result = economic_substance::analyze_economic_substance(&substance_input)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_jurisdiction_substance_test(
    args: JurisdictionSubstanceTestArgs,
) -> Result<Value, Box<dyn std::error::Error>> {
    let test_input: JurisdictionTestInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for jurisdiction substance test".into());
    };
    let result = jurisdiction_tests::run_jurisdiction_substance_test(&test_input)?;
    Ok(serde_json::to_value(result)?)
}
