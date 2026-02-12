use clap::Args;
use serde_json::Value;

use corp_finance_core::mortgage_analytics::mbs_analytics::{self, MbsAnalyticsInput};
use corp_finance_core::mortgage_analytics::prepayment::{self, PrepaymentInput};

use crate::input;

#[derive(Args)]
pub struct PrepaymentArgs {
    #[arg(long)]
    pub input: Option<String>,
}

#[derive(Args)]
pub struct MbsAnalyticsArgs {
    #[arg(long)]
    pub input: Option<String>,
}

pub fn run_prepayment(args: PrepaymentArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let pp_input: PrepaymentInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for prepayment analysis".into());
    };
    let result = prepayment::analyze_prepayment(&pp_input)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_mbs_analytics(args: MbsAnalyticsArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let mbs_input: MbsAnalyticsInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for MBS analytics".into());
    };
    let result = mbs_analytics::analyze_mbs(&mbs_input)?;
    Ok(serde_json::to_value(result)?)
}
