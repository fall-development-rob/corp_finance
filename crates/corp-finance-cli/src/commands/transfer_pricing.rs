use clap::Args;
use serde_json::Value;

use corp_finance_core::transfer_pricing::beps::{self, BepsInput};
use corp_finance_core::transfer_pricing::intercompany::{self, IntercompanyInput};

use crate::input;

/// Arguments for OECD BEPS compliance analysis
#[derive(Args)]
pub struct BepsArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

/// Arguments for intercompany transfer pricing analysis
#[derive(Args)]
pub struct IntercompanyArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

pub fn run_beps(args: BepsArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let beps_input: BepsInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for BEPS compliance analysis".into());
    };
    let result = beps::analyze_beps_compliance(&beps_input)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_intercompany(args: IntercompanyArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let ic_input: IntercompanyInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err(
            "--input <file.json> or stdin required for intercompany pricing analysis".into(),
        );
    };
    let result = intercompany::analyze_intercompany(&ic_input)?;
    Ok(serde_json::to_value(result)?)
}
