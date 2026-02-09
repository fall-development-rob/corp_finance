use clap::Args;
use rust_decimal::Decimal;
use serde_json::Value;

use corp_finance_core::credit::metrics::{self, CreditMetricsInput};

use crate::input;

/// Arguments for credit metrics calculation
#[derive(Args)]
#[command(allow_hyphen_values = true)]
pub struct CreditArgs {
    /// Path to JSON input file (overrides individual flags)
    #[arg(long)]
    pub input: Option<String>,

    /// Revenue
    #[arg(long)]
    pub revenue: Option<Decimal>,

    /// EBITDA
    #[arg(long)]
    pub ebitda: Option<Decimal>,

    /// EBIT
    #[arg(long)]
    pub ebit: Option<Decimal>,

    /// Interest expense
    #[arg(long)]
    pub interest_expense: Option<Decimal>,

    /// Depreciation and amortisation
    #[arg(long, alias = "da")]
    pub depreciation_amortisation: Option<Decimal>,

    /// Total debt
    #[arg(long)]
    pub total_debt: Option<Decimal>,

    /// Cash and equivalents
    #[arg(long)]
    pub cash: Option<Decimal>,

    /// Total assets
    #[arg(long)]
    pub total_assets: Option<Decimal>,

    /// Current assets
    #[arg(long)]
    pub current_assets: Option<Decimal>,

    /// Current liabilities
    #[arg(long)]
    pub current_liabilities: Option<Decimal>,

    /// Total equity
    #[arg(long)]
    pub total_equity: Option<Decimal>,

    /// Retained earnings
    #[arg(long)]
    pub retained_earnings: Option<Decimal>,

    /// Working capital
    #[arg(long)]
    pub working_capital: Option<Decimal>,

    /// Operating cash flow
    #[arg(long, alias = "ocf")]
    pub operating_cash_flow: Option<Decimal>,

    /// Capital expenditure
    #[arg(long)]
    pub capex: Option<Decimal>,

    /// Funds from operations
    #[arg(long, alias = "ffo")]
    pub funds_from_operations: Option<Decimal>,

    /// Lease payments
    #[arg(long)]
    pub lease_payments: Option<Decimal>,

    /// Preferred dividends
    #[arg(long)]
    pub preferred_dividends: Option<Decimal>,

    /// Market capitalisation
    #[arg(long)]
    pub market_cap: Option<Decimal>,
}

/// Arguments for debt capacity estimation
#[derive(Args)]
pub struct DebtCapacityArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

/// Arguments for covenant compliance testing
#[derive(Args)]
pub struct CovenantArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

pub fn run_credit_metrics(args: CreditArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let credit_input: CreditMetricsInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        CreditMetricsInput {
            revenue: args.revenue
                .ok_or("--revenue is required (or provide --input)")?,
            ebitda: args.ebitda
                .ok_or("--ebitda is required (or provide --input)")?,
            ebit: args.ebit
                .ok_or("--ebit is required (or provide --input)")?,
            interest_expense: args.interest_expense
                .ok_or("--interest-expense is required (or provide --input)")?,
            depreciation_amortisation: args.depreciation_amortisation
                .ok_or("--depreciation-amortisation is required (or provide --input)")?,
            total_debt: args.total_debt
                .ok_or("--total-debt is required (or provide --input)")?,
            cash: args.cash
                .ok_or("--cash is required (or provide --input)")?,
            total_assets: args.total_assets
                .ok_or("--total-assets is required (or provide --input)")?,
            current_assets: args.current_assets
                .ok_or("--current-assets is required (or provide --input)")?,
            current_liabilities: args.current_liabilities
                .ok_or("--current-liabilities is required (or provide --input)")?,
            total_equity: args.total_equity
                .ok_or("--total-equity is required (or provide --input)")?,
            retained_earnings: args.retained_earnings
                .ok_or("--retained-earnings is required (or provide --input)")?,
            working_capital: args.working_capital
                .ok_or("--working-capital is required (or provide --input)")?,
            operating_cash_flow: args.operating_cash_flow
                .ok_or("--operating-cash-flow is required (or provide --input)")?,
            capex: args.capex
                .ok_or("--capex is required (or provide --input)")?,
            funds_from_operations: args.funds_from_operations,
            lease_payments: args.lease_payments,
            preferred_dividends: args.preferred_dividends,
            market_cap: args.market_cap,
        }
    };

    let result = metrics::calculate_credit_metrics(&credit_input)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_debt_capacity(args: DebtCapacityArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let input_data: Value = if let Some(ref path) = args.input {
        input::file::read_json_value(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        data
    } else {
        return Err("--input file is required for debt capacity analysis".into());
    };

    Err(format!(
        "Debt capacity model not yet available. Input received: {}",
        serde_json::to_string_pretty(&input_data)?
    ).into())
}

pub fn run_covenant_test(args: CovenantArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let input_data: Value = if let Some(ref path) = args.input {
        input::file::read_json_value(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        data
    } else {
        return Err("--input file is required for covenant testing".into());
    };

    Err(format!(
        "Covenant testing not yet available. Input received: {}",
        serde_json::to_string_pretty(&input_data)?
    ).into())
}
