//! Carbon offset valuation analytics.
//!
//! Covers:
//! 1. **Permanence factor** -- discount for reversal risk and limited permanence.
//! 2. **Additionality factor** -- score-based adjustment for additionality.
//! 3. **Vintage factor** -- age-based discount on older offsets.
//! 4. **Certification premium** -- spread by certification standard.
//! 5. **Type premium** -- spread by offset project type.
//! 6. **Co-benefit premium** -- +2% per co-benefit, capped at +10%.
//! 7. **Quality score & rating** -- composite 0-100 score mapped to rating tiers.
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

/// Input for carbon offset valuation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OffsetValuationInput {
    /// Market reference price for offsets ($/tCO2e).
    pub base_price: Decimal,
    /// Credit type: "nature_based", "renewable_energy", "methane_capture",
    /// "direct_air_capture", "avoided_deforestation".
    pub credit_type: String,
    /// Expected permanence in years.
    pub permanence_years: Decimal,
    /// Additionality score (0-100).
    pub additionality_score: Decimal,
    /// Vintage year of the offset.
    pub vintage_year: u32,
    /// Current (reference) year.
    pub current_year: u32,
    /// Certification standard: "verra_vcs", "gold_standard", "cdm", "voluntary", "none".
    pub certification: String,
    /// Co-benefits (e.g. "biodiversity", "community", "water").
    pub co_benefits: Vec<String>,
    /// Probability of reversal (0-1).
    pub reversal_risk: Decimal,
}

/// Output of carbon offset valuation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OffsetValuationOutput {
    /// Final adjusted price per tonne.
    pub adjusted_price: Decimal,
    /// Permanence discount factor.
    pub permanence_factor: Decimal,
    /// Additionality factor (0-1).
    pub additionality_factor: Decimal,
    /// Vintage discount factor.
    pub vintage_factor: Decimal,
    /// Certification premium percentage applied.
    pub certification_premium: Decimal,
    /// Type premium percentage applied.
    pub type_premium: Decimal,
    /// Co-benefit premium percentage applied.
    pub co_benefit_premium: Decimal,
    /// Composite quality score (0-100).
    pub quality_score: Decimal,
    /// Quality rating tier.
    pub quality_rating: String,
    /// Effective price after all adjustments.
    pub effective_price: Decimal,
}

// ---------------------------------------------------------------------------
// Core calculation
// ---------------------------------------------------------------------------

