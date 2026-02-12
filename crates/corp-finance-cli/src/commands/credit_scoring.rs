use clap::Args;
use serde_json::Value;

use corp_finance_core::credit_scoring::calibration::{self, CalibrationInput};
use corp_finance_core::credit_scoring::intensity_model::{self, IntensityModelInput};
use corp_finance_core::credit_scoring::scorecard::{self, ScorecardInput};
use corp_finance_core::credit_scoring::structural_model::{self, MertonInput};
use corp_finance_core::credit_scoring::validation::{self, ValidationInput};

use crate::input;

#[derive(Args)]
pub struct CreditScorecardArgs {
    #[arg(long)]
    pub input: Option<String>,
}

#[derive(Args)]
pub struct MertonPdArgs {
    #[arg(long)]
    pub input: Option<String>,
}

#[derive(Args)]
pub struct IntensityModelArgs {
    #[arg(long)]
    pub input: Option<String>,
}

#[derive(Args)]
pub struct PdCalibrationArgs {
    #[arg(long)]
    pub input: Option<String>,
}

#[derive(Args)]
pub struct ScoringValidationArgs {
    #[arg(long)]
    pub input: Option<String>,
}

pub fn run_credit_scorecard(
    args: CreditScorecardArgs,
) -> Result<Value, Box<dyn std::error::Error>> {
    let input_data: ScorecardInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required".into());
    };
    let result = scorecard::calculate_scorecard(&input_data)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_merton_pd(args: MertonPdArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let input_data: MertonInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required".into());
    };
    let result = structural_model::calculate_merton(&input_data)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_intensity_model(args: IntensityModelArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let input_data: IntensityModelInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required".into());
    };
    let result = intensity_model::calculate_intensity_model(&input_data)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_pd_calibration(args: PdCalibrationArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let input_data: CalibrationInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required".into());
    };
    let result = calibration::calculate_calibration(&input_data)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_scoring_validation(
    args: ScoringValidationArgs,
) -> Result<Value, Box<dyn std::error::Error>> {
    let input_data: ValidationInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required".into());
    };
    let result = validation::calculate_validation(&input_data)?;
    Ok(serde_json::to_value(result)?)
}
