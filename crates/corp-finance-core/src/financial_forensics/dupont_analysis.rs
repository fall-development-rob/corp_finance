//! DuPont decomposition analysis (3-Way and 5-Way).
//!
//! Decomposes Return on Equity into its component drivers:
//! - 3-Way: Net Profit Margin x Asset Turnover x Equity Multiplier
//! - 5-Way: Tax Burden x Interest Burden x Operating Margin x Asset Turnover x Equity Multiplier
//!
//! All arithmetic uses `rust_decimal::Decimal`. No `f64`.

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

use crate::error::CorpFinanceError;
use crate::CorpFinanceResult;

type TrendResult = (
    Option<Decimal>,
    Option<Decimal>,
    Option<Decimal>,
    Option<Decimal>,
    Option<String>,
);

// ---------------------------------------------------------------------------
// Input / Output
// ---------------------------------------------------------------------------

/// Input for DuPont decomposition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DupontInput {
    pub net_income: Decimal,
    pub revenue: Decimal,
    pub total_assets: Decimal,
    pub shareholders_equity: Decimal,
    pub ebt: Decimal,
    pub ebit: Decimal,
    pub interest_expense: Decimal,
    pub tax_expense: Decimal,
    /// Prior period for trend analysis (all optional).
    pub prior_net_income: Option<Decimal>,
    pub prior_revenue: Option<Decimal>,
    pub prior_total_assets: Option<Decimal>,
    pub prior_equity: Option<Decimal>,
}

/// Output of DuPont decomposition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DupontOutput {
    // 3-Way decomposition
    pub roe: Decimal,
    pub net_profit_margin: Decimal,
    pub asset_turnover: Decimal,
    pub equity_multiplier: Decimal,
    pub roe_check: Decimal,
    // 5-Way decomposition
    pub tax_burden: Decimal,
    pub interest_burden: Decimal,
    pub operating_margin: Decimal,
    pub asset_turnover_5: Decimal,
    pub equity_multiplier_5: Decimal,
    pub roe_check_5: Decimal,
    // Trend (if prior period provided)
    pub roe_change: Option<Decimal>,
    pub margin_change: Option<Decimal>,
    pub turnover_change: Option<Decimal>,
    pub leverage_change: Option<Decimal>,
    pub primary_driver: Option<String>,
    // Qualitative assessment
    pub diagnosis: String,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn safe_div(num: Decimal, den: Decimal, ctx: &str) -> CorpFinanceResult<Decimal> {
    if den == Decimal::ZERO {
        return Err(CorpFinanceError::DivisionByZero {
            context: ctx.to_string(),
        });
    }
    Ok(num / den)
}

fn diagnose(
    margin: Decimal,
    turnover: Decimal,
    multiplier: Decimal,
    tax_burden: Decimal,
    interest_burden: Decimal,
) -> String {
    let mut parts = Vec::new();

    if margin < dec!(0.05) {
        parts.push("Low profit margins indicate pricing pressure or cost issues");
    } else if margin > dec!(0.20) {
        parts.push("Strong profit margins suggest competitive advantage");
    }

    if turnover < dec!(0.5) {
        parts.push(
            "Low asset turnover suggests capital-intensive operations or underutilized assets",
        );
    } else if turnover > dec!(2.0) {
        parts.push("High asset turnover indicates efficient asset utilization");
    }

    if multiplier > dec!(5.0) {
        parts.push("Very high financial leverage increases financial risk significantly");
    } else if multiplier > dec!(3.0) {
        parts.push("Elevated financial leverage increases risk");
    } else if multiplier < dec!(1.5) {
        parts.push("Conservative leverage with low financial risk");
    }

    if tax_burden < dec!(0.6) {
        parts.push("High effective tax rate reduces returns");
    }

    if interest_burden < dec!(0.7) {
        parts.push("Interest expense significantly erodes pre-tax profits");
    }

    if parts.is_empty() {
        "Balanced DuPont profile with no notable concerns".to_string()
    } else {
        parts.join(". ") + "."
    }
}

// ---------------------------------------------------------------------------
// Core function
// ---------------------------------------------------------------------------

