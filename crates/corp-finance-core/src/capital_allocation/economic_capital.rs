//! Economic Capital Calculation.
//!
//! Covers:
//! 1. **VaR-based Capital** -- EC = VaR(confidence) - Expected Loss
//! 2. **ES-based Capital** -- Expected Shortfall = average of losses exceeding VaR
//! 3. **Stress Capital Buffer** -- additional capital from stress scenarios (base/adverse/severe)
//! 4. **IRB Capital Requirement** -- Basel IRB formula with maturity adjustment
//! 5. **Capital Adequacy Ratio** -- total_capital / risk_weighted_assets
//!
//! All arithmetic uses `rust_decimal::Decimal`. No `f64`.

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

use crate::error::CorpFinanceError;
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Private math helpers: norm_cdf and norm_inv (Abramowitz & Stegun)
// ---------------------------------------------------------------------------

/// Natural logarithm via Newton's method (20 iterations).
/// ln(x) for x > 0; uses the identity ln(x) = 2 * atanh((x-1)/(x+1)).
fn decimal_ln(x: Decimal) -> Decimal {
    if x <= Decimal::ZERO {
        return Decimal::ZERO;
    }
    if x == Decimal::ONE {
        return Decimal::ZERO;
    }

    // Reduce x into [0.5, 2) range by extracting powers of 10
    let mut val = x;
    let mut adjust = Decimal::ZERO;
    let ln10 = dec!(2.302585092994046);

    while val > dec!(10) {
        val /= dec!(10);
        adjust += ln10;
    }
    while val < dec!(0.1) {
        val *= dec!(10);
        adjust -= ln10;
    }

    // Series: ln(x) = 2 * sum_{k=0}^{n} (1/(2k+1)) * ((x-1)/(x+1))^(2k+1)
    let z = (val - Decimal::ONE) / (val + Decimal::ONE);
    let z2 = z * z;
    let mut term = z;
    let mut sum = z;
    for k in 1u32..40 {
        term *= z2;
        let denom = Decimal::from(2 * k + 1);
        sum += term / denom;
    }
    dec!(2) * sum + adjust
}

/// Exponential via Taylor series (40 terms).
fn decimal_exp(x: Decimal) -> Decimal {
    let mut term = Decimal::ONE;
    let mut sum = Decimal::ONE;
    for i in 1u32..40 {
        term = term * x / Decimal::from(i);
        sum += term;
    }
    sum
}

/// Square root via Newton's method (20 iterations).
fn decimal_sqrt(x: Decimal) -> Decimal {
    if x <= Decimal::ZERO {
        return Decimal::ZERO;
    }
    let mut guess = x / dec!(2);
    if guess.is_zero() {
        guess = dec!(0.001);
    }
    for _ in 0..20 {
        guess = (guess + x / guess) / dec!(2);
    }
    guess
}

/// Cumulative standard normal distribution (Abramowitz & Stegun approximation).
fn norm_cdf(x: Decimal) -> Decimal {
    let a1 = dec!(0.254829592);
    let a2 = dec!(-0.284496736);
    let a3 = dec!(1.421413741);
    let a4 = dec!(-1.453152027);
    let a5 = dec!(1.061405429);
    let p = dec!(0.3275911);

    let sign = if x < Decimal::ZERO {
        dec!(-1)
    } else {
        Decimal::ONE
    };
    let abs_x = x.abs();
    let t = Decimal::ONE / (Decimal::ONE + p * abs_x);
    let t2 = t * t;
    let t3 = t2 * t;
    let t4 = t3 * t;
    let t5 = t4 * t;

    let y = Decimal::ONE
        - (a1 * t + a2 * t2 + a3 * t3 + a4 * t4 + a5 * t5)
            * decimal_exp(-(abs_x * abs_x) / dec!(2));

    (Decimal::ONE + sign * y) / dec!(2)
}

