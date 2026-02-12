use clap::Args;
use serde_json::Value;

use corp_finance_core::fund_of_funds::commitment_pacing::{self, CommitmentPacingInput};
use corp_finance_core::fund_of_funds::j_curve::{self, JCurveInput};
use corp_finance_core::fund_of_funds::manager_selection::{self, ManagerSelectionInput};
use corp_finance_core::fund_of_funds::portfolio_construction::{self, FofPortfolioInput};
use corp_finance_core::fund_of_funds::secondaries::{self, SecondariesPricingInput};

use crate::input;

#[derive(Args)]
pub struct JCurveArgs {
    #[arg(long)]
    pub input: Option<String>,
}

#[derive(Args)]
pub struct CommitmentPacingArgs {
    #[arg(long)]
    pub input: Option<String>,
}

#[derive(Args)]
pub struct ManagerSelectionArgs {
    #[arg(long)]
    pub input: Option<String>,
}

#[derive(Args)]
pub struct SecondariesPricingArgs {
    #[arg(long)]
    pub input: Option<String>,
}

#[derive(Args)]
pub struct FofPortfolioArgs {
    #[arg(long)]
    pub input: Option<String>,
}

pub fn run_j_curve(args: JCurveArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let input_data: JCurveInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required".into());
    };
    let result = j_curve::calculate_j_curve(&input_data)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_commitment_pacing(
    args: CommitmentPacingArgs,
) -> Result<Value, Box<dyn std::error::Error>> {
    let input_data: CommitmentPacingInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required".into());
    };
    let result = commitment_pacing::calculate_commitment_pacing(&input_data)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_manager_selection(
    args: ManagerSelectionArgs,
) -> Result<Value, Box<dyn std::error::Error>> {
    let input_data: ManagerSelectionInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required".into());
    };
    let result = manager_selection::analyze_manager_selection(&input_data)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_secondaries_pricing(
    args: SecondariesPricingArgs,
) -> Result<Value, Box<dyn std::error::Error>> {
    let input_data: SecondariesPricingInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required".into());
    };
    let result = secondaries::calculate_secondaries_pricing(&input_data)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_fof_portfolio(args: FofPortfolioArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let input_data: FofPortfolioInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required".into());
    };
    let result = portfolio_construction::analyze_fof_portfolio(&input_data)?;
    Ok(serde_json::to_value(result)?)
}