/// Perform DuPont decomposition (3-way and 5-way).
pub fn calculate_dupont(input: &DupontInput) -> CorpFinanceResult<DupontOutput> {
    // Validation
    if input.revenue == Decimal::ZERO {
        return Err(CorpFinanceError::DivisionByZero {
            context: "Revenue cannot be zero for DuPont analysis.".into(),
        });
    }
    if input.total_assets == Decimal::ZERO {
        return Err(CorpFinanceError::DivisionByZero {
            context: "Total assets cannot be zero for DuPont analysis.".into(),
        });
    }
    if input.shareholders_equity == Decimal::ZERO {
        return Err(CorpFinanceError::DivisionByZero {
            context: "Shareholders equity cannot be zero for DuPont analysis.".into(),
        });
    }
    if input.ebt == Decimal::ZERO && input.net_income != Decimal::ZERO {
        // Allow EBT=0 only if NI is also 0
        return Err(CorpFinanceError::DivisionByZero {
            context: "EBT cannot be zero when net income is non-zero.".into(),
        });
    }
    if input.ebit == Decimal::ZERO && input.ebt != Decimal::ZERO {
        return Err(CorpFinanceError::DivisionByZero {
            context: "EBIT cannot be zero when EBT is non-zero.".into(),
        });
    }

    // 3-Way decomposition
    let roe = safe_div(input.net_income, input.shareholders_equity, "ROE")?;
    let net_profit_margin = safe_div(input.net_income, input.revenue, "Net profit margin")?;
    let asset_turnover = safe_div(input.revenue, input.total_assets, "Asset turnover")?;
    let equity_multiplier = safe_div(
        input.total_assets,
        input.shareholders_equity,
        "Equity multiplier",
    )?;
    let roe_check = net_profit_margin * asset_turnover * equity_multiplier;

    // 5-Way decomposition
    let tax_burden = if input.ebt == Decimal::ZERO {
        Decimal::ZERO
    } else {
        safe_div(input.net_income, input.ebt, "Tax burden")?
    };
    let interest_burden = if input.ebit == Decimal::ZERO {
        Decimal::ZERO
    } else {
        safe_div(input.ebt, input.ebit, "Interest burden")?
    };
    let operating_margin = safe_div(input.ebit, input.revenue, "Operating margin")?;
    let asset_turnover_5 = asset_turnover;
    let equity_multiplier_5 = equity_multiplier;
    let roe_check_5 =
        tax_burden * interest_burden * operating_margin * asset_turnover_5 * equity_multiplier_5;

    // Trend analysis
    let (roe_change, margin_change, turnover_change, leverage_change, primary_driver) =
        compute_trend(input, net_profit_margin, asset_turnover, equity_multiplier)?;

    let diagnosis = diagnose(
        net_profit_margin,
        asset_turnover,
        equity_multiplier,
        tax_burden,
        interest_burden,
    );

    Ok(DupontOutput {
        roe,
        net_profit_margin,
        asset_turnover,
        equity_multiplier,
        roe_check,
        tax_burden,
        interest_burden,
        operating_margin,
        asset_turnover_5,
        equity_multiplier_5,
        roe_check_5,
        roe_change,
        margin_change,
        turnover_change,
        leverage_change,
        primary_driver,
        diagnosis,
    })
}

