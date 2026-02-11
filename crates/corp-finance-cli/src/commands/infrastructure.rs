use clap::Args;
use serde_json::Value;

use corp_finance_core::infrastructure::concession::{self, ConcessionInput};
use corp_finance_core::infrastructure::ppp_model::{self, PppModelInput};

use crate::input;

/// Arguments for PPP/PFI project financial model
#[derive(Args)]
pub struct PppModelArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

/// Arguments for concession valuation and analysis
#[derive(Args)]
pub struct ConcessionArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

pub fn run_ppp_model(args: PppModelArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let ppp_input: PppModelInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for PPP model analysis".into());
    };
    let result = ppp_model::model_ppp(&ppp_input)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_concession(args: ConcessionArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let concession_input: ConcessionInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for concession valuation".into());
    };
    let result = concession::value_concession(&concession_input)?;
    Ok(serde_json::to_value(result)?)
}
