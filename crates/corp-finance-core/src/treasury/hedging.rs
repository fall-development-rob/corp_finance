//! Hedge effectiveness and strategy analysis for corporate treasury.
//!
//! Implements IAS 39 / IFRS 9 / ASC 815 hedge effectiveness testing:
//! - **Dollar offset method**: cumulative gain/loss ratio
//! - **Regression analysis**: OLS slope and R-squared
//! - **VaR reduction**: unhedged vs hedged Value-at-Risk
//! - **P&L attribution**: total exposure, hedge, net, and ineffectiveness
//! - **Optimal hedge ratio**: minimum-variance from regression slope
//!
//! Supports Forward, Option, Swap, and Collar hedge instruments across
//! FairValue, CashFlow, and NetInvestment hedge types.
//!
//! All calculations use `rust_decimal::Decimal`. Square roots via Newton's
//! method (20 iterations), inverse normal via Abramowitz & Stegun.

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

use crate::{CorpFinanceError, CorpFinanceResult};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const NEWTON_SQRT_ITERATIONS: u32 = 20;

// ---------------------------------------------------------------------------
// Input / Output types
// ---------------------------------------------------------------------------

/// Type of hedge accounting relationship.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HedgeType {
    FairValue,
    CashFlow,
    NetInvestment,
}

/// Hedging instrument type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HedgeInstrument {
    Forward,
    Option,
    Swap,
    Collar,
}

/// Input for hedge effectiveness analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HedgingInput {
    /// Type of hedge relationship.
    pub hedge_type: HedgeType,
    /// Currency of the underlying exposure.
    pub exposure_currency: String,
    /// Currency used for hedging.
    pub hedge_currency: String,
    /// Notional amount of the exposure.
    pub notional_amount: Decimal,
    /// Notional amount of the hedge instrument.
    pub hedge_notional: Decimal,
    /// Type of hedge instrument.
    pub hedge_instrument: HedgeInstrument,
    /// Period-to-period changes in hedged item value.
    pub exposure_changes: Vec<Decimal>,
    /// Period-to-period changes in hedging instrument value.
    pub hedge_changes: Vec<Decimal>,
    /// Domestic risk-free rate.
    pub risk_free_rate_domestic: Decimal,
    /// Foreign risk-free rate.
    pub risk_free_rate_foreign: Decimal,
    /// Current spot FX rate.
    pub spot_rate: Decimal,
    /// Contracted forward FX rate.
    pub forward_rate: Decimal,
    /// Implied volatility for option-based hedges.
    pub volatility: Decimal,
    /// Hedge tenor in months.
    pub tenor_months: u32,
    /// Confidence level for VaR (e.g. 0.95).
    pub confidence_level: Decimal,
}

/// Output of hedge effectiveness analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HedgingOutput {
    /// Hedge ratio: hedge notional / exposure notional.
    pub hedge_ratio: Decimal,
    /// Dollar offset ratio: cumulative hedge gain / cumulative exposure loss.
    pub dollar_offset_ratio: Decimal,
    /// R-squared from OLS regression of hedge changes on exposure changes.
    pub regression_r_squared: Decimal,
    /// Slope (beta) from OLS regression.
    pub regression_slope: Decimal,
    /// Whether the hedge is highly effective (IAS 39: 80-125% offset AND R2>0.80).
    pub is_highly_effective: bool,
    /// Percentage effectiveness.
    pub effectiveness_pct: Decimal,
    /// Cost of the hedge (forward points or option premium estimate).
    pub hedge_cost: Decimal,
    /// Annualised carry cost of the hedge.
    pub carry_cost: Decimal,
    /// Value-at-Risk without the hedge.
    pub var_unhedged: Decimal,
    /// Value-at-Risk with the hedge.
    pub var_hedged: Decimal,
    /// Percentage reduction in VaR.
    pub var_reduction_pct: Decimal,
    /// Optimal (minimum variance) hedge ratio from regression.
    pub optimal_hedge_ratio: Decimal,
    /// P&L attribution breakdown.
    pub pnl_attribution: PnlAttribution,
    /// Accounting treatment description (IFRS 9 / ASC 815).
    pub accounting_treatment: String,
}

/// Breakdown of P&L components.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PnlAttribution {
    /// Total change in exposure value.
    pub total_exposure_change: Decimal,
    /// Total change in hedge instrument value.
    pub total_hedge_change: Decimal,
    /// Net position change (exposure + hedge).
    pub net_position_change: Decimal,
    /// Hedge ineffectiveness.
    pub ineffectiveness: Decimal,
}

// ---------------------------------------------------------------------------
// Decimal math helpers
// ---------------------------------------------------------------------------