fn compute_trend(
    input: &DupontInput,
    cur_margin: Decimal,
    cur_turnover: Decimal,
    cur_multiplier: Decimal,
) -> CorpFinanceResult<TrendResult> {
    let (prior_ni, prior_rev, prior_ta, prior_eq) = match (
        input.prior_net_income,
        input.prior_revenue,
        input.prior_total_assets,
        input.prior_equity,
    ) {
        (Some(ni), Some(rev), Some(ta), Some(eq)) => (ni, rev, ta, eq),
        _ => return Ok((None, None, None, None, None)),
    };

    if prior_rev == Decimal::ZERO || prior_ta == Decimal::ZERO || prior_eq == Decimal::ZERO {
        return Ok((None, None, None, None, None));
    }

    let prior_roe = prior_ni / prior_eq;
    let prior_margin = prior_ni / prior_rev;
    let prior_turnover = prior_rev / prior_ta;
    let prior_multiplier = prior_ta / prior_eq;

    let roe_chg = cur_margin * cur_turnover * cur_multiplier - prior_roe;
    let margin_chg = cur_margin - prior_margin;
    let turnover_chg = cur_turnover - prior_turnover;
    let leverage_chg = cur_multiplier - prior_multiplier;

    // Primary driver = component with largest absolute change
    let changes = [
        ("Profit margin".to_string(), margin_chg.abs()),
        ("Asset turnover".to_string(), turnover_chg.abs()),
        ("Financial leverage".to_string(), leverage_chg.abs()),
    ];
    let driver = changes
        .iter()
        .max_by(|a, b| a.1.cmp(&b.1))
        .map(|(name, _)| name.clone());

    Ok((
        Some(roe_chg),
        Some(margin_chg),
        Some(turnover_chg),
        Some(leverage_chg),
        driver,
    ))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn approx_eq(a: Decimal, b: Decimal, eps: Decimal) -> bool {
        (a - b).abs() < eps
    }

    fn base_input() -> DupontInput {
        DupontInput {
            net_income: dec!(100),
            revenue: dec!(1000),
            total_assets: dec!(2000),
            shareholders_equity: dec!(800),
            ebt: dec!(130),
            ebit: dec!(150),
            interest_expense: dec!(20),
            tax_expense: dec!(30),
            prior_net_income: None,
            prior_revenue: None,
            prior_total_assets: None,
            prior_equity: None,
        }
    }

    #[test]
    fn test_roe_calculation() {
        let out = calculate_dupont(&base_input()).unwrap();
        assert_eq!(out.roe, dec!(100) / dec!(800));
    }

    #[test]
    fn test_net_profit_margin() {
        let out = calculate_dupont(&base_input()).unwrap();
        assert_eq!(out.net_profit_margin, dec!(100) / dec!(1000));
    }

    #[test]
    fn test_asset_turnover() {
        let out = calculate_dupont(&base_input()).unwrap();
        assert_eq!(out.asset_turnover, dec!(1000) / dec!(2000));
    }

    #[test]
    fn test_equity_multiplier() {
        let out = calculate_dupont(&base_input()).unwrap();
        assert_eq!(out.equity_multiplier, dec!(2000) / dec!(800));
    }

    #[test]
    fn test_three_way_identity() {
        let out = calculate_dupont(&base_input()).unwrap();
        assert!(approx_eq(out.roe, out.roe_check, dec!(0.0001)));
    }

    #[test]
    fn test_tax_burden() {
        let out = calculate_dupont(&base_input()).unwrap();
        assert_eq!(out.tax_burden, dec!(100) / dec!(130));
    }

    #[test]
    fn test_interest_burden() {
        let out = calculate_dupont(&base_input()).unwrap();
        assert_eq!(out.interest_burden, dec!(130) / dec!(150));
    }

    #[test]
    fn test_operating_margin() {
        let out = calculate_dupont(&base_input()).unwrap();
        assert_eq!(out.operating_margin, dec!(150) / dec!(1000));
    }

    #[test]
    fn test_five_way_identity() {
        let out = calculate_dupont(&base_input()).unwrap();
        assert!(
            approx_eq(out.roe, out.roe_check_5, dec!(0.0001)),
            "ROE={} vs 5-way check={}",
            out.roe,
            out.roe_check_5
        );
    }

    #[test]
    fn test_high_margin_low_turnover() {
        let mut input = base_input();
        input.net_income = dec!(300);
        input.revenue = dec!(1000);
        input.total_assets = dec!(5000);
        input.shareholders_equity = dec!(3000);
        input.ebt = dec!(400);
        input.ebit = dec!(450);
        let out = calculate_dupont(&input).unwrap();
        assert!(out.net_profit_margin > dec!(0.2));
        assert!(out.asset_turnover < dec!(0.5));
    }

    #[test]
    fn test_high_leverage() {
        let mut input = base_input();
        input.shareholders_equity = dec!(200);
        let out = calculate_dupont(&input).unwrap();
        assert!(out.equity_multiplier > dec!(5.0));
    }

    #[test]
    fn test_trend_analysis_with_prior() {
        let mut input = base_input();
        input.prior_net_income = Some(dec!(80));
        input.prior_revenue = Some(dec!(900));
        input.prior_total_assets = Some(dec!(1800));
        input.prior_equity = Some(dec!(750));
        let out = calculate_dupont(&input).unwrap();
        assert!(out.roe_change.is_some());
        assert!(out.margin_change.is_some());
        assert!(out.turnover_change.is_some());
        assert!(out.leverage_change.is_some());
        assert!(out.primary_driver.is_some());
    }

    #[test]
    fn test_no_trend_without_prior() {
        let out = calculate_dupont(&base_input()).unwrap();
        assert!(out.roe_change.is_none());
        assert!(out.primary_driver.is_none());
    }

    #[test]
    fn test_primary_driver_margin() {
        let mut input = base_input();
        // Big margin change, small turnover and leverage change
        input.prior_net_income = Some(dec!(20));
        input.prior_revenue = Some(dec!(1000));
        input.prior_total_assets = Some(dec!(2000));
        input.prior_equity = Some(dec!(800));
        let out = calculate_dupont(&input).unwrap();
        assert_eq!(out.primary_driver.as_deref(), Some("Profit margin"));
    }

    #[test]
    fn test_zero_revenue_error() {
        let mut input = base_input();
        input.revenue = Decimal::ZERO;
        assert!(calculate_dupont(&input).is_err());
    }

    #[test]
    fn test_zero_total_assets_error() {
        let mut input = base_input();
        input.total_assets = Decimal::ZERO;
        assert!(calculate_dupont(&input).is_err());
    }

    #[test]
    fn test_zero_equity_error() {
        let mut input = base_input();
        input.shareholders_equity = Decimal::ZERO;
        assert!(calculate_dupont(&input).is_err());
    }

    #[test]
    fn test_diagnosis_contains_text() {
        let out = calculate_dupont(&base_input()).unwrap();
        assert!(!out.diagnosis.is_empty());
    }

    #[test]
    fn test_diagnosis_high_leverage() {
        let mut input = base_input();
        input.shareholders_equity = dec!(100);
        let out = calculate_dupont(&input).unwrap();
        assert!(out.diagnosis.contains("leverage"));
    }

    #[test]
    fn test_zero_ebt_zero_ni_ok() {
        let mut input = base_input();
        input.net_income = Decimal::ZERO;
        input.ebt = Decimal::ZERO;
        input.ebit = Decimal::ZERO;
        let out = calculate_dupont(&input).unwrap();
        assert_eq!(out.tax_burden, Decimal::ZERO);
        assert_eq!(out.interest_burden, Decimal::ZERO);
    }

    #[test]
    fn test_serialization_roundtrip() {
        let out = calculate_dupont(&base_input()).unwrap();
        let json = serde_json::to_string(&out).unwrap();
        let _deser: DupontOutput = serde_json::from_str(&json).unwrap();
    }
}
