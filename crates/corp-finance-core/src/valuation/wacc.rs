use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::error::CorpFinanceError;
use crate::types::{with_metadata, ComputationOutput, Rate};
use crate::CorpFinanceResult;

/// Input parameters for Weighted Average Cost of Capital calculation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaccInput {
    /// Risk-free rate (e.g. 10-year government bond yield)
    pub risk_free_rate: Rate,
    /// Equity risk premium (market return minus risk-free rate)
    pub equity_risk_premium: Rate,
    /// Levered beta of equity
    pub beta: Decimal,
    /// Pre-tax cost of debt
    pub cost_of_debt: Rate,
    /// Marginal corporate tax rate
    pub tax_rate: Rate,
    /// Weight of debt in capital structure (market value basis)
    pub debt_weight: Rate,
    /// Weight of equity in capital structure (market value basis)
    pub equity_weight: Rate,
    /// Small-cap / size premium (Duff & Phelps)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size_premium: Option<Rate>,
    /// Country risk premium for emerging markets
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country_risk_premium: Option<Rate>,
    /// Company-specific / alpha risk premium
    #[serde(skip_serializing_if = "Option::is_none")]
    pub specific_risk_premium: Option<Rate>,
    /// Unlevered (asset) beta — if provided, will re-lever via Hamada
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unlevered_beta: Option<Decimal>,
    /// Target debt-to-equity ratio for Hamada re-levering
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_debt_equity: Option<Decimal>,
}

/// Output of the WACC calculation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaccOutput {
    /// Weighted average cost of capital
    pub wacc: Rate,
    /// Cost of equity (via CAPM + premiums)
    pub cost_of_equity: Rate,
    /// After-tax cost of debt
    pub after_tax_cost_of_debt: Rate,
    /// Pre-tax cost of debt (echoed back)
    pub cost_of_debt_pretax: Rate,
    /// Levered beta used in the calculation
    pub levered_beta: Decimal,
    /// Unlevered beta (if computed via Hamada)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unlevered_beta: Option<Decimal>,
}

/// Calculate the Weighted Average Cost of Capital using CAPM.
///
/// Cost of equity: Ke = Rf + Beta * ERP + size_premium + country_risk + specific_risk
/// After-tax cost of debt: Kd_at = Kd * (1 - t)
/// WACC = Ke * We + Kd_at * Wd
///
/// If `unlevered_beta` is provided, the levered beta is computed using the
/// Hamada equation: Beta_L = Beta_U * (1 + (1 - t) * D/E).
pub fn calculate_wacc(input: &WaccInput) -> CorpFinanceResult<ComputationOutput<WaccOutput>> {
    let start = Instant::now();
    let mut warnings: Vec<String> = Vec::new();

    // --- Validation ---
    validate_wacc_input(input)?;

    // Check weight sum
    let weight_sum = input.debt_weight + input.equity_weight;
    let weight_tolerance = dec!(0.01);
    if (weight_sum - Decimal::ONE).abs() > weight_tolerance {
        return Err(CorpFinanceError::InvalidInput {
            field: "debt_weight + equity_weight".into(),
            reason: format!("Capital structure weights must sum to 1.0, got {weight_sum}"),
        });
    }

    // --- Compute levered beta (Hamada re-levering if applicable) ---
    let (levered_beta, unlevered_beta_out) = compute_beta(input, &mut warnings)?;

    // --- Cost of Equity (CAPM build-up) ---
    let cost_of_equity = compute_cost_of_equity(input, levered_beta);

    // --- After-tax cost of debt ---
    let after_tax_cost_of_debt = input.cost_of_debt * (Decimal::ONE - input.tax_rate);

    // --- WACC ---
    let wacc = cost_of_equity * input.equity_weight + after_tax_cost_of_debt * input.debt_weight;

    // --- Reasonableness warnings ---
    if levered_beta > dec!(3.0) {
        warnings.push(format!(
            "High beta ({levered_beta}): verify market data; betas above 3.0 are unusual"
        ));
    }
    if input.equity_risk_premium > dec!(0.10) {
        warnings.push(format!(
            "Equity risk premium ({}) exceeds 10%; verify estimate",
            input.equity_risk_premium
        ));
    }
    if wacc > dec!(0.20) {
        warnings.push(format!(
            "WACC of {wacc} exceeds 20%; appropriate for high-risk / emerging-market situations only"
        ));
    }

    let output = WaccOutput {
        wacc,
        cost_of_equity,
        after_tax_cost_of_debt,
        cost_of_debt_pretax: input.cost_of_debt,
        levered_beta,
        unlevered_beta: unlevered_beta_out,
    };

    let elapsed = start.elapsed().as_micros() as u64;

    Ok(with_metadata(
        "WACC via CAPM build-up",
        input,
        warnings,
        elapsed,
        output,
    ))
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn validate_wacc_input(input: &WaccInput) -> CorpFinanceResult<()> {
    if input.risk_free_rate < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "risk_free_rate".into(),
            reason: "Risk-free rate cannot be negative".into(),
        });
    }
    if input.equity_risk_premium < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "equity_risk_premium".into(),
            reason: "Equity risk premium cannot be negative".into(),
        });
    }
    if input.beta <= Decimal::ZERO {
        // Only check the explicit beta if we are NOT re-levering from unlevered
        if input.unlevered_beta.is_none() {
            return Err(CorpFinanceError::InvalidInput {
                field: "beta".into(),
                reason: "Beta must be positive".into(),
            });
        }
    }
    if input.cost_of_debt < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "cost_of_debt".into(),
            reason: "Cost of debt cannot be negative".into(),
        });
    }
    if input.tax_rate < Decimal::ZERO || input.tax_rate > Decimal::ONE {
        return Err(CorpFinanceError::InvalidInput {
            field: "tax_rate".into(),
            reason: "Tax rate must be between 0 and 1".into(),
        });
    }
    if input.debt_weight < Decimal::ZERO || input.equity_weight < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "debt_weight / equity_weight".into(),
            reason: "Capital structure weights cannot be negative".into(),
        });
    }
    Ok(())
}

