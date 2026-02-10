use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::error::CorpFinanceError;
use crate::types::{with_metadata, ComputationOutput, Money, Rate};
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Decimal math helpers (pure Decimal, no f64)
// ---------------------------------------------------------------------------

/// Iterative multiplication for (1 + r)^n where n is a positive integer.
/// For fractional exponents, falls back to exp(n * ln(1+r)).
fn compound_power(base: Decimal, exponent: Decimal) -> Decimal {
    // Check if exponent is a non-negative integer
    if exponent == exponent.trunc() && exponent >= Decimal::ZERO {
        let n = exponent.to_string().parse::<u64>().unwrap_or(0);
        if n == 0 {
            return Decimal::ONE;
        }
        let mut result = Decimal::ONE;
        for _ in 0..n {
            result *= base;
        }
        result
    } else {
        // Fractional or negative exponent: use exp(exponent * ln(base))
        exp_decimal(exponent * ln_decimal(base))
    }
}

/// Taylor series exp(x) with range reduction for |x| > 2.
fn exp_decimal(x: Decimal) -> Decimal {
    let two = Decimal::from(2);

    // Range reduction: find k such that |x / 2^k| <= 2
    let mut k: u32 = 0;
    let mut reduced = x;
    while reduced.abs() > two {
        reduced /= two;
        k += 1;
    }

    // Taylor series: exp(reduced) = sum_{n=0}^{24} reduced^n / n!
    let mut sum = Decimal::ONE;
    let mut term = Decimal::ONE;
    for n in 1..=25u64 {
        term *= reduced / Decimal::from(n);
        sum += term;
    }

    // Reverse the range reduction by repeated squaring
    for _ in 0..k {
        sum *= sum;
    }

    sum
}

/// Natural logarithm via Newton's method.
fn ln_decimal(x: Decimal) -> Decimal {
    if x <= Decimal::ZERO {
        return Decimal::ZERO;
    }
    if x == Decimal::ONE {
        return Decimal::ZERO;
    }

    let mut guess = Decimal::ZERO;
    let mut temp = x;
    let two = Decimal::from(2);
    let ln2_approx = dec!(0.6931471805599453);

    if temp > Decimal::ONE {
        while temp > two {
            temp /= two;
            guess += ln2_approx;
        }
    } else {
        while temp < Decimal::ONE {
            temp *= two;
            guess -= ln2_approx;
        }
    }

    // Newton iterations
    for _ in 0..20 {
        let ey = exp_decimal(guess);
        if ey.is_zero() {
            break;
        }
        guess = guess - Decimal::ONE + x / ey;
    }

    guess
}

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

/// Type of FX forward: deliverable or non-deliverable (NDF).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FxForwardType {
    /// Standard deliverable forward — physical exchange at maturity.
    Deliverable,
    /// Non-deliverable forward — cash settlement against a fixing rate.
    NonDeliverable,
}

// ---------------------------------------------------------------------------
// Function 1: price_fx_forward
// ---------------------------------------------------------------------------

/// Input for pricing an FX forward contract.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FxForwardInput {
    /// Current spot FX rate (domestic per foreign, e.g. 1.10 USD/EUR).
    pub spot_rate: Decimal,
    /// Domestic risk-free rate (annualised, decimal).
    pub domestic_rate: Rate,
    /// Foreign risk-free rate (annualised, decimal).
    pub foreign_rate: Rate,
    /// Time to delivery in years.
    pub time_to_expiry: Decimal,
    /// Notional amount in foreign currency.
    pub notional_foreign: Money,
    /// Deliverable or non-deliverable (NDF).
    pub forward_type: FxForwardType,
}

/// Output from FX forward pricing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FxForwardOutput {
    /// Forward FX rate: F = S * ((1+r_d)/(1+r_f))^T.
    pub forward_rate: Decimal,
    /// Forward points: F - S.
    pub forward_points: Decimal,
    /// Forward points expressed in pips: (F - S) * 10000.
    pub forward_points_pips: Decimal,
    /// Forward premium/discount annualised: ((F - S) / S) / T.
    pub forward_premium_discount: Decimal,
    /// Notional in domestic currency: notional_foreign * forward_rate.
    pub notional_domestic: Money,
    /// Present value of the forward contract (at inception, fair value = 0).
    pub present_value: Money,
    /// Implied rate differential: ln(F/S) / T.
    pub implied_rate_differential: Decimal,
    /// Whether covered interest parity holds within tolerance.
    pub covered_interest_parity_check: bool,
}

