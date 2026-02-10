use clap::Args;
use serde_json::Value;

use corp_finance_core::three_statement::model::{self, ThreeStatementInput};

use crate::input;

/// Arguments for three-statement financial model
#[derive(Args)]
pub struct ThreeStatementArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

pub fn run_three_statement(args: ThreeStatementArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let ts_input: ThreeStatementInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for three-statement model".into());
    };
    let result = model::build_three_statement_model(&ts_input)?;
    Ok(serde_json::to_value(result)?)
}
