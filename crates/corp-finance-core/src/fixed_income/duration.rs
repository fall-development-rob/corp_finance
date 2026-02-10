use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::error::CorpFinanceError;
use crate::types::{with_metadata, ComputationOutput, Money, Rate};
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Input parameters for bond duration, convexity, and risk analytics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DurationInput {
    /// Par / face value of the bond (e.g. 1000)
    pub face_value: Money,
    /// Annual coupon rate as a decimal (0.05 = 5%)
    pub coupon_rate: Rate,
    /// Coupon payments per year: 1 (annual), 2 (semi), 4 (quarterly), 12 (monthly)
    pub coupon_frequency: u8,
    /// Yield to maturity as a decimal
    pub ytm: Rate,
    /// Years remaining until maturity
    pub years_to_maturity: Decimal,
    /// Yield shift in basis points for effective duration (default 10 bps)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub yield_shift_bps: Option<Decimal>,
    /// Tenors for key rate duration analysis (e.g. [1, 2, 5, 10, 30])
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key_rate_tenors: Option<Vec<Decimal>>,
}

/// Output of the duration and convexity calculation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DurationOutput {
    /// Weighted-average time of cash flows (in years)
    pub macaulay_duration: Decimal,
    /// Macaulay duration / (1 + y/freq) -- percentage price sensitivity
    pub modified_duration: Decimal,
    /// (P_down - P_up) / (2 * P_base * delta_y) -- model-free sensitivity
    pub effective_duration: Decimal,
    /// Second-order price sensitivity
    pub convexity: Decimal,
    /// Dollar value of one basis point (modified_duration * price * 0.0001)
    pub dv01: Money,
    /// Full present value of the bond at the stated YTM
    pub price: Money,
    /// Bond price when yield is shifted up by yield_shift_bps
    pub price_up: Money,
    /// Bond price when yield is shifted down by yield_shift_bps
    pub price_down: Money,
    /// Estimated price change (%) for a 100 bp parallel shift using duration + convexity
    pub price_change_estimate: Decimal,
    /// Key rate durations (if key_rate_tenors were provided)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key_rate_durations: Option<Vec<KeyRateDuration>>,
}

/// A single key rate duration result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyRateDuration {
    /// The tenor point (years)
    pub tenor: Decimal,
    /// Partial duration at this tenor
    pub duration: Decimal,
    /// This tenor's contribution as a percentage of total key rate duration
    pub contribution_pct: Rate,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Calculate duration, convexity, DV01, and related risk analytics for a
