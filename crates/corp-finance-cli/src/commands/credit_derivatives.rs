use clap::Args;
use serde_json::Value;

use corp_finance_core::credit_derivatives::cds::{self, CdsInput};
use corp_finance_core::credit_derivatives::cva::{self, CvaInput};

use crate::input;

/// Arguments for CDS pricing
#[derive(Args)]
pub struct CdsArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

/// Arguments for CVA calculation
#[derive(Args)]
pub struct CvaArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

pub fn run_cds_pricing(args: CdsArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let cds_input: CdsInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for CDS pricing".into());
    };
    let result = cds::price_cds(&cds_input)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_cva_calculation(args: CvaArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let cva_input: CvaInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for CVA calculation".into());
    };
    let result = cva::calculate_cva(&cva_input)?;
    Ok(serde_json::to_value(result)?)
}
