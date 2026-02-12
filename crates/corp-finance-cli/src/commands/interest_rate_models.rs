use clap::Args;
use serde_json::Value;

use corp_finance_core::interest_rate_models::short_rate::{self, ShortRateInput};
use corp_finance_core::interest_rate_models::term_structure::{self, TermStructureInput};

use crate::input;

#[derive(Args)]
pub struct ShortRateArgs {
    #[arg(long)]
    pub input: Option<String>,
}

#[derive(Args)]
pub struct TermStructureFitArgs {
    #[arg(long)]
    pub input: Option<String>,
}

pub fn run_short_rate(args: ShortRateArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let sr_input: ShortRateInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for short rate analysis".into());
    };
    let result = short_rate::analyze_short_rate(&sr_input)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_term_structure_fit(
    args: TermStructureFitArgs,
) -> Result<Value, Box<dyn std::error::Error>> {
    let ts_input: TermStructureInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for term structure fitting".into());
    };
    let result = term_structure::fit_term_structure(&ts_input)?;
    Ok(serde_json::to_value(result)?)
}
