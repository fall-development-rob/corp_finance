//! Earnings Quality Composite Score.
//!
//! Combines four component scores into a single 0-100 rating:
//! 1. **Beneish component** -- M-Score mapped to 0/50/100.
//! 2. **Piotroski component** -- F-Score normalized to 0-100.
//! 3. **Accrual component** -- Sloan ratio mapped to 0/50/100, bonus for cash conversion.
//! 4. **Revenue component** -- Revenue quality score passed through directly.
//!
//! Classification:
//! - >= 75: "High Quality"
//! - >= 50: "Acceptable"
//! - >= 25: "Caution"
//! - < 25: "Red Flag"
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

/// Pre-computed component metrics for the earnings quality composite.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EarningsQualityCompositeInput {
    pub beneish_m_score: Decimal,
    pub piotroski_f_score: u8,
    pub sloan_ratio: Decimal,
    pub cash_conversion: Decimal,
    pub revenue_quality_score: Decimal,

    // Optional weights (default: equally weighted at 0.25 each)
    pub weight_beneish: Option<Decimal>,
    pub weight_piotroski: Option<Decimal>,
    pub weight_accrual: Option<Decimal>,
    pub weight_revenue: Option<Decimal>,
}

/// Normalized component scores.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentScores {
    pub beneish_component: Decimal,
    pub piotroski_component: Decimal,
    pub accrual_component: Decimal,
    pub revenue_component: Decimal,
}

/// Earnings quality composite results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EarningsQualityCompositeOutput {
    /// Weighted composite score (0-100).
    pub composite_score: Decimal,
    /// "High Quality", "Acceptable", "Caution", or "Red Flag".
    pub classification: String,
    /// Individual normalized component scores.
    pub component_scores: ComponentScores,
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const DEFAULT_WEIGHT: Decimal = dec!(0.25);
const BENEISH_SAFE: Decimal = dec!(-2.22);
const BENEISH_GREY: Decimal = dec!(-1.78);
const SLOAN_GREEN: Decimal = dec!(0.05);
const SLOAN_AMBER: Decimal = dec!(0.10);
const CASH_CONV_BONUS: Decimal = dec!(10);

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn abs_decimal(x: Decimal) -> Decimal {
    if x < Decimal::ZERO {
        -x
    } else {
        x
    }
}

/// Map Beneish M-Score to 0-100 component score.
fn score_beneish(m: Decimal) -> Decimal {
    if m < BENEISH_SAFE {
        dec!(100)
    } else if m < BENEISH_GREY {
        dec!(50)
    } else {
        Decimal::ZERO
    }
}

/// Map Piotroski F-Score (0-9) to 0-100 component score.
fn score_piotroski(f: u8) -> Decimal {
    let f_dec = Decimal::from(f);
    (f_dec / dec!(9)) * dec!(100)
}

/// Map Sloan ratio and cash conversion to 0-100 component score.
fn score_accrual(sloan: Decimal, cash_conv: Decimal) -> Decimal {
    let abs_sloan = abs_decimal(sloan);
    let mut base = if abs_sloan < SLOAN_GREEN {
        dec!(100)
    } else if abs_sloan < SLOAN_AMBER {
        dec!(50)
    } else {
        Decimal::ZERO
    };

    // Bonus for strong cash conversion
    if cash_conv > Decimal::ONE {
        base += CASH_CONV_BONUS;
        if base > dec!(100) {
            base = dec!(100);
        }
    }

    base
}