/// Returns (levered_beta, Option<unlevered_beta>).
fn compute_beta(
    input: &WaccInput,
    warnings: &mut Vec<String>,
) -> CorpFinanceResult<(Decimal, Option<Decimal>)> {
    match (input.unlevered_beta, input.target_debt_equity) {
        (Some(beta_u), Some(de)) => {
            if beta_u <= Decimal::ZERO {
                return Err(CorpFinanceError::InvalidInput {
                    field: "unlevered_beta".into(),
                    reason: "Unlevered beta must be positive".into(),
                });
            }
            if de < Decimal::ZERO {
                return Err(CorpFinanceError::InvalidInput {
                    field: "target_debt_equity".into(),
                    reason: "Target D/E ratio cannot be negative".into(),
                });
            }
            // Hamada equation: Beta_L = Beta_U * (1 + (1-t) * D/E)
            let beta_l = beta_u * (Decimal::ONE + (Decimal::ONE - input.tax_rate) * de);
            warnings.push(format!(
                "Levered beta re-calculated via Hamada equation: {beta_l} (from unlevered {beta_u}, D/E {de})"
            ));
            Ok((beta_l, Some(beta_u)))
        }
        (Some(beta_u), None) => {
            // Have unlevered beta but no target D/E — use equity/debt weights to derive D/E
            if input.equity_weight.is_zero() {
                return Err(CorpFinanceError::DivisionByZero {
                    context: "Cannot derive D/E ratio when equity weight is zero".into(),
                });
            }
            let de = input.debt_weight / input.equity_weight;
            let beta_l = beta_u * (Decimal::ONE + (Decimal::ONE - input.tax_rate) * de);
            warnings.push(format!(
                "Levered beta re-calculated via Hamada equation: {beta_l} (from unlevered {beta_u}, implied D/E {de})"
            ));
            Ok((beta_l, Some(beta_u)))
        }
        _ => Ok((input.beta, None)),
    }
}

