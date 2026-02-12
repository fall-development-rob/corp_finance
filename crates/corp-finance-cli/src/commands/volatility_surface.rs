use clap::Args;
use serde_json::Value;

use corp_finance_core::volatility_surface::implied_vol_surface::{self, ImpliedVolSurfaceInput};
use corp_finance_core::volatility_surface::sabr_model::{self, SabrCalibrationInput};

use crate::input;

#[derive(Args)]
pub struct ImpliedVolSurfaceArgs {
    #[arg(long)]
    pub input: Option<String>,
}

#[derive(Args)]
pub struct SabrCalibrationArgs {
    #[arg(long)]
    pub input: Option<String>,
}

pub fn run_implied_vol_surface(
    args: ImpliedVolSurfaceArgs,
) -> Result<Value, Box<dyn std::error::Error>> {
    let iv_input: ImpliedVolSurfaceInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for implied vol surface".into());
    };
    let result = implied_vol_surface::build_implied_vol_surface(&iv_input)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_sabr_calibration(
    args: SabrCalibrationArgs,
) -> Result<Value, Box<dyn std::error::Error>> {
    let sabr_input: SabrCalibrationInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for SABR calibration".into());
    };
    let result = sabr_model::calibrate_sabr(&sabr_input)?;
    Ok(serde_json::to_value(result)?)
}
