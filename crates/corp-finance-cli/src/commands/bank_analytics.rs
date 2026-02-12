use clap::Args;
use serde_json::Value;

use corp_finance_core::bank_analytics::camels::{self, CamelsInput};
use corp_finance_core::bank_analytics::cecl_provisioning::{self, CeclProvisioningInput};
use corp_finance_core::bank_analytics::deposit_beta::{self, DepositBetaInput};
use corp_finance_core::bank_analytics::loan_book::{self, LoanBookInput};
use corp_finance_core::bank_analytics::nim_analysis::{self, NimAnalysisInput};

use crate::input;

#[derive(Args)]
pub struct NimAnalysisArgs {
    #[arg(long)]
    pub input: Option<String>,
}

#[derive(Args)]
pub struct CamelsRatingArgs {
    #[arg(long)]
    pub input: Option<String>,
}

#[derive(Args)]
pub struct CeclProvisioningArgs {
    #[arg(long)]
    pub input: Option<String>,
}

#[derive(Args)]
pub struct DepositBetaArgs {
    #[arg(long)]
    pub input: Option<String>,
}

#[derive(Args)]
pub struct LoanBookArgs {
    #[arg(long)]
    pub input: Option<String>,
}

pub fn run_nim_analysis(args: NimAnalysisArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let input_data: NimAnalysisInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required".into());
    };
    let result = nim_analysis::analyze_nim(&input_data)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_camels_rating(args: CamelsRatingArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let input_data: CamelsInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required".into());
    };
    let result = camels::calculate_camels(&input_data)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_cecl_provisioning(
    args: CeclProvisioningArgs,
) -> Result<Value, Box<dyn std::error::Error>> {
    let input_data: CeclProvisioningInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required".into());
    };
    let result = cecl_provisioning::calculate_cecl(&input_data)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_deposit_beta(args: DepositBetaArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let input_data: DepositBetaInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required".into());
    };
    let result = deposit_beta::analyze_deposit_beta(&input_data)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_loan_book(args: LoanBookArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let input_data: LoanBookInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required".into());
    };
    let result = loan_book::analyze_loan_book(&input_data)?;
    Ok(serde_json::to_value(result)?)
}
