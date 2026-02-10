use clap::Args;
use serde_json::Value;

use corp_finance_core::derivatives::forwards::{
    self, BasisAnalysisInput, ForwardInput, ForwardPositionInput,
};
use corp_finance_core::derivatives::options::{self, ImpliedVolInput, OptionInput};
use corp_finance_core::derivatives::strategies::{self, StrategyInput};
use corp_finance_core::derivatives::swaps::{self, CurrencySwapInput, IrsInput};

use crate::input;

/// Arguments for option pricing
#[derive(Args)]
pub struct OptionPriceArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

pub fn run_option_price(args: OptionPriceArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let opt_input: OptionInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for option pricing".into());
    };
    let result = options::price_option(&opt_input)?;
    Ok(serde_json::to_value(result)?)
}

/// Arguments for implied volatility
#[derive(Args)]
pub struct ImpliedVolArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

pub fn run_implied_vol(args: ImpliedVolArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let iv_input: ImpliedVolInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for implied vol".into());
    };
    let result = options::implied_volatility(&iv_input)?;
    Ok(serde_json::to_value(result)?)
}

/// Arguments for forward pricing
#[derive(Args)]
pub struct ForwardPriceArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

pub fn run_forward_price(args: ForwardPriceArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let fwd_input: ForwardInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for forward pricing".into());
    };
    let result = forwards::price_forward(&fwd_input)?;
    Ok(serde_json::to_value(result)?)
}

/// Arguments for forward position valuation
#[derive(Args)]
pub struct ForwardPositionArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

pub fn run_forward_position(
    args: ForwardPositionArgs,
) -> Result<Value, Box<dyn std::error::Error>> {
    let pos_input: ForwardPositionInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for forward position".into());
    };
    let result = forwards::value_forward_position(&pos_input)?;
    Ok(serde_json::to_value(result)?)
}

/// Arguments for basis analysis
#[derive(Args)]
pub struct BasisAnalysisArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

pub fn run_basis_analysis(args: BasisAnalysisArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let basis_input: BasisAnalysisInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for basis analysis".into());
    };
    let result = forwards::futures_basis_analysis(&basis_input)?;
    Ok(serde_json::to_value(result)?)
}

/// Arguments for interest rate swap
#[derive(Args)]
pub struct IrsArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

pub fn run_irs(args: IrsArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let irs_input: IrsInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for IRS valuation".into());
    };
    let result = swaps::value_interest_rate_swap(&irs_input)?;
    Ok(serde_json::to_value(result)?)
}

/// Arguments for currency swap
#[derive(Args)]
pub struct CurrencySwapArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

pub fn run_currency_swap(args: CurrencySwapArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let cs_input: CurrencySwapInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for currency swap".into());
    };
    let result = swaps::value_currency_swap(&cs_input)?;
    Ok(serde_json::to_value(result)?)
}

/// Arguments for strategy analysis
#[derive(Args)]
pub struct StrategyArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

pub fn run_strategy(args: StrategyArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let strat_input: StrategyInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for strategy analysis".into());
    };
    let result = strategies::analyze_strategy(&strat_input)?;
    Ok(serde_json::to_value(result)?)
}
