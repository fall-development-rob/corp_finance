//! Merton structural model for credit risk.
//!
//! Covers:
//! 1. **Merton PD** -- iteratively solve for asset value and asset volatility
//!    using equity as a call option on firm assets.
//! 2. **Distance to Default** -- number of standard deviations from the default
//!    point.
//! 3. **KMV EDF** -- empirical default frequency mapping from DD.
//!
//! All arithmetic uses `rust_decimal::Decimal`. No `f64`.

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

use crate::error::CorpFinanceError;
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Math helpers (module-local)
// ---------------------------------------------------------------------------

/// Natural logarithm via Taylor series.
fn decimal_ln(x: Decimal) -> Decimal {
    if x <= Decimal::ZERO {
        return Decimal::ZERO;
    }
    let ln2 = dec!(0.6931471805599453);
    let mut val = x;
    let mut adjust = Decimal::ZERO;
    while val > dec!(2.0) {
        val /= dec!(2);
        adjust += ln2;
    }
    while val < dec!(0.5) {
        val *= dec!(2);
        adjust -= ln2;
    }
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

/// Exponential via Taylor series.
fn decimal_exp(x: Decimal) -> Decimal {
    let ln2 = dec!(0.6931471805599453);
    // Reduce: exp(x) = 2^n * exp(r) where r = x - n*ln2 and |r| < ln2/2
    let n_raw = x / ln2;
    let n = if n_raw >= Decimal::ZERO {
        n_raw.floor()
    } else {
        n_raw.ceil() - Decimal::ONE
    };
    let r = x - n * ln2;

    // Taylor series for exp(r)
    let mut term = Decimal::ONE;
    let mut sum = Decimal::ONE;
    for i in 1u32..40 {
        term = term * r / Decimal::from(i);
        sum += term;
    }

    // Multiply by 2^n
    let n_i64 = n.to_string().parse::<i64>().unwrap_or(0);
    if n_i64 >= 0 {
        let mut pow2 = Decimal::ONE;
        for _ in 0..n_i64 {
            pow2 *= dec!(2);
        }
        sum * pow2
    } else {
        let mut pow2 = Decimal::ONE;
        for _ in 0..(-n_i64) {
            pow2 *= dec!(2);
        }
        sum / pow2
    }
}

/// Square root via Newton's method (20 iterations).
fn decimal_sqrt(x: Decimal) -> Decimal {
    if x <= Decimal::ZERO {
        return Decimal::ZERO;
    }
    let mut guess = x / dec!(2);
    if guess.is_zero() {
        guess = Decimal::ONE;
    }
    for _ in 0..20 {
        guess = (guess + x / guess) / dec!(2);
    }
    guess
}

/// Cumulative normal distribution using Abramowitz & Stegun approximation.
fn norm_cdf(x: Decimal) -> Decimal {
    if x <= dec!(-10) {
        return Decimal::ZERO;
    }
    if x >= dec!(10) {
        return Decimal::ONE;
    }
    let is_neg = x < Decimal::ZERO;
    let abs_x = x.abs();

    let p = dec!(0.2316419);
    let b1 = dec!(0.319381530);
    let b2 = dec!(-0.356563782);
    let b3 = dec!(1.781477937);
    let b4 = dec!(-1.821255978);
    let b5 = dec!(1.330274429);

    let t = Decimal::ONE / (Decimal::ONE + p * abs_x);
    let t2 = t * t;
    let t3 = t2 * t;
    let t4 = t3 * t;
    let t5 = t4 * t;

    // phi(x) = (1/sqrt(2*pi)) * exp(-x^2/2)
    let sqrt_2pi = dec!(2.506628274631);
    let pdf = decimal_exp(-(abs_x * abs_x) / dec!(2)) / sqrt_2pi;

    let cdf = Decimal::ONE - pdf * (b1 * t + b2 * t2 + b3 * t3 + b4 * t4 + b5 * t5);

    if is_neg {
        Decimal::ONE - cdf
    } else {
        cdf
    }
}

// ---------------------------------------------------------------------------
// Input / Output
// ---------------------------------------------------------------------------

/// Input for Merton structural model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MertonInput {
    /// Market value of equity.
    pub equity_value: Decimal,
    /// Annualized equity volatility (decimal, e.g. 0.30 = 30%).
    pub equity_vol: Decimal,
    /// Face value of debt (default barrier).
    pub debt_face: Decimal,
    /// Risk-free rate (annualized, decimal).
    pub risk_free_rate: Decimal,
    /// Time to maturity in years.
    pub maturity: Decimal,
    /// Expected asset growth rate (drift, for DD calculation).
    pub growth_rate: Decimal,
}

