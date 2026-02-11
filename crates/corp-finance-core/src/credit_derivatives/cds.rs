use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

use crate::types::*;
use crate::{CorpFinanceError, CorpFinanceResult};

// ---------------------------------------------------------------------------
// Input / Output types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CdsInput {
    pub reference_entity: String,
    pub notional: Money,
    /// CDS spread in basis points (market quoted)
    pub spread_bps: Decimal,
    /// Expected recovery rate (e.g. 0.40)
    pub recovery_rate: Rate,
    /// Risk-free discount rate
    pub risk_free_rate: Rate,
    /// CDS tenor in years (1-30)
    pub maturity_years: u32,
    /// Premium payments per year (1, 2, or 4)
    pub payment_frequency: u32,
    /// Annual hazard rate / default probability. If None, implied from spread.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_probability: Option<Rate>,
    /// Market spread for MTM calculation. If different from spread_bps, calculate mark-to-market.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub market_spread_bps: Option<Decimal>,
    /// Counterparty credit rating for risk assessment
    #[serde(skip_serializing_if = "Option::is_none")]
    pub counterparty_rating: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurvivalPoint {
    pub year: u32,
    pub survival_probability: Rate,
    pub cumulative_default_probability: Rate,
    pub discount_factor: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreditTriangle {
    pub spread_bps: Decimal,
    pub default_probability: Rate,
    pub recovery_rate: Rate,
    pub loss_given_default: Rate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CdsOutput {
    pub reference_entity: String,
    pub notional: Money,
    pub spread_bps: Decimal,
    /// Annual premium payment (notional * spread / 10000)
    pub annual_premium: Money,
    /// Implied from spread if not provided: spread_bps / (10000 * (1 - recovery_rate))
    pub implied_default_probability: Rate,
    /// Year-by-year survival probabilities
    pub survival_probabilities: Vec<SurvivalPoint>,
    /// Present value of 1bp of premium (risky annuity)
    pub risky_pv01: Decimal,
    /// PV of protection payments
    pub protection_leg_pv: Money,
    /// PV of premium payments at the quoted spread
    pub premium_leg_pv: Money,
    /// MTM / upfront = (market_spread - contract_spread) * risky_pv01 * notional / 10000
    pub upfront_payment: Money,
    /// Same as upfront if market_spread given, else zero
    pub mark_to_market: Money,
    /// Fair spread = protection_leg_pv / risky_pv01 * 10000
    pub breakeven_spread_bps: Decimal,
    /// Dollar value of 1bp spread change
    pub dv01: Money,
    /// Loss given immediate default = notional * (1 - recovery_rate) - accrued
    pub jump_to_default: Money,
    /// Implied credit metrics
    pub credit_triangle: CreditTriangle,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Price a single-name credit default swap.
///
/// Calculates premium and protection leg PVs using a discrete hazard-rate
/// model, producing survival curves, risky PV01, mark-to-market, DV01,
/// jump-to-default exposure, and breakeven spread.
pub fn price_cds(input: &CdsInput) -> CorpFinanceResult<CdsOutput> {
    validate_cds_input(input)?;

    let lgd = Decimal::ONE - input.recovery_rate;

    // Implied default probability (hazard rate)
    let lambda = match input.default_probability {
        Some(pd) => pd,
        None => {
            // Credit triangle: spread = PD * LGD * 10000
            // => PD = spread / (10000 * LGD)
            if lgd.is_zero() {
                return Err(CorpFinanceError::DivisionByZero {
                    context: "Cannot imply default probability with zero LGD".to_string(),
                });
            }
            input.spread_bps / (dec!(10000) * lgd)
        }
    };

    // Build survival curve, discount factors, risky PV01, and protection leg PV
    let periods_per_year = input.payment_frequency;
    let total_periods = input.maturity_years * periods_per_year;
    let dt = Decimal::ONE / Decimal::from(periods_per_year);

    // Survival factor per period: (1 - lambda)^dt
    // For discrete model with sub-annual periods, use iterative approach.
    // Per-period survival = (1 - lambda * dt) clamped to [0, 1]
    // This is a first-order approximation suitable for small dt * lambda.
    let per_period_survival = (Decimal::ONE - lambda * dt).max(Decimal::ZERO);

    let mut survival_prob = Decimal::ONE;
    let mut discount_factor = Decimal::ONE;
    let one_plus_r = Decimal::ONE + input.risk_free_rate;

    // Per-period discount factor: 1 / (1+r)^dt
    // Compute iteratively: each period multiply by 1/(1 + r*dt) for simple,
    // or use per_period_discount = 1/(1+r)^(1/freq).
    // We use iterative multiplication to avoid powd.
    // For fractional powers, we approximate: (1+r)^(1/n) via Newton's method.
    let per_period_discount = nth_root(one_plus_r, periods_per_year);
    let per_period_discount_inv = if per_period_discount.is_zero() {
        Decimal::ONE
    } else {
        Decimal::ONE / per_period_discount
    };

    let mut risky_pv01 = Decimal::ZERO;
    let mut protection_leg = Decimal::ZERO;

    // Collect annual survival points
    let mut survival_points: Vec<SurvivalPoint> = Vec::new();

    for period in 1..=total_periods {
        let prev_s = survival_prob;
        survival_prob *= per_period_survival;
        discount_factor *= per_period_discount_inv;

        // Risky PV01: sum of dt * S(t) * D(t)
        risky_pv01 += dt * survival_prob * discount_factor;

        // Protection leg: LGD * (S(t-1) - S(t)) * D(t)
        let marginal_default = prev_s - survival_prob;
        protection_leg += lgd * marginal_default * discount_factor;

        // Record annual survival points
        if period % periods_per_year == 0 {
            let year = period / periods_per_year;
            let cum_default = Decimal::ONE - survival_prob;
            survival_points.push(SurvivalPoint {
                year,
                survival_probability: survival_prob,
                cumulative_default_probability: cum_default,
                discount_factor,
            });
        }
    }

    // Handle case where last period doesn't align with a year boundary
    if !total_periods.is_multiple_of(periods_per_year) {
        let year = input.maturity_years;
        let cum_default = Decimal::ONE - survival_prob;
        survival_points.push(SurvivalPoint {
            year,
            survival_probability: survival_prob,
            cumulative_default_probability: cum_default,
            discount_factor,
        });
    }

    // Protection leg PV (notional-scaled)
    let protection_leg_pv = protection_leg * input.notional;

    // Premium leg PV at quoted spread
    let spread_decimal = input.spread_bps / dec!(10000);
    let premium_leg_pv = spread_decimal * risky_pv01 * input.notional;

    // Annual premium
    let annual_premium = input.notional * spread_decimal;

    // Breakeven spread
    let breakeven_spread_bps = if risky_pv01.is_zero() {
        Decimal::ZERO
    } else {
        protection_leg / risky_pv01 * dec!(10000)
    };

    // DV01: risky_pv01 * notional / 10000
    let dv01 = risky_pv01 * input.notional / dec!(10000);

    // MTM / upfront payment
    let (upfront_payment, mark_to_market) = match input.market_spread_bps {
        Some(mkt_spread) => {
            let upfront =
                (mkt_spread - input.spread_bps) * risky_pv01 * input.notional / dec!(10000);
            (upfront, upfront)
        }
        None => (Decimal::ZERO, Decimal::ZERO),
    };

    // Jump to default: notional * LGD minus half a period of accrued premium
    // Accrued approximation: half a coupon period
    let accrued = spread_decimal * input.notional * dt / dec!(2);
    let jump_to_default = input.notional * lgd - accrued;

    let credit_triangle = CreditTriangle {
        spread_bps: input.spread_bps,
        default_probability: lambda,
        recovery_rate: input.recovery_rate,
        loss_given_default: lgd,
    };

    Ok(CdsOutput {
        reference_entity: input.reference_entity.clone(),
        notional: input.notional,
        spread_bps: input.spread_bps,
        annual_premium,
        implied_default_probability: lambda,
        survival_probabilities: survival_points,
        risky_pv01,
        protection_leg_pv,
        premium_leg_pv,
        upfront_payment,
        mark_to_market,
        breakeven_spread_bps,
        dv01,
        jump_to_default,
        credit_triangle,
    })
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

fn validate_cds_input(input: &CdsInput) -> CorpFinanceResult<()> {
    if input.notional <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "notional".into(),
            reason: "Notional must be positive.".into(),
        });
    }
    if input.recovery_rate < Decimal::ZERO || input.recovery_rate >= Decimal::ONE {
        return Err(CorpFinanceError::InvalidInput {
            field: "recovery_rate".into(),
            reason: "Recovery rate must be in [0, 1).".into(),
        });
    }
    if input.risk_free_rate < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "risk_free_rate".into(),
            reason: "Risk-free rate must be non-negative.".into(),
        });
    }
    if input.maturity_years < 1 || input.maturity_years > 30 {
        return Err(CorpFinanceError::InvalidInput {
            field: "maturity_years".into(),
            reason: "Maturity must be between 1 and 30 years.".into(),
        });
    }
    if input.payment_frequency != 1 && input.payment_frequency != 2 && input.payment_frequency != 4
    {
        return Err(CorpFinanceError::InvalidInput {
            field: "payment_frequency".into(),
            reason: "Payment frequency must be 1, 2, or 4.".into(),
        });
    }
    if input.spread_bps < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "spread_bps".into(),
            reason: "Spread must be non-negative.".into(),
        });
    }
    if let Some(pd) = input.default_probability {
        if pd < Decimal::ZERO || pd >= Decimal::ONE {
            return Err(CorpFinanceError::InvalidInput {
                field: "default_probability".into(),
                reason: "Default probability must be in [0, 1).".into(),
            });
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Math helpers
// ---------------------------------------------------------------------------

/// Compute the nth root of `x` via Newton's method: x^(1/n).
/// Uses 30 iterations for convergence.
fn nth_root(x: Decimal, n: u32) -> Decimal {
    if n == 0 || x.is_zero() {
        return x;
    }
    if n == 1 {
        return x;
    }

    let n_dec = Decimal::from(n);
    let n_minus_1 = n_dec - Decimal::ONE;

    // Initial guess: 1 + (x - 1) / n  (first-order Taylor)
    let mut guess = Decimal::ONE + (x - Decimal::ONE) / n_dec;

    for _ in 0..30 {
        // guess^(n-1) via iterative multiplication
        let mut power = Decimal::ONE;
        for _ in 0..(n - 1) {
            power *= guess;
        }
        if power.is_zero() {
            break;
        }
        // Newton step: new_guess = ((n-1)*guess + x / guess^(n-1)) / n
        guess = (n_minus_1 * guess + x / power) / n_dec;
    }

    guess
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn basic_cds_input() -> CdsInput {
        CdsInput {
            reference_entity: "Acme Corp".to_string(),
            notional: dec!(10_000_000),
            spread_bps: dec!(100), // 100 bps = 1%
            recovery_rate: dec!(0.40),
            risk_free_rate: dec!(0.05),
            maturity_years: 5,
            payment_frequency: 4,
            default_probability: None,
            market_spread_bps: None,
            counterparty_rating: None,
        }
    }

    #[test]
    fn test_basic_cds_pricing() {
        let input = basic_cds_input();
        let result = price_cds(&input).unwrap();

        assert_eq!(result.reference_entity, "Acme Corp");
        assert_eq!(result.notional, dec!(10_000_000));
        assert_eq!(result.spread_bps, dec!(100));
        assert!(result.annual_premium > Decimal::ZERO);
        assert!(result.risky_pv01 > Decimal::ZERO);
        assert!(result.protection_leg_pv > Decimal::ZERO);
        assert!(result.premium_leg_pv > Decimal::ZERO);
    }

    #[test]
    fn test_annual_premium_calculation() {
        let input = basic_cds_input();
        let result = price_cds(&input).unwrap();
        // annual_premium = notional * spread / 10000 = 10M * 100/10000 = 100,000
        assert_eq!(result.annual_premium, dec!(100_000));
    }

    #[test]
    fn test_implied_default_probability() {
        let input = basic_cds_input();
        let result = price_cds(&input).unwrap();

        // lambda = spread / (10000 * LGD) = 100 / (10000 * 0.6) = 100/6000
        let expected_lambda = dec!(100) / dec!(6000);
        assert_eq!(result.implied_default_probability, expected_lambda);
    }

    #[test]
    fn test_explicit_default_probability() {
        let mut input = basic_cds_input();
        input.default_probability = Some(dec!(0.02));
        let result = price_cds(&input).unwrap();

        assert_eq!(result.implied_default_probability, dec!(0.02));
    }

    #[test]
    fn test_survival_curve_decreasing() {
        let input = basic_cds_input();
        let result = price_cds(&input).unwrap();

        assert_eq!(result.survival_probabilities.len(), 5);
        let mut prev = Decimal::ONE;
        for sp in &result.survival_probabilities {
            assert!(
                sp.survival_probability < prev,
                "Survival probability should decrease: {} >= {}",
                sp.survival_probability,
                prev
            );
            assert!(
                sp.survival_probability > Decimal::ZERO,
                "Survival probability should be positive"
            );
            prev = sp.survival_probability;
        }
    }

    #[test]
    fn test_cumulative_default_probability() {
        let input = basic_cds_input();
        let result = price_cds(&input).unwrap();

        for sp in &result.survival_probabilities {
            let sum = sp.survival_probability + sp.cumulative_default_probability;
            // Should sum to 1
            let diff = (sum - Decimal::ONE).abs();
            assert!(
                diff < dec!(0.0001),
                "S(t) + CumPD(t) should equal 1, got {}",
                sum
            );
        }
    }

    #[test]
    fn test_discount_factors_decreasing() {
        let input = basic_cds_input();
        let result = price_cds(&input).unwrap();

        let mut prev = Decimal::ONE;
        for sp in &result.survival_probabilities {
            assert!(
                sp.discount_factor < prev,
                "Discount factors should decrease"
            );
            assert!(sp.discount_factor > Decimal::ZERO);
            prev = sp.discount_factor;
        }
    }

    #[test]
    fn test_dv01_positive() {
        let input = basic_cds_input();
        let result = price_cds(&input).unwrap();

        // DV01 = risky_pv01 * notional / 10000
        let expected_dv01 = result.risky_pv01 * dec!(10_000_000) / dec!(10000);
        let diff = (result.dv01 - expected_dv01).abs();
        assert!(
            diff < dec!(0.01),
            "DV01 mismatch: {} vs {}",
            result.dv01,
            expected_dv01
        );
        assert!(result.dv01 > Decimal::ZERO);
    }

    #[test]
    fn test_breakeven_spread() {
        let input = basic_cds_input();
        let result = price_cds(&input).unwrap();

        // Breakeven should be close to input spread for a fairly priced CDS
        // (slight difference due to discrete model vs. continuous)
        assert!(
            result.breakeven_spread_bps > Decimal::ZERO,
            "Breakeven spread should be positive"
        );
    }

    #[test]
    fn test_mtm_zero_when_no_market_spread() {
        let input = basic_cds_input();
        let result = price_cds(&input).unwrap();

        assert_eq!(result.upfront_payment, Decimal::ZERO);
        assert_eq!(result.mark_to_market, Decimal::ZERO);
    }

    #[test]
    fn test_mtm_with_market_spread() {
        let mut input = basic_cds_input();
        input.market_spread_bps = Some(dec!(150)); // 150 bps vs 100 bps contract
        let result = price_cds(&input).unwrap();

        // Market wider than contract => protection buyer gains => positive upfront
        assert!(
            result.upfront_payment > Decimal::ZERO,
            "Upfront should be positive when market spread > contract spread"
        );
        assert_eq!(result.mark_to_market, result.upfront_payment);
    }

    #[test]
    fn test_mtm_negative_when_spread_tightens() {
        let mut input = basic_cds_input();
        input.market_spread_bps = Some(dec!(50)); // 50 bps vs 100 bps contract
        let result = price_cds(&input).unwrap();

        // Market tighter than contract => protection buyer loses
        assert!(
            result.upfront_payment < Decimal::ZERO,
            "Upfront should be negative when market spread < contract spread"
        );
    }

    #[test]
    fn test_jump_to_default() {
        let input = basic_cds_input();
        let result = price_cds(&input).unwrap();

        // LGD exposure minus accrued
        let lgd_exposure = dec!(10_000_000) * dec!(0.60);
        assert!(
            result.jump_to_default < lgd_exposure,
            "JTD should be less than full LGD exposure (minus accrued)"
        );
        assert!(
            result.jump_to_default > Decimal::ZERO,
            "JTD should be positive"
        );
    }

    #[test]
    fn test_credit_triangle() {
        let input = basic_cds_input();
        let result = price_cds(&input).unwrap();

        assert_eq!(result.credit_triangle.spread_bps, dec!(100));
        assert_eq!(result.credit_triangle.recovery_rate, dec!(0.40));
        assert_eq!(result.credit_triangle.loss_given_default, dec!(0.60));
        // PD * LGD * 10000 should approximate spread
        let reconstructed =
            result.credit_triangle.default_probability * result.credit_triangle.loss_given_default;
        let diff = (reconstructed - dec!(0.01)).abs(); // spread = 100bps = 0.01
        assert!(
            diff < dec!(0.0001),
            "Credit triangle consistency: PD*LGD={} vs spread=0.01",
            reconstructed
        );
    }

    #[test]
    fn test_higher_spread_higher_protection_pv() {
        let input_low = basic_cds_input(); // 100 bps
        let mut input_high = basic_cds_input();
        input_high.spread_bps = dec!(500); // 500 bps

        let result_low = price_cds(&input_low).unwrap();
        let result_high = price_cds(&input_high).unwrap();

        // Higher spread implies higher default probability, hence higher protection leg PV
        assert!(
            result_high.protection_leg_pv > result_low.protection_leg_pv,
            "Higher spread should yield higher protection leg PV"
        );
    }

    #[test]
    fn test_quarterly_vs_annual_payments() {
        let mut input_q = basic_cds_input();
        input_q.payment_frequency = 4;
        let mut input_a = basic_cds_input();
        input_a.payment_frequency = 1;

        let result_q = price_cds(&input_q).unwrap();
        let result_a = price_cds(&input_a).unwrap();

        // More frequent payments generally yield a slightly different risky PV01
        assert_ne!(
            result_q.risky_pv01, result_a.risky_pv01,
            "Different frequencies should yield different risky PV01"
        );
    }

    #[test]
    fn test_zero_spread() {
        let mut input = basic_cds_input();
        input.spread_bps = Decimal::ZERO;
        let result = price_cds(&input).unwrap();

        assert_eq!(result.annual_premium, Decimal::ZERO);
        assert_eq!(result.premium_leg_pv, Decimal::ZERO);
        assert_eq!(result.implied_default_probability, Decimal::ZERO);
    }

    #[test]
    fn test_high_recovery_rate() {
        let mut input = basic_cds_input();
        input.recovery_rate = dec!(0.95); // very high recovery
        let result = price_cds(&input).unwrap();

        // Higher recovery => lower LGD => higher implied PD for same spread
        assert!(
            result.implied_default_probability > dec!(0.10),
            "High recovery should imply high PD for same spread"
        );
    }

    #[test]
    fn test_1yr_maturity() {
        let mut input = basic_cds_input();
        input.maturity_years = 1;
        let result = price_cds(&input).unwrap();

        assert_eq!(result.survival_probabilities.len(), 1);
        assert!(result.risky_pv01 > Decimal::ZERO);
        assert!(result.risky_pv01 < dec!(1)); // PV01 for 1yr should be small
    }

    #[test]
    fn test_long_maturity() {
        let mut input = basic_cds_input();
        input.maturity_years = 10;
        let result = price_cds(&input).unwrap();

        assert_eq!(result.survival_probabilities.len(), 10);
        // Longer maturity => higher risky PV01
        let result_5y = price_cds(&basic_cds_input()).unwrap();
        assert!(
            result.risky_pv01 > result_5y.risky_pv01,
            "10Y risky PV01 should exceed 5Y"
        );
    }

    // -- Validation tests --

    #[test]
    fn test_invalid_notional() {
        let mut input = basic_cds_input();
        input.notional = Decimal::ZERO;
        let err = price_cds(&input).unwrap_err();
        match err {
            CorpFinanceError::InvalidInput { field, .. } => assert_eq!(field, "notional"),
            other => panic!("Expected InvalidInput for notional, got {other:?}"),
        }
    }

    #[test]
    fn test_invalid_recovery_rate_too_high() {
        let mut input = basic_cds_input();
        input.recovery_rate = Decimal::ONE;
        let err = price_cds(&input).unwrap_err();
        match err {
            CorpFinanceError::InvalidInput { field, .. } => assert_eq!(field, "recovery_rate"),
            other => panic!("Expected InvalidInput for recovery_rate, got {other:?}"),
        }
    }

    #[test]
    fn test_invalid_recovery_rate_negative() {
        let mut input = basic_cds_input();
        input.recovery_rate = dec!(-0.1);
        let err = price_cds(&input).unwrap_err();
        match err {
            CorpFinanceError::InvalidInput { field, .. } => assert_eq!(field, "recovery_rate"),
            other => panic!("Expected InvalidInput for recovery_rate, got {other:?}"),
        }
    }

    #[test]
    fn test_invalid_maturity() {
        let mut input = basic_cds_input();
        input.maturity_years = 0;
        let err = price_cds(&input).unwrap_err();
        match err {
            CorpFinanceError::InvalidInput { field, .. } => assert_eq!(field, "maturity_years"),
            other => panic!("Expected InvalidInput for maturity_years, got {other:?}"),
        }
    }

    #[test]
    fn test_invalid_payment_frequency() {
        let mut input = basic_cds_input();
        input.payment_frequency = 3;
        let err = price_cds(&input).unwrap_err();
        match err {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "payment_frequency")
            }
            other => panic!("Expected InvalidInput for payment_frequency, got {other:?}"),
        }
    }

    #[test]
    fn test_invalid_default_probability() {
        let mut input = basic_cds_input();
        input.default_probability = Some(dec!(1.0));
        let err = price_cds(&input).unwrap_err();
        match err {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "default_probability")
            }
            other => panic!("Expected InvalidInput for default_probability, got {other:?}"),
        }
    }

    #[test]
    fn test_semiannual_payments() {
        let mut input = basic_cds_input();
        input.payment_frequency = 2;
        let result = price_cds(&input).unwrap();

        assert_eq!(result.survival_probabilities.len(), 5);
        assert!(result.risky_pv01 > Decimal::ZERO);
    }

    #[test]
    fn test_negative_spread_rejected() {
        let mut input = basic_cds_input();
        input.spread_bps = dec!(-10);
        let err = price_cds(&input).unwrap_err();
        match err {
            CorpFinanceError::InvalidInput { field, .. } => assert_eq!(field, "spread_bps"),
            other => panic!("Expected InvalidInput for spread_bps, got {other:?}"),
        }
    }

    #[test]
    fn test_nth_root_helper() {
        // 4th root of 16 = 2
        let result = nth_root(dec!(16), 4);
        let diff = (result - dec!(2)).abs();
        assert!(
            diff < dec!(0.0001),
            "4th root of 16 should be ~2, got {}",
            result
        );

        // square root of 4 = 2
        let result2 = nth_root(dec!(4), 2);
        let diff2 = (result2 - dec!(2)).abs();
        assert!(
            diff2 < dec!(0.0001),
            "sqrt(4) should be ~2, got {}",
            result2
        );
    }
}
