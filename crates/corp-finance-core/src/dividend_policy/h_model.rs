//! H-Model Dividend Discount Model.
//!
//! The H-model is a two-stage DDM variant where the growth rate declines
//! **linearly** from a short-term high rate (g_S) to a long-term stable
//! rate (g_L) over 2H years, where H is the half-life of the decline.
//!
//! Formula:
//!   P = D₀(1+g_L)/(r-g_L) + D₀·H·(g_S-g_L)/(r-g_L)
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

/// Input for the H-model dividend discount calculation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HModelInput {
    /// Current annual dividend per share (D₀).
    pub d0: Decimal,
    /// Required rate of return (cost of equity).
    pub r: Decimal,
    /// Short-term (high) growth rate.
    pub g_short: Decimal,
    /// Long-term (stable) growth rate.
    pub g_long: Decimal,
    /// Half-life in years of the growth decline (H).
    /// Growth declines linearly from g_short to g_long over 2H years.
    pub half_life: Decimal,
}

/// Output of the H-model dividend discount calculation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HModelOutput {
    /// H-model intrinsic value per share.
    pub intrinsic_value: Decimal,
    /// Gordon Growth (stable) component: D₀(1+g_L)/(r-g_L).
    pub stable_value: Decimal,
    /// Growth premium component: D₀·H·(g_S-g_L)/(r-g_L).
    pub growth_premium: Decimal,
    /// Growth premium as a percentage of total value.
    pub growth_premium_pct: Decimal,
    /// Blended implied growth rate.
    pub implied_growth: Decimal,
}

// ---------------------------------------------------------------------------
// Core function
// ---------------------------------------------------------------------------

