use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

use crate::error::CorpFinanceError;
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum EntityType {
    HoldingCompany,
    IPHolding,
    FinanceLease,
    FundManagement,
    Banking,
    Insurance,
    HQ,
    ServiceCentre,
    PureEquityHolding,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PremisesType {
    Dedicated,
    Shared,
    Virtual,
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ComplianceStatus {
    Compliant,
    PartiallyCompliant,
    NonCompliant,
    HighRisk,
}

// ---------------------------------------------------------------------------
// Input
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EconomicSubstanceInput {
    pub entity_name: String,
    /// Jurisdiction code: Cayman, BVI, Luxembourg, Ireland, Jersey, Guernsey,
    /// Singapore, Netherlands, Switzerland, etc.
    pub jurisdiction: String,
    pub entity_type: EntityType,
    pub activity_type: String,
    pub annual_revenue: Decimal,
    /// Ratio of passive income (interest, royalties, dividends) to total — 0..1
    pub passive_income_ratio: Decimal,
    pub local_employees: u32,
    pub local_qualified_directors: u32,
    pub total_directors: u32,
    pub has_local_premises: bool,
    pub premises_type: PremisesType,
    pub board_meetings_in_jurisdiction: u32,
    pub total_board_meetings: u32,
    pub annual_operating_expenditure: Decimal,
    pub local_expenditure: Decimal,
    pub ciga_performed_locally: bool,
    pub outsourced_ciga: bool,
    pub years_established: u32,
}

// ---------------------------------------------------------------------------
// Output
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubstanceScoreBreakdown {
    /// Personnel score (0-25)
    pub personnel: Decimal,
    /// Premises score (0-20)
    pub premises: Decimal,
    /// Decision-making score (0-25)
    pub decision_making: Decimal,
    /// Expenditure score (0-15)
    pub expenditure: Decimal,
    /// Core Income Generating Activities score (0-15)
    pub ciga: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PenaltyExposure {
    pub year_1: String,
    pub year_2: String,
    pub year_3_plus: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EconomicSubstanceOutput {
    /// Aggregate substance score 0-100
    pub substance_score: Decimal,
    pub score_breakdown: SubstanceScoreBreakdown,
    pub compliance_status: ComplianceStatus,
    pub jurisdiction_requirements: Vec<String>,
    pub gaps_identified: Vec<String>,
    pub remediation_recommendations: Vec<String>,
    pub penalty_exposure: PenaltyExposure,
    pub estimated_annual_substance_cost: Decimal,
    pub risk_of_treaty_denial: Decimal,
    pub methodology: String,
    pub assumptions: Vec<String>,
    pub warnings: Vec<String>,
}

// ---------------------------------------------------------------------------
// Scoring helpers
// ---------------------------------------------------------------------------

/// Personnel score (max 25 pts)
fn score_personnel(input: &EconomicSubstanceInput) -> Decimal {
    let mut score = Decimal::ZERO;

    // Local employees: up to 15 pts
    match input.local_employees {
        0 => {}
        1 => score += dec!(5),
        2..=4 => score += dec!(10),
        _ => score += dec!(15),
    }

    // Director quality: up to 10 pts
    if input.total_directors > 0 {
        let ratio =
            Decimal::from(input.local_qualified_directors) / Decimal::from(input.total_directors);
        if ratio >= dec!(0.5) {
            score += dec!(10);
        } else if ratio > Decimal::ZERO {
            score += dec!(5);
        }
    }

    // IP holding requires highest substance — cap score if inadequate
    if input.entity_type == EntityType::IPHolding && input.local_employees < 3 {
        score = score.min(dec!(10));
    }

    score.min(dec!(25))
}

/// Premises score (max 20 pts)
fn score_premises(input: &EconomicSubstanceInput) -> Decimal {
    if !input.has_local_premises {
        return Decimal::ZERO;
    }
    match input.premises_type {
        PremisesType::Dedicated => dec!(20),
        PremisesType::Shared => dec!(12),
        PremisesType::Virtual => dec!(5),
        PremisesType::None => Decimal::ZERO,
    }
}

/// Decision-making score (max 25 pts)
fn score_decision_making(input: &EconomicSubstanceInput) -> Decimal {
    let mut score = Decimal::ZERO;

    // Board meeting ratio: up to 15 pts
    if input.total_board_meetings > 0 {
        let ratio = Decimal::from(input.board_meetings_in_jurisdiction)
            / Decimal::from(input.total_board_meetings);
        if ratio >= dec!(0.75) {
            score += dec!(15);
        } else if ratio >= dec!(0.5) {
            score += dec!(10);
        } else if ratio > Decimal::ZERO {
            score += dec!(5);
        }
    }

    // Director residency: up to 10 pts
    if input.total_directors > 0 {
        let ratio =
            Decimal::from(input.local_qualified_directors) / Decimal::from(input.total_directors);
        if ratio > dec!(0.5) {
            score += dec!(10);
        } else if ratio >= dec!(0.5) {
            // exactly 50%
            score += dec!(7);
        } else if input.local_qualified_directors > 0 {
            score += dec!(3);
        }
    }

    score.min(dec!(25))
}

/// Expenditure score (max 15 pts)
fn score_expenditure(input: &EconomicSubstanceInput) -> Decimal {
    if input.annual_operating_expenditure.is_zero() {
        return Decimal::ZERO;
    }
    let ratio = input.local_expenditure / input.annual_operating_expenditure;
    if ratio >= dec!(0.75) {
        dec!(15)
    } else if ratio >= dec!(0.50) {
        dec!(10)
    } else if ratio >= dec!(0.25) {
        dec!(5)
    } else if ratio > Decimal::ZERO {
        dec!(2)
    } else {
        Decimal::ZERO
    }
}

/// Core Income Generating Activities score (max 15 pts)
fn score_ciga(input: &EconomicSubstanceInput) -> Decimal {
    if input.ciga_performed_locally && !input.outsourced_ciga {
        dec!(15)
    } else if input.ciga_performed_locally && input.outsourced_ciga {
        // Partially outsourced
        dec!(8)
    } else if input.outsourced_ciga {
        // Outsourced but some local oversight
        dec!(3)
    } else {
        Decimal::ZERO
    }
}

// ---------------------------------------------------------------------------
// Jurisdiction-specific requirements
// ---------------------------------------------------------------------------

fn jurisdiction_requirements(jurisdiction: &str, entity_type: &EntityType) -> Vec<String> {
    match jurisdiction.to_lowercase().as_str() {
        "cayman" | "cayman islands" => {
            let mut reqs = vec![
                "Directed and managed in Cayman Islands".to_string(),
                "Adequate number of employees in Cayman".to_string(),
                "Adequate expenditure incurred in Cayman".to_string(),
                "Adequate physical presence in Cayman".to_string(),
            ];
            if *entity_type == EntityType::IPHolding {
                reqs.push(
                    "IP holding: highest level of substance — CIGA must not be outsourced"
                        .to_string(),
                );
                reqs.push(
                    "IP holding: qualified employees with capability to supervise IP".to_string(),
                );
            }
            if *entity_type == EntityType::PureEquityHolding {
                reqs.push(
                    "Pure equity holding: reduced test — comply with Companies Act, \
                     adequate employees/premises"
                        .to_string(),
                );
            }
            reqs.push("Core Income Generating Activities (CIGA) conducted in Cayman".to_string());
            reqs
        }
        "bvi" | "british virgin islands" => {
            let mut reqs = vec![
                "Directed and managed in BVI".to_string(),
                "Adequate number of employees in BVI".to_string(),
                "Adequate expenditure incurred in BVI".to_string(),
                "Adequate physical presence in BVI".to_string(),
                "CIGA conducted in or from BVI".to_string(),
                "BOSS (Beneficial Ownership Secure Search) system compliance".to_string(),
            ];
            if *entity_type == EntityType::IPHolding {
                reqs.push("IP holding: highest substance — no outsourcing of CIGA".to_string());
            }
            reqs
        }
        "luxembourg" => vec![
            "Majority of directors resident in Luxembourg".to_string(),
            "Local qualified employees appropriate to activity".to_string(),
            "Office premises in Luxembourg".to_string(),
            "Decision-making and board meetings in Luxembourg".to_string(),
            "Compliance with ATAD anti-avoidance provisions".to_string(),
            "Transfer pricing: arm's length + substance for holding/IP companies".to_string(),
            "Circular 56bis compliance for securitisation vehicles".to_string(),
        ],
        "ireland" => {
            let mut reqs = vec![
                "Central management and control exercised in Ireland".to_string(),
                "Board meets in Ireland".to_string(),
                "Irish resident directors".to_string(),
                "Strategic decisions made in Ireland".to_string(),
            ];
            if entity_type == &EntityType::FinanceLease || entity_type == &EntityType::ServiceCentre
            {
                reqs.push(
                    "Section 110: Irish directors, Irish board meetings, Irish administrator"
                        .to_string(),
                );
            }
            reqs
        }
        "jersey" => vec![
            "Directed and managed in Jersey".to_string(),
            "Adequate employees in Jersey".to_string(),
            "Adequate expenditure in Jersey".to_string(),
            "Physical premises in Jersey".to_string(),
            "CIGA conducted in Jersey".to_string(),
        ],
        "guernsey" => vec![
            "Directed and managed in Guernsey".to_string(),
            "Adequate employees in Guernsey".to_string(),
            "Adequate expenditure in Guernsey".to_string(),
            "Physical presence in Guernsey".to_string(),
            "CIGA conducted in Guernsey".to_string(),
        ],
        "singapore" => vec![
            "Fund manager located in Singapore".to_string(),
            "Investment professionals based in Singapore".to_string(),
            "Section 13X/13U tax incentive conditions must be met".to_string(),
            "Board meetings held in Singapore".to_string(),
            "Local office and administrative staff".to_string(),
        ],
        "netherlands" => vec![
            "Economic nexus: substance for holding/licensing companies".to_string(),
            "UBO register compliance".to_string(),
            "Local qualified directors (majority)".to_string(),
            "Office premises in the Netherlands".to_string(),
            "Decision-making in the Netherlands".to_string(),
            "Adequate local employees appropriate to activities".to_string(),
        ],
        "switzerland" => vec![
            "Cantonal substance requirements for principal structures".to_string(),
            "Local qualified directors".to_string(),
            "Local employees with decision-making authority".to_string(),
            "Office premises in Switzerland".to_string(),
            "Board meetings in Switzerland".to_string(),
        ],
        _ => vec![
            format!(
                "General economic substance requirements for {}",
                jurisdiction
            ),
            "Directed and managed in jurisdiction".to_string(),
            "Adequate employees and premises".to_string(),
            "CIGA conducted locally".to_string(),
        ],
    }
}

// ---------------------------------------------------------------------------
// Penalty exposure by jurisdiction
// ---------------------------------------------------------------------------

fn penalty_exposure(jurisdiction: &str) -> PenaltyExposure {
    match jurisdiction.to_lowercase().as_str() {
        "cayman" | "cayman islands" => PenaltyExposure {
            year_1: "CI$10,000 fine for non-compliance in year 1".to_string(),
            year_2: "CI$100,000 fine for continued non-compliance in year 2".to_string(),
            year_3_plus: "Potential strike-off from register in year 3+".to_string(),
        },
        "bvi" | "british virgin islands" => PenaltyExposure {
            year_1: "US$5,000 fine for non-compliance in year 1".to_string(),
            year_2: "US$50,000 fine for continued non-compliance in year 2".to_string(),
            year_3_plus: "Potential strike-off and compulsory liquidation in year 3+".to_string(),
        },
        "luxembourg" => PenaltyExposure {
            year_1: "Denial of tax benefits under anti-abuse provisions".to_string(),
            year_2: "Withholding tax imposed on outbound payments; TP adjustments".to_string(),
            year_3_plus: "Full denial of parent-subsidiary directive exemptions".to_string(),
        },
        "ireland" => PenaltyExposure {
            year_1: "Revenue Commissioners may challenge tax residence".to_string(),
            year_2: "Denial of S.110 benefits; reclassification of income".to_string(),
            year_3_plus: "Full tax exposure plus penalties and interest".to_string(),
        },
        "jersey" | "guernsey" => PenaltyExposure {
            year_1: "GBP 5,000 fine for non-compliance in year 1".to_string(),
            year_2: "GBP 50,000 fine for continued non-compliance in year 2".to_string(),
            year_3_plus: "Potential strike-off from register in year 3+".to_string(),
        },
        "singapore" => PenaltyExposure {
            year_1: "Loss of Section 13X/13U tax incentive".to_string(),
            year_2: "Full tax exposure on fund income".to_string(),
            year_3_plus: "Potential penalties and interest on underpaid tax".to_string(),
        },
        "netherlands" => PenaltyExposure {
            year_1: "Denial of dividend withholding tax exemption".to_string(),
            year_2: "TP adjustments and denial of interest deduction".to_string(),
            year_3_plus: "Full denial of treaty benefits and EU directive exemptions".to_string(),
        },
        "switzerland" => PenaltyExposure {
            year_1: "Loss of cantonal tax regime benefits".to_string(),
            year_2: "TP adjustments and additional cantonal tax".to_string(),
            year_3_plus: "Full federal and cantonal tax exposure".to_string(),
        },
        _ => PenaltyExposure {
            year_1: "Potential denial of local tax benefits".to_string(),
            year_2: "Increased scrutiny and potential TP adjustments".to_string(),
            year_3_plus: "Full tax exposure and potential treaty denial".to_string(),
        },
    }
}

// ---------------------------------------------------------------------------
// Gap identification
// ---------------------------------------------------------------------------

fn identify_gaps(
    input: &EconomicSubstanceInput,
    _breakdown: &SubstanceScoreBreakdown,
) -> Vec<String> {
    let mut gaps = Vec::new();

    // Personnel gaps
    if input.local_employees == 0 {
        gaps.push("No local employees — minimum staffing required".to_string());
    } else if input.entity_type == EntityType::IPHolding && input.local_employees < 3 {
        gaps.push(
            "IP holding entity requires higher staffing levels (minimum 3 qualified)".to_string(),
        );
    }

    if input.local_qualified_directors == 0 {
        gaps.push("No local qualified directors — at least 1 required".to_string());
    }

    if input.total_directors > 0 {
        let ratio =
            Decimal::from(input.local_qualified_directors) / Decimal::from(input.total_directors);
        if ratio < dec!(0.5) {
            gaps.push("Less than 50% of directors are locally qualified".to_string());
        }
    }

    // Premises gaps
    if !input.has_local_premises {
        gaps.push("No local premises — physical presence required".to_string());
    } else if input.premises_type == PremisesType::Virtual {
        gaps.push("Virtual office may be insufficient for substance requirements".to_string());
    }

    // Decision-making gaps
    if input.total_board_meetings > 0 {
        let ratio = Decimal::from(input.board_meetings_in_jurisdiction)
            / Decimal::from(input.total_board_meetings);
        if ratio < dec!(0.5) {
            gaps.push("Fewer than 50% of board meetings held in jurisdiction".to_string());
        }
    } else {
        gaps.push("No board meetings recorded — governance gap".to_string());
    }

    // Expenditure gaps
    if !input.annual_operating_expenditure.is_zero() {
        let ratio = input.local_expenditure / input.annual_operating_expenditure;
        if ratio < dec!(0.25) {
            gaps.push(
                "Local expenditure is less than 25% of total operating expenditure".to_string(),
            );
        }
    }

    // CIGA gaps
    if !input.ciga_performed_locally {
        gaps.push("Core Income Generating Activities not performed locally".to_string());
    }
    if input.outsourced_ciga {
        if input.entity_type == EntityType::IPHolding {
            gaps.push(
                "IP holding: outsourcing of CIGA is not permitted — highest substance required"
                    .to_string(),
            );
        } else {
            gaps.push(
                "CIGA partially or fully outsourced — may weaken substance position".to_string(),
            );
        }
    }

    // Passive income gateway
    if input.passive_income_ratio > dec!(0.75) {
        gaps.push(
            "Passive income ratio exceeds 75% — triggers EU ATAD III / Unshell Directive gateway"
                .to_string(),
        );
    }

    gaps
}

// ---------------------------------------------------------------------------
// Remediation recommendations
// ---------------------------------------------------------------------------

fn remediation_recommendations(gaps: &[String], input: &EconomicSubstanceInput) -> Vec<String> {
    let mut recs = Vec::new();

    for gap in gaps {
        if gap.contains("No local employees") {
            recs.push(format!(
                "Hire at least {} local employees with qualifications relevant to {}",
                if input.entity_type == EntityType::IPHolding {
                    3
                } else {
                    1
                },
                input.activity_type
            ));
        }
        if gap.contains("No local qualified directors") {
            recs.push(
                "Appoint at least one locally qualified director with relevant expertise"
                    .to_string(),
            );
        }
        if gap.contains("Less than 50% of directors") {
            recs.push(
                "Increase proportion of locally qualified directors to at least 50%".to_string(),
            );
        }
        if gap.contains("No local premises") {
            recs.push("Secure dedicated office premises in jurisdiction".to_string());
        }
        if gap.contains("Virtual office") {
            recs.push("Upgrade from virtual office to shared or dedicated premises".to_string());
        }
        if gap.contains("board meetings") && gap.contains("Fewer than 50%") {
            recs.push("Hold majority of board meetings in jurisdiction (target >75%)".to_string());
        }
        if gap.contains("No board meetings recorded") {
            recs.push("Establish regular board meeting schedule in jurisdiction".to_string());
        }
        if gap.contains("Local expenditure is less than") {
            recs.push(
                "Increase local expenditure to at least 25-50% of operating costs".to_string(),
            );
        }
        if gap.contains("not performed locally") {
            recs.push("Relocate core income generating activities to jurisdiction".to_string());
        }
        if gap.contains("outsourcing of CIGA is not permitted") {
            recs.push(
                "IP holding: insource all CIGA — outsourcing disqualifies substance".to_string(),
            );
        }
        if gap.contains("CIGA partially or fully outsourced") {
            recs.push(
                "Reduce outsourcing of CIGA; retain adequate supervision locally".to_string(),
            );
        }
        if gap.contains("Passive income ratio exceeds 75%") {
            recs.push("Diversify revenue streams or increase active business income".to_string());
            recs.push(
                "Review EU ATAD III / Unshell Directive implications with tax counsel".to_string(),
            );
        }
    }

    if recs.is_empty() {
        recs.push(
            "No immediate remediation actions required — maintain current substance levels"
                .to_string(),
        );
    }

    recs
}

// ---------------------------------------------------------------------------
// Substance cost estimation
// ---------------------------------------------------------------------------

fn estimate_annual_substance_cost(input: &EconomicSubstanceInput) -> Decimal {
    let jurisdiction = input.jurisdiction.to_lowercase();
    // Base per-employee cost varies by jurisdiction
    let per_employee_cost = match jurisdiction.as_str() {
        "cayman" | "cayman islands" => dec!(80000),
        "bvi" | "british virgin islands" => dec!(60000),
        "luxembourg" => dec!(120000),
        "ireland" => dec!(90000),
        "jersey" | "guernsey" => dec!(85000),
        "singapore" => dec!(100000),
        "netherlands" => dec!(110000),
        "switzerland" => dec!(150000),
        _ => dec!(75000),
    };

    // Premises cost
    let premises_cost = match (&input.premises_type, jurisdiction.as_str()) {
        (PremisesType::Dedicated, "luxembourg") => dec!(120000),
        (PremisesType::Dedicated, "switzerland") => dec!(150000),
        (PremisesType::Dedicated, "singapore") => dec!(100000),
        (PremisesType::Dedicated, _) => dec!(80000),
        (PremisesType::Shared, _) => dec!(30000),
        (PremisesType::Virtual, _) => dec!(10000),
        (PremisesType::None, _) => Decimal::ZERO,
    };

    // Director fees (per qualified director)
    let director_cost = Decimal::from(input.local_qualified_directors) * dec!(25000);

    // Compliance and admin overhead
    let compliance_overhead = dec!(50000);

    let employee_cost = Decimal::from(input.local_employees) * per_employee_cost;

    employee_cost + premises_cost + director_cost + compliance_overhead
}

// ---------------------------------------------------------------------------
// Treaty denial risk
// ---------------------------------------------------------------------------

fn treaty_denial_risk(score: Decimal, input: &EconomicSubstanceInput) -> Decimal {
    let mut risk = Decimal::ZERO;

    // Low substance score is the main driver
    if score < dec!(30) {
        risk += dec!(0.50);
    } else if score < dec!(50) {
        risk += dec!(0.30);
    } else if score < dec!(70) {
        risk += dec!(0.10);
    }

    // High passive income increases risk
    if input.passive_income_ratio > dec!(0.75) {
        risk += dec!(0.20);
    } else if input.passive_income_ratio > dec!(0.50) {
        risk += dec!(0.10);
    }

    // Newly established entities face more scrutiny
    if input.years_established < 2 {
        risk += dec!(0.10);
    }

    // No local CIGA
    if !input.ciga_performed_locally {
        risk += dec!(0.15);
    }

    // Cap at 1.0
    risk.min(dec!(1))
}

// ---------------------------------------------------------------------------
// Compliance status determination
// ---------------------------------------------------------------------------

fn determine_compliance_status(score: Decimal, gaps: &[String]) -> ComplianceStatus {
    if score >= dec!(75) && gaps.is_empty() {
        ComplianceStatus::Compliant
    } else if score >= dec!(50) {
        ComplianceStatus::PartiallyCompliant
    } else if score >= dec!(30) {
        ComplianceStatus::NonCompliant
    } else {
        ComplianceStatus::HighRisk
    }
}

// ---------------------------------------------------------------------------
// Warnings
// ---------------------------------------------------------------------------

fn generate_warnings(input: &EconomicSubstanceInput, score: Decimal) -> Vec<String> {
    let mut warnings = Vec::new();

    if input.passive_income_ratio > dec!(0.75) {
        warnings.push(
            "Entity exceeds 75% passive income gateway — EU ATAD III / Unshell Directive applies"
                .to_string(),
        );
    }

    if input.entity_type == EntityType::IPHolding && input.outsourced_ciga {
        warnings
            .push("IP holding entity cannot outsource CIGA under Cayman/BVI ES Acts".to_string());
    }

    if score < dec!(30) {
        warnings
            .push("Substance score critically low — immediate remediation recommended".to_string());
    }

    if input.years_established == 0 {
        warnings.push("Newly incorporated entity — enhanced scrutiny expected".to_string());
    }

    if input.total_board_meetings == 0 {
        warnings.push("No board meetings recorded — governance deficiency".to_string());
    }

    if input.total_directors == 0 {
        warnings.push("No directors recorded — entity may not be properly constituted".to_string());
    }

    warnings
}

// ---------------------------------------------------------------------------
// Public function
// ---------------------------------------------------------------------------

/// Analyze economic substance for an entity in a given jurisdiction.
///
/// Returns a comprehensive assessment including a 0-100 substance score,
/// compliance status, gap analysis, remediation recommendations, and
/// penalty exposure.
pub fn analyze_economic_substance(
    input: &EconomicSubstanceInput,
) -> CorpFinanceResult<EconomicSubstanceOutput> {
    // Input validation
    if input.entity_name.is_empty() {
        return Err(CorpFinanceError::InvalidInput {
            field: "entity_name".to_string(),
            reason: "Entity name must not be empty".to_string(),
        });
    }
    if input.jurisdiction.is_empty() {
        return Err(CorpFinanceError::InvalidInput {
            field: "jurisdiction".to_string(),
            reason: "Jurisdiction must not be empty".to_string(),
        });
    }
    if input.passive_income_ratio < Decimal::ZERO || input.passive_income_ratio > Decimal::ONE {
        return Err(CorpFinanceError::InvalidInput {
            field: "passive_income_ratio".to_string(),
            reason: "Passive income ratio must be between 0 and 1".to_string(),
        });
    }
    if input.annual_revenue < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "annual_revenue".to_string(),
            reason: "Annual revenue must not be negative".to_string(),
        });
    }
    if input.local_qualified_directors > input.total_directors {
        return Err(CorpFinanceError::InvalidInput {
            field: "local_qualified_directors".to_string(),
            reason: "Local qualified directors cannot exceed total directors".to_string(),
        });
    }
    if input.board_meetings_in_jurisdiction > input.total_board_meetings {
        return Err(CorpFinanceError::InvalidInput {
            field: "board_meetings_in_jurisdiction".to_string(),
            reason: "Board meetings in jurisdiction cannot exceed total board meetings".to_string(),
        });
    }
    if input.local_expenditure > input.annual_operating_expenditure {
        return Err(CorpFinanceError::InvalidInput {
            field: "local_expenditure".to_string(),
            reason: "Local expenditure cannot exceed total operating expenditure".to_string(),
        });
    }
    if input.annual_operating_expenditure < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "annual_operating_expenditure".to_string(),
            reason: "Annual operating expenditure must not be negative".to_string(),
        });
    }

    // Score each dimension
    let personnel = score_personnel(input);
    let premises = score_premises(input);
    let decision_making = score_decision_making(input);
    let expenditure = score_expenditure(input);
    let ciga = score_ciga(input);

    let breakdown = SubstanceScoreBreakdown {
        personnel,
        premises,
        decision_making,
        expenditure,
        ciga,
    };

    let substance_score = personnel + premises + decision_making + expenditure + ciga;

    let gaps = identify_gaps(input, &breakdown);
    let compliance_status = determine_compliance_status(substance_score, &gaps);
    let recs = remediation_recommendations(&gaps, input);
    let reqs = jurisdiction_requirements(&input.jurisdiction, &input.entity_type);
    let penalty = penalty_exposure(&input.jurisdiction);
    let cost = estimate_annual_substance_cost(input);
    let treaty_risk = treaty_denial_risk(substance_score, input);
    let warnings = generate_warnings(input, substance_score);

    let assumptions = vec![
        "Substance scoring based on EU ATAD III gateway criteria and Cayman/BVI ES Act standards"
            .to_string(),
        "Personnel score weights full-time local employees and qualified director ratio".to_string(),
        "Premises score reflects dedicated > shared > virtual hierarchy".to_string(),
        "Expenditure score based on local-to-total operating expenditure ratio".to_string(),
        "Cost estimates use jurisdiction-specific benchmarks for employees, premises, and compliance"
            .to_string(),
        "Treaty denial risk considers substance score, passive income ratio, and entity age"
            .to_string(),
    ];

    Ok(EconomicSubstanceOutput {
        substance_score,
        score_breakdown: breakdown,
        compliance_status,
        jurisdiction_requirements: reqs,
        gaps_identified: gaps,
        remediation_recommendations: recs,
        penalty_exposure: penalty,
        estimated_annual_substance_cost: cost,
        risk_of_treaty_denial: treaty_risk,
        methodology: "Economic Substance Analysis per EU ATAD III / Unshell Directive criteria, \
                       Cayman Islands ES Act, BVI ES Act, and jurisdiction-specific substance \
                       requirements"
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

    fn base_input() -> EconomicSubstanceInput {
        EconomicSubstanceInput {
            entity_name: "TestCo Holdings".to_string(),
            jurisdiction: "Cayman".to_string(),
            entity_type: EntityType::HoldingCompany,
            activity_type: "Holding".to_string(),
            annual_revenue: dec!(10_000_000),
            passive_income_ratio: dec!(0.60),
            local_employees: 3,
            local_qualified_directors: 2,
            total_directors: 3,
            has_local_premises: true,
            premises_type: PremisesType::Dedicated,
            board_meetings_in_jurisdiction: 4,
            total_board_meetings: 4,
            annual_operating_expenditure: dec!(500_000),
            local_expenditure: dec!(400_000),
            ciga_performed_locally: true,
            outsourced_ciga: false,
            years_established: 5,
        }
    }

    // ------ Validation tests ------

    #[test]
    fn test_empty_entity_name_rejected() {
        let mut input = base_input();
        input.entity_name = "".to_string();
        let result = analyze_economic_substance(&input);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("entity_name"));
    }

    #[test]
    fn test_empty_jurisdiction_rejected() {
        let mut input = base_input();
        input.jurisdiction = "".to_string();
        let result = analyze_economic_substance(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_negative_passive_income_ratio_rejected() {
        let mut input = base_input();
        input.passive_income_ratio = dec!(-0.1);
        let result = analyze_economic_substance(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_passive_income_ratio_above_one_rejected() {
        let mut input = base_input();
        input.passive_income_ratio = dec!(1.1);
        let result = analyze_economic_substance(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_negative_revenue_rejected() {
        let mut input = base_input();
        input.annual_revenue = dec!(-1000);
        let result = analyze_economic_substance(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_local_directors_exceed_total_rejected() {
        let mut input = base_input();
        input.local_qualified_directors = 5;
        input.total_directors = 3;
        let result = analyze_economic_substance(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_board_meetings_exceed_total_rejected() {
        let mut input = base_input();
        input.board_meetings_in_jurisdiction = 6;
        input.total_board_meetings = 4;
        let result = analyze_economic_substance(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_local_expenditure_exceeds_total_rejected() {
        let mut input = base_input();
        input.local_expenditure = dec!(600_000);
        input.annual_operating_expenditure = dec!(500_000);
        let result = analyze_economic_substance(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_negative_operating_expenditure_rejected() {
        let mut input = base_input();
        input.annual_operating_expenditure = dec!(-100);
        input.local_expenditure = dec!(-200);
        let result = analyze_economic_substance(&input);
        assert!(result.is_err());
    }

    // ------ Scoring tests ------

    #[test]
    fn test_full_substance_score_high() {
        let input = base_input();
        let result = analyze_economic_substance(&input).unwrap();
        // Dedicated premises, good directors, all meetings local, good expenditure, CIGA local
        assert!(result.substance_score >= dec!(80));
    }

    #[test]
    fn test_score_breakdown_sums_to_total() {
        let input = base_input();
        let result = analyze_economic_substance(&input).unwrap();
        let b = &result.score_breakdown;
        let sum = b.personnel + b.premises + b.decision_making + b.expenditure + b.ciga;
        assert_eq!(sum, result.substance_score);
    }

    #[test]
    fn test_no_premises_score_zero() {
        let mut input = base_input();
        input.has_local_premises = false;
        input.premises_type = PremisesType::None;
        let result = analyze_economic_substance(&input).unwrap();
        assert_eq!(result.score_breakdown.premises, Decimal::ZERO);
    }

    #[test]
    fn test_dedicated_premises_full_score() {
        let input = base_input();
        let result = analyze_economic_substance(&input).unwrap();
        assert_eq!(result.score_breakdown.premises, dec!(20));
    }

    #[test]
    fn test_shared_premises_partial_score() {
        let mut input = base_input();
        input.premises_type = PremisesType::Shared;
        let result = analyze_economic_substance(&input).unwrap();
        assert_eq!(result.score_breakdown.premises, dec!(12));
    }

    #[test]
    fn test_virtual_premises_low_score() {
        let mut input = base_input();
        input.premises_type = PremisesType::Virtual;
        let result = analyze_economic_substance(&input).unwrap();
        assert_eq!(result.score_breakdown.premises, dec!(5));
    }

    #[test]
    fn test_zero_employees_low_personnel_score() {
        let mut input = base_input();
        input.local_employees = 0;
        let result = analyze_economic_substance(&input).unwrap();
        // Still gets director points
        assert!(result.score_breakdown.personnel <= dec!(10));
    }

    #[test]
    fn test_one_employee_partial_personnel_score() {
        let mut input = base_input();
        input.local_employees = 1;
        let result = analyze_economic_substance(&input).unwrap();
        assert!(result.score_breakdown.personnel >= dec!(5));
    }

    #[test]
    fn test_many_employees_high_personnel_score() {
        let mut input = base_input();
        input.local_employees = 10;
        let result = analyze_economic_substance(&input).unwrap();
        assert!(result.score_breakdown.personnel >= dec!(20));
    }

    #[test]
    fn test_no_directors_zero_director_component() {
        let mut input = base_input();
        input.total_directors = 0;
        input.local_qualified_directors = 0;
        let result = analyze_economic_substance(&input).unwrap();
        // Only employee component, no director points
        assert!(result.score_breakdown.personnel <= dec!(15));
    }

    #[test]
    fn test_all_meetings_in_jurisdiction_high_decision_score() {
        let input = base_input();
        let result = analyze_economic_substance(&input).unwrap();
        assert!(result.score_breakdown.decision_making >= dec!(20));
    }

    #[test]
    fn test_no_meetings_zero_decision_score() {
        let mut input = base_input();
        input.board_meetings_in_jurisdiction = 0;
        input.total_board_meetings = 0;
        input.total_directors = 0;
        input.local_qualified_directors = 0;
        let result = analyze_economic_substance(&input).unwrap();
        assert_eq!(result.score_breakdown.decision_making, Decimal::ZERO);
    }

    #[test]
    fn test_half_meetings_partial_decision_score() {
        let mut input = base_input();
        input.board_meetings_in_jurisdiction = 2;
        input.total_board_meetings = 4;
        let result = analyze_economic_substance(&input).unwrap();
        assert!(result.score_breakdown.decision_making >= dec!(10));
    }

    #[test]
    fn test_ciga_local_full_score() {
        let input = base_input();
        let result = analyze_economic_substance(&input).unwrap();
        assert_eq!(result.score_breakdown.ciga, dec!(15));
    }

    #[test]
    fn test_ciga_outsourced_partial_score() {
        let mut input = base_input();
        input.outsourced_ciga = true;
        let result = analyze_economic_substance(&input).unwrap();
        assert_eq!(result.score_breakdown.ciga, dec!(8));
    }

    #[test]
    fn test_ciga_not_local_but_outsourced_low_score() {
        let mut input = base_input();
        input.ciga_performed_locally = false;
        input.outsourced_ciga = true;
        let result = analyze_economic_substance(&input).unwrap();
        assert_eq!(result.score_breakdown.ciga, dec!(3));
    }

    #[test]
    fn test_ciga_not_local_not_outsourced_zero() {
        let mut input = base_input();
        input.ciga_performed_locally = false;
        input.outsourced_ciga = false;
        let result = analyze_economic_substance(&input).unwrap();
        assert_eq!(result.score_breakdown.ciga, Decimal::ZERO);
    }

    #[test]
    fn test_high_local_expenditure_full_score() {
        let input = base_input(); // 400k of 500k = 80%
        let result = analyze_economic_substance(&input).unwrap();
        assert_eq!(result.score_breakdown.expenditure, dec!(15));
    }

    #[test]
    fn test_low_local_expenditure_low_score() {
        let mut input = base_input();
        input.local_expenditure = dec!(50_000);
        let result = analyze_economic_substance(&input).unwrap();
        // 10% ratio — low score
        assert!(result.score_breakdown.expenditure <= dec!(2));
    }

    #[test]
    fn test_zero_operating_expenditure_zero_score() {
        let mut input = base_input();
        input.annual_operating_expenditure = Decimal::ZERO;
        input.local_expenditure = Decimal::ZERO;
        let result = analyze_economic_substance(&input).unwrap();
        assert_eq!(result.score_breakdown.expenditure, Decimal::ZERO);
    }

    // ------ Compliance status tests ------

    #[test]
    fn test_high_score_no_gaps_compliant() {
        let input = base_input();
        let result = analyze_economic_substance(&input).unwrap();
        assert_eq!(result.compliance_status, ComplianceStatus::Compliant);
    }

    #[test]
    fn test_medium_score_partially_compliant() {
        let mut input = base_input();
        input.has_local_premises = false;
        input.premises_type = PremisesType::None;
        input.ciga_performed_locally = false;
        input.outsourced_ciga = true;
        let result = analyze_economic_substance(&input).unwrap();
        assert_eq!(
            result.compliance_status,
            ComplianceStatus::PartiallyCompliant
        );
    }

    #[test]
    fn test_low_score_non_compliant() {
        let mut input = base_input();
        input.local_employees = 0;
        input.local_qualified_directors = 0;
        input.total_directors = 2;
        input.has_local_premises = false;
        input.premises_type = PremisesType::None;
        input.board_meetings_in_jurisdiction = 0;
        input.total_board_meetings = 4;
        input.local_expenditure = dec!(50_000);
        input.ciga_performed_locally = false;
        input.outsourced_ciga = false;
        let result = analyze_economic_substance(&input).unwrap();
        assert!(
            result.compliance_status == ComplianceStatus::NonCompliant
                || result.compliance_status == ComplianceStatus::HighRisk
        );
    }

    #[test]
    fn test_very_low_score_high_risk() {
        let mut input = base_input();
        input.local_employees = 0;
        input.local_qualified_directors = 0;
        input.total_directors = 0;
        input.has_local_premises = false;
        input.premises_type = PremisesType::None;
        input.board_meetings_in_jurisdiction = 0;
        input.total_board_meetings = 0;
        input.annual_operating_expenditure = Decimal::ZERO;
        input.local_expenditure = Decimal::ZERO;
        input.ciga_performed_locally = false;
        input.outsourced_ciga = false;
        let result = analyze_economic_substance(&input).unwrap();
        assert_eq!(result.compliance_status, ComplianceStatus::HighRisk);
        assert_eq!(result.substance_score, Decimal::ZERO);
    }

    // ------ Gap identification tests ------

    #[test]
    fn test_no_gaps_for_full_substance() {
        let input = base_input();
        let result = analyze_economic_substance(&input).unwrap();
        assert!(
            result.gaps_identified.is_empty(),
            "Expected no gaps, got: {:?}",
            result.gaps_identified
        );
    }

    #[test]
    fn test_gap_no_employees() {
        let mut input = base_input();
        input.local_employees = 0;
        let result = analyze_economic_substance(&input).unwrap();
        assert!(result
            .gaps_identified
            .iter()
            .any(|g| g.contains("No local employees")));
    }

    #[test]
    fn test_gap_no_directors() {
        let mut input = base_input();
        input.local_qualified_directors = 0;
        input.total_directors = 3;
        let result = analyze_economic_substance(&input).unwrap();
        assert!(result
            .gaps_identified
            .iter()
            .any(|g| g.contains("No local qualified directors")));
    }

    #[test]
    fn test_gap_no_premises() {
        let mut input = base_input();
        input.has_local_premises = false;
        let result = analyze_economic_substance(&input).unwrap();
        assert!(result
            .gaps_identified
            .iter()
            .any(|g| g.contains("No local premises")));
    }

    #[test]
    fn test_gap_virtual_office() {
        let mut input = base_input();
        input.premises_type = PremisesType::Virtual;
        let result = analyze_economic_substance(&input).unwrap();
        assert!(result
            .gaps_identified
            .iter()
            .any(|g| g.contains("Virtual office")));
    }

    #[test]
    fn test_gap_no_board_meetings() {
        let mut input = base_input();
        input.board_meetings_in_jurisdiction = 0;
        input.total_board_meetings = 0;
        let result = analyze_economic_substance(&input).unwrap();
        assert!(result
            .gaps_identified
            .iter()
            .any(|g| g.contains("No board meetings")));
    }

    #[test]
    fn test_gap_low_board_meeting_ratio() {
        let mut input = base_input();
        input.board_meetings_in_jurisdiction = 1;
        input.total_board_meetings = 4;
        let result = analyze_economic_substance(&input).unwrap();
        assert!(result
            .gaps_identified
            .iter()
            .any(|g| g.contains("board meetings")));
    }

    #[test]
    fn test_gap_low_expenditure() {
        let mut input = base_input();
        input.local_expenditure = dec!(50_000); // 10% of 500k
        let result = analyze_economic_substance(&input).unwrap();
        assert!(result
            .gaps_identified
            .iter()
            .any(|g| g.contains("Local expenditure")));
    }

    #[test]
    fn test_gap_ciga_not_local() {
        let mut input = base_input();
        input.ciga_performed_locally = false;
        let result = analyze_economic_substance(&input).unwrap();
        assert!(result
            .gaps_identified
            .iter()
            .any(|g| g.contains("not performed locally")));
    }

    #[test]
    fn test_gap_ciga_outsourced() {
        let mut input = base_input();
        input.outsourced_ciga = true;
        let result = analyze_economic_substance(&input).unwrap();
        assert!(result
            .gaps_identified
            .iter()
            .any(|g| g.contains("outsourced")));
    }

    #[test]
    fn test_gap_high_passive_income() {
        let mut input = base_input();
        input.passive_income_ratio = dec!(0.80);
        let result = analyze_economic_substance(&input).unwrap();
        assert!(result
            .gaps_identified
            .iter()
            .any(|g| g.contains("Passive income ratio")));
    }

    #[test]
    fn test_gap_ip_holding_insufficient_employees() {
        let mut input = base_input();
        input.entity_type = EntityType::IPHolding;
        input.local_employees = 2;
        let result = analyze_economic_substance(&input).unwrap();
        assert!(result
            .gaps_identified
            .iter()
            .any(|g| g.contains("IP holding")));
    }

    #[test]
    fn test_gap_ip_holding_outsourced_ciga() {
        let mut input = base_input();
        input.entity_type = EntityType::IPHolding;
        input.outsourced_ciga = true;
        let result = analyze_economic_substance(&input).unwrap();
        assert!(result
            .gaps_identified
            .iter()
            .any(|g| g.contains("outsourcing of CIGA is not permitted")));
    }

    // ------ Jurisdiction requirements tests ------

    #[test]
    fn test_cayman_requirements() {
        let input = base_input();
        let result = analyze_economic_substance(&input).unwrap();
        assert!(result
            .jurisdiction_requirements
            .iter()
            .any(|r| r.contains("Cayman")));
        assert!(result
            .jurisdiction_requirements
            .iter()
            .any(|r| r.contains("CIGA")));
    }

    #[test]
    fn test_bvi_requirements() {
        let mut input = base_input();
        input.jurisdiction = "BVI".to_string();
        let result = analyze_economic_substance(&input).unwrap();
        assert!(result
            .jurisdiction_requirements
            .iter()
            .any(|r| r.contains("BVI")));
        assert!(result
            .jurisdiction_requirements
            .iter()
            .any(|r| r.contains("BOSS")));
    }

    #[test]
    fn test_luxembourg_requirements() {
        let mut input = base_input();
        input.jurisdiction = "Luxembourg".to_string();
        let result = analyze_economic_substance(&input).unwrap();
        assert!(result
            .jurisdiction_requirements
            .iter()
            .any(|r| r.contains("Luxembourg")));
        assert!(result
            .jurisdiction_requirements
            .iter()
            .any(|r| r.contains("ATAD")));
    }

    #[test]
    fn test_ireland_requirements() {
        let mut input = base_input();
        input.jurisdiction = "Ireland".to_string();
        let result = analyze_economic_substance(&input).unwrap();
        assert!(result
            .jurisdiction_requirements
            .iter()
            .any(|r| r.contains("Ireland")));
    }

    #[test]
    fn test_ireland_s110_requirements() {
        let mut input = base_input();
        input.jurisdiction = "Ireland".to_string();
        input.entity_type = EntityType::FinanceLease;
        let result = analyze_economic_substance(&input).unwrap();
        assert!(result
            .jurisdiction_requirements
            .iter()
            .any(|r| r.contains("Section 110")));
    }

    #[test]
    fn test_jersey_requirements() {
        let mut input = base_input();
        input.jurisdiction = "Jersey".to_string();
        let result = analyze_economic_substance(&input).unwrap();
        assert!(result
            .jurisdiction_requirements
            .iter()
            .any(|r| r.contains("Jersey")));
    }

    #[test]
    fn test_singapore_requirements() {
        let mut input = base_input();
        input.jurisdiction = "Singapore".to_string();
        let result = analyze_economic_substance(&input).unwrap();
        assert!(result
            .jurisdiction_requirements
            .iter()
            .any(|r| r.contains("13X") || r.contains("13U")));
    }

    #[test]
    fn test_netherlands_requirements() {
        let mut input = base_input();
        input.jurisdiction = "Netherlands".to_string();
        let result = analyze_economic_substance(&input).unwrap();
        assert!(result
            .jurisdiction_requirements
            .iter()
            .any(|r| r.contains("UBO")));
    }

    #[test]
    fn test_switzerland_requirements() {
        let mut input = base_input();
        input.jurisdiction = "Switzerland".to_string();
        let result = analyze_economic_substance(&input).unwrap();
        assert!(result
            .jurisdiction_requirements
            .iter()
            .any(|r| r.contains("Cantonal") || r.contains("cantonal")));
    }

    #[test]
    fn test_unknown_jurisdiction_generic_requirements() {
        let mut input = base_input();
        input.jurisdiction = "Bermuda".to_string();
        let result = analyze_economic_substance(&input).unwrap();
        assert!(!result.jurisdiction_requirements.is_empty());
    }

    #[test]
    fn test_cayman_ip_holding_extra_requirements() {
        let mut input = base_input();
        input.entity_type = EntityType::IPHolding;
        let result = analyze_economic_substance(&input).unwrap();
        assert!(result
            .jurisdiction_requirements
            .iter()
            .any(|r| r.contains("IP holding")));
    }

    #[test]
    fn test_cayman_pure_equity_holding_reduced_test() {
        let mut input = base_input();
        input.entity_type = EntityType::PureEquityHolding;
        let result = analyze_economic_substance(&input).unwrap();
        assert!(result
            .jurisdiction_requirements
            .iter()
            .any(|r| r.contains("Pure equity holding")));
    }

    // ------ Penalty exposure tests ------

    #[test]
    fn test_cayman_penalty_exposure() {
        let input = base_input();
        let result = analyze_economic_substance(&input).unwrap();
        assert!(result.penalty_exposure.year_1.contains("CI$10,000"));
        assert!(result.penalty_exposure.year_2.contains("CI$100,000"));
        assert!(result.penalty_exposure.year_3_plus.contains("strike-off"));
    }

    #[test]
    fn test_bvi_penalty_exposure() {
        let mut input = base_input();
        input.jurisdiction = "BVI".to_string();
        let result = analyze_economic_substance(&input).unwrap();
        assert!(result.penalty_exposure.year_1.contains("US$5,000"));
    }

    #[test]
    fn test_luxembourg_penalty_exposure() {
        let mut input = base_input();
        input.jurisdiction = "Luxembourg".to_string();
        let result = analyze_economic_substance(&input).unwrap();
        assert!(result.penalty_exposure.year_1.contains("Denial"));
    }

    // ------ Cost estimation tests ------

    #[test]
    fn test_substance_cost_positive() {
        let input = base_input();
        let result = analyze_economic_substance(&input).unwrap();
        assert!(result.estimated_annual_substance_cost > Decimal::ZERO);
    }

    #[test]
    fn test_substance_cost_includes_employees() {
        let mut input = base_input();
        input.local_employees = 10;
        let result_many = analyze_economic_substance(&input).unwrap();
        input.local_employees = 1;
        let result_few = analyze_economic_substance(&input).unwrap();
        assert!(
            result_many.estimated_annual_substance_cost
                > result_few.estimated_annual_substance_cost
        );
    }

    #[test]
    fn test_substance_cost_varies_by_jurisdiction() {
        let mut input_cy = base_input();
        input_cy.jurisdiction = "Cayman".to_string();
        let cost_cy = analyze_economic_substance(&input_cy)
            .unwrap()
            .estimated_annual_substance_cost;

        let mut input_ch = base_input();
        input_ch.jurisdiction = "Switzerland".to_string();
        let cost_ch = analyze_economic_substance(&input_ch)
            .unwrap()
            .estimated_annual_substance_cost;

        // Switzerland is more expensive than Cayman
        assert!(cost_ch > cost_cy);
    }

    // ------ Treaty denial risk tests ------

    #[test]
    fn test_high_substance_low_treaty_risk() {
        let input = base_input();
        let result = analyze_economic_substance(&input).unwrap();
        assert!(result.risk_of_treaty_denial <= dec!(0.10));
    }

    #[test]
    fn test_low_substance_high_treaty_risk() {
        let mut input = base_input();
        input.local_employees = 0;
        input.local_qualified_directors = 0;
        input.total_directors = 0;
        input.has_local_premises = false;
        input.premises_type = PremisesType::None;
        input.board_meetings_in_jurisdiction = 0;
        input.total_board_meetings = 0;
        input.annual_operating_expenditure = Decimal::ZERO;
        input.local_expenditure = Decimal::ZERO;
        input.ciga_performed_locally = false;
        input.outsourced_ciga = false;
        input.passive_income_ratio = dec!(0.90);
        input.years_established = 0;
        let result = analyze_economic_substance(&input).unwrap();
        assert!(result.risk_of_treaty_denial >= dec!(0.50));
    }

    #[test]
    fn test_treaty_risk_capped_at_one() {
        let mut input = base_input();
        input.local_employees = 0;
        input.local_qualified_directors = 0;
        input.total_directors = 0;
        input.has_local_premises = false;
        input.premises_type = PremisesType::None;
        input.board_meetings_in_jurisdiction = 0;
        input.total_board_meetings = 0;
        input.annual_operating_expenditure = Decimal::ZERO;
        input.local_expenditure = Decimal::ZERO;
        input.ciga_performed_locally = false;
        input.outsourced_ciga = false;
        input.passive_income_ratio = dec!(0.95);
        input.years_established = 0;
        let result = analyze_economic_substance(&input).unwrap();
        assert!(result.risk_of_treaty_denial <= Decimal::ONE);
    }

    #[test]
    fn test_high_passive_income_increases_risk() {
        let mut low_passive = base_input();
        low_passive.passive_income_ratio = dec!(0.30);
        let result_low = analyze_economic_substance(&low_passive).unwrap();

        let mut high_passive = base_input();
        high_passive.passive_income_ratio = dec!(0.80);
        let result_high = analyze_economic_substance(&high_passive).unwrap();

        assert!(result_high.risk_of_treaty_denial >= result_low.risk_of_treaty_denial);
    }

    #[test]
    fn test_new_entity_higher_risk() {
        let mut old_entity = base_input();
        old_entity.years_established = 10;
        // Reduce substance to get nonzero risk
        old_entity.ciga_performed_locally = false;
        old_entity.outsourced_ciga = false;
        let result_old = analyze_economic_substance(&old_entity).unwrap();

        let mut new_entity = old_entity.clone();
        new_entity.years_established = 1;
        let result_new = analyze_economic_substance(&new_entity).unwrap();

        assert!(result_new.risk_of_treaty_denial >= result_old.risk_of_treaty_denial);
    }

    // ------ Warnings tests ------

    #[test]
    fn test_warning_passive_income_above_75() {
        let mut input = base_input();
        input.passive_income_ratio = dec!(0.80);
        let result = analyze_economic_substance(&input).unwrap();
        assert!(result.warnings.iter().any(|w| w.contains("ATAD III")));
    }

    #[test]
    fn test_warning_ip_holding_outsourced_ciga() {
        let mut input = base_input();
        input.entity_type = EntityType::IPHolding;
        input.outsourced_ciga = true;
        let result = analyze_economic_substance(&input).unwrap();
        assert!(result.warnings.iter().any(|w| w.contains("IP holding")));
    }

    #[test]
    fn test_warning_critically_low_score() {
        let mut input = base_input();
        input.local_employees = 0;
        input.local_qualified_directors = 0;
        input.total_directors = 0;
        input.has_local_premises = false;
        input.premises_type = PremisesType::None;
        input.board_meetings_in_jurisdiction = 0;
        input.total_board_meetings = 0;
        input.annual_operating_expenditure = Decimal::ZERO;
        input.local_expenditure = Decimal::ZERO;
        input.ciga_performed_locally = false;
        input.outsourced_ciga = false;
        let result = analyze_economic_substance(&input).unwrap();
        assert!(result.warnings.iter().any(|w| w.contains("critically low")));
    }

    #[test]
    fn test_warning_new_entity() {
        let mut input = base_input();
        input.years_established = 0;
        let result = analyze_economic_substance(&input).unwrap();
        assert!(result
            .warnings
            .iter()
            .any(|w| w.contains("Newly incorporated")));
    }

    #[test]
    fn test_warning_no_board_meetings() {
        let mut input = base_input();
        input.board_meetings_in_jurisdiction = 0;
        input.total_board_meetings = 0;
        let result = analyze_economic_substance(&input).unwrap();
        assert!(result
            .warnings
            .iter()
            .any(|w| w.contains("No board meetings")));
    }

    #[test]
    fn test_warning_no_directors() {
        let mut input = base_input();
        input.total_directors = 0;
        input.local_qualified_directors = 0;
        let result = analyze_economic_substance(&input).unwrap();
        assert!(result.warnings.iter().any(|w| w.contains("No directors")));
    }

    // ------ Remediation tests ------

    #[test]
    fn test_remediation_for_no_employees() {
        let mut input = base_input();
        input.local_employees = 0;
        let result = analyze_economic_substance(&input).unwrap();
        assert!(result
            .remediation_recommendations
            .iter()
            .any(|r| r.contains("Hire")));
    }

    #[test]
    fn test_remediation_for_no_premises() {
        let mut input = base_input();
        input.has_local_premises = false;
        let result = analyze_economic_substance(&input).unwrap();
        assert!(result
            .remediation_recommendations
            .iter()
            .any(|r| r.contains("Secure dedicated office")));
    }

    #[test]
    fn test_remediation_for_outsourced_ip_ciga() {
        let mut input = base_input();
        input.entity_type = EntityType::IPHolding;
        input.outsourced_ciga = true;
        let result = analyze_economic_substance(&input).unwrap();
        assert!(result
            .remediation_recommendations
            .iter()
            .any(|r| r.contains("insource")));
    }

    #[test]
    fn test_no_remediation_when_compliant() {
        let input = base_input();
        let result = analyze_economic_substance(&input).unwrap();
        // Should have the "no immediate remediation" message or be empty
        assert!(
            result.remediation_recommendations.len() == 1
                && result.remediation_recommendations[0].contains("No immediate"),
            "Expected no immediate remediation, got: {:?}",
            result.remediation_recommendations
        );
    }

    // ------ Metadata tests ------

    #[test]
    fn test_methodology_non_empty() {
        let input = base_input();
        let result = analyze_economic_substance(&input).unwrap();
        assert!(!result.methodology.is_empty());
    }

    #[test]
    fn test_assumptions_non_empty() {
        let input = base_input();
        let result = analyze_economic_substance(&input).unwrap();
        assert!(!result.assumptions.is_empty());
    }

    // ------ Entity type scoring behavior tests ------

    #[test]
    fn test_ip_holding_capped_personnel_score() {
        let mut input = base_input();
        input.entity_type = EntityType::IPHolding;
        input.local_employees = 2; // Below IP threshold of 3
        let result = analyze_economic_substance(&input).unwrap();
        assert!(result.score_breakdown.personnel <= dec!(10));
    }

    #[test]
    fn test_ip_holding_adequate_employees_full_score() {
        let mut input = base_input();
        input.entity_type = EntityType::IPHolding;
        input.local_employees = 5;
        let result = analyze_economic_substance(&input).unwrap();
        assert!(result.score_breakdown.personnel >= dec!(20));
    }

    // ------ Edge cases ------

    #[test]
    fn test_boundary_passive_income_exactly_75() {
        let mut input = base_input();
        input.passive_income_ratio = dec!(0.75);
        let result = analyze_economic_substance(&input).unwrap();
        // Exactly 75% should not trigger the >75% gap
        assert!(!result
            .gaps_identified
            .iter()
            .any(|g| g.contains("Passive income ratio")));
    }

    #[test]
    fn test_boundary_passive_income_exactly_zero() {
        let mut input = base_input();
        input.passive_income_ratio = Decimal::ZERO;
        let result = analyze_economic_substance(&input).unwrap();
        // Validate no passive income gap at zero ratio
        assert!(!result
            .gaps_identified
            .iter()
            .any(|g| g.contains("Passive income")));
    }

    #[test]
    fn test_boundary_passive_income_exactly_one() {
        let mut input = base_input();
        input.passive_income_ratio = Decimal::ONE;
        let result = analyze_economic_substance(&input).unwrap();
        assert!(result
            .gaps_identified
            .iter()
            .any(|g| g.contains("Passive income ratio")));
    }

    #[test]
    fn test_all_entity_types_succeed() {
        let entity_types = vec![
            EntityType::HoldingCompany,
            EntityType::IPHolding,
            EntityType::FinanceLease,
            EntityType::FundManagement,
            EntityType::Banking,
            EntityType::Insurance,
            EntityType::HQ,
            EntityType::ServiceCentre,
            EntityType::PureEquityHolding,
        ];
        for et in entity_types {
            let mut input = base_input();
            input.entity_type = et.clone();
            let result = analyze_economic_substance(&input);
            assert!(result.is_ok(), "Failed for entity type {:?}", et);
        }
    }

    #[test]
    fn test_serialization_roundtrip() {
        let input = base_input();
        let result = analyze_economic_substance(&input).unwrap();
        let json = serde_json::to_string(&result).unwrap();
        let deserialized: EconomicSubstanceOutput = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.substance_score, result.substance_score);
    }

    #[test]
    fn test_expenditure_50_percent_ratio() {
        let mut input = base_input();
        input.local_expenditure = dec!(250_000);
        input.annual_operating_expenditure = dec!(500_000);
        let result = analyze_economic_substance(&input).unwrap();
        assert_eq!(result.score_breakdown.expenditure, dec!(10));
    }

    #[test]
    fn test_expenditure_25_percent_ratio() {
        let mut input = base_input();
        input.local_expenditure = dec!(125_000);
        input.annual_operating_expenditure = dec!(500_000);
        let result = analyze_economic_substance(&input).unwrap();
        assert_eq!(result.score_breakdown.expenditure, dec!(5));
    }

    #[test]
    fn test_guernsey_requirements() {
        let mut input = base_input();
        input.jurisdiction = "Guernsey".to_string();
        let result = analyze_economic_substance(&input).unwrap();
        assert!(result
            .jurisdiction_requirements
            .iter()
            .any(|r| r.contains("Guernsey")));
    }
}
