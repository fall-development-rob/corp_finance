use clap::Args;
use serde_json::Value;

use corp_finance_core::regulatory_reporting::aifmd_reporting::{self, AifmdReportingInput};
use corp_finance_core::regulatory_reporting::sec_cftc_reporting::{self, SecCftcReportingInput};

use crate::input;

/// Arguments for AIFMD reporting
#[derive(Args)]
pub struct AifmdReportingArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

/// Arguments for SEC/CFTC reporting
#[derive(Args)]
pub struct SecCftcReportingArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

pub fn run_aifmd_reporting(args: AifmdReportingArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let aifmd_input: AifmdReportingInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for AIFMD reporting".into());
    };
    let result = aifmd_reporting::generate_aifmd_report(&aifmd_input)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_sec_cftc_reporting(
    args: SecCftcReportingArgs,
) -> Result<Value, Box<dyn std::error::Error>> {
    let sec_input: SecCftcReportingInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for SEC/CFTC reporting".into());
    };
    let result = sec_cftc_reporting::generate_sec_cftc_report(&sec_input)?;
    Ok(serde_json::to_value(result)?)
}
