//! Sovereign bond pricing and analysis module.
//!
//! Provides institutional-grade analytics for government bonds including
//! dirty/clean pricing, YTM via Newton-Raphson, duration, convexity,
//! spread decomposition, and local currency risk adjustments.

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

use crate::error::CorpFinanceError;
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const NEWTON_MAX_ITERATIONS: u32 = 50;
const NEWTON_EPSILON: Decimal = dec!(0.0000000001);

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Input parameters for sovereign bond analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SovereignBondInput {
    /// Par / face value
    pub face_value: Decimal,
    /// Annual coupon rate as a decimal (e.g. 0.05 = 5%)
    pub coupon_rate: Decimal,
    /// Years to maturity
    pub maturity_years: Decimal,
    /// Coupons per year: 1 = annual, 2 = semi-annual, 4 = quarterly
    pub payment_frequency: u32,
    /// Benchmark government (risk-free) rate
    pub risk_free_rate: Decimal,
    /// Credit spread over risk-free rate
    pub sovereign_spread: Decimal,
    /// Currency denomination code
    pub currency: String,
    /// Issuing country name
    pub country: String,
    /// True for local currency, false for hard currency (USD/EUR)
    pub is_local_currency: bool,
    /// Optional CPI inflation rate for real yield calculation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inflation_rate: Option<Decimal>,
    /// Optional FX volatility for local currency risk adjustment
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fx_volatility: Option<Decimal>,
}

/// Risk metrics for a sovereign bond.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SovereignBondRiskMetrics {
    /// Portion of spread attributable to credit risk (~60%)
    pub credit_risk_premium: Decimal,
    /// Portion of spread attributable to liquidity risk (~25%)
    pub liquidity_premium: Decimal,
    /// Currency risk premium (if local currency bond)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub currency_risk_premium: Option<Decimal>,
    /// Sum of all risk premia
    pub total_risk_premium: Decimal,
}

/// Output of sovereign bond analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SovereignBondOutput {
    /// Full price including accrued interest
    pub dirty_price: Decimal,
    /// Quoted price (excludes accrued interest)
    pub clean_price: Decimal,
    /// Yield to maturity via Newton-Raphson
    pub yield_to_maturity: Decimal,
    /// Real yield via Fisher equation (if inflation provided)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub real_yield: Option<Decimal>,
    /// Modified duration (price sensitivity)
    pub modified_duration: Decimal,
    /// Convexity (second-order price sensitivity)
    pub convexity: Decimal,
    /// Spread duration
    pub spread_duration: Decimal,
    /// Zero-volatility spread
    pub z_spread: Decimal,
    /// Local currency premium over hard currency equivalent
    #[serde(skip_serializing_if = "Option::is_none")]
    pub local_currency_premium: Option<Decimal>,
    /// Risk decomposition metrics
    pub risk_metrics: SovereignBondRiskMetrics,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Analyze a sovereign bond: pricing, YTM, duration, convexity, spread
