//! Accrual quality metrics for earnings analysis.
//!
//! Implements:
//! 1. **Sloan Ratio** -- (NI - CFO) / avg total assets; lower = higher quality.
//! 2. **Cash Conversion** -- CFO / NI; > 1.0 indicates strong cash backing.
//! 3. **Total Accruals** -- NI - CFO.
//! 4. **Working Capital Accrual** -- DCA - DCL.
//! 5. **Non-current Operating Accrual** -- total accruals - working capital accrual.
//! 6. **Quality Score** -- green / amber / red classification.
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

/// Financial data required for accrual quality analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccrualQualityInput {
    pub net_income: Decimal,
    pub cfo: Decimal,
    pub total_assets: Decimal,
    pub prior_total_assets: Decimal,
    pub current_assets: Decimal,
    pub prior_current_assets: Decimal,
    pub current_liabilities: Decimal,
    pub prior_current_liabilities: Decimal,
    pub depreciation: Decimal,
    pub revenue: Decimal,
    pub prior_revenue: Decimal,
    pub ppe: Decimal,
    pub prior_ppe: Decimal,
}

/// Accrual quality analysis results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccrualQualityOutput {
    /// (NI - CFO) / average total assets.
    pub sloan_ratio: Decimal,
    /// CFO / NI. `None` when NI == 0.
    pub cash_conversion: Option<Decimal>,
    /// NI - CFO.
    pub total_accruals: Decimal,
    /// Delta(current_assets) - Delta(current_liabilities).
    pub working_capital_accrual: Decimal,
    /// total_accruals - working_capital_accrual.
    pub noncurrent_operating_accrual: Decimal,
    /// "Green", "Amber", or "Red".
    pub accrual_quality_score: String,
}

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