/// Newton's method square root (20 iterations).
fn sqrt_decimal(x: Decimal) -> Decimal {
    if x <= Decimal::ZERO {
        return Decimal::ZERO;
    }
    if x == Decimal::ONE {
        return Decimal::ONE;
    }
    let two = Decimal::from(2u32);
    let mut guess = x / two;
    for _ in 0..NEWTON_SQRT_ITERATIONS {
        if guess.is_zero() {
            return Decimal::ZERO;
        }
        guess = (guess + x / guess) / two;
    }
    guess
}

/// Abramowitz & Stegun approximation of the cumulative normal distribution.
#[allow(dead_code)]
fn norm_cdf(x: Decimal) -> Decimal {
    let a1 = dec!(0.254829592);
    let a2 = dec!(-0.284496736);
    let a3 = dec!(1.421413741);
    let a4 = dec!(-1.453152027);
    let a5 = dec!(1.061405429);
    let p = dec!(0.3275911);

    let sign = if x < Decimal::ZERO {
        Decimal::NEGATIVE_ONE
    } else {
        Decimal::ONE
    };
    let abs_x = x.abs();

    let t = Decimal::ONE / (Decimal::ONE + p * abs_x);
    let t2 = t * t;
    let t3 = t2 * t;
    let t4 = t3 * t;
    let t5 = t4 * t;

    let poly = a1 * t + a2 * t2 + a3 * t3 + a4 * t4 + a5 * t5;

    // exp(-x^2 / 2): use Taylor series
    let exp_val = exp_decimal(-(abs_x * abs_x) / Decimal::from(2u32));

    let y = Decimal::ONE - poly * exp_val;
    (Decimal::ONE + sign * y) / Decimal::from(2u32)
}

/// Inverse of the standard normal CDF (quantile function).
/// Rational approximation (Abramowitz & Stegun 26.2.23).
fn norm_inv(p: Decimal) -> Decimal {
    if p <= Decimal::ZERO {
        return dec!(-10);
    }
    if p >= Decimal::ONE {
        return dec!(10);
    }

    let half = dec!(0.5);
    let sign;
    let pp;
    if p < half {
        sign = Decimal::NEGATIVE_ONE;
        pp = p;
    } else {
        sign = Decimal::ONE;
        pp = Decimal::ONE - p;
    };

    let two = Decimal::from(2u32);
    // t = sqrt(-2 * ln(pp))
    let ln_pp = ln_decimal(pp);
    let t = sqrt_decimal(-two * ln_pp);

    // Rational approximation coefficients
    let c0 = dec!(2.515517);
    let c1 = dec!(0.802853);
    let c2 = dec!(0.010328);
    let d1 = dec!(1.432788);
    let d2 = dec!(0.189269);
    let d3 = dec!(0.001308);

    let t2 = t * t;
    let t3 = t2 * t;

    let numerator = c0 + c1 * t + c2 * t2;
    let denominator = Decimal::ONE + d1 * t + d2 * t2 + d3 * t3;

    let result = t - numerator / denominator;
    sign * result
}

/// Taylor series exp(x) with range reduction.
fn exp_decimal(x: Decimal) -> Decimal {
    let two = Decimal::from(2u32);

    let mut k: u32 = 0;
    let mut reduced = x;
    while reduced.abs() > two {
        reduced /= two;
        k += 1;
    }

    let mut sum = Decimal::ONE;
    let mut term = Decimal::ONE;
    for n in 1..=30u64 {
        term *= reduced / Decimal::from(n);
        sum += term;
    }

    for _ in 0..k {
        sum *= sum;
    }

    sum
}