/// Calculate the H-model intrinsic value and decomposition.
pub fn calculate_h_model(input: &HModelInput) -> CorpFinanceResult<HModelOutput> {
    validate_input(input)?;

    let d0 = input.d0;
    let r = input.r;
    let g_l = input.g_long;
    let g_s = input.g_short;
    let h = input.half_life;

    // Denominator: r - g_L
    let denom = r - g_l;

    // Gordon Growth stable component
    let stable_value = d0 * (Decimal::ONE + g_l) / denom;

    // Growth premium
    let growth_premium = d0 * h * (g_s - g_l) / denom;

    // Total intrinsic value
    let intrinsic_value = stable_value + growth_premium;

    // Growth premium as percentage of total
    let growth_premium_pct = if intrinsic_value == Decimal::ZERO {
        Decimal::ZERO
    } else {
        growth_premium / intrinsic_value * dec!(100)
    };

    // Implied blended growth rate
    // From P = D₀(1+g_implied)/(r-g_implied) => solve is non-trivial,
    // but we can approximate: g_implied = g_L + H*(g_S - g_L)/(1+g_L+H*(g_S-g_L))
    // Simpler: back-solve from P*(r-g) = D0*(1+g) => g = (P*r - D0)/(P + D0)
    let implied_growth = if intrinsic_value + d0 == Decimal::ZERO {
        Decimal::ZERO
    } else {
        (intrinsic_value * r - d0) / (intrinsic_value + d0)
    };

    Ok(HModelOutput {
        intrinsic_value,
        stable_value,
        growth_premium,
        growth_premium_pct,
        implied_growth,
    })
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

fn validate_input(input: &HModelInput) -> CorpFinanceResult<()> {
    if input.d0 < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "d0".into(),
            reason: "Current dividend must be non-negative.".into(),
        });
    }
    if input.r <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "r".into(),
            reason: "Required rate of return must be positive.".into(),
        });
    }
    if input.r <= input.g_long {
        return Err(CorpFinanceError::FinancialImpossibility(
            "Required return (r) must exceed long-term growth (g_long) for convergent valuation."
                .into(),
        ));
    }
    if input.half_life < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "half_life".into(),
            reason: "Half-life must be non-negative.".into(),
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

    fn base_input() -> HModelInput {
        HModelInput {
            d0: dec!(2.00),
            r: dec!(0.10),
            g_short: dec!(0.15),
            g_long: dec!(0.04),
            half_life: dec!(5),
        }
    }

    #[test]
    fn test_basic_h_model() {
        let input = base_input();
        let out = calculate_h_model(&input).unwrap();
        // stable_value = 2*(1.04)/(0.10-0.04) = 2.08/0.06 = 34.6667
        assert!(approx_eq(out.stable_value, dec!(34.6667), dec!(0.01)));
        // growth_premium = 2*5*(0.15-0.04)/(0.10-0.04) = 2*5*0.11/0.06 = 18.3333
        assert!(approx_eq(out.growth_premium, dec!(18.3333), dec!(0.01)));
        // total = 34.6667 + 18.3333 = 53.0
        assert!(approx_eq(out.intrinsic_value, dec!(53.0), dec!(0.01)));
    }

    #[test]
    fn test_pure_gordon_when_h_zero() {
        let input = HModelInput {
            d0: dec!(2.00),
            r: dec!(0.10),
            g_short: dec!(0.15),
            g_long: dec!(0.04),
            half_life: Decimal::ZERO,
        };
        let out = calculate_h_model(&input).unwrap();
        // When H=0, growth premium = 0, pure Gordon
        assert_eq!(out.growth_premium, Decimal::ZERO);
        assert!(approx_eq(out.stable_value, dec!(34.6667), dec!(0.01)));
        assert!(approx_eq(
            out.intrinsic_value,
            out.stable_value,
            dec!(0.0001)
        ));
    }

    #[test]
    fn test_equal_growth_rates() {
        let input = HModelInput {
            d0: dec!(3.00),
            r: dec!(0.12),
            g_short: dec!(0.05),
            g_long: dec!(0.05),
            half_life: dec!(10),
        };
        let out = calculate_h_model(&input).unwrap();
        // g_s == g_l => growth premium = 0
        assert_eq!(out.growth_premium, Decimal::ZERO);
        // stable_value = 3*(1.05)/(0.12-0.05) = 3.15/0.07 = 45.0
        assert!(approx_eq(out.stable_value, dec!(45.0), dec!(0.01)));
    }

    #[test]
    fn test_growth_premium_pct() {
        let input = base_input();
        let out = calculate_h_model(&input).unwrap();
        // premium_pct = growth_premium / intrinsic_value * 100
        let expected_pct = out.growth_premium / out.intrinsic_value * dec!(100);
        assert!(approx_eq(out.growth_premium_pct, expected_pct, dec!(0.01)));
    }

    #[test]
    fn test_growth_premium_pct_h_zero() {
        let input = HModelInput {
            half_life: Decimal::ZERO,
            ..base_input()
        };
        let out = calculate_h_model(&input).unwrap();
        assert!(approx_eq(out.growth_premium_pct, Decimal::ZERO, dec!(0.01)));
    }

    #[test]
    fn test_high_growth_premium() {
        let input = HModelInput {
            d0: dec!(1.00),
            r: dec!(0.12),
            g_short: dec!(0.30),
            g_long: dec!(0.03),
            half_life: dec!(10),
        };
        let out = calculate_h_model(&input).unwrap();
        // growth premium = 1*10*(0.30-0.03)/(0.12-0.03) = 10*0.27/0.09 = 30.0
        assert!(approx_eq(out.growth_premium, dec!(30.0), dec!(0.01)));
        // stable = 1*(1.03)/0.09 = 11.4444
        assert!(approx_eq(out.stable_value, dec!(11.4444), dec!(0.01)));
    }

    #[test]
    fn test_negative_growth_long_term() {
        let input = HModelInput {
            d0: dec!(5.00),
            r: dec!(0.08),
            g_short: dec!(0.02),
            g_long: dec!(-0.02),
            half_life: dec!(3),
        };
        let out = calculate_h_model(&input).unwrap();
        // denom = 0.08 - (-0.02) = 0.10
        // stable = 5*(0.98)/0.10 = 49.0
        assert!(approx_eq(out.stable_value, dec!(49.0), dec!(0.01)));
        // growth = 5*3*(0.02-(-0.02))/0.10 = 15*0.04/0.10 = 6.0
        assert!(approx_eq(out.growth_premium, dec!(6.0), dec!(0.01)));
    }

    #[test]
    fn test_negative_g_short_below_g_long() {
        // g_short < g_long => negative premium (unusual but mathematically valid)
        let input = HModelInput {
            d0: dec!(2.00),
            r: dec!(0.10),
            g_short: dec!(0.02),
            g_long: dec!(0.04),
            half_life: dec!(5),
        };
        let out = calculate_h_model(&input).unwrap();
        // growth_premium = 2*5*(0.02-0.04)/(0.10-0.04) = 10*(-0.02)/0.06 = -3.3333
        assert!(out.growth_premium < Decimal::ZERO);
    }

    #[test]
    fn test_large_half_life() {
        let input = HModelInput {
            d0: dec!(1.00),
            r: dec!(0.10),
            g_short: dec!(0.20),
            g_long: dec!(0.03),
            half_life: dec!(50),
        };
        let out = calculate_h_model(&input).unwrap();
        // growth = 1*50*(0.17)/(0.07) = 8.5/0.07 = 121.4286
        assert!(approx_eq(out.growth_premium, dec!(121.4286), dec!(0.01)));
    }

    #[test]
    fn test_zero_dividend() {
        let input = HModelInput {
            d0: Decimal::ZERO,
            r: dec!(0.10),
            g_short: dec!(0.15),
            g_long: dec!(0.04),
            half_life: dec!(5),
        };
        let out = calculate_h_model(&input).unwrap();
        assert_eq!(out.intrinsic_value, Decimal::ZERO);
        assert_eq!(out.stable_value, Decimal::ZERO);
        assert_eq!(out.growth_premium, Decimal::ZERO);
    }

    #[test]
    fn test_reject_negative_dividend() {
        let input = HModelInput {
            d0: dec!(-1),
            ..base_input()
        };
        assert!(calculate_h_model(&input).is_err());
    }

    #[test]
    fn test_reject_r_equal_g_long() {
        let input = HModelInput {
            r: dec!(0.04),
            g_long: dec!(0.04),
            ..base_input()
        };
        assert!(calculate_h_model(&input).is_err());
    }

    #[test]
    fn test_reject_r_less_than_g_long() {
        let input = HModelInput {
            r: dec!(0.03),
            g_long: dec!(0.04),
            ..base_input()
        };
        assert!(calculate_h_model(&input).is_err());
    }

    #[test]
    fn test_reject_negative_half_life() {
        let input = HModelInput {
            half_life: dec!(-1),
            ..base_input()
        };
        assert!(calculate_h_model(&input).is_err());
    }

    #[test]
    fn test_reject_zero_r() {
        let input = HModelInput {
            r: Decimal::ZERO,
            g_long: dec!(-0.02),
            ..base_input()
        };
        assert!(calculate_h_model(&input).is_err());
    }

    #[test]
    fn test_implied_growth_between_g_long_and_g_short() {
        let input = base_input();
        let out = calculate_h_model(&input).unwrap();
        // Implied growth should be between g_long and g_short
        assert!(out.implied_growth >= input.g_long);
        assert!(out.implied_growth <= input.g_short);
    }

    #[test]
    fn test_implied_growth_equals_g_long_when_h_zero() {
        let input = HModelInput {
            half_life: Decimal::ZERO,
            ..base_input()
        };
        let out = calculate_h_model(&input).unwrap();
        // With H=0 pure Gordon, implied growth should = g_long
        assert!(approx_eq(out.implied_growth, input.g_long, dec!(0.001)));
    }

    #[test]
    fn test_stable_value_always_positive() {
        let input = base_input();
        let out = calculate_h_model(&input).unwrap();
        assert!(out.stable_value > Decimal::ZERO);
    }

    #[test]
    fn test_serialization_roundtrip() {
        let input = base_input();
        let out = calculate_h_model(&input).unwrap();
        let json = serde_json::to_string(&out).unwrap();
        let _: HModelOutput = serde_json::from_str(&json).unwrap();
    }

    #[test]
    fn test_small_spread_r_g() {
        // Very small spread: r - g_long = 0.001
        let input = HModelInput {
            d0: dec!(1.00),
            r: dec!(0.051),
            g_short: dec!(0.10),
            g_long: dec!(0.05),
            half_life: dec!(5),
        };
        let out = calculate_h_model(&input).unwrap();
        // Should produce very large values (narrow spread)
        assert!(out.intrinsic_value > dec!(1000));
    }

    #[test]
    fn test_intrinsic_equals_sum_of_components() {
        let input = base_input();
        let out = calculate_h_model(&input).unwrap();
        let sum = out.stable_value + out.growth_premium;
        assert!(approx_eq(out.intrinsic_value, sum, dec!(0.0001)));
    }
}
