//! Capital controls analysis for emerging-market investments.
//!
//! Implements:
//! 1. **Repatriation cost** -- time-value of repatriation delays
//! 2. **Annual WHT cost** -- withholding taxes on dividends/interest/royalties
//! 3. **FX friction cost** -- conversion spread cost
//! 4. **Total annual friction** -- sum of all friction costs
//! 5. **Effective yield haircut** -- friction as yield reduction
//! 6. **Control severity score** -- 0-100
//! 7. **Trapped cash risk** -- probability of unable to repatriate
//! 8. **Mitigation strategies** -- suggested approaches
//!
//! All arithmetic uses `rust_decimal::Decimal`. No `f64`.

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

use crate::error::CorpFinanceError;
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Input / Output
// ---------------------------------------------------------------------------

/// Input for capital controls analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapitalControlsInput {
    /// Country name / code.
    pub country: String,
    /// Control regime type: "open", "light", "moderate", "restrictive", "closed".
    pub control_type: String,
    /// Average delay for profit repatriation in days.
    pub repatriation_delay_days: u32,
    /// WHT on dividend repatriation (e.g. 0.15 = 15%).
    pub withholding_tax_dividends: Decimal,
    /// WHT on interest payments.
    pub withholding_tax_interest: Decimal,
    /// WHT on royalties.
    pub withholding_tax_royalties: Decimal,
    /// Additional FX cost due to controls in basis points.
    pub fx_conversion_spread: Decimal,
    /// Total investment amount.
    pub investment_amount: Decimal,
    /// Expected annual income from investment.
    pub expected_annual_income: Decimal,
    /// Investment holding period in years.
    pub holding_period_years: u32,
    /// Risk-free rate for time-value calculations.
    pub risk_free_rate: Decimal,
}

/// Output from capital controls analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapitalControlsOutput {
    /// Time-value cost of repatriation delay.
    pub repatriation_cost: Decimal,
    /// Total annual WHT cost (max of the three WHT rates applied to income).
    pub annual_wht_cost: Decimal,
    /// Annual FX friction cost.
    pub fx_friction_cost: Decimal,
    /// Sum of all annual friction costs.
    pub total_annual_friction: Decimal,
    /// Total friction as a yield reduction (total_friction / investment).
    pub effective_yield_haircut: Decimal,
    /// Expected return after all frictions.
    pub net_expected_return: Decimal,
    /// Control severity score 0-100.
    pub control_severity_score: Decimal,
    /// Estimated probability of being unable to repatriate (0-1).
    pub trapped_cash_risk: Decimal,
    /// Suggested mitigation strategies.
    pub mitigation_strategies: Vec<String>,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Map control_type string to base severity score.
fn control_type_to_base_severity(ct: &str) -> Option<Decimal> {
    match ct {
        "open" => Some(Decimal::ZERO),
        "light" => Some(dec!(20)),
        "moderate" => Some(dec!(40)),
        "restrictive" => Some(dec!(70)),
        "closed" => Some(dec!(95)),
        _ => None,
    }
}

/// Map control_type string to trapped cash probability.
fn control_type_to_trapped_risk(ct: &str) -> Decimal {
    match ct {
        "open" => dec!(0.01),
        "light" => dec!(0.03),
        "moderate" => dec!(0.08),
        "restrictive" => dec!(0.20),
        "closed" => dec!(0.50),
        _ => dec!(0.10),
    }
}

// ---------------------------------------------------------------------------
// Main function
// ---------------------------------------------------------------------------

