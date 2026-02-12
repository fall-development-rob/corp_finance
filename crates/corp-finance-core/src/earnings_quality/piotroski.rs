//! Piotroski F-Score model for financial strength assessment.
//!
//! Implements the 9-signal binary scoring model from Joseph Piotroski (2000):
//!
//! **Profitability**:
//! 1. ROA > 0
//! 2. CFO > 0
//! 3. DROA > 0 (improving return on assets)
//! 4. Accruals quality (CFO > NI)
//!
//! **Leverage / Liquidity**:
//! 5. DLeverage < 0 (deleveraging)
//! 6. DCurrent Ratio > 0
//! 7. No equity dilution
//!
//! **Operating Efficiency**:
//! 8. DGross Margin > 0
//! 9. DAsset Turnover > 0
//!
//! Score 7-9 = Strong, 4-6 = Moderate, 0-3 = Weak.
//!
//! All arithmetic uses `rust_decimal::Decimal`. No `f64`.

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::error::CorpFinanceError;
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Input / Output
// ---------------------------------------------------------------------------

/// Financial data required for the Piotroski F-Score calculation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PiotroskiInput {
    // Profitability
    pub net_income: Decimal,
    pub total_assets: Decimal,
    pub prior_total_assets: Decimal,
    pub cfo: Decimal,
    pub prior_net_income: Decimal,
    pub prior_cfo: Decimal,

    // Leverage / Liquidity
    pub current_long_term_debt: Decimal,
    pub prior_long_term_debt: Decimal,
    pub current_current_assets: Decimal,
    pub current_current_liabilities: Decimal,
    pub prior_current_assets: Decimal,
    pub prior_current_liabilities: Decimal,
    pub shares_outstanding: Decimal,
    pub prior_shares_outstanding: Decimal,

    // Operating efficiency
    pub current_gross_margin: Decimal,
    pub prior_gross_margin: Decimal,
    pub current_asset_turnover: Decimal,
    pub prior_asset_turnover: Decimal,
}

/// Individual signal results and the composite F-Score.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PiotroskiOutput {
    // Profitability signals
    pub roa_positive: bool,
    pub cfo_positive: bool,
    pub delta_roa_positive: bool,
    pub accruals_quality: bool,

    // Leverage / Liquidity signals
    pub leverage_decreasing: bool,
    pub current_ratio_increasing: bool,
    pub no_equity_dilution: bool,

    // Operating efficiency signals
    pub gross_margin_increasing: bool,
    pub asset_turnover_increasing: bool,

    /// Total score (0-9).
    pub f_score: u8,
    /// Classification: "Strong" (7-9), "Moderate" (4-6), "Weak" (0-3).
    pub strength: String,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn safe_div(num: Decimal, den: Decimal, ctx: &str) -> CorpFinanceResult<Decimal> {
    if den == Decimal::ZERO {
        return Err(CorpFinanceError::DivisionByZero {
            context: ctx.to_string(),
        });
    }
    Ok(num / den)
}

