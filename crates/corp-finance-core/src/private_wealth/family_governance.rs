//! Family office governance scoring and maturity assessment.
//!
//! Evaluates family governance across three dimensions:
//! - **Structure** -- constitution, succession plan, conflict resolution.
//! - **Process** -- investment committee, regular meetings, reporting frequency.
//! - **Development** -- next-gen education, philanthropy, external advisors.
//!
//! Produces a 0-100 governance score and maturity level (1-5).
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

/// Input for family governance scoring.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FamilyGovernanceInput {
    /// Number of family members in governance.
    pub family_members: u32,
    /// Number of generations involved.
    pub generations_active: u32,
    /// Has a written family constitution.
    pub has_family_constitution: bool,
    /// Has a formal investment committee.
    pub has_investment_committee: bool,
    /// Has a documented succession plan.
    pub has_succession_plan: bool,
    /// Has a formal conflict resolution process.
    pub has_conflict_resolution: bool,
    /// Has a next-generation education program.
    pub has_next_gen_education: bool,
    /// Engages external professional advisors.
    pub has_external_advisors: bool,
    /// Holds regular (quarterly+) family meetings.
    pub has_regular_meetings: bool,
    /// Has an organized philanthropy program.
    pub has_philanthropy_program: bool,
    /// Total assets under management.
    pub total_aum: Decimal,
    /// Number of investment vehicles.
    pub num_investment_vehicles: u32,
    /// Reporting frequency: "monthly", "quarterly", "annually".
    pub reporting_frequency: String,
}

/// Dimension scores for governance assessment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernanceDimensions {
    /// Structure score (constitution + succession + conflict resolution).
    pub structure_score: Decimal,
    /// Process score (investment committee + meetings + reporting).
    pub process_score: Decimal,
    /// Development score (next gen + philanthropy + external advisors).
    pub development_score: Decimal,
    /// Complexity score (AUM, vehicles, generations).
    pub complexity_score: Decimal,
}

/// Output of family governance analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FamilyGovernanceOutput {
    /// Overall governance score (0-100).
    pub governance_score: Decimal,
    /// Rating: Institutional / Structured / Developing / Informal.
    pub governance_rating: String,
    /// Dimension breakdown.
    pub dimension_scores: GovernanceDimensions,
    /// Specific improvement recommendations.
    pub recommendations: Vec<String>,
    /// Maturity level (1-5).
    pub maturity_level: u8,
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