/// Inverse standard normal (rational approximation, Abramowitz & Stegun / Beasley-Springer-Moro).
fn norm_inv(p: Decimal) -> Decimal {
    if p <= Decimal::ZERO {
        return dec!(-6);
    }
    if p >= Decimal::ONE {
        return dec!(6);
    }
    if p == dec!(0.5) {
        return Decimal::ZERO;
    }

    // Rational approximation for central region
    let half = dec!(0.5);
    if p > dec!(0.02425) && p < dec!(0.97575) {
        let q = p - half;
        let r = q * q;
        let num = ((((dec!(-39.69683028665376) * r + dec!(220.9460984245205)) * r
            + dec!(-275.9285104469687))
            * r
            + dec!(138.3577518672690))
            * r
            + dec!(-30.66479806614716))
            * r
            + dec!(2.506628277459239);
        let den_val = ((((dec!(-54.47609879822406) * r + dec!(161.5858368580410)) * r
            + dec!(-155.6989798598866))
            * r
            + dec!(66.80131188771972))
            * r
            + dec!(-13.28068155288572))
            * r
            + Decimal::ONE;
        return q * num / den_val;
    }

    // Tail approximation (Beasley-Springer-Moro)
    let r = if p < half {
        decimal_sqrt(dec!(-2) * decimal_ln(p))
    } else {
        decimal_sqrt(dec!(-2) * decimal_ln(Decimal::ONE - p))
    };

    let result = -(dec!(2.515517) + dec!(0.802853) * r + dec!(0.010328) * r * r)
        / (Decimal::ONE + dec!(1.432788) * r + dec!(0.189269) * r * r + dec!(0.001308) * r * r * r);

    if p < half {
        result
    } else {
        -result
    }
}

// ---------------------------------------------------------------------------
// Input / Output
// ---------------------------------------------------------------------------

/// Stress scenario losses.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StressScenario {
    /// Scenario name (e.g. "Base", "Adverse", "Severe").
    pub name: String,
    /// Estimated portfolio loss under this scenario.
    pub loss: Decimal,
}

/// Input for economic capital calculation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EconomicCapitalInput {
    /// Simulated portfolio losses (positive = loss, negative = gain).
    pub portfolio_losses: Vec<Decimal>,
    /// Confidence level for VaR/ES (e.g. 0.999 for 99.9%).
    pub confidence_level: Decimal,
    /// Probability of default (annualized, decimal).
    pub pd: Decimal,
    /// Loss given default (decimal, 0-1).
    pub lgd: Decimal,
    /// Exposure at default.
    pub ead: Decimal,
    /// Maturity in years.
    pub maturity: Decimal,
    /// Total available capital.
    pub total_capital: Decimal,
    /// Risk-weighted assets for CAR calculation.
    pub risk_weighted_assets: Decimal,
    /// Stress scenarios (base, adverse, severe).
    #[serde(default)]
    pub stress_scenarios: Vec<StressScenario>,
}

/// Output of the economic capital calculation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EconomicCapitalOutput {
    /// VaR-based economic capital (VaR - EL).
    pub var_capital: Decimal,
    /// Expected Shortfall based capital.
    pub es_capital: Decimal,
    /// IRB capital requirement (Basel formula).
    pub irb_capital: Decimal,
    /// Expected loss = PD * LGD * EAD.
    pub expected_loss: Decimal,
    /// Unexpected loss = VaR - EL.
    pub unexpected_loss: Decimal,
    /// Capital adequacy ratio = total_capital / RWA.
    pub capital_adequacy_ratio: Decimal,
    /// Additional stress capital buffer.
    pub stress_buffer: Decimal,
}

