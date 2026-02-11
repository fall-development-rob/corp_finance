use clap::Args;
use serde_json::Value;

use corp_finance_core::macro_economics::international::{self, InternationalInput};
use corp_finance_core::macro_economics::monetary_policy::{self, MonetaryPolicyInput};

use crate::input;

/// Arguments for monetary policy analysis
#[derive(Args)]
pub struct MonetaryPolicyArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

/// Arguments for international economics analysis
#[derive(Args)]
pub struct InternationalArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

pub fn run_monetary_policy(args: MonetaryPolicyArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let mp_input: MonetaryPolicyInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for monetary policy analysis".into());
    };
    let result = monetary_policy::analyze_monetary_policy(&mp_input)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_international(args: InternationalArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let intl_input: InternationalInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for international economics".into());
    };
    let result = international::analyze_international(&intl_input)?;
    Ok(serde_json::to_value(result)?)
}
