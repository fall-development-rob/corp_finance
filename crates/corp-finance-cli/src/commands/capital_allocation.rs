use clap::Args;
use serde_json::Value;

use corp_finance_core::capital_allocation::economic_capital::{self, EconomicCapitalInput};
use corp_finance_core::capital_allocation::euler_allocation::{self, EulerAllocationInput};
use corp_finance_core::capital_allocation::limit_management::{self, LimitManagementInput};
use corp_finance_core::capital_allocation::raroc::{self, RarocInput};
use corp_finance_core::capital_allocation::shapley_allocation::{self, ShapleyAllocationInput};

use crate::input;

#[derive(Args)]
pub struct EconomicCapitalArgs {
    #[arg(long)]
    pub input: Option<String>,
}

#[derive(Args)]
pub struct RarocArgs {
    #[arg(long)]
    pub input: Option<String>,
}

#[derive(Args)]
pub struct EulerAllocationArgs {
    #[arg(long)]
    pub input: Option<String>,
}

#[derive(Args)]
pub struct ShapleyAllocationArgs {
    #[arg(long)]
    pub input: Option<String>,
}

#[derive(Args)]
pub struct LimitManagementArgs {
    #[arg(long)]
    pub input: Option<String>,
}

pub fn run_economic_capital(
    args: EconomicCapitalArgs,
) -> Result<Value, Box<dyn std::error::Error>> {
    let input_data: EconomicCapitalInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required".into());
    };
    let result = economic_capital::calculate_economic_capital(&input_data)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_raroc(args: RarocArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let input_data: RarocInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required".into());
    };
    let result = raroc::calculate_raroc(&input_data)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_euler_allocation(
    args: EulerAllocationArgs,
) -> Result<Value, Box<dyn std::error::Error>> {
    let input_data: EulerAllocationInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required".into());
    };
    let result = euler_allocation::calculate_euler_allocation(&input_data)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_shapley_allocation(
    args: ShapleyAllocationArgs,
) -> Result<Value, Box<dyn std::error::Error>> {
    let input_data: ShapleyAllocationInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required".into());
    };
    let result = shapley_allocation::calculate_shapley_allocation(&input_data)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_limit_management(
    args: LimitManagementArgs,
) -> Result<Value, Box<dyn std::error::Error>> {
    let input_data: LimitManagementInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required".into());
    };
    let result = limit_management::evaluate_limits(&input_data)?;
    Ok(serde_json::to_value(result)?)
}