/// Natural logarithm via Newton's method (20 iterations).
fn ln_decimal(x: Decimal) -> Decimal {
    if x <= Decimal::ZERO {
        return Decimal::ZERO;
    }
    if x == Decimal::ONE {
        return Decimal::ZERO;
    }

    let two = Decimal::from(2u32);
    let ln2_approx = dec!(0.6931471805599453);

    // Range reduction: x = temp * 2^k => ln(x) = ln(temp) + k*ln(2)
    let mut offset = Decimal::ZERO;
    let mut temp = x;

    if temp > Decimal::ONE {
        while temp > two {
            temp /= two;
            offset += ln2_approx;
        }
    } else {
        while temp < Decimal::ONE {
            temp *= two;
            offset -= ln2_approx;
        }
    }

    // Newton iterations to find ln(temp) where temp is in [1, 2]
    // Start with initial guess based on (temp - 1) which is a good approx near 1
    let mut y = temp - Decimal::ONE;
    // Newton: y_{n+1} = y_n + 2*(temp - exp(y_n)) / (temp + exp(y_n))
    for _ in 0..20u32 {
        let ey = exp_decimal(y);
        let denom = temp + ey;
        if denom.is_zero() {
            break;
        }
        y += two * (temp - ey) / denom;
    }

    offset + y
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Analyse hedge effectiveness for a corporate treasury hedging programme.
///
/// Computes dollar offset ratio, OLS regression (slope + R-squared),
/// VaR reduction, P&L attribution, hedge cost, and accounting treatment.
pub fn analyze_hedging(input: &HedgingInput) -> CorpFinanceResult<HedgingOutput> {
    validate_input(input)?;

    // -- Hedge ratio ---------------------------------------------------------
    let hedge_ratio = if input.notional_amount.is_zero() {
        Decimal::ZERO
    } else {
        input.hedge_notional / input.notional_amount
    };

    // -- P&L Attribution -----------------------------------------------------
    let total_exposure_change: Decimal = input.exposure_changes.iter().copied().sum();
    let total_hedge_change: Decimal = input.hedge_changes.iter().copied().sum();
    let net_position_change = total_exposure_change + total_hedge_change;

    // -- Dollar offset ratio -------------------------------------------------
    // Dollar offset = -sum(hedge_changes) / sum(exposure_changes)
    let dollar_offset_ratio = if total_exposure_change.is_zero() {
        Decimal::ZERO
    } else {
        -(total_hedge_change / total_exposure_change)
    };

    // -- OLS regression: hedge_changes = alpha + beta * exposure_changes -----
    let n = input.exposure_changes.len();
    let n_dec = Decimal::from(n as u32);

    let (regression_slope, regression_r_squared) = if n < 2 {
        (Decimal::ZERO, Decimal::ZERO)
    } else {
        let sum_x: Decimal = input.exposure_changes.iter().copied().sum();
        let sum_y: Decimal = input.hedge_changes.iter().copied().sum();
        let mean_x = sum_x / n_dec;
        let mean_y = sum_y / n_dec;

        let mut ss_xy = Decimal::ZERO;
        let mut ss_xx = Decimal::ZERO;
        let mut ss_yy = Decimal::ZERO;

        for i in 0..n {
            let dx = input.exposure_changes[i] - mean_x;
            let dy = input.hedge_changes[i] - mean_y;
            ss_xy += dx * dy;
            ss_xx += dx * dx;
            ss_yy += dy * dy;
        }

        let slope = if ss_xx.is_zero() {
            Decimal::ZERO
        } else {
            ss_xy / ss_xx
        };

        let r_squared = if ss_xx.is_zero() || ss_yy.is_zero() {
            Decimal::ZERO
        } else {
            let r = ss_xy / (sqrt_decimal(ss_xx) * sqrt_decimal(ss_yy));
            r * r
        };

        (slope, r_squared)
    };

    // -- Effectiveness assessment --------------------------------------------
    // IAS 39 / IFRS 9: dollar offset 80-125% AND R² > 0.80
    let effectiveness_pct = dollar_offset_ratio * dec!(100);
    let is_highly_effective = dollar_offset_ratio >= dec!(0.80)
        && dollar_offset_ratio <= dec!(1.25)
        && regression_r_squared > dec!(0.80);

    // -- Ineffectiveness -----------------------------------------------------
    let ineffectiveness = net_position_change;

    let pnl_attribution = PnlAttribution {
        total_exposure_change,
        total_hedge_change,
        net_position_change,
        ineffectiveness,
    };

    // -- Hedge cost ----------------------------------------------------------
    let hedge_cost = compute_hedge_cost(input);

    // -- Carry cost (annualised) ---------------------------------------------
    let tenor_years = Decimal::from(input.tenor_months) / Decimal::from(12u32);
    let carry_cost = if tenor_years.is_zero() {
        Decimal::ZERO
    } else {
        hedge_cost / tenor_years
    };

    // -- VaR -----------------------------------------------------------------
    let z_score = norm_inv(input.confidence_level);
    let sqrt_tenor = sqrt_decimal(Decimal::from(input.tenor_months) / Decimal::from(12u32));

    let var_unhedged = input.notional_amount * input.volatility * z_score * sqrt_tenor;
    let var_unhedged_abs = var_unhedged.abs();

    // Hedged VaR: reduced by effectiveness (approximated via R²)
    let effectiveness_factor = if regression_r_squared > Decimal::ZERO {
        Decimal::ONE - regression_r_squared * hedge_ratio
    } else {
        Decimal::ONE
    };
    let effectiveness_factor_capped = if effectiveness_factor < Decimal::ZERO {
        Decimal::ZERO
    } else {
        effectiveness_factor
    };
    let var_hedged = var_unhedged_abs * effectiveness_factor_capped;

    let var_reduction_pct = if var_unhedged_abs.is_zero() {
        Decimal::ZERO
    } else {
        (var_unhedged_abs - var_hedged) / var_unhedged_abs * dec!(100)
    };

    // -- Optimal hedge ratio (minimum variance) ------------------------------
    // = regression slope (negative sign expected for a proper hedge)
    let optimal_hedge_ratio = regression_slope.abs();

    // -- Accounting treatment ------------------------------------------------
    let accounting_treatment =
        describe_accounting_treatment(&input.hedge_type, &input.hedge_instrument);

    Ok(HedgingOutput {
        hedge_ratio,
        dollar_offset_ratio,
        regression_r_squared,
        regression_slope,
        is_highly_effective,
        effectiveness_pct,
        hedge_cost,
        carry_cost,
        var_unhedged: var_unhedged_abs,
        var_hedged,
        var_reduction_pct,
        optimal_hedge_ratio,
        pnl_attribution,
        accounting_treatment,
    })
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn validate_input(input: &HedgingInput) -> CorpFinanceResult<()> {
    if input.exposure_changes.is_empty() || input.hedge_changes.is_empty() {
        return Err(CorpFinanceError::InsufficientData(
            "Exposure and hedge change vectors must not be empty.".into(),
        ));
    }
    if input.exposure_changes.len() != input.hedge_changes.len() {
        return Err(CorpFinanceError::InvalidInput {
            field: "exposure_changes / hedge_changes".into(),
            reason: "Exposure and hedge change vectors must have the same length.".into(),
        });
    }
    if input.notional_amount < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "notional_amount".into(),
            reason: "Notional amount cannot be negative.".into(),
        });
    }
    if input.hedge_notional < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "hedge_notional".into(),
            reason: "Hedge notional cannot be negative.".into(),
        });
    }
    if input.volatility < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "volatility".into(),
            reason: "Volatility cannot be negative.".into(),
        });
    }
    if input.tenor_months == 0 {
        return Err(CorpFinanceError::InvalidInput {
            field: "tenor_months".into(),
            reason: "Tenor must be at least 1 month.".into(),
        });
    }
    if input.confidence_level <= Decimal::ZERO || input.confidence_level >= Decimal::ONE {
        return Err(CorpFinanceError::InvalidInput {
            field: "confidence_level".into(),
            reason: "Confidence level must be between 0 and 1 (exclusive).".into(),
        });
    }
    if input.spot_rate <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "spot_rate".into(),
            reason: "Spot rate must be positive.".into(),
        });
    }
    Ok(())
}