fn compute_cost_of_equity(input: &WaccInput, levered_beta: Decimal) -> Rate {
    let mut ke = input.risk_free_rate + levered_beta * input.equity_risk_premium;
    if let Some(sp) = input.size_premium {
        ke += sp;
    }
    if let Some(crp) = input.country_risk_premium {
        ke += crp;
    }
    if let Some(srp) = input.specific_risk_premium {
        ke += srp;
    }
    ke
}

/// Unlever a beta using the Hamada equation.
///
/// Beta_U = Beta_L / (1 + (1 - t) * D/E)
pub fn unlever_beta(
    levered_beta: Decimal,
    tax_rate: Rate,
    debt_equity: Decimal,
) -> CorpFinanceResult<Decimal> {
    let denom = Decimal::ONE + (Decimal::ONE - tax_rate) * debt_equity;
    if denom.is_zero() {
        return Err(CorpFinanceError::DivisionByZero {
            context: "Hamada unlever denominator".into(),
        });
    }
    Ok(levered_beta / denom)
}

/// Re-lever a beta using the Hamada equation.
///
/// Beta_L = Beta_U * (1 + (1 - t) * D/E)
pub fn relever_beta(unlevered_beta: Decimal, tax_rate: Rate, debt_equity: Decimal) -> Decimal {
    unlevered_beta * (Decimal::ONE + (Decimal::ONE - tax_rate) * debt_equity)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    /// Build a typical US-market WACC input (Damodaran-style).
    fn sample_input() -> WaccInput {
        WaccInput {
            risk_free_rate: dec!(0.042),      // 10-year UST ~4.2%
            equity_risk_premium: dec!(0.055), // Damodaran ERP Jan 2024
            beta: dec!(1.10),
            cost_of_debt: dec!(0.055), // BBB spread
            tax_rate: dec!(0.21),      // US federal
            debt_weight: dec!(0.30),
            equity_weight: dec!(0.70),
            size_premium: None,
            country_risk_premium: None,
            specific_risk_premium: None,
            unlevered_beta: None,
            target_debt_equity: None,
        }
    }

    #[test]
    fn test_basic_wacc() {
        let input = sample_input();
        let result = calculate_wacc(&input).unwrap();
        let out = &result.result;

        // Ke = 0.042 + 1.10 * 0.055 = 0.042 + 0.0605 = 0.1025
        let expected_ke = dec!(0.1025);
        assert!(
            (out.cost_of_equity - expected_ke).abs() < dec!(0.0001),
            "Cost of equity: expected ~{expected_ke}, got {}",
            out.cost_of_equity
        );

        // Kd_at = 0.055 * (1 - 0.21) = 0.055 * 0.79 = 0.04345
        let expected_kd_at = dec!(0.04345);
        assert!(
            (out.after_tax_cost_of_debt - expected_kd_at).abs() < dec!(0.0001),
            "After-tax Kd: expected ~{expected_kd_at}, got {}",
            out.after_tax_cost_of_debt
        );

        // WACC = 0.1025 * 0.70 + 0.04345 * 0.30 = 0.07175 + 0.013035 = 0.084785
        let expected_wacc = dec!(0.084785);
        assert!(
            (out.wacc - expected_wacc).abs() < dec!(0.001),
            "WACC: expected ~{expected_wacc}, got {}",
            out.wacc
        );

        assert_eq!(out.levered_beta, dec!(1.10));
        assert!(out.unlevered_beta.is_none());
    }

    #[test]
    fn test_wacc_with_hamada_relevering() {
        let input = WaccInput {
            risk_free_rate: dec!(0.042),
            equity_risk_premium: dec!(0.055),
            beta: dec!(1.0), // will be overridden by Hamada
            cost_of_debt: dec!(0.055),
            tax_rate: dec!(0.21),
            debt_weight: dec!(0.40),
            equity_weight: dec!(0.60),
            size_premium: None,
            country_risk_premium: None,
            specific_risk_premium: None,
            unlevered_beta: Some(dec!(0.80)),
            target_debt_equity: Some(dec!(0.667)), // 40/60
        };

        let result = calculate_wacc(&input).unwrap();
        let out = &result.result;

        // Beta_L = 0.80 * (1 + (1-0.21) * 0.667) = 0.80 * (1 + 0.52693) = 0.80 * 1.52693 = 1.22154
        let expected_beta_l = dec!(0.80) * (Decimal::ONE + dec!(0.79) * dec!(0.667));
        assert!(
            (out.levered_beta - expected_beta_l).abs() < dec!(0.001),
            "Levered beta: expected ~{expected_beta_l}, got {}",
            out.levered_beta
        );
        assert_eq!(out.unlevered_beta, Some(dec!(0.80)));
    }

    #[test]
    fn test_wacc_with_premiums() {
        let input = WaccInput {
            risk_free_rate: dec!(0.042),
            equity_risk_premium: dec!(0.055),
            beta: dec!(1.20),
            cost_of_debt: dec!(0.070),
            tax_rate: dec!(0.25),
            debt_weight: dec!(0.35),
            equity_weight: dec!(0.65),
            size_premium: Some(dec!(0.015)),
            country_risk_premium: Some(dec!(0.025)),
            specific_risk_premium: Some(dec!(0.010)),
            unlevered_beta: None,
            target_debt_equity: None,
        };

        let result = calculate_wacc(&input).unwrap();
        let out = &result.result;

        // Ke = 0.042 + 1.20*0.055 + 0.015 + 0.025 + 0.01 = 0.042 + 0.066 + 0.05 = 0.158
        let expected_ke = dec!(0.158);
        assert!(
            (out.cost_of_equity - expected_ke).abs() < dec!(0.001),
            "Cost of equity with premiums: expected ~{expected_ke}, got {}",
            out.cost_of_equity
        );
    }

    #[test]
    fn test_wacc_weights_must_sum_to_one() {
        let mut input = sample_input();
        input.debt_weight = dec!(0.50);
        input.equity_weight = dec!(0.60);

        let result = calculate_wacc(&input);
        assert!(result.is_err());
        match result.unwrap_err() {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert!(field.contains("weight"));
            }
            e => panic!("Expected InvalidInput, got {e:?}"),
        }
    }

    #[test]
    fn test_negative_rate_rejected() {
        let mut input = sample_input();
        input.risk_free_rate = dec!(-0.01);

        let result = calculate_wacc(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_zero_beta_rejected() {
        let mut input = sample_input();
        input.beta = Decimal::ZERO;

        let result = calculate_wacc(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_high_beta_warning() {
        let mut input = sample_input();
        input.beta = dec!(3.5);

        let result = calculate_wacc(&input).unwrap();
        assert!(result.warnings.iter().any(|w| w.contains("High beta")));
    }

    #[test]
    fn test_high_erp_warning() {
        let mut input = sample_input();
        input.equity_risk_premium = dec!(0.12);

        let result = calculate_wacc(&input).unwrap();
        assert!(result
            .warnings
            .iter()
            .any(|w| w.contains("Equity risk premium")));
    }

    #[test]
    fn test_unlever_relever_roundtrip() {
        let beta_l = dec!(1.20);
        let tax = dec!(0.21);
        let de = dec!(0.50);

        let beta_u = unlever_beta(beta_l, tax, de).unwrap();
        let beta_l_back = relever_beta(beta_u, tax, de);

        assert!(
            (beta_l - beta_l_back).abs() < dec!(0.00001),
            "Round-trip failed: {beta_l} -> {beta_u} -> {beta_l_back}"
        );
    }

    #[test]
    fn test_damodaran_reference_values() {
        // Reference: Damodaran US market data Jan 2024
        // S&P 500 beta=1.0, Rf=4.2%, ERP=5.5%, typical BBB spread Kd=5.5%
        // 30% debt, 70% equity, 21% tax
        // Expected WACC ~8.5%
        let input = sample_input();
        let result = calculate_wacc(&input).unwrap();
        let wacc = result.result.wacc;

        assert!(
            wacc > dec!(0.07) && wacc < dec!(0.10),
            "Damodaran reference WACC should be ~8.5%, got {wacc}"
        );
    }

    #[test]
    fn test_methodology_string() {
        let input = sample_input();
        let result = calculate_wacc(&input).unwrap();
        assert_eq!(result.methodology, "WACC via CAPM build-up");
    }
}
