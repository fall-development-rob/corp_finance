use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::error::CorpFinanceError;
use crate::types::*;
use crate::CorpFinanceResult;

/// Input for Kelly Criterion position sizing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KellyInput {
    /// Probability of a winning trade (0 to 1)
    pub win_probability: Rate,
    /// Ratio of average win to average loss (positive)
    pub win_loss_ratio: Decimal,
    /// Fraction of full Kelly to use (e.g. 0.5 for half-Kelly)
    pub kelly_fraction: Rate,
    /// Portfolio value for absolute position sizing
    #[serde(skip_serializing_if = "Option::is_none")]
    pub portfolio_value: Option<Money>,
    /// Maximum position as a percentage of portfolio
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_position_pct: Option<Rate>,
}

/// Output of Kelly Criterion calculation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KellyOutput {
    /// Full Kelly percentage of portfolio to allocate
    pub full_kelly_pct: Rate,
    /// Fractional Kelly percentage
    pub fractional_kelly_pct: Rate,
    /// Recommended absolute position size (if portfolio_value provided)
    pub recommended_position: Option<Money>,
    /// Edge: expected profit per unit bet
    pub edge: Rate,
    /// Expected geometric growth rate
    pub growth_rate: Rate,
}

/// Calculate Kelly Criterion position sizing.
///
/// Full Kelly: f* = p - (1-p)/b
/// where p = win probability, b = win/loss ratio
pub fn calculate_kelly(input: &KellyInput) -> CorpFinanceResult<ComputationOutput<KellyOutput>> {
    let start = Instant::now();
    let mut warnings: Vec<String> = Vec::new();

    // Validate
    if input.win_probability <= Decimal::ZERO || input.win_probability >= Decimal::ONE {
        return Err(CorpFinanceError::InvalidInput {
            field: "win_probability".into(),
            reason: "Must be between 0 and 1 (exclusive)".into(),
        });
    }
    if input.win_loss_ratio <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "win_loss_ratio".into(),
            reason: "Must be positive".into(),
        });
    }
    if input.kelly_fraction <= Decimal::ZERO || input.kelly_fraction > Decimal::ONE {
        return Err(CorpFinanceError::InvalidInput {
            field: "kelly_fraction".into(),
            reason: "Must be between 0 (exclusive) and 1 (inclusive)".into(),
        });
    }

    let p = input.win_probability;
    let q = Decimal::ONE - p;
    let b = input.win_loss_ratio;

    // Full Kelly: f* = p - q/b
    let full_kelly = p - q / b;

    // Edge = p*b - q (expected value per unit bet)
    let edge = p * b - q;

    if full_kelly <= Decimal::ZERO {
        warnings.push("Negative edge: Kelly recommends no position".into());
    }

    // Clamp full kelly to [0, 1]
    let clamped_kelly = full_kelly.max(Decimal::ZERO).min(Decimal::ONE);

    // Fractional Kelly
    let mut fractional_kelly = clamped_kelly * input.kelly_fraction;

    // Apply max position constraint
    if let Some(max_pct) = input.max_position_pct {
        if fractional_kelly > max_pct {
            warnings.push(format!(
                "Fractional Kelly {fractional_kelly} capped at max_position_pct {max_pct}"
            ));
            fractional_kelly = max_pct;
        }
    }

    // Absolute position size
    let recommended_position = input.portfolio_value.map(|pv| pv * fractional_kelly);

    // Growth rate: p * ln(1 + f*b) + q * ln(1 - f)
    // Using fractional Kelly for the actual growth rate
    let growth_rate = if fractional_kelly > Decimal::ZERO && fractional_kelly < Decimal::ONE {
        let win_part = Decimal::ONE + fractional_kelly * b;
        let lose_part = Decimal::ONE - fractional_kelly;
        if win_part > Decimal::ZERO && lose_part > Decimal::ZERO {
            p * ln_decimal(win_part) + q * ln_decimal(lose_part)
        } else {
            Decimal::ZERO
        }
    } else {
        Decimal::ZERO
    };

    let output = KellyOutput {
        full_kelly_pct: clamped_kelly,
        fractional_kelly_pct: fractional_kelly,
        recommended_position,
        edge,
        growth_rate,
    };

    let elapsed = start.elapsed().as_micros() as u64;
    Ok(with_metadata(
        "Kelly Criterion Position Sizing",
        &serde_json::json!({
            "win_probability": input.win_probability.to_string(),
            "win_loss_ratio": input.win_loss_ratio.to_string(),
            "kelly_fraction": input.kelly_fraction.to_string(),
        }),
        warnings,
        elapsed,
        output,
    ))
}

