use clap::Args;
use serde_json::Value;

use corp_finance_core::lease_accounting::classification::{self, LeaseInput};
use corp_finance_core::lease_accounting::sale_leaseback::{self, SaleLeasebackInput};

use crate::input;

/// Arguments for lease classification
#[derive(Args)]
pub struct LeaseClassificationArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

/// Arguments for sale-leaseback analysis
#[derive(Args)]
pub struct SaleLeasebackArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

pub fn run_lease_classification(
    args: LeaseClassificationArgs,
) -> Result<Value, Box<dyn std::error::Error>> {
    let lease_input: LeaseInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for lease classification".into());
    };
    let result = classification::classify_lease(&lease_input)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_sale_leaseback(args: SaleLeasebackArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let slb_input: SaleLeasebackInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for sale-leaseback analysis".into());
    };
    let result = sale_leaseback::analyze_sale_leaseback(&slb_input)?;
    Ok(serde_json::to_value(result)?)
}