/// fixed-rate bullet bond.
///
/// Uses iterative discount-factor multiplication (never `powd()`) for full
/// decimal precision.
pub fn calculate_duration(
    input: &DurationInput,
) -> CorpFinanceResult<ComputationOutput<DurationOutput>> {
    let start = Instant::now();
    let warnings: Vec<String> = Vec::new();

    validate_input(input)?;

    let freq = Decimal::from(input.coupon_frequency);
    let total_periods = compute_total_periods(input);
    let coupon_per_period = input.face_value * input.coupon_rate / freq;
    let yield_per_period = input.ytm / freq;
    let shift_decimal = input.yield_shift_bps.unwrap_or(dec!(10)) * dec!(0.0001);

    // --- Core price at YTM ---
    let price = price_bond(
        coupon_per_period,
        input.face_value,
        yield_per_period,
        total_periods,
    )?;

    // --- Macaulay duration ---
    let macaulay_duration = compute_macaulay(
        coupon_per_period,
        input.face_value,
        yield_per_period,
        total_periods,
        freq,
        price,
    )?;

    // --- Modified duration ---
    let modified_duration = macaulay_duration / (Decimal::ONE + yield_per_period);

    // --- Effective duration (reprice with shifted yields) ---
    let yield_up = (input.ytm + shift_decimal) / freq;
    let yield_down = (input.ytm - shift_decimal) / freq;

    let price_up = price_bond(coupon_per_period, input.face_value, yield_up, total_periods)?;
    let price_down = price_bond(
        coupon_per_period,
        input.face_value,
        yield_down,
        total_periods,
    )?;

    let effective_duration = if price.is_zero() {
        return Err(CorpFinanceError::DivisionByZero {
            context: "effective duration: base price is zero".to_string(),
        });
    } else {
        (price_down - price_up) / (dec!(2) * price * shift_decimal)
    };

    // --- Convexity ---
    let convexity = compute_convexity(
        coupon_per_period,
        input.face_value,
        yield_per_period,
        total_periods,
        freq,
        price,
    )?;

    // --- DV01 ---
    let dv01 = modified_duration * price * dec!(0.0001);

    // --- Price change estimate for 100 bp parallel shift ---
    let delta_y = dec!(0.01);
    let price_change_estimate =
        -modified_duration * delta_y + dec!(0.5) * convexity * delta_y * delta_y;

    // --- Key rate durations (optional) ---
    let key_rate_durations = match &input.key_rate_tenors {
        Some(tenors) => Some(compute_key_rate_durations(
            input,
            coupon_per_period,
            total_periods,
            freq,
            price,
            shift_decimal,
            tenors,
        )?),
        None => None,
    };

    let output = DurationOutput {
        macaulay_duration,
        modified_duration,
        effective_duration,
        convexity,
        dv01,
        price,
        price_up,
        price_down,
        price_change_estimate,
        key_rate_durations,
    };

    let elapsed = start.elapsed().as_micros() as u64;
    let assumptions = serde_json::json!({
        "coupon_frequency": input.coupon_frequency,
        "yield_shift_bps": input.yield_shift_bps.unwrap_or(dec!(10)).to_string(),
        "settlement": "assumed on coupon date (no accrued interest)",
        "day_count": "30/360 (period-based)",
        "price_change_estimate_shift": "100 bps"
    });

    Ok(with_metadata(
        "Bond Duration & Convexity (CFA Fixed Income Analytics)",
        &assumptions,
        warnings,
        elapsed,
        output,
    ))
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn validate_input(input: &DurationInput) -> CorpFinanceResult<()> {
    if input.face_value <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "face_value".into(),
            reason: "Face value must be positive.".into(),
        });
    }
    if input.coupon_rate < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "coupon_rate".into(),
            reason: "Coupon rate must be non-negative.".into(),
        });
    }
    if !matches!(input.coupon_frequency, 1 | 2 | 4 | 12) {
        return Err(CorpFinanceError::InvalidInput {
            field: "coupon_frequency".into(),
            reason: "Coupon frequency must be 1, 2, 4, or 12.".into(),
        });
    }
    if input.ytm <= dec!(-1) {
        return Err(CorpFinanceError::InvalidInput {
            field: "ytm".into(),
            reason: "YTM must be greater than -1 (i.e. > -100%).".into(),
        });
    }
    if input.years_to_maturity <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "years_to_maturity".into(),
            reason: "Years to maturity must be positive.".into(),
        });
    }
    Ok(())
}

/// Compute the total number of coupon periods (integer).
fn compute_total_periods(input: &DurationInput) -> u32 {
    let periods = input.years_to_maturity * Decimal::from(input.coupon_frequency);
    // Round to nearest integer to handle fractional inputs like 10.0
    periods.round().to_string().parse::<u32>().unwrap_or(0)
}

/// Price a bullet bond using iterative discount factor multiplication.
///
/// PV = sum_{t=1..n} [ coupon / df_t ] + face_value / df_n
///
/// where df_t = df_{t-1} * (1 + y_per_period) (iterative, no powd).
fn price_bond(
    coupon: Money,
    face_value: Money,
    yield_per_period: Rate,
    total_periods: u32,
) -> CorpFinanceResult<Money> {
    let one_plus_y = Decimal::ONE + yield_per_period;
    if one_plus_y.is_zero() {
        return Err(CorpFinanceError::DivisionByZero {
            context: "bond pricing: (1 + yield_per_period) is zero".to_string(),
        });
    }

    let mut price = Decimal::ZERO;
    let mut df = Decimal::ONE; // discount factor accumulator

    for t in 1..=total_periods {
        df *= one_plus_y;
        let cf = if t == total_periods {
            coupon + face_value
        } else {
            coupon
        };
        price += cf / df;
    }

    Ok(price)
}

