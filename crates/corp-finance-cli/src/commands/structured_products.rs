use clap::Args;
use serde_json::Value;

use corp_finance_core::structured_products::exotic::{self, ExoticProductInput};
use corp_finance_core::structured_products::notes::{self, StructuredNoteInput};

use crate::input;

/// Arguments for structured note pricing
#[derive(Args)]
pub struct StructuredNoteArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

/// Arguments for exotic product pricing
#[derive(Args)]
pub struct ExoticProductArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

pub fn run_structured_note(args: StructuredNoteArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let sn_input: StructuredNoteInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for structured note pricing".into());
    };
    let result = notes::price_structured_note(&sn_input)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_exotic_product(args: ExoticProductArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let ep_input: ExoticProductInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for exotic product pricing".into());
    };
    let result = exotic::price_exotic(&ep_input)?;
    Ok(serde_json::to_value(result)?)
}
