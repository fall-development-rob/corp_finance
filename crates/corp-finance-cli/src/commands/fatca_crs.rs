use clap::Args;
use serde_json::Value;

use corp_finance_core::fatca_crs::classification::{self, EntityClassificationInput};
use corp_finance_core::fatca_crs::reporting::{self, FatcaCrsReportingInput};

use crate::input;

/// Arguments for FATCA/CRS reporting analysis
#[derive(Args)]
pub struct FatcaCrsReportingArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

/// Arguments for FATCA/CRS entity classification
#[derive(Args)]
pub struct EntityClassificationArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

pub fn run_fatca_crs_reporting(
    args: FatcaCrsReportingArgs,
) -> Result<Value, Box<dyn std::error::Error>> {
    let reporting_input: FatcaCrsReportingInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err(
            "--input <file.json> or stdin required for FATCA/CRS reporting analysis".into(),
        );
    };
    let result = reporting::analyze_fatca_crs_reporting(&reporting_input)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_entity_classification(
    args: EntityClassificationArgs,
) -> Result<Value, Box<dyn std::error::Error>> {
    let classification_input: EntityClassificationInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err(
            "--input <file.json> or stdin required for FATCA/CRS entity classification".into(),
        );
    };
    let result = classification::classify_entity(&classification_input)?;
    Ok(serde_json::to_value(result)?)
}
