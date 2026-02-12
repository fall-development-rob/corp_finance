use clap::Args;
use serde_json::Value;

use corp_finance_core::repo_financing::collateral_management::{self, CollateralInput};
use corp_finance_core::repo_financing::repo_rates::{self, RepoAnalyticsInput};

use crate::input;

#[derive(Args)]
pub struct RepoAnalyticsArgs {
    #[arg(long)]
    pub input: Option<String>,
}

#[derive(Args)]
pub struct CollateralArgs {
    #[arg(long)]
    pub input: Option<String>,
}

pub fn run_repo_analytics(args: RepoAnalyticsArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let repo_input: RepoAnalyticsInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for repo analytics".into());
    };
    let result = repo_rates::analyze_repo(&repo_input)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_collateral_analytics(args: CollateralArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let coll_input: CollateralInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for collateral analytics".into());
    };
    let result = collateral_management::analyze_collateral(&coll_input)?;
    Ok(serde_json::to_value(result)?)
}
