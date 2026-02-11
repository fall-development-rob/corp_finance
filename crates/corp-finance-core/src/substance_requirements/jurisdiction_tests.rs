use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

use crate::error::CorpFinanceError;
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SubstanceTestType {
    /// Cayman/BVI: Directed and Managed test
    DirectedAndManaged,
    /// Ireland/UK: Central Management and Control test
    CentralManagementAndControl,
    /// OECD/treaty: Place of Effective Management test
    PlaceOfEffectiveManagement,
    /// Cayman/BVI: ES Act CIGA + D&M + adequate resources
    EconomicSubstanceAct,
    /// Luxembourg: ATAD + TP substance
    ATADSubstance,
    /// Singapore: Section 13X/13U incentive conditions
    TaxIncentiveConditions,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RiskRating {
    Low,
    Medium,
    High,
    Critical,
}

// ---------------------------------------------------------------------------
// Input types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JurisdictionInfo {
    pub name: String,
    pub entity_type: String,
    pub activity: String,
    pub local_staff: u32,
    pub qualified_directors: u32,
    pub total_directors: u32,
    pub has_premises: bool,
    pub premises_dedicated: bool,
    pub board_meetings_local: u32,
    pub board_meetings_total: u32,
    pub annual_expenditure: Decimal,
    pub local_expenditure: Decimal,
    pub ciga_local: bool,
    pub outsourced_ciga: bool,
    /// Annual substance cost for this jurisdiction
    pub annual_substance_cost: Decimal,
    /// Tax savings obtained from this jurisdiction structure
    pub tax_savings: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JurisdictionTestInput {
    pub entity_name: String,
    pub jurisdictions: Vec<JurisdictionInfo>,
    /// If true, compare substance across all jurisdictions
    pub comparison_mode: bool,
    pub parent_jurisdiction: String,
    /// Whether the structure relies on tax treaty benefits
    pub treaty_reliance: bool,
    pub annual_tax_savings: Decimal,
    pub restructuring_budget: Decimal,
}

// ---------------------------------------------------------------------------
// Output types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JurisdictionResult {
    pub jurisdiction: String,
    pub test_type: String,
    pub passed: bool,
    pub score: Decimal,
    pub details: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparisonRow {
    pub jurisdiction: String,
    pub substance_cost: Decimal,
    pub tax_savings: Decimal,
    pub net_benefit: Decimal,
    pub risk_score: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostBenefitSummary {
    pub total_substance_cost: Decimal,
    pub total_tax_savings: Decimal,
    pub net_benefit: Decimal,
    pub payback_ratio: Decimal,
    pub recommendation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JurisdictionTestOutput {
    pub jurisdiction_results: Vec<JurisdictionResult>,
    pub comparison_matrix: Option<Vec<ComparisonRow>>,
    pub overall_risk_rating: RiskRating,
    pub recommended_actions: Vec<String>,
    pub optimal_jurisdiction: Option<String>,
    pub cost_benefit_summary: Option<CostBenefitSummary>,
    pub methodology: String,
    pub assumptions: Vec<String>,
    pub warnings: Vec<String>,
}

// ---------------------------------------------------------------------------
// Test-type determination by jurisdiction
// ---------------------------------------------------------------------------

fn determine_test_type(jurisdiction: &str) -> SubstanceTestType {
    match jurisdiction.to_lowercase().as_str() {
        "cayman" | "cayman islands" | "bvi" | "british virgin islands" | "jersey" | "guernsey" => {
            SubstanceTestType::DirectedAndManaged
        }
        "ireland" | "uk" | "united kingdom" => SubstanceTestType::CentralManagementAndControl,
        "luxembourg" | "netherlands" => SubstanceTestType::ATADSubstance,
        "singapore" => SubstanceTestType::TaxIncentiveConditions,
        "switzerland" => SubstanceTestType::PlaceOfEffectiveManagement,
        _ => SubstanceTestType::PlaceOfEffectiveManagement,
    }
}

fn test_type_label(tt: &SubstanceTestType) -> String {
    match tt {
        SubstanceTestType::DirectedAndManaged => "Directed and Managed".to_string(),
        SubstanceTestType::CentralManagementAndControl => {
            "Central Management and Control".to_string()
        }
        SubstanceTestType::PlaceOfEffectiveManagement => {
            "Place of Effective Management (POEM)".to_string()
        }
        SubstanceTestType::EconomicSubstanceAct => "Economic Substance Act".to_string(),
        SubstanceTestType::ATADSubstance => "ATAD Anti-Avoidance Substance".to_string(),
        SubstanceTestType::TaxIncentiveConditions => "Tax Incentive Conditions".to_string(),
    }
}

// ---------------------------------------------------------------------------
// Jurisdiction-specific scoring
// ---------------------------------------------------------------------------

/// Score a jurisdiction on the Directed and Managed test (Cayman/BVI/Jersey/Guernsey).
/// Returns (score 0-100, details).
fn score_directed_and_managed(j: &JurisdictionInfo) -> (Decimal, Vec<String>) {
    let mut score = Decimal::ZERO;
    let mut details = Vec::new();

    // Board meetings in jurisdiction: up to 35 pts
    if j.board_meetings_total > 0 {
        let ratio = Decimal::from(j.board_meetings_local) / Decimal::from(j.board_meetings_total);
        if ratio >= dec!(0.75) {
            score += dec!(35);
            details.push(format!(
                "Board meeting ratio {:.0}% — excellent (35/35)",
                ratio * dec!(100)
            ));
        } else if ratio >= dec!(0.50) {
            score += dec!(25);
            details.push(format!(
                "Board meeting ratio {:.0}% — adequate (25/35)",
                ratio * dec!(100)
            ));
        } else if ratio > Decimal::ZERO {
            score += dec!(10);
            details.push(format!(
                "Board meeting ratio {:.0}% — insufficient (10/35)",
                ratio * dec!(100)
            ));
        } else {
            details.push("No board meetings in jurisdiction (0/35)".to_string());
        }
    } else {
        details.push("No board meetings recorded (0/35)".to_string());
    }

    // Director residency: up to 25 pts
    if j.total_directors > 0 {
        let ratio = Decimal::from(j.qualified_directors) / Decimal::from(j.total_directors);
        if ratio > dec!(0.5) {
            score += dec!(25);
            details.push(format!(
                "Director residency {:.0}% — majority resident (25/25)",
                ratio * dec!(100)
            ));
        } else if ratio >= dec!(0.5) {
            score += dec!(15);
            details.push("Director residency exactly 50% — borderline (15/25)".to_string());
        } else if j.qualified_directors > 0 {
            score += dec!(8);
            details.push(format!(
                "Director residency {:.0}% — minority resident (8/25)",
                ratio * dec!(100)
            ));
        } else {
            details.push("No resident directors (0/25)".to_string());
        }
    } else {
        details.push("No directors recorded (0/25)".to_string());
    }

    // Adequate employees: up to 20 pts
    match j.local_staff {
        0 => details.push("No local employees (0/20)".to_string()),
        1 => {
            score += dec!(8);
            details.push("1 local employee — minimal (8/20)".to_string());
        }
        2..=4 => {
            score += dec!(15);
            details.push(format!(
                "{} local employees — adequate (15/20)",
                j.local_staff
            ));
        }
        _ => {
            score += dec!(20);
            details.push(format!(
                "{} local employees — strong (20/20)",
                j.local_staff
            ));
        }
    }

    // Adequate premises: up to 10 pts
    if j.has_premises && j.premises_dedicated {
        score += dec!(10);
        details.push("Dedicated premises (10/10)".to_string());
    } else if j.has_premises {
        score += dec!(5);
        details.push("Shared/non-dedicated premises (5/10)".to_string());
    } else {
        details.push("No local premises (0/10)".to_string());
    }

    // CIGA: up to 10 pts
    if j.ciga_local && !j.outsourced_ciga {
        score += dec!(10);
        details.push("CIGA performed locally, not outsourced (10/10)".to_string());
    } else if j.ciga_local {
        score += dec!(5);
        details.push("CIGA partially outsourced (5/10)".to_string());
    } else {
        details.push("CIGA not performed locally (0/10)".to_string());
    }

    (score.min(dec!(100)), details)
}

/// Score Central Management and Control test (Ireland/UK).
fn score_cmc(j: &JurisdictionInfo) -> (Decimal, Vec<String>) {
    let mut score = Decimal::ZERO;
    let mut details = Vec::new();

    // Board meetings: up to 30 pts (where strategic decisions are made)
    if j.board_meetings_total > 0 {
        let ratio = Decimal::from(j.board_meetings_local) / Decimal::from(j.board_meetings_total);
        if ratio >= dec!(0.75) {
            score += dec!(30);
            details.push(format!(
                "Strategic decision-making location: {:.0}% of board meetings local (30/30)",
                ratio * dec!(100)
            ));
        } else if ratio >= dec!(0.50) {
            score += dec!(20);
            details.push(format!(
                "Board meeting ratio {:.0}% — adequate CMC indicator (20/30)",
                ratio * dec!(100)
            ));
        } else if ratio > Decimal::ZERO {
            score += dec!(10);
            details.push(format!(
                "Board meeting ratio {:.0}% — weak CMC indicator (10/30)",
                ratio * dec!(100)
            ));
        } else {
            details.push("No local board meetings — CMC test fails (0/30)".to_string());
        }
    } else {
        details.push("No board meetings recorded (0/30)".to_string());
    }

    // Director composition: up to 30 pts
    if j.total_directors > 0 {
        let ratio = Decimal::from(j.qualified_directors) / Decimal::from(j.total_directors);
        if ratio > dec!(0.5) {
            score += dec!(30);
            details.push(format!(
                "Majority of directors resident — strong CMC (30/30), {:.0}%",
                ratio * dec!(100)
            ));
        } else if ratio >= dec!(0.5) {
            score += dec!(20);
            details.push("50% resident directors — borderline CMC (20/30)".to_string());
        } else if j.qualified_directors > 0 {
            score += dec!(10);
            details.push(format!(
                "Minority resident directors — weak CMC (10/30), {:.0}%",
                ratio * dec!(100)
            ));
        } else {
            details.push("No resident directors — CMC test fails (0/30)".to_string());
        }
    } else {
        details.push("No directors (0/30)".to_string());
    }

    // Local management staff: up to 20 pts
    match j.local_staff {
        0 => details.push("No local staff (0/20)".to_string()),
        1..=2 => {
            score += dec!(10);
            details.push(format!("{} local staff — minimal (10/20)", j.local_staff));
        }
        _ => {
            score += dec!(20);
            details.push(format!("{} local staff — adequate (20/20)", j.local_staff));
        }
    }

    // Premises: up to 10 pts
    if j.has_premises && j.premises_dedicated {
        score += dec!(10);
        details.push("Dedicated premises — supports CMC (10/10)".to_string());
    } else if j.has_premises {
        score += dec!(5);
        details.push("Shared premises (5/10)".to_string());
    } else {
        details.push("No premises (0/10)".to_string());
    }

    // Local expenditure: up to 10 pts
    if !j.annual_expenditure.is_zero() {
        let ratio = j.local_expenditure / j.annual_expenditure;
        if ratio >= dec!(0.50) {
            score += dec!(10);
            details.push(format!(
                "Local expenditure ratio {:.0}% — adequate (10/10)",
                ratio * dec!(100)
            ));
        } else if ratio >= dec!(0.25) {
            score += dec!(5);
            details.push(format!(
                "Local expenditure ratio {:.0}% — low (5/10)",
                ratio * dec!(100)
            ));
        } else {
            details.push(format!(
                "Local expenditure ratio {:.0}% — insufficient (0/10)",
                ratio * dec!(100)
            ));
        }
    }

    (score.min(dec!(100)), details)
}

/// Score Place of Effective Management (POEM) — OECD/treaty-based.
fn score_poem(j: &JurisdictionInfo) -> (Decimal, Vec<String>) {
    let mut score = Decimal::ZERO;
    let mut details = Vec::new();

    // Senior management location / board: up to 35 pts
    if j.board_meetings_total > 0 {
        let ratio = Decimal::from(j.board_meetings_local) / Decimal::from(j.board_meetings_total);
        if ratio >= dec!(0.75) {
            score += dec!(35);
            details.push(format!(
                "Senior management decisions: {:.0}% local board meetings (35/35)",
                ratio * dec!(100)
            ));
        } else if ratio >= dec!(0.50) {
            score += dec!(20);
            details.push(format!(
                "Senior management decisions: {:.0}% local (20/35)",
                ratio * dec!(100)
            ));
        } else {
            score += dec!(5);
            details.push(format!(
                "Senior management decisions: {:.0}% local — weak (5/35)",
                ratio * dec!(100)
            ));
        }
    } else {
        details.push("No board meetings data — cannot assess POEM (0/35)".to_string());
    }

    // Key commercial decisions: director residency as proxy — up to 25 pts
    if j.total_directors > 0 {
        let ratio = Decimal::from(j.qualified_directors) / Decimal::from(j.total_directors);
        if ratio > dec!(0.5) {
            score += dec!(25);
            details.push("Majority directors resident — strong POEM indicator (25/25)".to_string());
        } else if ratio >= dec!(0.5) {
            score += dec!(15);
            details.push("50% directors resident — borderline POEM (15/25)".to_string());
        } else if j.qualified_directors > 0 {
            score += dec!(8);
            details.push("Minority directors resident — weak POEM (8/25)".to_string());
        } else {
            details.push("No resident directors — POEM fails (0/25)".to_string());
        }
    } else {
        details.push("No directors (0/25)".to_string());
    }

    // Business activity location: up to 20 pts
    if j.ciga_local && !j.outsourced_ciga {
        score += dec!(20);
        details.push("Business conducted locally (20/20)".to_string());
    } else if j.ciga_local {
        score += dec!(10);
        details.push("Business partially outsourced (10/20)".to_string());
    } else {
        details.push("Business not conducted locally (0/20)".to_string());
    }

    // Local staff/operations: up to 10 pts
    if j.local_staff >= 3 {
        score += dec!(10);
        details.push(format!("{} local staff (10/10)", j.local_staff));
    } else if j.local_staff > 0 {
        score += dec!(5);
        details.push(format!("{} local staff — minimal (5/10)", j.local_staff));
    } else {
        details.push("No local staff (0/10)".to_string());
    }

    // Premises: up to 10 pts
    if j.has_premises && j.premises_dedicated {
        score += dec!(10);
        details.push("Dedicated premises (10/10)".to_string());
    } else if j.has_premises {
        score += dec!(5);
        details.push("Shared premises (5/10)".to_string());
    } else {
        details.push("No premises (0/10)".to_string());
    }

    (score.min(dec!(100)), details)
}

/// Score ATAD substance (Luxembourg/Netherlands).
fn score_atad(j: &JurisdictionInfo) -> (Decimal, Vec<String>) {
    let mut score = Decimal::ZERO;
    let mut details = Vec::new();

    // Director residency: up to 30 pts (majority required)
    if j.total_directors > 0 {
        let ratio = Decimal::from(j.qualified_directors) / Decimal::from(j.total_directors);
        if ratio > dec!(0.5) {
            score += dec!(30);
            details.push(format!(
                "Majority of directors locally resident ({:.0}%) — ATAD compliant (30/30)",
                ratio * dec!(100)
            ));
        } else if ratio >= dec!(0.5) {
            score += dec!(20);
            details.push("50% directors local — borderline ATAD (20/30)".to_string());
        } else if j.qualified_directors > 0 {
            score += dec!(10);
            details.push(format!(
                "Minority directors local ({:.0}%) — ATAD non-compliant (10/30)",
                ratio * dec!(100)
            ));
        } else {
            details.push("No local directors — ATAD fails (0/30)".to_string());
        }
    } else {
        details.push("No directors (0/30)".to_string());
    }

    // Local employees: up to 25 pts
    match j.local_staff {
        0 => details.push("No local employees (0/25)".to_string()),
        1 => {
            score += dec!(10);
            details.push("1 local employee — minimal (10/25)".to_string());
        }
        2..=4 => {
            score += dec!(18);
            details.push(format!(
                "{} local employees — adequate (18/25)",
                j.local_staff
            ));
        }
        _ => {
            score += dec!(25);
            details.push(format!(
                "{} local employees — strong (25/25)",
                j.local_staff
            ));
        }
    }

    // Decision-making: up to 20 pts
    if j.board_meetings_total > 0 {
        let ratio = Decimal::from(j.board_meetings_local) / Decimal::from(j.board_meetings_total);
        if ratio >= dec!(0.75) {
            score += dec!(20);
            details.push(format!(
                "Decision-making: {:.0}% local board meetings (20/20)",
                ratio * dec!(100)
            ));
        } else if ratio >= dec!(0.50) {
            score += dec!(12);
            details.push(format!(
                "Decision-making: {:.0}% local (12/20)",
                ratio * dec!(100)
            ));
        } else {
            score += dec!(5);
            details.push(format!(
                "Decision-making: {:.0}% local — weak (5/20)",
                ratio * dec!(100)
            ));
        }
    }

    // Premises: up to 15 pts
    if j.has_premises && j.premises_dedicated {
        score += dec!(15);
        details.push("Dedicated office premises (15/15)".to_string());
    } else if j.has_premises {
        score += dec!(8);
        details.push("Shared premises (8/15)".to_string());
    } else {
        details.push("No local premises (0/15)".to_string());
    }

    // Expenditure: up to 10 pts
    if !j.annual_expenditure.is_zero() {
        let ratio = j.local_expenditure / j.annual_expenditure;
        if ratio >= dec!(0.50) {
            score += dec!(10);
            details.push(format!(
                "Local expenditure {:.0}% (10/10)",
                ratio * dec!(100)
            ));
        } else if ratio >= dec!(0.25) {
            score += dec!(5);
            details.push(format!(
                "Local expenditure {:.0}% (5/10)",
                ratio * dec!(100)
            ));
        } else {
            details.push(format!(
                "Local expenditure {:.0}% — low (0/10)",
                ratio * dec!(100)
            ));
        }
    }

    (score.min(dec!(100)), details)
}

/// Score Singapore tax incentive conditions (Section 13X/13U).
fn score_singapore_incentive(j: &JurisdictionInfo) -> (Decimal, Vec<String>) {
    let mut score = Decimal::ZERO;
    let mut details = Vec::new();

    // Fund manager in Singapore: up to 30 pts (proxied by local staff / directors)
    if j.local_staff >= 3 && j.qualified_directors > 0 {
        score += dec!(30);
        details.push("Fund manager presence: adequate staff and directors (30/30)".to_string());
    } else if j.local_staff >= 1 && j.qualified_directors > 0 {
        score += dec!(20);
        details.push("Fund manager presence: minimal staff (20/30)".to_string());
    } else if j.qualified_directors > 0 {
        score += dec!(10);
        details.push("Director presence but no investment staff (10/30)".to_string());
    } else {
        details.push("No fund manager presence in Singapore (0/30)".to_string());
    }

    // Investment professionals: up to 25 pts
    match j.local_staff {
        0 => details.push("No investment professionals (0/25)".to_string()),
        1..=2 => {
            score += dec!(15);
            details.push(format!(
                "{} investment professionals — minimal (15/25)",
                j.local_staff
            ));
        }
        _ => {
            score += dec!(25);
            details.push(format!(
                "{} investment professionals — adequate (25/25)",
                j.local_staff
            ));
        }
    }

    // Board meetings: up to 20 pts
    if j.board_meetings_total > 0 {
        let ratio = Decimal::from(j.board_meetings_local) / Decimal::from(j.board_meetings_total);
        if ratio >= dec!(0.50) {
            score += dec!(20);
            details.push(format!(
                "Board meeting ratio {:.0}% — compliant (20/20)",
                ratio * dec!(100)
            ));
        } else if ratio > Decimal::ZERO {
            score += dec!(10);
            details.push(format!(
                "Board meeting ratio {:.0}% — low (10/20)",
                ratio * dec!(100)
            ));
        } else {
            details.push("No local board meetings (0/20)".to_string());
        }
    }

    // Office and admin: up to 15 pts
    if j.has_premises && j.premises_dedicated {
        score += dec!(15);
        details.push("Dedicated office (15/15)".to_string());
    } else if j.has_premises {
        score += dec!(8);
        details.push("Shared/serviced office (8/15)".to_string());
    } else {
        details.push("No office (0/15)".to_string());
    }

    // Expenditure: up to 10 pts
    if !j.annual_expenditure.is_zero() {
        let ratio = j.local_expenditure / j.annual_expenditure;
        if ratio >= dec!(0.50) {
            score += dec!(10);
            details.push(format!(
                "Local expenditure {:.0}% (10/10)",
                ratio * dec!(100)
            ));
        } else if ratio >= dec!(0.25) {
            score += dec!(5);
            details.push(format!(
                "Local expenditure {:.0}% (5/10)",
                ratio * dec!(100)
            ));
        }
    }

    (score.min(dec!(100)), details)
}

// ---------------------------------------------------------------------------
// Risk scoring for a single jurisdiction
// ---------------------------------------------------------------------------

fn jurisdiction_risk_score(score: Decimal, treaty_reliance: bool) -> Decimal {
    // Base risk from substance score: inverse relationship
    let base_risk = if score < dec!(30) {
        dec!(0.80)
    } else if score < dec!(50) {
        dec!(0.60)
    } else if score < dec!(70) {
        dec!(0.30)
    } else if score < dec!(85) {
        dec!(0.15)
    } else {
        dec!(0.05)
    };

    // Treaty reliance amplifies risk
    if treaty_reliance {
        (base_risk * dec!(1.3)).min(Decimal::ONE)
    } else {
        base_risk
    }
}

// ---------------------------------------------------------------------------
// Overall risk rating
// ---------------------------------------------------------------------------

fn overall_risk_from_scores(scores: &[Decimal]) -> RiskRating {
    if scores.is_empty() {
        return RiskRating::Critical;
    }
    let sum: Decimal = scores.iter().copied().sum();
    let avg = sum / Decimal::from(scores.len() as u32);

    if avg >= dec!(75) {
        RiskRating::Low
    } else if avg >= dec!(50) {
        RiskRating::Medium
    } else if avg >= dec!(30) {
        RiskRating::High
    } else {
        RiskRating::Critical
    }
}

// ---------------------------------------------------------------------------
// Recommended actions
// ---------------------------------------------------------------------------

fn recommended_actions(results: &[JurisdictionResult], treaty_reliance: bool) -> Vec<String> {
    let mut actions = Vec::new();

    for r in results {
        if !r.passed {
            actions.push(format!(
                "{}: Failed {} test (score {}) — remediation required",
                r.jurisdiction, r.test_type, r.score
            ));
        }
        if r.score < dec!(50) {
            actions.push(format!(
                "{}: Substance score below 50 — increase local presence",
                r.jurisdiction
            ));
        }
    }

    if treaty_reliance {
        let any_failed = results.iter().any(|r| !r.passed);
        if any_failed {
            actions.push(
                "Treaty reliance at risk — consider pre-filing ruling or advance pricing agreement"
                    .to_string(),
            );
        }
    }

    if actions.is_empty() {
        actions
            .push("All jurisdiction tests passed — maintain current substance levels".to_string());
    }

    actions
}

// ---------------------------------------------------------------------------
// Comparison and cost-benefit
// ---------------------------------------------------------------------------

fn build_comparison_matrix(
    jurisdictions: &[JurisdictionInfo],
    results: &[JurisdictionResult],
    treaty_reliance: bool,
) -> Vec<ComparisonRow> {
    jurisdictions
        .iter()
        .zip(results.iter())
        .map(|(j, r)| {
            let risk = jurisdiction_risk_score(r.score, treaty_reliance);
            ComparisonRow {
                jurisdiction: j.name.clone(),
                substance_cost: j.annual_substance_cost,
                tax_savings: j.tax_savings,
                net_benefit: j.tax_savings - j.annual_substance_cost,
                risk_score: risk,
            }
        })
        .collect()
}

fn find_optimal_jurisdiction(matrix: &[ComparisonRow]) -> Option<String> {
    if matrix.is_empty() {
        return None;
    }

    // Optimal = highest net_benefit with risk_score < 0.5
    matrix
        .iter()
        .filter(|r| r.risk_score < dec!(0.5))
        .max_by(|a, b| a.net_benefit.cmp(&b.net_benefit))
        .or_else(|| {
            // If all are risky, pick lowest risk
            matrix.iter().min_by(|a, b| a.risk_score.cmp(&b.risk_score))
        })
        .map(|r| r.jurisdiction.clone())
}

fn build_cost_benefit_summary(matrix: &[ComparisonRow]) -> CostBenefitSummary {
    let total_substance_cost: Decimal = matrix.iter().map(|r| r.substance_cost).sum();
    let total_tax_savings: Decimal = matrix.iter().map(|r| r.tax_savings).sum();
    let net_benefit = total_tax_savings - total_substance_cost;
    let payback_ratio = if total_substance_cost > Decimal::ZERO {
        total_tax_savings / total_substance_cost
    } else {
        Decimal::ZERO
    };

    let recommendation = if payback_ratio >= dec!(3) {
        "Excellent cost-benefit — substance costs well justified by tax savings".to_string()
    } else if payback_ratio >= dec!(1.5) {
        "Good cost-benefit — tax savings comfortably exceed substance costs".to_string()
    } else if payback_ratio >= dec!(1) {
        "Marginal cost-benefit — tax savings barely cover substance costs".to_string()
    } else if payback_ratio > Decimal::ZERO {
        "Negative cost-benefit — substance costs exceed tax savings; review structure".to_string()
    } else {
        "No tax savings identified — substance costs are a pure expense".to_string()
    };

    CostBenefitSummary {
        total_substance_cost,
        total_tax_savings,
        net_benefit,
        payback_ratio,
        recommendation,
    }
}

// ---------------------------------------------------------------------------
// Warnings
// ---------------------------------------------------------------------------

fn generate_warnings(input: &JurisdictionTestInput, results: &[JurisdictionResult]) -> Vec<String> {
    let mut warnings = Vec::new();

    for r in results {
        if r.score < dec!(30) {
            warnings.push(format!(
                "{}: Critically low substance score ({}) — high risk of challenge",
                r.jurisdiction, r.score
            ));
        }
    }

    if input.treaty_reliance {
        let any_below_50 = results.iter().any(|r| r.score < dec!(50));
        if any_below_50 {
            warnings.push(
                "Treaty reliance with low substance — risk of treaty benefit denial under MLI PPT"
                    .to_string(),
            );
        }
    }

    if input.jurisdictions.len() > 1 {
        let all_failed = results.iter().all(|r| !r.passed);
        if all_failed {
            warnings.push(
                "All jurisdictions failed substance tests — structure redesign recommended"
                    .to_string(),
            );
        }
    }

    if input.restructuring_budget > Decimal::ZERO {
        let total_cost: Decimal = input
            .jurisdictions
            .iter()
            .map(|j| j.annual_substance_cost)
            .sum();
        if total_cost > input.restructuring_budget {
            warnings.push(format!(
                "Total substance cost ({}) exceeds restructuring budget ({})",
                total_cost, input.restructuring_budget
            ));
        }
    }

    warnings
}

// ---------------------------------------------------------------------------
// Public function
// ---------------------------------------------------------------------------

/// Run jurisdiction-specific substance tests for one or more jurisdictions.
///
/// In comparison mode, produces a cost-benefit matrix and identifies the
/// optimal jurisdiction based on net tax benefit and risk score.
pub fn run_jurisdiction_substance_test(
    input: &JurisdictionTestInput,
) -> CorpFinanceResult<JurisdictionTestOutput> {
    // Input validation
    if input.entity_name.is_empty() {
        return Err(CorpFinanceError::InvalidInput {
            field: "entity_name".to_string(),
            reason: "Entity name must not be empty".to_string(),
        });
    }
    if input.jurisdictions.is_empty() {
        return Err(CorpFinanceError::InvalidInput {
            field: "jurisdictions".to_string(),
            reason: "At least one jurisdiction must be provided".to_string(),
        });
    }
    if input.parent_jurisdiction.is_empty() {
        return Err(CorpFinanceError::InvalidInput {
            field: "parent_jurisdiction".to_string(),
            reason: "Parent jurisdiction must not be empty".to_string(),
        });
    }

    // Validate each jurisdiction
    for (i, j) in input.jurisdictions.iter().enumerate() {
        if j.name.is_empty() {
            return Err(CorpFinanceError::InvalidInput {
                field: format!("jurisdictions[{}].name", i),
                reason: "Jurisdiction name must not be empty".to_string(),
            });
        }
        if j.qualified_directors > j.total_directors {
            return Err(CorpFinanceError::InvalidInput {
                field: format!("jurisdictions[{}].qualified_directors", i),
                reason: "Qualified directors cannot exceed total directors".to_string(),
            });
        }
        if j.board_meetings_local > j.board_meetings_total {
            return Err(CorpFinanceError::InvalidInput {
                field: format!("jurisdictions[{}].board_meetings_local", i),
                reason: "Local board meetings cannot exceed total board meetings".to_string(),
            });
        }
        if !j.annual_expenditure.is_zero() && j.local_expenditure > j.annual_expenditure {
            return Err(CorpFinanceError::InvalidInput {
                field: format!("jurisdictions[{}].local_expenditure", i),
                reason: "Local expenditure cannot exceed annual expenditure".to_string(),
            });
        }
        if j.annual_expenditure < Decimal::ZERO {
            return Err(CorpFinanceError::InvalidInput {
                field: format!("jurisdictions[{}].annual_expenditure", i),
                reason: "Annual expenditure must not be negative".to_string(),
            });
        }
    }

    if input.annual_tax_savings < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "annual_tax_savings".to_string(),
            reason: "Annual tax savings must not be negative".to_string(),
        });
    }

    // Score each jurisdiction
    let mut results: Vec<JurisdictionResult> = Vec::new();
    let mut scores: Vec<Decimal> = Vec::new();

    for j in &input.jurisdictions {
        let test_type = determine_test_type(&j.name);
        let (score, details) = match test_type {
            SubstanceTestType::DirectedAndManaged => score_directed_and_managed(j),
            SubstanceTestType::CentralManagementAndControl => score_cmc(j),
            SubstanceTestType::PlaceOfEffectiveManagement => score_poem(j),
            SubstanceTestType::ATADSubstance => score_atad(j),
            SubstanceTestType::TaxIncentiveConditions => score_singapore_incentive(j),
            SubstanceTestType::EconomicSubstanceAct => score_directed_and_managed(j),
        };

        let passed = score >= dec!(50);
        scores.push(score);

        results.push(JurisdictionResult {
            jurisdiction: j.name.clone(),
            test_type: test_type_label(&test_type),
            passed,
            score,
            details,
        });
    }

    let overall_risk = overall_risk_from_scores(&scores);
    let actions = recommended_actions(&results, input.treaty_reliance);
    let warnings = generate_warnings(input, &results);

    // Comparison mode outputs
    let (comparison_matrix, optimal_jurisdiction, cost_benefit_summary) = if input.comparison_mode
        && input.jurisdictions.len() > 1
    {
        let matrix = build_comparison_matrix(&input.jurisdictions, &results, input.treaty_reliance);
        let optimal = find_optimal_jurisdiction(&matrix);
        let cbs = build_cost_benefit_summary(&matrix);
        (Some(matrix), optimal, Some(cbs))
    } else {
        (None, None, None)
    };

    let assumptions = vec![
        "Substance test type determined by jurisdiction (D&M for offshore, CMC for Ireland/UK, POEM for OECD, ATAD for EU)"
            .to_string(),
        "Pass threshold set at score >= 50 out of 100".to_string(),
        "Optimal jurisdiction selection prioritizes net benefit with risk score < 0.5".to_string(),
        "Treaty reliance amplifies risk scores by 30%".to_string(),
        "Cost-benefit analysis uses annual substance costs and tax savings as provided".to_string(),
    ];

    Ok(JurisdictionTestOutput {
        jurisdiction_results: results,
        comparison_matrix,
        overall_risk_rating: overall_risk,
        recommended_actions: actions,
        optimal_jurisdiction,
        cost_benefit_summary,
        methodology: "Jurisdiction-specific substance testing: Directed & Managed (Cayman/BVI), \
             Central Management & Control (Ireland/UK), POEM (OECD/treaty), \
             ATAD Anti-Avoidance (Luxembourg/Netherlands), \
             Tax Incentive Conditions (Singapore)"
            .to_string(),
        assumptions,
        warnings,
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn base_jurisdiction() -> JurisdictionInfo {
        JurisdictionInfo {
            name: "Cayman".to_string(),
            entity_type: "HoldingCompany".to_string(),
            activity: "Holding".to_string(),
            local_staff: 3,
            qualified_directors: 2,
            total_directors: 3,
            has_premises: true,
            premises_dedicated: true,
            board_meetings_local: 4,
            board_meetings_total: 4,
            annual_expenditure: dec!(500_000),
            local_expenditure: dec!(400_000),
            ciga_local: true,
            outsourced_ciga: false,
            annual_substance_cost: dec!(300_000),
            tax_savings: dec!(1_000_000),
        }
    }

    fn base_input() -> JurisdictionTestInput {
        JurisdictionTestInput {
            entity_name: "TestCo Holdings".to_string(),
            jurisdictions: vec![base_jurisdiction()],
            comparison_mode: false,
            parent_jurisdiction: "UK".to_string(),
            treaty_reliance: false,
            annual_tax_savings: dec!(1_000_000),
            restructuring_budget: dec!(500_000),
        }
    }

    fn multi_jurisdiction_input() -> JurisdictionTestInput {
        let mut cy = base_jurisdiction();
        cy.name = "Cayman".to_string();
        cy.annual_substance_cost = dec!(200_000);
        cy.tax_savings = dec!(800_000);

        let mut lux = base_jurisdiction();
        lux.name = "Luxembourg".to_string();
        lux.annual_substance_cost = dec!(400_000);
        lux.tax_savings = dec!(1_200_000);

        let mut ie = base_jurisdiction();
        ie.name = "Ireland".to_string();
        ie.annual_substance_cost = dec!(300_000);
        ie.tax_savings = dec!(600_000);

        JurisdictionTestInput {
            entity_name: "MultiCo".to_string(),
            jurisdictions: vec![cy, lux, ie],
            comparison_mode: true,
            parent_jurisdiction: "UK".to_string(),
            treaty_reliance: true,
            annual_tax_savings: dec!(2_600_000),
            restructuring_budget: dec!(1_000_000),
        }
    }

    // ------ Validation tests ------

    #[test]
    fn test_empty_entity_name_rejected() {
        let mut input = base_input();
        input.entity_name = "".to_string();
        let result = run_jurisdiction_substance_test(&input);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("entity_name"));
    }

    #[test]
    fn test_empty_jurisdictions_rejected() {
        let mut input = base_input();
        input.jurisdictions = vec![];
        let result = run_jurisdiction_substance_test(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_parent_jurisdiction_rejected() {
        let mut input = base_input();
        input.parent_jurisdiction = "".to_string();
        let result = run_jurisdiction_substance_test(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_jurisdiction_name_rejected() {
        let mut input = base_input();
        input.jurisdictions[0].name = "".to_string();
        let result = run_jurisdiction_substance_test(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_qualified_directors_exceed_total_rejected() {
        let mut input = base_input();
        input.jurisdictions[0].qualified_directors = 5;
        input.jurisdictions[0].total_directors = 3;
        let result = run_jurisdiction_substance_test(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_local_meetings_exceed_total_rejected() {
        let mut input = base_input();
        input.jurisdictions[0].board_meetings_local = 6;
        input.jurisdictions[0].board_meetings_total = 4;
        let result = run_jurisdiction_substance_test(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_local_expenditure_exceeds_total_rejected() {
        let mut input = base_input();
        input.jurisdictions[0].local_expenditure = dec!(600_000);
        input.jurisdictions[0].annual_expenditure = dec!(500_000);
        let result = run_jurisdiction_substance_test(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_negative_expenditure_rejected() {
        let mut input = base_input();
        input.jurisdictions[0].annual_expenditure = dec!(-100);
        input.jurisdictions[0].local_expenditure = dec!(-200);
        let result = run_jurisdiction_substance_test(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_negative_tax_savings_rejected() {
        let mut input = base_input();
        input.annual_tax_savings = dec!(-100);
        let result = run_jurisdiction_substance_test(&input);
        assert!(result.is_err());
    }

    // ------ Single jurisdiction tests — Cayman (D&M) ------

    #[test]
    fn test_cayman_full_substance_passes() {
        let input = base_input();
        let result = run_jurisdiction_substance_test(&input).unwrap();
        assert_eq!(result.jurisdiction_results.len(), 1);
        assert!(result.jurisdiction_results[0].passed);
        assert!(result.jurisdiction_results[0].score >= dec!(80));
    }

    #[test]
    fn test_cayman_test_type_directed_and_managed() {
        let input = base_input();
        let result = run_jurisdiction_substance_test(&input).unwrap();
        assert_eq!(
            result.jurisdiction_results[0].test_type,
            "Directed and Managed"
        );
    }

    #[test]
    fn test_cayman_no_substance_fails() {
        let mut input = base_input();
        let j = &mut input.jurisdictions[0];
        j.local_staff = 0;
        j.qualified_directors = 0;
        j.total_directors = 2;
        j.has_premises = false;
        j.premises_dedicated = false;
        j.board_meetings_local = 0;
        j.board_meetings_total = 4;
        j.ciga_local = false;
        j.outsourced_ciga = false;
        let result = run_jurisdiction_substance_test(&input).unwrap();
        assert!(!result.jurisdiction_results[0].passed);
        assert!(result.jurisdiction_results[0].score < dec!(50));
    }

    #[test]
    fn test_cayman_partial_substance_borderline() {
        let mut input = base_input();
        let j = &mut input.jurisdictions[0];
        j.local_staff = 1;
        j.qualified_directors = 1;
        j.total_directors = 3;
        j.has_premises = true;
        j.premises_dedicated = false;
        j.board_meetings_local = 2;
        j.board_meetings_total = 4;
        j.ciga_local = true;
        j.outsourced_ciga = true;
        let result = run_jurisdiction_substance_test(&input).unwrap();
        // Should be around 50 — borderline
        let score = result.jurisdiction_results[0].score;
        assert!(score >= dec!(30) && score <= dec!(70));
    }

    #[test]
    fn test_cayman_details_non_empty() {
        let input = base_input();
        let result = run_jurisdiction_substance_test(&input).unwrap();
        assert!(!result.jurisdiction_results[0].details.is_empty());
    }

    // ------ BVI (D&M) ------

    #[test]
    fn test_bvi_uses_directed_and_managed() {
        let mut input = base_input();
        input.jurisdictions[0].name = "BVI".to_string();
        let result = run_jurisdiction_substance_test(&input).unwrap();
        assert_eq!(
            result.jurisdiction_results[0].test_type,
            "Directed and Managed"
        );
    }

    #[test]
    fn test_bvi_full_substance_passes() {
        let mut input = base_input();
        input.jurisdictions[0].name = "BVI".to_string();
        let result = run_jurisdiction_substance_test(&input).unwrap();
        assert!(result.jurisdiction_results[0].passed);
    }

    // ------ Ireland (CMC) ------

    #[test]
    fn test_ireland_uses_cmc() {
        let mut input = base_input();
        input.jurisdictions[0].name = "Ireland".to_string();
        let result = run_jurisdiction_substance_test(&input).unwrap();
        assert_eq!(
            result.jurisdiction_results[0].test_type,
            "Central Management and Control"
        );
    }

    #[test]
    fn test_ireland_full_substance_passes() {
        let mut input = base_input();
        input.jurisdictions[0].name = "Ireland".to_string();
        let result = run_jurisdiction_substance_test(&input).unwrap();
        assert!(result.jurisdiction_results[0].passed);
        assert!(result.jurisdiction_results[0].score >= dec!(80));
    }

    #[test]
    fn test_ireland_no_substance_fails() {
        let mut input = base_input();
        let j = &mut input.jurisdictions[0];
        j.name = "Ireland".to_string();
        j.local_staff = 0;
        j.qualified_directors = 0;
        j.total_directors = 3;
        j.has_premises = false;
        j.board_meetings_local = 0;
        j.board_meetings_total = 4;
        j.local_expenditure = Decimal::ZERO;
        let result = run_jurisdiction_substance_test(&input).unwrap();
        assert!(!result.jurisdiction_results[0].passed);
    }

    // ------ Luxembourg (ATAD) ------

    #[test]
    fn test_luxembourg_uses_atad() {
        let mut input = base_input();
        input.jurisdictions[0].name = "Luxembourg".to_string();
        let result = run_jurisdiction_substance_test(&input).unwrap();
        assert!(result.jurisdiction_results[0].test_type.contains("ATAD"));
    }

    #[test]
    fn test_luxembourg_full_substance_passes() {
        let mut input = base_input();
        input.jurisdictions[0].name = "Luxembourg".to_string();
        let result = run_jurisdiction_substance_test(&input).unwrap();
        assert!(result.jurisdiction_results[0].passed);
    }

    #[test]
    fn test_luxembourg_no_directors_fails() {
        let mut input = base_input();
        let j = &mut input.jurisdictions[0];
        j.name = "Luxembourg".to_string();
        j.qualified_directors = 0;
        j.total_directors = 0;
        j.local_staff = 0;
        j.has_premises = false;
        j.board_meetings_local = 0;
        j.board_meetings_total = 0;
        j.local_expenditure = Decimal::ZERO;
        let result = run_jurisdiction_substance_test(&input).unwrap();
        assert!(!result.jurisdiction_results[0].passed);
    }

    // ------ Netherlands (ATAD) ------

    #[test]
    fn test_netherlands_uses_atad() {
        let mut input = base_input();
        input.jurisdictions[0].name = "Netherlands".to_string();
        let result = run_jurisdiction_substance_test(&input).unwrap();
        assert!(result.jurisdiction_results[0].test_type.contains("ATAD"));
    }

    // ------ Singapore (Tax Incentive) ------

    #[test]
    fn test_singapore_uses_incentive_test() {
        let mut input = base_input();
        input.jurisdictions[0].name = "Singapore".to_string();
        let result = run_jurisdiction_substance_test(&input).unwrap();
        assert!(result.jurisdiction_results[0]
            .test_type
            .contains("Tax Incentive"));
    }

    #[test]
    fn test_singapore_full_substance_passes() {
        let mut input = base_input();
        input.jurisdictions[0].name = "Singapore".to_string();
        let result = run_jurisdiction_substance_test(&input).unwrap();
        assert!(result.jurisdiction_results[0].passed);
    }

    #[test]
    fn test_singapore_no_staff_fails() {
        let mut input = base_input();
        let j = &mut input.jurisdictions[0];
        j.name = "Singapore".to_string();
        j.local_staff = 0;
        j.qualified_directors = 0;
        j.total_directors = 2;
        j.has_premises = false;
        j.board_meetings_local = 0;
        j.board_meetings_total = 4;
        let result = run_jurisdiction_substance_test(&input).unwrap();
        assert!(!result.jurisdiction_results[0].passed);
    }

    // ------ Switzerland (POEM) ------

    #[test]
    fn test_switzerland_uses_poem() {
        let mut input = base_input();
        input.jurisdictions[0].name = "Switzerland".to_string();
        let result = run_jurisdiction_substance_test(&input).unwrap();
        assert!(result.jurisdiction_results[0].test_type.contains("POEM"));
    }

    #[test]
    fn test_switzerland_full_substance_passes() {
        let mut input = base_input();
        input.jurisdictions[0].name = "Switzerland".to_string();
        let result = run_jurisdiction_substance_test(&input).unwrap();
        assert!(result.jurisdiction_results[0].passed);
    }

    // ------ Jersey (D&M) ------

    #[test]
    fn test_jersey_uses_directed_and_managed() {
        let mut input = base_input();
        input.jurisdictions[0].name = "Jersey".to_string();
        let result = run_jurisdiction_substance_test(&input).unwrap();
        assert_eq!(
            result.jurisdiction_results[0].test_type,
            "Directed and Managed"
        );
    }

    // ------ Guernsey (D&M) ------

    #[test]
    fn test_guernsey_uses_directed_and_managed() {
        let mut input = base_input();
        input.jurisdictions[0].name = "Guernsey".to_string();
        let result = run_jurisdiction_substance_test(&input).unwrap();
        assert_eq!(
            result.jurisdiction_results[0].test_type,
            "Directed and Managed"
        );
    }

    // ------ UK (CMC) ------

    #[test]
    fn test_uk_uses_cmc() {
        let mut input = base_input();
        input.jurisdictions[0].name = "UK".to_string();
        let result = run_jurisdiction_substance_test(&input).unwrap();
        assert_eq!(
            result.jurisdiction_results[0].test_type,
            "Central Management and Control"
        );
    }

    // ------ Unknown jurisdiction defaults to POEM ------

    #[test]
    fn test_unknown_jurisdiction_uses_poem() {
        let mut input = base_input();
        input.jurisdictions[0].name = "Bermuda".to_string();
        let result = run_jurisdiction_substance_test(&input).unwrap();
        assert!(result.jurisdiction_results[0].test_type.contains("POEM"));
    }

    // ------ Overall risk rating ------

    #[test]
    fn test_high_scores_low_risk() {
        let input = base_input();
        let result = run_jurisdiction_substance_test(&input).unwrap();
        assert_eq!(result.overall_risk_rating, RiskRating::Low);
    }

    #[test]
    fn test_zero_scores_critical_risk() {
        let mut input = base_input();
        let j = &mut input.jurisdictions[0];
        j.local_staff = 0;
        j.qualified_directors = 0;
        j.total_directors = 0;
        j.has_premises = false;
        j.premises_dedicated = false;
        j.board_meetings_local = 0;
        j.board_meetings_total = 0;
        j.ciga_local = false;
        j.outsourced_ciga = false;
        j.annual_expenditure = Decimal::ZERO;
        j.local_expenditure = Decimal::ZERO;
        let result = run_jurisdiction_substance_test(&input).unwrap();
        assert_eq!(result.overall_risk_rating, RiskRating::Critical);
    }

    #[test]
    fn test_medium_scores_medium_or_low_risk() {
        let mut input = base_input();
        let j = &mut input.jurisdictions[0];
        j.local_staff = 2;
        j.qualified_directors = 1;
        j.total_directors = 3;
        j.has_premises = true;
        j.premises_dedicated = false;
        j.board_meetings_local = 2;
        j.board_meetings_total = 4;
        let result = run_jurisdiction_substance_test(&input).unwrap();
        assert!(
            result.overall_risk_rating == RiskRating::Low
                || result.overall_risk_rating == RiskRating::Medium
        );
    }

    // ------ Recommended actions ------

    #[test]
    fn test_all_passed_positive_action() {
        let input = base_input();
        let result = run_jurisdiction_substance_test(&input).unwrap();
        assert!(result
            .recommended_actions
            .iter()
            .any(|a| a.contains("passed") || a.contains("maintain")));
    }

    #[test]
    fn test_failed_generates_remediation_actions() {
        let mut input = base_input();
        let j = &mut input.jurisdictions[0];
        j.local_staff = 0;
        j.qualified_directors = 0;
        j.total_directors = 2;
        j.has_premises = false;
        j.board_meetings_local = 0;
        j.board_meetings_total = 4;
        j.ciga_local = false;
        let result = run_jurisdiction_substance_test(&input).unwrap();
        assert!(result
            .recommended_actions
            .iter()
            .any(|a| a.contains("remediation") || a.contains("increase")));
    }

    #[test]
    fn test_treaty_reliance_warning_action() {
        let mut input = base_input();
        input.treaty_reliance = true;
        let j = &mut input.jurisdictions[0];
        j.local_staff = 0;
        j.qualified_directors = 0;
        j.total_directors = 2;
        j.has_premises = false;
        j.board_meetings_local = 0;
        j.board_meetings_total = 4;
        j.ciga_local = false;
        let result = run_jurisdiction_substance_test(&input).unwrap();
        assert!(result
            .recommended_actions
            .iter()
            .any(|a| a.contains("Treaty") || a.contains("treaty")));
    }

    // ------ Comparison mode tests ------

    #[test]
    fn test_comparison_mode_produces_matrix() {
        let input = multi_jurisdiction_input();
        let result = run_jurisdiction_substance_test(&input).unwrap();
        assert!(result.comparison_matrix.is_some());
        let matrix = result.comparison_matrix.unwrap();
        assert_eq!(matrix.len(), 3);
    }

    #[test]
    fn test_comparison_mode_produces_optimal_jurisdiction() {
        let input = multi_jurisdiction_input();
        let result = run_jurisdiction_substance_test(&input).unwrap();
        assert!(result.optimal_jurisdiction.is_some());
    }

    #[test]
    fn test_comparison_mode_produces_cost_benefit_summary() {
        let input = multi_jurisdiction_input();
        let result = run_jurisdiction_substance_test(&input).unwrap();
        assert!(result.cost_benefit_summary.is_some());
        let cbs = result.cost_benefit_summary.unwrap();
        assert_eq!(
            cbs.net_benefit,
            cbs.total_tax_savings - cbs.total_substance_cost
        );
    }

    #[test]
    fn test_comparison_matrix_net_benefit_calculation() {
        let input = multi_jurisdiction_input();
        let result = run_jurisdiction_substance_test(&input).unwrap();
        let matrix = result.comparison_matrix.unwrap();
        for row in &matrix {
            assert_eq!(row.net_benefit, row.tax_savings - row.substance_cost);
        }
    }

    #[test]
    fn test_comparison_no_comparison_in_single_mode() {
        let input = base_input(); // comparison_mode = false
        let result = run_jurisdiction_substance_test(&input).unwrap();
        assert!(result.comparison_matrix.is_none());
        assert!(result.optimal_jurisdiction.is_none());
        assert!(result.cost_benefit_summary.is_none());
    }

    #[test]
    fn test_optimal_jurisdiction_is_best_net_benefit() {
        let input = multi_jurisdiction_input();
        let result = run_jurisdiction_substance_test(&input).unwrap();
        let matrix = result.comparison_matrix.as_ref().unwrap();
        let optimal = result.optimal_jurisdiction.as_ref().unwrap();

        // The optimal should be the one with highest net benefit among low-risk
        let low_risk: Vec<_> = matrix.iter().filter(|r| r.risk_score < dec!(0.5)).collect();
        if !low_risk.is_empty() {
            let best = low_risk
                .iter()
                .max_by(|a, b| a.net_benefit.cmp(&b.net_benefit))
                .unwrap();
            assert_eq!(*optimal, best.jurisdiction);
        }
    }

    #[test]
    fn test_cost_benefit_payback_ratio() {
        let input = multi_jurisdiction_input();
        let result = run_jurisdiction_substance_test(&input).unwrap();
        let cbs = result.cost_benefit_summary.unwrap();
        if cbs.total_substance_cost > Decimal::ZERO {
            assert_eq!(
                cbs.payback_ratio,
                cbs.total_tax_savings / cbs.total_substance_cost
            );
        }
    }

    #[test]
    fn test_cost_benefit_recommendation_text() {
        let input = multi_jurisdiction_input();
        let result = run_jurisdiction_substance_test(&input).unwrap();
        let cbs = result.cost_benefit_summary.unwrap();
        assert!(!cbs.recommendation.is_empty());
    }

    // ------ Comparison with different substance levels ------

    #[test]
    fn test_comparison_mixed_substance() {
        let mut input = multi_jurisdiction_input();
        // Make one jurisdiction have no substance
        let j = &mut input.jurisdictions[2];
        j.local_staff = 0;
        j.qualified_directors = 0;
        j.total_directors = 2;
        j.has_premises = false;
        j.board_meetings_local = 0;
        j.board_meetings_total = 4;
        j.ciga_local = false;

        let result = run_jurisdiction_substance_test(&input).unwrap();
        let matrix = result.comparison_matrix.unwrap();

        // The weak jurisdiction should have higher risk
        let ie_row = matrix.iter().find(|r| r.jurisdiction == "Ireland").unwrap();
        let cy_row = matrix.iter().find(|r| r.jurisdiction == "Cayman").unwrap();
        assert!(ie_row.risk_score > cy_row.risk_score);
    }

    // ------ Risk scoring ------

    #[test]
    fn test_risk_score_increases_with_treaty_reliance() {
        let mut input_no_treaty = base_input();
        input_no_treaty.treaty_reliance = false;
        // Weaken substance to get nonzero risk
        let j = &mut input_no_treaty.jurisdictions[0];
        j.local_staff = 1;
        j.qualified_directors = 0;
        j.total_directors = 2;
        j.has_premises = false;
        j.board_meetings_local = 1;
        j.board_meetings_total = 4;
        j.ciga_local = false;

        let mut input_treaty = input_no_treaty.clone();
        input_treaty.treaty_reliance = true;
        input_treaty.comparison_mode = true;
        input_treaty.jurisdictions.push(base_jurisdiction());
        input_treaty.jurisdictions[1].name = "BVI".to_string();

        input_no_treaty.comparison_mode = true;
        input_no_treaty.jurisdictions.push(base_jurisdiction());
        input_no_treaty.jurisdictions[1].name = "BVI".to_string();

        let result_no_treaty = run_jurisdiction_substance_test(&input_no_treaty).unwrap();
        let result_treaty = run_jurisdiction_substance_test(&input_treaty).unwrap();

        let m_no = result_no_treaty.comparison_matrix.unwrap();
        let m_yes = result_treaty.comparison_matrix.unwrap();

        // Treaty reliance should increase risk for the weak jurisdiction
        let risk_no = m_no[0].risk_score;
        let risk_yes = m_yes[0].risk_score;
        assert!(risk_yes >= risk_no);
    }

    // ------ Warnings ------

    #[test]
    fn test_warning_critically_low_score() {
        let mut input = base_input();
        let j = &mut input.jurisdictions[0];
        j.local_staff = 0;
        j.qualified_directors = 0;
        j.total_directors = 0;
        j.has_premises = false;
        j.premises_dedicated = false;
        j.board_meetings_local = 0;
        j.board_meetings_total = 0;
        j.ciga_local = false;
        j.annual_expenditure = Decimal::ZERO;
        j.local_expenditure = Decimal::ZERO;
        let result = run_jurisdiction_substance_test(&input).unwrap();
        assert!(result.warnings.iter().any(|w| w.contains("Critically low")));
    }

    #[test]
    fn test_warning_treaty_reliance_low_substance() {
        let mut input = base_input();
        input.treaty_reliance = true;
        let j = &mut input.jurisdictions[0];
        j.local_staff = 0;
        j.qualified_directors = 0;
        j.total_directors = 2;
        j.has_premises = false;
        j.board_meetings_local = 0;
        j.board_meetings_total = 4;
        j.ciga_local = false;
        let result = run_jurisdiction_substance_test(&input).unwrap();
        assert!(result
            .warnings
            .iter()
            .any(|w| w.contains("treaty") || w.contains("Treaty")));
    }

    #[test]
    fn test_warning_all_failed() {
        let mut input = multi_jurisdiction_input();
        for j in &mut input.jurisdictions {
            j.local_staff = 0;
            j.qualified_directors = 0;
            j.total_directors = 2;
            j.has_premises = false;
            j.premises_dedicated = false;
            j.board_meetings_local = 0;
            j.board_meetings_total = 4;
            j.ciga_local = false;
            j.outsourced_ciga = false;
            j.local_expenditure = Decimal::ZERO;
        }
        let result = run_jurisdiction_substance_test(&input).unwrap();
        assert!(result
            .warnings
            .iter()
            .any(|w| w.contains("All jurisdictions failed") || w.contains("redesign")));
    }

    #[test]
    fn test_warning_budget_exceeded() {
        let mut input = multi_jurisdiction_input();
        input.restructuring_budget = dec!(100_000); // Less than total substance cost
        let result = run_jurisdiction_substance_test(&input).unwrap();
        assert!(result
            .warnings
            .iter()
            .any(|w| w.contains("exceeds restructuring budget")));
    }

    #[test]
    fn test_no_warnings_for_good_substance() {
        let mut input = base_input();
        input.treaty_reliance = false;
        let result = run_jurisdiction_substance_test(&input).unwrap();
        // Should have no warnings (full substance, no treaty reliance)
        assert!(
            result.warnings.is_empty(),
            "Unexpected warnings: {:?}",
            result.warnings
        );
    }

    // ------ Metadata ------

    #[test]
    fn test_methodology_non_empty() {
        let input = base_input();
        let result = run_jurisdiction_substance_test(&input).unwrap();
        assert!(!result.methodology.is_empty());
    }

    #[test]
    fn test_assumptions_non_empty() {
        let input = base_input();
        let result = run_jurisdiction_substance_test(&input).unwrap();
        assert!(!result.assumptions.is_empty());
    }

    // ------ Serialization ------

    #[test]
    fn test_serialization_roundtrip() {
        let input = multi_jurisdiction_input();
        let result = run_jurisdiction_substance_test(&input).unwrap();
        let json = serde_json::to_string(&result).unwrap();
        let deserialized: JurisdictionTestOutput = serde_json::from_str(&json).unwrap();
        assert_eq!(
            deserialized.jurisdiction_results.len(),
            result.jurisdiction_results.len()
        );
    }

    // ------ Score details tests ------

    #[test]
    fn test_dm_board_meeting_75_percent_full_marks() {
        let mut j = base_jurisdiction();
        j.board_meetings_local = 3;
        j.board_meetings_total = 4; // 75%
        let (score, details) = score_directed_and_managed(&j);
        assert!(score >= dec!(80));
        assert!(details.iter().any(|d| d.contains("excellent")));
    }

    #[test]
    fn test_dm_no_ciga_zero_ciga_score() {
        let mut j = base_jurisdiction();
        j.ciga_local = false;
        j.outsourced_ciga = false;
        let (_, details) = score_directed_and_managed(&j);
        assert!(details.iter().any(|d| d.contains("CIGA not performed")));
    }

    #[test]
    fn test_cmc_majority_directors_full_marks() {
        let mut j = base_jurisdiction();
        j.name = "Ireland".to_string();
        let (_, details) = score_cmc(&j);
        assert!(details.iter().any(|d| d.contains("strong CMC")));
    }

    #[test]
    fn test_poem_no_board_meetings() {
        let mut j = base_jurisdiction();
        j.board_meetings_local = 0;
        j.board_meetings_total = 0;
        let (_, details) = score_poem(&j);
        assert!(details.iter().any(|d| d.contains("cannot assess POEM")));
    }

    #[test]
    fn test_atad_minority_directors() {
        let mut j = base_jurisdiction();
        j.qualified_directors = 1;
        j.total_directors = 4;
        let (_, details) = score_atad(&j);
        assert!(details.iter().any(|d| d.contains("non-compliant")));
    }

    #[test]
    fn test_singapore_no_fund_manager() {
        let mut j = base_jurisdiction();
        j.local_staff = 0;
        j.qualified_directors = 0;
        let (_, details) = score_singapore_incentive(&j);
        assert!(details.iter().any(|d| d.contains("No fund manager")));
    }

    // ------ Edge cases ------

    #[test]
    fn test_single_jurisdiction_comparison_mode_no_matrix() {
        let mut input = base_input();
        input.comparison_mode = true;
        // Only 1 jurisdiction — comparison mode requires >1
        let result = run_jurisdiction_substance_test(&input).unwrap();
        assert!(result.comparison_matrix.is_none());
    }

    #[test]
    fn test_zero_expenditure_no_division_error() {
        let mut input = base_input();
        input.jurisdictions[0].annual_expenditure = Decimal::ZERO;
        input.jurisdictions[0].local_expenditure = Decimal::ZERO;
        let result = run_jurisdiction_substance_test(&input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_zero_directors_no_division_error() {
        let mut input = base_input();
        input.jurisdictions[0].total_directors = 0;
        input.jurisdictions[0].qualified_directors = 0;
        let result = run_jurisdiction_substance_test(&input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_zero_board_meetings_no_division_error() {
        let mut input = base_input();
        input.jurisdictions[0].board_meetings_total = 0;
        input.jurisdictions[0].board_meetings_local = 0;
        let result = run_jurisdiction_substance_test(&input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_multiple_jurisdictions_all_results_returned() {
        let input = multi_jurisdiction_input();
        let result = run_jurisdiction_substance_test(&input).unwrap();
        assert_eq!(result.jurisdiction_results.len(), 3);
    }

    #[test]
    fn test_cost_benefit_zero_substance_cost() {
        let mut input = multi_jurisdiction_input();
        for j in &mut input.jurisdictions {
            j.annual_substance_cost = Decimal::ZERO;
        }
        let result = run_jurisdiction_substance_test(&input).unwrap();
        let cbs = result.cost_benefit_summary.unwrap();
        assert_eq!(cbs.payback_ratio, Decimal::ZERO);
    }

    #[test]
    fn test_cost_benefit_excellent_ratio() {
        let mut input = multi_jurisdiction_input();
        for j in &mut input.jurisdictions {
            j.annual_substance_cost = dec!(100_000);
            j.tax_savings = dec!(500_000); // 5x ratio
        }
        let result = run_jurisdiction_substance_test(&input).unwrap();
        let cbs = result.cost_benefit_summary.unwrap();
        assert!(cbs.recommendation.contains("Excellent"));
    }

    #[test]
    fn test_cost_benefit_negative_ratio() {
        let mut input = multi_jurisdiction_input();
        for j in &mut input.jurisdictions {
            j.annual_substance_cost = dec!(500_000);
            j.tax_savings = dec!(100_000); // 0.2x ratio
        }
        let result = run_jurisdiction_substance_test(&input).unwrap();
        let cbs = result.cost_benefit_summary.unwrap();
        assert!(cbs.recommendation.contains("Negative") || cbs.recommendation.contains("review"));
    }

    #[test]
    fn test_all_jurisdictions_have_correct_names() {
        let input = multi_jurisdiction_input();
        let result = run_jurisdiction_substance_test(&input).unwrap();
        let names: Vec<_> = result
            .jurisdiction_results
            .iter()
            .map(|r| r.jurisdiction.as_str())
            .collect();
        assert!(names.contains(&"Cayman"));
        assert!(names.contains(&"Luxembourg"));
        assert!(names.contains(&"Ireland"));
    }

    #[test]
    fn test_dm_50_percent_director_residency() {
        let mut j = base_jurisdiction();
        j.qualified_directors = 2;
        j.total_directors = 4; // exactly 50%
        let (_, details) = score_directed_and_managed(&j);
        assert!(details.iter().any(|d| d.contains("borderline")));
    }

    #[test]
    fn test_cmc_expenditure_ratio_scoring() {
        let mut j = base_jurisdiction();
        j.name = "Ireland".to_string();
        j.local_expenditure = dec!(200_000);
        j.annual_expenditure = dec!(500_000); // 40%
        let (_, details) = score_cmc(&j);
        assert!(details.iter().any(|d| d.contains("expenditure")));
    }
}