fn compute_hedge_cost(input: &HedgingInput) -> Decimal {
    match input.hedge_instrument {
        HedgeInstrument::Forward => {
            // Forward points cost = (forward_rate - spot_rate) * notional
            (input.forward_rate - input.spot_rate) * input.hedge_notional
        }
        HedgeInstrument::Option => {
            // Black-Scholes-like premium approximation
            // Premium ~ notional * vol * sqrt(T) * 0.4 (ATM heuristic)
            let tenor_years = Decimal::from(input.tenor_months) / Decimal::from(12u32);
            let sqrt_t = sqrt_decimal(tenor_years);
            input.hedge_notional * input.volatility * sqrt_t * dec!(0.4)
        }
        HedgeInstrument::Swap => {
            // Swap cost ~ notional * (domestic_rate - foreign_rate) * tenor
            let tenor_years = Decimal::from(input.tenor_months) / Decimal::from(12u32);
            let rate_diff = input.risk_free_rate_domestic - input.risk_free_rate_foreign;
            input.hedge_notional * rate_diff * tenor_years
        }
        HedgeInstrument::Collar => {
            // Collar: net of put premium minus call premium
            // Simplified: 50% of a single option premium (zero-cost collar approx)
            let tenor_years = Decimal::from(input.tenor_months) / Decimal::from(12u32);
            let sqrt_t = sqrt_decimal(tenor_years);
            input.hedge_notional * input.volatility * sqrt_t * dec!(0.2)
        }
    }
}