/// Price an FX forward using compound interest rate parity.
///
/// Forward rate: F = S * ((1 + r_d) / (1 + r_f))^T
///
/// For integer T, uses iterative multiplication. For fractional T,
/// uses exp(T * ln(ratio)).
pub fn price_fx_forward(
    input: &FxForwardInput,
) -> CorpFinanceResult<ComputationOutput<FxForwardOutput>> {
    let start = Instant::now();
    let mut warnings: Vec<String> = Vec::new();

    // -- Validation --
    if input.spot_rate <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "spot_rate".into(),
            reason: "Spot rate must be positive".into(),
        });
    }
    if input.time_to_expiry <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "time_to_expiry".into(),
            reason: "Time to expiry must be positive".into(),
        });
    }

    let one_plus_rf = Decimal::ONE + input.foreign_rate;
    if one_plus_rf <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "foreign_rate".into(),
            reason: "(1 + foreign_rate) must be positive for compound interest".into(),
        });
    }
    let one_plus_rd = Decimal::ONE + input.domestic_rate;
    if one_plus_rd <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "domestic_rate".into(),
            reason: "(1 + domestic_rate) must be positive for compound interest".into(),
        });
    }

    let s = input.spot_rate;
    let rd = input.domestic_rate;
    let rf = input.foreign_rate;
    let t = input.time_to_expiry;

    // -- Forward rate: F = S * ((1 + r_d) / (1 + r_f))^T --
    let ratio = one_plus_rd / one_plus_rf;
    let power = compound_power(ratio, t);
    let forward_rate = s * power;

    // -- Forward points --
    let forward_points = forward_rate - s;
    let forward_points_pips = forward_points * dec!(10000);

    // -- Annualised premium/discount: ((F - S) / S) / T --
    let forward_premium_discount = (forward_points / s) / t;

    // -- Notional domestic --
    let notional_domestic = input.notional_foreign * forward_rate;

    // -- Present value at inception --
    // At inception for a fair-value forward, PV = 0.
    // In general: PV = (F - K) * N / (1 + r_d)^T, where K = F at inception.
    // At inception K = F, so PV = 0.
    let present_value = Decimal::ZERO;

    // -- Implied rate differential --
    let implied_rate_differential = if t > Decimal::ZERO {
        ln_decimal(forward_rate / s) / t
    } else {
        Decimal::ZERO
    };

    // -- Covered interest parity check --
    // CIP: F/S = (1+r_d)^T / (1+r_f)^T
    // Check that the computed forward satisfies this within tolerance.
    let cip_lhs = forward_rate / s;
    let cip_rhs = power;
    let cip_diff = (cip_lhs - cip_rhs).abs();
    let covered_interest_parity_check = cip_diff < dec!(0.0001);

    // -- Warnings --
    if forward_premium_discount.abs() > dec!(0.10) {
        warnings.push(format!(
            "Annualised forward premium/discount of {:.4} exceeds 10%",
            forward_premium_discount
        ));
    }

    let rate_diff_bps = (rd - rf).abs() * dec!(10000);
    if rate_diff_bps > dec!(500) {
        warnings.push(format!(
            "Rate differential of {:.0} bps exceeds 500 bps",
            rate_diff_bps
        ));
    }

    let output = FxForwardOutput {
        forward_rate,
        forward_points,
        forward_points_pips,
        forward_premium_discount,
        notional_domestic,
        present_value,
        implied_rate_differential,
        covered_interest_parity_check,
    };

    let elapsed = start.elapsed().as_micros() as u64;
    Ok(with_metadata(
        "FX Forward Pricing via Covered Interest Rate Parity (Compound)",
        &serde_json::json!({
            "spot_rate": s.to_string(),
            "domestic_rate": rd.to_string(),
            "foreign_rate": rf.to_string(),
            "time_to_expiry": t.to_string(),
            "notional_foreign": input.notional_foreign.to_string(),
            "forward_type": format!("{:?}", input.forward_type),
        }),
        warnings,
        elapsed,
        output,
    ))
}

