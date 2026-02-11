use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

use crate::types::*;
use crate::{CorpFinanceError, CorpFinanceResult};

// ---------------------------------------------------------------------------
// Input / Output types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExposurePoint {
    pub time_years: Decimal,
    pub expected_exposure: Money,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub potential_future_exposure: Option<Money>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CvaInput {
    pub trade_description: String,
    /// Time-bucketed expected positive exposure (EPE)
    pub expected_exposure_profile: Vec<ExposurePoint>,
    /// Annual PD of counterparty
    pub counterparty_default_probability: Rate,
    /// Counterparty recovery rate
    pub counterparty_recovery_rate: Rate,
    /// Own PD for DVA (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub own_default_probability: Option<Rate>,
    /// Own recovery rate (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub own_recovery_rate: Option<Rate>,
    /// Risk-free discount rate
    pub risk_free_rate: Rate,
    /// Reduction in exposure from netting (e.g. 0.30 = 30% reduction)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub netting_benefit: Option<Rate>,
    /// Collateral posting threshold (exposure above this is collateralised)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub collateral_threshold: Option<Money>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdjustedExposure {
    pub time_years: Decimal,
    pub gross_exposure: Money,
    pub net_exposure: Money,
    pub collateralised_exposure: Money,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CvaRiskMetrics {
    pub counterparty_lgd: Rate,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub own_lgd: Option<Rate>,
    pub weighted_average_exposure: Money,
    pub effective_maturity: Decimal,
    pub exposure_reduction_pct: Rate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CvaOutput {
    pub trade_description: String,
    /// Unilateral CVA
    pub unilateral_cva: Money,
    /// DVA if own PD provided
    pub dva: Money,
    /// CVA - DVA
    pub bilateral_cva: Money,
    /// CVA expressed as running spread in bps
    pub cva_as_spread_bps: Decimal,
    /// Peak expected exposure
    pub exposure_at_default: Money,
    /// Expected credit loss
    pub expected_loss: Money,
    /// Exposure after netting and collateral
    pub adjusted_exposure_profile: Vec<AdjustedExposure>,
    pub risk_metrics: CvaRiskMetrics,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Calculate Credit Valuation Adjustment (CVA) and Debit Valuation Adjustment (DVA).
///
/// Computes unilateral and bilateral CVA using a discrete marginal-default-probability
/// framework, with optional netting and collateral adjustments.
pub fn calculate_cva(input: &CvaInput) -> CorpFinanceResult<CvaOutput> {
    validate_cva_input(input)?;

    let cpd = input.counterparty_default_probability;
    let c_recovery = input.counterparty_recovery_rate;
    let c_lgd = Decimal::ONE - c_recovery;

    let netting = input.netting_benefit.unwrap_or(Decimal::ZERO);
    let collateral_threshold = input.collateral_threshold;

    // Build adjusted exposure profile
    let mut adjusted_exposures: Vec<AdjustedExposure> = Vec::new();
    let mut total_gross = Decimal::ZERO;
    let mut total_collateralised = Decimal::ZERO;

    for ep in &input.expected_exposure_profile {
        let gross = ep.expected_exposure;
        let net = gross * (Decimal::ONE - netting);
        let collateralised = match collateral_threshold {
            Some(threshold) => {
                if net > threshold {
                    net - threshold
                } else {
                    Decimal::ZERO
                }
            }
            None => net,
        };

        total_gross += gross;
        total_collateralised += collateralised;

        adjusted_exposures.push(AdjustedExposure {
            time_years: ep.time_years,
            gross_exposure: gross,
            net_exposure: net,
            collateralised_exposure: collateralised,
        });
    }

    // Compute CVA using discrete marginal default probabilities
    // Survival prob S(t) = (1 - PD)^t via iterative multiplication
    // We need to handle arbitrary time points, so compute S(t) for each point.
    let mut unilateral_cva = Decimal::ZERO;
    let mut dva = Decimal::ZERO;

    let mut exposure_time_sum = Decimal::ZERO;
    let mut exposure_discount_sum = Decimal::ZERO;

    let n = adjusted_exposures.len();
    let mut peak_exposure = Decimal::ZERO;

    // Process each exposure bucket
    // For time t_i, marginal PD = S(t_{i-1}) - S(t_i)
    // Discount factor D(t) = 1/(1+r)^t computed iteratively

    let mut prev_time = Decimal::ZERO;
    let mut prev_c_survival = Decimal::ONE;
    let mut prev_o_survival = Decimal::ONE;

    for adj in &adjusted_exposures {
        let t = adj.time_years;
        let dt = t - prev_time;
        let epe = adj.collateralised_exposure;

        // Track peak
        if epe > peak_exposure {
            peak_exposure = epe;
        }

        // Counterparty survival at t: S_c(t) = S_c(t-1) * (1 - PD * dt)
        // For discrete annual PD over fractional periods
        let c_survival_t = prev_c_survival * (Decimal::ONE - cpd * dt).max(Decimal::ZERO);

        // Marginal default probability for this bucket
        let marginal_c_pd = prev_c_survival - c_survival_t;

        // Discount factor: 1/(1+r)^t
        let df = discount_factor_at(input.risk_free_rate, t);

        // CVA contribution: LGD_c * marginal_PD_c * D(t) * EPE(t)
        unilateral_cva += c_lgd * marginal_c_pd * df * epe;

        // DVA contribution (if own PD provided)
        if let Some(own_pd) = input.own_default_probability {
            let own_lgd = Decimal::ONE - input.own_recovery_rate.unwrap_or(dec!(0.40));
            let o_survival_t = prev_o_survival * (Decimal::ONE - own_pd * dt).max(Decimal::ZERO);
            let marginal_o_pd = prev_o_survival - o_survival_t;
            // Use EPE as proxy for ENE (expected negative exposure)
            dva += own_lgd * marginal_o_pd * df * epe;
            prev_o_survival = o_survival_t;
        }

        // Weighted average exposure and effective maturity accumulators
        exposure_time_sum += t * epe * df;
        exposure_discount_sum += epe * df;

        prev_c_survival = c_survival_t;
        prev_time = t;
    }

    let bilateral_cva = unilateral_cva - dva;

    // Expected loss = CVA (it is the expected credit loss)
    let expected_loss = unilateral_cva;

    // Effective maturity = Sigma(t * EPE(t) * D(t)) / Sigma(EPE(t) * D(t))
    let effective_maturity = if exposure_discount_sum.is_zero() {
        Decimal::ZERO
    } else {
        exposure_time_sum / exposure_discount_sum
    };

    // Weighted average exposure
    let n_points = Decimal::from(n as u32);
    let weighted_average_exposure = if n_points.is_zero() {
        Decimal::ZERO
    } else {
        total_collateralised / n_points
    };

    // CVA as running spread (bps):
    // CVA_spread = CVA / (risky_annuity * average_exposure) * 10000
    // Approximate risky annuity as effective_maturity (simplified)
    let cva_as_spread_bps = if effective_maturity.is_zero() || weighted_average_exposure.is_zero() {
        Decimal::ZERO
    } else {
        unilateral_cva / (effective_maturity * weighted_average_exposure) * dec!(10000)
    };

    // Exposure reduction percentage
    let exposure_reduction_pct = if total_gross.is_zero() {
        Decimal::ZERO
    } else {
        (total_gross - total_collateralised) / total_gross
    };

    let own_lgd_val = input
        .own_default_probability
        .map(|_| Decimal::ONE - input.own_recovery_rate.unwrap_or(dec!(0.40)));

    let risk_metrics = CvaRiskMetrics {
        counterparty_lgd: c_lgd,
        own_lgd: own_lgd_val,
        weighted_average_exposure,
        effective_maturity,
        exposure_reduction_pct,
    };

    Ok(CvaOutput {
        trade_description: input.trade_description.clone(),
        unilateral_cva,
        dva,
        bilateral_cva,
        cva_as_spread_bps,
        exposure_at_default: peak_exposure,
        expected_loss,
        adjusted_exposure_profile: adjusted_exposures,
        risk_metrics,
    })
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

fn validate_cva_input(input: &CvaInput) -> CorpFinanceResult<()> {
    if input.expected_exposure_profile.is_empty() {
        return Err(CorpFinanceError::InvalidInput {
            field: "expected_exposure_profile".into(),
            reason: "At least one exposure point is required.".into(),
        });
    }
    if input.counterparty_default_probability < Decimal::ZERO
        || input.counterparty_default_probability >= Decimal::ONE
    {
        return Err(CorpFinanceError::InvalidInput {
            field: "counterparty_default_probability".into(),
            reason: "Counterparty default probability must be in [0, 1).".into(),
        });
    }
    if input.counterparty_recovery_rate < Decimal::ZERO
        || input.counterparty_recovery_rate >= Decimal::ONE
    {
        return Err(CorpFinanceError::InvalidInput {
            field: "counterparty_recovery_rate".into(),
            reason: "Counterparty recovery rate must be in [0, 1).".into(),
        });
    }
    if let Some(own_pd) = input.own_default_probability {
        if own_pd < Decimal::ZERO || own_pd >= Decimal::ONE {
            return Err(CorpFinanceError::InvalidInput {
                field: "own_default_probability".into(),
                reason: "Own default probability must be in [0, 1).".into(),
            });
        }
    }
    if let Some(own_rr) = input.own_recovery_rate {
        if own_rr < Decimal::ZERO || own_rr >= Decimal::ONE {
            return Err(CorpFinanceError::InvalidInput {
                field: "own_recovery_rate".into(),
                reason: "Own recovery rate must be in [0, 1).".into(),
            });
        }
    }
    if input.risk_free_rate < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "risk_free_rate".into(),
            reason: "Risk-free rate must be non-negative.".into(),
        });
    }
    if let Some(nb) = input.netting_benefit {
        if nb < Decimal::ZERO || nb > Decimal::ONE {
            return Err(CorpFinanceError::InvalidInput {
                field: "netting_benefit".into(),
                reason: "Netting benefit must be in [0, 1].".into(),
            });
        }
    }
    for (i, ep) in input.expected_exposure_profile.iter().enumerate() {
        if ep.expected_exposure < Decimal::ZERO {
            return Err(CorpFinanceError::InvalidInput {
                field: format!("expected_exposure_profile[{}].expected_exposure", i),
                reason: "Expected exposure must be non-negative.".into(),
            });
        }
        if ep.time_years < Decimal::ZERO {
            return Err(CorpFinanceError::InvalidInput {
                field: format!("expected_exposure_profile[{}].time_years", i),
                reason: "Time must be non-negative.".into(),
            });
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Math helpers
// ---------------------------------------------------------------------------

/// Compute discount factor at time t: D(t) = 1 / (1+r)^t.
///
/// For integer years, use iterative multiplication. For fractional years,
/// decompose into integer part (iterative) and fractional part (nth root).
fn discount_factor_at(rate: Rate, t: Decimal) -> Decimal {
    if t.is_zero() {
        return Decimal::ONE;
    }
    if rate.is_zero() {
        return Decimal::ONE;
    }

    let one_plus_r = Decimal::ONE + rate;

    // Decompose t into integer and fractional parts
    let t_floor = t.floor();
    let t_frac = t - t_floor;
    let years_int = t_floor.to_string().parse::<u32>().unwrap_or(0);

    // (1+r)^integer_years via iterative multiplication
    let mut compound = Decimal::ONE;
    for _ in 0..years_int {
        compound *= one_plus_r;
    }

    // Fractional part: (1+r)^frac
    // Approximate using linear interpolation for small fractions:
    // (1+r)^f ~= 1 + r*f for small f (first-order Taylor)
    // For better accuracy, use nth_root approach when frac is a known fraction.
    if t_frac > Decimal::ZERO {
        // Use the approximation (1+r)^f ~= 1 + r*f + r*f*(f-1)/2 (second-order)
        let rf = rate * t_frac;
        let frac_compound = Decimal::ONE + rf + rf * (t_frac - Decimal::ONE) / dec!(2);
        compound *= frac_compound;
    }

    if compound.is_zero() {
        return Decimal::ONE;
    }

    Decimal::ONE / compound
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn basic_exposure_profile() -> Vec<ExposurePoint> {
        vec![
            ExposurePoint {
                time_years: dec!(1),
                expected_exposure: dec!(5_000_000),
                potential_future_exposure: Some(dec!(7_000_000)),
            },
            ExposurePoint {
                time_years: dec!(2),
                expected_exposure: dec!(4_500_000),
                potential_future_exposure: Some(dec!(6_500_000)),
            },
            ExposurePoint {
                time_years: dec!(3),
                expected_exposure: dec!(4_000_000),
                potential_future_exposure: Some(dec!(5_500_000)),
            },
            ExposurePoint {
                time_years: dec!(4),
                expected_exposure: dec!(3_000_000),
                potential_future_exposure: None,
            },
            ExposurePoint {
                time_years: dec!(5),
                expected_exposure: dec!(2_000_000),
                potential_future_exposure: None,
            },
        ]
    }

    fn basic_cva_input() -> CvaInput {
        CvaInput {
            trade_description: "5Y interest rate swap".to_string(),
            expected_exposure_profile: basic_exposure_profile(),
            counterparty_default_probability: dec!(0.02),
            counterparty_recovery_rate: dec!(0.40),
            own_default_probability: None,
            own_recovery_rate: None,
            risk_free_rate: dec!(0.05),
            netting_benefit: None,
            collateral_threshold: None,
        }
    }

    #[test]
    fn test_basic_cva() {
        let input = basic_cva_input();
        let result = calculate_cva(&input).unwrap();

        assert_eq!(result.trade_description, "5Y interest rate swap");
        assert!(
            result.unilateral_cva > Decimal::ZERO,
            "CVA should be positive"
        );
        assert_eq!(
            result.dva,
            Decimal::ZERO,
            "DVA should be zero without own PD"
        );
        assert_eq!(result.bilateral_cva, result.unilateral_cva);
    }

    #[test]
    fn test_cva_positive() {
        let input = basic_cva_input();
        let result = calculate_cva(&input).unwrap();

        // CVA = LGD * sum of marginal_PD * DF * EPE
        // With PD=2%, LGD=60%, 5yr exposure profile, CVA should be meaningful
        assert!(result.unilateral_cva > dec!(10_000), "CVA seems too small");
        assert!(
            result.unilateral_cva < dec!(1_000_000),
            "CVA seems too large for 2% PD"
        );
    }

    #[test]
    fn test_bilateral_cva_with_dva() {
        let mut input = basic_cva_input();
        input.own_default_probability = Some(dec!(0.01));
        input.own_recovery_rate = Some(dec!(0.40));

        let result = calculate_cva(&input).unwrap();

        assert!(result.dva > Decimal::ZERO, "DVA should be positive");
        assert!(
            result.bilateral_cva < result.unilateral_cva,
            "Bilateral CVA should be less than unilateral (DVA offsets)"
        );
        assert_eq!(
            result.bilateral_cva,
            result.unilateral_cva - result.dva,
            "Bilateral = Unilateral - DVA"
        );
    }

    #[test]
    fn test_dva_less_than_cva_when_own_pd_lower() {
        let mut input = basic_cva_input();
        input.own_default_probability = Some(dec!(0.01)); // own PD < counterparty PD
        input.own_recovery_rate = Some(dec!(0.40));

        let result = calculate_cva(&input).unwrap();

        // Own PD (1%) < counterparty PD (2%), same LGD => DVA < CVA
        assert!(
            result.dva < result.unilateral_cva,
            "DVA should be less than CVA when own PD < counterparty PD"
        );
    }

    #[test]
    fn test_netting_benefit() {
        let input_no_netting = basic_cva_input();

        let mut input_with_netting = basic_cva_input();
        input_with_netting.netting_benefit = Some(dec!(0.30)); // 30% reduction

        let result_no = calculate_cva(&input_no_netting).unwrap();
        let result_net = calculate_cva(&input_with_netting).unwrap();

        assert!(
            result_net.unilateral_cva < result_no.unilateral_cva,
            "Netting should reduce CVA"
        );

        // Check adjusted exposure profile
        for adj in &result_net.adjusted_exposure_profile {
            assert!(
                adj.net_exposure <= adj.gross_exposure,
                "Net exposure should be <= gross"
            );
        }
    }

    #[test]
    fn test_collateral_threshold() {
        let mut input = basic_cva_input();
        input.collateral_threshold = Some(dec!(3_000_000)); // 3M threshold

        let result = calculate_cva(&input).unwrap();

        // Exposure above 3M threshold is the collateralised amount
        for adj in &result.adjusted_exposure_profile {
            if adj.net_exposure > dec!(3_000_000) {
                assert!(
                    adj.collateralised_exposure > Decimal::ZERO,
                    "Should have positive collateralised exposure above threshold"
                );
                let expected = adj.net_exposure - dec!(3_000_000);
                assert_eq!(adj.collateralised_exposure, expected);
            } else {
                assert_eq!(
                    adj.collateralised_exposure,
                    Decimal::ZERO,
                    "Below threshold, collateralised exposure should be zero"
                );
            }
        }
    }

    #[test]
    fn test_collateral_reduces_cva() {
        let input_no_collateral = basic_cva_input();

        let mut input_collateral = basic_cva_input();
        input_collateral.collateral_threshold = Some(dec!(2_000_000));

        let result_no = calculate_cva(&input_no_collateral).unwrap();
        let result_col = calculate_cva(&input_collateral).unwrap();

        assert!(
            result_col.unilateral_cva < result_no.unilateral_cva,
            "Collateral should reduce CVA"
        );
    }

    #[test]
    fn test_netting_and_collateral_combined() {
        let mut input = basic_cva_input();
        input.netting_benefit = Some(dec!(0.30));
        input.collateral_threshold = Some(dec!(2_000_000));

        let result = calculate_cva(&input).unwrap();

        let no_mitigation = calculate_cva(&basic_cva_input()).unwrap();
        assert!(
            result.unilateral_cva < no_mitigation.unilateral_cva,
            "Combined netting + collateral should reduce CVA"
        );
    }

    #[test]
    fn test_cva_spread_positive() {
        let input = basic_cva_input();
        let result = calculate_cva(&input).unwrap();

        assert!(
            result.cva_as_spread_bps > Decimal::ZERO,
            "CVA spread should be positive"
        );
    }

    #[test]
    fn test_exposure_at_default() {
        let input = basic_cva_input();
        let result = calculate_cva(&input).unwrap();

        // Peak exposure should be the max of the exposure profile
        assert_eq!(
            result.exposure_at_default,
            dec!(5_000_000),
            "Peak exposure should be 5M"
        );
    }

    #[test]
    fn test_effective_maturity() {
        let input = basic_cva_input();
        let result = calculate_cva(&input).unwrap();

        // Effective maturity should be between 1 and 5 (weighted by exposure)
        assert!(
            result.risk_metrics.effective_maturity > dec!(1),
            "Effective maturity should be > 1"
        );
        assert!(
            result.risk_metrics.effective_maturity < dec!(5),
            "Effective maturity should be < 5"
        );
    }

    #[test]
    fn test_risk_metrics_lgd() {
        let input = basic_cva_input();
        let result = calculate_cva(&input).unwrap();

        assert_eq!(result.risk_metrics.counterparty_lgd, dec!(0.60));
        assert!(result.risk_metrics.own_lgd.is_none());
    }

    #[test]
    fn test_risk_metrics_own_lgd_with_dva() {
        let mut input = basic_cva_input();
        input.own_default_probability = Some(dec!(0.01));
        input.own_recovery_rate = Some(dec!(0.50));

        let result = calculate_cva(&input).unwrap();
        assert_eq!(result.risk_metrics.own_lgd, Some(dec!(0.50)));
    }

    #[test]
    fn test_single_exposure_point() {
        let input = CvaInput {
            trade_description: "Short-dated trade".to_string(),
            expected_exposure_profile: vec![ExposurePoint {
                time_years: dec!(0.5),
                expected_exposure: dec!(1_000_000),
                potential_future_exposure: None,
            }],
            counterparty_default_probability: dec!(0.05),
            counterparty_recovery_rate: dec!(0.40),
            own_default_probability: None,
            own_recovery_rate: None,
            risk_free_rate: dec!(0.03),
            netting_benefit: None,
            collateral_threshold: None,
        };

        let result = calculate_cva(&input).unwrap();
        assert!(result.unilateral_cva > Decimal::ZERO);
        assert_eq!(result.adjusted_exposure_profile.len(), 1);
    }

    #[test]
    fn test_zero_pd_zero_cva() {
        let mut input = basic_cva_input();
        input.counterparty_default_probability = Decimal::ZERO;

        let result = calculate_cva(&input).unwrap();
        assert_eq!(
            result.unilateral_cva,
            Decimal::ZERO,
            "Zero PD should yield zero CVA"
        );
    }

    #[test]
    fn test_higher_pd_higher_cva() {
        let input_low = basic_cva_input(); // PD = 2%

        let mut input_high = basic_cva_input();
        input_high.counterparty_default_probability = dec!(0.10); // PD = 10%

        let result_low = calculate_cva(&input_low).unwrap();
        let result_high = calculate_cva(&input_high).unwrap();

        assert!(
            result_high.unilateral_cva > result_low.unilateral_cva,
            "Higher PD should yield higher CVA"
        );
    }

    #[test]
    fn test_higher_lgd_higher_cva() {
        let input_low_lgd = basic_cva_input(); // recovery = 0.40, LGD = 0.60

        let mut input_high_lgd = basic_cva_input();
        input_high_lgd.counterparty_recovery_rate = dec!(0.20); // LGD = 0.80

        let result_low = calculate_cva(&input_low_lgd).unwrap();
        let result_high = calculate_cva(&input_high_lgd).unwrap();

        assert!(
            result_high.unilateral_cva > result_low.unilateral_cva,
            "Higher LGD should yield higher CVA"
        );
    }

    #[test]
    fn test_exposure_reduction_pct() {
        let mut input = basic_cva_input();
        input.netting_benefit = Some(dec!(0.50)); // 50% netting
        input.collateral_threshold = Some(dec!(1_000_000));

        let result = calculate_cva(&input).unwrap();

        assert!(
            result.risk_metrics.exposure_reduction_pct > Decimal::ZERO,
            "Should have positive exposure reduction"
        );
        assert!(
            result.risk_metrics.exposure_reduction_pct <= Decimal::ONE,
            "Reduction cannot exceed 100%"
        );
    }

    // -- Validation tests --

    #[test]
    fn test_empty_exposure_profile() {
        let mut input = basic_cva_input();
        input.expected_exposure_profile = vec![];
        let err = calculate_cva(&input).unwrap_err();
        match err {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "expected_exposure_profile")
            }
            other => panic!("Expected InvalidInput, got {other:?}"),
        }
    }

    #[test]
    fn test_invalid_counterparty_pd() {
        let mut input = basic_cva_input();
        input.counterparty_default_probability = dec!(1.0);
        let err = calculate_cva(&input).unwrap_err();
        match err {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "counterparty_default_probability")
            }
            other => panic!("Expected InvalidInput, got {other:?}"),
        }
    }

    #[test]
    fn test_invalid_counterparty_recovery() {
        let mut input = basic_cva_input();
        input.counterparty_recovery_rate = dec!(-0.1);
        let err = calculate_cva(&input).unwrap_err();
        match err {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "counterparty_recovery_rate")
            }
            other => panic!("Expected InvalidInput, got {other:?}"),
        }
    }

    #[test]
    fn test_invalid_own_pd() {
        let mut input = basic_cva_input();
        input.own_default_probability = Some(dec!(1.5));
        let err = calculate_cva(&input).unwrap_err();
        match err {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "own_default_probability")
            }
            other => panic!("Expected InvalidInput, got {other:?}"),
        }
    }

    #[test]
    fn test_invalid_netting_benefit() {
        let mut input = basic_cva_input();
        input.netting_benefit = Some(dec!(1.5));
        let err = calculate_cva(&input).unwrap_err();
        match err {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "netting_benefit")
            }
            other => panic!("Expected InvalidInput, got {other:?}"),
        }
    }

    #[test]
    fn test_negative_exposure_rejected() {
        let mut input = basic_cva_input();
        input.expected_exposure_profile[0].expected_exposure = dec!(-100);
        let err = calculate_cva(&input).unwrap_err();
        match err {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert!(field.contains("expected_exposure"))
            }
            other => panic!("Expected InvalidInput, got {other:?}"),
        }
    }

    #[test]
    fn test_discount_factor_helper() {
        // D(0) = 1
        assert_eq!(discount_factor_at(dec!(0.05), Decimal::ZERO), Decimal::ONE);

        // D(1) at 5% = 1/1.05
        let df1 = discount_factor_at(dec!(0.05), dec!(1));
        let expected = Decimal::ONE / dec!(1.05);
        let diff = (df1 - expected).abs();
        assert!(diff < dec!(0.0001), "DF(1) should be ~0.9524, got {}", df1);

        // D(2) at 5% = 1/1.1025
        let df2 = discount_factor_at(dec!(0.05), dec!(2));
        let expected2 = Decimal::ONE / dec!(1.1025);
        let diff2 = (df2 - expected2).abs();
        assert!(diff2 < dec!(0.0001), "DF(2) should be ~0.9070, got {}", df2);
    }
}