/// Compute economic capital analytics.
pub fn calculate_economic_capital(
    input: &EconomicCapitalInput,
) -> CorpFinanceResult<EconomicCapitalOutput> {
    validate_economic_capital_input(input)?;

    // Sort losses ascending
    let mut sorted_losses = input.portfolio_losses.clone();
    sorted_losses.sort();

    let n = sorted_losses.len();

    // Expected Loss from simulation
    let el_sim: Decimal = sorted_losses.iter().copied().sum::<Decimal>() / Decimal::from(n as u64);

    // VaR at confidence level (quantile)
    let var_index = {
        let idx_decimal = input.confidence_level * Decimal::from(n as u64);
        let idx = idx_decimal
            .to_string()
            .split('.')
            .next()
            .unwrap_or("0")
            .parse::<usize>()
            .unwrap_or(0);
        if idx >= n {
            n - 1
        } else {
            idx
        }
    };
    let var_value = sorted_losses[var_index];

    // VaR-based capital = VaR - EL (if positive)
    let var_capital = if var_value > el_sim {
        var_value - el_sim
    } else {
        Decimal::ZERO
    };

    // Expected Shortfall = average of losses above VaR
    let tail_losses: Vec<Decimal> = sorted_losses
        .iter()
        .filter(|&&l| l >= var_value)
        .copied()
        .collect();
    let es_capital = if tail_losses.is_empty() {
        var_capital
    } else {
        let es_avg =
            tail_losses.iter().copied().sum::<Decimal>() / Decimal::from(tail_losses.len() as u64);
        if es_avg > el_sim {
            es_avg - el_sim
        } else {
            Decimal::ZERO
        }
    };

    // Expected Loss (regulatory)
    let expected_loss = input.pd * input.lgd * input.ead;

    // IRB Capital Requirement
    let irb_capital = calculate_irb_capital(input.pd, input.lgd, input.ead, input.maturity)?;

    // Unexpected loss = IRB capital - EL (or VaR-based)
    let unexpected_loss = var_capital;

    // Capital Adequacy Ratio
    let capital_adequacy_ratio = if input.risk_weighted_assets.is_zero() {
        Decimal::ZERO
    } else {
        input.total_capital / input.risk_weighted_assets
    };

    // Stress buffer: max stress loss - expected loss (capped at zero)
    let stress_buffer = if input.stress_scenarios.is_empty() {
        Decimal::ZERO
    } else {
        let max_stress = input
            .stress_scenarios
            .iter()
            .map(|s| s.loss)
            .max()
            .unwrap_or(Decimal::ZERO);
        if max_stress > expected_loss {
            max_stress - expected_loss
        } else {
            Decimal::ZERO
        }
    };

    Ok(EconomicCapitalOutput {
        var_capital,
        es_capital,
        irb_capital,
        expected_loss,
        unexpected_loss,
        capital_adequacy_ratio,
        stress_buffer,
    })
}

/// Basel IRB capital requirement.
/// K = LGD * [N((N_inv(PD) + sqrt(rho)*N_inv(0.999))/sqrt(1-rho)) - PD] * maturity_adj
fn calculate_irb_capital(
    pd: Decimal,
    lgd: Decimal,
    ead: Decimal,
    maturity: Decimal,
) -> CorpFinanceResult<Decimal> {
    if pd.is_zero() {
        return Ok(Decimal::ZERO);
    }

    // Asset correlation: rho = 0.12*(1-exp(-50*PD))/(1-exp(-50))
    //                       + 0.24*(1 - (1-exp(-50*PD))/(1-exp(-50)))
    let exp_neg50pd = decimal_exp(dec!(-50) * pd);
    let exp_neg50 = decimal_exp(dec!(-50));
    let denom_corr = Decimal::ONE - exp_neg50;
    let ratio = if denom_corr.is_zero() {
        Decimal::ZERO
    } else {
        (Decimal::ONE - exp_neg50pd) / denom_corr
    };
    let rho = dec!(0.12) * ratio + dec!(0.24) * (Decimal::ONE - ratio);

    // Maturity adjustment: b = (0.11852 - 0.05478*ln(PD))^2
    let ln_pd = decimal_ln(pd);
    let b_base = dec!(0.11852) - dec!(0.05478) * ln_pd;
    let b = b_base * b_base;

    // Maturity factor: (1 + (M-2.5)*b) / (1-1.5*b)
    let mat_num = Decimal::ONE + (maturity - dec!(2.5)) * b;
    let mat_den = Decimal::ONE - dec!(1.5) * b;
    let maturity_adj = if mat_den.is_zero() {
        Decimal::ONE
    } else {
        mat_num / mat_den
    };

    // K = LGD * [N((N_inv(PD) + sqrt(rho)*N_inv(0.999))/sqrt(1-rho)) - PD] * maturity_adj
    let n_inv_pd = norm_inv(pd);
    let n_inv_999 = norm_inv(dec!(0.999));
    let sqrt_rho = decimal_sqrt(rho);
    let sqrt_one_minus_rho = decimal_sqrt(Decimal::ONE - rho);

    let z = if sqrt_one_minus_rho.is_zero() {
        Decimal::ZERO
    } else {
        (n_inv_pd + sqrt_rho * n_inv_999) / sqrt_one_minus_rho
    };

    let k_raw = lgd * (norm_cdf(z) - pd) * maturity_adj;
    let k = if k_raw < Decimal::ZERO {
        Decimal::ZERO
    } else {
        k_raw
    };

    Ok(k * ead)
}

