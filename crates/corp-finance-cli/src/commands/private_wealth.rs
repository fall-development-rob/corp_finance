use clap::Args;
use serde_json::Value;

use corp_finance_core::private_wealth::concentrated_stock::{self, ConcentratedStockInput};
use corp_finance_core::private_wealth::direct_indexing::{self, DirectIndexingInput};
use corp_finance_core::private_wealth::family_governance::{self, FamilyGovernanceInput};
use corp_finance_core::private_wealth::philanthropic_vehicles::{self, PhilanthropicInput};
use corp_finance_core::private_wealth::wealth_transfer::{self, WealthTransferInput};

use crate::input;

#[derive(Args)]
pub struct ConcentratedStockArgs {
    #[arg(long)]
    pub input: Option<String>,
}

#[derive(Args)]
pub struct PhilanthropicVehiclesArgs {
    #[arg(long)]
    pub input: Option<String>,
}

#[derive(Args)]
pub struct WealthTransferArgs {
    #[arg(long)]
    pub input: Option<String>,
}

#[derive(Args)]
pub struct DirectIndexingArgs {
    #[arg(long)]
    pub input: Option<String>,
}

#[derive(Args)]
pub struct FamilyGovernanceArgs {
    #[arg(long)]
    pub input: Option<String>,
}

pub fn run_concentrated_stock(
    args: ConcentratedStockArgs,
) -> Result<Value, Box<dyn std::error::Error>> {
    let input_data: ConcentratedStockInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required".into());
    };
    let result = concentrated_stock::analyze_concentrated_stock(&input_data)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_philanthropic_vehicles(
    args: PhilanthropicVehiclesArgs,
) -> Result<Value, Box<dyn std::error::Error>> {
    let input_data: PhilanthropicInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required".into());
    };
    let result = philanthropic_vehicles::compare_philanthropic_vehicles(&input_data)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_wealth_transfer(args: WealthTransferArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let input_data: WealthTransferInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required".into());
    };
    let result = wealth_transfer::analyze_wealth_transfer(&input_data)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_direct_indexing(args: DirectIndexingArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let input_data: DirectIndexingInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required".into());
    };
    let result = direct_indexing::analyze_direct_indexing(&input_data)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_family_governance(
    args: FamilyGovernanceArgs,
) -> Result<Value, Box<dyn std::error::Error>> {
    let input_data: FamilyGovernanceInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required".into());
    };
    let result = family_governance::evaluate_family_governance(&input_data)?;
    Ok(serde_json::to_value(result)?)
}