fn classify_quality(sloan: Decimal, cash_conv: Option<Decimal>) -> String {
    let abs_sloan = abs_decimal(sloan);
    if abs_sloan < dec!(0.05) {
        // Check for bonus: cash_conversion > 1.0
        if let Some(cc) = cash_conv {
            if cc > Decimal::ONE {
                return "Green".to_string();
            }
        }
        // Still green even without bonus if sloan is very low
        "Green".to_string()
    } else if abs_sloan < dec!(0.10) {
        "Amber".to_string()
    } else {
        "Red".to_string()
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Compute accrual quality metrics (Sloan ratio, cash conversion, decomposition).
pub fn calculate_accrual_quality(
    input: &AccrualQualityInput,
) -> CorpFinanceResult<AccrualQualityOutput> {
    // ---- Validation ----
    if input.total_assets <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "total_assets".into(),
            reason: "Must be positive".into(),
        });
    }
    if input.prior_total_assets <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "prior_total_assets".into(),
            reason: "Must be positive".into(),
        });
    }

    // ---- Total Accruals ----
    let total_accruals = input.net_income - input.cfo;

    // ---- Sloan Ratio ----
    let avg_assets = (input.total_assets + input.prior_total_assets) / dec!(2);
    if avg_assets == Decimal::ZERO {
        return Err(CorpFinanceError::DivisionByZero {
            context: "average total assets".into(),
        });
    }
    let sloan_ratio = total_accruals / avg_assets;

    // ---- Cash Conversion ----
    let cash_conversion = if input.net_income != Decimal::ZERO {
        Some(input.cfo / input.net_income)
    } else {
        None
    };

    // ---- Working Capital Accrual ----
    let delta_ca = input.current_assets - input.prior_current_assets;
    let delta_cl = input.current_liabilities - input.prior_current_liabilities;
    let working_capital_accrual = delta_ca - delta_cl;

    // ---- Non-current Operating Accrual ----
    let noncurrent_operating_accrual = total_accruals - working_capital_accrual;

    // ---- Classification ----
    let accrual_quality_score = classify_quality(sloan_ratio, cash_conversion);

    Ok(AccrualQualityOutput {
        sloan_ratio,
        cash_conversion,
        total_accruals,
        working_capital_accrual,
        noncurrent_operating_accrual,
        accrual_quality_score,
    })
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    /// Helper: company with high cash quality (CFO >> NI).
    fn high_quality_input() -> AccrualQualityInput {
        AccrualQualityInput {
            net_income: dec!(100),
            cfo: dec!(140),
            total_assets: dec!(2000),
            prior_total_assets: dec!(1900),
            current_assets: dec!(500),
            prior_current_assets: dec!(480),
            current_liabilities: dec!(300),
            prior_current_liabilities: dec!(290),
            depreciation: dec!(80),
            revenue: dec!(1000),
            prior_revenue: dec!(950),
            ppe: dec!(800),
            prior_ppe: dec!(770),
        }
    }

    /// Helper: company with low cash quality (NI >> CFO).
    fn low_quality_input() -> AccrualQualityInput {
        AccrualQualityInput {
            net_income: dec!(200),
            cfo: dec!(20),
            total_assets: dec!(1000),
            prior_total_assets: dec!(950),
            current_assets: dec!(400),
            prior_current_assets: dec!(300),
            current_liabilities: dec!(200),
            prior_current_liabilities: dec!(180),
            depreciation: dec!(50),
            revenue: dec!(800),
            prior_revenue: dec!(700),
            ppe: dec!(500),
            prior_ppe: dec!(450),
        }
    }

    #[test]
    fn test_high_quality_green() {
        let out = calculate_accrual_quality(&high_quality_input()).unwrap();
        assert_eq!(out.accrual_quality_score, "Green");
    }

    #[test]
    fn test_low_quality_red() {
        let out = calculate_accrual_quality(&low_quality_input()).unwrap();
        // sloan = (200-20)/((1000+950)/2) = 180/975 = 0.1846..
        assert_eq!(out.accrual_quality_score, "Red");
    }

    #[test]
    fn test_sloan_ratio_calculation() {
        let out = calculate_accrual_quality(&high_quality_input()).unwrap();
        // sloan = (100-140) / ((2000+1900)/2) = -40 / 1950
        let expected = dec!(-40) / dec!(1950);
        assert_eq!(out.sloan_ratio, expected);
    }

    #[test]
    fn test_negative_sloan_is_good() {
        let out = calculate_accrual_quality(&high_quality_input()).unwrap();
        assert!(out.sloan_ratio < Decimal::ZERO);
        assert_eq!(out.accrual_quality_score, "Green");
    }

    #[test]
    fn test_cash_conversion_greater_than_one() {
        let out = calculate_accrual_quality(&high_quality_input()).unwrap();
        // cash_conv = 140 / 100 = 1.4
        assert_eq!(out.cash_conversion, Some(dec!(1.4)));
    }

    #[test]
    fn test_cash_conversion_zero_ni() {
        let mut input = high_quality_input();
        input.net_income = Decimal::ZERO;
        let out = calculate_accrual_quality(&input).unwrap();
        assert_eq!(out.cash_conversion, None);
    }

    #[test]
    fn test_total_accruals() {
        let out = calculate_accrual_quality(&high_quality_input()).unwrap();
        // total_accruals = 100 - 140 = -40
        assert_eq!(out.total_accruals, dec!(-40));
    }

    #[test]
    fn test_working_capital_accrual() {
        let out = calculate_accrual_quality(&high_quality_input()).unwrap();
        // wc_accrual = (500-480) - (300-290) = 20 - 10 = 10
        assert_eq!(out.working_capital_accrual, dec!(10));
    }

    #[test]
    fn test_noncurrent_operating_accrual() {
        let out = calculate_accrual_quality(&high_quality_input()).unwrap();
        // noncurrent = total_accruals - wc_accrual = -40 - 10 = -50
        assert_eq!(out.noncurrent_operating_accrual, dec!(-50));
    }

    #[test]
    fn test_amber_classification() {
        // Sloan around 0.07 => Amber
        let input = AccrualQualityInput {
            net_income: dec!(100),
            cfo: dec!(30),
            total_assets: dec!(1000),
            prior_total_assets: dec!(1000),
            current_assets: dec!(400),
            prior_current_assets: dec!(380),
            current_liabilities: dec!(200),
            prior_current_liabilities: dec!(195),
            depreciation: dec!(50),
            revenue: dec!(800),
            prior_revenue: dec!(750),
            ppe: dec!(500),
            prior_ppe: dec!(480),
        };
        let out = calculate_accrual_quality(&input).unwrap();
        // sloan = (100-30)/1000 = 0.07
        assert_eq!(out.accrual_quality_score, "Amber");
    }

    #[test]
    fn test_zero_total_assets_rejected() {
        let mut input = high_quality_input();
        input.total_assets = Decimal::ZERO;
        let err = calculate_accrual_quality(&input).unwrap_err();
        match err {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "total_assets");
            }
            _ => panic!("Expected InvalidInput"),
        }
    }

    #[test]
    fn test_zero_prior_total_assets_rejected() {
        let mut input = high_quality_input();
        input.prior_total_assets = Decimal::ZERO;
        let err = calculate_accrual_quality(&input).unwrap_err();
        match err {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "prior_total_assets");
            }
            _ => panic!("Expected InvalidInput"),
        }
    }

    #[test]
    fn test_negative_net_income_handling() {
        let mut input = high_quality_input();
        input.net_income = dec!(-50);
        input.cfo = dec!(60);
        let out = calculate_accrual_quality(&input).unwrap();
        // total_accruals = -50 - 60 = -110
        assert_eq!(out.total_accruals, dec!(-110));
        // cash_conv = 60 / -50 = -1.2
        assert_eq!(out.cash_conversion, Some(dec!(-1.2)));
    }

    #[test]
    fn test_large_positive_accruals() {
        let mut input = low_quality_input();
        input.net_income = dec!(500);
        input.cfo = dec!(10);
        let out = calculate_accrual_quality(&input).unwrap();
        // sloan = (500-10)/((1000+950)/2) = 490/975 = 0.5025..
        assert_eq!(out.accrual_quality_score, "Red");
    }

    #[test]
    fn test_serialization_roundtrip() {
        let input = high_quality_input();
        let json = serde_json::to_string(&input).unwrap();
        let deser: AccrualQualityInput = serde_json::from_str(&json).unwrap();
        let out1 = calculate_accrual_quality(&input).unwrap();
        let out2 = calculate_accrual_quality(&deser).unwrap();
        assert_eq!(out1.sloan_ratio, out2.sloan_ratio);
    }

    #[test]
    fn test_output_serialization() {
        let out = calculate_accrual_quality(&high_quality_input()).unwrap();
        let json = serde_json::to_string(&out).unwrap();
        let deser: AccrualQualityOutput = serde_json::from_str(&json).unwrap();
        assert_eq!(out.sloan_ratio, deser.sloan_ratio);
        assert_eq!(out.accrual_quality_score, deser.accrual_quality_score);
    }
}
