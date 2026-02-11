use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

use crate::error::CorpFinanceError;
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum FatcaClassification {
    FFI,
    ActiveNFFE,
    PassiveNFFE,
    ExemptBeneficialOwner,
    DeemedCompliant,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CrsClassification {
    FinancialInstitution,
    ActiveNFE,
    PassiveNFE,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControllingPerson {
    pub name: String,
    pub tax_residence: String,
    pub ownership_pct: Decimal,
}

// ---------------------------------------------------------------------------
// Input / Output
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityClassificationInput {
    pub entity_name: String,
    pub entity_type: String,
    pub jurisdiction_of_incorporation: String,
    pub jurisdiction_of_tax_residence: String,
    pub gross_income: Decimal,
    pub passive_income: Decimal,
    pub total_assets: Decimal,
    pub passive_assets: Decimal,
    pub is_publicly_traded: bool,
    pub is_government_entity: bool,
    pub is_international_org: bool,
    pub is_pension_fund: bool,
    pub controlling_persons: Vec<ControllingPerson>,
    pub has_us_controlling_persons: bool,
    pub is_sponsored: bool,
    pub sponsor_giin: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityClassificationOutput {
    pub fatca_classification: FatcaClassification,
    pub crs_classification: CrsClassification,
    pub classification_rationale: Vec<String>,
    pub passive_income_ratio: Decimal,
    pub passive_asset_ratio: Decimal,
    pub reporting_required: bool,
    pub controlling_persons_reportable: Vec<ControllingPerson>,
    pub documentation_required: Vec<String>,
    pub withholding_rate: Decimal,
    pub exemptions_available: Vec<String>,
    pub risk_factors: Vec<String>,
    pub methodology: String,
    pub assumptions: Vec<String>,
    pub warnings: Vec<String>,
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const PASSIVE_INCOME_THRESHOLD: Decimal = dec!(0.50);
const PASSIVE_ASSET_THRESHOLD: Decimal = dec!(0.50);
const CONTROLLING_PERSON_THRESHOLD: Decimal = dec!(0.25);
const FATCA_WITHHOLDING_RATE: Decimal = dec!(0.30);

/// Entity types that qualify as FFI under FATCA.
const FFI_ENTITY_TYPES: &[&str] = &[
    "bank",
    "custodian",
    "investment_entity",
    "insurance_company",
    "broker_dealer",
    "trust_company",
    "fund",
    "hedge_fund",
    "private_equity_fund",
];

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Classify an entity under both FATCA and CRS frameworks.
///
/// Determines whether the entity is an FFI/NFFE (FATCA) or FI/NFE (CRS),
/// applies the active vs passive tests based on income and asset ratios,
/// identifies reportable controlling persons, and specifies documentation
/// and withholding requirements.
pub fn classify_entity(
    input: &EntityClassificationInput,
) -> CorpFinanceResult<EntityClassificationOutput> {
    validate_classification_input(input)?;

    let mut rationale: Vec<String> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();
    let mut documentation: Vec<String> = Vec::new();
    let mut exemptions: Vec<String> = Vec::new();
    let mut risk_factors: Vec<String> = Vec::new();

    // ------------------------------------------------------------------
    // 1. Calculate Ratios
    // ------------------------------------------------------------------
    let passive_income_ratio = if input.gross_income > Decimal::ZERO {
        input.passive_income / input.gross_income
    } else {
        Decimal::ZERO
    };

    let passive_asset_ratio = if input.total_assets > Decimal::ZERO {
        input.passive_assets / input.total_assets
    } else {
        Decimal::ZERO
    };

    // ------------------------------------------------------------------
    // 2. FATCA Classification
    // ------------------------------------------------------------------
    let fatca_classification = determine_fatca_classification(
        input,
        passive_income_ratio,
        &mut rationale,
        &mut exemptions,
    );

    // ------------------------------------------------------------------
    // 3. CRS Classification
    // ------------------------------------------------------------------
    let crs_classification = determine_crs_classification(
        input,
        passive_income_ratio,
        passive_asset_ratio,
        &mut rationale,
    );

    // ------------------------------------------------------------------
    // 4. Reporting & Controlling Persons
    // ------------------------------------------------------------------
    let reporting_required =
        determine_reporting_required(&fatca_classification, &crs_classification);

    let controlling_persons_reportable = if crs_classification == CrsClassification::PassiveNFE
        || fatca_classification == FatcaClassification::PassiveNFFE
    {
        identify_reportable_controlling_persons(input, &mut warnings)
    } else {
        Vec::new()
    };

    // ------------------------------------------------------------------
    // 5. Documentation Requirements
    // ------------------------------------------------------------------
    build_documentation_requirements(
        input,
        &fatca_classification,
        &crs_classification,
        &mut documentation,
    );

    // ------------------------------------------------------------------
    // 6. Withholding Rate
    // ------------------------------------------------------------------
    let withholding_rate = determine_withholding_rate(input, &fatca_classification);

    // ------------------------------------------------------------------
    // 7. Risk Factors
    // ------------------------------------------------------------------
    build_risk_factors(
        input,
        passive_income_ratio,
        passive_asset_ratio,
        &fatca_classification,
        &controlling_persons_reportable,
        &mut risk_factors,
    );

    // ------------------------------------------------------------------
    // 8. Assemble Output
    // ------------------------------------------------------------------
    let assumptions = vec![
        format!(
            "Passive income threshold for active/passive test: {}%.",
            (PASSIVE_INCOME_THRESHOLD * dec!(100)).normalize()
        ),
        format!(
            "Passive asset threshold for active/passive test: {}%.",
            (PASSIVE_ASSET_THRESHOLD * dec!(100)).normalize()
        ),
        format!(
            "Controlling person ownership threshold: {}%.",
            (CONTROLLING_PERSON_THRESHOLD * dec!(100)).normalize()
        ),
        "FATCA FFI types: banks, custodians, investment entities, insurance companies.".into(),
        "Exempt beneficial owner categories: government entities, international \
         organisations, central banks, pension funds."
            .into(),
    ];

    let methodology = "Entity Classification Analysis: apply FATCA (IRC Chapter 4) and \
        CRS (OECD) classification rules based on entity type, income/asset composition, \
        controlling person identification, and exempt category eligibility."
        .to_string();

    Ok(EntityClassificationOutput {
        fatca_classification,
        crs_classification,
        classification_rationale: rationale,
        passive_income_ratio,
        passive_asset_ratio,
        reporting_required,
        controlling_persons_reportable,
        documentation_required: documentation,
        withholding_rate,
        exemptions_available: exemptions,
        risk_factors,
        methodology,
        assumptions,
        warnings,
    })
}

// ---------------------------------------------------------------------------
// FATCA Classification Logic
// ---------------------------------------------------------------------------

fn determine_fatca_classification(
    input: &EntityClassificationInput,
    passive_income_ratio: Decimal,
    rationale: &mut Vec<String>,
    exemptions: &mut Vec<String>,
) -> FatcaClassification {
    // Check exempt beneficial owner first
    if input.is_government_entity {
        rationale
            .push("Entity is a government entity; qualifies as Exempt Beneficial Owner.".into());
        exemptions.push("Government entity exemption under FATCA Reg. 1.1471-6(b).".into());
        return FatcaClassification::ExemptBeneficialOwner;
    }
    if input.is_international_org {
        rationale.push(
            "Entity is an international organisation; qualifies as Exempt Beneficial Owner.".into(),
        );
        exemptions
            .push("International organisation exemption under FATCA Reg. 1.1471-6(c).".into());
        return FatcaClassification::ExemptBeneficialOwner;
    }
    if input.is_pension_fund {
        rationale.push("Entity is a pension fund; qualifies as Exempt Beneficial Owner.".into());
        exemptions.push("Pension fund exemption under FATCA Reg. 1.1471-6(f).".into());
        return FatcaClassification::ExemptBeneficialOwner;
    }

    // Check deemed-compliant (sponsored entity)
    if input.is_sponsored {
        if input.sponsor_giin.is_some() {
            rationale.push(
                "Entity is sponsored with a valid sponsor GIIN; classified as Deemed-Compliant FFI."
                    .into(),
            );
            exemptions.push(
                "Sponsored entity deemed-compliant status under FATCA Reg. 1.1471-5(f)(1)(i)."
                    .into(),
            );
            return FatcaClassification::DeemedCompliant;
        }
        rationale.push(
            "Entity claims sponsored status but no sponsor GIIN provided; \
             cannot qualify as deemed-compliant."
                .into(),
        );
    }

    // Check FFI based on entity type
    let entity_type_lower = input.entity_type.to_lowercase();
    let is_ffi_type = FFI_ENTITY_TYPES
        .iter()
        .any(|t| entity_type_lower.contains(t));

    if is_ffi_type {
        rationale.push(format!(
            "Entity type '{}' qualifies as a Foreign Financial Institution (FFI).",
            input.entity_type
        ));
        return FatcaClassification::FFI;
    }

    // Check publicly traded
    if input.is_publicly_traded {
        rationale.push(
            "Entity is publicly traded on a recognised stock exchange; classified as Active NFFE."
                .into(),
        );
        exemptions
            .push("Publicly traded entity exemption under FATCA Reg. 1.1472-1(c)(1)(i).".into());
        return FatcaClassification::ActiveNFFE;
    }

    // Active vs passive NFFE based on income test
    if passive_income_ratio < PASSIVE_INCOME_THRESHOLD {
        rationale.push(format!(
            "Passive income ratio ({:.1}%) is below 50% threshold; classified as Active NFFE.",
            passive_income_ratio * dec!(100)
        ));
        FatcaClassification::ActiveNFFE
    } else {
        rationale.push(format!(
            "Passive income ratio ({:.1}%) is at or above 50% threshold; classified as Passive NFFE.",
            passive_income_ratio * dec!(100)
        ));
        FatcaClassification::PassiveNFFE
    }
}

// ---------------------------------------------------------------------------
// CRS Classification Logic
// ---------------------------------------------------------------------------

fn determine_crs_classification(
    input: &EntityClassificationInput,
    passive_income_ratio: Decimal,
    passive_asset_ratio: Decimal,
    rationale: &mut Vec<String>,
) -> CrsClassification {
    // Financial Institution check
    let entity_type_lower = input.entity_type.to_lowercase();
    let is_fi_type = FFI_ENTITY_TYPES
        .iter()
        .any(|t| entity_type_lower.contains(t));

    if is_fi_type {
        rationale.push(format!(
            "Entity type '{}' qualifies as a CRS Financial Institution.",
            input.entity_type
        ));
        return CrsClassification::FinancialInstitution;
    }

    // Active NFE tests (any one qualifies)
    // Test 1: publicly traded
    if input.is_publicly_traded {
        rationale.push("CRS: Entity is publicly traded; classified as Active NFE.".into());
        return CrsClassification::ActiveNFE;
    }

    // Test 2: government / international org / pension
    if input.is_government_entity || input.is_international_org || input.is_pension_fund {
        let reason = if input.is_government_entity {
            "government entity"
        } else if input.is_international_org {
            "international organisation"
        } else {
            "pension fund"
        };
        rationale.push(format!(
            "CRS: Entity is a {}; classified as Active NFE.",
            reason
        ));
        return CrsClassification::ActiveNFE;
    }

    // Test 3: income and asset tests (both must be < 50% passive)
    if passive_income_ratio < PASSIVE_INCOME_THRESHOLD
        && passive_asset_ratio < PASSIVE_ASSET_THRESHOLD
    {
        rationale.push(format!(
            "CRS: Passive income ratio ({:.1}%) < 50% AND passive asset ratio ({:.1}%) < 50%; \
             classified as Active NFE.",
            passive_income_ratio * dec!(100),
            passive_asset_ratio * dec!(100)
        ));
        return CrsClassification::ActiveNFE;
    }

    // Default: Passive NFE
    rationale.push(format!(
        "CRS: Passive income ratio ({:.1}%) and/or passive asset ratio ({:.1}%) \
         at or above 50%; classified as Passive NFE.",
        passive_income_ratio * dec!(100),
        passive_asset_ratio * dec!(100)
    ));
    CrsClassification::PassiveNFE
}

// ---------------------------------------------------------------------------
// Reporting & Controlling Persons
// ---------------------------------------------------------------------------

fn determine_reporting_required(fatca: &FatcaClassification, crs: &CrsClassification) -> bool {
    matches!(
        fatca,
        FatcaClassification::FFI
            | FatcaClassification::PassiveNFFE
            | FatcaClassification::DeemedCompliant
    ) || matches!(
        crs,
        CrsClassification::FinancialInstitution | CrsClassification::PassiveNFE
    )
}

fn identify_reportable_controlling_persons(
    input: &EntityClassificationInput,
    warnings: &mut Vec<String>,
) -> Vec<ControllingPerson> {
    let mut reportable: Vec<ControllingPerson> = Vec::new();

    for cp in &input.controlling_persons {
        if cp.ownership_pct >= CONTROLLING_PERSON_THRESHOLD {
            reportable.push(cp.clone());
        }
    }

    if reportable.is_empty() && !input.controlling_persons.is_empty() {
        warnings.push(
            "No controlling persons meet the 25% ownership threshold. \
             Review whether senior managing officials should be reported instead."
                .into(),
        );
    }

    if input.has_us_controlling_persons {
        warnings.push(
            "US controlling persons identified; FATCA reporting required \
             for passive NFFE with US beneficial owners."
                .into(),
        );
    }

    reportable
}

// ---------------------------------------------------------------------------
// Documentation Requirements
// ---------------------------------------------------------------------------

fn build_documentation_requirements(
    input: &EntityClassificationInput,
    fatca: &FatcaClassification,
    crs: &CrsClassification,
    docs: &mut Vec<String>,
) {
    match fatca {
        FatcaClassification::FFI => {
            docs.push("Form W-8BEN-E (Certificate of Status of Beneficial Owner).".into());
            docs.push("GIIN registration confirmation.".into());
            docs.push("FATCA compliance programme documentation.".into());
        }
        FatcaClassification::ActiveNFFE => {
            docs.push("Form W-8BEN-E with Part XXV (Active NFFE certification).".into());
        }
        FatcaClassification::PassiveNFFE => {
            docs.push("Form W-8BEN-E with Part XXVI (Passive NFFE).".into());
            docs.push("Controlling person identification and documentation.".into());
            if input.has_us_controlling_persons {
                docs.push("Form W-9 for each US controlling person.".into());
            }
        }
        FatcaClassification::ExemptBeneficialOwner => {
            docs.push("Form W-8BEN-E with Part IV (Exempt Beneficial Owner).".into());
            docs.push("Supporting documentation for exempt status.".into());
        }
        FatcaClassification::DeemedCompliant => {
            docs.push("Form W-8BEN-E with applicable deemed-compliant certification.".into());
            if input.is_sponsored {
                docs.push("Sponsoring entity agreement and sponsor GIIN.".into());
            }
        }
    }

    match crs {
        CrsClassification::FinancialInstitution => {
            docs.push("CRS self-certification form (Entity).".into());
            docs.push("Registration with local tax authority as Reporting FI.".into());
        }
        CrsClassification::ActiveNFE => {
            docs.push("CRS self-certification form declaring Active NFE status.".into());
        }
        CrsClassification::PassiveNFE => {
            docs.push("CRS self-certification form declaring Passive NFE status.".into());
            docs.push(
                "Controlling person self-certification for each person with >= 25% ownership."
                    .into(),
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Withholding Rate
// ---------------------------------------------------------------------------

fn determine_withholding_rate(
    input: &EntityClassificationInput,
    fatca: &FatcaClassification,
) -> Decimal {
    match fatca {
        FatcaClassification::ExemptBeneficialOwner => Decimal::ZERO,
        FatcaClassification::DeemedCompliant => Decimal::ZERO,
        FatcaClassification::ActiveNFFE => Decimal::ZERO,
        FatcaClassification::FFI => {
            // FFI with GIIN (or sponsored) = 0%, without = 30%
            if input.is_sponsored && input.sponsor_giin.is_some() {
                Decimal::ZERO
            } else {
                // Assume compliant FFI (has GIIN) for classification purposes
                // Actual withholding depends on GIIN status checked at payment time
                Decimal::ZERO
            }
        }
        FatcaClassification::PassiveNFFE => {
            if input.has_us_controlling_persons {
                FATCA_WITHHOLDING_RATE
            } else {
                Decimal::ZERO
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Risk Factors
// ---------------------------------------------------------------------------

fn build_risk_factors(
    input: &EntityClassificationInput,
    passive_income_ratio: Decimal,
    passive_asset_ratio: Decimal,
    fatca: &FatcaClassification,
    reportable_cps: &[ControllingPerson],
    risk_factors: &mut Vec<String>,
) {
    // High passive income ratio
    if passive_income_ratio >= dec!(0.80) {
        risk_factors.push(format!(
            "Very high passive income ratio ({:.1}%) suggests investment holding entity.",
            passive_income_ratio * dec!(100)
        ));
    } else if passive_income_ratio >= PASSIVE_INCOME_THRESHOLD {
        risk_factors.push(format!(
            "Passive income ratio ({:.1}%) exceeds 50% threshold.",
            passive_income_ratio * dec!(100)
        ));
    }

    // High passive asset ratio
    if passive_asset_ratio >= dec!(0.80) {
        risk_factors.push(format!(
            "Very high passive asset ratio ({:.1}%) suggests investment holding entity.",
            passive_asset_ratio * dec!(100)
        ));
    } else if passive_asset_ratio >= PASSIVE_ASSET_THRESHOLD {
        risk_factors.push(format!(
            "Passive asset ratio ({:.1}%) exceeds 50% threshold.",
            passive_asset_ratio * dec!(100)
        ));
    }

    // US controlling persons with passive NFFE
    if *fatca == FatcaClassification::PassiveNFFE && input.has_us_controlling_persons {
        risk_factors.push(
            "Passive NFFE with US controlling persons: subject to 30% FATCA withholding.".into(),
        );
    }

    // Multiple reportable controlling persons
    if reportable_cps.len() > 3 {
        risk_factors.push(format!(
            "Complex ownership structure with {} reportable controlling persons.",
            reportable_cps.len()
        ));
    }

    // Sponsored entity without GIIN
    if input.is_sponsored && input.sponsor_giin.is_none() {
        risk_factors.push("Sponsored entity claims but no sponsor GIIN provided.".into());
    }

    // Jurisdiction mismatch
    if input.jurisdiction_of_incorporation != input.jurisdiction_of_tax_residence {
        risk_factors.push(format!(
            "Jurisdiction mismatch: incorporated in {} but tax resident in {}.",
            input.jurisdiction_of_incorporation, input.jurisdiction_of_tax_residence
        ));
    }
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

fn validate_classification_input(input: &EntityClassificationInput) -> CorpFinanceResult<()> {
    if input.entity_name.trim().is_empty() {
        return Err(CorpFinanceError::InvalidInput {
            field: "entity_name".into(),
            reason: "Entity name cannot be empty".into(),
        });
    }
    if input.entity_type.trim().is_empty() {
        return Err(CorpFinanceError::InvalidInput {
            field: "entity_type".into(),
            reason: "Entity type cannot be empty".into(),
        });
    }
    if input.gross_income < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "gross_income".into(),
            reason: "Gross income cannot be negative".into(),
        });
    }
    if input.passive_income < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "passive_income".into(),
            reason: "Passive income cannot be negative".into(),
        });
    }
    if input.total_assets < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "total_assets".into(),
            reason: "Total assets cannot be negative".into(),
        });
    }
    if input.passive_assets < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "passive_assets".into(),
            reason: "Passive assets cannot be negative".into(),
        });
    }
    if input.passive_income > input.gross_income {
        return Err(CorpFinanceError::InvalidInput {
            field: "passive_income".into(),
            reason: "Passive income cannot exceed gross income".into(),
        });
    }
    if input.passive_assets > input.total_assets {
        return Err(CorpFinanceError::InvalidInput {
            field: "passive_assets".into(),
            reason: "Passive assets cannot exceed total assets".into(),
        });
    }
    for cp in &input.controlling_persons {
        if cp.ownership_pct < Decimal::ZERO || cp.ownership_pct > Decimal::ONE {
            return Err(CorpFinanceError::InvalidInput {
                field: "controlling_persons.ownership_pct".into(),
                reason: format!(
                    "Ownership percentage for '{}' must be between 0 and 1",
                    cp.name
                ),
            });
        }
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

    fn base_entity_input() -> EntityClassificationInput {
        EntityClassificationInput {
            entity_name: "Acme Holdings Ltd".into(),
            entity_type: "holding_company".into(),
            jurisdiction_of_incorporation: "KY".into(),
            jurisdiction_of_tax_residence: "KY".into(),
            gross_income: dec!(10_000_000),
            passive_income: dec!(3_000_000),
            total_assets: dec!(50_000_000),
            passive_assets: dec!(15_000_000),
            is_publicly_traded: false,
            is_government_entity: false,
            is_international_org: false,
            is_pension_fund: false,
            controlling_persons: vec![
                ControllingPerson {
                    name: "John Doe".into(),
                    tax_residence: "US".into(),
                    ownership_pct: dec!(0.40),
                },
                ControllingPerson {
                    name: "Jane Smith".into(),
                    tax_residence: "GB".into(),
                    ownership_pct: dec!(0.30),
                },
            ],
            has_us_controlling_persons: true,
            is_sponsored: false,
            sponsor_giin: None,
        }
    }

    fn bank_input() -> EntityClassificationInput {
        EntityClassificationInput {
            entity_name: "Global Bank AG".into(),
            entity_type: "bank".into(),
            jurisdiction_of_incorporation: "CH".into(),
            jurisdiction_of_tax_residence: "CH".into(),
            gross_income: dec!(500_000_000),
            passive_income: dec!(200_000_000),
            total_assets: dec!(10_000_000_000),
            passive_assets: dec!(3_000_000_000),
            is_publicly_traded: true,
            is_government_entity: false,
            is_international_org: false,
            is_pension_fund: false,
            controlling_persons: vec![],
            has_us_controlling_persons: false,
            is_sponsored: false,
            sponsor_giin: None,
        }
    }

    // ---------------------------------------------------------------
    // FFI Classification Tests
    // ---------------------------------------------------------------

    #[test]
    fn test_bank_classified_as_ffi() {
        let input = bank_input();
        let result = classify_entity(&input).unwrap();

        assert_eq!(result.fatca_classification, FatcaClassification::FFI);
    }

    #[test]
    fn test_custodian_classified_as_ffi() {
        let mut input = base_entity_input();
        input.entity_type = "custodian".into();

        let result = classify_entity(&input).unwrap();

        assert_eq!(result.fatca_classification, FatcaClassification::FFI);
    }

    #[test]
    fn test_investment_entity_classified_as_ffi() {
        let mut input = base_entity_input();
        input.entity_type = "investment_entity".into();

        let result = classify_entity(&input).unwrap();

        assert_eq!(result.fatca_classification, FatcaClassification::FFI);
    }

    #[test]
    fn test_insurance_company_classified_as_ffi() {
        let mut input = base_entity_input();
        input.entity_type = "insurance_company".into();

        let result = classify_entity(&input).unwrap();

        assert_eq!(result.fatca_classification, FatcaClassification::FFI);
    }

    #[test]
    fn test_fund_classified_as_ffi() {
        let mut input = base_entity_input();
        input.entity_type = "hedge_fund".into();

        let result = classify_entity(&input).unwrap();

        assert_eq!(result.fatca_classification, FatcaClassification::FFI);
    }

    #[test]
    fn test_bank_classified_as_crs_fi() {
        let input = bank_input();
        let result = classify_entity(&input).unwrap();

        assert_eq!(
            result.crs_classification,
            CrsClassification::FinancialInstitution
        );
    }

    // ---------------------------------------------------------------
    // Active vs Passive NFFE/NFE Tests
    // ---------------------------------------------------------------

    #[test]
    fn test_active_nffe_below_50pct_passive_income() {
        let mut input = base_entity_input();
        input.passive_income = dec!(4_000_000); // 40% of 10M
        input.passive_assets = dec!(20_000_000); // 40% of 50M

        let result = classify_entity(&input).unwrap();

        assert_eq!(result.fatca_classification, FatcaClassification::ActiveNFFE);
    }

    #[test]
    fn test_passive_nffe_at_50pct_passive_income() {
        let mut input = base_entity_input();
        input.passive_income = dec!(5_000_000); // exactly 50%
        input.passive_assets = dec!(20_000_000); // 40%

        let result = classify_entity(&input).unwrap();

        assert_eq!(
            result.fatca_classification,
            FatcaClassification::PassiveNFFE
        );
    }

    #[test]
    fn test_passive_nffe_above_50pct_passive_income() {
        let mut input = base_entity_input();
        input.passive_income = dec!(7_000_000); // 70%
        input.passive_assets = dec!(35_000_000); // 70%

        let result = classify_entity(&input).unwrap();

        assert_eq!(
            result.fatca_classification,
            FatcaClassification::PassiveNFFE
        );
    }

    #[test]
    fn test_crs_active_nfe_both_ratios_below_50() {
        let mut input = base_entity_input();
        input.passive_income = dec!(4_000_000); // 40%
        input.passive_assets = dec!(20_000_000); // 40%

        let result = classify_entity(&input).unwrap();

        assert_eq!(result.crs_classification, CrsClassification::ActiveNFE);
    }

    #[test]
    fn test_crs_passive_nfe_income_at_50_assets_below() {
        let mut input = base_entity_input();
        input.passive_income = dec!(5_000_000); // exactly 50%
        input.passive_assets = dec!(20_000_000); // 40%

        let result = classify_entity(&input).unwrap();

        // CRS requires BOTH to be < 50% for Active NFE
        assert_eq!(result.crs_classification, CrsClassification::PassiveNFE);
    }

    #[test]
    fn test_crs_passive_nfe_income_below_assets_at_50() {
        let mut input = base_entity_input();
        input.passive_income = dec!(4_000_000); // 40%
        input.passive_assets = dec!(25_000_000); // exactly 50%

        let result = classify_entity(&input).unwrap();

        assert_eq!(result.crs_classification, CrsClassification::PassiveNFE);
    }

    #[test]
    fn test_crs_passive_nfe_both_ratios_at_50() {
        let mut input = base_entity_input();
        input.passive_income = dec!(5_000_000); // 50%
        input.passive_assets = dec!(25_000_000); // 50%

        let result = classify_entity(&input).unwrap();

        assert_eq!(result.crs_classification, CrsClassification::PassiveNFE);
    }

    // ---------------------------------------------------------------
    // Income Ratio Edge Cases
    // ---------------------------------------------------------------

    #[test]
    fn test_income_ratio_zero_gross_income() {
        let mut input = base_entity_input();
        input.gross_income = Decimal::ZERO;
        input.passive_income = Decimal::ZERO;
        input.total_assets = dec!(1_000_000);
        input.passive_assets = dec!(400_000);

        let result = classify_entity(&input).unwrap();

        assert_eq!(result.passive_income_ratio, Decimal::ZERO);
        // With zero income ratio and <50% asset ratio, should be Active
        assert_eq!(result.fatca_classification, FatcaClassification::ActiveNFFE);
    }

    #[test]
    fn test_income_ratio_just_below_threshold() {
        let mut input = base_entity_input();
        input.gross_income = dec!(1_000_000);
        input.passive_income = dec!(499_999);
        input.passive_assets = dec!(20_000_000);

        let result = classify_entity(&input).unwrap();

        assert!(result.passive_income_ratio < dec!(0.50));
        assert_eq!(result.fatca_classification, FatcaClassification::ActiveNFFE);
    }

    #[test]
    fn test_income_ratio_just_above_threshold() {
        let mut input = base_entity_input();
        input.gross_income = dec!(1_000_000);
        input.passive_income = dec!(500_001);

        let result = classify_entity(&input).unwrap();

        assert!(result.passive_income_ratio > dec!(0.50));
        assert_eq!(
            result.fatca_classification,
            FatcaClassification::PassiveNFFE
        );
    }

    // ---------------------------------------------------------------
    // Asset Ratio Edge Cases
    // ---------------------------------------------------------------

    #[test]
    fn test_asset_ratio_zero_total_assets() {
        let mut input = base_entity_input();
        input.total_assets = Decimal::ZERO;
        input.passive_assets = Decimal::ZERO;
        input.gross_income = dec!(1_000_000);
        input.passive_income = dec!(300_000);

        let result = classify_entity(&input).unwrap();

        assert_eq!(result.passive_asset_ratio, Decimal::ZERO);
    }

    #[test]
    fn test_asset_ratio_just_below_50() {
        let mut input = base_entity_input();
        input.total_assets = dec!(10_000_000);
        input.passive_assets = dec!(4_999_999);
        input.passive_income = dec!(3_000_000); // 30%

        let result = classify_entity(&input).unwrap();

        assert!(result.passive_asset_ratio < dec!(0.50));
        // CRS: both ratios below 50% = Active NFE
        assert_eq!(result.crs_classification, CrsClassification::ActiveNFE);
    }

    #[test]
    fn test_asset_ratio_exactly_50() {
        let mut input = base_entity_input();
        input.total_assets = dec!(10_000_000);
        input.passive_assets = dec!(5_000_000);
        input.passive_income = dec!(3_000_000); // 30%

        let result = classify_entity(&input).unwrap();

        // Asset ratio >= 50% => CRS Passive NFE (even though income < 50%)
        assert_eq!(result.crs_classification, CrsClassification::PassiveNFE);
    }

    // ---------------------------------------------------------------
    // Controlling Person Tests
    // ---------------------------------------------------------------

    #[test]
    fn test_controlling_persons_reported_for_passive_nffe() {
        let mut input = base_entity_input();
        input.passive_income = dec!(6_000_000); // 60% -> Passive NFFE
        input.passive_assets = dec!(30_000_000); // 60%

        let result = classify_entity(&input).unwrap();

        // Both controlling persons have >= 25% ownership
        assert_eq!(result.controlling_persons_reportable.len(), 2);
    }

    #[test]
    fn test_controlling_persons_not_reported_for_active_nffe() {
        let mut input = base_entity_input();
        input.passive_income = dec!(3_000_000); // 30% -> Active NFFE
        input.passive_assets = dec!(15_000_000); // 30%

        let result = classify_entity(&input).unwrap();

        assert!(result.controlling_persons_reportable.is_empty());
    }

    #[test]
    fn test_controlling_person_below_25pct_not_reported() {
        let mut input = base_entity_input();
        input.passive_income = dec!(6_000_000); // Passive
        input.passive_assets = dec!(30_000_000);
        input.controlling_persons = vec![
            ControllingPerson {
                name: "Big Owner".into(),
                tax_residence: "US".into(),
                ownership_pct: dec!(0.60),
            },
            ControllingPerson {
                name: "Small Owner".into(),
                tax_residence: "GB".into(),
                ownership_pct: dec!(0.20),
            },
        ];

        let result = classify_entity(&input).unwrap();

        assert_eq!(result.controlling_persons_reportable.len(), 1);
        assert_eq!(result.controlling_persons_reportable[0].name, "Big Owner");
    }

    #[test]
    fn test_controlling_person_exactly_25pct_reported() {
        let mut input = base_entity_input();
        input.passive_income = dec!(6_000_000); // Passive
        input.passive_assets = dec!(30_000_000);
        input.controlling_persons = vec![ControllingPerson {
            name: "Threshold Owner".into(),
            tax_residence: "DE".into(),
            ownership_pct: dec!(0.25),
        }];

        let result = classify_entity(&input).unwrap();

        assert_eq!(result.controlling_persons_reportable.len(), 1);
    }

    #[test]
    fn test_no_qualifying_controlling_persons_warning() {
        let mut input = base_entity_input();
        input.passive_income = dec!(6_000_000); // Passive
        input.passive_assets = dec!(30_000_000);
        input.controlling_persons = vec![ControllingPerson {
            name: "Minor Owner".into(),
            tax_residence: "FR".into(),
            ownership_pct: dec!(0.10),
        }];

        let result = classify_entity(&input).unwrap();

        assert!(result
            .warnings
            .iter()
            .any(|w| w.contains("senior managing officials")));
    }

    #[test]
    fn test_us_controlling_persons_warning() {
        let mut input = base_entity_input();
        input.passive_income = dec!(6_000_000); // Passive
        input.passive_assets = dec!(30_000_000);
        input.has_us_controlling_persons = true;

        let result = classify_entity(&input).unwrap();

        assert!(result
            .warnings
            .iter()
            .any(|w| w.contains("US controlling persons")));
    }

    // ---------------------------------------------------------------
    // Exempt Beneficial Owner Tests
    // ---------------------------------------------------------------

    #[test]
    fn test_government_entity_exempt() {
        let mut input = base_entity_input();
        input.is_government_entity = true;

        let result = classify_entity(&input).unwrap();

        assert_eq!(
            result.fatca_classification,
            FatcaClassification::ExemptBeneficialOwner
        );
        assert_eq!(result.withholding_rate, Decimal::ZERO);
    }

    #[test]
    fn test_international_org_exempt() {
        let mut input = base_entity_input();
        input.is_international_org = true;

        let result = classify_entity(&input).unwrap();

        assert_eq!(
            result.fatca_classification,
            FatcaClassification::ExemptBeneficialOwner
        );
    }

    #[test]
    fn test_pension_fund_exempt() {
        let mut input = base_entity_input();
        input.is_pension_fund = true;

        let result = classify_entity(&input).unwrap();

        assert_eq!(
            result.fatca_classification,
            FatcaClassification::ExemptBeneficialOwner
        );
    }

    #[test]
    fn test_government_entity_crs_active_nfe() {
        let mut input = base_entity_input();
        input.is_government_entity = true;

        let result = classify_entity(&input).unwrap();

        assert_eq!(result.crs_classification, CrsClassification::ActiveNFE);
    }

    #[test]
    fn test_exempt_beneficial_owner_no_withholding() {
        let mut input = base_entity_input();
        input.is_pension_fund = true;

        let result = classify_entity(&input).unwrap();

        assert_eq!(result.withholding_rate, Decimal::ZERO);
        assert!(!result.exemptions_available.is_empty());
    }

    #[test]
    fn test_exempt_overrides_passive_income_test() {
        let mut input = base_entity_input();
        input.is_government_entity = true;
        input.passive_income = dec!(9_000_000); // 90% passive
        input.passive_assets = dec!(45_000_000); // 90% passive

        let result = classify_entity(&input).unwrap();

        // Government entity is exempt regardless of income composition
        assert_eq!(
            result.fatca_classification,
            FatcaClassification::ExemptBeneficialOwner
        );
    }

    // ---------------------------------------------------------------
    // Publicly Traded Tests
    // ---------------------------------------------------------------

    #[test]
    fn test_publicly_traded_active_nffe() {
        let mut input = base_entity_input();
        input.is_publicly_traded = true;
        input.passive_income = dec!(8_000_000); // 80% passive

        let result = classify_entity(&input).unwrap();

        assert_eq!(result.fatca_classification, FatcaClassification::ActiveNFFE);
    }

    #[test]
    fn test_publicly_traded_crs_active_nfe() {
        let mut input = base_entity_input();
        input.is_publicly_traded = true;
        input.passive_income = dec!(8_000_000);
        input.passive_assets = dec!(40_000_000);

        let result = classify_entity(&input).unwrap();

        assert_eq!(result.crs_classification, CrsClassification::ActiveNFE);
    }

    #[test]
    fn test_publicly_traded_overrides_passive_test() {
        let mut input = base_entity_input();
        input.is_publicly_traded = true;
        input.passive_income = dec!(9_500_000); // 95% passive
        input.passive_assets = dec!(49_000_000); // 98% passive

        let result = classify_entity(&input).unwrap();

        // Publicly traded overrides income/asset tests
        assert_eq!(result.fatca_classification, FatcaClassification::ActiveNFFE);
        assert_eq!(result.crs_classification, CrsClassification::ActiveNFE);
    }

    #[test]
    fn test_publicly_traded_exemption_available() {
        let mut input = base_entity_input();
        input.is_publicly_traded = true;

        let result = classify_entity(&input).unwrap();

        assert!(result
            .exemptions_available
            .iter()
            .any(|e| e.contains("Publicly traded")));
    }

    // ---------------------------------------------------------------
    // Sponsored Entity Tests
    // ---------------------------------------------------------------

    #[test]
    fn test_sponsored_with_giin_deemed_compliant() {
        let mut input = base_entity_input();
        input.is_sponsored = true;
        input.sponsor_giin = Some("98Q96B.00000.SP.250".into());

        let result = classify_entity(&input).unwrap();

        assert_eq!(
            result.fatca_classification,
            FatcaClassification::DeemedCompliant
        );
    }

    #[test]
    fn test_sponsored_without_giin_not_deemed_compliant() {
        let mut input = base_entity_input();
        input.is_sponsored = true;
        input.sponsor_giin = None;
        input.passive_income = dec!(3_000_000); // 30% -> Active NFFE

        let result = classify_entity(&input).unwrap();

        // Without sponsor GIIN, falls through to income test
        assert_ne!(
            result.fatca_classification,
            FatcaClassification::DeemedCompliant
        );
    }

    #[test]
    fn test_sponsored_without_giin_risk_factor() {
        let mut input = base_entity_input();
        input.is_sponsored = true;
        input.sponsor_giin = None;

        let result = classify_entity(&input).unwrap();

        assert!(result
            .risk_factors
            .iter()
            .any(|r| r.contains("sponsor GIIN")));
    }

    #[test]
    fn test_deemed_compliant_no_withholding() {
        let mut input = base_entity_input();
        input.is_sponsored = true;
        input.sponsor_giin = Some("VALID_GIIN".into());

        let result = classify_entity(&input).unwrap();

        assert_eq!(result.withholding_rate, Decimal::ZERO);
    }

    // ---------------------------------------------------------------
    // Documentation Requirement Tests
    // ---------------------------------------------------------------

    #[test]
    fn test_ffi_requires_w8bene_and_giin() {
        let input = bank_input();
        let result = classify_entity(&input).unwrap();

        assert!(result
            .documentation_required
            .iter()
            .any(|d| d.contains("W-8BEN-E")));
        assert!(result
            .documentation_required
            .iter()
            .any(|d| d.contains("GIIN")));
    }

    #[test]
    fn test_passive_nffe_requires_controlling_person_docs() {
        let mut input = base_entity_input();
        input.passive_income = dec!(6_000_000);
        input.passive_assets = dec!(30_000_000);

        let result = classify_entity(&input).unwrap();

        assert!(result
            .documentation_required
            .iter()
            .any(|d| d.contains("Controlling person")));
    }

    #[test]
    fn test_passive_nffe_with_us_cp_requires_w9() {
        let mut input = base_entity_input();
        input.passive_income = dec!(6_000_000);
        input.passive_assets = dec!(30_000_000);
        input.has_us_controlling_persons = true;

        let result = classify_entity(&input).unwrap();

        assert!(result
            .documentation_required
            .iter()
            .any(|d| d.contains("W-9")));
    }

    #[test]
    fn test_exempt_bo_requires_part_iv() {
        let mut input = base_entity_input();
        input.is_government_entity = true;

        let result = classify_entity(&input).unwrap();

        assert!(result
            .documentation_required
            .iter()
            .any(|d| d.contains("Part IV")));
    }

    #[test]
    fn test_crs_fi_requires_registration() {
        let input = bank_input();
        let result = classify_entity(&input).unwrap();

        assert!(result
            .documentation_required
            .iter()
            .any(|d| d.contains("Registration")));
    }

    #[test]
    fn test_crs_passive_nfe_requires_self_certification() {
        let mut input = base_entity_input();
        input.passive_income = dec!(6_000_000);
        input.passive_assets = dec!(30_000_000);

        let result = classify_entity(&input).unwrap();

        assert!(result
            .documentation_required
            .iter()
            .any(|d| d.contains("CRS self-certification") && d.contains("Passive NFE")));
    }

    // ---------------------------------------------------------------
    // Withholding Rate Tests
    // ---------------------------------------------------------------

    #[test]
    fn test_withholding_30pct_passive_nffe_with_us_cp() {
        let mut input = base_entity_input();
        input.passive_income = dec!(6_000_000);
        input.passive_assets = dec!(30_000_000);
        input.has_us_controlling_persons = true;

        let result = classify_entity(&input).unwrap();

        assert_eq!(result.withholding_rate, dec!(0.30));
    }

    #[test]
    fn test_withholding_zero_passive_nffe_no_us_cp() {
        let mut input = base_entity_input();
        input.passive_income = dec!(6_000_000);
        input.passive_assets = dec!(30_000_000);
        input.has_us_controlling_persons = false;
        input.controlling_persons = vec![ControllingPerson {
            name: "UK Person".into(),
            tax_residence: "GB".into(),
            ownership_pct: dec!(0.60),
        }];

        let result = classify_entity(&input).unwrap();

        assert_eq!(result.withholding_rate, Decimal::ZERO);
    }

    #[test]
    fn test_withholding_zero_active_nffe() {
        let mut input = base_entity_input();
        input.passive_income = dec!(3_000_000);
        input.passive_assets = dec!(15_000_000);

        let result = classify_entity(&input).unwrap();

        assert_eq!(result.withholding_rate, Decimal::ZERO);
    }

    #[test]
    fn test_withholding_zero_exempt_bo() {
        let mut input = base_entity_input();
        input.is_government_entity = true;

        let result = classify_entity(&input).unwrap();

        assert_eq!(result.withholding_rate, Decimal::ZERO);
    }

    // ---------------------------------------------------------------
    // Reporting Required Tests
    // ---------------------------------------------------------------

    #[test]
    fn test_reporting_required_for_ffi() {
        let input = bank_input();
        let result = classify_entity(&input).unwrap();

        assert!(result.reporting_required);
    }

    #[test]
    fn test_reporting_required_for_passive_nffe() {
        let mut input = base_entity_input();
        input.passive_income = dec!(6_000_000);
        input.passive_assets = dec!(30_000_000);

        let result = classify_entity(&input).unwrap();

        assert!(result.reporting_required);
    }

    #[test]
    fn test_reporting_not_required_for_active_nffe_active_nfe() {
        let mut input = base_entity_input();
        input.passive_income = dec!(3_000_000); // 30%
        input.passive_assets = dec!(15_000_000); // 30%
        input.is_publicly_traded = true;

        let result = classify_entity(&input).unwrap();

        // Active NFFE + Active NFE (publicly traded) = no reporting
        assert!(!result.reporting_required);
    }

    #[test]
    fn test_reporting_required_for_deemed_compliant() {
        let mut input = base_entity_input();
        input.is_sponsored = true;
        input.sponsor_giin = Some("VALID".into());

        let result = classify_entity(&input).unwrap();

        assert!(result.reporting_required);
    }

    // ---------------------------------------------------------------
    // Risk Factor Tests
    // ---------------------------------------------------------------

    #[test]
    fn test_jurisdiction_mismatch_risk_factor() {
        let mut input = base_entity_input();
        input.jurisdiction_of_incorporation = "KY".into();
        input.jurisdiction_of_tax_residence = "GB".into();

        let result = classify_entity(&input).unwrap();

        assert!(result
            .risk_factors
            .iter()
            .any(|r| r.contains("Jurisdiction mismatch")));
    }

    #[test]
    fn test_no_jurisdiction_mismatch_when_same() {
        let mut input = base_entity_input();
        input.jurisdiction_of_incorporation = "DE".into();
        input.jurisdiction_of_tax_residence = "DE".into();

        let result = classify_entity(&input).unwrap();

        assert!(!result
            .risk_factors
            .iter()
            .any(|r| r.contains("Jurisdiction mismatch")));
    }

    #[test]
    fn test_high_passive_income_risk_factor() {
        let mut input = base_entity_input();
        input.passive_income = dec!(9_000_000); // 90%

        let result = classify_entity(&input).unwrap();

        assert!(result
            .risk_factors
            .iter()
            .any(|r| r.contains("Very high passive income")));
    }

    #[test]
    fn test_high_passive_asset_risk_factor() {
        let mut input = base_entity_input();
        input.passive_assets = dec!(45_000_000); // 90%

        let result = classify_entity(&input).unwrap();

        assert!(result
            .risk_factors
            .iter()
            .any(|r| r.contains("Very high passive asset")));
    }

    #[test]
    fn test_many_controlling_persons_risk_factor() {
        let mut input = base_entity_input();
        input.passive_income = dec!(6_000_000); // Passive
        input.passive_assets = dec!(30_000_000);
        input.controlling_persons = vec![
            ControllingPerson {
                name: "A".into(),
                tax_residence: "US".into(),
                ownership_pct: dec!(0.25),
            },
            ControllingPerson {
                name: "B".into(),
                tax_residence: "GB".into(),
                ownership_pct: dec!(0.25),
            },
            ControllingPerson {
                name: "C".into(),
                tax_residence: "DE".into(),
                ownership_pct: dec!(0.25),
            },
            ControllingPerson {
                name: "D".into(),
                tax_residence: "FR".into(),
                ownership_pct: dec!(0.25),
            },
        ];

        let result = classify_entity(&input).unwrap();

        assert!(result
            .risk_factors
            .iter()
            .any(|r| r.contains("Complex ownership")));
    }

    // ---------------------------------------------------------------
    // Validation Tests
    // ---------------------------------------------------------------

    #[test]
    fn test_validation_empty_entity_name() {
        let mut input = base_entity_input();
        input.entity_name = "".into();

        let result = classify_entity(&input);
        assert!(result.is_err());
        match result.unwrap_err() {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "entity_name");
            }
            other => panic!("Expected InvalidInput, got {:?}", other),
        }
    }

    #[test]
    fn test_validation_empty_entity_type() {
        let mut input = base_entity_input();
        input.entity_type = "  ".into();

        let result = classify_entity(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_validation_negative_gross_income() {
        let mut input = base_entity_input();
        input.gross_income = dec!(-1);

        let result = classify_entity(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_validation_negative_passive_income() {
        let mut input = base_entity_input();
        input.passive_income = dec!(-100);

        let result = classify_entity(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_validation_passive_income_exceeds_gross() {
        let mut input = base_entity_input();
        input.gross_income = dec!(100);
        input.passive_income = dec!(200);

        let result = classify_entity(&input);
        assert!(result.is_err());
        match result.unwrap_err() {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "passive_income");
            }
            other => panic!("Expected InvalidInput, got {:?}", other),
        }
    }

    #[test]
    fn test_validation_passive_assets_exceeds_total() {
        let mut input = base_entity_input();
        input.total_assets = dec!(100);
        input.passive_assets = dec!(200);

        let result = classify_entity(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_validation_negative_total_assets() {
        let mut input = base_entity_input();
        input.total_assets = dec!(-1);

        let result = classify_entity(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_validation_controlling_person_pct_over_1() {
        let mut input = base_entity_input();
        input.controlling_persons = vec![ControllingPerson {
            name: "Over".into(),
            tax_residence: "US".into(),
            ownership_pct: dec!(1.50),
        }];

        let result = classify_entity(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_validation_controlling_person_pct_negative() {
        let mut input = base_entity_input();
        input.controlling_persons = vec![ControllingPerson {
            name: "Negative".into(),
            tax_residence: "US".into(),
            ownership_pct: dec!(-0.10),
        }];

        let result = classify_entity(&input);
        assert!(result.is_err());
    }

    // ---------------------------------------------------------------
    // Output Structure Tests
    // ---------------------------------------------------------------

    #[test]
    fn test_methodology_populated() {
        let input = base_entity_input();
        let result = classify_entity(&input).unwrap();

        assert!(!result.methodology.is_empty());
        assert!(result.methodology.contains("FATCA"));
        assert!(result.methodology.contains("CRS"));
    }

    #[test]
    fn test_assumptions_populated() {
        let input = base_entity_input();
        let result = classify_entity(&input).unwrap();

        assert!(!result.assumptions.is_empty());
        assert!(result.assumptions.iter().any(|a| a.contains("50%")));
    }

    #[test]
    fn test_rationale_populated() {
        let input = base_entity_input();
        let result = classify_entity(&input).unwrap();

        assert!(!result.classification_rationale.is_empty());
    }

    #[test]
    fn test_serialization_roundtrip() {
        let input = base_entity_input();
        let result = classify_entity(&input).unwrap();

        let json = serde_json::to_string(&result).unwrap();
        let deserialized: EntityClassificationOutput = serde_json::from_str(&json).unwrap();

        assert_eq!(
            deserialized.fatca_classification,
            result.fatca_classification
        );
        assert_eq!(deserialized.crs_classification, result.crs_classification);
        assert_eq!(
            deserialized.passive_income_ratio,
            result.passive_income_ratio
        );
    }

    #[test]
    fn test_passive_income_ratio_calculation() {
        let mut input = base_entity_input();
        input.gross_income = dec!(10_000_000);
        input.passive_income = dec!(3_500_000);

        let result = classify_entity(&input).unwrap();

        assert_eq!(result.passive_income_ratio, dec!(0.35));
    }

    #[test]
    fn test_passive_asset_ratio_calculation() {
        let mut input = base_entity_input();
        input.total_assets = dec!(50_000_000);
        input.passive_assets = dec!(20_000_000);

        let result = classify_entity(&input).unwrap();

        assert_eq!(result.passive_asset_ratio, dec!(0.40));
    }
}