/// Compute carbon offset valuation with quality adjustments.
pub fn calculate_offset_valuation(
    input: &OffsetValuationInput,
) -> CorpFinanceResult<OffsetValuationOutput> {
    // --- Validation ---
    if input.base_price < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "base_price".into(),
            reason: "Base price cannot be negative".into(),
        });
    }
    if input.permanence_years < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "permanence_years".into(),
            reason: "Permanence years cannot be negative".into(),
        });
    }
    if input.additionality_score < Decimal::ZERO || input.additionality_score > dec!(100) {
        return Err(CorpFinanceError::InvalidInput {
            field: "additionality_score".into(),
            reason: "Additionality score must be between 0 and 100".into(),
        });
    }
    if input.reversal_risk < Decimal::ZERO || input.reversal_risk > Decimal::ONE {
        return Err(CorpFinanceError::InvalidInput {
            field: "reversal_risk".into(),
            reason: "Reversal risk must be between 0 and 1".into(),
        });
    }
    if input.vintage_year > input.current_year {
        return Err(CorpFinanceError::InvalidInput {
            field: "vintage_year".into(),
            reason: "Vintage year cannot be in the future".into(),
        });
    }
    let cert_lower = input.certification.to_lowercase();
    let valid_certs = ["verra_vcs", "gold_standard", "cdm", "voluntary", "none"];
    if !valid_certs.contains(&cert_lower.as_str()) {
        return Err(CorpFinanceError::InvalidInput {
            field: "certification".into(),
            reason: format!(
                "Unknown certification '{}'. Expected one of: {:?}",
                input.certification, valid_certs
            ),
        });
    }
    let type_lower = input.credit_type.to_lowercase();
    let valid_types = [
        "nature_based",
        "renewable_energy",
        "methane_capture",
        "direct_air_capture",
        "avoided_deforestation",
    ];
    if !valid_types.contains(&type_lower.as_str()) {
        return Err(CorpFinanceError::InvalidInput {
            field: "credit_type".into(),
            reason: format!(
                "Unknown credit type '{}'. Expected one of: {:?}",
                input.credit_type, valid_types
            ),
        });
    }

    // --- Permanence factor ---
    // permanence_factor = 1 - reversal_risk * (1 - min(permanence_years/100, 1))
    let perm_ratio = if input.permanence_years / dec!(100) < Decimal::ONE {
        input.permanence_years / dec!(100)
    } else {
        Decimal::ONE
    };
    let permanence_factor = Decimal::ONE - input.reversal_risk * (Decimal::ONE - perm_ratio);

    // --- Additionality factor ---
    let additionality_factor = input.additionality_score / dec!(100);

    // --- Vintage factor ---
    let age = input.current_year.saturating_sub(input.vintage_year);
    let vintage_discount = dec!(-0.03) * Decimal::from(age);
    let vintage_discount = if vintage_discount < dec!(-0.30) {
        dec!(-0.30)
    } else {
        vintage_discount
    };
    let vintage_factor = Decimal::ONE + vintage_discount;

    // --- Type premium ---
    let type_premium_pct = match type_lower.as_str() {
        "direct_air_capture" => dec!(0.25),
        "methane_capture" => dec!(0.10),
        "renewable_energy" => Decimal::ZERO,
        "nature_based" => dec!(-0.05),
        "avoided_deforestation" => dec!(-0.10),
        _ => Decimal::ZERO,
    };

    // --- Certification premium ---
    let cert_premium_pct = match cert_lower.as_str() {
        "gold_standard" => dec!(0.08),
        "verra_vcs" => dec!(0.05),
        "cdm" => dec!(0.02),
        "voluntary" => dec!(-0.05),
        "none" => dec!(-0.15),
        _ => Decimal::ZERO,
    };

    // --- Co-benefit premium ---
    let co_benefit_count = input.co_benefits.len() as u32;
    let co_benefit_raw = dec!(0.02) * Decimal::from(co_benefit_count);
    let co_benefit_premium_pct = if co_benefit_raw > dec!(0.10) {
        dec!(0.10)
    } else {
        co_benefit_raw
    };

    // --- Effective price ---
    // effective_price = base_price * permanence_factor * additionality_factor * vintage_factor
    //                   * (1 + type_premium) * (1 + cert_premium) * (1 + co_benefit_premium)
    let effective_price = input.base_price
        * permanence_factor
        * additionality_factor
        * vintage_factor
        * (Decimal::ONE + type_premium_pct)
        * (Decimal::ONE + cert_premium_pct)
        * (Decimal::ONE + co_benefit_premium_pct);

    // --- Quality score (0-100 composite) ---
    // permanence weight 30, additionality weight 30, vintage weight 20, certification weight 20
    let cert_score = match cert_lower.as_str() {
        "gold_standard" => dec!(100),
        "verra_vcs" => dec!(80),
        "cdm" => dec!(60),
        "voluntary" => dec!(30),
        "none" => dec!(10),
        _ => dec!(0),
    };
    let quality_score = permanence_factor * dec!(100) * dec!(0.30)
        + additionality_factor * dec!(100) * dec!(0.30)
        + vintage_factor * dec!(100) * dec!(0.20)
        + cert_score * dec!(0.20);

    // --- Quality rating ---
    let quality_rating = if quality_score >= dec!(80) {
        "Premium".to_string()
    } else if quality_score >= dec!(60) {
        "Standard".to_string()
    } else if quality_score >= dec!(40) {
        "Below Standard".to_string()
    } else {
        "Non-Investment Grade".to_string()
    };

    Ok(OffsetValuationOutput {
        adjusted_price: effective_price,
        permanence_factor,
        additionality_factor,
        vintage_factor,
        certification_premium: cert_premium_pct,
        type_premium: type_premium_pct,
        co_benefit_premium: co_benefit_premium_pct,
        quality_score,
        quality_rating,
        effective_price,
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn base_input() -> OffsetValuationInput {
        OffsetValuationInput {
            base_price: dec!(20),
            credit_type: "renewable_energy".into(),
            permanence_years: dec!(100),
            additionality_score: dec!(80),
            vintage_year: 2024,
            current_year: 2025,
            certification: "verra_vcs".into(),
            co_benefits: vec!["biodiversity".into(), "community".into()],
            reversal_risk: dec!(0.1),
        }
    }

    #[test]
    fn test_permanence_full_100_years() {
        let mut input = base_input();
        input.permanence_years = dec!(100);
        input.reversal_risk = dec!(0.5);
        let out = calculate_offset_valuation(&input).unwrap();
        // perm_ratio = min(100/100, 1) = 1.0
        // factor = 1 - 0.5 * (1-1) = 1.0
        assert_eq!(out.permanence_factor, Decimal::ONE);
    }

    #[test]
    fn test_permanence_partial() {
        let mut input = base_input();
        input.permanence_years = dec!(50);
        input.reversal_risk = dec!(0.2);
        let out = calculate_offset_valuation(&input).unwrap();
        // perm_ratio = 50/100 = 0.5
        // factor = 1 - 0.2 * (1 - 0.5) = 1 - 0.1 = 0.9
        assert_eq!(out.permanence_factor, dec!(0.9));
    }

    #[test]
    fn test_permanence_zero_reversal() {
        let mut input = base_input();
        input.reversal_risk = Decimal::ZERO;
        let out = calculate_offset_valuation(&input).unwrap();
        assert_eq!(out.permanence_factor, Decimal::ONE);
    }

    #[test]
    fn test_additionality_factor() {
        let input = base_input();
        let out = calculate_offset_valuation(&input).unwrap();
        assert_eq!(out.additionality_factor, dec!(0.80));
    }

    #[test]
    fn test_additionality_zero() {
        let mut input = base_input();
        input.additionality_score = Decimal::ZERO;
        let out = calculate_offset_valuation(&input).unwrap();
        assert_eq!(out.additionality_factor, Decimal::ZERO);
        assert_eq!(out.effective_price, Decimal::ZERO);
    }

    #[test]
    fn test_additionality_full() {
        let mut input = base_input();
        input.additionality_score = dec!(100);
        let out = calculate_offset_valuation(&input).unwrap();
        assert_eq!(out.additionality_factor, Decimal::ONE);
    }

    #[test]
    fn test_vintage_1_year_old() {
        let input = base_input();
        let out = calculate_offset_valuation(&input).unwrap();
        // age=1, discount = -0.03*1 = -3%
        assert_eq!(out.vintage_factor, dec!(0.97));
    }

    #[test]
    fn test_vintage_same_year() {
        let mut input = base_input();
        input.vintage_year = 2025;
        let out = calculate_offset_valuation(&input).unwrap();
        assert_eq!(out.vintage_factor, Decimal::ONE);
    }

    #[test]
    fn test_vintage_cap_at_30_pct() {
        let mut input = base_input();
        input.vintage_year = 2005;
        input.current_year = 2025;
        let out = calculate_offset_valuation(&input).unwrap();
        // age=20, discount = -0.03*20 = -60% but capped at -30%
        assert_eq!(out.vintage_factor, dec!(0.70));
    }

    #[test]
    fn test_type_dac_premium() {
        let mut input = base_input();
        input.credit_type = "direct_air_capture".into();
        let out = calculate_offset_valuation(&input).unwrap();
        assert_eq!(out.type_premium, dec!(0.25));
    }

    #[test]
    fn test_type_methane_premium() {
        let mut input = base_input();
        input.credit_type = "methane_capture".into();
        let out = calculate_offset_valuation(&input).unwrap();
        assert_eq!(out.type_premium, dec!(0.10));
    }

    #[test]
    fn test_type_nature_discount() {
        let mut input = base_input();
        input.credit_type = "nature_based".into();
        let out = calculate_offset_valuation(&input).unwrap();
        assert_eq!(out.type_premium, dec!(-0.05));
    }

    #[test]
    fn test_type_avoided_deforestation_discount() {
        let mut input = base_input();
        input.credit_type = "avoided_deforestation".into();
        let out = calculate_offset_valuation(&input).unwrap();
        assert_eq!(out.type_premium, dec!(-0.10));
    }

    #[test]
    fn test_certification_gold_standard() {
        let mut input = base_input();
        input.certification = "gold_standard".into();
        let out = calculate_offset_valuation(&input).unwrap();
        assert_eq!(out.certification_premium, dec!(0.08));
    }

    #[test]
    fn test_certification_none_discount() {
        let mut input = base_input();
        input.certification = "none".into();
        let out = calculate_offset_valuation(&input).unwrap();
        assert_eq!(out.certification_premium, dec!(-0.15));
    }

    #[test]
    fn test_co_benefits_two() {
        let input = base_input();
        let out = calculate_offset_valuation(&input).unwrap();
        assert_eq!(out.co_benefit_premium, dec!(0.04));
    }

    #[test]
    fn test_co_benefits_cap_at_10_pct() {
        let mut input = base_input();
        input.co_benefits = vec![
            "biodiversity".into(),
            "community".into(),
            "water".into(),
            "soil".into(),
            "air".into(),
            "education".into(),
        ];
        let out = calculate_offset_valuation(&input).unwrap();
        assert_eq!(out.co_benefit_premium, dec!(0.10));
    }

    #[test]
    fn test_co_benefits_none() {
        let mut input = base_input();
        input.co_benefits = vec![];
        let out = calculate_offset_valuation(&input).unwrap();
        assert_eq!(out.co_benefit_premium, Decimal::ZERO);
    }

    #[test]
    fn test_quality_rating_premium() {
        let mut input = base_input();
        input.additionality_score = dec!(100);
        input.permanence_years = dec!(100);
        input.reversal_risk = Decimal::ZERO;
        input.vintage_year = 2025;
        input.current_year = 2025;
        input.certification = "gold_standard".into();
        let out = calculate_offset_valuation(&input).unwrap();
        // perm=100*0.30=30, add=100*0.30=30, vintage=100*0.20=20, cert=100*0.20=20 => 100
        assert_eq!(out.quality_rating, "Premium");
    }

    #[test]
    fn test_quality_rating_non_investment_grade() {
        let mut input = base_input();
        input.additionality_score = dec!(10);
        input.permanence_years = dec!(5);
        input.reversal_risk = dec!(0.9);
        input.vintage_year = 2010;
        input.current_year = 2025;
        input.certification = "none".into();
        let out = calculate_offset_valuation(&input).unwrap();
        assert_eq!(out.quality_rating, "Non-Investment Grade");
    }

    #[test]
    fn test_negative_base_price_rejected() {
        let mut input = base_input();
        input.base_price = dec!(-5);
        let result = calculate_offset_valuation(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_additionality_out_of_range_rejected() {
        let mut input = base_input();
        input.additionality_score = dec!(150);
        let result = calculate_offset_valuation(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_reversal_risk_out_of_range_rejected() {
        let mut input = base_input();
        input.reversal_risk = dec!(1.5);
        let result = calculate_offset_valuation(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_certification_rejected() {
        let mut input = base_input();
        input.certification = "unknown".into();
        let result = calculate_offset_valuation(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_credit_type_rejected() {
        let mut input = base_input();
        input.credit_type = "unknown".into();
        let result = calculate_offset_valuation(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_effective_price_positive_high_quality() {
        let input = base_input();
        let out = calculate_offset_valuation(&input).unwrap();
        assert!(out.effective_price > Decimal::ZERO);
    }

    #[test]
    fn test_adjusted_equals_effective() {
        let input = base_input();
        let out = calculate_offset_valuation(&input).unwrap();
        assert_eq!(out.adjusted_price, out.effective_price);
    }
}
