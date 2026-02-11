use clap::Args;
use serde_json::Value;

use corp_finance_core::pension::funding::{self, PensionFundingInput};
use corp_finance_core::pension::ldi::{self, LdiInput};

use crate::input;

/// Arguments for pension funding analysis
#[derive(Args)]
pub struct PensionFundingArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

/// Arguments for LDI strategy
#[derive(Args)]
pub struct LdiStrategyArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

pub fn run_pension_funding(args: PensionFundingArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let pf_input: PensionFundingInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for pension funding analysis".into());
    };
    let result = funding::analyze_pension_funding(&pf_input)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_ldi_strategy(args: LdiStrategyArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let ldi_input: LdiInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for LDI strategy".into());
    };
    let result = ldi::design_ldi_strategy(&ldi_input)?;
    Ok(serde_json::to_value(result)?)
}
