use clap::Args;
use serde_json::Value;

use corp_finance_core::jurisdiction::fund_fees::{self, FundFeeInput};
use corp_finance_core::jurisdiction::gp_economics::{self, GpEconomicsInput};
use corp_finance_core::jurisdiction::investor_returns::{self, InvestorNetReturnsInput};
use corp_finance_core::jurisdiction::nav::{self, NavInput};
use corp_finance_core::jurisdiction::reconciliation::{self, ReconciliationInput};
use corp_finance_core::jurisdiction::ubti::{self, UbtiScreeningInput};
use corp_finance_core::jurisdiction::withholding_tax::{self, WhtInput};

use crate::input;

/// Arguments for fund fee modelling
#[derive(Args)]
pub struct FundFeesArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

pub fn run_fund_fees(args: FundFeesArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let fee_input: FundFeeInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for fund fees".into());
    };
    let result = fund_fees::calculate_fund_fees(&fee_input)?;
    Ok(serde_json::to_value(result)?)
}

/// Arguments for GAAP/IFRS reconciliation
#[derive(Args)]
pub struct GaapIfrsArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

pub fn run_gaap_ifrs(args: GaapIfrsArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let reconciliation_input: ReconciliationInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for GAAP/IFRS reconciliation".into());
    };
    let result = reconciliation::reconcile_accounting_standards(&reconciliation_input)?;
    Ok(serde_json::to_value(result)?)
}

/// Arguments for withholding tax calculation
#[derive(Args)]
pub struct WhtArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

pub fn run_wht(args: WhtArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let wht_input: WhtInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for withholding tax".into());
    };
    let result = withholding_tax::calculate_withholding_tax(&wht_input)?;
    Ok(serde_json::to_value(result)?)
}

/// Arguments for NAV calculation
#[derive(Args)]
pub struct NavArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

pub fn run_nav(args: NavArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let nav_input: NavInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for NAV calculation".into());
    };
    let result = nav::calculate_nav(&nav_input)?;
    Ok(serde_json::to_value(result)?)
}

/// Arguments for GP economics modelling
#[derive(Args)]
pub struct GpEconomicsArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

pub fn run_gp_economics(args: GpEconomicsArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let gp_input: GpEconomicsInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for GP economics".into());
    };
    let result = gp_economics::calculate_gp_economics(&gp_input)?;
    Ok(serde_json::to_value(result)?)
}

/// Arguments for investor net returns calculation
#[derive(Args)]
pub struct InvestorNetReturnsArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

pub fn run_investor_net_returns(
    args: InvestorNetReturnsArgs,
) -> Result<Value, Box<dyn std::error::Error>> {
    let investor_input: InvestorNetReturnsInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for investor net returns".into());
    };
    let result = investor_returns::calculate_investor_net_returns(&investor_input)?;
    Ok(serde_json::to_value(result)?)
}

/// Arguments for UBTI/ECI screening
#[derive(Args)]
pub struct UbtiScreeningArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

pub fn run_ubti_screening(args: UbtiScreeningArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let ubti_input: UbtiScreeningInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for UBTI/ECI screening".into());
    };
    let result = ubti::screen_ubti_eci(&ubti_input)?;
    Ok(serde_json::to_value(result)?)
}
