//! Political risk assessment for emerging-market investments.
//! Composite scoring (6 WGI dims), binary penalties, risk premium, insurance,
//! expected loss, and mitigation recommendations. All Decimal, no f64.

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

use crate::error::CorpFinanceError;
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Input / Output
// ---------------------------------------------------------------------------

/// Breakdown of the six World Governance Indicator dimensions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoliticalDimensions {
    pub political_stability: Decimal,
    pub regulatory_quality: Decimal,
    pub rule_of_law: Decimal,
    pub control_of_corruption: Decimal,
    pub voice_accountability: Decimal,
    pub government_effectiveness: Decimal,
}

/// Input for political risk assessment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoliticalRiskInput {
    /// Country name / code.
    pub country: String,
    /// Political stability score 0-100.
    pub political_stability: Decimal,
    /// Regulatory quality score 0-100.
    pub regulatory_quality: Decimal,
    /// Rule of law score 0-100.
    pub rule_of_law: Decimal,
    /// Control of corruption score 0-100.
    pub control_of_corruption: Decimal,
    /// Voice and accountability score 0-100.
    pub voice_accountability: Decimal,
    /// Government effectiveness score 0-100.
    pub government_effectiveness: Decimal,
    /// Has history of expropriation.
    pub expropriation_history: bool,
    /// Currently under international sanctions.
    pub sanctions_risk: bool,
    /// Active conflict zone.
    pub conflict_zone: bool,
    /// Total amount at risk.
    pub investment_amount: Decimal,
    /// Political risk insurance premium as % of investment (e.g. 0.02 = 2%).
    pub insurance_premium_rate: Decimal,
}

/// Output of political risk assessment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoliticalRiskOutput {
    /// Composite score 0-100 (before binary penalties, floored at 0).
    pub composite_score: Decimal,
    /// Qualitative risk rating.
    pub risk_rating: String,
    /// Per-dimension scores.
    pub dimension_scores: PoliticalDimensions,
    /// Flags triggered by binary risk factors.
    pub binary_risk_flags: Vec<String>,
    /// Estimated additional return required: (100 - score)/100 * 10%.
    pub risk_premium_estimate: Decimal,
    /// Insurance value = investment_amount * insurance_premium_rate.
    pub insurance_value: Decimal,
    /// Expected loss = investment * (1 - score/100) * 0.15.
    pub expected_loss: Decimal,
    /// Context-specific recommendations.
    pub recommendations: Vec<String>,
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const WEIGHT_STABILITY: Decimal = dec!(0.20);
const WEIGHT_REGULATORY: Decimal = dec!(0.15);
const WEIGHT_RULE_OF_LAW: Decimal = dec!(0.20);
const WEIGHT_CORRUPTION: Decimal = dec!(0.15);
const WEIGHT_VOICE: Decimal = dec!(0.10);
const WEIGHT_EFFECTIVENESS: Decimal = dec!(0.20);

const PENALTY_EXPROPRIATION: Decimal = dec!(15);
const PENALTY_SANCTIONS: Decimal = dec!(25);
const PENALTY_CONFLICT: Decimal = dec!(20);

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

