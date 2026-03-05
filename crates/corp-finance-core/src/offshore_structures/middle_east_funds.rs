use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

use crate::error::CorpFinanceError;
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Types — DIFC
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DifcFundInput {
    pub fund_name: String,
    /// "QIF", "ExemptFund", "DomesticFund"
    pub fund_type: String,
    pub fund_strategy: String,
    pub fund_size: Decimal,
    pub management_fee_rate: Decimal,
    pub performance_fee_rate: Decimal,
    pub minimum_subscription: Decimal,
    pub investor_count: u32,
    pub sharia_compliant: bool,
    pub sharia_board_members: u32,
    pub target_investors: Vec<String>,
}

// ---------------------------------------------------------------------------
// Types — ADGM
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdgmFundInput {
    pub fund_name: String,
    /// "Exempt", "QIF", "Public"
    pub fund_type: String,
    /// "InvestmentCompany", "InvestmentTrust", "LP"
    pub structure_type: String,
    pub fund_strategy: String,
    pub fund_size: Decimal,
    pub management_fee_rate: Decimal,
    pub performance_fee_rate: Decimal,
    pub minimum_subscription: Decimal,
    pub investor_count: u32,
    pub sharia_compliant: bool,
    pub target_investors: Vec<String>,
}

// ---------------------------------------------------------------------------
// Types — Sharia Compliance
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShariaScreeningCompany {
    pub name: String,
    /// Debt-to-market-cap ratio (e.g. 0.25 = 25%)
    pub debt_to_market_cap: Decimal,
    /// Interest income as percentage of total revenue (e.g. 0.03 = 3%)
    pub interest_income_pct: Decimal,
    /// Non-permissible revenue as percentage of total revenue
    pub non_permissible_revenue_pct: Decimal,
    /// Total dividends received from this company
    pub total_dividends: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShariaComplianceInput {
    /// "AAOIFI", "DJIM", "MSCI", "FTSE"
    pub screening_methodology: String,
    pub companies: Vec<ShariaScreeningCompany>,
    pub ssb_member_count: u32,
    pub ssb_qualified_scholars: u32,
    /// "Income", "Charity"
    pub purification_method: String,
}

// ---------------------------------------------------------------------------
// Types — Comparison
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DifcAdgmCompInput {
    pub fund_strategy: String,
    pub fund_size: Decimal,
    pub sharia_required: bool,
    pub target_investors: Vec<String>,
    pub preferred_legal_system: String,
}

// ---------------------------------------------------------------------------
// Output types — DIFC
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DifcStructureAnalysis {
    pub fund_type: String,
    pub description: String,
    pub minimum_subscription: Decimal,
    pub max_investors: u32,
    pub dfsa_process: String,
    pub approval_timeline_weeks: String,
    pub tax_guarantee_years: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DifcRegulatoryAnalysis {
    pub dfsa_license_category: String,
    pub dfsa_annual_fee: Decimal,
    pub compliance_officer_required: bool,
    pub mlro_required: bool,
    pub audit_required: bool,
    pub regulatory_reporting: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DifcTaxAnalysis {
    pub corporate_tax_rate: Decimal,
    pub withholding_tax_rate: Decimal,
    pub capital_gains_tax_rate: Decimal,
    pub vat_applicable: bool,
    pub tax_guarantee_period: String,
    pub distribution_tax_notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DifcShariaAnalysis {
    pub sharia_compliant: bool,
    pub ssb_required: bool,
    pub ssb_member_count: u32,
    pub ssb_adequate: bool,
    pub aaoifi_compliance: bool,
    pub cost_premium_pct: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DifcCostAnalysis {
    pub setup_cost_low: Decimal,
    pub setup_cost_high: Decimal,
    pub annual_cost_low: Decimal,
    pub annual_cost_high: Decimal,
    pub dfsa_fees: Decimal,
    pub sharia_premium: Decimal,
    pub total_annual_estimate: Decimal,
    pub cost_pct_of_aum: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DifcDistributionAnalysis {
    pub gcc_access: bool,
    pub bilateral_eu_treaties: bool,
    pub target_markets: Vec<String>,
    pub passport_notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DifcFundOutput {
    pub fund_name: String,
    pub jurisdiction: String,
    pub structure_analysis: DifcStructureAnalysis,
    pub regulatory: DifcRegulatoryAnalysis,
    pub tax_analysis: DifcTaxAnalysis,
    pub sharia_analysis: DifcShariaAnalysis,
    pub cost_analysis: DifcCostAnalysis,
    pub distribution_analysis: DifcDistributionAnalysis,
    pub recommendations: Vec<String>,
    pub warnings: Vec<String>,
}

// ---------------------------------------------------------------------------
// Output types — ADGM
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdgmStructureAnalysis {
    pub fund_type: String,
    pub structure_type: String,
    pub description: String,
    pub legal_framework: String,
    pub minimum_subscription: Decimal,
    pub max_investors: Option<u32>,
    pub tax_guarantee_years: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdgmCommonLawAnalysis {
    pub legal_system: String,
    pub court_system: String,
    pub enforcement_notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdgmRegulatoryAnalysis {
    pub fsra_license_category: String,
    pub fsra_annual_fee: Decimal,
    pub compliance_officer_required: bool,
    pub audit_required: bool,
    pub regulatory_reporting: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdgmCostAnalysis {
    pub setup_cost_low: Decimal,
    pub setup_cost_high: Decimal,
    pub annual_cost_low: Decimal,
    pub annual_cost_high: Decimal,
    pub fsra_fees: Decimal,
    pub total_annual_estimate: Decimal,
    pub cost_pct_of_aum: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdgmDifcComparison {
    pub adgm_advantage: Vec<String>,
    pub difc_advantage: Vec<String>,
    pub neutral: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdgmFundOutput {
    pub fund_name: String,
    pub jurisdiction: String,
    pub structure_analysis: AdgmStructureAnalysis,
    pub common_law_analysis: AdgmCommonLawAnalysis,
    pub regulatory: AdgmRegulatoryAnalysis,
    pub tax_analysis: DifcTaxAnalysis,
    pub cost_analysis: AdgmCostAnalysis,
    pub comparison_to_difc: AdgmDifcComparison,
    pub recommendations: Vec<String>,
    pub warnings: Vec<String>,
}

// ---------------------------------------------------------------------------
// Output types — Sharia Compliance
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShariaScreeningResult {
    pub company_name: String,
    pub debt_ratio_pass: bool,
    pub interest_income_pass: bool,
    pub non_permissible_pass: bool,
    pub overall_pass: bool,
    pub purification_amount: Decimal,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SsbGovernance {
    pub member_count: u32,
    pub qualified_scholars: u32,
    pub meets_minimum: bool,
    pub has_qualified_scholar: bool,
    pub governance_notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShariaComplianceOutput {
    pub screening_methodology: String,
    pub screening_results: Vec<ShariaScreeningResult>,
    pub ssb_governance: SsbGovernance,
    pub investment_restrictions: Vec<String>,
    pub total_purification_amount: Decimal,
    pub compliance_score: u32,
    pub cost_premium_vs_conventional: Decimal,
    pub recommendations: Vec<String>,
    pub warnings: Vec<String>,
}

// ---------------------------------------------------------------------------
// Output types — Comparison
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JurisdictionDimension {
    pub dimension: String,
    pub difc_value: String,
    pub adgm_value: String,
    pub advantage: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DifcAdgmCompOutput {
    pub dimensions: Vec<JurisdictionDimension>,
    pub recommended_jurisdiction: String,
    pub rationale: Vec<String>,
    pub difc_score: u32,
    pub adgm_score: u32,
    pub warnings: Vec<String>,
}

// ---------------------------------------------------------------------------
// Public API — DIFC Fund Analysis
// ---------------------------------------------------------------------------

pub fn analyze_difc_fund(input: &DifcFundInput) -> CorpFinanceResult<DifcFundOutput> {
    validate_difc_input(input)?;

    let mut warnings: Vec<String> = Vec::new();
    let mut recommendations: Vec<String> = Vec::new();

    // ------------------------------------------------------------------
    // 1. Structure Analysis
    // ------------------------------------------------------------------
    let structure_analysis =
        build_difc_structure_analysis(input, &mut recommendations, &mut warnings)?;

    // ------------------------------------------------------------------
    // 2. Regulatory Analysis
    // ------------------------------------------------------------------
    let regulatory = build_difc_regulatory(input, &mut recommendations);

    // ------------------------------------------------------------------
    // 3. Tax Analysis
    // ------------------------------------------------------------------
    let tax_analysis = build_difc_tax_analysis(&input.target_investors);

    // ------------------------------------------------------------------
    // 4. Sharia Analysis
    // ------------------------------------------------------------------
    let sharia_analysis = build_difc_sharia_analysis(input, &mut recommendations, &mut warnings);

    // ------------------------------------------------------------------
    // 5. Cost Analysis
    // ------------------------------------------------------------------
    let cost_analysis = build_difc_cost_analysis(input, &sharia_analysis);

    // ------------------------------------------------------------------
    // 6. Distribution Analysis
    // ------------------------------------------------------------------
    let distribution_analysis =
        build_difc_distribution_analysis(&input.target_investors, &mut recommendations);

    // ------------------------------------------------------------------
    // 7. Final warnings
    // ------------------------------------------------------------------
    if cost_analysis.cost_pct_of_aum > dec!(0.005) {
        warnings.push(format!(
            "Total annual cost is {:.2}% of AUM, above the typical 0.50% threshold",
            cost_analysis.cost_pct_of_aum * dec!(100)
        ));
    }

    Ok(DifcFundOutput {
        fund_name: input.fund_name.clone(),
        jurisdiction: "DIFC (Dubai International Financial Centre)".to_string(),
        structure_analysis,
        regulatory,
        tax_analysis,
        sharia_analysis,
        cost_analysis,
        distribution_analysis,
        recommendations,
        warnings,
    })
}

// ---------------------------------------------------------------------------
// Public API — ADGM Fund Analysis
// ---------------------------------------------------------------------------

pub fn analyze_adgm_fund(input: &AdgmFundInput) -> CorpFinanceResult<AdgmFundOutput> {
    validate_adgm_input(input)?;

    let mut warnings: Vec<String> = Vec::new();
    let mut recommendations: Vec<String> = Vec::new();

    // ------------------------------------------------------------------
    // 1. Structure Analysis
    // ------------------------------------------------------------------
    let structure_analysis =
        build_adgm_structure_analysis(input, &mut recommendations, &mut warnings)?;

    // ------------------------------------------------------------------
    // 2. Common Law Analysis
    // ------------------------------------------------------------------
    let common_law_analysis = build_adgm_common_law_analysis(&mut recommendations);

    // ------------------------------------------------------------------
    // 3. Regulatory Analysis
    // ------------------------------------------------------------------
    let regulatory = build_adgm_regulatory(input, &mut recommendations);

    // ------------------------------------------------------------------
    // 4. Tax Analysis (identical 0% regime)
    // ------------------------------------------------------------------
    let tax_analysis = build_adgm_tax_analysis(&input.target_investors);

    // ------------------------------------------------------------------
    // 5. Cost Analysis
    // ------------------------------------------------------------------
    let cost_analysis = build_adgm_cost_analysis(input);

    // ------------------------------------------------------------------
    // 6. Comparison to DIFC
    // ------------------------------------------------------------------
    let comparison_to_difc = build_adgm_difc_comparison(input);

    // ------------------------------------------------------------------
    // 7. Final warnings
    // ------------------------------------------------------------------
    if cost_analysis.cost_pct_of_aum > dec!(0.005) {
        warnings.push(format!(
            "Total annual cost is {:.2}% of AUM, above the typical 0.50% threshold",
            cost_analysis.cost_pct_of_aum * dec!(100)
        ));
    }

    Ok(AdgmFundOutput {
        fund_name: input.fund_name.clone(),
        jurisdiction: "ADGM (Abu Dhabi Global Market)".to_string(),
        structure_analysis,
        common_law_analysis,
        regulatory,
        tax_analysis,
        cost_analysis,
        comparison_to_difc,
        recommendations,
        warnings,
    })
}

// ---------------------------------------------------------------------------
// Public API — Sharia Compliance Check
// ---------------------------------------------------------------------------

pub fn sharia_compliance_check(
    input: &ShariaComplianceInput,
) -> CorpFinanceResult<ShariaComplianceOutput> {
    validate_sharia_input(input)?;

    let mut warnings: Vec<String> = Vec::new();
    let mut recommendations: Vec<String> = Vec::new();

    // ------------------------------------------------------------------
    // 1. Screen each company
    // ------------------------------------------------------------------
    let mut screening_results: Vec<ShariaScreeningResult> = Vec::new();
    let mut total_purification = Decimal::ZERO;
    let mut pass_count: u32 = 0;

    let (debt_threshold, interest_threshold, non_perm_threshold) =
        screening_thresholds(&input.screening_methodology);

    for company in &input.companies {
        let debt_pass = company.debt_to_market_cap < debt_threshold;
        let interest_pass = company.interest_income_pct < interest_threshold;
        let non_perm_pass = company.non_permissible_revenue_pct < non_perm_threshold;
        let overall = debt_pass && interest_pass && non_perm_pass;

        if overall {
            pass_count += 1;
        }

        let mut notes = Vec::new();
        if !debt_pass {
            notes.push(format!(
                "Debt-to-market-cap {:.1}% exceeds {:.0}% threshold",
                company.debt_to_market_cap * dec!(100),
                debt_threshold * dec!(100)
            ));
        }
        if !interest_pass {
            notes.push(format!(
                "Interest income {:.1}% exceeds {:.0}% threshold",
                company.interest_income_pct * dec!(100),
                interest_threshold * dec!(100)
            ));
        }
        if !non_perm_pass {
            notes.push(format!(
                "Non-permissible revenue {:.1}% exceeds {:.0}% threshold",
                company.non_permissible_revenue_pct * dec!(100),
                non_perm_threshold * dec!(100)
            ));
        }

        // Purification: dividend * (non_permissible_pct / total_revenue)
        // We approximate total_revenue as 1.0 (pct is already a ratio)
        let purification = company.total_dividends * company.non_permissible_revenue_pct;
        total_purification += purification;

        screening_results.push(ShariaScreeningResult {
            company_name: company.name.clone(),
            debt_ratio_pass: debt_pass,
            interest_income_pass: interest_pass,
            non_permissible_pass: non_perm_pass,
            overall_pass: overall,
            purification_amount: purification,
            notes,
        });
    }

    // ------------------------------------------------------------------
    // 2. SSB Governance
    // ------------------------------------------------------------------
    let meets_minimum = input.ssb_member_count >= 3;
    let has_qualified = input.ssb_qualified_scholars >= 1;

    let mut governance_notes = Vec::new();
    if !meets_minimum {
        governance_notes.push(format!(
            "SSB has {} members, minimum 3 required",
            input.ssb_member_count
        ));
        warnings.push("Sharia Supervisory Board does not meet minimum 3-member requirement".into());
    }
    if !has_qualified {
        governance_notes.push("No qualified Sharia scholar on SSB".into());
        warnings.push("At least 1 qualified Sharia scholar required on SSB".into());
    }
    if meets_minimum && has_qualified {
        governance_notes.push("SSB governance meets AAOIFI standards".into());
    }

    let ssb_governance = SsbGovernance {
        member_count: input.ssb_member_count,
        qualified_scholars: input.ssb_qualified_scholars,
        meets_minimum,
        has_qualified_scholar: has_qualified,
        governance_notes,
    };

    // ------------------------------------------------------------------
    // 3. Investment Restrictions
    // ------------------------------------------------------------------
    let investment_restrictions = vec![
        "No conventional financial services (banks, insurance)".to_string(),
        "No alcohol production or distribution".to_string(),
        "No pork-related products".to_string(),
        "No gambling or gaming operations".to_string(),
        "No tobacco manufacturing".to_string(),
        "No weapons or defense manufacturing".to_string(),
        "No adult entertainment".to_string(),
    ];

    // ------------------------------------------------------------------
    // 4. Compliance Score (0-100)
    // ------------------------------------------------------------------
    let total_companies = input.companies.len() as u32;
    let screening_score = if total_companies > 0 {
        (pass_count * 40) / total_companies
    } else {
        40
    };
    let ssb_score: u32 = if meets_minimum { 20 } else { 0 } + if has_qualified { 20 } else { 0 };
    let methodology_score: u32 = match input.screening_methodology.as_str() {
        "AAOIFI" => 20,
        "DJIM" | "MSCI" | "FTSE" => 15,
        _ => 10,
    };
    let compliance_score = (screening_score + ssb_score + methodology_score).min(100);

    // ------------------------------------------------------------------
    // 5. Cost Premium
    // ------------------------------------------------------------------
    // Sharia-compliant funds typically cost 15-25% more than conventional
    let cost_premium = dec!(0.20);

    // ------------------------------------------------------------------
    // 6. Recommendations
    // ------------------------------------------------------------------
    if input.screening_methodology != "AAOIFI" {
        recommendations.push(format!(
            "Consider AAOIFI screening methodology for GCC investor acceptance \
             (currently using {})",
            input.screening_methodology
        ));
    }

    if input.purification_method == "Income" {
        recommendations.push(
            "Income purification distributes non-permissible earnings to charity; \
             ensure proper documentation for audit trail"
                .to_string(),
        );
    }

    if total_purification > Decimal::ZERO {
        recommendations.push(format!(
            "Total purification amount of {:.2} should be donated to eligible charities \
             (not zakat-eligible causes)",
            total_purification
        ));
    }

    Ok(ShariaComplianceOutput {
        screening_methodology: input.screening_methodology.clone(),
        screening_results,
        ssb_governance,
        investment_restrictions,
        total_purification_amount: total_purification,
        compliance_score,
        cost_premium_vs_conventional: cost_premium,
        recommendations,
        warnings,
    })
}

// ---------------------------------------------------------------------------
// Public API — DIFC vs ADGM Comparison
// ---------------------------------------------------------------------------

pub fn difc_adgm_comparison(input: &DifcAdgmCompInput) -> CorpFinanceResult<DifcAdgmCompOutput> {
    validate_comparison_input(input)?;

    let mut warnings: Vec<String> = Vec::new();
    let mut difc_score: u32 = 0;
    let mut adgm_score: u32 = 0;

    let mut dimensions = Vec::new();

    // 1. Legal System
    let legal_advantage = if input.preferred_legal_system == "EnglishCommonLaw" {
        adgm_score += 2;
        "ADGM"
    } else {
        // CivilLaw or any other preference: neutral between the two
        difc_score += 1;
        adgm_score += 1;
        "Neutral"
    };
    dimensions.push(JurisdictionDimension {
        dimension: "Legal System".to_string(),
        difc_value: "DIFC Courts (independent common law courts, own precedent)".to_string(),
        adgm_value: "English common law (direct application, ADGM Courts)".to_string(),
        advantage: legal_advantage.to_string(),
    });

    // 2. Setup Cost
    let (difc_setup_low, difc_setup_high) = difc_setup_cost_range(&input.fund_strategy);
    let (adgm_setup_low, adgm_setup_high) = adgm_setup_cost_range(&input.fund_strategy);
    let setup_advantage = if adgm_setup_high < difc_setup_high {
        adgm_score += 1;
        "ADGM"
    } else {
        difc_score += 1;
        "DIFC"
    };
    dimensions.push(JurisdictionDimension {
        dimension: "Setup Cost".to_string(),
        difc_value: format!("USD {}-{}", difc_setup_low, difc_setup_high),
        adgm_value: format!("USD {}-{}", adgm_setup_low, adgm_setup_high),
        advantage: setup_advantage.to_string(),
    });

    // 3. Annual Cost
    let (difc_ann_low, difc_ann_high) = difc_annual_cost_range(&input.fund_strategy);
    let (adgm_ann_low, adgm_ann_high) = adgm_annual_cost_range(&input.fund_strategy);
    let annual_advantage = if adgm_ann_high < difc_ann_high {
        adgm_score += 1;
        "ADGM"
    } else {
        difc_score += 1;
        "DIFC"
    };
    dimensions.push(JurisdictionDimension {
        dimension: "Annual Cost".to_string(),
        difc_value: format!("USD {}-{}", difc_ann_low, difc_ann_high),
        adgm_value: format!("USD {}-{}", adgm_ann_low, adgm_ann_high),
        advantage: annual_advantage.to_string(),
    });

    // 4. Fund Types
    dimensions.push(JurisdictionDimension {
        dimension: "Fund Types".to_string(),
        difc_value: "QIF, Exempt Fund, Domestic Fund".to_string(),
        adgm_value: "Exempt, QIF, Public".to_string(),
        advantage: "Neutral".to_string(),
    });
    difc_score += 1;
    adgm_score += 1;

    // 5. Regulatory Timeline
    dimensions.push(JurisdictionDimension {
        dimension: "Regulatory Timeline".to_string(),
        difc_value: "QIF: 5-day notification; Exempt: 4-6 weeks; Domestic: 8-12 weeks".to_string(),
        adgm_value: "Exempt: 2-4 weeks; QIF: 4-6 weeks; Public: 8-12 weeks".to_string(),
        advantage: "Neutral".to_string(),
    });
    difc_score += 1;
    adgm_score += 1;

    // 6. Sharia Framework
    let sharia_advantage = if input.sharia_required {
        difc_score += 2;
        "DIFC"
    } else {
        difc_score += 1;
        adgm_score += 1;
        "Neutral"
    };
    dimensions.push(JurisdictionDimension {
        dimension: "Sharia Framework".to_string(),
        difc_value: "Mature Sharia ecosystem, extensive SSB pool, AAOIFI aligned".to_string(),
        adgm_value: "Growing Sharia framework, fewer established scholars locally".to_string(),
        advantage: sharia_advantage.to_string(),
    });

    // 7. Distribution Reach
    let has_eu = input
        .target_investors
        .iter()
        .any(|t| t == "EU" || t == "Europe");
    let dist_advantage = if has_eu {
        difc_score += 1;
        "DIFC"
    } else {
        difc_score += 1;
        adgm_score += 1;
        "Neutral"
    };
    dimensions.push(JurisdictionDimension {
        dimension: "Distribution Reach".to_string(),
        difc_value: "GCC access, bilateral EU treaties, 40+ DTAs".to_string(),
        adgm_value: "GCC access, growing treaty network, 30+ DTAs".to_string(),
        advantage: dist_advantage.to_string(),
    });

    // 8. Service Provider Ecosystem
    difc_score += 2;
    adgm_score += 1;
    dimensions.push(JurisdictionDimension {
        dimension: "Service Provider Ecosystem".to_string(),
        difc_value: "Established since 2004, deep ecosystem of administrators, auditors, law firms"
            .to_string(),
        adgm_value:
            "Established 2015, rapidly growing ecosystem but narrower service provider base"
                .to_string(),
        advantage: "DIFC".to_string(),
    });

    // 9. Reputational Score
    difc_score += 2;
    adgm_score += 1;
    dimensions.push(JurisdictionDimension {
        dimension: "Reputational Score".to_string(),
        difc_value: "Tier-1 IFC, 20+ year track record, 4000+ registered entities".to_string(),
        adgm_value: "Emerging tier-1 IFC, strong growth trajectory, 1500+ entities".to_string(),
        advantage: "DIFC".to_string(),
    });

    // Determine recommendation
    let recommended = if difc_score > adgm_score {
        "DIFC".to_string()
    } else if adgm_score > difc_score {
        "ADGM".to_string()
    } else {
        "Either — both jurisdictions equally suitable".to_string()
    };

    let mut rationale = Vec::new();
    if difc_score > adgm_score {
        rationale
            .push("DIFC has a deeper service provider ecosystem and longer track record".into());
        if input.sharia_required {
            rationale.push("DIFC offers a more mature Sharia-compliant fund framework".into());
        }
    } else if adgm_score > difc_score {
        rationale.push(
            "ADGM offers English common law (direct application) and lower setup costs".into(),
        );
    } else {
        rationale.push(
            "Both jurisdictions offer equivalent 0% tax regimes and similar fund structures".into(),
        );
    }

    if input.fund_size < dec!(50_000_000) {
        warnings.push(
            "Fund size below USD 50M may face challenges attracting institutional \
             service providers in either jurisdiction"
                .into(),
        );
    }

    Ok(DifcAdgmCompOutput {
        dimensions,
        recommended_jurisdiction: recommended,
        rationale,
        difc_score,
        adgm_score,
        warnings,
    })
}

// ---------------------------------------------------------------------------
// Helpers — DIFC Structure
// ---------------------------------------------------------------------------

fn build_difc_structure_analysis(
    input: &DifcFundInput,
    recommendations: &mut Vec<String>,
    warnings: &mut Vec<String>,
) -> CorpFinanceResult<DifcStructureAnalysis> {
    let (description, max_investors, dfsa_process, timeline) = match input.fund_type.as_str() {
        "QIF" => {
            if input.minimum_subscription < dec!(500_000) {
                return Err(CorpFinanceError::InvalidInput {
                    field: "minimum_subscription".into(),
                    reason: format!(
                        "QIF requires minimum subscription of USD 500,000; got {}",
                        input.minimum_subscription
                    ),
                });
            }
            recommendations
                .push("QIF offers fastest time-to-market with 5-day DFSA notification only".into());
            (
                "Qualified Investor Fund — professional/institutional investors only".to_string(),
                100u32,
                "5-day DFSA notification (no prior approval required)".to_string(),
                "1-2 weeks".to_string(),
            )
        }
        "ExemptFund" => {
            if input.minimum_subscription < dec!(50_000) {
                return Err(CorpFinanceError::InvalidInput {
                    field: "minimum_subscription".into(),
                    reason: format!(
                        "Exempt Fund requires minimum subscription of USD 50,000; got {}",
                        input.minimum_subscription
                    ),
                });
            }
            if input.investor_count > 100 {
                return Err(CorpFinanceError::InvalidInput {
                    field: "investor_count".into(),
                    reason: format!(
                        "Exempt Fund maximum 100 investors; got {}",
                        input.investor_count
                    ),
                });
            }
            (
                "Exempt Fund — limited to 100 investors, DFSA registration required".to_string(),
                100,
                "DFSA registration (application and review)".to_string(),
                "4-6 weeks".to_string(),
            )
        }
        "DomesticFund" => {
            recommendations.push(
                "Domestic Fund requires full DFSA authorization; \
                 ensure adequate lead time (8-12 weeks)"
                    .into(),
            );
            (
                "Domestic Fund — retail-eligible, full DFSA authorization".to_string(),
                u32::MAX,
                "Full DFSA authorization (prospectus review, approval)".to_string(),
                "8-12 weeks".to_string(),
            )
        }
        other => {
            return Err(CorpFinanceError::InvalidInput {
                field: "fund_type".into(),
                reason: format!(
                    "Unknown DIFC fund type '{}'. Expected: QIF, ExemptFund, DomesticFund",
                    other
                ),
            });
        }
    };

    if input.investor_count > max_investors && max_investors != u32::MAX {
        warnings.push(format!(
            "{} fund type has maximum {} investors; current count is {}",
            input.fund_type, max_investors, input.investor_count
        ));
    }

    Ok(DifcStructureAnalysis {
        fund_type: input.fund_type.clone(),
        description,
        minimum_subscription: input.minimum_subscription,
        max_investors,
        dfsa_process,
        approval_timeline_weeks: timeline,
        tax_guarantee_years: 50,
    })
}

fn build_difc_regulatory(
    input: &DifcFundInput,
    recommendations: &mut Vec<String>,
) -> DifcRegulatoryAnalysis {
    // DFSA Category 3C license for fund managers
    let dfsa_fee = match input.fund_type.as_str() {
        "QIF" => dec!(10_000),
        "ExemptFund" => dec!(15_000),
        "DomesticFund" => dec!(20_000),
        _ => dec!(15_000),
    };

    recommendations.push("Appoint a DFSA-licensed fund manager with Category 3C license".into());

    if input.fund_size >= dec!(500_000_000) {
        recommendations
            .push("Large fund (>$500M): consider dedicated compliance team in DIFC".into());
    }

    DifcRegulatoryAnalysis {
        dfsa_license_category: "Category 3C (Collective Investment Fund Management)".to_string(),
        dfsa_annual_fee: dfsa_fee,
        compliance_officer_required: true,
        mlro_required: true,
        audit_required: true,
        regulatory_reporting: match input.fund_type.as_str() {
            "QIF" => "Annual audited financial statements".to_string(),
            "ExemptFund" => "Semi-annual reporting to DFSA".to_string(),
            "DomesticFund" => "Quarterly reporting to DFSA, monthly NAV".to_string(),
            _ => "Annual reporting".to_string(),
        },
    }
}

fn build_difc_tax_analysis(target_investors: &[String]) -> DifcTaxAnalysis {
    let mut distribution_notes = vec![
        "0% tax on fund income (50-year DIFC guarantee)".to_string(),
        "0% withholding tax on distributions to all investors".to_string(),
        "No capital gains tax on disposals".to_string(),
    ];

    if target_investors.iter().any(|t| t == "GCC") {
        distribution_notes
            .push("GCC investors: no additional tax in most GCC jurisdictions".to_string());
    }
    if target_investors.iter().any(|t| t == "EU" || t == "Europe") {
        distribution_notes.push(
            "EU investors: check bilateral treaty network for withholding tax relief".to_string(),
        );
    }

    DifcTaxAnalysis {
        corporate_tax_rate: Decimal::ZERO,
        withholding_tax_rate: Decimal::ZERO,
        capital_gains_tax_rate: Decimal::ZERO,
        vat_applicable: false,
        tax_guarantee_period: "50 years from DIFC establishment (2004)".to_string(),
        distribution_tax_notes: distribution_notes,
    }
}

fn build_difc_sharia_analysis(
    input: &DifcFundInput,
    recommendations: &mut Vec<String>,
    warnings: &mut Vec<String>,
) -> DifcShariaAnalysis {
    if !input.sharia_compliant {
        return DifcShariaAnalysis {
            sharia_compliant: false,
            ssb_required: false,
            ssb_member_count: 0,
            ssb_adequate: true,
            aaoifi_compliance: false,
            cost_premium_pct: Decimal::ZERO,
        };
    }

    let ssb_adequate = input.sharia_board_members >= 3;
    if !ssb_adequate {
        warnings.push(format!(
            "Sharia fund requires minimum 3 SSB members; only {} provided",
            input.sharia_board_members
        ));
    }

    recommendations.push("Ensure SSB includes at least 1 AAOIFI-recognized Sharia scholar".into());
    recommendations.push(
        "Implement ongoing Sharia compliance monitoring and periodic portfolio purification".into(),
    );

    DifcShariaAnalysis {
        sharia_compliant: true,
        ssb_required: true,
        ssb_member_count: input.sharia_board_members,
        ssb_adequate,
        aaoifi_compliance: true,
        cost_premium_pct: dec!(0.20),
    }
}

fn build_difc_cost_analysis(
    input: &DifcFundInput,
    sharia: &DifcShariaAnalysis,
) -> DifcCostAnalysis {
    let (setup_low, setup_high) = difc_setup_cost_range(&input.fund_strategy);
    let (annual_low, annual_high) = difc_annual_cost_range(&input.fund_strategy);

    let dfsa_fee = match input.fund_type.as_str() {
        "QIF" => dec!(10_000),
        "ExemptFund" => dec!(15_000),
        "DomesticFund" => dec!(20_000),
        _ => dec!(15_000),
    };

    let sharia_premium = if sharia.sharia_compliant {
        // SSB fees + Sharia audit + compliance monitoring
        dec!(30_000)
    } else {
        Decimal::ZERO
    };

    // Mid-point estimate for total annual
    let mid_annual = (annual_low + annual_high) / dec!(2);
    let total_annual = mid_annual + dfsa_fee + sharia_premium;

    let cost_pct = if input.fund_size > Decimal::ZERO {
        total_annual / input.fund_size
    } else {
        Decimal::ZERO
    };

    DifcCostAnalysis {
        setup_cost_low: setup_low,
        setup_cost_high: setup_high,
        annual_cost_low: annual_low,
        annual_cost_high: annual_high,
        dfsa_fees: dfsa_fee,
        sharia_premium,
        total_annual_estimate: total_annual,
        cost_pct_of_aum: cost_pct,
    }
}

fn build_difc_distribution_analysis(
    target_investors: &[String],
    recommendations: &mut Vec<String>,
) -> DifcDistributionAnalysis {
    let gcc_access = true;
    let bilateral_eu = target_investors.iter().any(|t| t == "EU" || t == "Europe");

    let mut passport_notes = vec![
        "DIFC funds can distribute to professional investors across GCC".to_string(),
        "No passporting regime equivalent to EU AIFMD; bilateral arrangements apply".to_string(),
    ];

    if bilateral_eu {
        passport_notes.push(
            "EU distribution requires bilateral treaty or national private placement".to_string(),
        );
        recommendations
            .push("For EU distribution, consider appointing an EU-based placement agent".into());
    }

    let target_markets: Vec<String> = target_investors
        .iter()
        .map(|t| match t.as_str() {
            "GCC" => "Gulf Cooperation Council (Saudi Arabia, UAE, Kuwait, Bahrain, Oman, Qatar)"
                .to_string(),
            "EU" | "Europe" => "European Union (via bilateral arrangements)".to_string(),
            "Asia" => "Asia-Pacific (bilateral arrangements)".to_string(),
            other => other.to_string(),
        })
        .collect();

    DifcDistributionAnalysis {
        gcc_access,
        bilateral_eu_treaties: bilateral_eu,
        target_markets,
        passport_notes,
    }
}

// ---------------------------------------------------------------------------
// Helpers — ADGM Structure
// ---------------------------------------------------------------------------

fn build_adgm_structure_analysis(
    input: &AdgmFundInput,
    recommendations: &mut Vec<String>,
    _warnings: &mut [String],
) -> CorpFinanceResult<AdgmStructureAnalysis> {
    let valid_types = ["Exempt", "QIF", "Public"];
    if !valid_types.contains(&input.fund_type.as_str()) {
        return Err(CorpFinanceError::InvalidInput {
            field: "fund_type".into(),
            reason: format!(
                "Unknown ADGM fund type '{}'. Expected: {:?}",
                input.fund_type, valid_types
            ),
        });
    }

    let valid_structures = ["InvestmentCompany", "InvestmentTrust", "LP"];
    if !valid_structures.contains(&input.structure_type.as_str()) {
        return Err(CorpFinanceError::InvalidInput {
            field: "structure_type".into(),
            reason: format!(
                "Unknown ADGM structure '{}'. Expected: {:?}",
                input.structure_type, valid_structures
            ),
        });
    }

    let (description, max_investors) = match input.fund_type.as_str() {
        "Exempt" => (
            "Exempt Fund — limited distribution, streamlined FSRA approval".to_string(),
            Some(100u32),
        ),
        "QIF" => (
            "Qualified Investor Fund — professional/institutional investors".to_string(),
            Some(100u32),
        ),
        "Public" => (
            "Public Fund — retail-eligible, full FSRA authorization required".to_string(),
            None,
        ),
        _ => unreachable!(),
    };

    let legal_framework = match input.structure_type.as_str() {
        "InvestmentCompany" => "ADGM Companies Regulations 2020 (English common law basis)",
        "InvestmentTrust" => "ADGM Trust Regulations (English trust law basis)",
        "LP" => "ADGM Limited Partnership Regulations (English common law LP)",
        _ => unreachable!(),
    };

    if input.structure_type == "LP" {
        recommendations.push(
            "LP structure is most common for PE/VC strategies in ADGM; \
             ensure GP entity is also ADGM-registered"
                .into(),
        );
    }

    Ok(AdgmStructureAnalysis {
        fund_type: input.fund_type.clone(),
        structure_type: input.structure_type.clone(),
        description,
        legal_framework: legal_framework.to_string(),
        minimum_subscription: input.minimum_subscription,
        max_investors,
        tax_guarantee_years: 50,
    })
}

fn build_adgm_common_law_analysis(recommendations: &mut Vec<String>) -> AdgmCommonLawAnalysis {
    recommendations.push(
        "ADGM applies English common law directly — legal opinions from \
         English-qualified counsel are directly applicable"
            .into(),
    );

    AdgmCommonLawAnalysis {
        legal_system: "English common law (direct application)".to_string(),
        court_system: "ADGM Courts — English-qualified judges, appeals to Abu Dhabi Global \
                       Market Courts of Appeal"
            .to_string(),
        enforcement_notes: vec![
            "Judgments enforceable across UAE via Abu Dhabi Judicial Department".to_string(),
            "English common law precedent directly applicable".to_string(),
            "Arbitration supported via ADGM Arbitration Centre".to_string(),
        ],
    }
}

fn build_adgm_regulatory(
    input: &AdgmFundInput,
    recommendations: &mut Vec<String>,
) -> AdgmRegulatoryAnalysis {
    let fsra_fee = match input.fund_type.as_str() {
        "Exempt" => dec!(10_000),
        "QIF" => dec!(12_000),
        "Public" => dec!(20_000),
        _ => dec!(12_000),
    };

    recommendations.push("Appoint FSRA-authorized fund manager in ADGM".into());

    AdgmRegulatoryAnalysis {
        fsra_license_category: "Fund Manager (Category 3C equivalent)".to_string(),
        fsra_annual_fee: fsra_fee,
        compliance_officer_required: true,
        audit_required: true,
        regulatory_reporting: match input.fund_type.as_str() {
            "Exempt" => "Annual audited financial statements".to_string(),
            "QIF" => "Semi-annual reporting to FSRA".to_string(),
            "Public" => "Quarterly reporting to FSRA, monthly NAV".to_string(),
            _ => "Annual reporting".to_string(),
        },
    }
}

fn build_adgm_tax_analysis(target_investors: &[String]) -> DifcTaxAnalysis {
    let mut distribution_notes = vec![
        "0% tax on fund income (50-year ADGM guarantee)".to_string(),
        "0% withholding tax on distributions".to_string(),
        "No capital gains tax".to_string(),
    ];

    if target_investors.iter().any(|t| t == "GCC") {
        distribution_notes
            .push("GCC investors: no additional tax in most GCC jurisdictions".to_string());
    }

    DifcTaxAnalysis {
        corporate_tax_rate: Decimal::ZERO,
        withholding_tax_rate: Decimal::ZERO,
        capital_gains_tax_rate: Decimal::ZERO,
        vat_applicable: false,
        tax_guarantee_period: "50 years from ADGM establishment (2015)".to_string(),
        distribution_tax_notes: distribution_notes,
    }
}

fn build_adgm_cost_analysis(input: &AdgmFundInput) -> AdgmCostAnalysis {
    let (setup_low, setup_high) = adgm_setup_cost_range(&input.fund_strategy);
    let (annual_low, annual_high) = adgm_annual_cost_range(&input.fund_strategy);

    let fsra_fee = match input.fund_type.as_str() {
        "Exempt" => dec!(10_000),
        "QIF" => dec!(12_000),
        "Public" => dec!(20_000),
        _ => dec!(12_000),
    };

    let mid_annual = (annual_low + annual_high) / dec!(2);
    let total_annual = mid_annual + fsra_fee;

    let cost_pct = if input.fund_size > Decimal::ZERO {
        total_annual / input.fund_size
    } else {
        Decimal::ZERO
    };

    AdgmCostAnalysis {
        setup_cost_low: setup_low,
        setup_cost_high: setup_high,
        annual_cost_low: annual_low,
        annual_cost_high: annual_high,
        fsra_fees: fsra_fee,
        total_annual_estimate: total_annual,
        cost_pct_of_aum: cost_pct,
    }
}

fn build_adgm_difc_comparison(input: &AdgmFundInput) -> AdgmDifcComparison {
    let mut adgm_adv = vec![
        "English common law directly applied (not adapted)".to_string(),
        "Generally lower setup and annual costs".to_string(),
        "Streamlined FSRA approval process".to_string(),
    ];

    let mut difc_adv = vec![
        "Deeper service provider ecosystem (established 2004)".to_string(),
        "More established Sharia-compliant fund framework".to_string(),
        "Broader bilateral treaty network (40+ DTAs)".to_string(),
    ];

    let neutral = vec![
        "Both offer 0% tax regime with 50-year guarantee".to_string(),
        "Both are free zones with independent regulatory frameworks".to_string(),
        "Both offer QIF/Exempt/Public fund categories".to_string(),
    ];

    if input.sharia_compliant {
        difc_adv.push("More mature Sharia ecosystem with deeper SSB scholar pool".to_string());
    }

    if input.structure_type == "LP" {
        adgm_adv.push("LP structure benefits from English LP Act precedent".to_string());
    }

    AdgmDifcComparison {
        adgm_advantage: adgm_adv,
        difc_advantage: difc_adv,
        neutral,
    }
}

// ---------------------------------------------------------------------------
// Helpers — Cost Ranges
// ---------------------------------------------------------------------------

fn difc_setup_cost_range(_strategy: &str) -> (Decimal, Decimal) {
    (dec!(40_000), dec!(80_000))
}

fn difc_annual_cost_range(_strategy: &str) -> (Decimal, Decimal) {
    (dec!(100_000), dec!(250_000))
}

fn adgm_setup_cost_range(_strategy: &str) -> (Decimal, Decimal) {
    (dec!(30_000), dec!(70_000))
}

fn adgm_annual_cost_range(_strategy: &str) -> (Decimal, Decimal) {
    (dec!(80_000), dec!(200_000))
}

// ---------------------------------------------------------------------------
// Helpers — Sharia Screening Thresholds
// ---------------------------------------------------------------------------

fn screening_thresholds(methodology: &str) -> (Decimal, Decimal, Decimal) {
    // Returns (debt_threshold, interest_threshold, non_permissible_threshold)
    match methodology {
        "AAOIFI" => (dec!(0.33), dec!(0.05), dec!(0.05)),
        "DJIM" => (dec!(0.33), dec!(0.05), dec!(0.05)),
        "MSCI" => (dec!(0.3333), dec!(0.05), dec!(0.05)),
        "FTSE" => (dec!(0.33), dec!(0.05), dec!(0.05)),
        _ => (dec!(0.33), dec!(0.05), dec!(0.05)),
    }
}

// ---------------------------------------------------------------------------
// Validation — DIFC
// ---------------------------------------------------------------------------

fn validate_difc_input(input: &DifcFundInput) -> CorpFinanceResult<()> {
    if input.fund_name.trim().is_empty() {
        return Err(CorpFinanceError::InvalidInput {
            field: "fund_name".into(),
            reason: "Fund name cannot be empty".into(),
        });
    }

    let valid_types = ["QIF", "ExemptFund", "DomesticFund"];
    if !valid_types.contains(&input.fund_type.as_str()) {
        return Err(CorpFinanceError::InvalidInput {
            field: "fund_type".into(),
            reason: format!(
                "Unknown DIFC fund type '{}'. Valid: {:?}",
                input.fund_type, valid_types
            ),
        });
    }

    if input.fund_size <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "fund_size".into(),
            reason: "Fund size must be greater than zero".into(),
        });
    }

    if input.management_fee_rate < Decimal::ZERO || input.management_fee_rate >= Decimal::ONE {
        return Err(CorpFinanceError::InvalidInput {
            field: "management_fee_rate".into(),
            reason: "Management fee rate must be >= 0 and < 1".into(),
        });
    }

    if input.performance_fee_rate < Decimal::ZERO || input.performance_fee_rate >= Decimal::ONE {
        return Err(CorpFinanceError::InvalidInput {
            field: "performance_fee_rate".into(),
            reason: "Performance fee rate must be >= 0 and < 1".into(),
        });
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Validation — ADGM
// ---------------------------------------------------------------------------

fn validate_adgm_input(input: &AdgmFundInput) -> CorpFinanceResult<()> {
    if input.fund_name.trim().is_empty() {
        return Err(CorpFinanceError::InvalidInput {
            field: "fund_name".into(),
            reason: "Fund name cannot be empty".into(),
        });
    }

    if input.fund_size <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "fund_size".into(),
            reason: "Fund size must be greater than zero".into(),
        });
    }

    if input.management_fee_rate < Decimal::ZERO || input.management_fee_rate >= Decimal::ONE {
        return Err(CorpFinanceError::InvalidInput {
            field: "management_fee_rate".into(),
            reason: "Management fee rate must be >= 0 and < 1".into(),
        });
    }

    if input.performance_fee_rate < Decimal::ZERO || input.performance_fee_rate >= Decimal::ONE {
        return Err(CorpFinanceError::InvalidInput {
            field: "performance_fee_rate".into(),
            reason: "Performance fee rate must be >= 0 and < 1".into(),
        });
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Validation — Sharia
// ---------------------------------------------------------------------------

fn validate_sharia_input(input: &ShariaComplianceInput) -> CorpFinanceResult<()> {
    let valid_methods = ["AAOIFI", "DJIM", "MSCI", "FTSE"];
    if !valid_methods.contains(&input.screening_methodology.as_str()) {
        return Err(CorpFinanceError::InvalidInput {
            field: "screening_methodology".into(),
            reason: format!(
                "Unknown methodology '{}'. Valid: {:?}",
                input.screening_methodology, valid_methods
            ),
        });
    }

    let valid_purification = ["Income", "Charity"];
    if !valid_purification.contains(&input.purification_method.as_str()) {
        return Err(CorpFinanceError::InvalidInput {
            field: "purification_method".into(),
            reason: format!(
                "Unknown purification method '{}'. Valid: {:?}",
                input.purification_method, valid_purification
            ),
        });
    }

    if input.companies.is_empty() {
        return Err(CorpFinanceError::InvalidInput {
            field: "companies".into(),
            reason: "At least one company required for Sharia screening".into(),
        });
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Validation — Comparison
// ---------------------------------------------------------------------------

fn validate_comparison_input(input: &DifcAdgmCompInput) -> CorpFinanceResult<()> {
    if input.fund_size <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "fund_size".into(),
            reason: "Fund size must be greater than zero".into(),
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
    use rust_decimal_macros::dec;

    // ======================================================================
    // Test helpers
    // ======================================================================

    fn difc_qif_input() -> DifcFundInput {
        DifcFundInput {
            fund_name: "Gulf Alpha QIF".to_string(),
            fund_type: "QIF".to_string(),
            fund_strategy: "Hedge".to_string(),
            fund_size: dec!(200_000_000),
            management_fee_rate: dec!(0.02),
            performance_fee_rate: dec!(0.20),
            minimum_subscription: dec!(500_000),
            investor_count: 50,
            sharia_compliant: false,
            sharia_board_members: 0,
            target_investors: vec!["GCC".to_string()],
        }
    }

    fn difc_exempt_input() -> DifcFundInput {
        DifcFundInput {
            fund_name: "MENA Exempt Fund".to_string(),
            fund_type: "ExemptFund".to_string(),
            fund_strategy: "PE".to_string(),
            fund_size: dec!(100_000_000),
            management_fee_rate: dec!(0.015),
            performance_fee_rate: dec!(0.20),
            minimum_subscription: dec!(100_000),
            investor_count: 80,
            sharia_compliant: false,
            sharia_board_members: 0,
            target_investors: vec!["GCC".to_string(), "EU".to_string()],
        }
    }

    fn difc_domestic_input() -> DifcFundInput {
        DifcFundInput {
            fund_name: "Dubai Retail Fund".to_string(),
            fund_type: "DomesticFund".to_string(),
            fund_strategy: "RealEstate".to_string(),
            fund_size: dec!(500_000_000),
            management_fee_rate: dec!(0.01),
            performance_fee_rate: dec!(0.15),
            minimum_subscription: dec!(10_000),
            investor_count: 500,
            sharia_compliant: true,
            sharia_board_members: 3,
            target_investors: vec!["GCC".to_string()],
        }
    }

    fn adgm_lp_input() -> AdgmFundInput {
        AdgmFundInput {
            fund_name: "Abu Dhabi PE Fund I".to_string(),
            fund_type: "Exempt".to_string(),
            structure_type: "LP".to_string(),
            fund_strategy: "PE".to_string(),
            fund_size: dec!(300_000_000),
            management_fee_rate: dec!(0.015),
            performance_fee_rate: dec!(0.20),
            minimum_subscription: dec!(1_000_000),
            investor_count: 30,
            sharia_compliant: false,
            target_investors: vec!["GCC".to_string()],
        }
    }

    fn sharia_input_pass() -> ShariaComplianceInput {
        ShariaComplianceInput {
            screening_methodology: "AAOIFI".to_string(),
            companies: vec![
                ShariaScreeningCompany {
                    name: "Halal Tech Corp".to_string(),
                    debt_to_market_cap: dec!(0.20),
                    interest_income_pct: dec!(0.02),
                    non_permissible_revenue_pct: dec!(0.01),
                    total_dividends: dec!(100_000),
                },
                ShariaScreeningCompany {
                    name: "Clean Energy Ltd".to_string(),
                    debt_to_market_cap: dec!(0.10),
                    interest_income_pct: dec!(0.01),
                    non_permissible_revenue_pct: dec!(0.005),
                    total_dividends: dec!(50_000),
                },
            ],
            ssb_member_count: 3,
            ssb_qualified_scholars: 1,
            purification_method: "Income".to_string(),
        }
    }

    fn comparison_input() -> DifcAdgmCompInput {
        DifcAdgmCompInput {
            fund_strategy: "PE".to_string(),
            fund_size: dec!(200_000_000),
            sharia_required: false,
            target_investors: vec!["GCC".to_string()],
            preferred_legal_system: "EnglishCommonLaw".to_string(),
        }
    }

    // ======================================================================
    // DIFC Fund Tests
    // ======================================================================

    #[test]
    fn test_difc_qif_basic() {
        let input = difc_qif_input();
        let result = analyze_difc_fund(&input).unwrap();

        assert_eq!(
            result.jurisdiction,
            "DIFC (Dubai International Financial Centre)"
        );
        assert_eq!(result.structure_analysis.fund_type, "QIF");
        assert_eq!(result.structure_analysis.tax_guarantee_years, 50);
        assert_eq!(result.structure_analysis.max_investors, 100);
    }

    #[test]
    fn test_difc_qif_min_subscription_500k() {
        let input = difc_qif_input();
        let result = analyze_difc_fund(&input).unwrap();
        assert!(result.structure_analysis.minimum_subscription >= dec!(500_000));
    }

    #[test]
    fn test_difc_qif_below_min_subscription() {
        let mut input = difc_qif_input();
        input.minimum_subscription = dec!(499_999);
        let result = analyze_difc_fund(&input);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("500,000"));
    }

    #[test]
    fn test_difc_exempt_fund_basic() {
        let input = difc_exempt_input();
        let result = analyze_difc_fund(&input).unwrap();

        assert_eq!(result.structure_analysis.fund_type, "ExemptFund");
        assert_eq!(result.structure_analysis.max_investors, 100);
    }

    #[test]
    fn test_difc_exempt_fund_min_50k() {
        let mut input = difc_exempt_input();
        input.minimum_subscription = dec!(49_999);
        let result = analyze_difc_fund(&input);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("50,000"));
    }

    #[test]
    fn test_difc_exempt_fund_101_investors_fails() {
        let mut input = difc_exempt_input();
        input.investor_count = 101;
        let result = analyze_difc_fund(&input);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("100"));
    }

    #[test]
    fn test_difc_exempt_fund_100_investors_ok() {
        let mut input = difc_exempt_input();
        input.investor_count = 100;
        let result = analyze_difc_fund(&input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_difc_domestic_fund_basic() {
        let input = difc_domestic_input();
        let result = analyze_difc_fund(&input).unwrap();

        assert_eq!(result.structure_analysis.fund_type, "DomesticFund");
        assert_eq!(result.structure_analysis.max_investors, u32::MAX);
    }

    #[test]
    fn test_difc_domestic_fund_sharia() {
        let input = difc_domestic_input();
        let result = analyze_difc_fund(&input).unwrap();

        assert!(result.sharia_analysis.sharia_compliant);
        assert!(result.sharia_analysis.ssb_required);
        assert!(result.sharia_analysis.ssb_adequate);
        assert_eq!(result.sharia_analysis.cost_premium_pct, dec!(0.20));
    }

    #[test]
    fn test_difc_zero_tax() {
        let input = difc_qif_input();
        let result = analyze_difc_fund(&input).unwrap();

        assert_eq!(result.tax_analysis.corporate_tax_rate, Decimal::ZERO);
        assert_eq!(result.tax_analysis.withholding_tax_rate, Decimal::ZERO);
        assert_eq!(result.tax_analysis.capital_gains_tax_rate, Decimal::ZERO);
        assert!(!result.tax_analysis.vat_applicable);
    }

    #[test]
    fn test_difc_regulatory_dfsa_category() {
        let input = difc_qif_input();
        let result = analyze_difc_fund(&input).unwrap();

        assert!(result.regulatory.dfsa_license_category.contains("3C"));
        assert!(result.regulatory.compliance_officer_required);
        assert!(result.regulatory.mlro_required);
    }

    #[test]
    fn test_difc_cost_analysis_ranges() {
        let input = difc_qif_input();
        let result = analyze_difc_fund(&input).unwrap();

        assert_eq!(result.cost_analysis.setup_cost_low, dec!(40_000));
        assert_eq!(result.cost_analysis.setup_cost_high, dec!(80_000));
        assert_eq!(result.cost_analysis.annual_cost_low, dec!(100_000));
        assert_eq!(result.cost_analysis.annual_cost_high, dec!(250_000));
    }

    #[test]
    fn test_difc_sharia_premium_non_sharia() {
        let input = difc_qif_input();
        let result = analyze_difc_fund(&input).unwrap();
        assert_eq!(result.cost_analysis.sharia_premium, Decimal::ZERO);
    }

    #[test]
    fn test_difc_sharia_premium_sharia() {
        let input = difc_domestic_input();
        let result = analyze_difc_fund(&input).unwrap();
        assert!(result.cost_analysis.sharia_premium > Decimal::ZERO);
    }

    #[test]
    fn test_difc_distribution_gcc() {
        let input = difc_qif_input();
        let result = analyze_difc_fund(&input).unwrap();
        assert!(result.distribution_analysis.gcc_access);
    }

    #[test]
    fn test_difc_distribution_eu_bilateral() {
        let input = difc_exempt_input();
        let result = analyze_difc_fund(&input).unwrap();
        assert!(result.distribution_analysis.bilateral_eu_treaties);
    }

    #[test]
    fn test_difc_invalid_fund_type() {
        let mut input = difc_qif_input();
        input.fund_type = "InvalidType".to_string();
        let result = analyze_difc_fund(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_difc_empty_fund_name() {
        let mut input = difc_qif_input();
        input.fund_name = "".to_string();
        let result = analyze_difc_fund(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_difc_zero_fund_size() {
        let mut input = difc_qif_input();
        input.fund_size = Decimal::ZERO;
        let result = analyze_difc_fund(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_difc_negative_fee_rate() {
        let mut input = difc_qif_input();
        input.management_fee_rate = dec!(-0.01);
        let result = analyze_difc_fund(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_difc_sharia_fund_insufficient_ssb() {
        let mut input = difc_domestic_input();
        input.sharia_board_members = 2;
        let result = analyze_difc_fund(&input).unwrap();
        assert!(!result.sharia_analysis.ssb_adequate);
        assert!(result.warnings.iter().any(|w| w.contains("3 SSB members")));
    }

    // ======================================================================
    // ADGM Fund Tests
    // ======================================================================

    #[test]
    fn test_adgm_lp_basic() {
        let input = adgm_lp_input();
        let result = analyze_adgm_fund(&input).unwrap();

        assert_eq!(result.jurisdiction, "ADGM (Abu Dhabi Global Market)");
        assert_eq!(result.structure_analysis.structure_type, "LP");
        assert_eq!(result.structure_analysis.tax_guarantee_years, 50);
    }

    #[test]
    fn test_adgm_english_common_law() {
        let input = adgm_lp_input();
        let result = analyze_adgm_fund(&input).unwrap();

        assert!(result
            .common_law_analysis
            .legal_system
            .contains("English common law"));
        assert!(result
            .common_law_analysis
            .court_system
            .contains("ADGM Courts"));
    }

    #[test]
    fn test_adgm_zero_tax() {
        let input = adgm_lp_input();
        let result = analyze_adgm_fund(&input).unwrap();

        assert_eq!(result.tax_analysis.corporate_tax_rate, Decimal::ZERO);
        assert_eq!(result.tax_analysis.withholding_tax_rate, Decimal::ZERO);
        assert_eq!(result.tax_analysis.capital_gains_tax_rate, Decimal::ZERO);
    }

    #[test]
    fn test_adgm_cost_analysis_lower_than_difc() {
        let input = adgm_lp_input();
        let result = analyze_adgm_fund(&input).unwrap();

        assert_eq!(result.cost_analysis.setup_cost_low, dec!(30_000));
        assert_eq!(result.cost_analysis.setup_cost_high, dec!(70_000));
        assert_eq!(result.cost_analysis.annual_cost_low, dec!(80_000));
        assert_eq!(result.cost_analysis.annual_cost_high, dec!(200_000));
    }

    #[test]
    fn test_adgm_regulatory_fsra() {
        let input = adgm_lp_input();
        let result = analyze_adgm_fund(&input).unwrap();

        assert!(result.regulatory.compliance_officer_required);
        assert!(result.regulatory.audit_required);
        assert!(result.regulatory.fsra_annual_fee > Decimal::ZERO);
    }

    #[test]
    fn test_adgm_comparison_to_difc_included() {
        let input = adgm_lp_input();
        let result = analyze_adgm_fund(&input).unwrap();

        assert!(!result.comparison_to_difc.adgm_advantage.is_empty());
        assert!(!result.comparison_to_difc.difc_advantage.is_empty());
        assert!(!result.comparison_to_difc.neutral.is_empty());
    }

    #[test]
    fn test_adgm_investment_company_structure() {
        let mut input = adgm_lp_input();
        input.structure_type = "InvestmentCompany".to_string();
        let result = analyze_adgm_fund(&input).unwrap();

        assert!(result
            .structure_analysis
            .legal_framework
            .contains("Companies Regulations"));
    }

    #[test]
    fn test_adgm_investment_trust_structure() {
        let mut input = adgm_lp_input();
        input.structure_type = "InvestmentTrust".to_string();
        let result = analyze_adgm_fund(&input).unwrap();

        assert!(result
            .structure_analysis
            .legal_framework
            .contains("Trust Regulations"));
    }

    #[test]
    fn test_adgm_qif_fund_type() {
        let mut input = adgm_lp_input();
        input.fund_type = "QIF".to_string();
        let result = analyze_adgm_fund(&input).unwrap();

        assert_eq!(result.structure_analysis.fund_type, "QIF");
        assert_eq!(result.structure_analysis.max_investors, Some(100));
    }

    #[test]
    fn test_adgm_public_fund_type() {
        let mut input = adgm_lp_input();
        input.fund_type = "Public".to_string();
        let result = analyze_adgm_fund(&input).unwrap();

        assert_eq!(result.structure_analysis.fund_type, "Public");
        assert!(result.structure_analysis.max_investors.is_none());
    }

    #[test]
    fn test_adgm_invalid_fund_type() {
        let mut input = adgm_lp_input();
        input.fund_type = "Invalid".to_string();
        let result = analyze_adgm_fund(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_adgm_invalid_structure_type() {
        let mut input = adgm_lp_input();
        input.structure_type = "SICAV".to_string();
        let result = analyze_adgm_fund(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_adgm_empty_name() {
        let mut input = adgm_lp_input();
        input.fund_name = " ".to_string();
        let result = analyze_adgm_fund(&input);
        assert!(result.is_err());
    }

    // ======================================================================
    // Sharia Compliance Tests
    // ======================================================================

    #[test]
    fn test_sharia_all_pass() {
        let input = sharia_input_pass();
        let result = sharia_compliance_check(&input).unwrap();

        assert_eq!(result.screening_results.len(), 2);
        assert!(result.screening_results[0].overall_pass);
        assert!(result.screening_results[1].overall_pass);
    }

    #[test]
    fn test_sharia_debt_ratio_fail() {
        let mut input = sharia_input_pass();
        input.companies[0].debt_to_market_cap = dec!(0.40);
        let result = sharia_compliance_check(&input).unwrap();

        assert!(!result.screening_results[0].debt_ratio_pass);
        assert!(!result.screening_results[0].overall_pass);
        assert!(result.screening_results[1].overall_pass);
    }

    #[test]
    fn test_sharia_interest_income_fail() {
        let mut input = sharia_input_pass();
        input.companies[0].interest_income_pct = dec!(0.06);
        let result = sharia_compliance_check(&input).unwrap();

        assert!(!result.screening_results[0].interest_income_pass);
        assert!(!result.screening_results[0].overall_pass);
    }

    #[test]
    fn test_sharia_non_permissible_fail() {
        let mut input = sharia_input_pass();
        input.companies[0].non_permissible_revenue_pct = dec!(0.08);
        let result = sharia_compliance_check(&input).unwrap();

        assert!(!result.screening_results[0].non_permissible_pass);
        assert!(!result.screening_results[0].overall_pass);
    }

    #[test]
    fn test_sharia_purification_calculation() {
        let input = sharia_input_pass();
        let result = sharia_compliance_check(&input).unwrap();

        // Company 1: 100_000 * 0.01 = 1_000
        assert_eq!(result.screening_results[0].purification_amount, dec!(1000));
        // Company 2: 50_000 * 0.005 = 250
        assert_eq!(result.screening_results[1].purification_amount, dec!(250));
        // Total: 1_250
        assert_eq!(result.total_purification_amount, dec!(1250));
    }

    #[test]
    fn test_sharia_ssb_adequate() {
        let input = sharia_input_pass();
        let result = sharia_compliance_check(&input).unwrap();

        assert!(result.ssb_governance.meets_minimum);
        assert!(result.ssb_governance.has_qualified_scholar);
    }

    #[test]
    fn test_sharia_ssb_only_2_members_fails() {
        let mut input = sharia_input_pass();
        input.ssb_member_count = 2;
        let result = sharia_compliance_check(&input).unwrap();

        assert!(!result.ssb_governance.meets_minimum);
        assert!(result.warnings.iter().any(|w| w.contains("3-member")));
    }

    #[test]
    fn test_sharia_ssb_no_qualified_scholar() {
        let mut input = sharia_input_pass();
        input.ssb_qualified_scholars = 0;
        let result = sharia_compliance_check(&input).unwrap();

        assert!(!result.ssb_governance.has_qualified_scholar);
        assert!(result
            .warnings
            .iter()
            .any(|w| w.contains("qualified Sharia scholar")));
    }

    #[test]
    fn test_sharia_investment_restrictions() {
        let input = sharia_input_pass();
        let result = sharia_compliance_check(&input).unwrap();

        assert!(result
            .investment_restrictions
            .iter()
            .any(|r| r.contains("alcohol")));
        assert!(result
            .investment_restrictions
            .iter()
            .any(|r| r.contains("gambling")));
        assert!(result
            .investment_restrictions
            .iter()
            .any(|r| r.contains("pork")));
        assert!(result
            .investment_restrictions
            .iter()
            .any(|r| r.contains("tobacco")));
        assert!(result
            .investment_restrictions
            .iter()
            .any(|r| r.contains("weapons")));
    }

    #[test]
    fn test_sharia_compliance_score_all_pass() {
        let input = sharia_input_pass();
        let result = sharia_compliance_check(&input).unwrap();

        // screening: 40/40 + ssb: 20+20 + methodology: 20 = 100
        assert_eq!(result.compliance_score, 100);
    }

    #[test]
    fn test_sharia_compliance_score_half_fail() {
        let mut input = sharia_input_pass();
        input.companies[0].debt_to_market_cap = dec!(0.50); // fail this one
        let result = sharia_compliance_check(&input).unwrap();

        // screening: 1/2 pass = 20/40 + ssb: 40 + methodology: 20 = 80
        assert_eq!(result.compliance_score, 80);
    }

    #[test]
    fn test_sharia_cost_premium() {
        let input = sharia_input_pass();
        let result = sharia_compliance_check(&input).unwrap();
        assert_eq!(result.cost_premium_vs_conventional, dec!(0.20));
    }

    #[test]
    fn test_sharia_invalid_methodology() {
        let mut input = sharia_input_pass();
        input.screening_methodology = "Unknown".to_string();
        let result = sharia_compliance_check(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_sharia_invalid_purification_method() {
        let mut input = sharia_input_pass();
        input.purification_method = "Magic".to_string();
        let result = sharia_compliance_check(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_sharia_empty_companies() {
        let mut input = sharia_input_pass();
        input.companies.clear();
        let result = sharia_compliance_check(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_sharia_boundary_debt_ratio_just_below() {
        let mut input = sharia_input_pass();
        input.companies[0].debt_to_market_cap = dec!(0.329);
        let result = sharia_compliance_check(&input).unwrap();
        assert!(result.screening_results[0].debt_ratio_pass);
    }

    #[test]
    fn test_sharia_boundary_debt_ratio_exactly_33() {
        let mut input = sharia_input_pass();
        input.companies[0].debt_to_market_cap = dec!(0.33);
        let result = sharia_compliance_check(&input).unwrap();
        // 0.33 is not < 0.33, so should fail
        assert!(!result.screening_results[0].debt_ratio_pass);
    }

    // ======================================================================
    // Comparison Tests
    // ======================================================================

    #[test]
    fn test_comparison_basic() {
        let input = comparison_input();
        let result = difc_adgm_comparison(&input).unwrap();

        assert!(!result.dimensions.is_empty());
        assert!(!result.recommended_jurisdiction.is_empty());
        assert!(!result.rationale.is_empty());
    }

    #[test]
    fn test_comparison_dimensions_count() {
        let input = comparison_input();
        let result = difc_adgm_comparison(&input).unwrap();
        assert_eq!(result.dimensions.len(), 9);
    }

    #[test]
    fn test_comparison_english_common_law_preference() {
        let input = comparison_input();
        let result = difc_adgm_comparison(&input).unwrap();

        let legal = result
            .dimensions
            .iter()
            .find(|d| d.dimension == "Legal System")
            .unwrap();
        assert_eq!(legal.advantage, "ADGM");
    }

    #[test]
    fn test_comparison_sharia_prefers_difc() {
        let mut input = comparison_input();
        input.sharia_required = true;
        input.preferred_legal_system = "Any".to_string();
        let result = difc_adgm_comparison(&input).unwrap();

        let sharia = result
            .dimensions
            .iter()
            .find(|d| d.dimension == "Sharia Framework")
            .unwrap();
        assert_eq!(sharia.advantage, "DIFC");
    }

    #[test]
    fn test_comparison_small_fund_warning() {
        let mut input = comparison_input();
        input.fund_size = dec!(40_000_000);
        let result = difc_adgm_comparison(&input).unwrap();

        assert!(result.warnings.iter().any(|w| w.contains("50M")));
    }

    #[test]
    fn test_comparison_zero_fund_size_fails() {
        let mut input = comparison_input();
        input.fund_size = Decimal::ZERO;
        let result = difc_adgm_comparison(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_comparison_scores_positive() {
        let input = comparison_input();
        let result = difc_adgm_comparison(&input).unwrap();

        assert!(result.difc_score > 0);
        assert!(result.adgm_score > 0);
    }

    #[test]
    fn test_comparison_setup_cost_adgm_cheaper() {
        let input = comparison_input();
        let result = difc_adgm_comparison(&input).unwrap();

        let setup = result
            .dimensions
            .iter()
            .find(|d| d.dimension == "Setup Cost")
            .unwrap();
        assert_eq!(setup.advantage, "ADGM");
    }

    #[test]
    fn test_comparison_annual_cost_adgm_cheaper() {
        let input = comparison_input();
        let result = difc_adgm_comparison(&input).unwrap();

        let annual = result
            .dimensions
            .iter()
            .find(|d| d.dimension == "Annual Cost")
            .unwrap();
        assert_eq!(annual.advantage, "ADGM");
    }
}