fn classify(score: u8) -> String {
    match score {
        7..=9 => "Strong".to_string(),
        4..=6 => "Moderate".to_string(),
        _ => "Weak".to_string(),
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Compute the Piotroski F-Score (9 binary signals, score 0-9).
pub fn calculate_piotroski_f_score(input: &PiotroskiInput) -> CorpFinanceResult<PiotroskiOutput> {
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
    if input.shares_outstanding <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "shares_outstanding".into(),
            reason: "Must be positive".into(),
        });
    }
    if input.prior_shares_outstanding <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "prior_shares_outstanding".into(),
            reason: "Must be positive".into(),
        });
    }

    // ---- Signal 1: ROA > 0 ----
    let roa = safe_div(input.net_income, input.total_assets, "ROA")?;
    let roa_positive = roa > Decimal::ZERO;

    // ---- Signal 2: CFO > 0 ----
    let cfo_positive = input.cfo > Decimal::ZERO;

    // ---- Signal 3: Delta ROA > 0 ----
    let prior_roa = safe_div(
        input.prior_net_income,
        input.prior_total_assets,
        "prior ROA",
    )?;
    let delta_roa_positive = roa > prior_roa;

    // ---- Signal 4: Accruals quality (CFO > NI) ----
    let accruals_quality = input.cfo > input.net_income;

    // ---- Signal 5: Leverage decreasing ----
    let leverage_decreasing = input.current_long_term_debt < input.prior_long_term_debt;

    // ---- Signal 6: Current ratio increasing ----
    let current_ratio = safe_div(
        input.current_current_assets,
        input.current_current_liabilities,
        "current ratio",
    )?;
    let prior_current_ratio = safe_div(
        input.prior_current_assets,
        input.prior_current_liabilities,
        "prior current ratio",
    )?;
    let current_ratio_increasing = current_ratio > prior_current_ratio;

    // ---- Signal 7: No equity dilution ----
    let no_equity_dilution = input.shares_outstanding <= input.prior_shares_outstanding;

    // ---- Signal 8: Gross margin increasing ----
    let gross_margin_increasing = input.current_gross_margin > input.prior_gross_margin;

    // ---- Signal 9: Asset turnover increasing ----
    let asset_turnover_increasing = input.current_asset_turnover > input.prior_asset_turnover;

    // ---- Tally ----
    let signals = [
        roa_positive,
        cfo_positive,
        delta_roa_positive,
        accruals_quality,
        leverage_decreasing,
        current_ratio_increasing,
        no_equity_dilution,
        gross_margin_increasing,
        asset_turnover_increasing,
    ];
    let f_score = signals.iter().filter(|&&s| s).count() as u8;
    let strength = classify(f_score);

    Ok(PiotroskiOutput {
        roa_positive,
        cfo_positive,
        delta_roa_positive,
        accruals_quality,
        leverage_decreasing,
        current_ratio_increasing,
        no_equity_dilution,
        gross_margin_increasing,
        asset_turnover_increasing,
        f_score,
        strength,
    })
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    /// Helper: build a strong company (all 9 signals positive).
    fn strong_input() -> PiotroskiInput {
        PiotroskiInput {
            net_income: dec!(100),
            total_assets: dec!(1000),
            prior_total_assets: dec!(1000),
            cfo: dec!(150), // > NI => accruals quality
            prior_net_income: dec!(80),
            prior_cfo: dec!(120),
            current_long_term_debt: dec!(200),
            prior_long_term_debt: dec!(250), // deleveraging
            current_current_assets: dec!(400),
            current_current_liabilities: dec!(200), // CR = 2.0
            prior_current_assets: dec!(350),
            prior_current_liabilities: dec!(200), // prior CR = 1.75
            shares_outstanding: dec!(100),
            prior_shares_outstanding: dec!(100), // no dilution
            current_gross_margin: dec!(0.45),
            prior_gross_margin: dec!(0.40), // improving
            current_asset_turnover: dec!(1.2),
            prior_asset_turnover: dec!(1.1), // improving
        }
    }

    /// Helper: build a weak company (all 9 signals negative).
    fn weak_input() -> PiotroskiInput {
        PiotroskiInput {
            net_income: dec!(-50),
            total_assets: dec!(1000),
            prior_total_assets: dec!(1000),
            cfo: dec!(-60), // negative CFO, and CFO < NI (-60 < -50) => accruals signal false
            prior_net_income: dec!(-20), // prior ROA = -0.02, current ROA = -0.05 => declining
            prior_cfo: dec!(10),
            current_long_term_debt: dec!(400),
            prior_long_term_debt: dec!(300), // more leverage
            current_current_assets: dec!(200),
            current_current_liabilities: dec!(300), // CR = 0.67
            prior_current_assets: dec!(250),
            prior_current_liabilities: dec!(300), // prior CR = 0.83 => declining
            shares_outstanding: dec!(120),
            prior_shares_outstanding: dec!(100), // dilution
            current_gross_margin: dec!(0.30),
            prior_gross_margin: dec!(0.35), // declining
            current_asset_turnover: dec!(0.8),
            prior_asset_turnover: dec!(0.9), // declining
        }
    }

    #[test]
    fn test_perfect_score() {
        let out = calculate_piotroski_f_score(&strong_input()).unwrap();
        assert_eq!(out.f_score, 9);
        assert_eq!(out.strength, "Strong");
    }

    #[test]
    fn test_zero_score() {
        let out = calculate_piotroski_f_score(&weak_input()).unwrap();
        assert_eq!(out.f_score, 0);
        assert_eq!(out.strength, "Weak");
    }

    #[test]
    fn test_roa_signal() {
        let mut input = weak_input();
        input.net_income = dec!(10); // positive ROA
        let out = calculate_piotroski_f_score(&input).unwrap();
        assert!(out.roa_positive);
    }

    #[test]
    fn test_cfo_signal() {
        let mut input = weak_input();
        input.cfo = dec!(50);
        let out = calculate_piotroski_f_score(&input).unwrap();
        assert!(out.cfo_positive);
    }

    #[test]
    fn test_delta_roa_signal() {
        let mut input = weak_input();
        input.net_income = dec!(100); // ROA = 0.10
        input.prior_net_income = dec!(50); // prior ROA = 0.05
        let out = calculate_piotroski_f_score(&input).unwrap();
        assert!(out.delta_roa_positive);
    }

    #[test]
    fn test_accruals_quality_signal() {
        let mut input = weak_input();
        input.cfo = dec!(200);
        input.net_income = dec!(100);
        let out = calculate_piotroski_f_score(&input).unwrap();
        assert!(out.accruals_quality);
    }

    #[test]
    fn test_leverage_signal() {
        let mut input = weak_input();
        input.current_long_term_debt = dec!(200);
        input.prior_long_term_debt = dec!(300);
        let out = calculate_piotroski_f_score(&input).unwrap();
        assert!(out.leverage_decreasing);
    }

    #[test]
    fn test_current_ratio_signal() {
        let mut input = weak_input();
        input.current_current_assets = dec!(400);
        input.current_current_liabilities = dec!(200);
        input.prior_current_assets = dec!(300);
        input.prior_current_liabilities = dec!(200);
        let out = calculate_piotroski_f_score(&input).unwrap();
        assert!(out.current_ratio_increasing);
    }

    #[test]
    fn test_no_dilution_signal() {
        let mut input = weak_input();
        input.shares_outstanding = dec!(100);
        input.prior_shares_outstanding = dec!(100);
        let out = calculate_piotroski_f_score(&input).unwrap();
        assert!(out.no_equity_dilution);
    }

    #[test]
    fn test_gross_margin_signal() {
        let mut input = weak_input();
        input.current_gross_margin = dec!(0.40);
        input.prior_gross_margin = dec!(0.35);
        let out = calculate_piotroski_f_score(&input).unwrap();
        assert!(out.gross_margin_increasing);
    }

    #[test]
    fn test_asset_turnover_signal() {
        let mut input = weak_input();
        input.current_asset_turnover = dec!(1.2);
        input.prior_asset_turnover = dec!(1.0);
        let out = calculate_piotroski_f_score(&input).unwrap();
        assert!(out.asset_turnover_increasing);
    }

    #[test]
    fn test_moderate_score() {
        let mut input = weak_input();
        // Turn on exactly 5 signals
        input.net_income = dec!(100); // ROA+ , DROA+
        input.cfo = dec!(150); // CFO+, accruals+
        input.current_long_term_debt = dec!(200);
        input.prior_long_term_debt = dec!(300); // leverage-
        let out = calculate_piotroski_f_score(&input).unwrap();
        assert_eq!(out.strength, "Moderate");
        assert!(out.f_score >= 4 && out.f_score <= 6);
    }

    #[test]
    fn test_boundary_strong_7() {
        assert_eq!(classify(7), "Strong");
    }

    #[test]
    fn test_boundary_moderate_4() {
        assert_eq!(classify(4), "Moderate");
    }

    #[test]
    fn test_boundary_moderate_6() {
        assert_eq!(classify(6), "Moderate");
    }

    #[test]
    fn test_boundary_weak_3() {
        assert_eq!(classify(3), "Weak");
    }

    #[test]
    fn test_zero_total_assets_rejected() {
        let mut input = strong_input();
        input.total_assets = Decimal::ZERO;
        let err = calculate_piotroski_f_score(&input).unwrap_err();
        match err {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "total_assets");
            }
            _ => panic!("Expected InvalidInput"),
        }
    }

    #[test]
    fn test_zero_prior_total_assets_rejected() {
        let mut input = strong_input();
        input.prior_total_assets = Decimal::ZERO;
        let err = calculate_piotroski_f_score(&input).unwrap_err();
        match err {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "prior_total_assets");
            }
            _ => panic!("Expected InvalidInput"),
        }
    }

    #[test]
    fn test_zero_shares_rejected() {
        let mut input = strong_input();
        input.shares_outstanding = Decimal::ZERO;
        assert!(calculate_piotroski_f_score(&input).is_err());
    }

    #[test]
    fn test_zero_prior_shares_rejected() {
        let mut input = strong_input();
        input.prior_shares_outstanding = Decimal::ZERO;
        assert!(calculate_piotroski_f_score(&input).is_err());
    }

    #[test]
    fn test_serialization_roundtrip() {
        let input = strong_input();
        let json = serde_json::to_string(&input).unwrap();
        let deser: PiotroskiInput = serde_json::from_str(&json).unwrap();
        let out1 = calculate_piotroski_f_score(&input).unwrap();
        let out2 = calculate_piotroski_f_score(&deser).unwrap();
        assert_eq!(out1.f_score, out2.f_score);
    }

    #[test]
    fn test_output_serialization() {
        let out = calculate_piotroski_f_score(&strong_input()).unwrap();
        let json = serde_json::to_string(&out).unwrap();
        let deser: PiotroskiOutput = serde_json::from_str(&json).unwrap();
        assert_eq!(out.f_score, deser.f_score);
        assert_eq!(out.strength, deser.strength);
    }
}