/// Compute Macaulay duration: sum of [t_years * PV(CF)] / Price.
///
/// t_years = period_index / coupon_frequency.
fn compute_macaulay(
    coupon: Money,
    face_value: Money,
    yield_per_period: Rate,
    total_periods: u32,
    freq: Decimal,
    price: Money,
) -> CorpFinanceResult<Decimal> {
    if price.is_zero() {
        return Err(CorpFinanceError::DivisionByZero {
            context: "Macaulay duration: bond price is zero".to_string(),
        });
    }

    let one_plus_y = Decimal::ONE + yield_per_period;
    let mut weighted_sum = Decimal::ZERO;
    let mut df = Decimal::ONE;

    for t in 1..=total_periods {
        df *= one_plus_y;
        let t_years = Decimal::from(t) / freq;
        let cf = if t == total_periods {
            coupon + face_value
        } else {
            coupon
        };
        let pv_cf = cf / df;
        weighted_sum += t_years * pv_cf;
    }

    Ok(weighted_sum / price)
}

/// Compute convexity: sum of [t * (t + 1/freq) * PV(CF)] / (Price * (1+y/f)^2).
///
/// Here t is measured in years. The denominator (1+y/f)^2 is computed via
/// iterative multiplication.
fn compute_convexity(
    coupon: Money,
    face_value: Money,
    yield_per_period: Rate,
    total_periods: u32,
    freq: Decimal,
    price: Money,
) -> CorpFinanceResult<Decimal> {
    if price.is_zero() {
        return Err(CorpFinanceError::DivisionByZero {
            context: "convexity: bond price is zero".to_string(),
        });
    }

    let one_plus_y = Decimal::ONE + yield_per_period;
    let mut numerator = Decimal::ZERO;
    let mut df = Decimal::ONE;

    for t in 1..=total_periods {
        df *= one_plus_y;
        let t_years = Decimal::from(t) / freq;
        let t_years_next = t_years + Decimal::ONE / freq;
        let cf = if t == total_periods {
            coupon + face_value
        } else {
            coupon
        };
        let pv_cf = cf / df;
        numerator += t_years * t_years_next * pv_cf;
    }

    // (1 + y/f)^2 via iterative multiplication
    let one_plus_y_sq = one_plus_y * one_plus_y;
    let denominator = price * one_plus_y_sq;

    if denominator.is_zero() {
        return Err(CorpFinanceError::DivisionByZero {
            context: "convexity: denominator is zero".to_string(),
        });
    }

    Ok(numerator / denominator)
}

/// Compute key rate durations by shifting individual tenor-point spot rates.
///
/// For each tenor point the algorithm:
/// 1. Builds a simple par-curve interpolation from YTM (flat baseline).
/// 2. Bumps the spot rate at the specified tenor by +/- shift.
/// 3. Interpolates the bump linearly to neighbouring tenors.
/// 4. Reprices the bond under each shifted curve.
/// 5. Computes partial duration = (P_down - P_up) / (2 * P_base * shift).
fn compute_key_rate_durations(
    input: &DurationInput,
    coupon: Money,
    total_periods: u32,
    freq: Decimal,
    base_price: Money,
    shift: Decimal,
    tenors: &[Decimal],
) -> CorpFinanceResult<Vec<KeyRateDuration>> {
    if base_price.is_zero() {
        return Err(CorpFinanceError::DivisionByZero {
            context: "key rate durations: base price is zero".to_string(),
        });
    }

    let mut results: Vec<KeyRateDuration> = Vec::with_capacity(tenors.len());

    for &tenor in tenors {
        // Price with yield curve shifted UP at this tenor
        let price_up =
            price_with_key_rate_shift(input, coupon, total_periods, freq, tenor, shift, tenors)?;
        // Price with yield curve shifted DOWN at this tenor
        let price_down =
            price_with_key_rate_shift(input, coupon, total_periods, freq, tenor, -shift, tenors)?;

        let partial_dur = (price_down - price_up) / (dec!(2) * base_price * shift);

        results.push(KeyRateDuration {
            tenor,
            duration: partial_dur,
            contribution_pct: Decimal::ZERO, // filled in below
        });
    }

    // Compute contribution percentages
    let total_krd: Decimal = results.iter().map(|r| r.duration).sum();
    if !total_krd.is_zero() {
        for r in &mut results {
            r.contribution_pct = r.duration / total_krd;
        }
    }

    Ok(results)
}

