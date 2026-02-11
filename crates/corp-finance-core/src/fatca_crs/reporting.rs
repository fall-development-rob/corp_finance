use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

use crate::error::CorpFinanceError;
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum IgaModel {
    Model1,
    Model2,
    NonIGA,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AccountType {
    Depository,
    Custodial,
    EquityInterest,
    DebtInterest,
    CashValueInsurance,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DueDiligenceLevel {
    Simplified,
    Standard,
    Enhanced,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CrsApproach {
    Wider,
    Narrower,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FatcaStatus {
    pub compliant: bool,
    pub withholding_risk_pct: Decimal,
    pub reporting_obligations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrsStatus {
    pub reportable_jurisdictions: Vec<String>,
    pub due_diligence_level: DueDiligenceLevel,
    pub approach: CrsApproach,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportingDeadline {
    pub jurisdiction: String,
    pub framework: String,
    pub deadline_description: String,
    pub reporting_year: i32,
}

// ---------------------------------------------------------------------------
// Input / Output
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FatcaCrsReportingInput {
    pub institution_name: String,
    pub jurisdiction: String,
    pub iga_model: IgaModel,
    pub account_count: u64,
    pub aggregate_balance_usd: Decimal,
    pub account_types: Vec<AccountType>,
    pub us_indicia_found: u32,
    pub has_giin: bool,
    pub crs_participating: bool,
    pub crs_jurisdictions: Vec<String>,
    pub reporting_year: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FatcaCrsReportingOutput {
    pub fatca_status: FatcaStatus,
    pub crs_status: CrsStatus,
    pub compliance_score: Decimal,
    pub risk_level: RiskLevel,
    pub withholding_exposure_usd: Decimal,
    pub reporting_deadlines: Vec<ReportingDeadline>,
    pub remediation_items: Vec<String>,
    pub methodology: String,
    pub assumptions: Vec<String>,
    pub warnings: Vec<String>,
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const FATCA_WITHHOLDING_RATE: Decimal = dec!(0.30);
const FATCA_INDIVIDUAL_THRESHOLD: Decimal = dec!(50_000);
const FATCA_ENTITY_THRESHOLD: Decimal = dec!(250_000);

/// Wider-approach CRS jurisdictions (example set for analysis purposes).
const WIDER_APPROACH_JURISDICTIONS: &[&str] = &[
    "GB", "DE", "FR", "AU", "CA", "JP", "NL", "IE", "LU", "SG", "HK", "NZ", "NO", "SE", "DK", "FI",
    "IT", "ES", "BE", "AT", "PT", "CZ", "PL",
];

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Analyze FATCA and CRS reporting obligations for a financial institution.
///
/// Evaluates compliance readiness across due diligence, reporting, and
/// withholding dimensions. Returns a comprehensive assessment including
/// compliance score (0-100), risk level, withholding exposure, deadlines,
/// and remediation items.
pub fn analyze_fatca_crs_reporting(
    input: &FatcaCrsReportingInput,
) -> CorpFinanceResult<FatcaCrsReportingOutput> {
    validate_reporting_input(input)?;

    let mut warnings: Vec<String> = Vec::new();
    let mut remediation_items: Vec<String> = Vec::new();

    // ------------------------------------------------------------------
    // 1. FATCA Status
    // ------------------------------------------------------------------
    let fatca_status = assess_fatca_status(input, &mut warnings, &mut remediation_items);

    // ------------------------------------------------------------------
    // 2. CRS Status
    // ------------------------------------------------------------------
    let crs_status = assess_crs_status(input, &mut warnings, &mut remediation_items);

    // ------------------------------------------------------------------
    // 3. Compliance Score (0-100)
    // ------------------------------------------------------------------
    let compliance_score = calculate_compliance_score(input, &fatca_status, &crs_status);

    // ------------------------------------------------------------------
    // 4. Risk Level
    // ------------------------------------------------------------------
    let risk_level = classify_risk(compliance_score, input);

    // ------------------------------------------------------------------
    // 5. Withholding Exposure
    // ------------------------------------------------------------------
    let withholding_exposure_usd = calculate_withholding_exposure(input, &fatca_status);

    if withholding_exposure_usd > Decimal::ZERO {
        warnings.push(format!(
            "Potential FATCA withholding exposure of ${} identified.",
            withholding_exposure_usd
        ));
    }

    // ------------------------------------------------------------------
    // 6. Reporting Deadlines
    // ------------------------------------------------------------------
    let reporting_deadlines = build_reporting_deadlines(input);

    // ------------------------------------------------------------------
    // 7. Assemble Output
    // ------------------------------------------------------------------
    let assumptions = vec![
        "FATCA withholding rate assumed at 30% on FDAP income for non-compliant accounts.".into(),
        format!(
            "FATCA individual reporting threshold: ${}, entity threshold: ${}.",
            FATCA_INDIVIDUAL_THRESHOLD, FATCA_ENTITY_THRESHOLD
        ),
        "CRS approach determined by institution jurisdiction.".into(),
        "Compliance scoring weights: GIIN registration 25%, due diligence 25%, \
         reporting readiness 25%, withholding controls 25%."
            .into(),
    ];

    let methodology = "FATCA/CRS Reporting Analysis: assess compliance across IGA model, \
        GIIN registration, due diligence procedures, CRS participation, and \
        withholding obligations per US IRC Chapter 4 and OECD CRS framework."
        .to_string();

    Ok(FatcaCrsReportingOutput {
        fatca_status,
        crs_status,
        compliance_score,
        risk_level,
        withholding_exposure_usd,
        reporting_deadlines,
        remediation_items,
        methodology,
        assumptions,
        warnings,
    })
}

// ---------------------------------------------------------------------------
// FATCA Assessment
// ---------------------------------------------------------------------------

fn assess_fatca_status(
    input: &FatcaCrsReportingInput,
    warnings: &mut Vec<String>,
    remediation: &mut Vec<String>,
) -> FatcaStatus {
    let mut obligations: Vec<String> = Vec::new();
    let mut compliant = true;
    let mut withholding_risk_pct = Decimal::ZERO;

    // GIIN registration check
    if !input.has_giin {
        compliant = false;
        withholding_risk_pct = FATCA_WITHHOLDING_RATE;
        remediation.push(
            "Register for a GIIN (Global Intermediary Identification Number) with the IRS.".into(),
        );
        warnings.push(
            "Missing GIIN registration exposes the institution to 30% FATCA withholding.".into(),
        );
    }

    // IGA-specific obligations
    match input.iga_model {
        IgaModel::Model1 => {
            obligations.push(
                "Report US-reportable accounts to local tax authority for exchange with IRS."
                    .into(),
            );
            obligations.push("Perform due diligence to identify US indicia in accounts.".into());
        }
        IgaModel::Model2 => {
            obligations.push("Report directly to the IRS with local authority consent.".into());
            obligations.push("Register with IRS and maintain GIIN.".into());
            obligations.push("Perform due diligence to identify US indicia in accounts.".into());
        }
        IgaModel::NonIGA => {
            obligations.push("Register directly with IRS as a Participating FFI.".into());
            obligations
                .push("Withhold 30% on FDAP payments to non-compliant account holders.".into());
            obligations.push("Report all US-reportable accounts directly to the IRS.".into());
            if !input.has_giin {
                withholding_risk_pct = FATCA_WITHHOLDING_RATE;
            }
        }
    }

    // US indicia check
    if input.us_indicia_found > 0 {
        obligations.push(format!(
            "Resolve {} US indicia found across accounts (US birthplace, address, phone, \
             standing instructions, or power of attorney).",
            input.us_indicia_found
        ));
        if !input.has_giin {
            remediation.push(format!(
                "Investigate and document {} US indicia before next reporting deadline.",
                input.us_indicia_found
            ));
        }
    }

    // Account type obligations
    if input
        .account_types
        .contains(&AccountType::CashValueInsurance)
    {
        obligations.push("Cash value insurance contracts require Annex II review.".into());
    }
    if input.account_types.contains(&AccountType::EquityInterest)
        || input.account_types.contains(&AccountType::DebtInterest)
    {
        obligations.push(
            "Equity/debt interests require investment entity classification analysis.".into(),
        );
    }

    // Threshold-based reporting
    if input.aggregate_balance_usd > FATCA_ENTITY_THRESHOLD {
        obligations.push(
            "Aggregate balance exceeds entity threshold ($250,000); full reporting required."
                .into(),
        );
    } else if input.aggregate_balance_usd > FATCA_INDIVIDUAL_THRESHOLD {
        obligations.push(
            "Aggregate balance exceeds individual threshold ($50,000); reporting required.".into(),
        );
    }

    // Non-IGA without GIIN is non-compliant
    if input.iga_model == IgaModel::NonIGA && !input.has_giin {
        compliant = false;
        remediation.push(
            "Non-IGA jurisdiction without GIIN: register as Participating FFI immediately.".into(),
        );
    }

    FatcaStatus {
        compliant,
        withholding_risk_pct,
        reporting_obligations: obligations,
    }
}

// ---------------------------------------------------------------------------
// CRS Assessment
// ---------------------------------------------------------------------------

fn assess_crs_status(
    input: &FatcaCrsReportingInput,
    warnings: &mut Vec<String>,
    remediation: &mut Vec<String>,
) -> CrsStatus {
    let mut reportable_jurisdictions: Vec<String> = Vec::new();

    if !input.crs_participating {
        warnings.push(
            "Institution is not CRS-participating; no CRS reporting obligations apply.".into(),
        );
        return CrsStatus {
            reportable_jurisdictions,
            due_diligence_level: DueDiligenceLevel::Simplified,
            approach: CrsApproach::Narrower,
        };
    }

    // Reportable jurisdictions = CRS partner jurisdictions
    reportable_jurisdictions = input.crs_jurisdictions.clone();

    if reportable_jurisdictions.is_empty() {
        remediation.push(
            "CRS participating but no reportable jurisdictions specified. \
             Identify all CRS partner jurisdictions for reporting."
                .into(),
        );
    }

    // Due diligence level based on account volume and balance
    let due_diligence_level =
        if input.aggregate_balance_usd > dec!(1_000_000) || input.account_count > 1000 {
            DueDiligenceLevel::Enhanced
        } else if input.aggregate_balance_usd > dec!(250_000) || input.account_count > 100 {
            DueDiligenceLevel::Standard
        } else {
            DueDiligenceLevel::Simplified
        };

    // Wider vs narrower approach based on institution jurisdiction
    let approach =
        if WIDER_APPROACH_JURISDICTIONS.contains(&input.jurisdiction.to_uppercase().as_str()) {
            CrsApproach::Wider
        } else {
            CrsApproach::Narrower
        };

    if approach == CrsApproach::Wider {
        warnings.push(
            "Wider-approach jurisdiction: must report on all non-resident account holders, \
             not only those in CRS-partner jurisdictions."
                .into(),
        );
    }

    CrsStatus {
        reportable_jurisdictions,
        due_diligence_level,
        approach,
    }
}

// ---------------------------------------------------------------------------
// Compliance Scoring
// ---------------------------------------------------------------------------

/// Calculate compliance score on 0-100 scale.
/// Four equally-weighted dimensions (25 pts each):
///   1. GIIN/Registration readiness
///   2. Due diligence procedures
///   3. Reporting readiness
///   4. Withholding controls
fn calculate_compliance_score(
    input: &FatcaCrsReportingInput,
    fatca: &FatcaStatus,
    crs: &CrsStatus,
) -> Decimal {
    let mut score = Decimal::ZERO;

    // Dimension 1: GIIN / Registration (25 pts)
    if input.has_giin {
        score += dec!(25);
    } else {
        // Partial credit if IGA Model 1 (local authority handles some registration)
        if input.iga_model == IgaModel::Model1 {
            score += dec!(10);
        }
    }

    // Dimension 2: Due Diligence (25 pts)
    // Credit for having identified US indicia (means DD is happening)
    let dd_score = if input.account_count == 0 {
        dec!(25) // No accounts = no DD needed = full marks
    } else {
        let mut pts = dec!(10); // base credit for operating
                                // Enhanced DD gets more credit
        match crs.due_diligence_level {
            DueDiligenceLevel::Enhanced => pts += dec!(10),
            DueDiligenceLevel::Standard => pts += dec!(7),
            DueDiligenceLevel::Simplified => pts += dec!(5),
        }
        // Having found and reported indicia = active monitoring
        if input.us_indicia_found > 0 {
            pts += dec!(5);
        }
        pts.min(dec!(25))
    };
    score += dd_score;

    // Dimension 3: Reporting Readiness (25 pts)
    let reporting_score = {
        let mut pts = Decimal::ZERO;
        // CRS participation
        if input.crs_participating {
            pts += dec!(10);
        }
        // Has jurisdictions identified
        if !input.crs_jurisdictions.is_empty() {
            pts += dec!(5);
        }
        // FATCA reporting obligations identified
        if !fatca.reporting_obligations.is_empty() {
            pts += dec!(5);
        }
        // Account types identified (data completeness)
        if !input.account_types.is_empty() {
            pts += dec!(5);
        }
        pts.min(dec!(25))
    };
    score += reporting_score;

    // Dimension 4: Withholding Controls (25 pts)
    let withholding_score = if fatca.withholding_risk_pct == Decimal::ZERO {
        dec!(25)
    } else if fatca.withholding_risk_pct <= dec!(0.15) {
        dec!(15)
    } else {
        dec!(5) // High withholding risk = low score
    };
    score += withholding_score;

    score.min(dec!(100))
}

// ---------------------------------------------------------------------------
// Risk Classification
// ---------------------------------------------------------------------------

fn classify_risk(compliance_score: Decimal, input: &FatcaCrsReportingInput) -> RiskLevel {
    // Base risk from compliance score
    let base_risk = if compliance_score >= dec!(80) {
        RiskLevel::Low
    } else if compliance_score >= dec!(60) {
        RiskLevel::Medium
    } else if compliance_score >= dec!(40) {
        RiskLevel::High
    } else {
        RiskLevel::Critical
    };

    // Escalate for critical factors
    if !input.has_giin && input.iga_model == IgaModel::NonIGA {
        return RiskLevel::Critical;
    }
    if !input.has_giin && input.us_indicia_found > 10 {
        return match base_risk {
            RiskLevel::Low => RiskLevel::Medium,
            RiskLevel::Medium => RiskLevel::High,
            _ => RiskLevel::Critical,
        };
    }

    base_risk
}

// ---------------------------------------------------------------------------
// Withholding Exposure
// ---------------------------------------------------------------------------

fn calculate_withholding_exposure(input: &FatcaCrsReportingInput, fatca: &FatcaStatus) -> Decimal {
    if fatca.compliant {
        return Decimal::ZERO;
    }

    // Exposure = aggregate balance * withholding rate * proportion of accounts with indicia
    let indicia_proportion = if input.account_count > 0 {
        Decimal::from(input.us_indicia_found) / Decimal::from(input.account_count)
    } else {
        Decimal::ZERO
    };

    // Cap proportion at 1.0 (indicia count can exceed account count in edge cases)
    let capped_proportion = indicia_proportion.min(Decimal::ONE);

    input.aggregate_balance_usd * FATCA_WITHHOLDING_RATE * capped_proportion
}

// ---------------------------------------------------------------------------
// Reporting Deadlines
// ---------------------------------------------------------------------------

fn build_reporting_deadlines(input: &FatcaCrsReportingInput) -> Vec<ReportingDeadline> {
    let mut deadlines: Vec<ReportingDeadline> = Vec::new();
    let year = input.reporting_year;

    // FATCA deadline
    match input.iga_model {
        IgaModel::Model1 => {
            deadlines.push(ReportingDeadline {
                jurisdiction: input.jurisdiction.clone(),
                framework: "FATCA (IGA Model 1)".into(),
                deadline_description: format!(
                    "Report to local tax authority by September 30, {} \
                     (local authority exchanges with IRS).",
                    year + 1
                ),
                reporting_year: year,
            });
        }
        IgaModel::Model2 => {
            deadlines.push(ReportingDeadline {
                jurisdiction: input.jurisdiction.clone(),
                framework: "FATCA (IGA Model 2)".into(),
                deadline_description: format!("Report directly to IRS by March 31, {}.", year + 1),
                reporting_year: year,
            });
        }
        IgaModel::NonIGA => {
            deadlines.push(ReportingDeadline {
                jurisdiction: input.jurisdiction.clone(),
                framework: "FATCA (Non-IGA)".into(),
                deadline_description: format!(
                    "Report directly to IRS by March 31, {}. \
                     Withholding obligations ongoing throughout the year.",
                    year + 1
                ),
                reporting_year: year,
            });
        }
    }

    // CRS deadlines
    if input.crs_participating {
        deadlines.push(ReportingDeadline {
            jurisdiction: input.jurisdiction.clone(),
            framework: "CRS".into(),
            deadline_description: format!(
                "CRS reporting to local competent authority typically due by \
                 June 30 to September 30, {} (varies by jurisdiction).",
                year + 1
            ),
            reporting_year: year,
        });
    }

    deadlines
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

fn validate_reporting_input(input: &FatcaCrsReportingInput) -> CorpFinanceResult<()> {
    if input.institution_name.trim().is_empty() {
        return Err(CorpFinanceError::InvalidInput {
            field: "institution_name".into(),
            reason: "Institution name cannot be empty".into(),
        });
    }
    if input.jurisdiction.trim().is_empty() {
        return Err(CorpFinanceError::InvalidInput {
            field: "jurisdiction".into(),
            reason: "Jurisdiction cannot be empty".into(),
        });
    }
    if input.aggregate_balance_usd < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "aggregate_balance_usd".into(),
            reason: "Aggregate balance cannot be negative".into(),
        });
    }
    if input.reporting_year < 2010 || input.reporting_year > 2100 {
        return Err(CorpFinanceError::InvalidInput {
            field: "reporting_year".into(),
            reason: "Reporting year must be between 2010 and 2100".into(),
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

    // ---------------------------------------------------------------
    // Test Helpers
    // ---------------------------------------------------------------

    fn base_input() -> FatcaCrsReportingInput {
        FatcaCrsReportingInput {
            institution_name: "Test Bank AG".into(),
            jurisdiction: "DE".into(),
            iga_model: IgaModel::Model1,
            account_count: 500,
            aggregate_balance_usd: dec!(100_000_000),
            account_types: vec![AccountType::Depository, AccountType::Custodial],
            us_indicia_found: 5,
            has_giin: true,
            crs_participating: true,
            crs_jurisdictions: vec!["US".into(), "GB".into(), "FR".into()],
            reporting_year: 2025,
        }
    }

    // ---------------------------------------------------------------
    // IGA Model Tests
    // ---------------------------------------------------------------

    #[test]
    fn test_iga_model1_compliant_with_giin() {
        let input = base_input();
        let result = analyze_fatca_crs_reporting(&input).unwrap();

        assert!(result.fatca_status.compliant);
        assert_eq!(result.fatca_status.withholding_risk_pct, Decimal::ZERO);
        assert!(result
            .fatca_status
            .reporting_obligations
            .iter()
            .any(|o| o.contains("local tax authority")));
    }

    #[test]
    fn test_iga_model2_reporting_directly_to_irs() {
        let mut input = base_input();
        input.iga_model = IgaModel::Model2;

        let result = analyze_fatca_crs_reporting(&input).unwrap();

        assert!(result.fatca_status.compliant);
        assert!(result
            .fatca_status
            .reporting_obligations
            .iter()
            .any(|o| o.contains("directly to the IRS")));
    }

    #[test]
    fn test_iga_model2_requires_giin() {
        let mut input = base_input();
        input.iga_model = IgaModel::Model2;

        let result = analyze_fatca_crs_reporting(&input).unwrap();

        assert!(result
            .fatca_status
            .reporting_obligations
            .iter()
            .any(|o| o.contains("GIIN")));
    }

    #[test]
    fn test_non_iga_withholding_obligation() {
        let mut input = base_input();
        input.iga_model = IgaModel::NonIGA;

        let result = analyze_fatca_crs_reporting(&input).unwrap();

        assert!(result
            .fatca_status
            .reporting_obligations
            .iter()
            .any(|o| o.contains("Withhold 30%")));
    }

    #[test]
    fn test_non_iga_without_giin_critical_risk() {
        let mut input = base_input();
        input.iga_model = IgaModel::NonIGA;
        input.has_giin = false;

        let result = analyze_fatca_crs_reporting(&input).unwrap();

        assert!(!result.fatca_status.compliant);
        assert_eq!(result.fatca_status.withholding_risk_pct, dec!(0.30));
        assert_eq!(result.risk_level, RiskLevel::Critical);
    }

    #[test]
    fn test_non_iga_with_giin_compliant() {
        let mut input = base_input();
        input.iga_model = IgaModel::NonIGA;
        input.has_giin = true;

        let result = analyze_fatca_crs_reporting(&input).unwrap();

        assert!(result.fatca_status.compliant);
    }

    // ---------------------------------------------------------------
    // CRS Tests
    // ---------------------------------------------------------------

    #[test]
    fn test_crs_wider_approach_for_de_jurisdiction() {
        let input = base_input(); // jurisdiction = "DE"
        let result = analyze_fatca_crs_reporting(&input).unwrap();

        assert_eq!(result.crs_status.approach, CrsApproach::Wider);
    }

    #[test]
    fn test_crs_narrower_approach_for_non_wider_jurisdiction() {
        let mut input = base_input();
        input.jurisdiction = "BM".into(); // Bermuda, not in wider list

        let result = analyze_fatca_crs_reporting(&input).unwrap();

        assert_eq!(result.crs_status.approach, CrsApproach::Narrower);
    }

    #[test]
    fn test_crs_not_participating_no_obligations() {
        let mut input = base_input();
        input.crs_participating = false;

        let result = analyze_fatca_crs_reporting(&input).unwrap();

        assert!(result.crs_status.reportable_jurisdictions.is_empty());
        assert_eq!(
            result.crs_status.due_diligence_level,
            DueDiligenceLevel::Simplified
        );
    }

    #[test]
    fn test_crs_participating_with_jurisdictions() {
        let input = base_input();
        let result = analyze_fatca_crs_reporting(&input).unwrap();

        assert_eq!(result.crs_status.reportable_jurisdictions.len(), 3);
        assert!(result
            .crs_status
            .reportable_jurisdictions
            .contains(&"US".to_string()));
    }

    #[test]
    fn test_crs_enhanced_dd_for_large_balance() {
        let mut input = base_input();
        input.aggregate_balance_usd = dec!(5_000_000);
        input.account_count = 50;

        let result = analyze_fatca_crs_reporting(&input).unwrap();

        assert_eq!(
            result.crs_status.due_diligence_level,
            DueDiligenceLevel::Enhanced
        );
    }

    #[test]
    fn test_crs_enhanced_dd_for_many_accounts() {
        let mut input = base_input();
        input.account_count = 2000;
        input.aggregate_balance_usd = dec!(100_000);

        let result = analyze_fatca_crs_reporting(&input).unwrap();

        assert_eq!(
            result.crs_status.due_diligence_level,
            DueDiligenceLevel::Enhanced
        );
    }

    #[test]
    fn test_crs_standard_dd_for_moderate_balance() {
        let mut input = base_input();
        input.aggregate_balance_usd = dec!(500_000);
        input.account_count = 50;

        let result = analyze_fatca_crs_reporting(&input).unwrap();

        assert_eq!(
            result.crs_status.due_diligence_level,
            DueDiligenceLevel::Standard
        );
    }

    #[test]
    fn test_crs_simplified_dd_for_small_balance_few_accounts() {
        let mut input = base_input();
        input.aggregate_balance_usd = dec!(100_000);
        input.account_count = 50;

        let result = analyze_fatca_crs_reporting(&input).unwrap();

        assert_eq!(
            result.crs_status.due_diligence_level,
            DueDiligenceLevel::Simplified
        );
    }

    #[test]
    fn test_crs_wider_approach_warning_generated() {
        let input = base_input(); // DE = wider approach
        let result = analyze_fatca_crs_reporting(&input).unwrap();

        assert!(result.warnings.iter().any(|w| w.contains("Wider-approach")));
    }

    #[test]
    fn test_crs_empty_jurisdictions_remediation() {
        let mut input = base_input();
        input.crs_jurisdictions = vec![];

        let result = analyze_fatca_crs_reporting(&input).unwrap();

        assert!(result
            .remediation_items
            .iter()
            .any(|r| r.contains("CRS partner jurisdictions")));
    }

    // ---------------------------------------------------------------
    // Compliance Score Tests
    // ---------------------------------------------------------------

    #[test]
    fn test_compliance_score_fully_compliant() {
        let input = base_input();
        let result = analyze_fatca_crs_reporting(&input).unwrap();

        assert!(
            result.compliance_score >= dec!(70),
            "Fully compliant institution should score >= 70, got {}",
            result.compliance_score
        );
    }

    #[test]
    fn test_compliance_score_no_giin_lower() {
        let with_giin = {
            let input = base_input();
            analyze_fatca_crs_reporting(&input)
                .unwrap()
                .compliance_score
        };
        let without_giin = {
            let mut input = base_input();
            input.has_giin = false;
            analyze_fatca_crs_reporting(&input)
                .unwrap()
                .compliance_score
        };

        assert!(
            without_giin < with_giin,
            "Score without GIIN ({}) should be lower than with GIIN ({})",
            without_giin,
            with_giin
        );
    }

    #[test]
    fn test_compliance_score_model1_partial_credit_no_giin() {
        let mut input = base_input();
        input.has_giin = false;
        input.iga_model = IgaModel::Model1;

        let model1_result = analyze_fatca_crs_reporting(&input).unwrap();

        input.iga_model = IgaModel::Model2;
        let model2_result = analyze_fatca_crs_reporting(&input).unwrap();

        assert!(
            model1_result.compliance_score > model2_result.compliance_score,
            "Model 1 without GIIN ({}) should score higher than Model 2 without GIIN ({}) \
             due to partial credit",
            model1_result.compliance_score,
            model2_result.compliance_score
        );
    }

    #[test]
    fn test_compliance_score_zero_accounts_full_dd_credit() {
        let mut input = base_input();
        input.account_count = 0;
        input.aggregate_balance_usd = Decimal::ZERO;
        input.us_indicia_found = 0;

        let result = analyze_fatca_crs_reporting(&input).unwrap();

        // With no accounts, DD dimension should give full 25 points
        assert!(
            result.compliance_score >= dec!(70),
            "Zero accounts should get full DD credit, got score {}",
            result.compliance_score
        );
    }

    #[test]
    fn test_compliance_score_not_crs_participating_lower() {
        let participating = {
            let input = base_input();
            analyze_fatca_crs_reporting(&input)
                .unwrap()
                .compliance_score
        };
        let not_participating = {
            let mut input = base_input();
            input.crs_participating = false;
            analyze_fatca_crs_reporting(&input)
                .unwrap()
                .compliance_score
        };

        assert!(
            not_participating < participating,
            "Non-CRS score ({}) should be lower than CRS score ({})",
            not_participating,
            participating
        );
    }

    #[test]
    fn test_compliance_score_capped_at_100() {
        let input = base_input();
        let result = analyze_fatca_crs_reporting(&input).unwrap();

        assert!(result.compliance_score <= dec!(100));
    }

    #[test]
    fn test_compliance_score_minimum_zero() {
        // Even worst case should not go below 0
        let mut input = base_input();
        input.has_giin = false;
        input.crs_participating = false;
        input.crs_jurisdictions = vec![];
        input.account_types = vec![];

        let result = analyze_fatca_crs_reporting(&input).unwrap();

        assert!(result.compliance_score >= Decimal::ZERO);
    }

    // ---------------------------------------------------------------
    // Withholding Calculation Tests
    // ---------------------------------------------------------------

    #[test]
    fn test_withholding_exposure_zero_when_compliant() {
        let input = base_input();
        let result = analyze_fatca_crs_reporting(&input).unwrap();

        assert_eq!(result.withholding_exposure_usd, Decimal::ZERO);
    }

    #[test]
    fn test_withholding_exposure_nonzero_when_non_compliant() {
        let mut input = base_input();
        input.has_giin = false;

        let result = analyze_fatca_crs_reporting(&input).unwrap();

        assert!(result.withholding_exposure_usd > Decimal::ZERO);
    }

    #[test]
    fn test_withholding_exposure_proportional_to_indicia() {
        let mut input = base_input();
        input.has_giin = false;
        input.us_indicia_found = 10;
        input.account_count = 100;

        let result = analyze_fatca_crs_reporting(&input).unwrap();

        // 10/100 = 10% of accounts, 30% rate, $100M balance
        // Exposure = 100M * 0.30 * 0.10 = $3,000,000
        let expected = dec!(100_000_000) * dec!(0.30) * dec!(0.10);
        assert_eq!(result.withholding_exposure_usd, expected);
    }

    #[test]
    fn test_withholding_exposure_capped_at_full_balance() {
        let mut input = base_input();
        input.has_giin = false;
        input.us_indicia_found = 200; // More indicia than accounts
        input.account_count = 100;
        input.aggregate_balance_usd = dec!(1_000_000);

        let result = analyze_fatca_crs_reporting(&input).unwrap();

        // Proportion capped at 1.0
        let max_exposure = dec!(1_000_000) * dec!(0.30);
        assert_eq!(result.withholding_exposure_usd, max_exposure);
    }

    #[test]
    fn test_withholding_exposure_zero_balance() {
        let mut input = base_input();
        input.has_giin = false;
        input.aggregate_balance_usd = Decimal::ZERO;

        let result = analyze_fatca_crs_reporting(&input).unwrap();

        assert_eq!(result.withholding_exposure_usd, Decimal::ZERO);
    }

    #[test]
    fn test_withholding_exposure_zero_indicia() {
        let mut input = base_input();
        input.has_giin = false;
        input.us_indicia_found = 0;

        let result = analyze_fatca_crs_reporting(&input).unwrap();

        assert_eq!(result.withholding_exposure_usd, Decimal::ZERO);
    }

    #[test]
    fn test_withholding_exposure_zero_accounts() {
        let mut input = base_input();
        input.has_giin = false;
        input.account_count = 0;
        input.us_indicia_found = 5;

        let result = analyze_fatca_crs_reporting(&input).unwrap();

        assert_eq!(result.withholding_exposure_usd, Decimal::ZERO);
    }

    // ---------------------------------------------------------------
    // Risk Level Tests
    // ---------------------------------------------------------------

    #[test]
    fn test_risk_level_low_for_compliant() {
        let input = base_input();
        let result = analyze_fatca_crs_reporting(&input).unwrap();

        assert_eq!(result.risk_level, RiskLevel::Low);
    }

    #[test]
    fn test_risk_level_critical_non_iga_no_giin() {
        let mut input = base_input();
        input.iga_model = IgaModel::NonIGA;
        input.has_giin = false;

        let result = analyze_fatca_crs_reporting(&input).unwrap();

        assert_eq!(result.risk_level, RiskLevel::Critical);
    }

    #[test]
    fn test_risk_level_escalated_for_many_indicia_no_giin() {
        let mut input = base_input();
        input.has_giin = false;
        input.us_indicia_found = 15;
        input.iga_model = IgaModel::Model1;

        let result = analyze_fatca_crs_reporting(&input).unwrap();

        // Should be escalated from base level
        assert!(
            result.risk_level == RiskLevel::High || result.risk_level == RiskLevel::Critical,
            "Risk should be escalated for many indicia without GIIN, got {:?}",
            result.risk_level
        );
    }

    #[test]
    fn test_risk_level_medium_moderate_compliance() {
        let mut input = base_input();
        input.has_giin = false;
        input.iga_model = IgaModel::Model1; // partial GIIN credit
        input.us_indicia_found = 3;

        let result = analyze_fatca_crs_reporting(&input).unwrap();

        // Should be Medium or High (not Low, not Critical for Model1)
        assert!(
            result.risk_level == RiskLevel::Medium || result.risk_level == RiskLevel::High,
            "Expected Medium or High risk, got {:?}",
            result.risk_level
        );
    }

    // ---------------------------------------------------------------
    // Missing GIIN Tests
    // ---------------------------------------------------------------

    #[test]
    fn test_missing_giin_remediation_item() {
        let mut input = base_input();
        input.has_giin = false;

        let result = analyze_fatca_crs_reporting(&input).unwrap();

        assert!(result.remediation_items.iter().any(|r| r.contains("GIIN")));
    }

    #[test]
    fn test_missing_giin_warning() {
        let mut input = base_input();
        input.has_giin = false;

        let result = analyze_fatca_crs_reporting(&input).unwrap();

        assert!(result
            .warnings
            .iter()
            .any(|w| w.contains("GIIN") || w.contains("withholding")));
    }

    #[test]
    fn test_missing_giin_withholding_risk_30pct() {
        let mut input = base_input();
        input.has_giin = false;

        let result = analyze_fatca_crs_reporting(&input).unwrap();

        assert_eq!(result.fatca_status.withholding_risk_pct, dec!(0.30));
    }

    // ---------------------------------------------------------------
    // Account Type Tests
    // ---------------------------------------------------------------

    #[test]
    fn test_cash_value_insurance_obligation() {
        let mut input = base_input();
        input.account_types = vec![AccountType::CashValueInsurance];

        let result = analyze_fatca_crs_reporting(&input).unwrap();

        assert!(result
            .fatca_status
            .reporting_obligations
            .iter()
            .any(|o| o.contains("Cash value insurance")));
    }

    #[test]
    fn test_equity_debt_interest_obligation() {
        let mut input = base_input();
        input.account_types = vec![AccountType::EquityInterest];

        let result = analyze_fatca_crs_reporting(&input).unwrap();

        assert!(result
            .fatca_status
            .reporting_obligations
            .iter()
            .any(|o| o.contains("investment entity classification")));
    }

    #[test]
    fn test_depository_custodial_no_special_obligations() {
        let mut input = base_input();
        input.account_types = vec![AccountType::Depository, AccountType::Custodial];

        let result = analyze_fatca_crs_reporting(&input).unwrap();

        // Should not trigger cash value or equity/debt obligations
        assert!(!result
            .fatca_status
            .reporting_obligations
            .iter()
            .any(|o| o.contains("Cash value insurance")));
        assert!(!result
            .fatca_status
            .reporting_obligations
            .iter()
            .any(|o| o.contains("investment entity classification")));
    }

    #[test]
    fn test_all_account_types_combined() {
        let mut input = base_input();
        input.account_types = vec![
            AccountType::Depository,
            AccountType::Custodial,
            AccountType::EquityInterest,
            AccountType::DebtInterest,
            AccountType::CashValueInsurance,
        ];

        let result = analyze_fatca_crs_reporting(&input).unwrap();

        assert!(result
            .fatca_status
            .reporting_obligations
            .iter()
            .any(|o| o.contains("Cash value insurance")));
        assert!(result
            .fatca_status
            .reporting_obligations
            .iter()
            .any(|o| o.contains("investment entity classification")));
    }

    // ---------------------------------------------------------------
    // Threshold Tests
    // ---------------------------------------------------------------

    #[test]
    fn test_balance_above_entity_threshold() {
        let mut input = base_input();
        input.aggregate_balance_usd = dec!(300_000);

        let result = analyze_fatca_crs_reporting(&input).unwrap();

        assert!(result
            .fatca_status
            .reporting_obligations
            .iter()
            .any(|o| o.contains("entity threshold")));
    }

    #[test]
    fn test_balance_above_individual_threshold_below_entity() {
        let mut input = base_input();
        input.aggregate_balance_usd = dec!(75_000);

        let result = analyze_fatca_crs_reporting(&input).unwrap();

        assert!(result
            .fatca_status
            .reporting_obligations
            .iter()
            .any(|o| o.contains("individual threshold")));
    }

    #[test]
    fn test_balance_below_individual_threshold() {
        let mut input = base_input();
        input.aggregate_balance_usd = dec!(30_000);

        let result = analyze_fatca_crs_reporting(&input).unwrap();

        // Should NOT have threshold-related obligation
        assert!(!result
            .fatca_status
            .reporting_obligations
            .iter()
            .any(|o| o.contains("threshold")));
    }

    #[test]
    fn test_balance_exactly_at_individual_threshold() {
        let mut input = base_input();
        input.aggregate_balance_usd = dec!(50_000);

        let result = analyze_fatca_crs_reporting(&input).unwrap();

        // $50k is not > $50k
        assert!(!result
            .fatca_status
            .reporting_obligations
            .iter()
            .any(|o| o.contains("threshold")));
    }

    #[test]
    fn test_balance_exactly_at_entity_threshold() {
        let mut input = base_input();
        input.aggregate_balance_usd = dec!(250_000);

        let result = analyze_fatca_crs_reporting(&input).unwrap();

        // $250k is not > $250k; should only trigger individual threshold
        assert!(result
            .fatca_status
            .reporting_obligations
            .iter()
            .any(|o| o.contains("individual threshold")));
        assert!(!result
            .fatca_status
            .reporting_obligations
            .iter()
            .any(|o| o.contains("entity threshold")));
    }

    // ---------------------------------------------------------------
    // Reporting Deadline Tests
    // ---------------------------------------------------------------

    #[test]
    fn test_model1_deadline_sept_30() {
        let input = base_input();
        let result = analyze_fatca_crs_reporting(&input).unwrap();

        assert!(result
            .reporting_deadlines
            .iter()
            .any(|d| d.framework.contains("Model 1")
                && d.deadline_description.contains("September 30")));
    }

    #[test]
    fn test_model2_deadline_march_31() {
        let mut input = base_input();
        input.iga_model = IgaModel::Model2;

        let result = analyze_fatca_crs_reporting(&input).unwrap();

        assert!(result.reporting_deadlines.iter().any(
            |d| d.framework.contains("Model 2") && d.deadline_description.contains("March 31")
        ));
    }

    #[test]
    fn test_non_iga_deadline_march_31() {
        let mut input = base_input();
        input.iga_model = IgaModel::NonIGA;

        let result = analyze_fatca_crs_reporting(&input).unwrap();

        assert!(result.reporting_deadlines.iter().any(
            |d| d.framework.contains("Non-IGA") && d.deadline_description.contains("March 31")
        ));
    }

    #[test]
    fn test_crs_deadline_generated() {
        let input = base_input();
        let result = analyze_fatca_crs_reporting(&input).unwrap();

        assert!(result
            .reporting_deadlines
            .iter()
            .any(|d| d.framework == "CRS"));
    }

    #[test]
    fn test_no_crs_deadline_when_not_participating() {
        let mut input = base_input();
        input.crs_participating = false;

        let result = analyze_fatca_crs_reporting(&input).unwrap();

        assert!(!result
            .reporting_deadlines
            .iter()
            .any(|d| d.framework == "CRS"));
    }

    #[test]
    fn test_deadline_reporting_year_matches_input() {
        let mut input = base_input();
        input.reporting_year = 2024;

        let result = analyze_fatca_crs_reporting(&input).unwrap();

        for deadline in &result.reporting_deadlines {
            assert_eq!(deadline.reporting_year, 2024);
        }
    }

    // ---------------------------------------------------------------
    // Validation Tests
    // ---------------------------------------------------------------

    #[test]
    fn test_validation_empty_institution_name() {
        let mut input = base_input();
        input.institution_name = "".into();

        let result = analyze_fatca_crs_reporting(&input);
        assert!(result.is_err());
        match result.unwrap_err() {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "institution_name");
            }
            other => panic!("Expected InvalidInput, got {:?}", other),
        }
    }

    #[test]
    fn test_validation_empty_jurisdiction() {
        let mut input = base_input();
        input.jurisdiction = "  ".into();

        let result = analyze_fatca_crs_reporting(&input);
        assert!(result.is_err());
        match result.unwrap_err() {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "jurisdiction");
            }
            other => panic!("Expected InvalidInput, got {:?}", other),
        }
    }

    #[test]
    fn test_validation_negative_balance() {
        let mut input = base_input();
        input.aggregate_balance_usd = dec!(-1);

        let result = analyze_fatca_crs_reporting(&input);
        assert!(result.is_err());
        match result.unwrap_err() {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "aggregate_balance_usd");
            }
            other => panic!("Expected InvalidInput, got {:?}", other),
        }
    }

    #[test]
    fn test_validation_reporting_year_too_low() {
        let mut input = base_input();
        input.reporting_year = 2005;

        let result = analyze_fatca_crs_reporting(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_validation_reporting_year_too_high() {
        let mut input = base_input();
        input.reporting_year = 2200;

        let result = analyze_fatca_crs_reporting(&input);
        assert!(result.is_err());
    }

    // ---------------------------------------------------------------
    // Output Structure Tests
    // ---------------------------------------------------------------

    #[test]
    fn test_methodology_populated() {
        let input = base_input();
        let result = analyze_fatca_crs_reporting(&input).unwrap();

        assert!(!result.methodology.is_empty());
        assert!(result.methodology.contains("FATCA"));
        assert!(result.methodology.contains("CRS"));
    }

    #[test]
    fn test_assumptions_populated() {
        let input = base_input();
        let result = analyze_fatca_crs_reporting(&input).unwrap();

        assert!(!result.assumptions.is_empty());
        assert!(result.assumptions.iter().any(|a| a.contains("30%")));
    }

    #[test]
    fn test_withholding_warning_when_exposure() {
        let mut input = base_input();
        input.has_giin = false;
        input.us_indicia_found = 10;

        let result = analyze_fatca_crs_reporting(&input).unwrap();

        assert!(result
            .warnings
            .iter()
            .any(|w| w.contains("withholding exposure")));
    }

    #[test]
    fn test_us_indicia_obligation_generated() {
        let mut input = base_input();
        input.us_indicia_found = 8;

        let result = analyze_fatca_crs_reporting(&input).unwrap();

        assert!(result
            .fatca_status
            .reporting_obligations
            .iter()
            .any(|o| o.contains("8 US indicia")));
    }

    #[test]
    fn test_zero_indicia_no_indicia_obligation() {
        let mut input = base_input();
        input.us_indicia_found = 0;

        let result = analyze_fatca_crs_reporting(&input).unwrap();

        assert!(!result
            .fatca_status
            .reporting_obligations
            .iter()
            .any(|o| o.contains("US indicia found")));
    }

    #[test]
    fn test_serialization_roundtrip() {
        let input = base_input();
        let result = analyze_fatca_crs_reporting(&input).unwrap();

        let json = serde_json::to_string(&result).unwrap();
        let deserialized: FatcaCrsReportingOutput = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.compliance_score, result.compliance_score);
        assert_eq!(
            deserialized.withholding_exposure_usd,
            result.withholding_exposure_usd
        );
    }
}