fn validate_score(name: &str, val: Decimal) -> CorpFinanceResult<()> {
    if val < Decimal::ZERO || val > dec!(100) {
        return Err(CorpFinanceError::InvalidInput {
            field: name.to_string(),
            reason: "Score must be between 0 and 100".to_string(),
        });
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Main function
// ---------------------------------------------------------------------------

/// Assess political risk for an emerging-market investment.
pub fn assess_political_risk(input: &PoliticalRiskInput) -> CorpFinanceResult<PoliticalRiskOutput> {
    // Validate all dimension scores
    validate_score("political_stability", input.political_stability)?;
    validate_score("regulatory_quality", input.regulatory_quality)?;
    validate_score("rule_of_law", input.rule_of_law)?;
    validate_score("control_of_corruption", input.control_of_corruption)?;
    validate_score("voice_accountability", input.voice_accountability)?;
    validate_score("government_effectiveness", input.government_effectiveness)?;

    if input.investment_amount < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "investment_amount".to_string(),
            reason: "Investment amount cannot be negative".to_string(),
        });
    }
    if input.insurance_premium_rate < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "insurance_premium_rate".to_string(),
            reason: "Insurance premium rate cannot be negative".to_string(),
        });
    }

    // Weighted composite (before penalties)
    let raw_composite = WEIGHT_STABILITY * input.political_stability
        + WEIGHT_REGULATORY * input.regulatory_quality
        + WEIGHT_RULE_OF_LAW * input.rule_of_law
        + WEIGHT_CORRUPTION * input.control_of_corruption
        + WEIGHT_VOICE * input.voice_accountability
        + WEIGHT_EFFECTIVENESS * input.government_effectiveness;

    // Binary penalties
    let mut penalty = Decimal::ZERO;
    let mut flags = Vec::new();

    if input.expropriation_history {
        penalty += PENALTY_EXPROPRIATION;
        flags.push("Expropriation history".to_string());
    }
    if input.sanctions_risk {
        penalty += PENALTY_SANCTIONS;
        flags.push("Sanctions risk".to_string());
    }
    if input.conflict_zone {
        penalty += PENALTY_CONFLICT;
        flags.push("Conflict zone".to_string());
    }

    let composite_score = if raw_composite - penalty < Decimal::ZERO {
        Decimal::ZERO
    } else {
        raw_composite - penalty
    };

    // Risk rating
    let risk_rating = if composite_score >= dec!(80) {
        "Low"
    } else if composite_score >= dec!(60) {
        "Moderate"
    } else if composite_score >= dec!(40) {
        "Elevated"
    } else if composite_score >= dec!(20) {
        "High"
    } else {
        "Very High"
    }
    .to_string();

    // Risk premium: (100 - score) / 100 * 10%
    let risk_premium_estimate = (dec!(100) - composite_score) / dec!(100) * dec!(0.10);

    // Insurance value
    let insurance_value = input.investment_amount * input.insurance_premium_rate;

    // Expected loss = investment * (1 - score/100) * 0.15
    let expected_loss =
        input.investment_amount * (Decimal::ONE - composite_score / dec!(100)) * dec!(0.15);

    // Recommendations
    let mut recommendations = Vec::new();
    if input.expropriation_history {
        recommendations.push("Consider bilateral investment treaty (BIT) protection".into());
    }
    if input.sanctions_risk {
        recommendations.push("Obtain sanctions compliance legal opinion before proceeding".into());
    }
    if input.conflict_zone {
        recommendations.push(
            "Secure political risk insurance (PRI) covering war and civil disturbance".into(),
        );
    }
    if composite_score < dec!(60) {
        recommendations.push(
            "Structure investment with strong contractual protections and exit rights".into(),
        );
    }
    if insurance_value < expected_loss {
        recommendations.push(
            "Insurance premium appears favourable relative to expected loss -- consider purchasing"
                .into(),
        );
    }
    if composite_score >= dec!(60) && flags.is_empty() {
        recommendations.push("Moderate risk -- standard due diligence procedures apply".into());
    }

    let dimension_scores = PoliticalDimensions {
        political_stability: input.political_stability,
        regulatory_quality: input.regulatory_quality,
        rule_of_law: input.rule_of_law,
        control_of_corruption: input.control_of_corruption,
        voice_accountability: input.voice_accountability,
        government_effectiveness: input.government_effectiveness,
    };

    Ok(PoliticalRiskOutput {
        composite_score,
        risk_rating,
        dimension_scores,
        binary_risk_flags: flags,
        risk_premium_estimate,
        insurance_value,
        expected_loss,
        recommendations,
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn base_input() -> PoliticalRiskInput {
        PoliticalRiskInput {
            country: "Brazil".to_string(),
            political_stability: dec!(55),
            regulatory_quality: dec!(60),
            rule_of_law: dec!(50),
            control_of_corruption: dec!(45),
            voice_accountability: dec!(65),
            government_effectiveness: dec!(55),
            expropriation_history: false,
            sanctions_risk: false,
            conflict_zone: false,
            investment_amount: dec!(100_000_000),
            insurance_premium_rate: dec!(0.015),
        }
    }

    #[test]
    fn test_stable_democracy() {
        let mut input = base_input();
        input.political_stability = dec!(85);
        input.regulatory_quality = dec!(90);
        input.rule_of_law = dec!(88);
        input.control_of_corruption = dec!(85);
        input.voice_accountability = dec!(92);
        input.government_effectiveness = dec!(87);
        let out = assess_political_risk(&input).unwrap();
        assert_eq!(out.risk_rating, "Low");
        assert!(out.composite_score >= dec!(80));
    }

    #[test]
    fn test_frontier_market() {
        let mut input = base_input();
        input.political_stability = dec!(30);
        input.regulatory_quality = dec!(25);
        input.rule_of_law = dec!(20);
        input.control_of_corruption = dec!(15);
        input.voice_accountability = dec!(35);
        input.government_effectiveness = dec!(25);
        let out = assess_political_risk(&input).unwrap();
        assert!(out.composite_score < dec!(40));
        assert!(out.risk_rating == "High" || out.risk_rating == "Very High");
    }

    #[test]
    fn test_sanctions_flag() {
        let mut input = base_input();
        input.sanctions_risk = true;
        let out = assess_political_risk(&input).unwrap();
        assert!(out
            .binary_risk_flags
            .contains(&"Sanctions risk".to_string()));
        // Should lower composite by 25
        let without = assess_political_risk(&{
            let mut i = input.clone();
            i.sanctions_risk = false;
            i
        })
        .unwrap();
        assert_eq!(
            without.composite_score - out.composite_score,
            PENALTY_SANCTIONS
        );
    }

    #[test]
    fn test_conflict_zone_flag() {
        let mut input = base_input();
        input.conflict_zone = true;
        let out = assess_political_risk(&input).unwrap();
        assert!(out.binary_risk_flags.contains(&"Conflict zone".to_string()));
    }

    #[test]
    fn test_expropriation_flag() {
        let mut input = base_input();
        input.expropriation_history = true;
        let out = assess_political_risk(&input).unwrap();
        assert!(out
            .binary_risk_flags
            .contains(&"Expropriation history".to_string()));
    }

    #[test]
    fn test_all_penalties_floor_at_zero() {
        let mut input = base_input();
        input.political_stability = dec!(10);
        input.regulatory_quality = dec!(10);
        input.rule_of_law = dec!(10);
        input.control_of_corruption = dec!(10);
        input.voice_accountability = dec!(10);
        input.government_effectiveness = dec!(10);
        input.expropriation_history = true;
        input.sanctions_risk = true;
        input.conflict_zone = true;
        let out = assess_political_risk(&input).unwrap();
        assert_eq!(out.composite_score, Decimal::ZERO);
        assert_eq!(out.risk_rating, "Very High");
    }

    #[test]
    fn test_risk_premium_formula() {
        let input = base_input();
        let out = assess_political_risk(&input).unwrap();
        let expected = (dec!(100) - out.composite_score) / dec!(100) * dec!(0.10);
        assert_eq!(out.risk_premium_estimate, expected);
    }

    #[test]
    fn test_insurance_value() {
        let input = base_input();
        let out = assess_political_risk(&input).unwrap();
        assert_eq!(out.insurance_value, dec!(100_000_000) * dec!(0.015));
    }

    #[test]
    fn test_expected_loss() {
        let input = base_input();
        let out = assess_political_risk(&input).unwrap();
        let expected =
            input.investment_amount * (Decimal::ONE - out.composite_score / dec!(100)) * dec!(0.15);
        assert_eq!(out.expected_loss, expected);
    }

    #[test]
    fn test_dimension_scores_passthrough() {
        let input = base_input();
        let out = assess_political_risk(&input).unwrap();
        assert_eq!(out.dimension_scores.political_stability, dec!(55));
        assert_eq!(out.dimension_scores.regulatory_quality, dec!(60));
        assert_eq!(out.dimension_scores.rule_of_law, dec!(50));
    }

    #[test]
    fn test_composite_weights_sum_to_one() {
        let total = WEIGHT_STABILITY
            + WEIGHT_REGULATORY
            + WEIGHT_RULE_OF_LAW
            + WEIGHT_CORRUPTION
            + WEIGHT_VOICE
            + WEIGHT_EFFECTIVENESS;
        assert_eq!(total, Decimal::ONE);
    }

    #[test]
    fn test_moderate_rating() {
        let mut input = base_input();
        // Set all to 65 -> composite = 65, rating = Moderate
        input.political_stability = dec!(65);
        input.regulatory_quality = dec!(65);
        input.rule_of_law = dec!(65);
        input.control_of_corruption = dec!(65);
        input.voice_accountability = dec!(65);
        input.government_effectiveness = dec!(65);
        let out = assess_political_risk(&input).unwrap();
        assert_eq!(out.composite_score, dec!(65));
        assert_eq!(out.risk_rating, "Moderate");
    }

    #[test]
    fn test_elevated_rating() {
        let mut input = base_input();
        input.political_stability = dec!(45);
        input.regulatory_quality = dec!(45);
        input.rule_of_law = dec!(45);
        input.control_of_corruption = dec!(45);
        input.voice_accountability = dec!(45);
        input.government_effectiveness = dec!(45);
        let out = assess_political_risk(&input).unwrap();
        assert_eq!(out.composite_score, dec!(45));
        assert_eq!(out.risk_rating, "Elevated");
    }

    #[test]
    fn test_high_rating() {
        let mut input = base_input();
        input.political_stability = dec!(25);
        input.regulatory_quality = dec!(25);
        input.rule_of_law = dec!(25);
        input.control_of_corruption = dec!(25);
        input.voice_accountability = dec!(25);
        input.government_effectiveness = dec!(25);
        let out = assess_political_risk(&input).unwrap();
        assert_eq!(out.composite_score, dec!(25));
        assert_eq!(out.risk_rating, "High");
    }

    #[test]
    fn test_invalid_score_above_100() {
        let mut input = base_input();
        input.political_stability = dec!(105);
        let err = assess_political_risk(&input).unwrap_err();
        assert!(matches!(err, CorpFinanceError::InvalidInput { .. }));
    }

    #[test]
    fn test_invalid_score_negative() {
        let mut input = base_input();
        input.rule_of_law = dec!(-5);
        let err = assess_political_risk(&input).unwrap_err();
        assert!(matches!(err, CorpFinanceError::InvalidInput { .. }));
    }

    #[test]
    fn test_invalid_negative_investment() {
        let mut input = base_input();
        input.investment_amount = dec!(-1000);
        let err = assess_political_risk(&input).unwrap_err();
        assert!(matches!(err, CorpFinanceError::InvalidInput { .. }));
    }

    #[test]
    fn test_recommendations_conflict() {
        let mut input = base_input();
        input.conflict_zone = true;
        let out = assess_political_risk(&input).unwrap();
        assert!(out.recommendations.iter().any(|r| r.contains("war")));
    }

    #[test]
    fn test_recommendations_sanctions() {
        let mut input = base_input();
        input.sanctions_risk = true;
        let out = assess_political_risk(&input).unwrap();
        assert!(out
            .recommendations
            .iter()
            .any(|r| r.contains("sanctions compliance")));
    }

    #[test]
    fn test_insurance_comparison() {
        let mut input = base_input();
        input.insurance_premium_rate = dec!(0.005); // cheap insurance
        input.political_stability = dec!(30);
        input.regulatory_quality = dec!(30);
        input.rule_of_law = dec!(30);
        input.control_of_corruption = dec!(30);
        input.voice_accountability = dec!(30);
        input.government_effectiveness = dec!(30);
        let out = assess_political_risk(&input).unwrap();
        // Insurance is cheap relative to expected loss
        assert!(out.insurance_value < out.expected_loss);
        assert!(out
            .recommendations
            .iter()
            .any(|r| r.contains("Insurance premium appears favourable")));
    }
}
