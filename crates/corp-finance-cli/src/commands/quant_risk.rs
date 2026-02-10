use clap::Args;
use serde_json::Value;

use corp_finance_core::quant_risk::black_litterman::{self, BlackLittermanInput};
use corp_finance_core::quant_risk::factor_models::{self, FactorModelInput};
use corp_finance_core::quant_risk::risk_parity::{self, RiskParityInput};
use corp_finance_core::quant_risk::stress_testing::{self, StressTestInput};

use crate::input;

/// Arguments for factor model regression
#[derive(Args)]
pub struct FactorModelArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

/// Arguments for Black-Litterman portfolio optimisation
#[derive(Args)]
pub struct BlackLittermanArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

/// Arguments for risk-parity portfolio construction
#[derive(Args)]
pub struct RiskParityArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

/// Arguments for portfolio stress testing
#[derive(Args)]
pub struct StressTestArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

pub fn run_factor_model(args: FactorModelArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let fm_input: FactorModelInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for factor model".into());
    };
    let result = factor_models::run_factor_model(&fm_input)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_black_litterman(args: BlackLittermanArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let bl_input: BlackLittermanInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for Black-Litterman model".into());
    };
    let result = black_litterman::run_black_litterman(&bl_input)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_risk_parity(args: RiskParityArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let rp_input: RiskParityInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for risk parity".into());
    };
    let result = risk_parity::calculate_risk_parity(&rp_input)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_stress_test(args: StressTestArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let st_input: StressTestInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for stress testing".into());
    };
    let result = stress_testing::run_stress_test(&st_input)?;
    Ok(serde_json::to_value(result)?)
}