// ---------------------------------------------------------------------------
// Function 2: calculate_cross_rate
// ---------------------------------------------------------------------------

/// Input for calculating a cross rate from two currency pairs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossRateInput {
    /// First exchange rate, e.g. 1.10.
    pub rate1: Decimal,
    /// First pair label, e.g. "USD/EUR".
    pub rate1_pair: String,
    /// Second exchange rate, e.g. 150.0.
    pub rate2: Decimal,
    /// Second pair label, e.g. "USD/JPY".
    pub rate2_pair: String,
    /// Target pair, e.g. "EUR/JPY".
    pub target_pair: String,
}

/// Output from cross rate calculation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossRateOutput {
    /// The computed cross rate.
    pub cross_rate: Decimal,
    /// Bid-ask spread (not computed from spot alone; informational placeholder).
    pub bid_ask_spread: Option<Decimal>,
    /// Human-readable derivation of how the cross rate was computed.
    pub derivation: String,
}

/// Parse a currency pair string like "USD/EUR" into (base, quote).
fn parse_pair(pair: &str) -> CorpFinanceResult<(String, String)> {
    let parts: Vec<&str> = pair.split('/').collect();
    if parts.len() != 2 {
        return Err(CorpFinanceError::InvalidInput {
            field: "currency_pair".into(),
            reason: format!("Expected format 'CCY1/CCY2', got '{}'", pair),
        });
    }
    Ok((
        parts[0].trim().to_uppercase(),
        parts[1].trim().to_uppercase(),
    ))
}

/// Calculate a cross rate from two currency pairs sharing a common currency.
///
/// For example, given USD/EUR = 1.10 and USD/JPY = 150.0, derive EUR/JPY.
///
/// Interpretation: "USD/EUR = 1.10" means 1 EUR costs 1.10 USD, i.e. the rate
/// quotes how many units of the first (numerator) currency per one unit of the
/// second (denominator) currency.
///
/// Logic:
/// - Find the common currency between the two input pairs.
/// - Use algebraic manipulation to derive the target cross rate.
pub fn calculate_cross_rate(
    input: &CrossRateInput,
) -> CorpFinanceResult<ComputationOutput<CrossRateOutput>> {
    let start = Instant::now();
    let warnings: Vec<String> = Vec::new();

    // -- Validation --
    if input.rate1 <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "rate1".into(),
            reason: "Exchange rate must be positive".into(),
        });
    }
    if input.rate2 <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "rate2".into(),
            reason: "Exchange rate must be positive".into(),
        });
    }

    let (base1, quote1) = parse_pair(&input.rate1_pair)?;
    let (base2, quote2) = parse_pair(&input.rate2_pair)?;
    let (target_base, target_quote) = parse_pair(&input.target_pair)?;

    // Build a map of implied rates: we know base1/quote1 = rate1, etc.
    // We need to find target_base/target_quote.
    //
    // Strategy: express all currencies in terms of a common currency, then derive.
    // rate1 = base1 per 1 quote1  (base1/quote1)
    // rate2 = base2 per 1 quote2  (base2/quote2)
    //
    // We need: target_base per 1 target_quote

    let (cross_rate, derivation) = derive_cross_rate(
        &base1,
        &quote1,
        input.rate1,
        &base2,
        &quote2,
        input.rate2,
        &target_base,
        &target_quote,
    )?;

    let output = CrossRateOutput {
        cross_rate,
        bid_ask_spread: None,
        derivation,
    };

    let elapsed = start.elapsed().as_micros() as u64;
    Ok(with_metadata(
        "Cross Rate Derivation from Two Currency Pairs",
        &serde_json::json!({
            "rate1_pair": input.rate1_pair,
            "rate1": input.rate1.to_string(),
            "rate2_pair": input.rate2_pair,
            "rate2": input.rate2.to_string(),
            "target_pair": input.target_pair,
        }),
        warnings,
        elapsed,
        output,
    ))
}

