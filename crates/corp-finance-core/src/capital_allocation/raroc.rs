//! Risk-Adjusted Return on Capital (RAROC) Analytics.
//!
//! Covers:
//! 1. **RAROC** -- (revenue - costs - expected_loss) / economic_capital
//! 2. **RORAC** -- net_income / economic_capital
//! 3. **RAROC vs Hurdle** -- compare RAROC to cost-of-equity hurdle rate
//! 4. **EVA** -- Economic Value Added = (RAROC - hurdle) * economic_capital
//! 5. **SVA** -- Shareholder Value Added = EVA / (1 + cost_of_equity)
//! 6. **Risk-Adjusted Pricing** -- minimum spread = (EL + EC*hurdle + opex) / exposure
//!
//! All arithmetic uses `rust_decimal::Decimal`. No `f64`.

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::error::CorpFinanceError;
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Input / Output
// ---------------------------------------------------------------------------

/// Input for RAROC calculation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RarocInput {
    /// Total revenue from the business unit / exposure.
    pub revenue: Decimal,
    /// Operating costs.
    pub operating_costs: Decimal,
    /// Expected loss (PD * LGD * EAD or provisioned amount).
    pub expected_loss: Decimal,
    /// Economic capital allocated to this exposure.
    pub economic_capital: Decimal,
    /// Hurdle rate (cost of equity, decimal: 0.12 = 12%).
    pub hurdle_rate: Decimal,
    /// Cost of equity for SVA discounting (decimal: 0.10 = 10%).
    pub cost_of_equity: Decimal,
    /// Total exposure (for risk-adjusted pricing).
    pub exposure: Decimal,
}

/// Output of the RAROC calculation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RarocOutput {
    /// RAROC = (revenue - costs - EL) / EC.
    pub raroc: Decimal,
    /// RORAC = (revenue - costs) / EC.
    pub rorac: Decimal,
    /// EVA = (RAROC - hurdle) * EC.
    pub eva: Decimal,
    /// SVA = EVA / (1 + cost_of_equity).
    pub sva: Decimal,
    /// Spread to hurdle = RAROC - hurdle.
    pub spread_to_hurdle: Decimal,
    /// Risk-adjusted minimum price (spread).
    pub risk_adjusted_price: Decimal,
    /// Whether the exposure creates value (RAROC > hurdle).
    pub value_creation: bool,
}

/// Compute RAROC and related metrics.
pub fn calculate_raroc(input: &RarocInput) -> CorpFinanceResult<RarocOutput> {
    validate_raroc_input(input)?;

    // Net income (before capital charge)
    let net_income = input.revenue - input.operating_costs;

    // Risk-adjusted net income
    let risk_adj_income = net_income - input.expected_loss;

    // RAROC = (revenue - costs - EL) / EC
    let raroc = if input.economic_capital.is_zero() {
        Decimal::ZERO
    } else {
        risk_adj_income / input.economic_capital
    };

    // RORAC = net_income / EC
    let rorac = if input.economic_capital.is_zero() {
        Decimal::ZERO
    } else {
        net_income / input.economic_capital
    };

    // EVA = (RAROC - hurdle) * EC
    let eva = (raroc - input.hurdle_rate) * input.economic_capital;

    // SVA = EVA / (1 + cost_of_equity)
    let sva_denom = Decimal::ONE + input.cost_of_equity;
    let sva = if sva_denom.is_zero() {
        Decimal::ZERO
    } else {
        eva / sva_denom
    };

    // Spread to hurdle
    let spread_to_hurdle = raroc - input.hurdle_rate;

    // Risk-adjusted pricing: min spread = (EL + EC*hurdle + opex) / exposure
    let risk_adjusted_price = if input.exposure.is_zero() {
        Decimal::ZERO
    } else {
        (input.expected_loss + input.economic_capital * input.hurdle_rate + input.operating_costs)
            / input.exposure
    };

    // Value creation flag
    let value_creation = raroc > input.hurdle_rate;

    Ok(RarocOutput {
        raroc,
        rorac,
        eva,
        sva,
        spread_to_hurdle,
        risk_adjusted_price,
        value_creation,
    })
}

