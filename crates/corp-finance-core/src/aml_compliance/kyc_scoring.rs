use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CustomerType {
    Individual,
    Corporate,
    Trust,
    Foundation,
    Partnership,
    PEP,
    ComplexStructure,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PepCategory {
    DomesticPEP,
    ForeignPEP,
    InternationalOrgPEP,
    FamilyMember,
    CloseAssociate,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SourceOfWealth {
    Employment,
    Business,
    Inheritance,
    Investment,
    Unclear,
    HighRiskIndustry,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProductType {
    RetailBanking,
    PrivateBanking,
    CorrespondentBanking,
    TradeFinance,
    FundInvestment,
    CustodyServices,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Channel {
    FaceToFace,
    Online,
    IntroducedBusiness,
    ThirdParty,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
    Prohibited,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DueDiligenceLevel {
    SDD,
    CDD,
    EDD,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MonitoringFrequency {
    Monthly,
    Quarterly,
    Annual,
    Triennial,
}

// ---------------------------------------------------------------------------
// Input / Output structs
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KycRiskInput {
    pub customer_name: String,
    pub customer_type: CustomerType,
    pub jurisdiction_of_incorporation: String,
    pub jurisdiction_of_operations: Vec<String>,
    pub is_pep: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pep_category: Option<PepCategory>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub years_since_pep_role: Option<u32>,
    pub source_of_wealth: SourceOfWealth,
    pub source_of_funds: String,
    pub product_type: ProductType,
    pub channel: Channel,
    pub annual_transaction_volume: Decimal,
    pub average_transaction_size: Decimal,
    pub cross_border_transaction_pct: Decimal,
    pub cash_transaction_pct: Decimal,
    pub ownership_layers: u32,
    pub has_nominee_directors: bool,
    pub has_bearer_shares: bool,
    pub adverse_media_hits: u32,
    pub industry: String,
    pub expected_account_activity: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KycRiskOutput {
    pub overall_risk_score: Decimal,
    pub risk_level: RiskLevel,
    pub risk_breakdown: RiskBreakdown,
    pub due_diligence_level: DueDiligenceLevel,
    pub pep_assessment: Option<PepAssessment>,
    pub red_flags: Vec<RedFlag>,
    pub monitoring_frequency: MonitoringFrequency,
    pub documentation_required: Vec<String>,
    pub recommended_actions: Vec<String>,
    pub jurisdiction_risk_details: Vec<JurisdictionRisk>,
    pub methodology: String,
    pub assumptions: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskBreakdown {
    pub customer_score: Decimal,
    pub geographic_score: Decimal,
    pub product_score: Decimal,
    pub transaction_score: Decimal,
    pub source_of_wealth_score: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PepAssessment {
    pub category: PepCategory,
    pub de_pep_eligible: bool,
    pub edd_required: bool,
    pub senior_approval_required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedFlag {
    pub category: String,
    pub description: String,
    pub severity: RiskLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JurisdictionRisk {
    pub jurisdiction: String,
    pub risk_level: RiskLevel,
    pub fatf_status: String,
    pub eu_list_status: String,
}

// ---------------------------------------------------------------------------
// FATF / jurisdiction risk tables
// ---------------------------------------------------------------------------

/// Countries on the FATF black list (high-risk, call for action).
const FATF_BLACK_LIST: &[&str] = &["north korea", "dprk", "iran", "myanmar"];

/// Countries on the FATF grey list (increased monitoring).
const FATF_GREY_LIST: &[&str] = &[
    "albania",
    "barbados",
    "burkina faso",
    "cameroon",
    "cayman islands",
    "croatia",
    "democratic republic of the congo",
    "gibraltar",
    "haiti",
    "jamaica",
    "jordan",
    "mali",
    "mozambique",
    "nigeria",
    "panama",
    "philippines",
    "senegal",
    "south africa",
    "south sudan",
    "syria",
    "tanzania",
    "turkey",
    "uganda",
    "united arab emirates",
    "vietnam",
    "yemen",
];

/// Comprehensive sanctions jurisdictions (full embargoes).
const SANCTIONED_COUNTRIES: &[&str] = &["north korea", "dprk", "iran", "cuba", "syria", "crimea"];

/// EU high-risk third country list (representative subset).
const EU_HIGH_RISK: &[&str] = &[
    "afghanistan",
    "barbados",
    "burkina faso",
    "cambodia",
    "cayman islands",
    "democratic republic of the congo",
    "gibraltar",
    "haiti",
    "jamaica",
    "jordan",
    "mali",
    "morocco",
    "mozambique",
    "myanmar",
    "nicaragua",
    "nigeria",
    "pakistan",
    "panama",
    "philippines",
    "senegal",
    "south sudan",
    "syria",
    "tanzania",
    "trinidad and tobago",
    "turkey",
    "uganda",
    "united arab emirates",
    "vietnam",
    "yemen",
];

/// High-risk industries from an AML perspective.
const HIGH_RISK_INDUSTRIES: &[&str] = &[
    "gambling",
    "casino",
    "cryptocurrency",
    "crypto",
    "money service",
    "msb",
    "precious metals",
    "arms",
    "weapons",
    "tobacco",
    "marijuana",
    "cannabis",
    "adult entertainment",
    "virtual assets",
];

// ---------------------------------------------------------------------------
// Helper functions
// ---------------------------------------------------------------------------

fn normalize_jurisdiction(j: &str) -> String {
    j.trim().to_lowercase()
}

fn is_fatf_black_list(jurisdiction: &str) -> bool {
    let j = normalize_jurisdiction(jurisdiction);
    FATF_BLACK_LIST.iter().any(|&c| j.contains(c))
}

fn is_fatf_grey_list(jurisdiction: &str) -> bool {
    let j = normalize_jurisdiction(jurisdiction);
    FATF_GREY_LIST.iter().any(|&c| j.contains(c))
}

fn is_sanctioned(jurisdiction: &str) -> bool {
    let j = normalize_jurisdiction(jurisdiction);
    SANCTIONED_COUNTRIES.iter().any(|&c| j.contains(c))
}

fn is_eu_high_risk(jurisdiction: &str) -> bool {
    let j = normalize_jurisdiction(jurisdiction);
    EU_HIGH_RISK.iter().any(|&c| j.contains(c))
}

fn is_high_risk_industry(industry: &str) -> bool {
    let i = industry.trim().to_lowercase();
    HIGH_RISK_INDUSTRIES.iter().any(|&hi| i.contains(hi))
}

fn classify_jurisdiction(jurisdiction: &str) -> JurisdictionRisk {
    let j_lower = normalize_jurisdiction(jurisdiction);

    if is_sanctioned(&j_lower) {
        JurisdictionRisk {
            jurisdiction: jurisdiction.to_string(),
            risk_level: RiskLevel::Prohibited,
            fatf_status: "Sanctioned / Embargoed".to_string(),
            eu_list_status: if is_eu_high_risk(&j_lower) {
                "EU high-risk third country".to_string()
            } else {
                "N/A".to_string()
            },
        }
    } else if is_fatf_black_list(&j_lower) {
        JurisdictionRisk {
            jurisdiction: jurisdiction.to_string(),
            risk_level: RiskLevel::Critical,
            fatf_status: "FATF Black List".to_string(),
            eu_list_status: if is_eu_high_risk(&j_lower) {
                "EU high-risk third country".to_string()
            } else {
                "N/A".to_string()
            },
        }
    } else if is_fatf_grey_list(&j_lower) {
        JurisdictionRisk {
            jurisdiction: jurisdiction.to_string(),
            risk_level: RiskLevel::High,
            fatf_status: "FATF Grey List".to_string(),
            eu_list_status: if is_eu_high_risk(&j_lower) {
                "EU high-risk third country".to_string()
            } else {
                "N/A".to_string()
            },
        }
    } else if is_eu_high_risk(&j_lower) {
        JurisdictionRisk {
            jurisdiction: jurisdiction.to_string(),
            risk_level: RiskLevel::High,
            fatf_status: "Not listed".to_string(),
            eu_list_status: "EU high-risk third country".to_string(),
        }
    } else {
        JurisdictionRisk {
            jurisdiction: jurisdiction.to_string(),
            risk_level: RiskLevel::Low,
            fatf_status: "Not listed".to_string(),
            eu_list_status: "Not listed".to_string(),
        }
    }
}

// ---------------------------------------------------------------------------
// Scoring functions
// ---------------------------------------------------------------------------

/// Customer type risk score (max 25).
fn score_customer_type(input: &KycRiskInput) -> Decimal {
    let base = match input.customer_type {
        CustomerType::Individual => dec!(5),
        CustomerType::Corporate => dec!(10),
        CustomerType::Partnership => dec!(10),
        CustomerType::Trust => dec!(15),
        CustomerType::Foundation => dec!(15),
        CustomerType::ComplexStructure => dec!(20),
        CustomerType::PEP => dec!(25),
    };
    // If the customer is flagged as PEP but type is not PEP, bump to 25
    if input.is_pep && base < dec!(25) {
        dec!(25)
    } else {
        base
    }
}

/// Geographic risk score (max 25).
/// Takes highest risk across all jurisdictions.
fn score_geographic(input: &KycRiskInput) -> Decimal {
    let mut all_jurisdictions = vec![input.jurisdiction_of_incorporation.clone()];
    all_jurisdictions.extend(input.jurisdiction_of_operations.clone());

    let mut max_score = dec!(5); // low-risk default

    for j in &all_jurisdictions {
        let score = if is_sanctioned(j) || is_fatf_black_list(j) {
            dec!(25)
        } else if is_fatf_grey_list(j) {
            dec!(20)
        } else if is_eu_high_risk(j) {
            dec!(18)
        } else {
            dec!(5)
        };
        if score > max_score {
            max_score = score;
        }
    }

    max_score
}

/// Product risk score (max 20).
fn score_product(input: &KycRiskInput) -> Decimal {
    let base = match input.product_type {
        ProductType::RetailBanking => dec!(5),
        ProductType::CustodyServices => dec!(7),
        ProductType::FundInvestment => dec!(8),
        ProductType::PrivateBanking => dec!(10),
        ProductType::CorrespondentBanking => dec!(15),
        ProductType::TradeFinance => dec!(20),
    };

    // Channel adjustment: add up to 5 for high-risk channels, cap at 20
    let channel_adj = match input.channel {
        Channel::FaceToFace => dec!(0),
        Channel::Online => dec!(3),
        Channel::IntroducedBusiness => dec!(2),
        Channel::ThirdParty => dec!(4),
    };

    let total = base + channel_adj;
    if total > dec!(20) {
        dec!(20)
    } else {
        total
    }
}

/// Transaction pattern risk score (max 15).
fn score_transaction(input: &KycRiskInput) -> Decimal {
    let mut score = dec!(3); // baseline normal

    // High cross-border percentage
    if input.cross_border_transaction_pct > dec!(50) {
        score += dec!(3);
    }
    if input.cross_border_transaction_pct > dec!(80) {
        score += dec!(2);
    }

    // High cash percentage (structuring indicator)
    if input.cash_transaction_pct > dec!(30) {
        score += dec!(3);
    }
    if input.cash_transaction_pct > dec!(60) {
        score += dec!(2);
    }

    // Very high transaction volume relative to size
    // (threshold: >10M annual volume is elevated)
    if input.annual_transaction_volume > dec!(10_000_000) {
        score += dec!(2);
    }

    // Cap at 15
    if score > dec!(15) {
        dec!(15)
    } else {
        score
    }
}

/// Source of wealth risk score (max 15).
fn score_source_of_wealth(input: &KycRiskInput) -> Decimal {
    let base = match input.source_of_wealth {
        SourceOfWealth::Employment => dec!(3),
        SourceOfWealth::Investment => dec!(5),
        SourceOfWealth::Business => dec!(6),
        SourceOfWealth::Inheritance => dec!(8),
        SourceOfWealth::Unclear => dec!(12),
        SourceOfWealth::HighRiskIndustry => dec!(15),
    };

    // Additional risk for high-risk industry description
    let industry_adj = if is_high_risk_industry(&input.industry) {
        dec!(3)
    } else {
        dec!(0)
    };

    let total = base + industry_adj;
    if total > dec!(15) {
        dec!(15)
    } else {
        total
    }
}

// ---------------------------------------------------------------------------
// Red flag detection
// ---------------------------------------------------------------------------

fn detect_red_flags(input: &KycRiskInput) -> Vec<RedFlag> {
    let mut flags = Vec::new();

    // Shell company indicators
    if input.has_nominee_directors {
        flags.push(RedFlag {
            category: "Shell Company".to_string(),
            description: "Nominee directors present — potential shell company indicator"
                .to_string(),
            severity: RiskLevel::High,
        });
    }
    if input.has_bearer_shares {
        flags.push(RedFlag {
            category: "Shell Company".to_string(),
            description: "Bearer shares present — ownership concealment risk".to_string(),
            severity: RiskLevel::Critical,
        });
    }

    // Complex ownership
    if input.ownership_layers > 3 {
        flags.push(RedFlag {
            category: "Ownership Complexity".to_string(),
            description: format!(
                "{} ownership layers — complex structure obscuring UBO",
                input.ownership_layers
            ),
            severity: if input.ownership_layers > 5 {
                RiskLevel::Critical
            } else {
                RiskLevel::High
            },
        });
    }

    // Adverse media
    if input.adverse_media_hits > 0 {
        let severity = if input.adverse_media_hits >= 5 {
            RiskLevel::Critical
        } else if input.adverse_media_hits >= 2 {
            RiskLevel::High
        } else {
            RiskLevel::Medium
        };
        flags.push(RedFlag {
            category: "Adverse Media".to_string(),
            description: format!(
                "{} adverse media hit(s) identified",
                input.adverse_media_hits
            ),
            severity,
        });
    }

    // High cash transaction percentage
    if input.cash_transaction_pct > dec!(50) {
        flags.push(RedFlag {
            category: "Transaction Pattern".to_string(),
            description: format!(
                "Cash transactions at {}% — significantly above normal levels",
                input.cash_transaction_pct
            ),
            severity: RiskLevel::High,
        });
    }

    // Structuring indicator: high volume with small avg transaction
    if input.annual_transaction_volume > dec!(1_000_000)
        && input.average_transaction_size < dec!(10_000)
        && input.average_transaction_size > Decimal::ZERO
    {
        let implied_count = input.annual_transaction_volume / input.average_transaction_size;
        if implied_count > dec!(500) {
            flags.push(RedFlag {
                category: "Transaction Pattern".to_string(),
                description: "High volume of small transactions — potential structuring indicator"
                    .to_string(),
                severity: RiskLevel::High,
            });
        }
    }

    // PEP with unclear source of wealth
    if input.is_pep && input.source_of_wealth == SourceOfWealth::Unclear {
        flags.push(RedFlag {
            category: "PEP Risk".to_string(),
            description: "Politically Exposed Person with unclear source of wealth".to_string(),
            severity: RiskLevel::Critical,
        });
    }

    // Jurisdiction mismatch
    if !input.jurisdiction_of_operations.is_empty() {
        let incorp = normalize_jurisdiction(&input.jurisdiction_of_incorporation);
        let has_mismatch = input
            .jurisdiction_of_operations
            .iter()
            .any(|j| normalize_jurisdiction(j) != incorp);
        if has_mismatch {
            flags.push(RedFlag {
                category: "Jurisdiction Mismatch".to_string(),
                description: "Incorporation jurisdiction differs from operational jurisdictions"
                    .to_string(),
                severity: RiskLevel::Medium,
            });
        }
    }

    // High-risk industry
    if is_high_risk_industry(&input.industry) {
        flags.push(RedFlag {
            category: "Industry Risk".to_string(),
            description: format!(
                "Industry '{}' is classified as high-risk for AML purposes",
                input.industry
            ),
            severity: RiskLevel::High,
        });
    }

    // Sanctioned jurisdiction operations
    for j in &input.jurisdiction_of_operations {
        if is_sanctioned(j) {
            flags.push(RedFlag {
                category: "Sanctions".to_string(),
                description: format!("Operations in sanctioned jurisdiction: {}", j),
                severity: RiskLevel::Critical,
            });
        }
    }
    if is_sanctioned(&input.jurisdiction_of_incorporation) {
        flags.push(RedFlag {
            category: "Sanctions".to_string(),
            description: format!(
                "Incorporated in sanctioned jurisdiction: {}",
                input.jurisdiction_of_incorporation
            ),
            severity: RiskLevel::Critical,
        });
    }

    flags
}

// ---------------------------------------------------------------------------
// Due diligence & monitoring
// ---------------------------------------------------------------------------

fn determine_due_diligence(
    risk_score: Decimal,
    risk_level: &RiskLevel,
    is_pep: bool,
) -> DueDiligenceLevel {
    if is_pep {
        return DueDiligenceLevel::EDD;
    }
    match risk_level {
        RiskLevel::Prohibited | RiskLevel::Critical => DueDiligenceLevel::EDD,
        RiskLevel::High => DueDiligenceLevel::EDD,
        RiskLevel::Medium => DueDiligenceLevel::CDD,
        RiskLevel::Low => {
            if risk_score <= dec!(25) {
                DueDiligenceLevel::SDD
            } else {
                DueDiligenceLevel::CDD
            }
        }
    }
}

fn determine_monitoring_frequency(risk_level: &RiskLevel, is_pep: bool) -> MonitoringFrequency {
    if is_pep {
        return MonitoringFrequency::Monthly;
    }
    match risk_level {
        RiskLevel::Prohibited | RiskLevel::Critical => MonitoringFrequency::Monthly,
        RiskLevel::High => MonitoringFrequency::Quarterly,
        RiskLevel::Medium => MonitoringFrequency::Annual,
        RiskLevel::Low => MonitoringFrequency::Triennial,
    }
}

fn determine_risk_level(score: Decimal) -> RiskLevel {
    if score >= dec!(85) {
        RiskLevel::Prohibited
    } else if score >= dec!(70) {
        RiskLevel::Critical
    } else if score >= dec!(50) {
        RiskLevel::High
    } else if score >= dec!(30) {
        RiskLevel::Medium
    } else {
        RiskLevel::Low
    }
}

fn build_documentation_required(dd_level: &DueDiligenceLevel, input: &KycRiskInput) -> Vec<String> {
    let mut docs = vec![
        "Government-issued photo ID".to_string(),
        "Proof of address (utility bill or bank statement)".to_string(),
    ];

    match dd_level {
        DueDiligenceLevel::SDD => {
            // minimal extras
        }
        DueDiligenceLevel::CDD => {
            docs.push("Source of funds declaration".to_string());
            if matches!(
                input.customer_type,
                CustomerType::Corporate | CustomerType::Partnership
            ) {
                docs.push("Certificate of incorporation".to_string());
                docs.push("Register of directors and shareholders".to_string());
            }
        }
        DueDiligenceLevel::EDD => {
            docs.push("Source of funds declaration".to_string());
            docs.push("Source of wealth evidence".to_string());
            docs.push("Enhanced background check report".to_string());
            if input.is_pep {
                docs.push("Senior management sign-off".to_string());
                docs.push("PEP screening report".to_string());
            }
            if matches!(
                input.customer_type,
                CustomerType::Corporate
                    | CustomerType::Partnership
                    | CustomerType::Trust
                    | CustomerType::Foundation
                    | CustomerType::ComplexStructure
            ) {
                docs.push("Certificate of incorporation".to_string());
                docs.push("Register of directors and shareholders".to_string());
                docs.push("Full ownership chain / UBO declaration".to_string());
                docs.push("Audited financial statements".to_string());
            }
            if input.ownership_layers > 3 {
                docs.push("Organisational chart with all ownership layers".to_string());
            }
        }
    }

    docs
}

fn build_recommended_actions(
    risk_level: &RiskLevel,
    dd_level: &DueDiligenceLevel,
    input: &KycRiskInput,
    red_flags: &[RedFlag],
) -> Vec<String> {
    let mut actions = Vec::new();

    match risk_level {
        RiskLevel::Prohibited => {
            actions.push(
                "BLOCK: Do not onboard — sanctioned or prohibited jurisdiction/entity".to_string(),
            );
            actions.push("File SAR immediately if attempted onboarding".to_string());
        }
        RiskLevel::Critical => {
            actions.push("Escalate to MLRO for senior management decision".to_string());
            actions.push("Conduct full Enhanced Due Diligence before proceeding".to_string());
        }
        RiskLevel::High => {
            actions.push("Conduct Enhanced Due Diligence before onboarding".to_string());
            actions.push("Assign dedicated compliance officer for monitoring".to_string());
        }
        RiskLevel::Medium => {
            actions.push("Complete standard Customer Due Diligence".to_string());
        }
        RiskLevel::Low => {
            actions.push("Proceed with Simplified Due Diligence".to_string());
        }
    }

    if input.is_pep {
        actions.push("Obtain senior management approval for PEP relationship".to_string());
        actions.push("Verify source of wealth independently".to_string());
    }

    if !red_flags.is_empty() {
        let critical_count = red_flags
            .iter()
            .filter(|f| matches!(f.severity, RiskLevel::Critical))
            .count();
        if critical_count > 0 {
            actions.push(format!(
                "Investigate {} critical red flag(s) before proceeding",
                critical_count
            ));
        }
    }

    if matches!(dd_level, DueDiligenceLevel::EDD) && input.ownership_layers > 3 {
        actions.push("Map full beneficial ownership chain through all layers".to_string());
    }

    actions
}

// ---------------------------------------------------------------------------
// PEP assessment
// ---------------------------------------------------------------------------

fn assess_pep(input: &KycRiskInput) -> Option<PepAssessment> {
    if !input.is_pep {
        return None;
    }

    let category = input
        .pep_category
        .clone()
        .unwrap_or(PepCategory::ForeignPEP);

    let de_pep_eligible = match input.years_since_pep_role {
        Some(years) => years >= 24,
        None => false,
    };

    let edd_required = true; // always for PEPs

    let senior_approval_required = matches!(
        category,
        PepCategory::DomesticPEP | PepCategory::ForeignPEP | PepCategory::InternationalOrgPEP
    );

    Some(PepAssessment {
        category,
        de_pep_eligible,
        edd_required,
        senior_approval_required,
    })
}

// ---------------------------------------------------------------------------
// Main public function
// ---------------------------------------------------------------------------

/// Assess KYC/AML risk for a customer using FATF-based methodology.
///
/// Returns a comprehensive risk assessment with scores, due diligence
/// requirements, red flags, and recommended actions.
pub fn assess_kyc_risk(input: &KycRiskInput) -> CorpFinanceResult<KycRiskOutput> {
    // Input validation
    if input.customer_name.trim().is_empty() {
        return Err(crate::CorpFinanceError::InvalidInput {
            field: "customer_name".to_string(),
            reason: "Customer name must not be empty".to_string(),
        });
    }
    if input.jurisdiction_of_incorporation.trim().is_empty() {
        return Err(crate::CorpFinanceError::InvalidInput {
            field: "jurisdiction_of_incorporation".to_string(),
            reason: "Jurisdiction of incorporation must not be empty".to_string(),
        });
    }
    if input.cross_border_transaction_pct < Decimal::ZERO
        || input.cross_border_transaction_pct > dec!(100)
    {
        return Err(crate::CorpFinanceError::InvalidInput {
            field: "cross_border_transaction_pct".to_string(),
            reason: "Must be between 0 and 100".to_string(),
        });
    }
    if input.cash_transaction_pct < Decimal::ZERO || input.cash_transaction_pct > dec!(100) {
        return Err(crate::CorpFinanceError::InvalidInput {
            field: "cash_transaction_pct".to_string(),
            reason: "Must be between 0 and 100".to_string(),
        });
    }
    if input.annual_transaction_volume < Decimal::ZERO {
        return Err(crate::CorpFinanceError::InvalidInput {
            field: "annual_transaction_volume".to_string(),
            reason: "Must be non-negative".to_string(),
        });
    }

    // Score each risk dimension
    let customer_score = score_customer_type(input);
    let geographic_score = score_geographic(input);
    let product_score = score_product(input);
    let transaction_score = score_transaction(input);
    let source_of_wealth_score = score_source_of_wealth(input);

    let overall_risk_score = customer_score
        + geographic_score
        + product_score
        + transaction_score
        + source_of_wealth_score;

    // Cap at 100
    let overall_risk_score = if overall_risk_score > dec!(100) {
        dec!(100)
    } else {
        overall_risk_score
    };

    let risk_level = determine_risk_level(overall_risk_score);
    let due_diligence_level =
        determine_due_diligence(overall_risk_score, &risk_level, input.is_pep);
    let monitoring_frequency = determine_monitoring_frequency(&risk_level, input.is_pep);

    let red_flags = detect_red_flags(input);
    let pep_assessment = assess_pep(input);

    // Build jurisdiction risk details
    let mut jurisdiction_risk_details = Vec::new();
    jurisdiction_risk_details.push(classify_jurisdiction(&input.jurisdiction_of_incorporation));
    for j in &input.jurisdiction_of_operations {
        jurisdiction_risk_details.push(classify_jurisdiction(j));
    }

    let documentation_required = build_documentation_required(&due_diligence_level, input);
    let recommended_actions =
        build_recommended_actions(&risk_level, &due_diligence_level, input, &red_flags);

    let risk_breakdown = RiskBreakdown {
        customer_score,
        geographic_score,
        product_score,
        transaction_score,
        source_of_wealth_score,
    };

    let mut assumptions = vec![
        "Risk scoring follows FATF risk-based approach methodology".to_string(),
        "Jurisdiction lists based on FATF mutual evaluations as of assessment date".to_string(),
        "PEP de-classification period set at 24 months per conservative standard".to_string(),
    ];

    let mut warnings = Vec::new();

    // Additional warnings
    if matches!(risk_level, RiskLevel::Prohibited) {
        warnings.push(
            "Customer or jurisdiction is subject to comprehensive sanctions — onboarding prohibited"
                .to_string(),
        );
    }
    if input.is_pep {
        assumptions
            .push("PEP status verified against domestic and international databases".to_string());
    }
    if input.adverse_media_hits > 0 {
        warnings.push(format!(
            "{} adverse media hit(s) require manual review",
            input.adverse_media_hits
        ));
    }

    Ok(KycRiskOutput {
        overall_risk_score,
        risk_level,
        risk_breakdown,
        due_diligence_level,
        pep_assessment,
        red_flags,
        monitoring_frequency,
        documentation_required,
        recommended_actions,
        jurisdiction_risk_details,
        methodology: "FATF Risk-Based Approach — customer, geographic, product, transaction, and source of wealth scoring (0-100)".to_string(),
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

    fn base_input() -> KycRiskInput {
        KycRiskInput {
            customer_name: "Test Customer".to_string(),
            customer_type: CustomerType::Individual,
            jurisdiction_of_incorporation: "United Kingdom".to_string(),
            jurisdiction_of_operations: vec!["United Kingdom".to_string()],
            is_pep: false,
            pep_category: None,
            years_since_pep_role: None,
            source_of_wealth: SourceOfWealth::Employment,
            source_of_funds: "Salary".to_string(),
            product_type: ProductType::RetailBanking,
            channel: Channel::FaceToFace,
            annual_transaction_volume: dec!(50_000),
            average_transaction_size: dec!(500),
            cross_border_transaction_pct: dec!(5),
            cash_transaction_pct: dec!(2),
            ownership_layers: 1,
            has_nominee_directors: false,
            has_bearer_shares: false,
            adverse_media_hits: 0,
            industry: "Technology".to_string(),
            expected_account_activity: "Standard salary deposits and personal spending".to_string(),
        }
    }

    // === Customer type tests ===

    #[test]
    fn test_individual_low_risk() {
        let input = base_input();
        let result = assess_kyc_risk(&input).unwrap();
        assert_eq!(result.risk_level, RiskLevel::Low);
        assert!(result.overall_risk_score <= dec!(30));
        assert_eq!(result.due_diligence_level, DueDiligenceLevel::SDD);
        assert_eq!(result.monitoring_frequency, MonitoringFrequency::Triennial);
    }

    #[test]
    fn test_corporate_customer() {
        let mut input = base_input();
        input.customer_type = CustomerType::Corporate;
        let result = assess_kyc_risk(&input).unwrap();
        assert_eq!(result.risk_breakdown.customer_score, dec!(10));
    }

    #[test]
    fn test_trust_customer() {
        let mut input = base_input();
        input.customer_type = CustomerType::Trust;
        let result = assess_kyc_risk(&input).unwrap();
        assert_eq!(result.risk_breakdown.customer_score, dec!(15));
    }

    #[test]
    fn test_foundation_customer() {
        let mut input = base_input();
        input.customer_type = CustomerType::Foundation;
        let result = assess_kyc_risk(&input).unwrap();
        assert_eq!(result.risk_breakdown.customer_score, dec!(15));
    }

    #[test]
    fn test_partnership_customer() {
        let mut input = base_input();
        input.customer_type = CustomerType::Partnership;
        let result = assess_kyc_risk(&input).unwrap();
        assert_eq!(result.risk_breakdown.customer_score, dec!(10));
    }

    #[test]
    fn test_complex_structure_customer() {
        let mut input = base_input();
        input.customer_type = CustomerType::ComplexStructure;
        let result = assess_kyc_risk(&input).unwrap();
        assert_eq!(result.risk_breakdown.customer_score, dec!(20));
    }

    #[test]
    fn test_pep_customer_type() {
        let mut input = base_input();
        input.customer_type = CustomerType::PEP;
        input.is_pep = true;
        input.pep_category = Some(PepCategory::DomesticPEP);
        let result = assess_kyc_risk(&input).unwrap();
        assert_eq!(result.risk_breakdown.customer_score, dec!(25));
        assert!(result.pep_assessment.is_some());
    }

    // === PEP scenario tests ===

    #[test]
    fn test_domestic_pep() {
        let mut input = base_input();
        input.is_pep = true;
        input.pep_category = Some(PepCategory::DomesticPEP);
        let result = assess_kyc_risk(&input).unwrap();
        let pep = result.pep_assessment.unwrap();
        assert_eq!(pep.category, PepCategory::DomesticPEP);
        assert!(pep.edd_required);
        assert!(pep.senior_approval_required);
        assert!(!pep.de_pep_eligible);
        assert_eq!(result.due_diligence_level, DueDiligenceLevel::EDD);
        assert_eq!(result.monitoring_frequency, MonitoringFrequency::Monthly);
    }

    #[test]
    fn test_foreign_pep() {
        let mut input = base_input();
        input.is_pep = true;
        input.pep_category = Some(PepCategory::ForeignPEP);
        let result = assess_kyc_risk(&input).unwrap();
        let pep = result.pep_assessment.unwrap();
        assert_eq!(pep.category, PepCategory::ForeignPEP);
        assert!(pep.senior_approval_required);
    }

    #[test]
    fn test_international_org_pep() {
        let mut input = base_input();
        input.is_pep = true;
        input.pep_category = Some(PepCategory::InternationalOrgPEP);
        let result = assess_kyc_risk(&input).unwrap();
        let pep = result.pep_assessment.unwrap();
        assert_eq!(pep.category, PepCategory::InternationalOrgPEP);
        assert!(pep.senior_approval_required);
    }

    #[test]
    fn test_pep_family_member() {
        let mut input = base_input();
        input.is_pep = true;
        input.pep_category = Some(PepCategory::FamilyMember);
        let result = assess_kyc_risk(&input).unwrap();
        let pep = result.pep_assessment.unwrap();
        assert_eq!(pep.category, PepCategory::FamilyMember);
        assert!(!pep.senior_approval_required);
        assert!(pep.edd_required);
    }

    #[test]
    fn test_pep_close_associate() {
        let mut input = base_input();
        input.is_pep = true;
        input.pep_category = Some(PepCategory::CloseAssociate);
        let result = assess_kyc_risk(&input).unwrap();
        let pep = result.pep_assessment.unwrap();
        assert_eq!(pep.category, PepCategory::CloseAssociate);
        assert!(!pep.senior_approval_required);
    }

    #[test]
    fn test_de_pep_eligible() {
        let mut input = base_input();
        input.is_pep = true;
        input.pep_category = Some(PepCategory::DomesticPEP);
        input.years_since_pep_role = Some(25);
        let result = assess_kyc_risk(&input).unwrap();
        let pep = result.pep_assessment.unwrap();
        assert!(pep.de_pep_eligible);
    }

    #[test]
    fn test_de_pep_not_eligible_too_recent() {
        let mut input = base_input();
        input.is_pep = true;
        input.pep_category = Some(PepCategory::DomesticPEP);
        input.years_since_pep_role = Some(12);
        let result = assess_kyc_risk(&input).unwrap();
        let pep = result.pep_assessment.unwrap();
        assert!(!pep.de_pep_eligible);
    }

    #[test]
    fn test_pep_default_category_when_none() {
        let mut input = base_input();
        input.is_pep = true;
        input.pep_category = None;
        let result = assess_kyc_risk(&input).unwrap();
        let pep = result.pep_assessment.unwrap();
        assert_eq!(pep.category, PepCategory::ForeignPEP);
    }

    #[test]
    fn test_non_pep_type_but_is_pep_flag() {
        let mut input = base_input();
        input.customer_type = CustomerType::Individual;
        input.is_pep = true;
        input.pep_category = Some(PepCategory::DomesticPEP);
        let result = assess_kyc_risk(&input).unwrap();
        // Customer score should be bumped to 25
        assert_eq!(result.risk_breakdown.customer_score, dec!(25));
    }

    // === Geographic risk tests ===

    #[test]
    fn test_low_risk_jurisdiction() {
        let input = base_input(); // UK
        let result = assess_kyc_risk(&input).unwrap();
        assert_eq!(result.risk_breakdown.geographic_score, dec!(5));
    }

    #[test]
    fn test_fatf_grey_list_jurisdiction() {
        let mut input = base_input();
        input.jurisdiction_of_incorporation = "Turkey".to_string();
        let result = assess_kyc_risk(&input).unwrap();
        assert_eq!(result.risk_breakdown.geographic_score, dec!(20));
    }

    #[test]
    fn test_fatf_black_list_jurisdiction() {
        let mut input = base_input();
        input.jurisdiction_of_incorporation = "Iran".to_string();
        let result = assess_kyc_risk(&input).unwrap();
        assert_eq!(result.risk_breakdown.geographic_score, dec!(25));
    }

    #[test]
    fn test_sanctioned_jurisdiction() {
        let mut input = base_input();
        input.jurisdiction_of_incorporation = "North Korea".to_string();
        let result = assess_kyc_risk(&input).unwrap();
        assert_eq!(result.risk_breakdown.geographic_score, dec!(25));
    }

    #[test]
    fn test_eu_high_risk_jurisdiction() {
        let mut input = base_input();
        input.jurisdiction_of_incorporation = "Cambodia".to_string();
        input.jurisdiction_of_operations = vec!["Cambodia".to_string()];
        let result = assess_kyc_risk(&input).unwrap();
        assert_eq!(result.risk_breakdown.geographic_score, dec!(18));
    }

    #[test]
    fn test_multiple_jurisdictions_highest_risk() {
        let mut input = base_input();
        input.jurisdiction_of_incorporation = "United States".to_string();
        input.jurisdiction_of_operations = vec!["United States".to_string(), "Iran".to_string()];
        let result = assess_kyc_risk(&input).unwrap();
        // Should use highest risk (Iran = 25)
        assert_eq!(result.risk_breakdown.geographic_score, dec!(25));
    }

    #[test]
    fn test_jurisdiction_risk_details_populated() {
        let mut input = base_input();
        input.jurisdiction_of_operations = vec!["Germany".to_string(), "Turkey".to_string()];
        let result = assess_kyc_risk(&input).unwrap();
        // Incorporation + 2 operations = 3 entries
        assert_eq!(result.jurisdiction_risk_details.len(), 3);
    }

    #[test]
    fn test_sanctioned_jurisdiction_prohibited() {
        let mut input = base_input();
        input.jurisdiction_of_incorporation = "Cuba".to_string();
        let result = assess_kyc_risk(&input).unwrap();
        let detail = &result.jurisdiction_risk_details[0];
        assert_eq!(detail.risk_level, RiskLevel::Prohibited);
    }

    // === Product risk tests ===

    #[test]
    fn test_retail_banking_low_product_risk() {
        let input = base_input();
        let result = assess_kyc_risk(&input).unwrap();
        assert_eq!(result.risk_breakdown.product_score, dec!(5));
    }

    #[test]
    fn test_private_banking_product_risk() {
        let mut input = base_input();
        input.product_type = ProductType::PrivateBanking;
        let result = assess_kyc_risk(&input).unwrap();
        assert_eq!(result.risk_breakdown.product_score, dec!(10));
    }

    #[test]
    fn test_correspondent_banking_product_risk() {
        let mut input = base_input();
        input.product_type = ProductType::CorrespondentBanking;
        let result = assess_kyc_risk(&input).unwrap();
        assert_eq!(result.risk_breakdown.product_score, dec!(15));
    }

    #[test]
    fn test_trade_finance_product_risk() {
        let mut input = base_input();
        input.product_type = ProductType::TradeFinance;
        let result = assess_kyc_risk(&input).unwrap();
        assert_eq!(result.risk_breakdown.product_score, dec!(20));
    }

    #[test]
    fn test_fund_investment_product_risk() {
        let mut input = base_input();
        input.product_type = ProductType::FundInvestment;
        let result = assess_kyc_risk(&input).unwrap();
        assert_eq!(result.risk_breakdown.product_score, dec!(8));
    }

    #[test]
    fn test_custody_services_product_risk() {
        let mut input = base_input();
        input.product_type = ProductType::CustodyServices;
        let result = assess_kyc_risk(&input).unwrap();
        assert_eq!(result.risk_breakdown.product_score, dec!(7));
    }

    #[test]
    fn test_online_channel_adds_product_risk() {
        let mut input = base_input();
        input.product_type = ProductType::RetailBanking;
        input.channel = Channel::Online;
        let result = assess_kyc_risk(&input).unwrap();
        // retail (5) + online (3) = 8
        assert_eq!(result.risk_breakdown.product_score, dec!(8));
    }

    #[test]
    fn test_third_party_channel_adds_product_risk() {
        let mut input = base_input();
        input.product_type = ProductType::RetailBanking;
        input.channel = Channel::ThirdParty;
        let result = assess_kyc_risk(&input).unwrap();
        // retail (5) + third party (4) = 9
        assert_eq!(result.risk_breakdown.product_score, dec!(9));
    }

    #[test]
    fn test_product_score_capped_at_20() {
        let mut input = base_input();
        input.product_type = ProductType::TradeFinance; // 20
        input.channel = Channel::ThirdParty; // +4
        let result = assess_kyc_risk(&input).unwrap();
        assert_eq!(result.risk_breakdown.product_score, dec!(20));
    }

    // === Transaction pattern tests ===

    #[test]
    fn test_normal_transaction_pattern() {
        let input = base_input();
        let result = assess_kyc_risk(&input).unwrap();
        assert_eq!(result.risk_breakdown.transaction_score, dec!(3));
    }

    #[test]
    fn test_high_cross_border_pct() {
        let mut input = base_input();
        input.cross_border_transaction_pct = dec!(60);
        let result = assess_kyc_risk(&input).unwrap();
        // base (3) + >50 (3) = 6
        assert_eq!(result.risk_breakdown.transaction_score, dec!(6));
    }

    #[test]
    fn test_very_high_cross_border_pct() {
        let mut input = base_input();
        input.cross_border_transaction_pct = dec!(90);
        let result = assess_kyc_risk(&input).unwrap();
        // base (3) + >50 (3) + >80 (2) = 8
        assert_eq!(result.risk_breakdown.transaction_score, dec!(8));
    }

    #[test]
    fn test_high_cash_pct() {
        let mut input = base_input();
        input.cash_transaction_pct = dec!(40);
        let result = assess_kyc_risk(&input).unwrap();
        // base (3) + >30 (3) = 6
        assert_eq!(result.risk_breakdown.transaction_score, dec!(6));
    }

    #[test]
    fn test_very_high_cash_pct() {
        let mut input = base_input();
        input.cash_transaction_pct = dec!(70);
        let result = assess_kyc_risk(&input).unwrap();
        // base (3) + >30 (3) + >60 (2) = 8
        assert_eq!(result.risk_breakdown.transaction_score, dec!(8));
    }

    #[test]
    fn test_high_transaction_volume() {
        let mut input = base_input();
        input.annual_transaction_volume = dec!(15_000_000);
        let result = assess_kyc_risk(&input).unwrap();
        // base (3) + >10M (2) = 5
        assert_eq!(result.risk_breakdown.transaction_score, dec!(5));
    }

    #[test]
    fn test_transaction_score_capped_at_15() {
        let mut input = base_input();
        input.cross_border_transaction_pct = dec!(95);
        input.cash_transaction_pct = dec!(80);
        input.annual_transaction_volume = dec!(50_000_000);
        let result = assess_kyc_risk(&input).unwrap();
        assert_eq!(result.risk_breakdown.transaction_score, dec!(15));
    }

    // === Source of wealth tests ===

    #[test]
    fn test_employment_source_of_wealth() {
        let input = base_input();
        let result = assess_kyc_risk(&input).unwrap();
        assert_eq!(result.risk_breakdown.source_of_wealth_score, dec!(3));
    }

    #[test]
    fn test_business_source_of_wealth() {
        let mut input = base_input();
        input.source_of_wealth = SourceOfWealth::Business;
        let result = assess_kyc_risk(&input).unwrap();
        assert_eq!(result.risk_breakdown.source_of_wealth_score, dec!(6));
    }

    #[test]
    fn test_inheritance_source_of_wealth() {
        let mut input = base_input();
        input.source_of_wealth = SourceOfWealth::Inheritance;
        let result = assess_kyc_risk(&input).unwrap();
        assert_eq!(result.risk_breakdown.source_of_wealth_score, dec!(8));
    }

    #[test]
    fn test_unclear_source_of_wealth() {
        let mut input = base_input();
        input.source_of_wealth = SourceOfWealth::Unclear;
        let result = assess_kyc_risk(&input).unwrap();
        assert_eq!(result.risk_breakdown.source_of_wealth_score, dec!(12));
    }

    #[test]
    fn test_high_risk_industry_source_of_wealth() {
        let mut input = base_input();
        input.source_of_wealth = SourceOfWealth::HighRiskIndustry;
        let result = assess_kyc_risk(&input).unwrap();
        assert_eq!(result.risk_breakdown.source_of_wealth_score, dec!(15));
    }

    #[test]
    fn test_investment_source_of_wealth() {
        let mut input = base_input();
        input.source_of_wealth = SourceOfWealth::Investment;
        let result = assess_kyc_risk(&input).unwrap();
        assert_eq!(result.risk_breakdown.source_of_wealth_score, dec!(5));
    }

    #[test]
    fn test_high_risk_industry_name_adds_score() {
        let mut input = base_input();
        input.source_of_wealth = SourceOfWealth::Business;
        input.industry = "Gambling operations".to_string();
        let result = assess_kyc_risk(&input).unwrap();
        // business (6) + gambling adj (3) = 9
        assert_eq!(result.risk_breakdown.source_of_wealth_score, dec!(9));
    }

    #[test]
    fn test_source_of_wealth_score_capped_at_15() {
        let mut input = base_input();
        input.source_of_wealth = SourceOfWealth::HighRiskIndustry;
        input.industry = "Casino".to_string();
        let result = assess_kyc_risk(&input).unwrap();
        // high_risk (15) + casino adj (3) = capped at 15
        assert_eq!(result.risk_breakdown.source_of_wealth_score, dec!(15));
    }

    // === Due diligence level tests ===

    #[test]
    fn test_sdd_for_low_risk() {
        let input = base_input();
        let result = assess_kyc_risk(&input).unwrap();
        assert_eq!(result.due_diligence_level, DueDiligenceLevel::SDD);
    }

    #[test]
    fn test_cdd_for_medium_risk() {
        let mut input = base_input();
        input.customer_type = CustomerType::Corporate;
        input.source_of_wealth = SourceOfWealth::Business;
        input.cross_border_transaction_pct = dec!(60);
        let result = assess_kyc_risk(&input).unwrap();
        assert_eq!(result.risk_level, RiskLevel::Medium);
        assert_eq!(result.due_diligence_level, DueDiligenceLevel::CDD);
    }

    #[test]
    fn test_edd_for_high_risk() {
        let mut input = base_input();
        input.customer_type = CustomerType::ComplexStructure;
        input.source_of_wealth = SourceOfWealth::Unclear;
        input.product_type = ProductType::PrivateBanking;
        input.channel = Channel::Online;
        let result = assess_kyc_risk(&input).unwrap();
        assert!(matches!(
            result.risk_level,
            RiskLevel::High | RiskLevel::Critical
        ));
        assert_eq!(result.due_diligence_level, DueDiligenceLevel::EDD);
    }

    #[test]
    fn test_edd_always_for_pep() {
        let mut input = base_input();
        input.is_pep = true;
        input.pep_category = Some(PepCategory::DomesticPEP);
        let result = assess_kyc_risk(&input).unwrap();
        assert_eq!(result.due_diligence_level, DueDiligenceLevel::EDD);
    }

    // === Red flag tests ===

    #[test]
    fn test_no_red_flags_clean_customer() {
        let input = base_input();
        let result = assess_kyc_risk(&input).unwrap();
        assert!(result.red_flags.is_empty());
    }

    #[test]
    fn test_red_flag_nominee_directors() {
        let mut input = base_input();
        input.has_nominee_directors = true;
        let result = assess_kyc_risk(&input).unwrap();
        assert!(result
            .red_flags
            .iter()
            .any(|f| f.category == "Shell Company" && f.description.contains("Nominee")));
    }

    #[test]
    fn test_red_flag_bearer_shares() {
        let mut input = base_input();
        input.has_bearer_shares = true;
        let result = assess_kyc_risk(&input).unwrap();
        assert!(result
            .red_flags
            .iter()
            .any(|f| f.category == "Shell Company" && matches!(f.severity, RiskLevel::Critical)));
    }

    #[test]
    fn test_red_flag_complex_ownership() {
        let mut input = base_input();
        input.ownership_layers = 5;
        let result = assess_kyc_risk(&input).unwrap();
        assert!(result
            .red_flags
            .iter()
            .any(|f| f.category == "Ownership Complexity"));
    }

    #[test]
    fn test_red_flag_very_complex_ownership() {
        let mut input = base_input();
        input.ownership_layers = 7;
        let result = assess_kyc_risk(&input).unwrap();
        let flag = result
            .red_flags
            .iter()
            .find(|f| f.category == "Ownership Complexity")
            .unwrap();
        assert_eq!(flag.severity, RiskLevel::Critical);
    }

    #[test]
    fn test_red_flag_adverse_media_single() {
        let mut input = base_input();
        input.adverse_media_hits = 1;
        let result = assess_kyc_risk(&input).unwrap();
        let flag = result
            .red_flags
            .iter()
            .find(|f| f.category == "Adverse Media")
            .unwrap();
        assert_eq!(flag.severity, RiskLevel::Medium);
    }

    #[test]
    fn test_red_flag_adverse_media_multiple() {
        let mut input = base_input();
        input.adverse_media_hits = 3;
        let result = assess_kyc_risk(&input).unwrap();
        let flag = result
            .red_flags
            .iter()
            .find(|f| f.category == "Adverse Media")
            .unwrap();
        assert_eq!(flag.severity, RiskLevel::High);
    }

    #[test]
    fn test_red_flag_adverse_media_critical() {
        let mut input = base_input();
        input.adverse_media_hits = 5;
        let result = assess_kyc_risk(&input).unwrap();
        let flag = result
            .red_flags
            .iter()
            .find(|f| f.category == "Adverse Media")
            .unwrap();
        assert_eq!(flag.severity, RiskLevel::Critical);
    }

    #[test]
    fn test_red_flag_high_cash_pct() {
        let mut input = base_input();
        input.cash_transaction_pct = dec!(55);
        let result = assess_kyc_risk(&input).unwrap();
        assert!(result
            .red_flags
            .iter()
            .any(|f| f.category == "Transaction Pattern" && f.description.contains("Cash")));
    }

    #[test]
    fn test_red_flag_structuring_indicator() {
        let mut input = base_input();
        input.annual_transaction_volume = dec!(5_000_000);
        input.average_transaction_size = dec!(9_000);
        let result = assess_kyc_risk(&input).unwrap();
        assert!(result
            .red_flags
            .iter()
            .any(|f| f.description.contains("structuring")));
    }

    #[test]
    fn test_red_flag_pep_unclear_wealth() {
        let mut input = base_input();
        input.is_pep = true;
        input.pep_category = Some(PepCategory::DomesticPEP);
        input.source_of_wealth = SourceOfWealth::Unclear;
        let result = assess_kyc_risk(&input).unwrap();
        assert!(result.red_flags.iter().any(|f| f.category == "PEP Risk"));
    }

    #[test]
    fn test_red_flag_jurisdiction_mismatch() {
        let mut input = base_input();
        input.jurisdiction_of_incorporation = "Cayman Islands".to_string();
        input.jurisdiction_of_operations = vec!["United States".to_string()];
        let result = assess_kyc_risk(&input).unwrap();
        assert!(result
            .red_flags
            .iter()
            .any(|f| f.category == "Jurisdiction Mismatch"));
    }

    #[test]
    fn test_red_flag_high_risk_industry() {
        let mut input = base_input();
        input.industry = "Cryptocurrency exchange".to_string();
        let result = assess_kyc_risk(&input).unwrap();
        assert!(result
            .red_flags
            .iter()
            .any(|f| f.category == "Industry Risk"));
    }

    #[test]
    fn test_red_flag_sanctioned_operations() {
        let mut input = base_input();
        input.jurisdiction_of_operations = vec!["United Kingdom".to_string(), "Cuba".to_string()];
        let result = assess_kyc_risk(&input).unwrap();
        assert!(result
            .red_flags
            .iter()
            .any(|f| f.category == "Sanctions" && f.description.contains("Cuba")));
    }

    #[test]
    fn test_red_flag_sanctioned_incorporation() {
        let mut input = base_input();
        input.jurisdiction_of_incorporation = "Syria".to_string();
        let result = assess_kyc_risk(&input).unwrap();
        assert!(result
            .red_flags
            .iter()
            .any(|f| f.category == "Sanctions" && f.description.contains("Syria")));
    }

    // === Monitoring frequency tests ===

    #[test]
    fn test_monthly_monitoring_for_pep() {
        let mut input = base_input();
        input.is_pep = true;
        input.pep_category = Some(PepCategory::DomesticPEP);
        let result = assess_kyc_risk(&input).unwrap();
        assert_eq!(result.monitoring_frequency, MonitoringFrequency::Monthly);
    }

    #[test]
    fn test_triennial_monitoring_for_low_risk() {
        let input = base_input();
        let result = assess_kyc_risk(&input).unwrap();
        assert_eq!(result.monitoring_frequency, MonitoringFrequency::Triennial);
    }

    #[test]
    fn test_quarterly_monitoring_for_high_risk() {
        let mut input = base_input();
        input.customer_type = CustomerType::ComplexStructure;
        input.source_of_wealth = SourceOfWealth::Unclear;
        input.product_type = ProductType::PrivateBanking;
        input.channel = Channel::Online;
        let result = assess_kyc_risk(&input).unwrap();
        // Score should push to high risk => quarterly
        if matches!(result.risk_level, RiskLevel::High) {
            assert_eq!(result.monitoring_frequency, MonitoringFrequency::Quarterly);
        }
    }

    // === Documentation tests ===

    #[test]
    fn test_edd_documentation_includes_source_of_wealth() {
        let mut input = base_input();
        input.is_pep = true;
        input.pep_category = Some(PepCategory::DomesticPEP);
        let result = assess_kyc_risk(&input).unwrap();
        assert!(result
            .documentation_required
            .iter()
            .any(|d| d.contains("Source of wealth")));
    }

    #[test]
    fn test_edd_documentation_includes_pep_report() {
        let mut input = base_input();
        input.is_pep = true;
        input.pep_category = Some(PepCategory::DomesticPEP);
        let result = assess_kyc_risk(&input).unwrap();
        assert!(result
            .documentation_required
            .iter()
            .any(|d| d.contains("PEP screening")));
    }

    #[test]
    fn test_corporate_edd_documentation() {
        let mut input = base_input();
        input.customer_type = CustomerType::Corporate;
        input.is_pep = true;
        input.pep_category = Some(PepCategory::DomesticPEP);
        let result = assess_kyc_risk(&input).unwrap();
        assert!(result
            .documentation_required
            .iter()
            .any(|d| d.contains("UBO")));
    }

    // === Recommended actions tests ===

    #[test]
    fn test_prohibited_actions_block() {
        let mut input = base_input();
        input.jurisdiction_of_incorporation = "North Korea".to_string();
        input.customer_type = CustomerType::ComplexStructure;
        input.source_of_wealth = SourceOfWealth::HighRiskIndustry;
        input.product_type = ProductType::TradeFinance;
        input.cash_transaction_pct = dec!(70);
        input.cross_border_transaction_pct = dec!(90);
        let result = assess_kyc_risk(&input).unwrap();
        if matches!(result.risk_level, RiskLevel::Prohibited) {
            assert!(result
                .recommended_actions
                .iter()
                .any(|a| a.contains("BLOCK")));
        }
    }

    #[test]
    fn test_critical_actions_escalate() {
        let mut input = base_input();
        input.customer_type = CustomerType::ComplexStructure;
        input.jurisdiction_of_incorporation = "Myanmar".to_string();
        input.source_of_wealth = SourceOfWealth::Unclear;
        input.product_type = ProductType::CorrespondentBanking;
        let result = assess_kyc_risk(&input).unwrap();
        if matches!(result.risk_level, RiskLevel::Critical) {
            assert!(result
                .recommended_actions
                .iter()
                .any(|a| a.contains("MLRO")));
        }
    }

    #[test]
    fn test_pep_actions_senior_approval() {
        let mut input = base_input();
        input.is_pep = true;
        input.pep_category = Some(PepCategory::DomesticPEP);
        let result = assess_kyc_risk(&input).unwrap();
        assert!(result
            .recommended_actions
            .iter()
            .any(|a| a.contains("senior management")));
    }

    // === Input validation tests ===

    #[test]
    fn test_empty_customer_name_error() {
        let mut input = base_input();
        input.customer_name = "".to_string();
        let result = assess_kyc_risk(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_jurisdiction_error() {
        let mut input = base_input();
        input.jurisdiction_of_incorporation = "".to_string();
        let result = assess_kyc_risk(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_cross_border_pct_error() {
        let mut input = base_input();
        input.cross_border_transaction_pct = dec!(105);
        let result = assess_kyc_risk(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_negative_cross_border_pct_error() {
        let mut input = base_input();
        input.cross_border_transaction_pct = dec!(-1);
        let result = assess_kyc_risk(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_negative_transaction_volume_error() {
        let mut input = base_input();
        input.annual_transaction_volume = dec!(-100);
        let result = assess_kyc_risk(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_cash_pct_error() {
        let mut input = base_input();
        input.cash_transaction_pct = dec!(101);
        let result = assess_kyc_risk(&input);
        assert!(result.is_err());
    }

    // === Edge case tests ===

    #[test]
    fn test_zero_transactions() {
        let mut input = base_input();
        input.annual_transaction_volume = Decimal::ZERO;
        input.average_transaction_size = Decimal::ZERO;
        let result = assess_kyc_risk(&input).unwrap();
        assert_eq!(result.risk_breakdown.transaction_score, dec!(3));
    }

    #[test]
    fn test_max_ownership_layers() {
        let mut input = base_input();
        input.ownership_layers = 20;
        let result = assess_kyc_risk(&input).unwrap();
        assert!(result
            .red_flags
            .iter()
            .any(|f| f.category == "Ownership Complexity"
                && matches!(f.severity, RiskLevel::Critical)));
    }

    #[test]
    fn test_overall_score_capped_at_100() {
        let mut input = base_input();
        input.customer_type = CustomerType::PEP;
        input.is_pep = true;
        input.pep_category = Some(PepCategory::DomesticPEP);
        input.jurisdiction_of_incorporation = "North Korea".to_string();
        input.product_type = ProductType::TradeFinance;
        input.cash_transaction_pct = dec!(80);
        input.cross_border_transaction_pct = dec!(90);
        input.source_of_wealth = SourceOfWealth::HighRiskIndustry;
        input.annual_transaction_volume = dec!(50_000_000);
        let result = assess_kyc_risk(&input).unwrap();
        assert!(result.overall_risk_score <= dec!(100));
    }

    #[test]
    fn test_methodology_populated() {
        let input = base_input();
        let result = assess_kyc_risk(&input).unwrap();
        assert!(result.methodology.contains("FATF"));
    }

    #[test]
    fn test_assumptions_populated() {
        let input = base_input();
        let result = assess_kyc_risk(&input).unwrap();
        assert!(!result.assumptions.is_empty());
    }

    #[test]
    fn test_warnings_for_prohibited() {
        let mut input = base_input();
        input.customer_type = CustomerType::PEP;
        input.is_pep = true;
        input.pep_category = Some(PepCategory::DomesticPEP);
        input.jurisdiction_of_incorporation = "North Korea".to_string();
        input.product_type = ProductType::TradeFinance;
        input.source_of_wealth = SourceOfWealth::HighRiskIndustry;
        input.cash_transaction_pct = dec!(80);
        input.cross_border_transaction_pct = dec!(90);
        input.annual_transaction_volume = dec!(50_000_000);
        let result = assess_kyc_risk(&input).unwrap();
        if matches!(result.risk_level, RiskLevel::Prohibited) {
            assert!(result.warnings.iter().any(|w| w.contains("sanctions")));
        }
    }

    #[test]
    fn test_warnings_for_adverse_media() {
        let mut input = base_input();
        input.adverse_media_hits = 3;
        let result = assess_kyc_risk(&input).unwrap();
        assert!(result.warnings.iter().any(|w| w.contains("adverse media")));
    }

    #[test]
    fn test_serde_round_trip() {
        let input = base_input();
        let json = serde_json::to_string(&input).unwrap();
        let deserialized: KycRiskInput = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.customer_name, input.customer_name);
    }

    #[test]
    fn test_output_serde_round_trip() {
        let input = base_input();
        let result = assess_kyc_risk(&input).unwrap();
        let json = serde_json::to_string(&result).unwrap();
        let deserialized: KycRiskOutput = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.overall_risk_score, result.overall_risk_score);
    }

    // === Composite scenario tests ===

    #[test]
    fn test_high_risk_composite() {
        let mut input = base_input();
        input.customer_type = CustomerType::Trust;
        input.jurisdiction_of_incorporation = "Panama".to_string();
        input.product_type = ProductType::PrivateBanking;
        input.channel = Channel::Online;
        input.source_of_wealth = SourceOfWealth::Inheritance;
        input.cross_border_transaction_pct = dec!(60);
        input.has_nominee_directors = true;
        input.adverse_media_hits = 1;
        let result = assess_kyc_risk(&input).unwrap();
        assert!(result.overall_risk_score >= dec!(50));
        assert!(matches!(
            result.risk_level,
            RiskLevel::High | RiskLevel::Critical | RiskLevel::Prohibited
        ));
        assert_eq!(result.due_diligence_level, DueDiligenceLevel::EDD);
    }

    #[test]
    fn test_medium_risk_composite() {
        let mut input = base_input();
        input.customer_type = CustomerType::Corporate;
        input.product_type = ProductType::FundInvestment;
        input.source_of_wealth = SourceOfWealth::Business;
        input.cross_border_transaction_pct = dec!(30);
        let result = assess_kyc_risk(&input).unwrap();
        assert!(result.overall_risk_score >= dec!(25));
    }

    #[test]
    fn test_empty_operations_jurisdictions() {
        let mut input = base_input();
        input.jurisdiction_of_operations = vec![];
        let result = assess_kyc_risk(&input).unwrap();
        // Should still work with just incorporation jurisdiction
        assert!(result.overall_risk_score > Decimal::ZERO);
        assert_eq!(result.jurisdiction_risk_details.len(), 1);
    }

    #[test]
    fn test_case_insensitive_jurisdiction() {
        let mut input = base_input();
        input.jurisdiction_of_incorporation = "IRAN".to_string();
        let result = assess_kyc_risk(&input).unwrap();
        assert_eq!(result.risk_breakdown.geographic_score, dec!(25));
    }

    #[test]
    fn test_dprk_alias() {
        let mut input = base_input();
        input.jurisdiction_of_incorporation = "DPRK".to_string();
        let result = assess_kyc_risk(&input).unwrap();
        assert_eq!(result.risk_breakdown.geographic_score, dec!(25));
    }
}
