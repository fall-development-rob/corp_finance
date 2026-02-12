use clap::Args;
use serde_json::Value;

use corp_finance_core::risk_budgeting::factor_risk_budget::{self, FactorRiskBudgetInput};
use corp_finance_core::risk_budgeting::tail_risk::{self, TailRiskInput};

use crate::input;

#[derive(Args)]
pub struct FactorRiskBudgetArgs {
    #[arg(long)]
    pub input: Option<String>,
}

#[derive(Args)]
pub struct TailRiskArgs {
    #[arg(long)]
    pub input: Option<String>,
}

pub fn run_factor_risk_budget(
    args: FactorRiskBudgetArgs,
) -> Result<Value, Box<dyn std::error::Error>> {
    let frb_input: FactorRiskBudgetInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for factor risk budget".into());
    };
    let result = factor_risk_budget::analyze_factor_risk_budget(&frb_input)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_tail_risk(args: TailRiskArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let tr_input: TailRiskInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for tail risk analysis".into());
    };
    let result = tail_risk::analyze_tail_risk(&tr_input)?;
    Ok(serde_json::to_value(result)?)
}