/// decomposition, and risk metrics.
pub fn analyze_sovereign_bond(
    input: &SovereignBondInput,
) -> CorpFinanceResult<SovereignBondOutput> {
    validate_input(input)?;

    let freq = Decimal::from(input.payment_frequency);
    let total_periods = compute_total_periods(input.maturity_years, input.payment_frequency);
    let coupon_per_period = input.face_value * input.coupon_rate / freq;
    let discount_rate = input.risk_free_rate + input.sovereign_spread;
    let periodic_rate = discount_rate / freq;

    // --- Price the bond (dirty price = PV of all cashflows) ---
    let dirty_price = price_bond_iterative(
        coupon_per_period,
        input.face_value,
        periodic_rate,
        total_periods,
    )?;

    // --- Accrued interest (assume settlement at period start, so zero) ---
    // For a general-purpose tool, we treat the price as if settled at
    // the beginning of a coupon period. Accrued = 0 means clean = dirty.
    let clean_price = dirty_price;

    // --- YTM via Newton-Raphson ---
    let yield_to_maturity = compute_ytm_newton(
        dirty_price,
        coupon_per_period,
        input.face_value,
        total_periods,
        freq,
    )?;

    // --- Real yield (Fisher equation) ---
    let real_yield = input.inflation_rate.map(|infl| {
        if Decimal::ONE + infl == Decimal::ZERO {
            Decimal::ZERO
        } else {
            (Decimal::ONE + yield_to_maturity) / (Decimal::ONE + infl) - Decimal::ONE
        }
    });

    // --- Macaulay duration ---
    let macaulay_duration = compute_macaulay_duration(
        coupon_per_period,
        input.face_value,
        periodic_rate,
        total_periods,
        freq,
        dirty_price,
    )?;

    // --- Modified duration ---
    let yield_per_period = yield_to_maturity / freq;
    let modified_duration = macaulay_duration / (Decimal::ONE + yield_per_period);

    // --- Convexity ---
    let convexity = compute_convexity(
        coupon_per_period,
        input.face_value,
        periodic_rate,
        total_periods,
        freq,
        dirty_price,
    )?;

    // --- Spread duration: modified duration for spread-sensitive bonds ---
    let spread_duration = modified_duration;

    // --- Z-spread via Newton-Raphson ---
    let z_spread = compute_z_spread(
        dirty_price,
        coupon_per_period,
        input.face_value,
        input.risk_free_rate,
        total_periods,
        freq,
    )?;

    // --- Local currency premium ---
    let local_currency_premium = if input.is_local_currency {
        match input.fx_volatility {
            Some(fx_vol) => {
                let sqrt_maturity = decimal_sqrt(input.maturity_years);
                Some(fx_vol * sqrt_maturity)
            }
            None => Some(Decimal::ZERO),
        }
    } else {
        None
    };

    // --- Risk decomposition ---
    let risk_metrics = decompose_risk(
        input.sovereign_spread,
        input.is_local_currency,
        &local_currency_premium,
    );

    Ok(SovereignBondOutput {
        dirty_price,
        clean_price,
        yield_to_maturity,
        real_yield,
        modified_duration,
        convexity,
        spread_duration,
        z_spread,
        local_currency_premium,
        risk_metrics,
    })
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

fn validate_input(input: &SovereignBondInput) -> CorpFinanceResult<()> {
    if input.face_value <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "face_value".into(),
            reason: "Face value must be positive".into(),
        });
    }
    if input.coupon_rate < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "coupon_rate".into(),
            reason: "Coupon rate cannot be negative".into(),
        });
    }
    if input.maturity_years <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "maturity_years".into(),
            reason: "Maturity must be positive".into(),
        });
    }
    if !matches!(input.payment_frequency, 1 | 2 | 4) {
        return Err(CorpFinanceError::InvalidInput {
            field: "payment_frequency".into(),
            reason: "Payment frequency must be 1, 2, or 4".into(),
        });
    }
    if input.risk_free_rate < dec!(-0.10) {
        return Err(CorpFinanceError::InvalidInput {
            field: "risk_free_rate".into(),
            reason: "Risk-free rate must be >= -10%".into(),
        });
    }
    if input.sovereign_spread < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "sovereign_spread".into(),
            reason: "Sovereign spread cannot be negative".into(),
        });
    }
    if input.country.is_empty() {
        return Err(CorpFinanceError::InvalidInput {
            field: "country".into(),
            reason: "Country name is required".into(),
        });
    }
    if input.currency.is_empty() {
        return Err(CorpFinanceError::InvalidInput {
            field: "currency".into(),
            reason: "Currency code is required".into(),
        });
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Compute total coupon periods (rounded to nearest integer).
fn compute_total_periods(maturity_years: Decimal, frequency: u32) -> u32 {
    let periods = maturity_years * Decimal::from(frequency);
    periods.round().to_string().parse::<u32>().unwrap_or(0)
}

/// Price a bond using iterative discount factor multiplication (never powd).
///
/// PV = sum_{t=1..n} [ coupon / df_t ] + face_value / df_n
fn price_bond_iterative(
    coupon: Decimal,
    face_value: Decimal,
    yield_per_period: Decimal,
    total_periods: u32,
) -> CorpFinanceResult<Decimal> {
    let one_plus_y = Decimal::ONE + yield_per_period;
    if one_plus_y.is_zero() {
        return Err(CorpFinanceError::DivisionByZero {
            context: "bond pricing: (1 + yield_per_period) is zero".into(),
        });
    }

    let mut price = Decimal::ZERO;
    let mut df = Decimal::ONE;

    for t in 1..=total_periods {
        df *= one_plus_y;
        if df.is_zero() {
            return Err(CorpFinanceError::DivisionByZero {
                context: format!("bond pricing: discount factor is zero at period {t}"),
            });
        }
        let cf = if t == total_periods {
            coupon + face_value
        } else {
            coupon
        };
        price += cf / df;
    }

    Ok(price)
}

/// Compute YTM via Newton-Raphson method.
///
/// Solves for y such that:
///   price = sum_{t=1..n} coupon / (1+y/freq)^t + face / (1+y/freq)^n
fn compute_ytm_newton(
    target_price: Decimal,
    coupon: Decimal,
    face_value: Decimal,
    total_periods: u32,
    freq: Decimal,
) -> CorpFinanceResult<Decimal> {
    if target_price <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "price".into(),
            reason: "Target price must be positive for YTM calculation".into(),
        });
    }

    // Initial guess: current yield as starting point
    let annual_coupon = coupon * freq;
    let mut y = if target_price > Decimal::ZERO {
        annual_coupon / target_price
    } else {
        dec!(0.05)
    };

    for iteration in 0..NEWTON_MAX_ITERATIONS {
        let periodic_y = y / freq;
        let one_plus_py = Decimal::ONE + periodic_y;

        if one_plus_py <= Decimal::ZERO {
            y = dec!(0.01);
            continue;
        }

        // Compute price and its derivative at current y
        let mut price_val = Decimal::ZERO;
        let mut dprice_val = Decimal::ZERO;
        let mut df = Decimal::ONE;

        for t in 1..=total_periods {
            df *= one_plus_py;
            if df.is_zero() {
                break;
            }
            let cf = if t == total_periods {
                coupon + face_value
            } else {
                coupon
            };
            price_val += cf / df;
            // d/dy (cf / (1+y/f)^t) = -t/f * cf / (1+y/f)^(t+1)
            let t_dec = Decimal::from(t);
            dprice_val -= t_dec / freq * cf / (df * one_plus_py);
        }

        let f_val = price_val - target_price;

        if f_val.abs() < NEWTON_EPSILON {
            return Ok(y);
        }

        if dprice_val.is_zero() {
            return Err(CorpFinanceError::ConvergenceFailure {
                function: "YTM Newton-Raphson".into(),
                iterations: iteration,
                last_delta: f_val,
            });
        }

        y -= f_val / dprice_val;

        // Guard against divergence
        if y < dec!(-0.50) {
            y = dec!(-0.50);
        } else if y > dec!(2.0) {
            y = dec!(2.0);
        }
    }

    Err(CorpFinanceError::ConvergenceFailure {
        function: "YTM Newton-Raphson".into(),
        iterations: NEWTON_MAX_ITERATIONS,
        last_delta: Decimal::ZERO,
    })
}