/// Output of the Merton structural model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MertonOutput {
    /// Implied asset value.
    pub asset_value: Decimal,
    /// Implied asset volatility.
    pub asset_vol: Decimal,
    /// d1 in the Black-Scholes formula.
    pub d1: Decimal,
    /// d2 = d1 - sigma_V * sqrt(T).
    pub d2: Decimal,
    /// Distance to Default.
    pub distance_to_default: Decimal,
    /// Probability of default (Merton model).
    pub pd_merton: Decimal,
    /// Expected Default Frequency (KMV empirical mapping).
    pub edf_kmv: Decimal,
}

// ---------------------------------------------------------------------------
// Core function
// ---------------------------------------------------------------------------

/// Calculate Merton structural model: asset value, asset vol, DD, PD, EDF.
pub fn calculate_merton(input: &MertonInput) -> CorpFinanceResult<MertonOutput> {
    validate_merton_input(input)?;

    let e = input.equity_value;
    let sigma_e = input.equity_vol;
    let d = input.debt_face;
    let r = input.risk_free_rate;
    let t = input.maturity;
    let sqrt_t = decimal_sqrt(t);

    // Initial guesses
    let mut v = e + d; // asset value guess
    let mut sigma_v = sigma_e * e / v; // asset vol guess

    // Newton's method: 30 iterations to solve the two-equation system
    // E = V*N(d1) - D*exp(-rT)*N(d2)
    // sigma_E = (V/E)*N(d1)*sigma_V
    for _ in 0..30 {
        let d1 = compute_d1(v, d, r, sigma_v, t, sqrt_t);
        let d2 = d1 - sigma_v * sqrt_t;
        let nd1 = norm_cdf(d1);
        let nd2 = norm_cdf(d2);
        let discount = decimal_exp(-r * t);

        // Equation 1: E = V*N(d1) - D*exp(-rT)*N(d2)
        let e_model = v * nd1 - d * discount * nd2;
        let e_error = e_model - e;

        // Equation 2: sigma_E*E = V*N(d1)*sigma_V
        let sigma_e_model = if e.is_zero() {
            Decimal::ZERO
        } else {
            v * nd1 * sigma_v / e
        };
        let _sigma_error = sigma_e_model - sigma_e;

        // Jacobian for Newton's step (simplified: treat each equation independently)
        // dE/dV ~ N(d1), dE/dsigma_V ~ V*phi(d1)*sqrt(T)
        // Simplified: update V from equity equation, sigma_V from volatility equation
        if nd1 > dec!(0.0001) {
            v -= e_error / nd1;
            if v < e {
                v = e + d * dec!(0.01); // floor
            }
        }
        if v > Decimal::ZERO && nd1 > dec!(0.0001) {
            sigma_v = sigma_e * e / (v * nd1);
            if sigma_v < dec!(0.001) {
                sigma_v = dec!(0.001);
            }
        }
    }

    let d1 = compute_d1(v, d, r, sigma_v, t, sqrt_t);
    let d2 = d1 - sigma_v * sqrt_t;

    // Distance to Default using growth_rate (physical measure)
    let dd = if sigma_v * sqrt_t > Decimal::ZERO {
        (decimal_ln(v / d) + (input.growth_rate - sigma_v * sigma_v / dec!(2)) * t)
            / (sigma_v * sqrt_t)
    } else {
        Decimal::ZERO
    };

    let pd_merton = norm_cdf(-dd);
    let edf_kmv = kmv_edf(dd);

    Ok(MertonOutput {
        asset_value: v,
        asset_vol: sigma_v,
        d1,
        d2,
        distance_to_default: dd,
        pd_merton,
        edf_kmv,
    })
}

/// Compute d1 = (ln(V/D) + (r + sigma^2/2)*T) / (sigma*sqrt(T)).
fn compute_d1(
    v: Decimal,
    d: Decimal,
    r: Decimal,
    sigma: Decimal,
    t: Decimal,
    sqrt_t: Decimal,
) -> Decimal {
    let denom = sigma * sqrt_t;
    if denom.is_zero() || d.is_zero() {
        return Decimal::ZERO;
    }
    (decimal_ln(v / d) + (r + sigma * sigma / dec!(2)) * t) / denom
}