fn validate_raroc_input(input: &RarocInput) -> CorpFinanceResult<()> {
    if input.economic_capital < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "economic_capital".into(),
            reason: "Economic capital must be non-negative.".into(),
        });
    }
    if input.expected_loss < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "expected_loss".into(),
            reason: "Expected loss must be non-negative.".into(),
        });
    }
    if input.hurdle_rate < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "hurdle_rate".into(),
            reason: "Hurdle rate must be non-negative.".into(),
        });
    }
    if input.cost_of_equity < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "cost_of_equity".into(),
            reason: "Cost of equity must be non-negative.".into(),
        });
    }
    if input.exposure < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "exposure".into(),
            reason: "Exposure must be non-negative.".into(),
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

    fn make_base_input() -> RarocInput {
        RarocInput {
            revenue: dec!(100_000),
            operating_costs: dec!(30_000),
            expected_loss: dec!(10_000),
            economic_capital: dec!(500_000),
            hurdle_rate: dec!(0.12),
            cost_of_equity: dec!(0.10),
            exposure: dec!(5_000_000),
        }
    }

    #[test]
    fn test_raroc_basic_calculation() {
        let input = make_base_input();
        let out = calculate_raroc(&input).unwrap();
        // RAROC = (100k - 30k - 10k) / 500k = 60k / 500k = 0.12
        assert_eq!(out.raroc, dec!(0.12));
    }

    #[test]
    fn test_rorac_basic_calculation() {
        let input = make_base_input();
        let out = calculate_raroc(&input).unwrap();
        // RORAC = (100k - 30k) / 500k = 70k / 500k = 0.14
        assert_eq!(out.rorac, dec!(0.14));
    }

    #[test]
    fn test_eva_at_hurdle_is_zero() {
        let input = make_base_input();
        let out = calculate_raroc(&input).unwrap();
        // RAROC = 0.12 = hurdle, so EVA = 0
        assert_eq!(out.eva, Decimal::ZERO);
    }

    #[test]
    fn test_eva_positive_when_above_hurdle() {
        let mut input = make_base_input();
        input.revenue = dec!(150_000); // RAROC = (150k-30k-10k)/500k = 0.22
        let out = calculate_raroc(&input).unwrap();
        assert!(out.eva > Decimal::ZERO);
    }

    #[test]
    fn test_eva_negative_when_below_hurdle() {
        let mut input = make_base_input();
        input.revenue = dec!(50_000); // RAROC = (50k-30k-10k)/500k = 0.02
        let out = calculate_raroc(&input).unwrap();
        assert!(out.eva < Decimal::ZERO);
    }

    #[test]
    fn test_sva_is_discounted_eva() {
        let mut input = make_base_input();
        input.revenue = dec!(200_000);
        let out = calculate_raroc(&input).unwrap();
        let expected_sva = out.eva / (Decimal::ONE + input.cost_of_equity);
        assert_eq!(out.sva, expected_sva);
    }

    #[test]
    fn test_spread_to_hurdle() {
        let mut input = make_base_input();
        input.revenue = dec!(200_000);
        let out = calculate_raroc(&input).unwrap();
        assert_eq!(out.spread_to_hurdle, out.raroc - input.hurdle_rate);
    }

    #[test]
    fn test_value_creation_true_above_hurdle() {
        let mut input = make_base_input();
        input.revenue = dec!(200_000);
        let out = calculate_raroc(&input).unwrap();
        assert!(out.value_creation);
    }

    #[test]
    fn test_value_creation_false_below_hurdle() {
        let mut input = make_base_input();
        input.revenue = dec!(50_000);
        let out = calculate_raroc(&input).unwrap();
        assert!(!out.value_creation);
    }

    #[test]
    fn test_value_creation_false_at_hurdle() {
        let input = make_base_input(); // RAROC = 0.12 = hurdle
        let out = calculate_raroc(&input).unwrap();
        assert!(!out.value_creation); // not strictly above
    }

    #[test]
    fn test_risk_adjusted_price() {
        let input = make_base_input();
        let out = calculate_raroc(&input).unwrap();
        // min_spread = (10k + 500k*0.12 + 30k) / 5M = (10000+60000+30000)/5000000 = 100000/5000000 = 0.02
        let expected = (dec!(10_000) + dec!(500_000) * dec!(0.12) + dec!(30_000)) / dec!(5_000_000);
        assert_eq!(out.risk_adjusted_price, expected);
    }

    #[test]
    fn test_zero_economic_capital_returns_zero_raroc() {
        let mut input = make_base_input();
        input.economic_capital = Decimal::ZERO;
        let out = calculate_raroc(&input).unwrap();
        assert_eq!(out.raroc, Decimal::ZERO);
        assert_eq!(out.rorac, Decimal::ZERO);
    }

    #[test]
    fn test_zero_exposure_returns_zero_price() {
        let mut input = make_base_input();
        input.exposure = Decimal::ZERO;
        let out = calculate_raroc(&input).unwrap();
        assert_eq!(out.risk_adjusted_price, Decimal::ZERO);
    }

    #[test]
    fn test_rorac_gt_raroc_when_el_positive() {
        let input = make_base_input();
        let out = calculate_raroc(&input).unwrap();
        assert!(
            out.rorac >= out.raroc,
            "RORAC {} should be >= RAROC {} when EL > 0",
            out.rorac,
            out.raroc
        );
    }

    #[test]
    fn test_high_revenue_high_raroc() {
        let mut input = make_base_input();
        input.revenue = dec!(1_000_000);
        let out = calculate_raroc(&input).unwrap();
        // RAROC = (1M - 30k - 10k) / 500k = 1.92
        assert!(
            approx_eq(out.raroc, dec!(1.92), dec!(0.001)),
            "RAROC should be ~1.92, got {}",
            out.raroc
        );
    }

    #[test]
    fn test_negative_net_income_negative_raroc() {
        let mut input = make_base_input();
        input.revenue = dec!(10_000); // Net = 10k - 30k = -20k, risk_adj = -30k
        let out = calculate_raroc(&input).unwrap();
        assert!(out.raroc < Decimal::ZERO);
    }

    #[test]
    fn test_eva_equals_spread_times_ec() {
        let mut input = make_base_input();
        input.revenue = dec!(200_000);
        let out = calculate_raroc(&input).unwrap();
        let expected_eva = out.spread_to_hurdle * input.economic_capital;
        assert!(
            approx_eq(out.eva, expected_eva, dec!(0.01)),
            "EVA {} should equal spread * EC {}",
            out.eva,
            expected_eva
        );
    }

    #[test]
    fn test_sva_sign_matches_eva() {
        let mut input = make_base_input();
        input.revenue = dec!(200_000);
        let out = calculate_raroc(&input).unwrap();
        assert!((out.sva > Decimal::ZERO) == (out.eva > Decimal::ZERO));
    }

    #[test]
    fn test_sva_magnitude_less_than_eva() {
        let mut input = make_base_input();
        input.revenue = dec!(200_000);
        let out = calculate_raroc(&input).unwrap();
        // SVA = EVA/(1+COE) < EVA when COE > 0
        assert!(out.sva.abs() <= out.eva.abs());
    }

    // -- Validation tests --

    #[test]
    fn test_reject_negative_economic_capital() {
        let mut input = make_base_input();
        input.economic_capital = dec!(-1);
        assert!(calculate_raroc(&input).is_err());
    }

    #[test]
    fn test_reject_negative_expected_loss() {
        let mut input = make_base_input();
        input.expected_loss = dec!(-1);
        assert!(calculate_raroc(&input).is_err());
    }

    #[test]
    fn test_reject_negative_hurdle_rate() {
        let mut input = make_base_input();
        input.hurdle_rate = dec!(-0.01);
        assert!(calculate_raroc(&input).is_err());
    }

    #[test]
    fn test_reject_negative_cost_of_equity() {
        let mut input = make_base_input();
        input.cost_of_equity = dec!(-0.01);
        assert!(calculate_raroc(&input).is_err());
    }

    #[test]
    fn test_reject_negative_exposure() {
        let mut input = make_base_input();
        input.exposure = dec!(-1);
        assert!(calculate_raroc(&input).is_err());
    }

    #[test]
    fn test_serialization_roundtrip() {
        let input = make_base_input();
        let out = calculate_raroc(&input).unwrap();
        let json = serde_json::to_string(&out).unwrap();
        let _: RarocOutput = serde_json::from_str(&json).unwrap();
    }

    #[test]
    fn test_large_expected_loss_reduces_raroc() {
        let mut input = make_base_input();
        input.expected_loss = dec!(60_000); // risk_adj = 100k-30k-60k = 10k => RAROC = 0.02
        let out = calculate_raroc(&input).unwrap();
        assert!(
            approx_eq(out.raroc, dec!(0.02), dec!(0.001)),
            "RAROC should be ~0.02, got {}",
            out.raroc
        );
    }

    #[test]
    fn test_zero_hurdle_always_value_creation() {
        let mut input = make_base_input();
        input.hurdle_rate = Decimal::ZERO;
        input.revenue = dec!(100_000);
        let out = calculate_raroc(&input).unwrap();
        // RAROC = 0.12 > 0 = hurdle
        assert!(out.value_creation);
    }
}
