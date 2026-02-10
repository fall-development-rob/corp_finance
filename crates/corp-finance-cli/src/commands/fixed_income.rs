use clap::Args;
use serde_json::Value;

use corp_finance_core::fixed_income::bonds::{self, BondPricingInput};
use corp_finance_core::fixed_income::duration::{self, DurationInput};
use corp_finance_core::fixed_income::spreads::{self, CreditSpreadInput};
use corp_finance_core::fixed_income::yields::{
    self, BondYieldInput, BootstrapInput, NelsonSiegelInput,
};

use crate::input;

/// Arguments for bond pricing
#[derive(Args)]
pub struct BondPricingArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

pub fn run_bond_pricing(args: BondPricingArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let pricing_input: BondPricingInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for bond pricing".into());
    };
    let result = bonds::price_bond(&pricing_input)?;
    Ok(serde_json::to_value(result)?)
}

/// Arguments for bond yield calculation
#[derive(Args)]
pub struct BondYieldArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

pub fn run_bond_yield(args: BondYieldArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let yield_input: BondYieldInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for bond yield".into());
    };
    let result = yields::calculate_bond_yield(&yield_input)?;
    Ok(serde_json::to_value(result)?)
}

/// Arguments for spot curve bootstrapping
#[derive(Args)]
pub struct BootstrapArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

pub fn run_bootstrap(args: BootstrapArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let bootstrap_input: BootstrapInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for bootstrap".into());
    };
    let result = yields::bootstrap_spot_curve(&bootstrap_input)?;
    Ok(serde_json::to_value(result)?)
}

/// Arguments for Nelson-Siegel yield curve fitting
#[derive(Args)]
pub struct NelsonSiegelArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

pub fn run_nelson_siegel(args: NelsonSiegelArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let ns_input: NelsonSiegelInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for Nelson-Siegel".into());
    };
    let result = yields::fit_nelson_siegel(&ns_input)?;
    Ok(serde_json::to_value(result)?)
}

/// Arguments for duration & convexity
#[derive(Args)]
pub struct DurationArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

pub fn run_duration(args: DurationArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let dur_input: DurationInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for duration".into());
    };
    let result = duration::calculate_duration(&dur_input)?;
    Ok(serde_json::to_value(result)?)
}

/// Arguments for credit spread analysis
#[derive(Args)]
pub struct CreditSpreadArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

pub fn run_credit_spreads(args: CreditSpreadArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let spread_input: CreditSpreadInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for credit spreads".into());
    };
    let result = spreads::calculate_credit_spreads(&spread_input)?;
    Ok(serde_json::to_value(result)?)
}