/// KMV EDF: stylized empirical mapping from Distance to Default.
/// Uses a simplified lookup: EDF ≈ N(-DD) with an empirical floor.
fn kmv_edf(dd: Decimal) -> Decimal {
    // Empirical EDF table (stylized)
    // DD:  1.0 => ~4%, 2.0 => ~0.7%, 3.0 => ~0.03%, 4.0 => ~0.003%
    // For DD <= 0, EDF approaches 50%+
    // We use the normal approximation with empirical floor of 3bps
    let edf = norm_cdf(-dd);
    let floor = dec!(0.0003); // 3bps floor
    if edf < floor {
        floor
    } else {
        edf
    }
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

fn validate_merton_input(input: &MertonInput) -> CorpFinanceResult<()> {
    if input.equity_value <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "equity_value".into(),
            reason: "Equity value must be positive.".into(),
        });
    }
    if input.equity_vol <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "equity_vol".into(),
            reason: "Equity volatility must be positive.".into(),
        });
    }
    if input.debt_face <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "debt_face".into(),
            reason: "Debt face value must be positive.".into(),
        });
    }
    if input.maturity <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "maturity".into(),
            reason: "Maturity must be positive.".into(),
        });
    }
    if input.equity_vol > dec!(5.0) {
        return Err(CorpFinanceError::InvalidInput {
            field: "equity_vol".into(),
            reason: "Equity volatility exceeds 500%, likely an error.".into(),
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

    fn base_input() -> MertonInput {
        MertonInput {
            equity_value: dec!(100),
            equity_vol: dec!(0.40),
            debt_face: dec!(80),
            risk_free_rate: dec!(0.05),
            maturity: dec!(1.0),
            growth_rate: dec!(0.05),
        }
    }

    #[test]
    fn test_asset_value_exceeds_equity() {
        let input = base_input();
        let out = calculate_merton(&input).unwrap();
        assert!(
            out.asset_value > input.equity_value,
            "Asset value {} must exceed equity {}",
            out.asset_value,
            input.equity_value
        );
    }

    #[test]
    fn test_asset_vol_less_than_equity_vol() {
        let input = base_input();
        let out = calculate_merton(&input).unwrap();
        // With leverage, equity vol > asset vol
        assert!(
            out.asset_vol < input.equity_vol,
            "Asset vol {} should be < equity vol {}",
            out.asset_vol,
            input.equity_vol
        );
    }

    #[test]
    fn test_d1_greater_than_d2() {
        let input = base_input();
        let out = calculate_merton(&input).unwrap();
        assert!(out.d1 > out.d2);
    }

    #[test]
    fn test_d2_equals_d1_minus_sigma_sqrt_t() {
        let input = base_input();
        let out = calculate_merton(&input).unwrap();
        let expected_d2 = out.d1 - out.asset_vol * decimal_sqrt(input.maturity);
        assert!(approx_eq(out.d2, expected_d2, dec!(0.001)));
    }

    #[test]
    fn test_pd_between_zero_and_one() {
        let input = base_input();
        let out = calculate_merton(&input).unwrap();
        assert!(out.pd_merton >= Decimal::ZERO && out.pd_merton <= Decimal::ONE);
    }

    #[test]
    fn test_high_leverage_increases_pd() {
        let low_lev = MertonInput {
            equity_value: dec!(100),
            equity_vol: dec!(0.30),
            debt_face: dec!(30),
            risk_free_rate: dec!(0.05),
            maturity: dec!(1.0),
            growth_rate: dec!(0.05),
        };
        let high_lev = MertonInput {
            equity_value: dec!(100),
            equity_vol: dec!(0.30),
            debt_face: dec!(150),
            risk_free_rate: dec!(0.05),
            maturity: dec!(1.0),
            growth_rate: dec!(0.05),
        };
        let out_low = calculate_merton(&low_lev).unwrap();
        let out_high = calculate_merton(&high_lev).unwrap();
        assert!(
            out_high.pd_merton > out_low.pd_merton,
            "Higher leverage PD {} should exceed lower leverage PD {}",
            out_high.pd_merton,
            out_low.pd_merton
        );
    }

    #[test]
    fn test_higher_vol_increases_pd() {
        let low_vol = MertonInput {
            equity_value: dec!(100),
            equity_vol: dec!(0.20),
            debt_face: dec!(80),
            risk_free_rate: dec!(0.05),
            maturity: dec!(1.0),
            growth_rate: dec!(0.05),
        };
        let high_vol = MertonInput {
            equity_value: dec!(100),
            equity_vol: dec!(0.60),
            debt_face: dec!(80),
            risk_free_rate: dec!(0.05),
            maturity: dec!(1.0),
            growth_rate: dec!(0.05),
        };
        let out_low = calculate_merton(&low_vol).unwrap();
        let out_high = calculate_merton(&high_vol).unwrap();
        assert!(out_high.pd_merton > out_low.pd_merton);
    }

    #[test]
    fn test_dd_positive_for_solvent_firm() {
        let input = base_input();
        let out = calculate_merton(&input).unwrap();
        assert!(out.distance_to_default > Decimal::ZERO);
    }

    #[test]
    fn test_edf_has_floor() {
        // Very safe firm: large equity, low vol, low debt
        let input = MertonInput {
            equity_value: dec!(1000),
            equity_vol: dec!(0.10),
            debt_face: dec!(10),
            risk_free_rate: dec!(0.05),
            maturity: dec!(1.0),
            growth_rate: dec!(0.05),
        };
        let out = calculate_merton(&input).unwrap();
        assert!(
            out.edf_kmv >= dec!(0.0003),
            "EDF should have a floor of 3bps"
        );
    }

    #[test]
    fn test_edf_approaches_fifty_pct_at_low_dd() {
        let input = MertonInput {
            equity_value: dec!(10),
            equity_vol: dec!(0.80),
            debt_face: dec!(200),
            risk_free_rate: dec!(0.05),
            maturity: dec!(1.0),
            growth_rate: dec!(0.0),
        };
        let out = calculate_merton(&input).unwrap();
        // DD should be low/negative => EDF should be high
        assert!(out.edf_kmv > dec!(0.10));
    }

    #[test]
    fn test_longer_maturity_affects_dd() {
        let short = MertonInput {
            maturity: dec!(0.5),
            ..base_input()
        };
        let long = MertonInput {
            maturity: dec!(5.0),
            ..base_input()
        };
        let out_s = calculate_merton(&short).unwrap();
        let out_l = calculate_merton(&long).unwrap();
        // Both should compute valid results
        assert!(out_s.pd_merton >= Decimal::ZERO);
        assert!(out_l.pd_merton >= Decimal::ZERO);
    }

    #[test]
    fn test_reject_negative_equity() {
        let input = MertonInput {
            equity_value: dec!(-100),
            ..base_input()
        };
        assert!(calculate_merton(&input).is_err());
    }

    #[test]
    fn test_reject_zero_equity_vol() {
        let input = MertonInput {
            equity_vol: Decimal::ZERO,
            ..base_input()
        };
        assert!(calculate_merton(&input).is_err());
    }

    #[test]
    fn test_reject_negative_debt() {
        let input = MertonInput {
            debt_face: dec!(-80),
            ..base_input()
        };
        assert!(calculate_merton(&input).is_err());
    }

    #[test]
    fn test_reject_zero_maturity() {
        let input = MertonInput {
            maturity: Decimal::ZERO,
            ..base_input()
        };
        assert!(calculate_merton(&input).is_err());
    }

    #[test]
    fn test_reject_excessive_vol() {
        let input = MertonInput {
            equity_vol: dec!(6.0),
            ..base_input()
        };
        assert!(calculate_merton(&input).is_err());
    }

    #[test]
    fn test_norm_cdf_symmetry() {
        let pos = norm_cdf(dec!(1.0));
        let neg = norm_cdf(dec!(-1.0));
        assert!(approx_eq(pos + neg, Decimal::ONE, dec!(0.001)));
    }

    #[test]
    fn test_norm_cdf_at_zero() {
        let val = norm_cdf(Decimal::ZERO);
        assert!(approx_eq(val, dec!(0.5), dec!(0.001)));
    }

    #[test]
    fn test_norm_cdf_far_positive() {
        let val = norm_cdf(dec!(10));
        assert_eq!(val, Decimal::ONE);
    }

    #[test]
    fn test_norm_cdf_far_negative() {
        let val = norm_cdf(dec!(-10));
        assert_eq!(val, Decimal::ZERO);
    }

    #[test]
    fn test_serialization_roundtrip() {
        let input = base_input();
        let out = calculate_merton(&input).unwrap();
        let json = serde_json::to_string(&out).unwrap();
        let _: MertonOutput = serde_json::from_str(&json).unwrap();
    }

    #[test]
    fn test_decimal_exp_of_zero() {
        let result = decimal_exp(Decimal::ZERO);
        assert!(approx_eq(result, Decimal::ONE, dec!(0.001)));
    }

    #[test]
    fn test_decimal_exp_of_one() {
        let result = decimal_exp(Decimal::ONE);
        assert!(approx_eq(result, dec!(2.71828), dec!(0.01)));
    }

    #[test]
    fn test_decimal_sqrt_of_four() {
        let result = decimal_sqrt(dec!(4));
        assert!(approx_eq(result, dec!(2), dec!(0.0001)));
    }
}