/// Classify the composite score into a rating.
fn classify(score: Decimal) -> String {
    if score >= dec!(75) {
        "High Quality".to_string()
    } else if score >= dec!(50) {
        "Acceptable".to_string()
    } else if score >= dec!(25) {
        "Caution".to_string()
    } else {
        "Red Flag".to_string()
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Compute the earnings quality composite score from pre-computed component metrics.
pub fn calculate_earnings_quality_composite(
    input: &EarningsQualityCompositeInput,
) -> CorpFinanceResult<EarningsQualityCompositeOutput> {
    // ---- Validation ----
    if input.piotroski_f_score > 9 {
        return Err(CorpFinanceError::InvalidInput {
            field: "piotroski_f_score".into(),
            reason: "Must be 0-9".into(),
        });
    }
    if input.revenue_quality_score < Decimal::ZERO || input.revenue_quality_score > dec!(100) {
        return Err(CorpFinanceError::InvalidInput {
            field: "revenue_quality_score".into(),
            reason: "Must be in [0, 100]".into(),
        });
    }

    // ---- Resolve weights ----
    let w_b = input.weight_beneish.unwrap_or(DEFAULT_WEIGHT);
    let w_p = input.weight_piotroski.unwrap_or(DEFAULT_WEIGHT);
    let w_a = input.weight_accrual.unwrap_or(DEFAULT_WEIGHT);
    let w_r = input.weight_revenue.unwrap_or(DEFAULT_WEIGHT);

    let total_weight = w_b + w_p + w_a + w_r;
    if total_weight <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "weights".into(),
            reason: "Sum of weights must be positive".into(),
        });
    }

    // Validate no negative weights
    if w_b < Decimal::ZERO || w_p < Decimal::ZERO || w_a < Decimal::ZERO || w_r < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "weights".into(),
            reason: "Weights must be non-negative".into(),
        });
    }

    // ---- Component scores ----
    let beneish_component = score_beneish(input.beneish_m_score);
    let piotroski_component = score_piotroski(input.piotroski_f_score);
    let accrual_component = score_accrual(input.sloan_ratio, input.cash_conversion);
    let revenue_component = input.revenue_quality_score;

    // ---- Weighted composite ----
    let weighted_sum = w_b * beneish_component
        + w_p * piotroski_component
        + w_a * accrual_component
        + w_r * revenue_component;
    let composite_score = weighted_sum / total_weight;

    // Clamp to [0, 100]
    let composite_score = if composite_score > dec!(100) {
        dec!(100)
    } else if composite_score < Decimal::ZERO {
        Decimal::ZERO
    } else {
        composite_score
    };

    let classification = classify(composite_score);

    Ok(EarningsQualityCompositeOutput {
        composite_score,
        classification,
        component_scores: ComponentScores {
            beneish_component,
            piotroski_component,
            accrual_component,
            revenue_component,
        },
    })
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn high_quality_input() -> EarningsQualityCompositeInput {
        EarningsQualityCompositeInput {
            beneish_m_score: dec!(-3.0), // safe
            piotroski_f_score: 8,
            sloan_ratio: dec!(0.02),
            cash_conversion: dec!(1.3),
            revenue_quality_score: dec!(85),
            weight_beneish: None,
            weight_piotroski: None,
            weight_accrual: None,
            weight_revenue: None,
        }
    }

    fn red_flag_input() -> EarningsQualityCompositeInput {
        EarningsQualityCompositeInput {
            beneish_m_score: dec!(-1.0), // manipulative
            piotroski_f_score: 1,
            sloan_ratio: dec!(0.15),
            cash_conversion: dec!(0.3),
            revenue_quality_score: dec!(10),
            weight_beneish: None,
            weight_piotroski: None,
            weight_accrual: None,
            weight_revenue: None,
        }
    }

    #[test]
    fn test_high_quality_classification() {
        let out = calculate_earnings_quality_composite(&high_quality_input()).unwrap();
        assert_eq!(out.classification, "High Quality");
        assert!(out.composite_score >= dec!(75));
    }

    #[test]
    fn test_red_flag_classification() {
        let out = calculate_earnings_quality_composite(&red_flag_input()).unwrap();
        assert_eq!(out.classification, "Red Flag");
        assert!(out.composite_score < dec!(25));
    }

    #[test]
    fn test_beneish_component_safe() {
        let comp = score_beneish(dec!(-3.0));
        assert_eq!(comp, dec!(100));
    }

    #[test]
    fn test_beneish_component_grey() {
        let comp = score_beneish(dec!(-2.0));
        assert_eq!(comp, dec!(50));
    }

    #[test]
    fn test_beneish_component_danger() {
        let comp = score_beneish(dec!(-1.5));
        assert_eq!(comp, Decimal::ZERO);
    }

    #[test]
    fn test_piotroski_component_perfect() {
        let comp = score_piotroski(9);
        assert_eq!(comp, dec!(100));
    }

    #[test]
    fn test_piotroski_component_zero() {
        let comp = score_piotroski(0);
        assert_eq!(comp, Decimal::ZERO);
    }

    #[test]
    fn test_piotroski_component_mid() {
        // 5/9 * 100 = 55.555..
        let comp = score_piotroski(5);
        let expected = (dec!(5) / dec!(9)) * dec!(100);
        assert_eq!(comp, expected);
    }

    #[test]
    fn test_accrual_component_green_with_bonus() {
        let comp = score_accrual(dec!(0.03), dec!(1.5));
        assert_eq!(comp, dec!(100)); // 100, capped at 100 after bonus
    }

    #[test]
    fn test_accrual_component_green_without_bonus() {
        let comp = score_accrual(dec!(0.03), dec!(0.8));
        assert_eq!(comp, dec!(100)); // base = 100, no bonus
    }

    #[test]
    fn test_accrual_component_amber() {
        let comp = score_accrual(dec!(0.07), dec!(0.8));
        assert_eq!(comp, dec!(50));
    }

    #[test]
    fn test_accrual_component_amber_with_bonus() {
        let comp = score_accrual(dec!(0.07), dec!(1.2));
        assert_eq!(comp, dec!(60)); // 50 + 10 bonus
    }

    #[test]
    fn test_accrual_component_red() {
        let comp = score_accrual(dec!(0.15), dec!(0.5));
        assert_eq!(comp, Decimal::ZERO);
    }

    #[test]
    fn test_equal_weights() {
        let input = high_quality_input();
        let out = calculate_earnings_quality_composite(&input).unwrap();
        // beneish=100, piotroski=8/9*100=88.88, accrual=100, revenue=85
        // avg = (100 + 88.88 + 100 + 85) / 4 = 93.47
        assert!(out.composite_score > dec!(90));
    }

    #[test]
    fn test_custom_weights() {
        let mut input = high_quality_input();
        input.weight_beneish = Some(dec!(0.40));
        input.weight_piotroski = Some(dec!(0.20));
        input.weight_accrual = Some(dec!(0.20));
        input.weight_revenue = Some(dec!(0.20));
        let out = calculate_earnings_quality_composite(&input).unwrap();
        // Beneish has higher weight, all components strong => still High Quality
        assert_eq!(out.classification, "High Quality");
    }

    #[test]
    fn test_acceptable_classification() {
        let input = EarningsQualityCompositeInput {
            beneish_m_score: dec!(-2.0), // grey => 50
            piotroski_f_score: 5,        // ~55.5
            sloan_ratio: dec!(0.07),     // amber => 50
            cash_conversion: dec!(0.9),
            revenue_quality_score: dec!(60),
            weight_beneish: None,
            weight_piotroski: None,
            weight_accrual: None,
            weight_revenue: None,
        };
        let out = calculate_earnings_quality_composite(&input).unwrap();
        assert_eq!(out.classification, "Acceptable");
    }

    #[test]
    fn test_caution_classification() {
        let input = EarningsQualityCompositeInput {
            beneish_m_score: dec!(-1.5), // danger => 0
            piotroski_f_score: 3,        // 33.3
            sloan_ratio: dec!(0.07),     // amber => 50
            cash_conversion: dec!(0.8),
            revenue_quality_score: dec!(40),
            weight_beneish: None,
            weight_piotroski: None,
            weight_accrual: None,
            weight_revenue: None,
        };
        let out = calculate_earnings_quality_composite(&input).unwrap();
        assert_eq!(out.classification, "Caution");
    }

    #[test]
    fn test_boundary_75() {
        let class = classify(dec!(75));
        assert_eq!(class, "High Quality");
    }

    #[test]
    fn test_boundary_50() {
        let class = classify(dec!(50));
        assert_eq!(class, "Acceptable");
    }

    #[test]
    fn test_boundary_25() {
        let class = classify(dec!(25));
        assert_eq!(class, "Caution");
    }

    #[test]
    fn test_boundary_24() {
        let class = classify(dec!(24));
        assert_eq!(class, "Red Flag");
    }

    #[test]
    fn test_invalid_f_score() {
        let mut input = high_quality_input();
        input.piotroski_f_score = 10;
        assert!(calculate_earnings_quality_composite(&input).is_err());
    }

    #[test]
    fn test_invalid_revenue_score_high() {
        let mut input = high_quality_input();
        input.revenue_quality_score = dec!(101);
        assert!(calculate_earnings_quality_composite(&input).is_err());
    }

    #[test]
    fn test_invalid_revenue_score_negative() {
        let mut input = high_quality_input();
        input.revenue_quality_score = dec!(-1);
        assert!(calculate_earnings_quality_composite(&input).is_err());
    }

    #[test]
    fn test_negative_weight_rejected() {
        let mut input = high_quality_input();
        input.weight_beneish = Some(dec!(-0.1));
        assert!(calculate_earnings_quality_composite(&input).is_err());
    }

    #[test]
    fn test_zero_total_weight_rejected() {
        let mut input = high_quality_input();
        input.weight_beneish = Some(Decimal::ZERO);
        input.weight_piotroski = Some(Decimal::ZERO);
        input.weight_accrual = Some(Decimal::ZERO);
        input.weight_revenue = Some(Decimal::ZERO);
        assert!(calculate_earnings_quality_composite(&input).is_err());
    }

    #[test]
    fn test_serialization_roundtrip() {
        let input = high_quality_input();
        let json = serde_json::to_string(&input).unwrap();
        let deser: EarningsQualityCompositeInput = serde_json::from_str(&json).unwrap();
        let out1 = calculate_earnings_quality_composite(&input).unwrap();
        let out2 = calculate_earnings_quality_composite(&deser).unwrap();
        assert_eq!(out1.composite_score, out2.composite_score);
    }

    #[test]
    fn test_output_serialization() {
        let out = calculate_earnings_quality_composite(&high_quality_input()).unwrap();
        let json = serde_json::to_string(&out).unwrap();
        let deser: EarningsQualityCompositeOutput = serde_json::from_str(&json).unwrap();
        assert_eq!(out.composite_score, deser.composite_score);
        assert_eq!(out.classification, deser.classification);
    }

    #[test]
    fn test_negative_sloan_treated_as_positive() {
        // Negative sloan (CFO > NI) should be treated the same as positive for abs comparison
        let comp = score_accrual(dec!(-0.03), dec!(1.0));
        assert_eq!(comp, dec!(100)); // |sloan| < 0.05 => 100
    }
}