/// Natural logarithm approximation for Decimal using the series:
/// ln(x) = 2 * sum_{k=0}^{N} (1/(2k+1)) * ((x-1)/(x+1))^(2k+1)
/// Valid for x > 0.
fn ln_decimal(x: Decimal) -> Decimal {
    if x <= Decimal::ZERO {
        return Decimal::ZERO;
    }
    if x == Decimal::ONE {
        return Decimal::ZERO;
    }

    // For values far from 1, use: ln(x) = ln(x / 2^k) + k * ln(2)
    // We bring x into range [0.5, 2] for better convergence
    let ln2 = dec!(0.6931471805599453);
    let mut val = x;
    let mut k: i64 = 0;

    while val > dec!(2) {
        val /= dec!(2);
        k += 1;
    }
    while val < dec!(0.5) {
        val *= dec!(2);
        k -= 1;
    }

    // Series: ln(val) using (val-1)/(val+1)
    let u = (val - Decimal::ONE) / (val + Decimal::ONE);
    let u2 = u * u;
    let mut term = u;
    let mut sum = u;

    for n in 1..=20 {
        term *= u2;
        let denom = Decimal::from(2 * n + 1);
        sum += term / denom;
    }

    dec!(2) * sum + Decimal::from(k) * ln2
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_basic_kelly() {
        // Classic coin flip with 2:1 payoff and 50% win rate
        // f* = 0.5 - 0.5/2 = 0.25
        let input = KellyInput {
            win_probability: dec!(0.5),
            win_loss_ratio: dec!(2),
            kelly_fraction: dec!(1.0),
            portfolio_value: None,
            max_position_pct: None,
        };
        let result = calculate_kelly(&input).unwrap();
        assert_eq!(result.result.full_kelly_pct, dec!(0.25));
    }

    #[test]
    fn test_fractional_kelly() {
        let input = KellyInput {
            win_probability: dec!(0.5),
            win_loss_ratio: dec!(2),
            kelly_fraction: dec!(0.5),
            portfolio_value: Some(dec!(100000)),
            max_position_pct: None,
        };
        let result = calculate_kelly(&input).unwrap();
        assert_eq!(result.result.fractional_kelly_pct, dec!(0.125));
        assert_eq!(result.result.recommended_position, Some(dec!(12500)));
    }

    #[test]
    fn test_edge_calculation() {
        // p=0.6, b=1.5 => edge = 0.6*1.5 - 0.4 = 0.5
        let input = KellyInput {
            win_probability: dec!(0.6),
            win_loss_ratio: dec!(1.5),
            kelly_fraction: dec!(1.0),
            portfolio_value: None,
            max_position_pct: None,
        };
        let result = calculate_kelly(&input).unwrap();
        assert_eq!(result.result.edge, dec!(0.5));
    }

    #[test]
    fn test_negative_edge() {
        // p=0.3, b=1 => f* = 0.3 - 0.7/1 = -0.4 => clamped to 0
        let input = KellyInput {
            win_probability: dec!(0.3),
            win_loss_ratio: dec!(1),
            kelly_fraction: dec!(1.0),
            portfolio_value: None,
            max_position_pct: None,
        };
        let result = calculate_kelly(&input).unwrap();
        assert_eq!(result.result.full_kelly_pct, Decimal::ZERO);
        assert!(result.result.edge < Decimal::ZERO);
    }

    #[test]
    fn test_max_position_cap() {
        let input = KellyInput {
            win_probability: dec!(0.6),
            win_loss_ratio: dec!(3.0),
            kelly_fraction: dec!(1.0),
            portfolio_value: Some(dec!(100000)),
            max_position_pct: Some(dec!(0.10)),
        };
        let result = calculate_kelly(&input).unwrap();
        assert!(result.result.fractional_kelly_pct <= dec!(0.10));
    }

    #[test]
    fn test_growth_rate_positive() {
        let input = KellyInput {
            win_probability: dec!(0.6),
            win_loss_ratio: dec!(2),
            kelly_fraction: dec!(0.5),
            portfolio_value: None,
            max_position_pct: None,
        };
        let result = calculate_kelly(&input).unwrap();
        assert!(result.result.growth_rate > Decimal::ZERO);
    }

    #[test]
    fn test_invalid_probability() {
        let input = KellyInput {
            win_probability: dec!(1.5),
            win_loss_ratio: dec!(2),
            kelly_fraction: dec!(1.0),
            portfolio_value: None,
            max_position_pct: None,
        };
        assert!(calculate_kelly(&input).is_err());
    }

    #[test]
    fn test_invalid_ratio() {
        let input = KellyInput {
            win_probability: dec!(0.5),
            win_loss_ratio: dec!(-1),
            kelly_fraction: dec!(1.0),
            portfolio_value: None,
            max_position_pct: None,
        };
        assert!(calculate_kelly(&input).is_err());
    }
}