fn validate(input: &FamilyGovernanceInput) -> CorpFinanceResult<()> {
    if input.family_members == 0 {
        return Err(CorpFinanceError::InvalidInput {
            field: "family_members".into(),
            reason: "must be at least 1".into(),
        });
    }
    if input.generations_active == 0 {
        return Err(CorpFinanceError::InvalidInput {
            field: "generations_active".into(),
            reason: "must be at least 1".into(),
        });
    }
    if input.total_aum < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "total_aum".into(),
            reason: "cannot be negative".into(),
        });
    }
    let freq = input.reporting_frequency.to_lowercase();
    if freq != "monthly" && freq != "quarterly" && freq != "annually" {
        return Err(CorpFinanceError::InvalidInput {
            field: "reporting_frequency".into(),
            reason: "must be 'monthly', 'quarterly', or 'annually'".into(),
        });
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Core function
// ---------------------------------------------------------------------------

/// Evaluate family office governance and produce a maturity score.
pub fn evaluate_family_governance(
    input: &FamilyGovernanceInput,
) -> CorpFinanceResult<FamilyGovernanceOutput> {
    validate(input)?;

    let ten = dec!(10);

    // Structure dimension (max 30)
    let constitution_pts = if input.has_family_constitution {
        ten
    } else {
        Decimal::ZERO
    };
    let succession_pts = if input.has_succession_plan {
        ten
    } else {
        Decimal::ZERO
    };
    let conflict_pts = if input.has_conflict_resolution {
        ten
    } else {
        Decimal::ZERO
    };
    let structure_score = constitution_pts + succession_pts + conflict_pts;

    // Process dimension (max 30)
    let committee_pts = if input.has_investment_committee {
        ten
    } else {
        Decimal::ZERO
    };
    let meetings_pts = if input.has_regular_meetings {
        ten
    } else {
        Decimal::ZERO
    };
    let freq = input.reporting_frequency.to_lowercase();
    let reporting_pts = if input.has_investment_committee || input.has_regular_meetings {
        ten
    } else {
        Decimal::ZERO
    };
    let process_score = committee_pts + meetings_pts + reporting_pts;

    // Development dimension (max 30)
    let nextgen_pts = if input.has_next_gen_education {
        ten
    } else {
        Decimal::ZERO
    };
    let philanthropy_pts = if input.has_philanthropy_program {
        ten
    } else {
        Decimal::ZERO
    };
    let advisors_pts = if input.has_external_advisors {
        ten
    } else {
        Decimal::ZERO
    };
    let development_score = nextgen_pts + philanthropy_pts + advisors_pts;

    // Base score from booleans (max 90)
    let mut score = structure_score + process_score + development_score;

    // Complexity adjustment
    let mut complexity_score = Decimal::ZERO;
    if input.total_aum > dec!(100_000_000) {
        score += dec!(5);
        complexity_score += dec!(5);
    }
    if input.generations_active > 2 {
        score += dec!(5);
        complexity_score += dec!(5);
    }

    // Reporting frequency bonus
    if freq == "monthly" {
        score += dec!(5);
    } else if freq == "quarterly" {
        score += dec!(2);
    }

    // Cap at 100
    if score > dec!(100) {
        score = dec!(100);
    }

    // Rating
    let governance_rating = if score >= dec!(80) {
        "Institutional".to_string()
    } else if score >= dec!(60) {
        "Structured".to_string()
    } else if score >= dec!(40) {
        "Developing".to_string()
    } else {
        "Informal".to_string()
    };

    // Maturity level: score/20, rounded up, capped at 5
    let level_raw = score / dec!(20);
    // Ceiling: if fractional, round up
    let level_truncated = level_raw.trunc();
    let level_ceil = if level_raw > level_truncated {
        level_truncated + Decimal::ONE
    } else {
        level_truncated
    };
    let maturity_u8 = if level_ceil > dec!(5) {
        5u8
    } else if level_ceil < Decimal::ONE {
        1u8
    } else {
        // Safe conversion -- value is 1-5
        let v: u32 = level_ceil.try_into().unwrap_or(1);
        v as u8
    };

    // Recommendations
    let mut recommendations = Vec::new();
    if !input.has_family_constitution {
        recommendations.push(
            "Develop a written family constitution to codify values, mission, and governance rules"
                .into(),
        );
    }
    if !input.has_succession_plan {
        recommendations
            .push("Create a documented succession plan for leadership transitions".into());
    }
    if !input.has_conflict_resolution {
        recommendations.push(
            "Establish a formal conflict resolution mechanism (e.g. mediation, family council)"
                .into(),
        );
    }
    if !input.has_investment_committee {
        recommendations.push(
            "Form a dedicated investment committee with clear mandate and meeting cadence".into(),
        );
    }
    if !input.has_regular_meetings {
        recommendations
            .push("Institute quarterly family meetings to align on strategy and operations".into());
    }
    if !input.has_next_gen_education {
        recommendations.push(
            "Launch a next-generation education program covering finance, governance, and values"
                .into(),
        );
    }
    if !input.has_external_advisors {
        recommendations.push(
            "Engage external professional advisors for independent oversight and expertise".into(),
        );
    }
    if !input.has_philanthropy_program {
        recommendations.push(
            "Develop a structured philanthropy program to align family purpose and engagement"
                .into(),
        );
    }
    if freq == "annually" {
        recommendations
            .push("Increase reporting frequency to at least quarterly for better oversight".into());
    }

    Ok(FamilyGovernanceOutput {
        governance_score: score,
        governance_rating,
        dimension_scores: GovernanceDimensions {
            structure_score,
            process_score,
            development_score,
            complexity_score,
        },
        recommendations,
        maturity_level: maturity_u8,
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn full_input() -> FamilyGovernanceInput {
        FamilyGovernanceInput {
            family_members: 12,
            generations_active: 3,
            has_family_constitution: true,
            has_investment_committee: true,
            has_succession_plan: true,
            has_conflict_resolution: true,
            has_next_gen_education: true,
            has_external_advisors: true,
            has_regular_meetings: true,
            has_philanthropy_program: true,
            total_aum: dec!(500_000_000),
            num_investment_vehicles: 8,
            reporting_frequency: "monthly".into(),
        }
    }

    fn minimal_input() -> FamilyGovernanceInput {
        FamilyGovernanceInput {
            family_members: 2,
            generations_active: 1,
            has_family_constitution: false,
            has_investment_committee: false,
            has_succession_plan: false,
            has_conflict_resolution: false,
            has_next_gen_education: false,
            has_external_advisors: false,
            has_regular_meetings: false,
            has_philanthropy_program: false,
            total_aum: dec!(5_000_000),
            num_investment_vehicles: 1,
            reporting_frequency: "annually".into(),
        }
    }

    #[test]
    fn test_full_institutional_score() {
        let out = evaluate_family_governance(&full_input()).unwrap();
        // 90 (booleans) + 5 (AUM>100M) + 5 (gen>2) + 5 (monthly) = 100 (capped)
        assert_eq!(out.governance_score, dec!(100));
    }

    #[test]
    fn test_full_institutional_rating() {
        let out = evaluate_family_governance(&full_input()).unwrap();
        assert_eq!(out.governance_rating, "Institutional");
    }

    #[test]
    fn test_full_maturity_level_5() {
        let out = evaluate_family_governance(&full_input()).unwrap();
        assert_eq!(out.maturity_level, 5);
    }

    #[test]
    fn test_minimal_informal() {
        let out = evaluate_family_governance(&minimal_input()).unwrap();
        assert_eq!(out.governance_rating, "Informal");
    }

    #[test]
    fn test_minimal_low_score() {
        let out = evaluate_family_governance(&minimal_input()).unwrap();
        assert_eq!(out.governance_score, Decimal::ZERO);
    }

    #[test]
    fn test_minimal_maturity_level_1() {
        let out = evaluate_family_governance(&minimal_input()).unwrap();
        assert_eq!(out.maturity_level, 1);
    }

    #[test]
    fn test_minimal_many_recommendations() {
        let out = evaluate_family_governance(&minimal_input()).unwrap();
        // 8 missing booleans + 1 for annual reporting = 9
        assert_eq!(out.recommendations.len(), 9);
    }

    #[test]
    fn test_full_no_recommendations() {
        let out = evaluate_family_governance(&full_input()).unwrap();
        assert!(out.recommendations.is_empty());
    }

    #[test]
    fn test_structure_dimension() {
        let mut inp = minimal_input();
        inp.has_family_constitution = true;
        inp.has_succession_plan = true;
        let out = evaluate_family_governance(&inp).unwrap();
        assert_eq!(out.dimension_scores.structure_score, dec!(20));
    }

    #[test]
    fn test_process_dimension() {
        let mut inp = minimal_input();
        inp.has_investment_committee = true;
        inp.has_regular_meetings = true;
        let out = evaluate_family_governance(&inp).unwrap();
        // committee(10) + meetings(10) + reporting(10 because committee is true) = 30
        assert_eq!(out.dimension_scores.process_score, dec!(30));
    }

    #[test]
    fn test_development_dimension() {
        let mut inp = minimal_input();
        inp.has_next_gen_education = true;
        inp.has_philanthropy_program = true;
        inp.has_external_advisors = true;
        let out = evaluate_family_governance(&inp).unwrap();
        assert_eq!(out.dimension_scores.development_score, dec!(30));
    }

    #[test]
    fn test_complexity_aum_bonus() {
        let mut inp = minimal_input();
        inp.total_aum = dec!(200_000_000);
        let out = evaluate_family_governance(&inp).unwrap();
        assert_eq!(out.dimension_scores.complexity_score, dec!(5));
    }

    #[test]
    fn test_complexity_generation_bonus() {
        let mut inp = minimal_input();
        inp.generations_active = 4;
        let out = evaluate_family_governance(&inp).unwrap();
        assert_eq!(out.dimension_scores.complexity_score, dec!(5));
    }

    #[test]
    fn test_quarterly_reporting_bonus() {
        let mut inp = minimal_input();
        inp.reporting_frequency = "quarterly".into();
        let out = evaluate_family_governance(&inp).unwrap();
        assert_eq!(out.governance_score, dec!(2));
    }

    #[test]
    fn test_monthly_reporting_bonus() {
        let mut inp = minimal_input();
        inp.reporting_frequency = "monthly".into();
        let out = evaluate_family_governance(&inp).unwrap();
        assert_eq!(out.governance_score, dec!(5));
    }

    #[test]
    fn test_developing_rating() {
        let mut inp = minimal_input();
        inp.has_family_constitution = true;
        inp.has_investment_committee = true;
        inp.has_succession_plan = true;
        inp.has_regular_meetings = true;
        // 10+10+10+10+10(reporting)=50
        let out = evaluate_family_governance(&inp).unwrap();
        assert_eq!(out.governance_rating, "Developing");
    }

    #[test]
    fn test_structured_rating() {
        let mut inp = minimal_input();
        inp.has_family_constitution = true;
        inp.has_investment_committee = true;
        inp.has_succession_plan = true;
        inp.has_conflict_resolution = true;
        inp.has_regular_meetings = true;
        inp.has_next_gen_education = true;
        // 10*6 + 10(reporting from committee) = 70
        let out = evaluate_family_governance(&inp).unwrap();
        assert_eq!(out.governance_rating, "Structured");
    }

    #[test]
    fn test_invalid_zero_members() {
        let mut inp = minimal_input();
        inp.family_members = 0;
        assert!(evaluate_family_governance(&inp).is_err());
    }

    #[test]
    fn test_invalid_zero_generations() {
        let mut inp = minimal_input();
        inp.generations_active = 0;
        assert!(evaluate_family_governance(&inp).is_err());
    }

    #[test]
    fn test_invalid_reporting_frequency() {
        let mut inp = minimal_input();
        inp.reporting_frequency = "weekly".into();
        assert!(evaluate_family_governance(&inp).is_err());
    }

    #[test]
    fn test_negative_aum() {
        let mut inp = minimal_input();
        inp.total_aum = dec!(-1);
        assert!(evaluate_family_governance(&inp).is_err());
    }

    #[test]
    fn test_score_capped_at_100() {
        let out = evaluate_family_governance(&full_input()).unwrap();
        assert!(out.governance_score <= dec!(100));
    }
}
