use clap::Args;
use serde_json::Value;

use corp_finance_core::credit_portfolio::migration::{self, MigrationInput};
use corp_finance_core::credit_portfolio::portfolio_risk::{self, PortfolioRiskInput};

use crate::input;

/// Arguments for portfolio credit risk analysis
#[derive(Args)]
pub struct PortfolioCreditRiskArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

/// Arguments for rating migration analysis
#[derive(Args)]
pub struct MigrationArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

pub fn run_portfolio_credit_risk(
    args: PortfolioCreditRiskArgs,
) -> Result<Value, Box<dyn std::error::Error>> {
    let risk_input: PortfolioRiskInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for portfolio credit risk".into());
    };
    let result = portfolio_risk::calculate_portfolio_risk(&risk_input)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_migration(args: MigrationArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let mig_input: MigrationInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for migration analysis".into());
    };
    let result = migration::calculate_migration(&mig_input)?;
    Ok(serde_json::to_value(result)?)
}
