use clap::Args;
use serde_json::Value;

use corp_finance_core::ma::merger_model::{self, MergerInput};

use crate::input;

/// Arguments for merger accretion/dilution analysis
#[derive(Args)]
pub struct MergerArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

pub fn run_merger(args: MergerArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let merger_input: MergerInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for merger model".into());
    };
    let result = merger_model::analyze_merger(&merger_input)?;
    Ok(serde_json::to_value(result)?)
}