fn validate_economic_capital_input(input: &EconomicCapitalInput) -> CorpFinanceResult<()> {
    if input.portfolio_losses.is_empty() {
        return Err(CorpFinanceError::InsufficientData(
            "Portfolio losses must contain at least one scenario.".into(),
        ));
    }
    if input.confidence_level <= Decimal::ZERO || input.confidence_level >= Decimal::ONE {
        return Err(CorpFinanceError::InvalidInput {
            field: "confidence_level".into(),
            reason: "Confidence level must be in (0, 1).".into(),
        });
    }
    if input.pd < Decimal::ZERO || input.pd > Decimal::ONE {
        return Err(CorpFinanceError::InvalidInput {
            field: "pd".into(),
            reason: "Probability of default must be in [0, 1].".into(),
        });
    }
    if input.lgd < Decimal::ZERO || input.lgd > Decimal::ONE {
        return Err(CorpFinanceError::InvalidInput {
            field: "lgd".into(),
            reason: "Loss given default must be in [0, 1].".into(),
        });
    }
    if input.ead < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "ead".into(),
            reason: "Exposure at default must be non-negative.".into(),
        });
    }
    if input.maturity <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "maturity".into(),
            reason: "Maturity must be positive.".into(),
        });
    }
    if input.total_capital < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "total_capital".into(),
            reason: "Total capital must be non-negative.".into(),
        });
    }
    if input.risk_weighted_assets < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "risk_weighted_assets".into(),
            reason: "Risk-weighted assets must be non-negative.".into(),
        });
    }
    Ok(())
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

    fn make_base_input() -> EconomicCapitalInput {
        // 1000 loss scenarios: 0, 1, 2, ..., 999
        let losses: Vec<Decimal> = (0..1000).map(|i| Decimal::from(i)).collect();
        EconomicCapitalInput {
            portfolio_losses: losses,
            confidence_level: dec!(0.999),
            pd: dec!(0.02),
            lgd: dec!(0.45),
            ead: dec!(1_000_000),
            maturity: dec!(3),
            total_capital: dec!(150_000),
            risk_weighted_assets: dec!(1_000_000),
            stress_scenarios: vec![
                StressScenario {
                    name: "Base".into(),
                    loss: dec!(5_000),
                },
                StressScenario {
                    name: "Adverse".into(),
                    loss: dec!(15_000),
                },
                StressScenario {
                    name: "Severe".into(),
                    loss: dec!(30_000),
                },
            ],
        }
    }

    #[test]
    fn test_var_capital_positive() {
        let input = make_base_input();
        let out = calculate_economic_capital(&input).unwrap();
        assert!(out.var_capital > Decimal::ZERO);
    }

    #[test]
    fn test_es_capital_gte_var_capital() {
        let input = make_base_input();
        let out = calculate_economic_capital(&input).unwrap();
        assert!(
            out.es_capital >= out.var_capital,
            "ES {} should be >= VaR {}",
            out.es_capital,
            out.var_capital
        );
    }

    #[test]
    fn test_expected_loss_equals_pd_lgd_ead() {
        let input = make_base_input();
        let out = calculate_economic_capital(&input).unwrap();
        let expected = dec!(0.02) * dec!(0.45) * dec!(1_000_000);
        assert_eq!(out.expected_loss, expected);
    }

    #[test]
    fn test_irb_capital_positive_for_nonzero_pd() {
        let input = make_base_input();
        let out = calculate_economic_capital(&input).unwrap();
        assert!(
            out.irb_capital > Decimal::ZERO,
            "IRB capital should be positive"
        );
    }

    #[test]
    fn test_irb_capital_zero_for_zero_pd() {
        let mut input = make_base_input();
        input.pd = Decimal::ZERO;
        let out = calculate_economic_capital(&input).unwrap();
        assert_eq!(out.irb_capital, Decimal::ZERO);
    }

    #[test]
    fn test_capital_adequacy_ratio() {
        let input = make_base_input();
        let out = calculate_economic_capital(&input).unwrap();
        let expected = dec!(150_000) / dec!(1_000_000);
        assert_eq!(out.capital_adequacy_ratio, expected);
    }

    #[test]
    fn test_capital_adequacy_ratio_zero_rwa() {
        let mut input = make_base_input();
        input.risk_weighted_assets = Decimal::ZERO;
        let out = calculate_economic_capital(&input).unwrap();
        assert_eq!(out.capital_adequacy_ratio, Decimal::ZERO);
    }

    #[test]
    fn test_stress_buffer_positive() {
        let input = make_base_input();
        let out = calculate_economic_capital(&input).unwrap();
        // Severe loss 30000 > EL 9000, so buffer > 0
        assert!(
            out.stress_buffer > Decimal::ZERO,
            "Stress buffer should be positive"
        );
    }

    #[test]
    fn test_stress_buffer_is_max_minus_el() {
        let input = make_base_input();
        let out = calculate_economic_capital(&input).unwrap();
        let expected_buffer = dec!(30_000) - out.expected_loss;
        assert_eq!(out.stress_buffer, expected_buffer);
    }

    #[test]
    fn test_stress_buffer_zero_when_no_scenarios() {
        let mut input = make_base_input();
        input.stress_scenarios = vec![];
        let out = calculate_economic_capital(&input).unwrap();
        assert_eq!(out.stress_buffer, Decimal::ZERO);
    }

    #[test]
    fn test_unexpected_loss_equals_var_capital() {
        let input = make_base_input();
        let out = calculate_economic_capital(&input).unwrap();
        assert_eq!(out.unexpected_loss, out.var_capital);
    }

    #[test]
    fn test_higher_confidence_higher_var() {
        let mut input_low = make_base_input();
        input_low.confidence_level = dec!(0.95);
        let out_low = calculate_economic_capital(&input_low).unwrap();

        let input_high = make_base_input();
        let out_high = calculate_economic_capital(&input_high).unwrap();

        assert!(
            out_high.var_capital >= out_low.var_capital,
            "Higher confidence {} should produce higher VaR than lower {}",
            out_high.var_capital,
            out_low.var_capital
        );
    }

    #[test]
    fn test_higher_lgd_higher_irb() {
        let input_low = make_base_input();
        let out_low = calculate_economic_capital(&input_low).unwrap();

        let mut input_high = make_base_input();
        input_high.lgd = dec!(0.80);
        let out_high = calculate_economic_capital(&input_high).unwrap();

        assert!(
            out_high.irb_capital > out_low.irb_capital,
            "Higher LGD should produce higher IRB capital"
        );
    }

    #[test]
    fn test_higher_ead_higher_el() {
        let input_base = make_base_input();
        let out_base = calculate_economic_capital(&input_base).unwrap();

        let mut input_high = make_base_input();
        input_high.ead = dec!(2_000_000);
        let out_high = calculate_economic_capital(&input_high).unwrap();

        assert!(out_high.expected_loss > out_base.expected_loss);
    }

    #[test]
    fn test_single_loss_scenario() {
        let input = EconomicCapitalInput {
            portfolio_losses: vec![dec!(100)],
            confidence_level: dec!(0.99),
            pd: dec!(0.01),
            lgd: dec!(0.40),
            ead: dec!(500_000),
            maturity: dec!(1),
            total_capital: dec!(50_000),
            risk_weighted_assets: dec!(500_000),
            stress_scenarios: vec![],
        };
        let out = calculate_economic_capital(&input).unwrap();
        assert!(out.var_capital >= Decimal::ZERO);
    }

    #[test]
    fn test_uniform_losses_expected_value() {
        let input = make_base_input();
        let out = calculate_economic_capital(&input).unwrap();
        // Mean of 0..999 = 499.5
        let el_sim = dec!(499.5);
        assert!(
            approx_eq(out.var_capital + el_sim, dec!(999), dec!(1)),
            "VaR + EL should be ~999"
        );
    }

    #[test]
    fn test_irb_capital_proportional_to_ead() {
        let input1 = make_base_input();
        let out1 = calculate_economic_capital(&input1).unwrap();

        let mut input2 = make_base_input();
        input2.ead = dec!(2_000_000);
        let out2 = calculate_economic_capital(&input2).unwrap();

        // IRB capital should roughly double
        let ratio = out2.irb_capital / out1.irb_capital;
        assert!(
            approx_eq(ratio, dec!(2), dec!(0.01)),
            "IRB ratio should be ~2, got {}",
            ratio
        );
    }

    // -- Validation tests --

    #[test]
    fn test_reject_empty_losses() {
        let mut input = make_base_input();
        input.portfolio_losses = vec![];
        assert!(calculate_economic_capital(&input).is_err());
    }

    #[test]
    fn test_reject_confidence_zero() {
        let mut input = make_base_input();
        input.confidence_level = Decimal::ZERO;
        assert!(calculate_economic_capital(&input).is_err());
    }

    #[test]
    fn test_reject_confidence_one() {
        let mut input = make_base_input();
        input.confidence_level = Decimal::ONE;
        assert!(calculate_economic_capital(&input).is_err());
    }

    #[test]
    fn test_reject_negative_pd() {
        let mut input = make_base_input();
        input.pd = dec!(-0.01);
        assert!(calculate_economic_capital(&input).is_err());
    }

    #[test]
    fn test_reject_lgd_above_one() {
        let mut input = make_base_input();
        input.lgd = dec!(1.5);
        assert!(calculate_economic_capital(&input).is_err());
    }

    #[test]
    fn test_reject_negative_ead() {
        let mut input = make_base_input();
        input.ead = dec!(-100);
        assert!(calculate_economic_capital(&input).is_err());
    }

    #[test]
    fn test_reject_zero_maturity() {
        let mut input = make_base_input();
        input.maturity = Decimal::ZERO;
        assert!(calculate_economic_capital(&input).is_err());
    }

    #[test]
    fn test_reject_negative_capital() {
        let mut input = make_base_input();
        input.total_capital = dec!(-1);
        assert!(calculate_economic_capital(&input).is_err());
    }

    #[test]
    fn test_reject_negative_rwa() {
        let mut input = make_base_input();
        input.risk_weighted_assets = dec!(-1);
        assert!(calculate_economic_capital(&input).is_err());
    }

    #[test]
    fn test_serialization_roundtrip() {
        let input = make_base_input();
        let out = calculate_economic_capital(&input).unwrap();
        let json = serde_json::to_string(&out).unwrap();
        let _: EconomicCapitalOutput = serde_json::from_str(&json).unwrap();
    }

    #[test]
    fn test_norm_cdf_at_zero() {
        let result = norm_cdf(Decimal::ZERO);
        assert!(
            approx_eq(result, dec!(0.5), dec!(0.001)),
            "N(0) should be ~0.5, got {}",
            result
        );
    }

    #[test]
    fn test_norm_cdf_monotone() {
        let a = norm_cdf(dec!(-1));
        let b = norm_cdf(Decimal::ZERO);
        let c = norm_cdf(dec!(1));
        assert!(a < b && b < c);
    }

    #[test]
    fn test_norm_inv_at_half() {
        let result = norm_inv(dec!(0.5));
        assert!(
            approx_eq(result, Decimal::ZERO, dec!(0.01)),
            "N_inv(0.5) should be ~0, got {}",
            result
        );
    }
}