/// Reprice a bond after applying a key-rate shift at a specific tenor.
///
/// The shift is applied with linear interpolation: full weight at the target
/// tenor, tapering to zero at the adjacent tenor points (or bond boundaries).
fn price_with_key_rate_shift(
    input: &DurationInput,
    coupon: Money,
    total_periods: u32,
    freq: Decimal,
    target_tenor: Decimal,
    shift: Decimal,
    tenors: &[Decimal],
) -> CorpFinanceResult<Money> {
    let one_base = Decimal::ONE + input.ytm / freq;
    if one_base.is_zero() {
        return Err(CorpFinanceError::DivisionByZero {
            context: "key rate shift: (1 + ytm/freq) is zero".to_string(),
        });
    }

    // Find the neighbouring tenors for interpolation bounds
    let lower_bound = tenors
        .iter()
        .filter(|&&t| t < target_tenor)
        .copied()
        .last()
        .unwrap_or(Decimal::ZERO);
    let upper_bound = tenors
        .iter()
        .filter(|&&t| t > target_tenor)
        .copied()
        .next()
        .unwrap_or(input.years_to_maturity);

    let mut price = Decimal::ZERO;

    for t in 1..=total_periods {
        // We rebuild the discount factor from scratch for each period
        // because the shift varies by period (linear interpolation).
        // This is O(n^2) but n <= ~360 periods so acceptable.
        let mut local_df = Decimal::ONE;
        for s in 1..=t {
            let s_years = Decimal::from(s) / freq;
            let w = key_rate_weight(s_years, target_tenor, lower_bound, upper_bound);
            let s_shift = shift * w;
            let s_yield = input.ytm / freq + s_shift / freq;
            local_df *= Decimal::ONE + s_yield;
        }

        if local_df.is_zero() {
            return Err(CorpFinanceError::DivisionByZero {
                context: format!("key rate shift: discount factor is zero at period {t}"),
            });
        }

        let cf = if t == total_periods {
            coupon + input.face_value
        } else {
            coupon
        };
        price += cf / local_df;
    }

    Ok(price)
}