/// Compute Macaulay duration: sum of [t_years * PV(CF)] / Price.
fn compute_macaulay_duration(
    coupon: Decimal,
    face_value: Decimal,
    yield_per_period: Decimal,
    total_periods: u32,
    freq: Decimal,
    price: Decimal,
) -> CorpFinanceResult<Decimal> {
    if price.is_zero() {
        return Err(CorpFinanceError::DivisionByZero {
            context: "Macaulay duration: bond price is zero".into(),
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

/// Compute convexity: sum of [t*(t+1/f) * PV(CF)] / (Price * (1+y/f)^2).
fn compute_convexity(
    coupon: Decimal,
    face_value: Decimal,
    yield_per_period: Decimal,
    total_periods: u32,
    freq: Decimal,
    price: Decimal,
) -> CorpFinanceResult<Decimal> {
    if price.is_zero() {
        return Err(CorpFinanceError::DivisionByZero {
            context: "convexity: bond price is zero".into(),
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

    let one_plus_y_sq = one_plus_y * one_plus_y;
    let denominator = price * one_plus_y_sq;

    if denominator.is_zero() {
        return Err(CorpFinanceError::DivisionByZero {
            context: "convexity: denominator is zero".into(),
        });
    }

    Ok(numerator / denominator)
}

/// Compute z-spread via Newton-Raphson.
///
/// Finds z such that:
///   price = sum_{t=1..n} CF_t / (1 + (r + z)/freq)^t
/// where r is the risk-free rate.
fn compute_z_spread(
    target_price: Decimal,
    coupon: Decimal,
    face_value: Decimal,
    risk_free_rate: Decimal,
    total_periods: u32,
    freq: Decimal,
) -> CorpFinanceResult<Decimal> {
    let mut z = dec!(0.01); // initial guess: 100 bps

    for iteration in 0..NEWTON_MAX_ITERATIONS {
        let periodic_rate = (risk_free_rate + z) / freq;
        let one_plus_r = Decimal::ONE + periodic_rate;

        if one_plus_r <= Decimal::ZERO {
            z = dec!(0.001);
            continue;
        }

        let mut price_val = Decimal::ZERO;
        let mut dprice_val = Decimal::ZERO;
        let mut df = Decimal::ONE;

        for t in 1..=total_periods {
            df *= one_plus_r;
            if df.is_zero() {
                break;
            }
            let cf = if t == total_periods {
                coupon + face_value
            } else {
                coupon
            };
            price_val += cf / df;
            let t_dec = Decimal::from(t);
            dprice_val -= t_dec / freq * cf / (df * one_plus_r);
        }

        let f_val = price_val - target_price;

        if f_val.abs() < NEWTON_EPSILON {
            return Ok(z);
        }

        if dprice_val.is_zero() {
            return Err(CorpFinanceError::ConvergenceFailure {
                function: "Z-spread Newton-Raphson".into(),
                iterations: iteration,
                last_delta: f_val,
            });
        }

        z -= f_val / dprice_val;

        if z < dec!(-0.50) {
            z = dec!(-0.50);
        } else if z > dec!(5.0) {
            z = dec!(5.0);
        }
    }

    Err(CorpFinanceError::ConvergenceFailure {
        function: "Z-spread Newton-Raphson".into(),
        iterations: NEWTON_MAX_ITERATIONS,
        last_delta: Decimal::ZERO,
    })
}

/// Square root via Newton's method (20 iterations).
fn decimal_sqrt(x: Decimal) -> Decimal {
    if x <= Decimal::ZERO {
        return Decimal::ZERO;
    }
    if x == Decimal::ONE {
        return Decimal::ONE;
    }

    let mut guess = x / dec!(2);
    if guess.is_zero() {
        guess = dec!(0.001);
    }

    for _ in 0..20 {
        if guess.is_zero() {
            return Decimal::ZERO;
        }
        guess = (guess + x / guess) / dec!(2);
    }

    guess
}

/// Decompose sovereign spread into credit, liquidity, and currency risk components.
fn decompose_risk(
    sovereign_spread: Decimal,
    is_local_currency: bool,
    local_currency_premium: &Option<Decimal>,
) -> SovereignBondRiskMetrics {
    if is_local_currency {
        let currency_premium = local_currency_premium.unwrap_or(Decimal::ZERO);
        // Remaining spread after currency premium
        let remaining = if sovereign_spread > currency_premium {
            sovereign_spread - currency_premium
        } else {
            sovereign_spread
        };
        let credit = remaining * dec!(0.60);
        let liquidity = remaining * dec!(0.25);
        // Adjust currency premium: if spread < currency premium, cap it
        let actual_currency = if sovereign_spread > currency_premium {
            currency_premium
        } else {
            sovereign_spread * dec!(0.15)
        };
        let total = credit + liquidity + actual_currency;

        SovereignBondRiskMetrics {
            credit_risk_premium: credit,
            liquidity_premium: liquidity,
            currency_risk_premium: Some(actual_currency),
            total_risk_premium: total,
        }
    } else {
        let credit = sovereign_spread * dec!(0.60);
        let liquidity = sovereign_spread * dec!(0.25);
        let remainder = sovereign_spread - credit - liquidity;
        let total = credit + liquidity + remainder;

        SovereignBondRiskMetrics {
            credit_risk_premium: credit,
            liquidity_premium: liquidity,
            currency_risk_premium: None,
            total_risk_premium: total,
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn assert_close(actual: Decimal, expected: Decimal, tolerance: Decimal, label: &str) {
        let diff = (actual - expected).abs();
        assert!(
            diff <= tolerance,
            "{label}: expected ~{expected}, got {actual} (diff {diff} > tolerance {tolerance})"
        );
    }

    /// Standard 10-year sovereign bond (hard currency, semi-annual)
    fn standard_sovereign() -> SovereignBondInput {
        SovereignBondInput {
            face_value: dec!(1000),
            coupon_rate: dec!(0.05),
            maturity_years: dec!(10),
            payment_frequency: 2,
            risk_free_rate: dec!(0.03),
            sovereign_spread: dec!(0.02),
            currency: "USD".into(),
            country: "Brazil".into(),
            is_local_currency: false,
            inflation_rate: None,
            fx_volatility: None,
        }
    }

    /// High-yield emerging market bond
    fn high_yield_sovereign() -> SovereignBondInput {
        SovereignBondInput {
            face_value: dec!(1000),
            coupon_rate: dec!(0.10),
            maturity_years: dec!(5),
            payment_frequency: 2,
            risk_free_rate: dec!(0.04),
            sovereign_spread: dec!(0.08),
            currency: "USD".into(),
            country: "Argentina".into(),
            is_local_currency: false,
            inflation_rate: Some(dec!(0.25)),
            fx_volatility: None,
        }
    }

    /// Local currency bond
    fn local_currency_sovereign() -> SovereignBondInput {
        SovereignBondInput {
            face_value: dec!(1000),
            coupon_rate: dec!(0.08),
            maturity_years: dec!(7),
            payment_frequency: 2,
            risk_free_rate: dec!(0.04),
            sovereign_spread: dec!(0.04),
            currency: "BRL".into(),
            country: "Brazil".into(),
            is_local_currency: true,
            inflation_rate: Some(dec!(0.04)),
            fx_volatility: Some(dec!(0.15)),
        }
    }

    // -----------------------------------------------------------------------
    // 1. Basic pricing: par bond
    // -----------------------------------------------------------------------
    #[test]
    fn test_par_bond_pricing() {
        // When coupon rate == discount rate, price should be near par
        let input = SovereignBondInput {
            coupon_rate: dec!(0.05),
            risk_free_rate: dec!(0.03),
            sovereign_spread: dec!(0.02),
            ..standard_sovereign()
        };
        let result = analyze_sovereign_bond(&input).unwrap();

        assert_close(
            result.dirty_price,
            dec!(1000),
            dec!(1.0),
            "Par bond (coupon=discount) price should be ~1000",
        );
    }

    // -----------------------------------------------------------------------
    // 2. Premium bond: coupon > discount rate
    // -----------------------------------------------------------------------
    #[test]
    fn test_premium_bond_pricing() {
        let input = SovereignBondInput {
            coupon_rate: dec!(0.07),
            risk_free_rate: dec!(0.03),
            sovereign_spread: dec!(0.02),
            ..standard_sovereign()
        };
        let result = analyze_sovereign_bond(&input).unwrap();

        assert!(
            result.dirty_price > dec!(1000),
            "Premium bond should price above par, got {}",
            result.dirty_price
        );
    }

    // -----------------------------------------------------------------------
    // 3. Discount bond: coupon < discount rate
    // -----------------------------------------------------------------------
    #[test]
    fn test_discount_bond_pricing() {
        let input = SovereignBondInput {
            coupon_rate: dec!(0.03),
            risk_free_rate: dec!(0.03),
            sovereign_spread: dec!(0.02),
            ..standard_sovereign()
        };
        let result = analyze_sovereign_bond(&input).unwrap();

        assert!(
            result.dirty_price < dec!(1000),
            "Discount bond should price below par, got {}",
            result.dirty_price
        );
    }

    // -----------------------------------------------------------------------
    // 4. Zero coupon sovereign
    // -----------------------------------------------------------------------
    #[test]
    fn test_zero_coupon_sovereign() {
        let input = SovereignBondInput {
            coupon_rate: dec!(0),
            maturity_years: dec!(5),
            risk_free_rate: dec!(0.03),
            sovereign_spread: dec!(0.02),
            ..standard_sovereign()
        };
        let result = analyze_sovereign_bond(&input).unwrap();

        // Price = 1000 / (1.025)^10
        let mut factor = Decimal::ONE;
        for _ in 0..10 {
            factor *= dec!(1.025);
        }
        let expected = dec!(1000) / factor;

        assert_close(
            result.dirty_price,
            expected,
            dec!(0.01),
            "Zero coupon sovereign price",
        );
    }

    // -----------------------------------------------------------------------
    // 5. YTM convergence for par bond
    // -----------------------------------------------------------------------
    #[test]
    fn test_ytm_convergence_par_bond() {
        let input = SovereignBondInput {
            coupon_rate: dec!(0.05),
            risk_free_rate: dec!(0.03),
            sovereign_spread: dec!(0.02),
            ..standard_sovereign()
        };
        let result = analyze_sovereign_bond(&input).unwrap();

        // YTM should be approximately equal to discount rate = 5%
        assert_close(
            result.yield_to_maturity,
            dec!(0.05),
            dec!(0.001),
            "YTM for par bond should match discount rate",
        );
    }

    // -----------------------------------------------------------------------
    // 6. YTM convergence for high-yield bond
    // -----------------------------------------------------------------------
    #[test]
    fn test_ytm_convergence_high_yield() {
        let input = high_yield_sovereign();
        let result = analyze_sovereign_bond(&input).unwrap();

        // Discount rate = 4% + 8% = 12%
        // Coupon = 10% < 12%, so bond is at discount and YTM > coupon rate
        assert_close(
            result.yield_to_maturity,
            dec!(0.12),
            dec!(0.005),
            "YTM for high-yield should approximate discount rate",
        );
    }

    // -----------------------------------------------------------------------
    // 7. Real yield calculation (Fisher equation)
    // -----------------------------------------------------------------------
    #[test]
    fn test_real_yield_fisher() {
        let input = SovereignBondInput {
            inflation_rate: Some(dec!(0.03)),
            ..standard_sovereign()
        };
        let result = analyze_sovereign_bond(&input).unwrap();

        // Real yield = (1 + nominal) / (1 + inflation) - 1
        let nominal = result.yield_to_maturity;
        let expected_real = (Decimal::ONE + nominal) / (Decimal::ONE + dec!(0.03)) - Decimal::ONE;

        let real = result.real_yield.expect("Real yield should be present");
        assert_close(real, expected_real, dec!(0.001), "Fisher real yield");
    }

    // -----------------------------------------------------------------------
    // 8. Real yield with high inflation
    // -----------------------------------------------------------------------
    #[test]
    fn test_real_yield_high_inflation() {
        let input = high_yield_sovereign();
        let result = analyze_sovereign_bond(&input).unwrap();

        let real = result.real_yield.expect("Real yield should be present");
        // Nominal ~12%, inflation 25% => real yield should be negative
        assert!(
            real < Decimal::ZERO,
            "Real yield should be negative when inflation > nominal, got {}",
            real
        );
    }

    // -----------------------------------------------------------------------
    // 9. No real yield without inflation input
    // -----------------------------------------------------------------------
    #[test]
    fn test_no_real_yield_without_inflation() {
        let input = standard_sovereign();
        let result = analyze_sovereign_bond(&input).unwrap();

        assert!(
            result.real_yield.is_none(),
            "Real yield should be None when no inflation provided"
        );
    }

    // -----------------------------------------------------------------------
    // 10. Modified duration positive
    // -----------------------------------------------------------------------
    #[test]
    fn test_modified_duration_positive() {
        let result = analyze_sovereign_bond(&standard_sovereign()).unwrap();

        assert!(
            result.modified_duration > Decimal::ZERO,
            "Modified duration should be positive, got {}",
            result.modified_duration
        );
    }

    // -----------------------------------------------------------------------
    // 11. Modified duration less than maturity for coupon bond
    // -----------------------------------------------------------------------
    #[test]
    fn test_modified_duration_less_than_maturity() {
        let input = standard_sovereign();
        let result = analyze_sovereign_bond(&input).unwrap();

        assert!(
            result.modified_duration < input.maturity_years,
            "Modified duration ({}) should be less than maturity ({})",
            result.modified_duration,
            input.maturity_years
        );
    }

    // -----------------------------------------------------------------------
    // 12. Higher coupon -> lower duration
    // -----------------------------------------------------------------------
    #[test]
    fn test_higher_coupon_lower_duration() {
        let low_coupon = standard_sovereign();
        let high_coupon = SovereignBondInput {
            coupon_rate: dec!(0.10),
            ..standard_sovereign()
        };

        let result_low = analyze_sovereign_bond(&low_coupon).unwrap();
        let result_high = analyze_sovereign_bond(&high_coupon).unwrap();

        assert!(
            result_high.modified_duration < result_low.modified_duration,
            "Higher coupon duration ({}) should be less than lower coupon ({})",
            result_high.modified_duration,
            result_low.modified_duration
        );
    }

    // -----------------------------------------------------------------------
    // 13. Convexity positive
    // -----------------------------------------------------------------------
    #[test]
    fn test_convexity_positive() {
        let result = analyze_sovereign_bond(&standard_sovereign()).unwrap();

        assert!(
            result.convexity > Decimal::ZERO,
            "Convexity should be positive, got {}",
            result.convexity
        );
    }

    // -----------------------------------------------------------------------
    // 14. Convexity magnitude reasonable for 10y bond
    // -----------------------------------------------------------------------
    #[test]
    fn test_convexity_magnitude() {
        let result = analyze_sovereign_bond(&standard_sovereign()).unwrap();

        // For a 10-year semi-annual bond, convexity is typically 50-120
        assert!(
            result.convexity > dec!(30) && result.convexity < dec!(150),
            "10y bond convexity should be in reasonable range, got {}",
            result.convexity
        );
    }

    // -----------------------------------------------------------------------
    // 15. Spread duration equals modified duration
    // -----------------------------------------------------------------------
    #[test]
    fn test_spread_duration() {
        let result = analyze_sovereign_bond(&standard_sovereign()).unwrap();

        assert_eq!(
            result.spread_duration, result.modified_duration,
            "Spread duration should equal modified duration"
        );
    }

    // -----------------------------------------------------------------------
    // 16. Z-spread convergence
    // -----------------------------------------------------------------------
    #[test]
    fn test_z_spread_convergence() {
        let result = analyze_sovereign_bond(&standard_sovereign()).unwrap();

        // Z-spread should be close to the sovereign spread
        assert_close(
            result.z_spread,
            dec!(0.02),
            dec!(0.005),
            "Z-spread should approximate the sovereign spread",
        );
    }

    // -----------------------------------------------------------------------
    // 17. Z-spread for zero spread
    // -----------------------------------------------------------------------
    #[test]
    fn test_z_spread_zero_spread() {
        let input = SovereignBondInput {
            sovereign_spread: dec!(0),
            coupon_rate: dec!(0.03), // coupon = risk-free
            ..standard_sovereign()
        };
        let result = analyze_sovereign_bond(&input).unwrap();

        assert_close(
            result.z_spread,
            Decimal::ZERO,
            dec!(0.002),
            "Z-spread should be ~0 when spread is 0",
        );
    }

    // -----------------------------------------------------------------------
    // 18. Local currency premium present
    // -----------------------------------------------------------------------
    #[test]
    fn test_local_currency_premium_present() {
        let result = analyze_sovereign_bond(&local_currency_sovereign()).unwrap();

        let premium = result
            .local_currency_premium
            .expect("Local currency premium should be present");
        assert!(
            premium > Decimal::ZERO,
            "Local currency premium should be positive, got {}",
            premium
        );
    }

    // -----------------------------------------------------------------------
    // 19. Local currency premium calculation
    // -----------------------------------------------------------------------
    #[test]
    fn test_local_currency_premium_value() {
        let input = local_currency_sovereign();
        let result = analyze_sovereign_bond(&input).unwrap();

        // Premium = fx_volatility * sqrt(maturity) = 0.15 * sqrt(7)
        let expected = dec!(0.15) * decimal_sqrt(dec!(7));
        let premium = result.local_currency_premium.unwrap();

        assert_close(
            premium,
            expected,
            dec!(0.001),
            "Local currency premium = fx_vol * sqrt(maturity)",
        );
    }

    // -----------------------------------------------------------------------
    // 20. No local currency premium for hard currency
    // -----------------------------------------------------------------------
    #[test]
    fn test_no_local_currency_premium_hard_currency() {
        let result = analyze_sovereign_bond(&standard_sovereign()).unwrap();

        assert!(
            result.local_currency_premium.is_none(),
            "Hard currency bond should have no local currency premium"
        );
    }

    // -----------------------------------------------------------------------
    // 21. Risk decomposition: credit is ~60% of spread
    // -----------------------------------------------------------------------
    #[test]
    fn test_risk_decomposition_credit() {
        let result = analyze_sovereign_bond(&standard_sovereign()).unwrap();

        // For hard currency: credit = 60% of spread = 0.60 * 0.02 = 0.012
        assert_close(
            result.risk_metrics.credit_risk_premium,
            dec!(0.012),
            dec!(0.001),
            "Credit risk premium should be ~60% of spread",
        );
    }

    // -----------------------------------------------------------------------
    // 22. Risk decomposition: liquidity is ~25% of spread
    // -----------------------------------------------------------------------
    #[test]
    fn test_risk_decomposition_liquidity() {
        let result = analyze_sovereign_bond(&standard_sovereign()).unwrap();

        assert_close(
            result.risk_metrics.liquidity_premium,
            dec!(0.005),
            dec!(0.001),
            "Liquidity premium should be ~25% of spread",
        );
    }

    // -----------------------------------------------------------------------
    // 23. Risk decomposition: total equals spread
    // -----------------------------------------------------------------------
    #[test]
    fn test_risk_decomposition_total() {
        let result = analyze_sovereign_bond(&standard_sovereign()).unwrap();

        assert_close(
            result.risk_metrics.total_risk_premium,
            dec!(0.02),
            dec!(0.001),
            "Total risk premium should equal sovereign spread",
        );
    }

    // -----------------------------------------------------------------------
    // 24. Risk decomposition: currency risk for local currency
    // -----------------------------------------------------------------------
    #[test]
    fn test_risk_decomposition_currency_risk() {
        let result = analyze_sovereign_bond(&local_currency_sovereign()).unwrap();

        assert!(
            result.risk_metrics.currency_risk_premium.is_some(),
            "Local currency bond should have currency risk premium"
        );
    }

    // -----------------------------------------------------------------------
    // 25. Risk decomposition: no currency risk for hard currency
    // -----------------------------------------------------------------------
    #[test]
    fn test_risk_decomposition_no_currency_risk() {
        let result = analyze_sovereign_bond(&standard_sovereign()).unwrap();

        assert!(
            result.risk_metrics.currency_risk_premium.is_none(),
            "Hard currency bond should have no currency risk premium"
        );
    }

    // -----------------------------------------------------------------------
    // 26. Annual payment frequency
    // -----------------------------------------------------------------------
    #[test]
    fn test_annual_payment_frequency() {
        let input = SovereignBondInput {
            payment_frequency: 1,
            ..standard_sovereign()
        };
        let result = analyze_sovereign_bond(&input).unwrap();

        assert!(
            result.dirty_price > Decimal::ZERO,
            "Annual bond should have positive price"
        );
        assert!(
            result.modified_duration > Decimal::ZERO,
            "Annual bond should have positive duration"
        );
    }

    // -----------------------------------------------------------------------
    // 27. Quarterly payment frequency
    // -----------------------------------------------------------------------
    #[test]
    fn test_quarterly_payment_frequency() {
        let input = SovereignBondInput {
            payment_frequency: 4,
            ..standard_sovereign()
        };
        let result = analyze_sovereign_bond(&input).unwrap();

        assert!(
            result.dirty_price > Decimal::ZERO,
            "Quarterly bond should have positive price"
        );
    }

    // -----------------------------------------------------------------------
    // 28. Short maturity (1 year)
    // -----------------------------------------------------------------------
    #[test]
    fn test_short_maturity_bond() {
        let input = SovereignBondInput {
            maturity_years: dec!(1),
            ..standard_sovereign()
        };
        let result = analyze_sovereign_bond(&input).unwrap();

        // Duration of a 1-year bond should be close to 1
        assert!(
            result.modified_duration < dec!(1.1),
            "1-year bond modified duration should be < 1.1, got {}",
            result.modified_duration
        );
    }

    // -----------------------------------------------------------------------
    // 29. Long maturity (30 years)
    // -----------------------------------------------------------------------
    #[test]
    fn test_long_maturity_bond() {
        let input = SovereignBondInput {
            maturity_years: dec!(30),
            ..standard_sovereign()
        };
        let result = analyze_sovereign_bond(&input).unwrap();

        // 30-year bond should have higher duration than 10-year
        let result_10y = analyze_sovereign_bond(&standard_sovereign()).unwrap();

        assert!(
            result.modified_duration > result_10y.modified_duration,
            "30y duration ({}) should exceed 10y duration ({})",
            result.modified_duration,
            result_10y.modified_duration
        );
    }

    // -----------------------------------------------------------------------
    // 30. Zero spread sovereign (risk-free equivalent)
    // -----------------------------------------------------------------------
    #[test]
    fn test_zero_spread_sovereign() {
        let input = SovereignBondInput {
            sovereign_spread: dec!(0),
            ..standard_sovereign()
        };
        let result = analyze_sovereign_bond(&input).unwrap();

        assert!(
            result.dirty_price > Decimal::ZERO,
            "Zero spread bond should still have positive price"
        );
        assert_close(
            result.risk_metrics.total_risk_premium,
            Decimal::ZERO,
            dec!(0.0001),
            "Zero spread should have zero total risk premium",
        );
    }

    // -----------------------------------------------------------------------
    // 31. Clean price equals dirty price (on coupon date)
    // -----------------------------------------------------------------------
    #[test]
    fn test_clean_equals_dirty() {
        let result = analyze_sovereign_bond(&standard_sovereign()).unwrap();

        assert_eq!(
            result.clean_price, result.dirty_price,
            "On coupon date, clean should equal dirty"
        );
    }

    // -----------------------------------------------------------------------
    // 32. Invalid face value
    // -----------------------------------------------------------------------
    #[test]
    fn test_invalid_face_value() {
        let input = SovereignBondInput {
            face_value: dec!(-1000),
            ..standard_sovereign()
        };
        let err = analyze_sovereign_bond(&input).unwrap_err();
        match err {
            CorpFinanceError::InvalidInput { field, .. } => assert_eq!(field, "face_value"),
            other => panic!("Expected InvalidInput for face_value, got {other:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // 33. Invalid coupon rate (negative)
    // -----------------------------------------------------------------------
    #[test]
    fn test_invalid_coupon_rate() {
        let input = SovereignBondInput {
            coupon_rate: dec!(-0.01),
            ..standard_sovereign()
        };
        let err = analyze_sovereign_bond(&input).unwrap_err();
        match err {
            CorpFinanceError::InvalidInput { field, .. } => assert_eq!(field, "coupon_rate"),
            other => panic!("Expected InvalidInput for coupon_rate, got {other:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // 34. Invalid maturity (zero)
    // -----------------------------------------------------------------------
    #[test]
    fn test_invalid_maturity() {
        let input = SovereignBondInput {
            maturity_years: dec!(0),
            ..standard_sovereign()
        };
        let err = analyze_sovereign_bond(&input).unwrap_err();
        match err {
            CorpFinanceError::InvalidInput { field, .. } => assert_eq!(field, "maturity_years"),
            other => panic!("Expected InvalidInput for maturity_years, got {other:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // 35. Invalid payment frequency
    // -----------------------------------------------------------------------
    #[test]
    fn test_invalid_payment_frequency() {
        let input = SovereignBondInput {
            payment_frequency: 3,
            ..standard_sovereign()
        };
        let err = analyze_sovereign_bond(&input).unwrap_err();
        match err {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "payment_frequency")
            }
            other => panic!("Expected InvalidInput for payment_frequency, got {other:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // 36. Negative sovereign spread rejected
    // -----------------------------------------------------------------------
    #[test]
    fn test_negative_spread_rejected() {
        let input = SovereignBondInput {
            sovereign_spread: dec!(-0.01),
            ..standard_sovereign()
        };
        let err = analyze_sovereign_bond(&input).unwrap_err();
        match err {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "sovereign_spread")
            }
            other => panic!("Expected InvalidInput for sovereign_spread, got {other:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // 37. Empty country rejected
    // -----------------------------------------------------------------------
    #[test]
    fn test_empty_country_rejected() {
        let input = SovereignBondInput {
            country: "".into(),
            ..standard_sovereign()
        };
        let err = analyze_sovereign_bond(&input).unwrap_err();
        match err {
            CorpFinanceError::InvalidInput { field, .. } => assert_eq!(field, "country"),
            other => panic!("Expected InvalidInput for country, got {other:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // 38. Empty currency rejected
    // -----------------------------------------------------------------------
    #[test]
    fn test_empty_currency_rejected() {
        let input = SovereignBondInput {
            currency: "".into(),
            ..standard_sovereign()
        };
        let err = analyze_sovereign_bond(&input).unwrap_err();
        match err {
            CorpFinanceError::InvalidInput { field, .. } => assert_eq!(field, "currency"),
            other => panic!("Expected InvalidInput for currency, got {other:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // 39. Very high spread sovereign
    // -----------------------------------------------------------------------
    #[test]
    fn test_very_high_spread() {
        let input = SovereignBondInput {
            sovereign_spread: dec!(0.20), // 2000 bps
            coupon_rate: dec!(0.15),
            ..standard_sovereign()
        };
        let result = analyze_sovereign_bond(&input).unwrap();

        // Bond should price at a steep discount
        assert!(
            result.dirty_price < dec!(1000),
            "High spread bond should be at discount, got {}",
            result.dirty_price
        );
        assert!(
            result.yield_to_maturity > dec!(0.15),
            "YTM should be > 15% for high-spread bond, got {}",
            result.yield_to_maturity
        );
    }

    // -----------------------------------------------------------------------
    // 40. Local currency with no FX volatility
    // -----------------------------------------------------------------------
    #[test]
    fn test_local_currency_no_fx_vol() {
        let input = SovereignBondInput {
            is_local_currency: true,
            fx_volatility: None,
            ..standard_sovereign()
        };
        let result = analyze_sovereign_bond(&input).unwrap();

        let premium = result
            .local_currency_premium
            .expect("Should have local currency premium");
        assert_eq!(
            premium,
            Decimal::ZERO,
            "Without FX vol, local currency premium should be 0"
        );
    }

    // -----------------------------------------------------------------------
    // 41. Sqrt helper correctness
    // -----------------------------------------------------------------------
    #[test]
    fn test_decimal_sqrt_values() {
        let sqrt_4 = decimal_sqrt(dec!(4));
        assert_close(sqrt_4, dec!(2), dec!(0.0001), "sqrt(4) = 2");

        let sqrt_9 = decimal_sqrt(dec!(9));
        assert_close(sqrt_9, dec!(3), dec!(0.0001), "sqrt(9) = 3");

        let sqrt_0 = decimal_sqrt(dec!(0));
        assert_eq!(sqrt_0, Decimal::ZERO, "sqrt(0) = 0");

        let sqrt_1 = decimal_sqrt(dec!(1));
        assert_eq!(sqrt_1, Decimal::ONE, "sqrt(1) = 1");
    }

    // -----------------------------------------------------------------------
    // 42. Duration ordering by maturity
    // -----------------------------------------------------------------------
    #[test]
    fn test_duration_ordering_by_maturity() {
        let short = SovereignBondInput {
            maturity_years: dec!(2),
            ..standard_sovereign()
        };
        let medium = SovereignBondInput {
            maturity_years: dec!(5),
            ..standard_sovereign()
        };
        let long = SovereignBondInput {
            maturity_years: dec!(10),
            ..standard_sovereign()
        };

        let r_short = analyze_sovereign_bond(&short).unwrap();
        let r_medium = analyze_sovereign_bond(&medium).unwrap();
        let r_long = analyze_sovereign_bond(&long).unwrap();

        assert!(
            r_short.modified_duration < r_medium.modified_duration,
            "2y duration ({}) < 5y duration ({})",
            r_short.modified_duration,
            r_medium.modified_duration
        );
        assert!(
            r_medium.modified_duration < r_long.modified_duration,
            "5y duration ({}) < 10y duration ({})",
            r_medium.modified_duration,
            r_long.modified_duration
        );
    }

    // -----------------------------------------------------------------------
    // 43. Convexity ordering by maturity
    // -----------------------------------------------------------------------
    #[test]
    fn test_convexity_ordering_by_maturity() {
        let short = SovereignBondInput {
            maturity_years: dec!(2),
            ..standard_sovereign()
        };
        let long = SovereignBondInput {
            maturity_years: dec!(10),
            ..standard_sovereign()
        };

        let r_short = analyze_sovereign_bond(&short).unwrap();
        let r_long = analyze_sovereign_bond(&long).unwrap();

        assert!(
            r_short.convexity < r_long.convexity,
            "2y convexity ({}) < 10y convexity ({})",
            r_short.convexity,
            r_long.convexity
        );
    }

    // -----------------------------------------------------------------------
    // 44. Zero coupon has duration equal to maturity
    // -----------------------------------------------------------------------
    #[test]
    fn test_zero_coupon_duration_equals_maturity() {
        let input = SovereignBondInput {
            coupon_rate: dec!(0),
            maturity_years: dec!(5),
            ..standard_sovereign()
        };
        let result = analyze_sovereign_bond(&input).unwrap();

        // Macaulay duration of zero coupon = maturity
        // Modified = Macaulay / (1 + y/f), so modified < maturity
        // But the internal Macaulay = maturity for zero coupon
        let freq = Decimal::from(input.payment_frequency);
        let periodic_y = result.yield_to_maturity / freq;
        let implied_macaulay = result.modified_duration * (Decimal::ONE + periodic_y);

        assert_close(
            implied_macaulay,
            dec!(5),
            dec!(0.05),
            "Zero coupon Macaulay duration should equal maturity",
        );
    }
}
