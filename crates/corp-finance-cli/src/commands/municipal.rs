use clap::Args;
use serde_json::Value;

use corp_finance_core::municipal::analysis::{self, MuniAnalysisInput};
use corp_finance_core::municipal::bonds::{self, MuniBondInput};

use crate::input;

/// Arguments for municipal bond pricing
#[derive(Args)]
pub struct MuniBondArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

/// Arguments for municipal credit analysis
#[derive(Args)]
pub struct MuniAnalysisArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

pub fn run_muni_bond(args: MuniBondArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let mb_input: MuniBondInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for muni bond pricing".into());
    };
    let result = bonds::price_muni_bond(&mb_input)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_muni_analysis(args: MuniAnalysisArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let ma_input: MuniAnalysisInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for municipal analysis".into());
    };
    let result = analysis::analyze_municipal(&ma_input)?;
    Ok(serde_json::to_value(result)?)
}
