use clap::Args;
use serde_json::Value;

use corp_finance_core::real_options::decision_tree::{self, DecisionTreeInput};
use corp_finance_core::real_options::valuation::{self, RealOptionInput};

use crate::input;

/// Arguments for real option valuation
#[derive(Args)]
pub struct RealOptionArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

/// Arguments for decision tree analysis
#[derive(Args)]
pub struct DecisionTreeArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

pub fn run_real_option(args: RealOptionArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let option_input: RealOptionInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for real option valuation".into());
    };
    let result = valuation::value_real_option(&option_input)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_decision_tree(args: DecisionTreeArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let tree_input: DecisionTreeInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for decision tree analysis".into());
    };
    let result = decision_tree::analyze_decision_tree(&tree_input)?;
    Ok(serde_json::to_value(result)?)
}