/// Linear interpolation weight for key-rate duration.
///
/// Returns 1.0 at target_tenor, tapering linearly to 0 at lower_bound and
/// upper_bound. Returns 0 outside the range.
fn key_rate_weight(t_years: Decimal, target: Decimal, lower: Decimal, upper: Decimal) -> Decimal {
    if t_years == target {
        Decimal::ONE
    } else if t_years > lower && t_years < target {
        let span = target - lower;
        if span.is_zero() {
            Decimal::ONE
        } else {
            (t_years - lower) / span
        }
    } else if t_years > target && t_years < upper {
        let span = upper - target;
        if span.is_zero() {
            Decimal::ONE
        } else {
            (upper - t_years) / span
        }
    } else {
        Decimal::ZERO
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    /// Standard 10-year, 5% semi-annual bond at 5% YTM (priced at par).
    fn par_bond_input() -> DurationInput {
        DurationInput {
            face_value: dec!(1000),
            coupon_rate: dec!(0.05),
            coupon_frequency: 2,
            ytm: dec!(0.05),
            years_to_maturity: dec!(10),
            yield_shift_bps: None,
            key_rate_tenors: None,
        }
    }

    /// Zero coupon 10-year bond.
    fn zero_coupon_input() -> DurationInput {
        DurationInput {
            face_value: dec!(1000),
            coupon_rate: dec!(0.0),
            coupon_frequency: 2,
            ytm: dec!(0.05),
            years_to_maturity: dec!(10),
            yield_shift_bps: None,
            key_rate_tenors: None,
        }
    }

    fn assert_close(actual: Decimal, expected: Decimal, tolerance: Decimal, label: &str) {
        let diff = (actual - expected).abs();
        assert!(
            diff <= tolerance,
            "{label}: expected ~{expected}, got {actual} (diff {diff} > tolerance {tolerance})"
        );
    }

    // --- Test 1: Zero coupon Macaulay = maturity ---
    #[test]
    fn test_zero_coupon_macaulay_equals_maturity() {
        let input = zero_coupon_input();
        let result = calculate_duration(&input).unwrap();
        let out = &result.result;

        assert_close(
            out.macaulay_duration,
            dec!(10),
            dec!(0.0001),
            "Zero-coupon Macaulay duration should equal maturity",
        );
    }

    // --- Test 2: Coupon bond Macaulay < maturity ---
    #[test]
    fn test_coupon_bond_macaulay_less_than_maturity() {
        let input = par_bond_input();
        let result = calculate_duration(&input).unwrap();
        let out = &result.result;

        assert!(
            out.macaulay_duration < dec!(10),
            "Coupon bond Macaulay ({}) should be less than maturity (10)",
            out.macaulay_duration
        );
        assert!(
            out.macaulay_duration > Decimal::ZERO,
            "Macaulay duration should be positive"
        );
    }

    // --- Test 3: Modified = Macaulay / (1 + y/freq) ---
    #[test]
    fn test_modified_duration_relationship() {
        let input = par_bond_input();
        let result = calculate_duration(&input).unwrap();
        let out = &result.result;

        let expected_mod = out.macaulay_duration / (Decimal::ONE + dec!(0.05) / dec!(2));
        assert_close(
            out.modified_duration,
            expected_mod,
            dec!(0.000001),
            "Modified duration = Macaulay / (1 + y/freq)",
        );
    }

    // --- Test 4: Effective ~ modified for option-free bond ---
    #[test]
    fn test_effective_vs_modified_close() {
        let input = par_bond_input();
        let result = calculate_duration(&input).unwrap();
        let out = &result.result;

        // For an option-free bullet bond, effective should be very close to modified
        assert_close(
            out.effective_duration,
            out.modified_duration,
            dec!(0.05),
            "Effective duration should approximate modified for option-free bonds",
        );
    }

    // --- Test 5: Higher coupon -> lower duration ---
    #[test]
    fn test_higher_coupon_lower_duration() {
        let low_coupon = par_bond_input(); // 5% coupon
        let mut high_coupon = par_bond_input();
        high_coupon.coupon_rate = dec!(0.08);

        let result_low = calculate_duration(&low_coupon).unwrap();
        let result_high = calculate_duration(&high_coupon).unwrap();

        assert!(
            result_high.result.macaulay_duration < result_low.result.macaulay_duration,
            "Higher coupon ({}) should have lower duration than lower coupon ({})",
            result_high.result.macaulay_duration,
            result_low.result.macaulay_duration
        );
    }

    // --- Test 6: Higher YTM -> lower duration ---
    #[test]
    fn test_higher_ytm_lower_duration() {
        let mut low_ytm = par_bond_input();
        low_ytm.ytm = dec!(0.03);

        let mut high_ytm = par_bond_input();
        high_ytm.ytm = dec!(0.08);

        let result_low = calculate_duration(&low_ytm).unwrap();
        let result_high = calculate_duration(&high_ytm).unwrap();

        assert!(
            result_high.result.macaulay_duration < result_low.result.macaulay_duration,
            "Higher YTM ({}) should have lower Macaulay duration than lower YTM ({})",
            result_high.result.macaulay_duration,
            result_low.result.macaulay_duration
        );
    }

    // --- Test 7: Longer maturity -> higher duration ---
    #[test]
    fn test_longer_maturity_higher_duration() {
        let short = DurationInput {
            years_to_maturity: dec!(5),
            ..par_bond_input()
        };
        let long = DurationInput {
            years_to_maturity: dec!(30),
            ..par_bond_input()
        };

        let result_short = calculate_duration(&short).unwrap();
        let result_long = calculate_duration(&long).unwrap();

        assert!(
            result_long.result.macaulay_duration > result_short.result.macaulay_duration,
            "30-year duration ({}) should exceed 5-year duration ({})",
            result_long.result.macaulay_duration,
            result_short.result.macaulay_duration
        );
    }

    // --- Test 8: Convexity positive ---
    #[test]
    fn test_convexity_positive() {
        let input = par_bond_input();
        let result = calculate_duration(&input).unwrap();

        assert!(
            result.result.convexity > Decimal::ZERO,
            "Convexity should be positive for a standard bond, got {}",
            result.result.convexity
        );
    }

    // --- Test 9: DV01 calculation ---
    #[test]
    fn test_dv01_calculation() {
        let input = par_bond_input();
        let result = calculate_duration(&input).unwrap();
        let out = &result.result;

        let expected_dv01 = out.modified_duration * out.price * dec!(0.0001);
        assert_close(
            out.dv01,
            expected_dv01,
            dec!(0.000001),
            "DV01 = modified_duration * price * 0.0001",
        );

        // For a par bond with ~7.8y modified duration and price ~1000
        // DV01 should be roughly 0.78
        assert!(
            out.dv01 > dec!(0.5) && out.dv01 < dec!(1.5),
            "DV01 for 10y par bond should be in a reasonable range, got {}",
            out.dv01
        );
    }

    // --- Test 10: Price change estimate ---
    #[test]
    fn test_price_change_estimate() {
        let input = par_bond_input();
        let result = calculate_duration(&input).unwrap();
        let out = &result.result;

        // For 100bp shift: estimate = -mod_dur * 0.01 + 0.5 * convexity * 0.01^2
        let delta_y = dec!(0.01);
        let expected =
            -out.modified_duration * delta_y + dec!(0.5) * out.convexity * delta_y * delta_y;

        assert_close(
            out.price_change_estimate,
            expected,
            dec!(0.000001),
            "Price change estimate formula",
        );

        // Should be negative (price falls when yield rises)
        assert!(
            out.price_change_estimate < Decimal::ZERO,
            "Price change estimate for +100bp should be negative, got {}",
            out.price_change_estimate
        );
    }

    // --- Test 11: Zero coupon convexity ---
    #[test]
    fn test_zero_coupon_convexity() {
        let input = zero_coupon_input();
        let result = calculate_duration(&input).unwrap();
        let out = &result.result;

        // Zero coupon convexity should be positive and roughly t*(t+1/f)/(1+y/f)^2
        assert!(
            out.convexity > Decimal::ZERO,
            "Zero-coupon convexity should be positive, got {}",
            out.convexity
        );

        // For a zero-coupon bond, convexity ~ T * (T + 1/f) / (1+y/f)^2
        // T=10, f=2: 10 * 10.5 / (1.025)^2 = 105 / 1.050625 ~ 99.93
        assert!(
            out.convexity > dec!(90) && out.convexity < dec!(110),
            "Zero-coupon 10y convexity should be ~100, got {}",
            out.convexity
        );
    }

    // --- Test 12: Semi-annual vs annual duration ---
    #[test]
    fn test_semiannual_vs_annual_duration() {
        let semi = par_bond_input(); // frequency = 2

        let annual = DurationInput {
            coupon_frequency: 1,
            ..par_bond_input()
        };

        let result_semi = calculate_duration(&semi).unwrap();
        let result_annual = calculate_duration(&annual).unwrap();

        // Semi-annual coupons arrive earlier => slightly lower Macaulay duration
        assert!(
            result_semi.result.macaulay_duration < result_annual.result.macaulay_duration,
            "Semi-annual Macaulay ({}) should be less than annual ({})",
            result_semi.result.macaulay_duration,
            result_annual.result.macaulay_duration
        );
    }

    // --- Test 13: Key rate durations sum ~ effective duration ---
    #[test]
    fn test_key_rate_durations_sum() {
        let input = DurationInput {
            key_rate_tenors: Some(vec![dec!(1), dec!(2), dec!(5), dec!(10)]),
            ..par_bond_input()
        };

        let result = calculate_duration(&input).unwrap();
        let out = &result.result;

        let krds = out
            .key_rate_durations
            .as_ref()
            .expect("key_rate_durations should be Some");
        assert_eq!(krds.len(), 4, "Should have 4 key rate tenors");

        let krd_sum: Decimal = krds.iter().map(|k| k.duration).sum();

        // The sum of key rate durations should approximate the effective duration.
        // Allow a wider tolerance because key-rate decomposition with a flat curve
        // and linear interpolation is an approximation.
        assert_close(
            krd_sum,
            out.effective_duration,
            dec!(1.0),
            "Sum of key rate durations should approximate effective duration",
        );

        // Contribution percentages should sum to ~1.0
        let contrib_sum: Decimal = krds.iter().map(|k| k.contribution_pct).sum();
        assert_close(
            contrib_sum,
            Decimal::ONE,
            dec!(0.01),
            "Contribution percentages should sum to ~1.0",
        );
    }

    // --- Test 14: Invalid face value error ---
    #[test]
    fn test_invalid_face_value_error() {
        let mut input = par_bond_input();
        input.face_value = dec!(-1000);

        let err = calculate_duration(&input).unwrap_err();
        match err {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "face_value");
            }
            other => panic!("Expected InvalidInput for face_value, got {other:?}"),
        }
    }

    // --- Test 15: Negative YTM works ---
    #[test]
    fn test_negative_ytm_works() {
        let input = DurationInput {
            ytm: dec!(-0.005), // -0.5% -- common in European sovereigns
            ..par_bond_input()
        };

        let result = calculate_duration(&input).unwrap();
        let out = &result.result;

        // Duration should still be positive
        assert!(
            out.macaulay_duration > Decimal::ZERO,
            "Macaulay duration should be positive even with negative YTM, got {}",
            out.macaulay_duration
        );

        // Price should be above par for negative yield with coupon
        assert!(
            out.price > input.face_value,
            "Price ({}) should exceed face value ({}) for negative YTM",
            out.price,
            input.face_value
        );
    }

    // --- Test 16: Metadata populated ---
    #[test]
    fn test_metadata_populated() {
        let input = par_bond_input();
        let result = calculate_duration(&input).unwrap();

        assert!(!result.methodology.is_empty());
        assert!(result.methodology.contains("Duration"));
        assert_eq!(result.metadata.precision, "rust_decimal_128bit");
        assert!(result.metadata.computation_time_us < 1_000_000); // under 1 second
    }

    // --- Test 17: Par bond prices at face value ---
    #[test]
    fn test_par_bond_price() {
        // When coupon rate == YTM, price should equal face value
        let input = par_bond_input();
        let result = calculate_duration(&input).unwrap();

        assert_close(
            result.result.price,
            dec!(1000),
            dec!(0.01),
            "Par bond (coupon == YTM) should price at face value",
        );
    }

    // --- Test 18: Invalid coupon frequency error ---
    #[test]
    fn test_invalid_coupon_frequency_error() {
        let mut input = par_bond_input();
        input.coupon_frequency = 3; // not 1, 2, 4, or 12

        let err = calculate_duration(&input).unwrap_err();
        match err {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "coupon_frequency");
            }
            other => panic!("Expected InvalidInput for coupon_frequency, got {other:?}"),
        }
    }

    // --- Test 19: Zero maturity rejected ---
    #[test]
    fn test_zero_maturity_rejected() {
        let mut input = par_bond_input();
        input.years_to_maturity = Decimal::ZERO;

        let err = calculate_duration(&input).unwrap_err();
        match err {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "years_to_maturity");
            }
            other => panic!("Expected InvalidInput for years_to_maturity, got {other:?}"),
        }
    }

    // --- Test 20: Monthly coupon frequency ---
    #[test]
    fn test_monthly_frequency() {
        let input = DurationInput {
            coupon_frequency: 12,
            ..par_bond_input()
        };

        let result = calculate_duration(&input).unwrap();
        let out = &result.result;

        // Monthly coupons arrive earlier => even lower duration than semi-annual
        let semi = calculate_duration(&par_bond_input()).unwrap();
        assert!(
            out.macaulay_duration < semi.result.macaulay_duration,
            "Monthly Macaulay ({}) should be less than semi-annual ({})",
            out.macaulay_duration,
            semi.result.macaulay_duration
        );
    }

    // --- Test 21: Price up < price base < price down ---
    #[test]
    fn test_price_shift_ordering() {
        let input = par_bond_input();
        let result = calculate_duration(&input).unwrap();
        let out = &result.result;

        assert!(
            out.price_up < out.price,
            "Price with yield shifted up ({}) should be less than base price ({})",
            out.price_up,
            out.price
        );
        assert!(
            out.price_down > out.price,
            "Price with yield shifted down ({}) should exceed base price ({})",
            out.price_down,
            out.price
        );
    }
}
