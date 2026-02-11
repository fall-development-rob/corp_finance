use clap::Args;
use serde_json::Value;

use corp_finance_core::trade_finance::letter_of_credit::{self, LetterOfCreditInput};
use corp_finance_core::trade_finance::supply_chain::{self, SupplyChainFinanceInput};

use crate::input;

/// Arguments for letter of credit pricing
#[derive(Args)]
pub struct LetterOfCreditArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

/// Arguments for supply chain finance analysis
#[derive(Args)]
pub struct SupplyChainFinanceArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

pub fn run_letter_of_credit(args: LetterOfCreditArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let lc_input: LetterOfCreditInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for letter of credit pricing".into());
    };
    let result = letter_of_credit::price_letter_of_credit(&lc_input)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_supply_chain_finance(
    args: SupplyChainFinanceArgs,
) -> Result<Value, Box<dyn std::error::Error>> {
    let scf_input: SupplyChainFinanceInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err(
            "--input <file.json> or stdin required for supply chain finance analysis".into(),
        );
    };
    let result = supply_chain::analyze_supply_chain_finance(&scf_input)?;
    Ok(serde_json::to_value(result)?)
}
