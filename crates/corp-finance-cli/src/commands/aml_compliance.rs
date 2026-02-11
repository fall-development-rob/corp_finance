use clap::Args;
use serde_json::Value;

use corp_finance_core::aml_compliance::kyc_scoring::{self, KycRiskInput};
use corp_finance_core::aml_compliance::sanctions_screening::{self, SanctionsScreeningInput};

use crate::input;

/// Arguments for KYC risk assessment
#[derive(Args)]
pub struct KycRiskArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

/// Arguments for sanctions screening
#[derive(Args)]
pub struct SanctionsScreeningArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

pub fn run_kyc_risk(args: KycRiskArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let kyc_input: KycRiskInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for KYC risk assessment".into());
    };
    let result = kyc_scoring::assess_kyc_risk(&kyc_input)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_sanctions_screening(
    args: SanctionsScreeningArgs,
) -> Result<Value, Box<dyn std::error::Error>> {
    let screening_input: SanctionsScreeningInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for sanctions screening".into());
    };
    let result = sanctions_screening::screen_sanctions(&screening_input)?;
    Ok(serde_json::to_value(result)?)
}