fn describe_accounting_treatment(hedge_type: &HedgeType, instrument: &HedgeInstrument) -> String {
    let instrument_name = match instrument {
        HedgeInstrument::Forward => "forward contract",
        HedgeInstrument::Option => "purchased option",
        HedgeInstrument::Swap => "interest rate / currency swap",
        HedgeInstrument::Collar => "collar (option combination)",
    };

    match hedge_type {
        HedgeType::FairValue => format!(
            "Fair value hedge (IFRS 9 / ASC 815): Changes in the fair value of the {} \
             and the hedged item are recognised in profit or loss. \
             Basis adjustment applied to the hedged item carrying amount.",
            instrument_name
        ),
        HedgeType::CashFlow => format!(
            "Cash flow hedge (IFRS 9 / ASC 815): Effective portion of the {} \
             gain/loss is recognised in OCI (cash flow hedge reserve). \
             Ineffective portion is recognised in profit or loss. \
             Reclassified to P&L when the hedged cash flow affects earnings.",
            instrument_name
        ),
        HedgeType::NetInvestment => format!(
            "Net investment hedge (IFRS 9 / ASC 815): Effective portion of the {} \
             gain/loss is recognised in OCI (translation reserve). \
             Recycled to P&L on disposal of the foreign operation.",
            instrument_name
        ),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    // -- Test helpers --------------------------------------------------------

    fn default_input() -> HedgingInput {
        HedgingInput {
            hedge_type: HedgeType::CashFlow,
            exposure_currency: "USD".to_string(),
            hedge_currency: "EUR".to_string(),
            notional_amount: dec!(10_000_000),
            hedge_notional: dec!(10_000_000),
            hedge_instrument: HedgeInstrument::Forward,
            exposure_changes: vec![
                dec!(-100_000),
                dec!(50_000),
                dec!(-80_000),
                dec!(30_000),
                dec!(-120_000),
                dec!(60_000),
            ],
            hedge_changes: vec![
                dec!(95_000),
                dec!(-48_000),
                dec!(78_000),
                dec!(-29_000),
                dec!(115_000),
                dec!(-58_000),
            ],
            risk_free_rate_domestic: dec!(0.05),
            risk_free_rate_foreign: dec!(0.03),
            spot_rate: dec!(1.10),
            forward_rate: dec!(1.12),
            volatility: dec!(0.12),
            tenor_months: 12,
            confidence_level: dec!(0.95),
        }
    }

    fn perfect_hedge_input() -> HedgingInput {
        HedgingInput {
            hedge_type: HedgeType::FairValue,
            exposure_currency: "USD".to_string(),
            hedge_currency: "GBP".to_string(),
            notional_amount: dec!(5_000_000),
            hedge_notional: dec!(5_000_000),
            hedge_instrument: HedgeInstrument::Forward,
            exposure_changes: vec![dec!(-100_000), dec!(200_000), dec!(-150_000), dec!(100_000)],
            hedge_changes: vec![dec!(100_000), dec!(-200_000), dec!(150_000), dec!(-100_000)],
            risk_free_rate_domestic: dec!(0.04),
            risk_free_rate_foreign: dec!(0.02),
            spot_rate: dec!(1.25),
            forward_rate: dec!(1.27),
            volatility: dec!(0.10),
            tenor_months: 6,
            confidence_level: dec!(0.95),
        }
    }

    fn option_hedge_input() -> HedgingInput {
        let mut input = default_input();
        input.hedge_instrument = HedgeInstrument::Option;
        input
    }

    // -- Validation tests ----------------------------------------------------

    #[test]
    fn test_empty_exposure_changes_rejected() {
        let mut input = default_input();
        input.exposure_changes = vec![];
        input.hedge_changes = vec![];
        assert!(analyze_hedging(&input).is_err());
    }

    #[test]
    fn test_mismatched_lengths_rejected() {
        let mut input = default_input();
        input.exposure_changes.push(dec!(10_000));
        assert!(analyze_hedging(&input).is_err());
    }

    #[test]
    fn test_negative_notional_rejected() {
        let mut input = default_input();
        input.notional_amount = dec!(-1);
        assert!(analyze_hedging(&input).is_err());
    }

    #[test]
    fn test_negative_hedge_notional_rejected() {
        let mut input = default_input();
        input.hedge_notional = dec!(-1);
        assert!(analyze_hedging(&input).is_err());
    }

    #[test]
    fn test_negative_volatility_rejected() {
        let mut input = default_input();
        input.volatility = dec!(-0.01);
        assert!(analyze_hedging(&input).is_err());
    }

    #[test]
    fn test_zero_tenor_rejected() {
        let mut input = default_input();
        input.tenor_months = 0;
        assert!(analyze_hedging(&input).is_err());
    }

    #[test]
    fn test_invalid_confidence_level_rejected() {
        let mut input = default_input();
        input.confidence_level = dec!(1.0);
        assert!(analyze_hedging(&input).is_err());
    }

    #[test]
    fn test_zero_confidence_level_rejected() {
        let mut input = default_input();
        input.confidence_level = Decimal::ZERO;
        assert!(analyze_hedging(&input).is_err());
    }

    #[test]
    fn test_zero_spot_rate_rejected() {
        let mut input = default_input();
        input.spot_rate = Decimal::ZERO;
        assert!(analyze_hedging(&input).is_err());
    }

    // -- Hedge ratio tests ---------------------------------------------------

    #[test]
    fn test_hedge_ratio_one_to_one() {
        let input = default_input();
        let result = analyze_hedging(&input).unwrap();
        assert_eq!(result.hedge_ratio, Decimal::ONE);
    }

    #[test]
    fn test_hedge_ratio_partial() {
        let mut input = default_input();
        input.hedge_notional = dec!(8_000_000);
        let result = analyze_hedging(&input).unwrap();
        assert_eq!(result.hedge_ratio, dec!(0.8));
    }

    #[test]
    fn test_hedge_ratio_zero_notional() {
        let mut input = default_input();
        input.notional_amount = Decimal::ZERO;
        let result = analyze_hedging(&input).unwrap();
        assert_eq!(result.hedge_ratio, Decimal::ZERO);
    }

    // -- Dollar offset tests -------------------------------------------------

    #[test]
    fn test_dollar_offset_perfect_hedge() {
        let input = perfect_hedge_input();
        let result = analyze_hedging(&input).unwrap();
        // Sum exposure = -100k + 200k - 150k + 100k = 50k
        // Sum hedge = 100k - 200k + 150k - 100k = -50k
        // Dollar offset = -(-50k) / 50k = 1.0
        assert_eq!(result.dollar_offset_ratio, Decimal::ONE);
    }

    #[test]
    fn test_dollar_offset_range() {
        let input = default_input();
        let result = analyze_hedging(&input).unwrap();
        // For a reasonable hedge, offset should be positive
        assert!(
            result.dollar_offset_ratio > Decimal::ZERO,
            "Dollar offset should be positive for an opposing hedge"
        );
    }

    #[test]
    fn test_dollar_offset_zero_exposure() {
        let mut input = default_input();
        input.exposure_changes = vec![Decimal::ZERO; 6];
        let result = analyze_hedging(&input).unwrap();
        assert_eq!(result.dollar_offset_ratio, Decimal::ZERO);
    }

    // -- Regression tests ----------------------------------------------------

    #[test]
    fn test_regression_perfect_negative_correlation() {
        let input = perfect_hedge_input();
        let result = analyze_hedging(&input).unwrap();
        // Perfect negative correlation: R² should be 1.0
        let r2_diff = (result.regression_r_squared - Decimal::ONE).abs();
        assert!(
            r2_diff < dec!(0.01),
            "R² should be ~1.0 for perfect negative correlation, got {}",
            result.regression_r_squared
        );
    }

    #[test]
    fn test_regression_slope_negative_for_hedge() {
        let input = perfect_hedge_input();
        let result = analyze_hedging(&input).unwrap();
        // Hedge changes are opposite to exposure: slope should be -1
        let slope_diff = (result.regression_slope - dec!(-1)).abs();
        assert!(
            slope_diff < dec!(0.01),
            "Slope should be ~-1.0 for perfect hedge, got {}",
            result.regression_slope
        );
    }

    #[test]
    fn test_regression_with_two_points() {
        let mut input = default_input();
        input.exposure_changes = vec![dec!(-100), dec!(200)];
        input.hedge_changes = vec![dec!(90), dec!(-180)];
        let result = analyze_hedging(&input).unwrap();
        // Should still compute regression with 2 points
        assert!(result.regression_r_squared > Decimal::ZERO);
    }

    #[test]
    fn test_regression_single_point() {
        let mut input = default_input();
        input.exposure_changes = vec![dec!(-100)];
        input.hedge_changes = vec![dec!(95)];
        let result = analyze_hedging(&input).unwrap();
        // n < 2: regression should return zeros
        assert_eq!(result.regression_slope, Decimal::ZERO);
        assert_eq!(result.regression_r_squared, Decimal::ZERO);
    }

    // -- Effectiveness tests -------------------------------------------------

    #[test]
    fn test_highly_effective_perfect_hedge() {
        let input = perfect_hedge_input();
        let result = analyze_hedging(&input).unwrap();
        assert!(
            result.is_highly_effective,
            "A perfect hedge should be highly effective"
        );
    }

    #[test]
    fn test_not_effective_when_offset_outside_range() {
        let mut input = default_input();
        // Make hedge changes much larger than exposure to push offset > 1.25
        input.hedge_changes = vec![
            dec!(200_000),
            dec!(-96_000),
            dec!(160_000),
            dec!(-60_000),
            dec!(240_000),
            dec!(-120_000),
        ];
        let result = analyze_hedging(&input).unwrap();
        // Dollar offset should be well above 1.25
        assert!(
            !result.is_highly_effective,
            "Hedge with offset > 1.25 should not be highly effective"
        );
    }

    #[test]
    fn test_effectiveness_pct_calculation() {
        let input = default_input();
        let result = analyze_hedging(&input).unwrap();
        let expected = result.dollar_offset_ratio * dec!(100);
        assert_eq!(result.effectiveness_pct, expected);
    }

    // -- Hedge cost tests ----------------------------------------------------

    #[test]
    fn test_forward_hedge_cost() {
        let input = default_input();
        let result = analyze_hedging(&input).unwrap();
        // Forward cost = (1.12 - 1.10) * 10M = 200,000
        let expected = (dec!(1.12) - dec!(1.10)) * dec!(10_000_000);
        assert_eq!(result.hedge_cost, expected);
    }

    #[test]
    fn test_option_hedge_cost_positive() {
        let input = option_hedge_input();
        let result = analyze_hedging(&input).unwrap();
        assert!(
            result.hedge_cost > Decimal::ZERO,
            "Option premium should be positive"
        );
    }

    #[test]
    fn test_swap_hedge_cost() {
        let mut input = default_input();
        input.hedge_instrument = HedgeInstrument::Swap;
        let result = analyze_hedging(&input).unwrap();
        // Swap cost = 10M * (0.05 - 0.03) * 1 = 200,000
        let expected = dec!(10_000_000) * dec!(0.02) * Decimal::ONE;
        assert_eq!(result.hedge_cost, expected);
    }

    #[test]
    fn test_collar_hedge_cost_less_than_option() {
        let input_option = option_hedge_input();
        let mut input_collar = default_input();
        input_collar.hedge_instrument = HedgeInstrument::Collar;

        let result_option = analyze_hedging(&input_option).unwrap();
        let result_collar = analyze_hedging(&input_collar).unwrap();
        assert!(
            result_collar.hedge_cost < result_option.hedge_cost,
            "Collar cost should be less than option (zero-cost collar approx)"
        );
    }

    // -- Carry cost tests ----------------------------------------------------

    #[test]
    fn test_carry_cost_annualised() {
        let input = default_input();
        let result = analyze_hedging(&input).unwrap();
        // Tenor = 12 months = 1 year, so carry_cost = hedge_cost / 1
        assert_eq!(result.carry_cost, result.hedge_cost);
    }

    #[test]
    fn test_carry_cost_six_month() {
        let mut input = default_input();
        input.tenor_months = 6;
        let result = analyze_hedging(&input).unwrap();
        // carry_cost = hedge_cost / 0.5 = hedge_cost * 2
        let expected = result.hedge_cost * Decimal::from(2u32);
        let diff = (result.carry_cost - expected).abs();
        assert!(
            diff < dec!(0.01),
            "Carry cost should be hedge_cost / (6/12)"
        );
    }

    // -- VaR tests -----------------------------------------------------------

    #[test]
    fn test_var_unhedged_positive() {
        let input = default_input();
        let result = analyze_hedging(&input).unwrap();
        assert!(
            result.var_unhedged > Decimal::ZERO,
            "Unhedged VaR should be positive"
        );
    }

    #[test]
    fn test_var_hedged_less_than_unhedged() {
        let input = default_input();
        let result = analyze_hedging(&input).unwrap();
        assert!(
            result.var_hedged <= result.var_unhedged,
            "Hedged VaR should be <= unhedged VaR"
        );
    }

    #[test]
    fn test_var_reduction_percentage() {
        let input = default_input();
        let result = analyze_hedging(&input).unwrap();
        if result.var_unhedged > Decimal::ZERO {
            let expected =
                (result.var_unhedged - result.var_hedged) / result.var_unhedged * dec!(100);
            let diff = (result.var_reduction_pct - expected).abs();
            assert!(diff < dec!(0.01), "VaR reduction % calculation mismatch");
        }
    }

    #[test]
    fn test_var_increases_with_volatility() {
        let input_low = default_input();
        let mut input_high = default_input();
        input_high.volatility = dec!(0.25);

        let result_low = analyze_hedging(&input_low).unwrap();
        let result_high = analyze_hedging(&input_high).unwrap();

        assert!(
            result_high.var_unhedged > result_low.var_unhedged,
            "Higher volatility should increase VaR"
        );
    }

    // -- P&L attribution tests -----------------------------------------------

    #[test]
    fn test_pnl_totals_match() {
        let input = default_input();
        let result = analyze_hedging(&input).unwrap();
        let pnl = &result.pnl_attribution;

        let sum_exp: Decimal = input.exposure_changes.iter().copied().sum();
        let sum_hedge: Decimal = input.hedge_changes.iter().copied().sum();

        assert_eq!(pnl.total_exposure_change, sum_exp);
        assert_eq!(pnl.total_hedge_change, sum_hedge);
        assert_eq!(pnl.net_position_change, sum_exp + sum_hedge);
    }

    #[test]
    fn test_pnl_perfect_hedge_near_zero_net() {
        let input = perfect_hedge_input();
        let result = analyze_hedging(&input).unwrap();
        assert_eq!(
            result.pnl_attribution.net_position_change,
            Decimal::ZERO,
            "Perfect hedge should have zero net position change"
        );
    }

    // -- Optimal hedge ratio tests -------------------------------------------

    #[test]
    fn test_optimal_hedge_ratio_perfect() {
        let input = perfect_hedge_input();
        let result = analyze_hedging(&input).unwrap();
        let diff = (result.optimal_hedge_ratio - Decimal::ONE).abs();
        assert!(
            diff < dec!(0.01),
            "Optimal hedge ratio should be ~1.0 for perfect hedge, got {}",
            result.optimal_hedge_ratio
        );
    }

    // -- Accounting treatment tests ------------------------------------------

    #[test]
    fn test_accounting_treatment_cash_flow() {
        let input = default_input();
        let result = analyze_hedging(&input).unwrap();
        assert!(result.accounting_treatment.contains("Cash flow hedge"));
        assert!(result.accounting_treatment.contains("OCI"));
    }

    #[test]
    fn test_accounting_treatment_fair_value() {
        let input = perfect_hedge_input();
        let result = analyze_hedging(&input).unwrap();
        assert!(result.accounting_treatment.contains("Fair value hedge"));
        assert!(result.accounting_treatment.contains("profit or loss"));
    }

    #[test]
    fn test_accounting_treatment_net_investment() {
        let mut input = default_input();
        input.hedge_type = HedgeType::NetInvestment;
        let result = analyze_hedging(&input).unwrap();
        assert!(result.accounting_treatment.contains("Net investment hedge"));
        assert!(result.accounting_treatment.contains("translation reserve"));
    }

    // -- Edge case tests -----------------------------------------------------

    #[test]
    fn test_zero_volatility_zero_var() {
        let mut input = default_input();
        input.volatility = Decimal::ZERO;
        let result = analyze_hedging(&input).unwrap();
        assert_eq!(result.var_unhedged, Decimal::ZERO);
        assert_eq!(result.var_hedged, Decimal::ZERO);
    }

    #[test]
    fn test_serialization_roundtrip() {
        let input = default_input();
        let result = analyze_hedging(&input).unwrap();
        let json = serde_json::to_string(&result).unwrap();
        let _deserialized: HedgingOutput = serde_json::from_str(&json).unwrap();
    }

    // -- Math helper tests ---------------------------------------------------

    #[test]
    fn test_sqrt_decimal_basic() {
        let result = sqrt_decimal(dec!(4));
        let diff = (result - dec!(2)).abs();
        assert!(
            diff < dec!(0.0000001),
            "sqrt(4) should be ~2, got {}",
            result
        );
    }

    #[test]
    fn test_sqrt_decimal_zero() {
        assert_eq!(sqrt_decimal(Decimal::ZERO), Decimal::ZERO);
    }

    #[test]
    fn test_sqrt_decimal_one() {
        assert_eq!(sqrt_decimal(Decimal::ONE), Decimal::ONE);
    }

    #[test]
    fn test_exp_decimal_zero() {
        let result = exp_decimal(Decimal::ZERO);
        assert_eq!(result, Decimal::ONE);
    }

    #[test]
    fn test_ln_decimal_one() {
        assert_eq!(ln_decimal(Decimal::ONE), Decimal::ZERO);
    }

    #[test]
    fn test_norm_cdf_at_zero() {
        let result = norm_cdf(Decimal::ZERO);
        let diff = (result - dec!(0.5)).abs();
        assert!(
            diff < dec!(0.001),
            "norm_cdf(0) should be ~0.5, got {}",
            result
        );
    }

    #[test]
    fn test_norm_inv_at_half() {
        let result = norm_inv(dec!(0.5));
        let diff = result.abs();
        assert!(
            diff < dec!(0.01),
            "norm_inv(0.5) should be ~0, got {}",
            result
        );
    }
}
