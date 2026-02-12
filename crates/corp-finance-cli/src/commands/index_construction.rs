use clap::Args;
use serde_json::Value;

use corp_finance_core::index_construction::rebalancing::{self, RebalancingInput};
use corp_finance_core::index_construction::reconstitution::{self, ReconstitutionInput};
use corp_finance_core::index_construction::smart_beta::{self, SmartBetaInput};
use corp_finance_core::index_construction::tracking_error::{self, TrackingErrorInput};
use corp_finance_core::index_construction::weighting::{self, WeightingInput};

use crate::input;

#[derive(Args)]
pub struct IndexWeightingArgs {
    #[arg(long)]
    pub input: Option<String>,
}

#[derive(Args)]
pub struct IndexRebalancingArgs {
    #[arg(long)]
    pub input: Option<String>,
}

#[derive(Args)]
pub struct TrackingErrorArgs {
    #[arg(long)]
    pub input: Option<String>,
}

#[derive(Args)]
pub struct SmartBetaArgs {
    #[arg(long)]
    pub input: Option<String>,
}

#[derive(Args)]
pub struct IndexReconstitutionArgs {
    #[arg(long)]
    pub input: Option<String>,
}

pub fn run_index_weighting(args: IndexWeightingArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let input_data: WeightingInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required".into());
    };
    let result = weighting::calculate_weighting(&input_data)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_index_rebalancing(
    args: IndexRebalancingArgs,
) -> Result<Value, Box<dyn std::error::Error>> {
    let input_data: RebalancingInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required".into());
    };
    let result = rebalancing::calculate_rebalancing(&input_data)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_tracking_error(args: TrackingErrorArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let input_data: TrackingErrorInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required".into());
    };
    let result = tracking_error::calculate_tracking_error(&input_data)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_smart_beta(args: SmartBetaArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let input_data: SmartBetaInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required".into());
    };
    let result = smart_beta::calculate_smart_beta(&input_data)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_index_reconstitution(
    args: IndexReconstitutionArgs,
) -> Result<Value, Box<dyn std::error::Error>> {
    let input_data: ReconstitutionInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required".into());
    };
    let result = reconstitution::calculate_reconstitution(&input_data)?;
    Ok(serde_json::to_value(result)?)
}