/// Analyse capital controls impact on an emerging-market investment.
pub fn analyse_capital_controls(
    input: &CapitalControlsInput,
) -> CorpFinanceResult<CapitalControlsOutput> {
    // Validation
    let base_severity = control_type_to_base_severity(&input.control_type).ok_or_else(|| {
        CorpFinanceError::InvalidInput {
            field: "control_type".to_string(),
            reason: format!(
                "Unknown control type '{}'. Use: open, light, moderate, restrictive, closed",
                input.control_type
            ),
        }
    })?;

    if input.investment_amount <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "investment_amount".to_string(),
            reason: "Investment amount must be positive".to_string(),
        });
    }
    if input.holding_period_years == 0 {
        return Err(CorpFinanceError::InvalidInput {
            field: "holding_period_years".to_string(),
            reason: "Holding period must be at least 1 year".to_string(),
        });
    }
    if input.withholding_tax_dividends < Decimal::ZERO
        || input.withholding_tax_interest < Decimal::ZERO
        || input.withholding_tax_royalties < Decimal::ZERO
    {
        return Err(CorpFinanceError::InvalidInput {
            field: "withholding_tax".to_string(),
            reason: "WHT rates cannot be negative".to_string(),
        });
    }
    if input.fx_conversion_spread < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "fx_conversion_spread".to_string(),
            reason: "FX spread cannot be negative".to_string(),
        });
    }
    if input.risk_free_rate < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "risk_free_rate".to_string(),
            reason: "Risk-free rate cannot be negative".to_string(),
        });
    }

    // 1. Repatriation cost = income * (delay/365) * risk_free_rate
    let delay_fraction = Decimal::from(input.repatriation_delay_days) / dec!(365);
    let repatriation_cost = input.expected_annual_income * delay_fraction * input.risk_free_rate;

    // 2. WHT cost = income * max(wht_dividends, wht_interest, wht_royalties)
    let max_wht = input
        .withholding_tax_dividends
        .max(input.withholding_tax_interest)
        .max(input.withholding_tax_royalties);
    let annual_wht_cost = input.expected_annual_income * max_wht;

    // 3. FX friction cost = investment * fx_spread / 10000
    let fx_friction_cost = input.investment_amount * input.fx_conversion_spread / dec!(10000);

    // 4. Total annual friction
    let total_annual_friction = repatriation_cost + annual_wht_cost + fx_friction_cost;

    // 5. Effective yield haircut
    let effective_yield_haircut = total_annual_friction / input.investment_amount;

    // 6. Net expected return
    let gross_yield = input.expected_annual_income / input.investment_amount;
    let net_expected_return = gross_yield - effective_yield_haircut;

    // 7. Control severity score: base + adjustments for delay and WHT
    // Delay adjustment: +1 per 10 days of delay, capped at +20
    let delay_adj = {
        let raw = Decimal::from(input.repatriation_delay_days) / dec!(10);
        if raw > dec!(20) {
            dec!(20)
        } else {
            raw
        }
    };
    // WHT adjustment: max_wht * 50 (so 20% WHT adds 10 points)
    let wht_adj = {
        let raw = max_wht * dec!(50);
        if raw > dec!(15) {
            dec!(15)
        } else {
            raw
        }
    };

    let severity_raw = base_severity + delay_adj + wht_adj;
    let control_severity_score = if severity_raw > dec!(100) {
        dec!(100)
    } else {
        severity_raw
    };

    // 8. Trapped cash risk
    let trapped_cash_risk = control_type_to_trapped_risk(&input.control_type);

    // 9. Mitigation strategies
    let mut strategies = Vec::new();
    if input.repatriation_delay_days > 30 {
        strategies.push(
            "Negotiate accelerated repatriation clauses in investment agreements".to_string(),
        );
    }
    if max_wht > dec!(0.10) {
        strategies.push("Explore tax treaty network to reduce withholding tax rates".to_string());
    }
    if input.fx_conversion_spread > dec!(50) {
        strategies.push("Use offshore NDF market to reduce FX conversion costs".to_string());
    }
    if input.control_type == "restrictive" || input.control_type == "closed" {
        strategies.push(
            "Structure investment as debt rather than equity to facilitate repatriation"
                .to_string(),
        );
        strategies.push(
            "Consider local reinvestment strategy to deploy trapped cash productively".to_string(),
        );
    }
    if input.control_type == "moderate" {
        strategies.push(
            "Maintain detailed documentation of all capital flows for regulatory compliance"
                .to_string(),
        );
    }
    if strategies.is_empty() {
        strategies.push("Open regime -- minimal capital control friction".to_string());
    }

    Ok(CapitalControlsOutput {
        repatriation_cost,
        annual_wht_cost,
        fx_friction_cost,
        total_annual_friction,
        effective_yield_haircut,
        net_expected_return,
        control_severity_score,
        trapped_cash_risk,
        mitigation_strategies: strategies,
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn base_input() -> CapitalControlsInput {
        CapitalControlsInput {
            country: "India".to_string(),
            control_type: "moderate".to_string(),
            repatriation_delay_days: 45,
            withholding_tax_dividends: dec!(0.15),
            withholding_tax_interest: dec!(0.10),
            withholding_tax_royalties: dec!(0.10),
            fx_conversion_spread: dec!(30), // 30 bps
            investment_amount: dec!(50_000_000),
            expected_annual_income: dec!(5_000_000),
            holding_period_years: 5,
            risk_free_rate: dec!(0.04),
        }
    }

    #[test]
    fn test_open_market() {
        let mut input = base_input();
        input.control_type = "open".to_string();
        input.repatriation_delay_days = 0;
        input.withholding_tax_dividends = Decimal::ZERO;
        input.withholding_tax_interest = Decimal::ZERO;
        input.withholding_tax_royalties = Decimal::ZERO;
        input.fx_conversion_spread = Decimal::ZERO;
        let out = analyse_capital_controls(&input).unwrap();
        assert_eq!(out.repatriation_cost, Decimal::ZERO);
        assert_eq!(out.annual_wht_cost, Decimal::ZERO);
        assert_eq!(out.fx_friction_cost, Decimal::ZERO);
        assert_eq!(out.control_severity_score, Decimal::ZERO);
        assert_eq!(out.trapped_cash_risk, dec!(0.01));
    }

    #[test]
    fn test_restrictive_controls() {
        let mut input = base_input();
        input.control_type = "restrictive".to_string();
        input.repatriation_delay_days = 180;
        let out = analyse_capital_controls(&input).unwrap();
        assert!(out.control_severity_score >= dec!(70));
        assert_eq!(out.trapped_cash_risk, dec!(0.20));
    }

    #[test]
    fn test_closed_controls() {
        let mut input = base_input();
        input.control_type = "closed".to_string();
        input.repatriation_delay_days = 365;
        let out = analyse_capital_controls(&input).unwrap();
        assert!(out.control_severity_score >= dec!(95));
        assert_eq!(out.trapped_cash_risk, dec!(0.50));
    }

    #[test]
    fn test_repatriation_cost_formula() {
        let input = base_input();
        let out = analyse_capital_controls(&input).unwrap();
        // cost = 5M * (45/365) * 0.04 ~ 24,657.53
        let diff = (out.repatriation_cost - dec!(24657.53)).abs();
        assert!(
            diff < dec!(1.0),
            "repatriation_cost={}",
            out.repatriation_cost
        );
    }

    #[test]
    fn test_wht_takes_max() {
        let input = base_input();
        let out = analyse_capital_controls(&input).unwrap();
        // max WHT = 15% (dividends)
        let expected = dec!(5_000_000) * dec!(0.15);
        assert_eq!(out.annual_wht_cost, expected);
    }

    #[test]
    fn test_fx_friction_cost() {
        let input = base_input();
        let out = analyse_capital_controls(&input).unwrap();
        // fx cost = 50M * 30 / 10000 = 150,000
        assert_eq!(out.fx_friction_cost, dec!(150_000));
    }

    #[test]
    fn test_total_friction_is_sum() {
        let input = base_input();
        let out = analyse_capital_controls(&input).unwrap();
        let sum = out.repatriation_cost + out.annual_wht_cost + out.fx_friction_cost;
        assert_eq!(out.total_annual_friction, sum);
    }

    #[test]
    fn test_effective_yield_haircut() {
        let input = base_input();
        let out = analyse_capital_controls(&input).unwrap();
        let expected = out.total_annual_friction / input.investment_amount;
        assert_eq!(out.effective_yield_haircut, expected);
    }

    #[test]
    fn test_net_expected_return() {
        let input = base_input();
        let out = analyse_capital_controls(&input).unwrap();
        let gross = input.expected_annual_income / input.investment_amount;
        assert_eq!(out.net_expected_return, gross - out.effective_yield_haircut);
    }

    #[test]
    fn test_high_wht_rate() {
        let mut input = base_input();
        input.withholding_tax_dividends = dec!(0.30);
        let out = analyse_capital_controls(&input).unwrap();
        assert_eq!(out.annual_wht_cost, dec!(5_000_000) * dec!(0.30));
    }

    #[test]
    fn test_long_delay() {
        let mut input = base_input();
        input.repatriation_delay_days = 365;
        let out = analyse_capital_controls(&input).unwrap();
        let expected = dec!(5_000_000) * dec!(365) / dec!(365) * dec!(0.04);
        assert_eq!(out.repatriation_cost, expected);
    }

    #[test]
    fn test_severity_score_capped_at_100() {
        let mut input = base_input();
        input.control_type = "closed".to_string();
        input.repatriation_delay_days = 500;
        input.withholding_tax_dividends = dec!(0.40);
        let out = analyse_capital_controls(&input).unwrap();
        assert!(out.control_severity_score <= dec!(100));
    }

    #[test]
    fn test_light_controls() {
        let mut input = base_input();
        input.control_type = "light".to_string();
        input.repatriation_delay_days = 10;
        let out = analyse_capital_controls(&input).unwrap();
        assert_eq!(out.trapped_cash_risk, dec!(0.03));
        assert!(out.control_severity_score >= dec!(20));
    }

    #[test]
    fn test_invalid_control_type() {
        let mut input = base_input();
        input.control_type = "unknown".to_string();
        let err = analyse_capital_controls(&input).unwrap_err();
        assert!(matches!(err, CorpFinanceError::InvalidInput { .. }));
    }

    #[test]
    fn test_invalid_zero_investment() {
        let mut input = base_input();
        input.investment_amount = Decimal::ZERO;
        let err = analyse_capital_controls(&input).unwrap_err();
        assert!(matches!(err, CorpFinanceError::InvalidInput { .. }));
    }

    #[test]
    fn test_invalid_zero_holding() {
        let mut input = base_input();
        input.holding_period_years = 0;
        let err = analyse_capital_controls(&input).unwrap_err();
        assert!(matches!(err, CorpFinanceError::InvalidInput { .. }));
    }

    #[test]
    fn test_invalid_negative_wht() {
        let mut input = base_input();
        input.withholding_tax_dividends = dec!(-0.05);
        let err = analyse_capital_controls(&input).unwrap_err();
        assert!(matches!(err, CorpFinanceError::InvalidInput { .. }));
    }

    #[test]
    fn test_mitigation_delay_strategy() {
        let input = base_input();
        let out = analyse_capital_controls(&input).unwrap();
        assert!(out
            .mitigation_strategies
            .iter()
            .any(|s| s.contains("repatriation")));
    }

    #[test]
    fn test_mitigation_wht_strategy() {
        let input = base_input();
        let out = analyse_capital_controls(&input).unwrap();
        assert!(out
            .mitigation_strategies
            .iter()
            .any(|s| s.contains("tax treaty")));
    }

    #[test]
    fn test_trapped_cash_moderate() {
        let input = base_input();
        let out = analyse_capital_controls(&input).unwrap();
        assert_eq!(out.trapped_cash_risk, dec!(0.08));
    }

    #[test]
    fn test_mitigation_restrictive() {
        let mut input = base_input();
        input.control_type = "restrictive".to_string();
        let out = analyse_capital_controls(&input).unwrap();
        assert!(out
            .mitigation_strategies
            .iter()
            .any(|s| s.contains("debt rather than equity")));
    }
}