/// Derive the cross rate given two known rates and the target pair.
///
/// We consider all combinations of how the common currency links the pairs.
fn derive_cross_rate(
    b1: &str,
    q1: &str,
    r1: Decimal,
    b2: &str,
    q2: &str,
    r2: Decimal,
    tb: &str,
    tq: &str,
) -> CorpFinanceResult<(Decimal, String)> {
    // Collect the four currencies involved in the two pairs.
    // The common currency appears in both pairs.
    //
    // We can express any currency X in terms of any other currency Y by
    // chaining the two known rates.
    //
    // For pair1 B1/Q1 = r1: 1 Q1 = r1 B1, or 1 B1 = (1/r1) Q1.
    // For pair2 B2/Q2 = r2: 1 Q2 = r2 B2, or 1 B2 = (1/r2) Q2.
    //
    // We want TB/TQ = how many TB per 1 TQ.

    // Helper: get the rate of currency X in terms of currency Y from a single pair.
    // Returns Some(rate) where rate = X per 1 Y.
    let rate_from_pair =
        |x: &str, y: &str, base: &str, quote: &str, rate: Decimal| -> Option<Decimal> {
            if x == base && y == quote {
                // base/quote = rate => base per 1 quote = rate => X per 1 Y = rate
                Some(rate)
            } else if x == quote && y == base {
                // We want quote per 1 base = 1/rate
                Some(Decimal::ONE / rate)
            } else {
                None
            }
        };

    // Direct: can we get TB/TQ from a single pair?
    if let Some(r) = rate_from_pair(tb, tq, b1, q1, r1) {
        return Ok((
            r,
            format!("{tb}/{tq} obtained directly from {b1}/{q1} = {r1}"),
        ));
    }
    if let Some(r) = rate_from_pair(tb, tq, b2, q2, r2) {
        return Ok((
            r,
            format!("{tb}/{tq} obtained directly from {b2}/{q2} = {r2}"),
        ));
    }

    // Indirect: find common currency C such that we can chain:
    //   TB per 1 C (from one pair) * C per 1 TQ (from the other pair)
    // = TB per 1 TQ

    // Try all four currencies as intermediary
    let all_currencies = [b1, q1, b2, q2];

    for &c in &all_currencies {
        // We need: TB per 1 C from one pair, and C per 1 TQ from the other pair.

        // Option A: TB/C from pair1, C/TQ from pair2
        if let (Some(tb_per_c), Some(c_per_tq)) = (
            rate_from_pair(tb, c, b1, q1, r1),
            rate_from_pair(c, tq, b2, q2, r2),
        ) {
            let cross = tb_per_c * c_per_tq;
            return Ok((
                cross,
                format!(
                    "{tb}/{tq} = ({tb}/{c}) * ({c}/{tq}) \
                     = ({b1}/{q1} derived {tb_per_c}) * ({b2}/{q2} derived {c_per_tq}) \
                     = {cross}"
                ),
            ));
        }

        // Option B: TB/C from pair2, C/TQ from pair1
        if let (Some(tb_per_c), Some(c_per_tq)) = (
            rate_from_pair(tb, c, b2, q2, r2),
            rate_from_pair(c, tq, b1, q1, r1),
        ) {
            let cross = tb_per_c * c_per_tq;
            return Ok((
                cross,
                format!(
                    "{tb}/{tq} = ({tb}/{c}) * ({c}/{tq}) \
                     = ({b2}/{q2} derived {tb_per_c}) * ({b1}/{q1} derived {c_per_tq}) \
                     = {cross}"
                ),
            ));
        }
    }

    Err(CorpFinanceError::InvalidInput {
        field: "target_pair".into(),
        reason: format!(
            "Cannot derive {tb}/{tq} from {b1}/{q1} and {b2}/{q2}: \
             no common currency path found"
        ),
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn tol() -> Decimal {
        dec!(0.01)
    }

    fn tight_tol() -> Decimal {
        dec!(0.001)
    }

    fn assert_approx(actual: Decimal, expected: Decimal, tolerance: Decimal, label: &str) {
        let diff = (actual - expected).abs();
        assert!(
            diff <= tolerance,
            "{label}: expected ~{expected}, got {actual} (diff={diff}, tol={tolerance})"
        );
    }

    // -----------------------------------------------------------------------
    // 1. Basic USD/EUR forward
    // -----------------------------------------------------------------------
    #[test]
    fn test_usd_eur_forward_basic() {
        // S = 1.10 USD/EUR, r_d(USD) = 5%, r_f(EUR) = 3%, T = 1
        // F = 1.10 * (1.05/1.03)^1 = 1.10 * 1.019417.. ~ 1.12136
        let input = FxForwardInput {
            spot_rate: dec!(1.10),
            domestic_rate: dec!(0.05),
            foreign_rate: dec!(0.03),
            time_to_expiry: Decimal::ONE,
            notional_foreign: dec!(1000000),
            forward_type: FxForwardType::Deliverable,
        };
        let result = price_fx_forward(&input).unwrap();
        let out = &result.result;

        let expected_fwd = dec!(1.10) * (dec!(1.05) / dec!(1.03));
        assert_approx(out.forward_rate, expected_fwd, tight_tol(), "USD/EUR fwd");
        assert!(out.forward_rate > dec!(1.10), "USD at premium vs EUR");
        assert!(out.covered_interest_parity_check, "CIP should hold");
    }

    // -----------------------------------------------------------------------
    // 2. Forward points calculation
    // -----------------------------------------------------------------------
    #[test]
    fn test_forward_points() {
        let input = FxForwardInput {
            spot_rate: dec!(1.10),
            domestic_rate: dec!(0.05),
            foreign_rate: dec!(0.03),
            time_to_expiry: Decimal::ONE,
            notional_foreign: dec!(1000000),
            forward_type: FxForwardType::Deliverable,
        };
        let result = price_fx_forward(&input).unwrap();
        let out = &result.result;

        let expected_points = out.forward_rate - dec!(1.10);
        assert_eq!(out.forward_points, expected_points);
        // Points should be positive (domestic rate > foreign rate)
        assert!(out.forward_points > Decimal::ZERO);

        // Pips = points * 10000
        let expected_pips = expected_points * dec!(10000);
        assert_approx(out.forward_points_pips, expected_pips, dec!(0.0001), "pips");
    }

    // -----------------------------------------------------------------------
    // 3. NDF forward type accepted
    // -----------------------------------------------------------------------
    #[test]
    fn test_ndf_forward() {
        let input = FxForwardInput {
            spot_rate: dec!(7.25),
            domestic_rate: dec!(0.04),
            foreign_rate: dec!(0.02),
            time_to_expiry: dec!(0.5),
            notional_foreign: dec!(5000000),
            forward_type: FxForwardType::NonDeliverable,
        };
        let result = price_fx_forward(&input).unwrap();
        let out = &result.result;

        // F = 7.25 * (1.04/1.02)^0.5
        assert!(out.forward_rate > dec!(7.25));
        assert_eq!(input.forward_type, FxForwardType::NonDeliverable);
    }

    // -----------------------------------------------------------------------
    // 4. Forward premium/discount annualised
    // -----------------------------------------------------------------------
    #[test]
    fn test_forward_premium_discount() {
        // Short expiry: T = 0.25 (3 months)
        let input = FxForwardInput {
            spot_rate: dec!(1.10),
            domestic_rate: dec!(0.05),
            foreign_rate: dec!(0.03),
            time_to_expiry: dec!(0.25),
            notional_foreign: dec!(1000000),
            forward_type: FxForwardType::Deliverable,
        };
        let result = price_fx_forward(&input).unwrap();
        let out = &result.result;

        // Premium should be roughly (r_d - r_f) = 2% annualised
        assert_approx(
            out.forward_premium_discount,
            dec!(0.02),
            dec!(0.005),
            "annualised premium",
        );
    }

    // -----------------------------------------------------------------------
    // 5. Negative interest rates (JPY/EUR scenario)
    // -----------------------------------------------------------------------
    #[test]
    fn test_negative_interest_rates() {
        // Domestic rate (EUR) = -0.5%, Foreign rate (USD) = 2%, S = 0.85
        let input = FxForwardInput {
            spot_rate: dec!(0.85),
            domestic_rate: dec!(-0.005),
            foreign_rate: dec!(0.02),
            time_to_expiry: Decimal::ONE,
            notional_foreign: dec!(1000000),
            forward_type: FxForwardType::Deliverable,
        };
        let result = price_fx_forward(&input).unwrap();
        let out = &result.result;

        // When domestic rate < foreign rate, forward < spot (discount)
        assert!(
            out.forward_rate < dec!(0.85),
            "EUR at discount when r_d < r_f"
        );
        assert!(out.forward_points < Decimal::ZERO);
        assert!(out.covered_interest_parity_check);
    }

    // -----------------------------------------------------------------------
    // 6. CIP check holds at inception
    // -----------------------------------------------------------------------
    #[test]
    fn test_cip_check_holds() {
        let input = FxForwardInput {
            spot_rate: dec!(150.0),
            domestic_rate: dec!(0.005),
            foreign_rate: dec!(0.04),
            time_to_expiry: dec!(2),
            notional_foreign: dec!(10000000),
            forward_type: FxForwardType::Deliverable,
        };
        let result = price_fx_forward(&input).unwrap();
        assert!(
            result.result.covered_interest_parity_check,
            "CIP must hold for a fair-value forward"
        );
    }

    // -----------------------------------------------------------------------
    // 7. Notional domestic calculation
    // -----------------------------------------------------------------------
    #[test]
    fn test_notional_domestic() {
        let input = FxForwardInput {
            spot_rate: dec!(1.10),
            domestic_rate: dec!(0.05),
            foreign_rate: dec!(0.03),
            time_to_expiry: Decimal::ONE,
            notional_foreign: dec!(1000000),
            forward_type: FxForwardType::Deliverable,
        };
        let result = price_fx_forward(&input).unwrap();
        let out = &result.result;

        let expected_notional = dec!(1000000) * out.forward_rate;
        assert_eq!(out.notional_domestic, expected_notional);
    }

    // -----------------------------------------------------------------------
    // 8. PV at inception is zero
    // -----------------------------------------------------------------------
    #[test]
    fn test_pv_at_inception_zero() {
        let input = FxForwardInput {
            spot_rate: dec!(1.10),
            domestic_rate: dec!(0.05),
            foreign_rate: dec!(0.03),
            time_to_expiry: Decimal::ONE,
            notional_foreign: dec!(1000000),
            forward_type: FxForwardType::Deliverable,
        };
        let result = price_fx_forward(&input).unwrap();
        assert_eq!(result.result.present_value, Decimal::ZERO);
    }

    // -----------------------------------------------------------------------
    // 9. Large rate differential triggers warning
    // -----------------------------------------------------------------------
    #[test]
    fn test_large_rate_differential_warning() {
        // 10% vs 2% = 800 bps differential
        let input = FxForwardInput {
            spot_rate: dec!(15.0),
            domestic_rate: dec!(0.10),
            foreign_rate: dec!(0.02),
            time_to_expiry: Decimal::ONE,
            notional_foreign: dec!(1000000),
            forward_type: FxForwardType::Deliverable,
        };
        let result = price_fx_forward(&input).unwrap();
        assert!(
            result.warnings.iter().any(|w| w.contains("bps")),
            "Should warn about large rate differential"
        );
    }

    // -----------------------------------------------------------------------
    // 10. Large premium triggers warning
    // -----------------------------------------------------------------------
    #[test]
    fn test_large_premium_warning() {
        // Very large rate diff over long horizon => big premium
        let input = FxForwardInput {
            spot_rate: dec!(1.0),
            domestic_rate: dec!(0.20),
            foreign_rate: dec!(0.01),
            time_to_expiry: dec!(2),
            notional_foreign: dec!(1000000),
            forward_type: FxForwardType::Deliverable,
        };
        let result = price_fx_forward(&input).unwrap();
        assert!(
            result.warnings.iter().any(|w| w.contains("premium")),
            "Should warn about large annualised premium"
        );
    }

    // -----------------------------------------------------------------------
    // 11. Cross rate EUR/JPY from USD/EUR and USD/JPY
    // -----------------------------------------------------------------------
    #[test]
    fn test_cross_rate_eur_jpy() {
        // USD/EUR = 1.10 (1 EUR = 1.10 USD)
        // USD/JPY = 150.0 (1 JPY = 150.0 USD) -- wait, that's wrong.
        // Convention: USD/JPY = 150 means 1 JPY costs 150 USD? No.
        // Standard: USD/JPY = 150 means 1 USD = 150 JPY, i.e. JPY/USD = 150.
        // But in our notation: "USD/JPY" = USD per 1 JPY = 1/150.
        //
        // Let's use a clearer convention matching our parse:
        // rate1_pair = "USD/EUR" with rate1 = 1.10 means 1.10 USD per 1 EUR
        // rate2_pair = "JPY/USD" with rate2 = 150.0 means 150 JPY per 1 USD
        // Target: "JPY/EUR" = JPY per 1 EUR
        //
        // JPY/EUR = (JPY/USD) * (USD/EUR) = 150 * 1.10 = 165
        let input = CrossRateInput {
            rate1: dec!(1.10),
            rate1_pair: "USD/EUR".to_string(),
            rate2: dec!(150.0),
            rate2_pair: "JPY/USD".to_string(),
            target_pair: "JPY/EUR".to_string(),
        };
        let result = calculate_cross_rate(&input).unwrap();
        let out = &result.result;

        // JPY/EUR = 150 * 1.10 = 165
        assert_approx(out.cross_rate, dec!(165.0), tol(), "EUR/JPY cross");
    }

    // -----------------------------------------------------------------------
    // 12. Cross rate inverse
    // -----------------------------------------------------------------------
    #[test]
    fn test_cross_rate_inverse() {
        // GBP/USD = 1.25 means 1.25 GBP per 1 USD
        // We want USD/GBP from the same pair
        // USD/GBP = 1 / 1.25 = 0.80
        let input = CrossRateInput {
            rate1: dec!(1.25),
            rate1_pair: "GBP/USD".to_string(),
            rate2: dec!(150.0),
            rate2_pair: "JPY/USD".to_string(),
            target_pair: "USD/GBP".to_string(),
        };
        let result = calculate_cross_rate(&input).unwrap();
        let out = &result.result;

        assert_approx(out.cross_rate, dec!(0.80), tol(), "USD/GBP inverse");
    }

    // -----------------------------------------------------------------------
    // 13. Cross rate with CHF
    // -----------------------------------------------------------------------
    #[test]
    fn test_cross_rate_chf() {
        // USD/CHF rate: 0.92 USD per 1 CHF
        // USD/EUR rate: 1.10 USD per 1 EUR
        // Target: CHF/EUR = CHF per 1 EUR
        //
        // CHF/EUR = (CHF/USD) * (USD/EUR) = (1/0.92) * 1.10 = 1.1957
        let input = CrossRateInput {
            rate1: dec!(0.92),
            rate1_pair: "USD/CHF".to_string(),
            rate2: dec!(1.10),
            rate2_pair: "USD/EUR".to_string(),
            target_pair: "CHF/EUR".to_string(),
        };
        let result = calculate_cross_rate(&input).unwrap();
        let out = &result.result;

        let expected = dec!(1.10) / dec!(0.92);
        assert_approx(out.cross_rate, expected, tol(), "CHF/EUR cross");
    }

    // -----------------------------------------------------------------------
    // 14. Validation: spot rate must be positive
    // -----------------------------------------------------------------------
    #[test]
    fn test_validation_spot_rate_positive() {
        let input = FxForwardInput {
            spot_rate: Decimal::ZERO,
            domestic_rate: dec!(0.05),
            foreign_rate: dec!(0.03),
            time_to_expiry: Decimal::ONE,
            notional_foreign: dec!(1000000),
            forward_type: FxForwardType::Deliverable,
        };
        let err = price_fx_forward(&input).unwrap_err();
        match err {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "spot_rate");
            }
            e => panic!("Expected InvalidInput for spot_rate, got {e:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // 15. Validation: time to expiry must be positive
    // -----------------------------------------------------------------------
    #[test]
    fn test_validation_time_positive() {
        let input = FxForwardInput {
            spot_rate: dec!(1.10),
            domestic_rate: dec!(0.05),
            foreign_rate: dec!(0.03),
            time_to_expiry: Decimal::ZERO,
            notional_foreign: dec!(1000000),
            forward_type: FxForwardType::Deliverable,
        };
        let err = price_fx_forward(&input).unwrap_err();
        match err {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "time_to_expiry");
            }
            e => panic!("Expected InvalidInput for time_to_expiry, got {e:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // 16. Cross rate validation: rate must be positive
    // -----------------------------------------------------------------------
    #[test]
    fn test_cross_rate_validation() {
        let input = CrossRateInput {
            rate1: Decimal::ZERO,
            rate1_pair: "USD/EUR".to_string(),
            rate2: dec!(150.0),
            rate2_pair: "USD/JPY".to_string(),
            target_pair: "EUR/JPY".to_string(),
        };
        let err = calculate_cross_rate(&input).unwrap_err();
        match err {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "rate1");
            }
            e => panic!("Expected InvalidInput for rate1, got {e:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // 17. Cross rate: invalid pair format
    // -----------------------------------------------------------------------
    #[test]
    fn test_cross_rate_invalid_pair_format() {
        let input = CrossRateInput {
            rate1: dec!(1.10),
            rate1_pair: "USDEUR".to_string(), // missing slash
            rate2: dec!(150.0),
            rate2_pair: "USD/JPY".to_string(),
            target_pair: "EUR/JPY".to_string(),
        };
        assert!(calculate_cross_rate(&input).is_err());
    }

    // -----------------------------------------------------------------------
    // 18. Implied rate differential
    // -----------------------------------------------------------------------
    #[test]
    fn test_implied_rate_differential() {
        let input = FxForwardInput {
            spot_rate: dec!(1.10),
            domestic_rate: dec!(0.05),
            foreign_rate: dec!(0.03),
            time_to_expiry: Decimal::ONE,
            notional_foreign: dec!(1000000),
            forward_type: FxForwardType::Deliverable,
        };
        let result = price_fx_forward(&input).unwrap();
        let out = &result.result;

        // Implied diff = ln(F/S) / T ~ r_d - r_f ~ 0.02
        assert_approx(
            out.implied_rate_differential,
            dec!(0.02),
            dec!(0.005),
            "implied rate diff",
        );
    }

    // -----------------------------------------------------------------------
    // 19. Metadata populated
    // -----------------------------------------------------------------------
    #[test]
    fn test_metadata_populated() {
        let input = FxForwardInput {
            spot_rate: dec!(1.10),
            domestic_rate: dec!(0.05),
            foreign_rate: dec!(0.03),
            time_to_expiry: Decimal::ONE,
            notional_foreign: dec!(1000000),
            forward_type: FxForwardType::Deliverable,
        };
        let result = price_fx_forward(&input).unwrap();

        assert!(result.methodology.contains("FX Forward"));
        assert_eq!(result.metadata.precision, "rust_decimal_128bit");
        assert!(!result.metadata.version.is_empty());
    }

    // -----------------------------------------------------------------------
    // 20. Multi-year forward with fractional time
    // -----------------------------------------------------------------------
    #[test]
    fn test_multi_year_fractional() {
        // T = 2.5 years
        let input = FxForwardInput {
            spot_rate: dec!(1.10),
            domestic_rate: dec!(0.04),
            foreign_rate: dec!(0.02),
            time_to_expiry: dec!(2.5),
            notional_foreign: dec!(1000000),
            forward_type: FxForwardType::Deliverable,
        };
        let result = price_fx_forward(&input).unwrap();
        let out = &result.result;

        // F = 1.10 * (1.04/1.02)^2.5
        // ratio = 1.04/1.02 ~ 1.01961
        // (1.01961)^2.5 ~ exp(2.5 * ln(1.01961)) ~ exp(2.5 * 0.01942) ~ exp(0.04855) ~ 1.04975
        // F ~ 1.10 * 1.04975 ~ 1.15473
        assert!(out.forward_rate > dec!(1.15));
        assert!(out.forward_rate < dec!(1.16));
        assert!(out.covered_interest_parity_check);
    }

    // -----------------------------------------------------------------------
    // 21. Cross rate no common currency error
    // -----------------------------------------------------------------------
    #[test]
    fn test_cross_rate_no_common_currency() {
        let input = CrossRateInput {
            rate1: dec!(1.10),
            rate1_pair: "USD/EUR".to_string(),
            rate2: dec!(0.85),
            rate2_pair: "GBP/CHF".to_string(),
            target_pair: "AUD/NZD".to_string(),
        };
        assert!(calculate_cross_rate(&input).is_err());
    }
}
