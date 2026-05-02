use clap::Args;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde_json::Value;

use corp_finance_core::types::Currency;
use corp_finance_core::valuation::comps::{self, CompsInput};
use corp_finance_core::valuation::dcf::{self, DcfInput, TerminalMethod};
use corp_finance_core::valuation::wacc::{self, WaccInput};

use crate::input;

/// Arguments for WACC calculation
#[derive(Args)]
#[command(allow_hyphen_values = true)]
pub struct WaccArgs {
    /// Risk-free rate (e.g. 0.042 for 4.2%)
    #[arg(long)]
    pub risk_free_rate: Option<Decimal>,

    /// Equity risk premium (e.g. 0.055 for 5.5%)
    #[arg(long, alias = "erp")]
    pub equity_risk_premium: Option<Decimal>,

    /// Levered beta
    #[arg(long)]
    pub beta: Option<Decimal>,

    /// Pre-tax cost of debt
    #[arg(long)]
    pub cost_of_debt: Option<Decimal>,

    /// Marginal corporate tax rate
    #[arg(long)]
    pub tax_rate: Option<Decimal>,

    /// Debt weight in capital structure (market value basis)
    #[arg(long)]
    pub debt_weight: Option<Decimal>,

    /// Equity weight in capital structure (market value basis)
    #[arg(long)]
    pub equity_weight: Option<Decimal>,

    /// Size premium (Duff & Phelps)
    #[arg(long)]
    pub size_premium: Option<Decimal>,

    /// Country risk premium
    #[arg(long)]
    pub country_risk: Option<Decimal>,

    /// Company-specific risk premium
    #[arg(long)]
    pub specific_risk: Option<Decimal>,

    /// Unlevered (asset) beta for Hamada re-levering
    #[arg(long)]
    pub unlevered_beta: Option<Decimal>,

    /// Target debt-to-equity ratio for Hamada re-levering
    #[arg(long)]
    pub target_debt_equity: Option<Decimal>,

    /// Path to JSON input file (overrides individual flags)
    #[arg(long)]
    pub input: Option<String>,
}

/// Arguments for DCF valuation
#[derive(Args)]
pub struct DcfArgs {
    /// Path to JSON input file with DCF parameters
    #[arg(long)]
    pub input: Option<String>,

    /// Base revenue for projection
    #[arg(long)]
    pub base_revenue: Option<Decimal>,

    /// Revenue growth rate
    #[arg(long)]
    pub growth_rate: Option<Decimal>,

    /// EBITDA margin
    #[arg(long)]
    pub ebitda_margin: Option<Decimal>,

    /// Discount rate (WACC)
    #[arg(long)]
    pub discount_rate: Option<Decimal>,

    /// Terminal growth rate
    #[arg(long)]
    pub terminal_growth: Option<Decimal>,

    /// Projection years
    #[arg(long, default_value = "5")]
    pub years: u32,

    /// Capex as % of revenue (default 0.04)
    #[arg(long)]
    pub capex_pct: Option<Decimal>,

    /// Change in NWC as % of revenue (default 0.01)
    #[arg(long)]
    pub nwc_pct: Option<Decimal>,

    /// Marginal tax rate (default 0.25)
    #[arg(long)]
    pub tax_rate: Option<Decimal>,

    /// Net debt for equity bridge (optional)
    #[arg(long)]
    pub net_debt: Option<Decimal>,

    /// Diluted shares outstanding for per-share value (optional)
    #[arg(long)]
    pub shares_outstanding: Option<Decimal>,
}

/// Arguments for comparable company analysis
#[derive(Args)]
pub struct CompsArgs {
    /// Path to JSON input file with comparable company data
    #[arg(long)]
    pub input: Option<String>,
}

pub fn run_wacc(args: WaccArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let wacc_input: WaccInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        WaccInput {
            risk_free_rate: args
                .risk_free_rate
                .ok_or("--risk-free-rate is required (or provide --input)")?,
            equity_risk_premium: args
                .equity_risk_premium
                .ok_or("--equity-risk-premium is required (or provide --input)")?,
            beta: args.beta.unwrap_or(dec!(1.0)),
            cost_of_debt: args
                .cost_of_debt
                .ok_or("--cost-of-debt is required (or provide --input)")?,
            tax_rate: args
                .tax_rate
                .ok_or("--tax-rate is required (or provide --input)")?,
            debt_weight: args
                .debt_weight
                .ok_or("--debt-weight is required (or provide --input)")?,
            equity_weight: args
                .equity_weight
                .ok_or("--equity-weight is required (or provide --input)")?,
            size_premium: args.size_premium,
            country_risk_premium: args.country_risk,
            specific_risk_premium: args.specific_risk,
            unlevered_beta: args.unlevered_beta,
            target_debt_equity: args.target_debt_equity,
        }
    };

    let result = wacc::calculate_wacc(&wacc_input)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_dcf(args: DcfArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let dcf_input: DcfInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        let base_revenue = args
            .base_revenue
            .ok_or("--base-revenue is required (or provide --input)")?;
        let growth_rate = args
            .growth_rate
            .ok_or("--growth-rate is required (or provide --input)")?;
        let ebitda_margin = args
            .ebitda_margin
            .ok_or("--ebitda-margin is required (or provide --input)")?;
        let discount_rate = args
            .discount_rate
            .ok_or("--discount-rate is required (or provide --input)")?;
        let terminal_growth = args
            .terminal_growth
            .ok_or("--terminal-growth is required for Gordon Growth terminal value")?;

        DcfInput {
            base_revenue,
            revenue_growth_rates: vec![growth_rate; args.years as usize],
            ebitda_margin,
            ebit_margin: None,
            da_as_pct_revenue: None,
            capex_as_pct_revenue: args.capex_pct.unwrap_or(dec!(0.04)),
            nwc_as_pct_revenue: args.nwc_pct.unwrap_or(dec!(0.01)),
            tax_rate: args.tax_rate.unwrap_or(dec!(0.25)),
            wacc: discount_rate,
            wacc_input: None,
            terminal_method: TerminalMethod::GordonGrowth,
            terminal_growth_rate: Some(terminal_growth),
            terminal_exit_multiple: None,
            currency: Currency::USD,
            forecast_years: Some(args.years),
            mid_year_convention: Some(true),
            net_debt: args.net_debt,
            minority_interest: None,
            shares_outstanding: args.shares_outstanding,
        }
    };

    let result = dcf::calculate_dcf(&dcf_input)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_comps(args: CompsArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let comps_input: CompsInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input file (or stdin JSON) is required for comps analysis".into());
    };

    let result = comps::calculate_comps(&comps_input)?;
    Ok(serde_json::to_value(result)?)
}
