use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

use crate::error::CorpFinanceError;
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Types — VCC Structure
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubFundInfo {
    pub name: String,
    pub strategy: String,
    pub target_aum: Decimal,
    pub currency: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VccInput {
    pub fund_name: String,
    /// "Standalone", "Umbrella"
    pub vcc_type: String,
    pub sub_funds: Vec<SubFundInfo>,
    /// "RFMC", "LRFMC", "A_LFMC"
    pub manager_license: String,
    pub fund_size: Decimal,
    pub management_fee_rate: Decimal,
    pub performance_fee_rate: Decimal,
    /// "S13O", "S13U", "S13D"
    pub tax_incentive_scheme: Option<String>,
    pub investment_professionals_sg: u32,
    pub local_business_spending: Decimal,
    pub target_investors: Vec<String>,
}

// ---------------------------------------------------------------------------
// Types — Sub-Fund Allocation
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubFundAllocationInput {
    pub fund_name: String,
    pub sub_funds: Vec<SubFundInfo>,
    pub total_fund_size: Decimal,
    /// "AumWeighted", "EqualWeighted", "Hybrid"
    pub allocation_method: String,
    pub shared_costs: SharedCosts,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharedCosts {
    pub board_fees: Decimal,
    pub company_secretary: Decimal,
    pub registered_office: Decimal,
    pub compliance: Decimal,
    pub audit_umbrella: Decimal,
}

// ---------------------------------------------------------------------------
// Types — Tax Incentive
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaxIncentiveInput {
    pub fund_name: String,
    pub fund_size: Decimal,
    pub qualifying_income: Decimal,
    pub non_qualifying_income: Decimal,
    pub investment_professionals_sg: u32,
    pub local_business_spending: Decimal,
    /// "S13O", "S13U", "S13D"
    pub scheme: String,
    pub is_resident: bool,
}

// ---------------------------------------------------------------------------
// Types — VCC vs Cayman SPC Comparison
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VccCaymanCompInput {
    pub fund_name: String,
    pub fund_size: Decimal,
    pub num_sub_funds: u32,
    pub management_fee_rate: Decimal,
    pub target_investors: Vec<String>,
    pub investment_professionals_sg: u32,
    pub local_business_spending: Decimal,
}

// ---------------------------------------------------------------------------
// Output types — VCC Structure
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VccStructureAnalysis {
    pub vcc_type: String,
    pub description: String,
    pub segregated_assets: bool,
    pub suitable_strategies: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicensingAnalysis {
    pub license_type: String,
    pub aum_limit: Option<Decimal>,
    pub investor_limit: Option<u32>,
    pub requirements: Vec<String>,
    pub compliant: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubFundAnalysis {
    pub name: String,
    pub strategy: String,
    pub target_aum: Decimal,
    pub currency: String,
    pub pct_of_total: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VccRegulatoryAnalysis {
    pub sg_resident_director_required: bool,
    pub registered_office_sg: bool,
    pub company_secretary_required: bool,
    pub vcc_act_2018: bool,
    pub mas_regulated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VccSubstanceAnalysis {
    pub substance_score: u32,
    pub investment_professionals: u32,
    pub local_spending: Decimal,
    pub sg_resident_director: bool,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VccCostAnalysis {
    pub setup_cost_low: Decimal,
    pub setup_cost_high: Decimal,
    pub annual_cost_low: Decimal,
    pub annual_cost_high: Decimal,
    pub cost_pct_of_aum: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VccOutput {
    pub structure_analysis: VccStructureAnalysis,
    pub licensing_analysis: LicensingAnalysis,
    pub sub_fund_analysis: Vec<SubFundAnalysis>,
    pub regulatory: VccRegulatoryAnalysis,
    pub substance_analysis: VccSubstanceAnalysis,
    pub cost_analysis: VccCostAnalysis,
    pub recommendations: Vec<String>,
    pub warnings: Vec<String>,
}

// ---------------------------------------------------------------------------
// Output types — Sub-Fund Allocation
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubFundEconomics {
    pub name: String,
    pub aum: Decimal,
    pub aum_weight: Decimal,
    pub allocated_shared_cost: Decimal,
    pub direct_cost: Decimal,
    pub total_cost: Decimal,
    pub ter: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregateEconomics {
    pub total_shared_costs: Decimal,
    pub total_direct_costs: Decimal,
    pub total_costs: Decimal,
    pub aggregate_ter: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubFundAllocationOutput {
    pub per_sub_fund_economics: Vec<SubFundEconomics>,
    pub aggregate_economics: AggregateEconomics,
    pub marginal_cost_per_sub_fund: Decimal,
}

// ---------------------------------------------------------------------------
// Output types — Tax Incentive
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConditionResult {
    pub condition: String,
    pub required: String,
    pub actual: String,
    pub passed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemeEligibility {
    pub scheme: String,
    pub eligible: bool,
    pub conditions_met: u32,
    pub conditions_total: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemeComparison {
    pub scheme: String,
    pub eligible: bool,
    pub tax_savings: Decimal,
    pub key_requirement: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaxIncentiveOutput {
    pub scheme_eligibility: SchemeEligibility,
    pub conditions: Vec<ConditionResult>,
    pub tax_savings: Decimal,
    pub non_qualifying_income_tax: Decimal,
    pub recommendation: String,
    pub comparison_all_schemes: Vec<SchemeComparison>,
}

// ---------------------------------------------------------------------------
// Output types — VCC vs Cayman SPC Comparison
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JurisdictionMetrics {
    pub jurisdiction: String,
    pub setup_cost: Decimal,
    pub annual_cost: Decimal,
    pub corporate_tax_rate: Decimal,
    pub substance_score: u32,
    pub fatca_crs_compliant: bool,
    pub distribution_reach: String,
    pub redomiciliation_ease: String,
    pub regulatory_timeline_weeks: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VccCaymanCompOutput {
    pub vcc_metrics: JurisdictionMetrics,
    pub cayman_spc_metrics: JurisdictionMetrics,
    pub cost_differential_setup: Decimal,
    pub cost_differential_annual: Decimal,
    pub recommendation: String,
    pub comparison_notes: Vec<String>,
}

// ---------------------------------------------------------------------------
// Public API — 1. Analyze VCC Structure
// ---------------------------------------------------------------------------

pub fn analyze_vcc_structure(input: &VccInput) -> CorpFinanceResult<VccOutput> {
    validate_vcc_input(input)?;

    let mut warnings: Vec<String> = Vec::new();
    let mut recommendations: Vec<String> = Vec::new();

    // ------------------------------------------------------------------
    // 1. Structure Analysis
    // ------------------------------------------------------------------
    let structure_analysis =
        build_vcc_structure_analysis(&input.vcc_type, &mut recommendations, &mut warnings);

    // ------------------------------------------------------------------
    // 2. Licensing Analysis
    // ------------------------------------------------------------------
    let licensing_analysis = build_licensing_analysis(input, &mut recommendations, &mut warnings);

    // ------------------------------------------------------------------
    // 3. Sub-Fund Analysis (if Umbrella)
    // ------------------------------------------------------------------
    let sub_fund_analysis = if input.vcc_type == "Umbrella" && !input.sub_funds.is_empty() {
        build_sub_fund_analysis(input, &mut warnings)
    } else {
        Vec::new()
    };

    // ------------------------------------------------------------------
    // 4. Regulatory Analysis
    // ------------------------------------------------------------------
    let regulatory = VccRegulatoryAnalysis {
        sg_resident_director_required: true,
        registered_office_sg: true,
        company_secretary_required: true,
        vcc_act_2018: true,
        mas_regulated: true,
    };

    recommendations
        .push("VCC Act 2018 requires at least one Singapore-resident director".to_string());
    recommendations.push("Registered office must be in Singapore".to_string());

    // ------------------------------------------------------------------
    // 5. Substance Analysis
    // ------------------------------------------------------------------
    let substance_analysis = build_substance_analysis(input, &mut recommendations);

    // ------------------------------------------------------------------
    // 6. Cost Analysis
    // ------------------------------------------------------------------
    let (setup_low, setup_high, annual_low, annual_high) =
        estimate_vcc_costs(&input.vcc_type, input.sub_funds.len() as u32);

    let mid_annual = (annual_low + annual_high) / dec!(2);
    let cost_pct_of_aum = if input.fund_size > Decimal::ZERO {
        mid_annual / input.fund_size
    } else {
        Decimal::ZERO
    };

    let cost_analysis = VccCostAnalysis {
        setup_cost_low: setup_low,
        setup_cost_high: setup_high,
        annual_cost_low: annual_low,
        annual_cost_high: annual_high,
        cost_pct_of_aum,
    };

    // ------------------------------------------------------------------
    // 7. Tax incentive recommendation
    // ------------------------------------------------------------------
    if let Some(ref scheme) = input.tax_incentive_scheme {
        match scheme.as_str() {
            "S13O" => {
                if input.fund_size < dec!(10_000_000) {
                    warnings.push("Fund size below SGD 10M minimum for S13O incentive".to_string());
                }
            }
            "S13U" => {
                if input.fund_size < dec!(50_000_000) {
                    warnings.push("Fund size below SGD 50M minimum for S13U incentive".to_string());
                }
            }
            "S13D" => {
                if input
                    .target_investors
                    .iter()
                    .any(|i| i == "SingaporeResident")
                {
                    warnings.push(
                        "S13D is for offshore funds with non-resident investors; \
                         Singapore-resident investors may disqualify"
                            .to_string(),
                    );
                }
            }
            _ => {}
        }
    } else {
        recommendations.push(
            "Consider applying for S13O or S13U tax incentive to exempt \
             qualifying income from 17% corporate tax"
                .to_string(),
        );
    }

    // ------------------------------------------------------------------
    // 8. General warnings
    // ------------------------------------------------------------------
    if cost_pct_of_aum > dec!(0.005) {
        warnings.push(format!(
            "Estimated mid-range annual cost is {:.2}% of AUM, above the \
             typical 0.50% threshold",
            cost_pct_of_aum * dec!(100)
        ));
    }

    if input.vcc_type == "Umbrella" && input.sub_funds.len() < 2 {
        warnings.push(
            "Umbrella VCC with fewer than 2 sub-funds offers no cost benefit \
             over Standalone VCC"
                .to_string(),
        );
    }

    Ok(VccOutput {
        structure_analysis,
        licensing_analysis,
        sub_fund_analysis,
        regulatory,
        substance_analysis,
        cost_analysis,
        recommendations,
        warnings,
    })
}

// ---------------------------------------------------------------------------
// Public API — 2. VCC Sub-Fund Allocation
// ---------------------------------------------------------------------------

pub fn vcc_sub_fund_allocation(
    input: &SubFundAllocationInput,
) -> CorpFinanceResult<SubFundAllocationOutput> {
    validate_sub_fund_allocation_input(input)?;

    let total_shared = input.shared_costs.board_fees
        + input.shared_costs.company_secretary
        + input.shared_costs.registered_office
        + input.shared_costs.compliance
        + input.shared_costs.audit_umbrella;

    let n = input.sub_funds.len() as u32;
    let total_aum: Decimal = input.sub_funds.iter().map(|sf| sf.target_aum).sum();

    let mut per_sub_fund_economics = Vec::new();
    let mut total_direct_costs = Decimal::ZERO;

    for sf in &input.sub_funds {
        let aum_weight = if total_aum > Decimal::ZERO {
            sf.target_aum / total_aum
        } else {
            Decimal::ZERO
        };

        let allocated_shared = match input.allocation_method.as_str() {
            "AumWeighted" => total_shared * aum_weight,
            "EqualWeighted" => {
                if n > 0 {
                    total_shared / Decimal::from(n)
                } else {
                    Decimal::ZERO
                }
            }
            _ => {
                // 50% AUM-weighted + 50% equal-weighted
                let aum_portion = total_shared * aum_weight * dec!(0.5);
                let equal_portion = if n > 0 {
                    total_shared * dec!(0.5) / Decimal::from(n)
                } else {
                    Decimal::ZERO
                };
                aum_portion + equal_portion
            }
        };

        // Direct sub-fund costs (admin, audit specific to sub-fund)
        let direct_cost = estimate_sub_fund_direct_cost(sf.target_aum);
        total_direct_costs += direct_cost;

        let total_cost = allocated_shared + direct_cost;
        let ter = if sf.target_aum > Decimal::ZERO {
            total_cost / sf.target_aum
        } else {
            Decimal::ZERO
        };

        per_sub_fund_economics.push(SubFundEconomics {
            name: sf.name.clone(),
            aum: sf.target_aum,
            aum_weight,
            allocated_shared_cost: allocated_shared,
            direct_cost,
            total_cost,
            ter,
        });
    }

    let total_costs = total_shared + total_direct_costs;
    let aggregate_ter = if total_aum > Decimal::ZERO {
        total_costs / total_aum
    } else {
        Decimal::ZERO
    };

    let aggregate_economics = AggregateEconomics {
        total_shared_costs: total_shared,
        total_direct_costs,
        total_costs,
        aggregate_ter,
    };

    // Marginal cost of adding one more sub-fund (incremental direct cost
    // at median AUM level, assuming shared costs are spread thinner)
    let median_aum = if !input.sub_funds.is_empty() {
        total_aum / Decimal::from(n)
    } else {
        Decimal::ZERO
    };
    let marginal_cost = estimate_sub_fund_direct_cost(median_aum);

    Ok(SubFundAllocationOutput {
        per_sub_fund_economics,
        aggregate_economics,
        marginal_cost_per_sub_fund: marginal_cost,
    })
}

// ---------------------------------------------------------------------------
// Public API — 3. Tax Incentive Analysis
// ---------------------------------------------------------------------------

pub fn tax_incentive_analysis(input: &TaxIncentiveInput) -> CorpFinanceResult<TaxIncentiveOutput> {
    validate_tax_incentive_input(input)?;

    let sg_corporate_tax_rate = dec!(0.17);

    let (conditions, scheme_eligible) = evaluate_scheme_conditions(input);

    let conditions_met = conditions.iter().filter(|c| c.passed).count() as u32;
    let conditions_total = conditions.len() as u32;

    let scheme_eligibility = SchemeEligibility {
        scheme: input.scheme.clone(),
        eligible: scheme_eligible,
        conditions_met,
        conditions_total,
    };

    // Tax savings: qualifying income exempt from 17% corporate tax
    let tax_savings = if scheme_eligible {
        input.qualifying_income * sg_corporate_tax_rate
    } else {
        Decimal::ZERO
    };

    let non_qualifying_income_tax = input.non_qualifying_income * sg_corporate_tax_rate;

    let recommendation = if scheme_eligible {
        format!(
            "{} incentive eligible: SGD {} in annual tax savings on qualifying income",
            input.scheme, tax_savings
        )
    } else {
        let failed: Vec<String> = conditions
            .iter()
            .filter(|c| !c.passed)
            .map(|c| c.condition.clone())
            .collect();
        format!(
            "{} not eligible. Failed conditions: {}. Consider alternative schemes.",
            input.scheme,
            failed.join(", ")
        )
    };

    // Compare all three schemes
    let comparison_all_schemes = build_scheme_comparison(input, sg_corporate_tax_rate);

    Ok(TaxIncentiveOutput {
        scheme_eligibility,
        conditions,
        tax_savings,
        non_qualifying_income_tax,
        recommendation,
        comparison_all_schemes,
    })
}

// ---------------------------------------------------------------------------
// Public API — 4. VCC vs Cayman SPC Comparison
// ---------------------------------------------------------------------------

pub fn vcc_vs_cayman_spc(input: &VccCaymanCompInput) -> CorpFinanceResult<VccCaymanCompOutput> {
    validate_comparison_input(input)?;

    let num_subs = input.num_sub_funds.max(1);

    // VCC costs (SGD)
    let vcc_setup = dec!(50_000) + Decimal::from(num_subs) * dec!(10_000);
    let vcc_annual = dec!(100_000) + Decimal::from(num_subs) * dec!(25_000);

    // VCC substance
    let mut vcc_substance: u32 = 3; // base: SG director, office, secretary
    if input.investment_professionals_sg >= 1 {
        vcc_substance += 2;
    }
    if input.investment_professionals_sg >= 3 {
        vcc_substance += 1;
    }
    if input.local_business_spending >= dec!(200_000) {
        vcc_substance += 2;
    }
    vcc_substance = vcc_substance.min(10);

    let vcc_metrics = JurisdictionMetrics {
        jurisdiction: "Singapore".to_string(),
        setup_cost: vcc_setup,
        annual_cost: vcc_annual,
        corporate_tax_rate: dec!(0.17),
        substance_score: vcc_substance,
        fatca_crs_compliant: true,
        distribution_reach: "Asia-Pacific primary; global with tax treaty network".to_string(),
        redomiciliation_ease: "Inward redomiciliation supported under VCC Act".to_string(),
        regulatory_timeline_weeks: 8,
    };

    // Cayman SPC costs (USD ~ SGD for comparison)
    let spc_setup = dec!(40_000) + Decimal::from(num_subs) * dec!(8_000);
    let spc_annual = dec!(80_000) + Decimal::from(num_subs) * dec!(20_000);

    let cayman_spc_metrics = JurisdictionMetrics {
        jurisdiction: "Cayman Islands".to_string(),
        setup_cost: spc_setup,
        annual_cost: spc_annual,
        corporate_tax_rate: Decimal::ZERO,
        substance_score: 4,
        fatca_crs_compliant: true,
        distribution_reach: "Global; established for US/EU institutional investors".to_string(),
        redomiciliation_ease: "Outward migration possible; inward less common".to_string(),
        regulatory_timeline_weeks: 4,
    };

    let cost_differential_setup = vcc_setup - spc_setup;
    let cost_differential_annual = vcc_annual - spc_annual;

    let mut comparison_notes = vec![
        "Singapore VCC offers tax incentive schemes (S13O/S13U/S13D) that \
         can offset higher base costs"
            .to_string(),
        "Cayman SPC has zero corporate tax but no treaty network benefits".to_string(),
        "VCC benefits from Singapore's 80+ double tax treaty network".to_string(),
        "Cayman SPC has longer track record for institutional LP acceptance".to_string(),
        "VCC statutory asset segregation equivalent to Cayman SPC ring-fencing".to_string(),
    ];

    if input.target_investors.iter().any(|i| i.contains("Asia")) {
        comparison_notes.push(
            "Asia-focused investor base favors Singapore VCC for proximity \
             and regulatory familiarity"
                .to_string(),
        );
    }

    if input.target_investors.iter().any(|i| i.contains("US")) {
        comparison_notes.push(
            "US institutional investors may prefer Cayman SPC due to \
             established legal framework and precedent"
                .to_string(),
        );
    }

    let recommendation = if input.target_investors.iter().any(|i| i.contains("Asia"))
        && input.investment_professionals_sg >= 1
    {
        "Singapore VCC recommended for Asia-focused fund with local substance; \
         tax incentives can offset higher costs"
            .to_string()
    } else if input.fund_size >= dec!(500_000_000)
        && input.target_investors.iter().any(|i| i.contains("US"))
    {
        "Cayman SPC recommended for large fund targeting US institutional investors; \
         established legal framework and lower base costs"
            .to_string()
    } else {
        "Consider dual structure: Cayman SPC master with Singapore VCC feeder \
         for Asia-Pacific distribution"
            .to_string()
    };

    Ok(VccCaymanCompOutput {
        vcc_metrics,
        cayman_spc_metrics,
        cost_differential_setup,
        cost_differential_annual,
        recommendation,
        comparison_notes,
    })
}

// ---------------------------------------------------------------------------
// Helpers — VCC Structure
// ---------------------------------------------------------------------------

fn build_vcc_structure_analysis(
    vcc_type: &str,
    recommendations: &mut Vec<String>,
    _warnings: &mut [String],
) -> VccStructureAnalysis {
    match vcc_type {
        "Standalone" => {
            recommendations.push(
                "Standalone VCC is suitable for single-strategy funds with \
                 a focused investment mandate"
                    .to_string(),
            );
            VccStructureAnalysis {
                vcc_type: "Standalone".to_string(),
                description: "Single fund entity incorporated as a VCC under \
                              the VCC Act 2018"
                    .to_string(),
                segregated_assets: false,
                suitable_strategies: vec![
                    "Hedge".to_string(),
                    "PE".to_string(),
                    "VC".to_string(),
                    "RealEstate".to_string(),
                    "Credit".to_string(),
                ],
            }
        }
        _ => {
            // "Umbrella" (default for non-Standalone)
            recommendations.push(
                "Umbrella VCC allows multiple sub-funds with segregated \
                 assets under one corporate entity"
                    .to_string(),
            );
            VccStructureAnalysis {
                vcc_type: "Umbrella".to_string(),
                description: "Umbrella VCC with multiple sub-funds; each sub-fund \
                              has segregated assets and liabilities under VCC Act 2018"
                    .to_string(),
                segregated_assets: true,
                suitable_strategies: vec![
                    "Hedge".to_string(),
                    "PE".to_string(),
                    "VC".to_string(),
                    "RealEstate".to_string(),
                    "Credit".to_string(),
                    "MultiStrategy".to_string(),
                ],
            }
        }
    }
}

fn build_licensing_analysis(
    input: &VccInput,
    recommendations: &mut Vec<String>,
    warnings: &mut Vec<String>,
) -> LicensingAnalysis {
    match input.manager_license.as_str() {
        "RFMC" => {
            let aum_limit = dec!(250_000_000);
            let investor_limit = 30u32;
            let investor_count = input.target_investors.len() as u32;

            let mut compliant = true;
            let mut requirements = vec![
                "AUM <= SGD 250M".to_string(),
                "Maximum 30 qualified investors".to_string(),
                "Base capital SGD 250,000".to_string(),
                "Annual audit of fund manager".to_string(),
            ];

            if input.fund_size > aum_limit {
                compliant = false;
                warnings.push(format!(
                    "RFMC AUM limit is SGD 250M; fund size SGD {} exceeds limit",
                    input.fund_size
                ));
            }

            if investor_count > investor_limit {
                compliant = false;
                warnings.push(format!(
                    "RFMC investor limit is 30; {} target investors exceeds limit",
                    investor_count
                ));
            }

            if !compliant {
                recommendations.push(
                    "Consider upgrading to LRFMC or A-LFMC license for larger \
                     fund or more investors"
                        .to_string(),
                );
            }

            requirements.push(format!(
                "Current: {} investors, SGD {} AUM",
                investor_count, input.fund_size
            ));

            LicensingAnalysis {
                license_type: "RFMC (Registered Fund Management Company)".to_string(),
                aum_limit: Some(aum_limit),
                investor_limit: Some(investor_limit),
                requirements,
                compliant,
            }
        }
        "LRFMC" => {
            recommendations.push(
                "LRFMC has lighter regulatory burden than A-LFMC but limited \
                 to accredited/institutional investors"
                    .to_string(),
            );
            LicensingAnalysis {
                license_type: "LRFMC (Licensed Fund Management Company — Retail Exempt)"
                    .to_string(),
                aum_limit: None,
                investor_limit: None,
                requirements: vec![
                    "Base capital SGD 250,000".to_string(),
                    "MAS CMS license required".to_string(),
                    "Accredited/institutional investors only".to_string(),
                    "Annual audit of fund manager".to_string(),
                    "Compliance officer appointment".to_string(),
                ],
                compliant: true,
            }
        }
        _ => {
            // "A_LFMC"
            recommendations.push(
                "A-LFMC provides broadest distribution capability including \
                 retail investors"
                    .to_string(),
            );
            LicensingAnalysis {
                license_type: "A-LFMC (Licensed Fund Management Company — Full)".to_string(),
                aum_limit: None,
                investor_limit: None,
                requirements: vec![
                    "Base capital SGD 1,000,000".to_string(),
                    "MAS CMS license required".to_string(),
                    "No investor restriction".to_string(),
                    "Annual audit of fund manager".to_string(),
                    "Compliance officer and risk management function".to_string(),
                    "Board of directors with independent members".to_string(),
                ],
                compliant: true,
            }
        }
    }
}

fn build_sub_fund_analysis(input: &VccInput, warnings: &mut Vec<String>) -> Vec<SubFundAnalysis> {
    let total_sub_aum: Decimal = input.sub_funds.iter().map(|sf| sf.target_aum).sum();

    // Check 1% tolerance
    if input.fund_size > Decimal::ZERO {
        let diff = (total_sub_aum - input.fund_size).abs();
        let tolerance = input.fund_size * dec!(0.01);
        if diff > tolerance {
            warnings.push(format!(
                "Sub-fund AUM total (SGD {}) differs from fund size (SGD {}) \
                 by more than 1%",
                total_sub_aum, input.fund_size
            ));
        }
    }

    input
        .sub_funds
        .iter()
        .map(|sf| {
            let pct = if input.fund_size > Decimal::ZERO {
                sf.target_aum / input.fund_size
            } else {
                Decimal::ZERO
            };
            SubFundAnalysis {
                name: sf.name.clone(),
                strategy: sf.strategy.clone(),
                target_aum: sf.target_aum,
                currency: sf.currency.clone(),
                pct_of_total: pct,
            }
        })
        .collect()
}

fn build_substance_analysis(
    input: &VccInput,
    recommendations: &mut Vec<String>,
) -> VccSubstanceAnalysis {
    let mut score: u32 = 0;
    let mut recs: Vec<String> = Vec::new();

    // Singapore-resident director (mandatory under VCC Act)
    score += 2;

    // Investment professionals in Singapore
    if input.investment_professionals_sg >= 3 {
        score += 3;
    } else if input.investment_professionals_sg >= 1 {
        score += 2;
    } else {
        recs.push(
            "No investment professionals in Singapore; hire at least 1 for \
             S13O eligibility"
                .to_string(),
        );
    }

    // Local business spending
    if input.local_business_spending >= dec!(200_000) {
        score += 2;
    } else if input.local_business_spending > Decimal::ZERO {
        score += 1;
        recs.push(format!(
            "Local spending SGD {} below SGD 200k minimum for tax incentive eligibility",
            input.local_business_spending
        ));
    } else {
        recs.push(
            "No local business spending in Singapore; minimum SGD 200k \
             required for S13O/S13U"
                .to_string(),
        );
    }

    // Fund size adds substance expectation
    if input.fund_size >= dec!(50_000_000) {
        score += 2;
    } else if input.fund_size >= dec!(10_000_000) {
        score += 1;
    }

    let final_score = score.min(10);

    if final_score < 5 {
        recs.push(
            "Substance score below 5/10 — increase local presence for \
             regulatory and tax incentive compliance"
                .to_string(),
        );
    }

    recommendations.append(&mut recs);

    VccSubstanceAnalysis {
        substance_score: final_score,
        investment_professionals: input.investment_professionals_sg,
        local_spending: input.local_business_spending,
        sg_resident_director: true,
        recommendations: Vec::new(),
    }
}

// ---------------------------------------------------------------------------
// Helpers — Cost Estimation
// ---------------------------------------------------------------------------

fn estimate_vcc_costs(vcc_type: &str, num_sub_funds: u32) -> (Decimal, Decimal, Decimal, Decimal) {
    match vcc_type {
        "Standalone" => (
            dec!(30_000),  // setup low
            dec!(50_000),  // setup high
            dec!(80_000),  // annual low
            dec!(120_000), // annual high
        ),
        _ => {
            // Umbrella — base + per sub-fund increment
            let base_setup_low = dec!(40_000);
            let base_setup_high = dec!(60_000);
            let per_sub_setup = dec!(10_000);

            let base_annual_low = dec!(80_000);
            let base_annual_high = dec!(130_000);
            let per_sub_annual = dec!(20_000);

            let n = Decimal::from(num_sub_funds.max(1));

            (
                base_setup_low + n * per_sub_setup,
                base_setup_high + n * per_sub_setup * dec!(1.5),
                base_annual_low + n * per_sub_annual,
                base_annual_high + n * per_sub_annual * dec!(1.5),
            )
        }
    }
}

fn estimate_sub_fund_direct_cost(aum: Decimal) -> Decimal {
    // Sub-fund admin + audit + custody
    if aum >= dec!(500_000_000) {
        dec!(80_000)
    } else if aum >= dec!(100_000_000) {
        dec!(50_000)
    } else if aum >= dec!(10_000_000) {
        dec!(30_000)
    } else {
        dec!(20_000)
    }
}

// ---------------------------------------------------------------------------
// Helpers — Tax Incentive
// ---------------------------------------------------------------------------

fn evaluate_scheme_conditions(input: &TaxIncentiveInput) -> (Vec<ConditionResult>, bool) {
    let mut conditions = Vec::new();
    let mut all_passed = true;

    match input.scheme.as_str() {
        "S13O" => {
            let fund_size_ok = input.fund_size >= dec!(10_000_000);
            conditions.push(ConditionResult {
                condition: "Minimum fund size SGD 10M".to_string(),
                required: "SGD 10,000,000".to_string(),
                actual: format!("SGD {}", input.fund_size),
                passed: fund_size_ok,
            });
            if !fund_size_ok {
                all_passed = false;
            }

            let ip_ok = input.investment_professionals_sg >= 1;
            conditions.push(ConditionResult {
                condition: "Minimum 1 investment professional in Singapore".to_string(),
                required: "1".to_string(),
                actual: format!("{}", input.investment_professionals_sg),
                passed: ip_ok,
            });
            if !ip_ok {
                all_passed = false;
            }

            let spend_ok = input.local_business_spending >= dec!(200_000);
            conditions.push(ConditionResult {
                condition: "Minimum SGD 200k local business spending".to_string(),
                required: "SGD 200,000".to_string(),
                actual: format!("SGD {}", input.local_business_spending),
                passed: spend_ok,
            });
            if !spend_ok {
                all_passed = false;
            }

            let manager_sg_ok = true; // VCC manager assumed in SG
            conditions.push(ConditionResult {
                condition: "Fund managed by Singapore-based manager".to_string(),
                required: "Yes".to_string(),
                actual: "Yes (VCC)".to_string(),
                passed: manager_sg_ok,
            });
        }
        "S13U" => {
            let fund_size_ok = input.fund_size >= dec!(50_000_000);
            conditions.push(ConditionResult {
                condition: "Minimum fund size SGD 50M".to_string(),
                required: "SGD 50,000,000".to_string(),
                actual: format!("SGD {}", input.fund_size),
                passed: fund_size_ok,
            });
            if !fund_size_ok {
                all_passed = false;
            }

            let ip_ok = input.investment_professionals_sg >= 3;
            conditions.push(ConditionResult {
                condition: "Minimum 3 investment professionals in Singapore".to_string(),
                required: "3".to_string(),
                actual: format!("{}", input.investment_professionals_sg),
                passed: ip_ok,
            });
            if !ip_ok {
                all_passed = false;
            }

            let spend_ok = input.local_business_spending >= dec!(200_000);
            conditions.push(ConditionResult {
                condition: "Minimum SGD 200k local business spending".to_string(),
                required: "SGD 200,000".to_string(),
                actual: format!("SGD {}", input.local_business_spending),
                passed: spend_ok,
            });
            if !spend_ok {
                all_passed = false;
            }

            let manager_sg_ok = true;
            conditions.push(ConditionResult {
                condition: "Fund managed by Singapore-based manager".to_string(),
                required: "Yes".to_string(),
                actual: "Yes (VCC)".to_string(),
                passed: manager_sg_ok,
            });
        }
        "S13D" => {
            let non_resident_ok = !input.is_resident;
            conditions.push(ConditionResult {
                condition: "Fund is non-resident for tax purposes".to_string(),
                required: "Non-resident".to_string(),
                actual: if input.is_resident {
                    "Resident".to_string()
                } else {
                    "Non-resident".to_string()
                },
                passed: non_resident_ok,
            });
            if !non_resident_ok {
                all_passed = false;
            }

            // S13D has no AUM minimum
            conditions.push(ConditionResult {
                condition: "No minimum fund size".to_string(),
                required: "None".to_string(),
                actual: format!("SGD {}", input.fund_size),
                passed: true,
            });
        }
        _ => {
            all_passed = false;
        }
    }

    (conditions, all_passed)
}

fn build_scheme_comparison(input: &TaxIncentiveInput, tax_rate: Decimal) -> Vec<SchemeComparison> {
    let qualifying = input.qualifying_income;

    // S13O
    let s13o_eligible = input.fund_size >= dec!(10_000_000)
        && input.investment_professionals_sg >= 1
        && input.local_business_spending >= dec!(200_000);
    let s13o_savings = if s13o_eligible {
        qualifying * tax_rate
    } else {
        Decimal::ZERO
    };

    // S13U
    let s13u_eligible = input.fund_size >= dec!(50_000_000)
        && input.investment_professionals_sg >= 3
        && input.local_business_spending >= dec!(200_000);
    let s13u_savings = if s13u_eligible {
        qualifying * tax_rate
    } else {
        Decimal::ZERO
    };

    // S13D
    let s13d_eligible = !input.is_resident;
    let s13d_savings = if s13d_eligible {
        qualifying * tax_rate
    } else {
        Decimal::ZERO
    };

    vec![
        SchemeComparison {
            scheme: "S13O".to_string(),
            eligible: s13o_eligible,
            tax_savings: s13o_savings,
            key_requirement: "Fund size >= SGD 10M, 1+ IP in SG, SGD 200k spend".to_string(),
        },
        SchemeComparison {
            scheme: "S13U".to_string(),
            eligible: s13u_eligible,
            tax_savings: s13u_savings,
            key_requirement: "Fund size >= SGD 50M, 3+ IP in SG, SGD 200k spend".to_string(),
        },
        SchemeComparison {
            scheme: "S13D".to_string(),
            eligible: s13d_eligible,
            tax_savings: s13d_savings,
            key_requirement: "Non-resident fund, no AUM minimum".to_string(),
        },
    ]
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

fn validate_vcc_input(input: &VccInput) -> CorpFinanceResult<()> {
    if input.fund_name.trim().is_empty() {
        return Err(CorpFinanceError::InvalidInput {
            field: "fund_name".into(),
            reason: "Fund name cannot be empty".into(),
        });
    }

    let valid_types = ["Standalone", "Umbrella"];
    if !valid_types.contains(&input.vcc_type.as_str()) {
        return Err(CorpFinanceError::InvalidInput {
            field: "vcc_type".into(),
            reason: format!(
                "Unknown VCC type '{}'. Valid: {:?}",
                input.vcc_type, valid_types
            ),
        });
    }

    let valid_licenses = ["RFMC", "LRFMC", "A_LFMC"];
    if !valid_licenses.contains(&input.manager_license.as_str()) {
        return Err(CorpFinanceError::InvalidInput {
            field: "manager_license".into(),
            reason: format!(
                "Unknown license type '{}'. Valid: {:?}",
                input.manager_license, valid_licenses
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

    // RFMC-specific validation
    if input.manager_license == "RFMC" {
        if input.fund_size > dec!(250_000_000) {
            return Err(CorpFinanceError::InvalidInput {
                field: "fund_size".into(),
                reason: "RFMC license limits AUM to SGD 250M".into(),
            });
        }
        if input.target_investors.len() > 30 {
            return Err(CorpFinanceError::InvalidInput {
                field: "target_investors".into(),
                reason: "RFMC license limits investors to 30 qualified investors".into(),
            });
        }
    }

    if let Some(ref scheme) = input.tax_incentive_scheme {
        let valid_schemes = ["S13O", "S13U", "S13D"];
        if !valid_schemes.contains(&scheme.as_str()) {
            return Err(CorpFinanceError::InvalidInput {
                field: "tax_incentive_scheme".into(),
                reason: format!(
                    "Unknown tax incentive scheme '{}'. Valid: {:?}",
                    scheme, valid_schemes
                ),
            });
        }
    }

    // Sub-fund AUM tolerance check (only for Umbrella with sub-funds)
    if input.vcc_type == "Umbrella" && !input.sub_funds.is_empty() {
        let total_sub_aum: Decimal = input.sub_funds.iter().map(|sf| sf.target_aum).sum();
        let diff = (total_sub_aum - input.fund_size).abs();
        let tolerance = input.fund_size * dec!(0.01);
        if diff > tolerance {
            return Err(CorpFinanceError::InvalidInput {
                field: "sub_funds".into(),
                reason: format!(
                    "Sub-fund AUM total (SGD {}) differs from fund_size (SGD {}) \
                     by more than 1%",
                    total_sub_aum, input.fund_size
                ),
            });
        }
    }

    Ok(())
}

fn validate_sub_fund_allocation_input(input: &SubFundAllocationInput) -> CorpFinanceResult<()> {
    if input.fund_name.trim().is_empty() {
        return Err(CorpFinanceError::InvalidInput {
            field: "fund_name".into(),
            reason: "Fund name cannot be empty".into(),
        });
    }

    if input.sub_funds.is_empty() {
        return Err(CorpFinanceError::InvalidInput {
            field: "sub_funds".into(),
            reason: "At least one sub-fund is required".into(),
        });
    }

    if input.total_fund_size <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "total_fund_size".into(),
            reason: "Total fund size must be greater than zero".into(),
        });
    }

    let valid_methods = ["AumWeighted", "EqualWeighted", "Hybrid"];
    if !valid_methods.contains(&input.allocation_method.as_str()) {
        return Err(CorpFinanceError::InvalidInput {
            field: "allocation_method".into(),
            reason: format!(
                "Unknown allocation method '{}'. Valid: {:?}",
                input.allocation_method, valid_methods
            ),
        });
    }

    for sf in &input.sub_funds {
        if sf.target_aum <= Decimal::ZERO {
            return Err(CorpFinanceError::InvalidInput {
                field: format!("sub_funds[{}].target_aum", sf.name),
                reason: "Sub-fund AUM must be greater than zero".into(),
            });
        }
    }

    Ok(())
}

fn validate_tax_incentive_input(input: &TaxIncentiveInput) -> CorpFinanceResult<()> {
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

    if input.qualifying_income < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "qualifying_income".into(),
            reason: "Qualifying income cannot be negative".into(),
        });
    }

    if input.non_qualifying_income < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "non_qualifying_income".into(),
            reason: "Non-qualifying income cannot be negative".into(),
        });
    }

    let valid_schemes = ["S13O", "S13U", "S13D"];
    if !valid_schemes.contains(&input.scheme.as_str()) {
        return Err(CorpFinanceError::InvalidInput {
            field: "scheme".into(),
            reason: format!(
                "Unknown scheme '{}'. Valid: {:?}",
                input.scheme, valid_schemes
            ),
        });
    }

    Ok(())
}

fn validate_comparison_input(input: &VccCaymanCompInput) -> CorpFinanceResult<()> {
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

    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    // ------------------------------------------------------------------
    // Test helpers
    // ------------------------------------------------------------------

    fn standalone_vcc_input() -> VccInput {
        VccInput {
            fund_name: "SG Alpha Fund".to_string(),
            vcc_type: "Standalone".to_string(),
            sub_funds: vec![],
            manager_license: "LRFMC".to_string(),
            fund_size: dec!(100_000_000),
            management_fee_rate: dec!(0.02),
            performance_fee_rate: dec!(0.20),
            tax_incentive_scheme: Some("S13O".to_string()),
            investment_professionals_sg: 2,
            local_business_spending: dec!(300_000),
            target_investors: vec![
                "AsiaInstitutional".to_string(),
                "AccreditedInvestor".to_string(),
            ],
        }
    }

    fn umbrella_vcc_input() -> VccInput {
        VccInput {
            fund_name: "SG Umbrella VCC".to_string(),
            vcc_type: "Umbrella".to_string(),
            sub_funds: vec![
                SubFundInfo {
                    name: "Equity Sub-Fund".to_string(),
                    strategy: "Equity".to_string(),
                    target_aum: dec!(50_000_000),
                    currency: "SGD".to_string(),
                },
                SubFundInfo {
                    name: "Fixed Income Sub-Fund".to_string(),
                    strategy: "FixedIncome".to_string(),
                    target_aum: dec!(30_000_000),
                    currency: "SGD".to_string(),
                },
                SubFundInfo {
                    name: "Credit Sub-Fund".to_string(),
                    strategy: "Credit".to_string(),
                    target_aum: dec!(20_000_000),
                    currency: "USD".to_string(),
                },
            ],
            manager_license: "LRFMC".to_string(),
            fund_size: dec!(100_000_000),
            management_fee_rate: dec!(0.015),
            performance_fee_rate: dec!(0.15),
            tax_incentive_scheme: Some("S13O".to_string()),
            investment_professionals_sg: 3,
            local_business_spending: dec!(250_000),
            target_investors: vec!["AsiaInstitutional".to_string()],
        }
    }

    fn sub_fund_allocation_input() -> SubFundAllocationInput {
        SubFundAllocationInput {
            fund_name: "SG Umbrella VCC".to_string(),
            sub_funds: vec![
                SubFundInfo {
                    name: "Equity".to_string(),
                    strategy: "Equity".to_string(),
                    target_aum: dec!(50_000_000),
                    currency: "SGD".to_string(),
                },
                SubFundInfo {
                    name: "Bond".to_string(),
                    strategy: "FixedIncome".to_string(),
                    target_aum: dec!(30_000_000),
                    currency: "SGD".to_string(),
                },
                SubFundInfo {
                    name: "Credit".to_string(),
                    strategy: "Credit".to_string(),
                    target_aum: dec!(20_000_000),
                    currency: "USD".to_string(),
                },
            ],
            total_fund_size: dec!(100_000_000),
            allocation_method: "AumWeighted".to_string(),
            shared_costs: SharedCosts {
                board_fees: dec!(50_000),
                company_secretary: dec!(15_000),
                registered_office: dec!(10_000),
                compliance: dec!(30_000),
                audit_umbrella: dec!(20_000),
            },
        }
    }

    fn tax_incentive_s13o_input() -> TaxIncentiveInput {
        TaxIncentiveInput {
            fund_name: "SG Alpha Fund".to_string(),
            fund_size: dec!(100_000_000),
            qualifying_income: dec!(5_000_000),
            non_qualifying_income: dec!(500_000),
            investment_professionals_sg: 2,
            local_business_spending: dec!(300_000),
            scheme: "S13O".to_string(),
            is_resident: true,
        }
    }

    fn comparison_input() -> VccCaymanCompInput {
        VccCaymanCompInput {
            fund_name: "Global Fund".to_string(),
            fund_size: dec!(200_000_000),
            num_sub_funds: 3,
            management_fee_rate: dec!(0.02),
            target_investors: vec![
                "AsiaInstitutional".to_string(),
                "USInstitutional".to_string(),
            ],
            investment_professionals_sg: 2,
            local_business_spending: dec!(250_000),
        }
    }

    // ------------------------------------------------------------------
    // 1. Standalone VCC — basic structure
    // ------------------------------------------------------------------
    #[test]
    fn test_standalone_vcc_structure() {
        let input = standalone_vcc_input();
        let result = analyze_vcc_structure(&input).unwrap();

        assert_eq!(result.structure_analysis.vcc_type, "Standalone");
        assert!(!result.structure_analysis.segregated_assets);
    }

    // ------------------------------------------------------------------
    // 2. Umbrella VCC — segregated assets
    // ------------------------------------------------------------------
    #[test]
    fn test_umbrella_vcc_segregated_assets() {
        let input = umbrella_vcc_input();
        let result = analyze_vcc_structure(&input).unwrap();

        assert_eq!(result.structure_analysis.vcc_type, "Umbrella");
        assert!(result.structure_analysis.segregated_assets);
    }

    // ------------------------------------------------------------------
    // 3. Umbrella VCC — 3 sub-funds parsed
    // ------------------------------------------------------------------
    #[test]
    fn test_umbrella_vcc_sub_funds() {
        let input = umbrella_vcc_input();
        let result = analyze_vcc_structure(&input).unwrap();

        assert_eq!(result.sub_fund_analysis.len(), 3);
        assert_eq!(result.sub_fund_analysis[0].name, "Equity Sub-Fund");
        assert_eq!(result.sub_fund_analysis[1].name, "Fixed Income Sub-Fund");
        assert_eq!(result.sub_fund_analysis[2].name, "Credit Sub-Fund");
    }

    // ------------------------------------------------------------------
    // 4. Sub-fund percentage of total
    // ------------------------------------------------------------------
    #[test]
    fn test_sub_fund_pct_of_total() {
        let input = umbrella_vcc_input();
        let result = analyze_vcc_structure(&input).unwrap();

        assert_eq!(result.sub_fund_analysis[0].pct_of_total, dec!(0.5));
        assert_eq!(result.sub_fund_analysis[1].pct_of_total, dec!(0.3));
        assert_eq!(result.sub_fund_analysis[2].pct_of_total, dec!(0.2));
    }

    // ------------------------------------------------------------------
    // 5. LRFMC licensing — compliant
    // ------------------------------------------------------------------
    #[test]
    fn test_lrfmc_licensing_compliant() {
        let input = standalone_vcc_input();
        let result = analyze_vcc_structure(&input).unwrap();

        assert!(result.licensing_analysis.compliant);
        assert!(result.licensing_analysis.aum_limit.is_none());
    }

    // ------------------------------------------------------------------
    // 6. RFMC licensing — within limits
    // ------------------------------------------------------------------
    #[test]
    fn test_rfmc_licensing_within_limits() {
        let mut input = standalone_vcc_input();
        input.manager_license = "RFMC".to_string();
        input.fund_size = dec!(200_000_000);
        input.target_investors = (0..25).map(|i| format!("Investor_{}", i)).collect();
        let result = analyze_vcc_structure(&input).unwrap();

        assert!(result.licensing_analysis.compliant);
        assert_eq!(result.licensing_analysis.aum_limit, Some(dec!(250_000_000)));
        assert_eq!(result.licensing_analysis.investor_limit, Some(30));
    }

    // ------------------------------------------------------------------
    // 7. RFMC — AUM exceeds limit (validation error)
    // ------------------------------------------------------------------
    #[test]
    fn test_rfmc_aum_exceeds_limit() {
        let mut input = standalone_vcc_input();
        input.manager_license = "RFMC".to_string();
        input.fund_size = dec!(300_000_000);
        let result = analyze_vcc_structure(&input);
        assert!(result.is_err());
    }

    // ------------------------------------------------------------------
    // 8. RFMC — investor count at boundary (30)
    // ------------------------------------------------------------------
    #[test]
    fn test_rfmc_investor_count_at_boundary() {
        let mut input = standalone_vcc_input();
        input.manager_license = "RFMC".to_string();
        input.fund_size = dec!(100_000_000);
        input.target_investors = (0..30).map(|i| format!("Investor_{}", i)).collect();
        let result = analyze_vcc_structure(&input);
        assert!(result.is_ok());
    }

    // ------------------------------------------------------------------
    // 9. RFMC — investor count exceeds boundary (31)
    // ------------------------------------------------------------------
    #[test]
    fn test_rfmc_investor_count_exceeds_boundary() {
        let mut input = standalone_vcc_input();
        input.manager_license = "RFMC".to_string();
        input.fund_size = dec!(100_000_000);
        input.target_investors = (0..31).map(|i| format!("Investor_{}", i)).collect();
        let result = analyze_vcc_structure(&input);
        assert!(result.is_err());
    }

    // ------------------------------------------------------------------
    // 10. Regulatory — VCC Act requirements
    // ------------------------------------------------------------------
    #[test]
    fn test_vcc_regulatory_requirements() {
        let input = standalone_vcc_input();
        let result = analyze_vcc_structure(&input).unwrap();

        assert!(result.regulatory.sg_resident_director_required);
        assert!(result.regulatory.registered_office_sg);
        assert!(result.regulatory.company_secretary_required);
        assert!(result.regulatory.vcc_act_2018);
    }

    // ------------------------------------------------------------------
    // 11. Substance score — good substance
    // ------------------------------------------------------------------
    #[test]
    fn test_substance_score_good() {
        let input = umbrella_vcc_input();
        let result = analyze_vcc_structure(&input).unwrap();

        // 3 IPs, 250k spend, 100M fund, SG director → high score
        assert!(
            result.substance_analysis.substance_score >= 7,
            "Expected score >= 7, got {}",
            result.substance_analysis.substance_score
        );
    }

    // ------------------------------------------------------------------
    // 12. Substance score — no IPs
    // ------------------------------------------------------------------
    #[test]
    fn test_substance_score_no_ips() {
        let mut input = standalone_vcc_input();
        input.investment_professionals_sg = 0;
        input.local_business_spending = Decimal::ZERO;
        input.fund_size = dec!(5_000_000);
        // RFMC would fail with 5M, use LRFMC
        input.manager_license = "LRFMC".to_string();
        let result = analyze_vcc_structure(&input).unwrap();

        assert!(
            result.substance_analysis.substance_score < 5,
            "Expected score < 5, got {}",
            result.substance_analysis.substance_score
        );
    }

    // ------------------------------------------------------------------
    // 13. Cost analysis — standalone range
    // ------------------------------------------------------------------
    #[test]
    fn test_cost_analysis_standalone() {
        let input = standalone_vcc_input();
        let result = analyze_vcc_structure(&input).unwrap();

        assert_eq!(result.cost_analysis.setup_cost_low, dec!(30_000));
        assert_eq!(result.cost_analysis.setup_cost_high, dec!(50_000));
        assert!(result.cost_analysis.annual_cost_low >= dec!(80_000));
        assert!(result.cost_analysis.annual_cost_high <= dec!(200_000));
    }

    // ------------------------------------------------------------------
    // 14. Cost analysis — umbrella higher than standalone
    // ------------------------------------------------------------------
    #[test]
    fn test_cost_umbrella_higher_than_standalone() {
        let standalone = standalone_vcc_input();
        let umbrella = umbrella_vcc_input();
        let r1 = analyze_vcc_structure(&standalone).unwrap();
        let r2 = analyze_vcc_structure(&umbrella).unwrap();

        assert!(r2.cost_analysis.setup_cost_low > r1.cost_analysis.setup_cost_low);
        assert!(r2.cost_analysis.annual_cost_low > r1.cost_analysis.annual_cost_low);
    }

    // ------------------------------------------------------------------
    // 15. Cost pct of AUM is reasonable
    // ------------------------------------------------------------------
    #[test]
    fn test_cost_pct_of_aum_reasonable() {
        let input = standalone_vcc_input();
        let result = analyze_vcc_structure(&input).unwrap();

        assert!(result.cost_analysis.cost_pct_of_aum > Decimal::ZERO);
        assert!(
            result.cost_analysis.cost_pct_of_aum < dec!(0.05),
            "Cost should be < 5% of AUM"
        );
    }

    // ------------------------------------------------------------------
    // 16. Tax incentive warning when fund below S13O minimum
    // ------------------------------------------------------------------
    #[test]
    fn test_tax_warning_below_s13o_minimum() {
        let mut input = standalone_vcc_input();
        input.fund_size = dec!(5_000_000);
        input.tax_incentive_scheme = Some("S13O".to_string());
        // Must not be RFMC since 5M < 250M is fine, but we need LRFMC
        input.manager_license = "LRFMC".to_string();
        let result = analyze_vcc_structure(&input).unwrap();

        assert!(result.warnings.iter().any(|w| w.contains("S13O")));
    }

    // ------------------------------------------------------------------
    // 17. A-LFMC licensing — no investor limit
    // ------------------------------------------------------------------
    #[test]
    fn test_a_lfmc_no_limits() {
        let mut input = standalone_vcc_input();
        input.manager_license = "A_LFMC".to_string();
        let result = analyze_vcc_structure(&input).unwrap();

        assert!(result.licensing_analysis.aum_limit.is_none());
        assert!(result.licensing_analysis.investor_limit.is_none());
        assert!(result.licensing_analysis.compliant);
    }

    // ------------------------------------------------------------------
    // 18. Validation — empty fund name
    // ------------------------------------------------------------------
    #[test]
    fn test_validation_empty_fund_name() {
        let mut input = standalone_vcc_input();
        input.fund_name = "".to_string();
        let result = analyze_vcc_structure(&input);
        assert!(result.is_err());
        match result.unwrap_err() {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "fund_name");
            }
            other => panic!("Expected InvalidInput, got: {other}"),
        }
    }

    // ------------------------------------------------------------------
    // 19. Validation — invalid VCC type
    // ------------------------------------------------------------------
    #[test]
    fn test_validation_invalid_vcc_type() {
        let mut input = standalone_vcc_input();
        input.vcc_type = "Hybrid".to_string();
        let result = analyze_vcc_structure(&input);
        assert!(result.is_err());
    }

    // ------------------------------------------------------------------
    // 20. Validation — negative fund size
    // ------------------------------------------------------------------
    #[test]
    fn test_validation_negative_fund_size() {
        let mut input = standalone_vcc_input();
        input.fund_size = dec!(-1);
        let result = analyze_vcc_structure(&input);
        assert!(result.is_err());
    }

    // ------------------------------------------------------------------
    // 21. Validation — zero fund size
    // ------------------------------------------------------------------
    #[test]
    fn test_validation_zero_fund_size() {
        let mut input = standalone_vcc_input();
        input.fund_size = Decimal::ZERO;
        let result = analyze_vcc_structure(&input);
        assert!(result.is_err());
    }

    // ------------------------------------------------------------------
    // 22. Validation — invalid license type
    // ------------------------------------------------------------------
    #[test]
    fn test_validation_invalid_license() {
        let mut input = standalone_vcc_input();
        input.manager_license = "CMS".to_string();
        let result = analyze_vcc_structure(&input);
        assert!(result.is_err());
    }

    // ------------------------------------------------------------------
    // 23. Validation — fee rate out of range
    // ------------------------------------------------------------------
    #[test]
    fn test_validation_fee_rate_too_high() {
        let mut input = standalone_vcc_input();
        input.management_fee_rate = dec!(1.5);
        let result = analyze_vcc_structure(&input);
        assert!(result.is_err());
    }

    // ------------------------------------------------------------------
    // 24. Validation — invalid tax incentive scheme
    // ------------------------------------------------------------------
    #[test]
    fn test_validation_invalid_tax_scheme() {
        let mut input = standalone_vcc_input();
        input.tax_incentive_scheme = Some("S13Z".to_string());
        let result = analyze_vcc_structure(&input);
        assert!(result.is_err());
    }

    // ------------------------------------------------------------------
    // 25. Sub-fund allocation — AUM weighted
    // ------------------------------------------------------------------
    #[test]
    fn test_sub_fund_allocation_aum_weighted() {
        let input = sub_fund_allocation_input();
        let result = vcc_sub_fund_allocation(&input).unwrap();

        assert_eq!(result.per_sub_fund_economics.len(), 3);

        // Total shared = 50k + 15k + 10k + 30k + 20k = 125k
        let total_shared = dec!(125_000);
        assert_eq!(result.aggregate_economics.total_shared_costs, total_shared);

        // Equity (50M/100M = 50%) gets 50% of shared = 62,500
        let equity = &result.per_sub_fund_economics[0];
        assert_eq!(equity.aum_weight, dec!(0.5));
        assert_eq!(equity.allocated_shared_cost, dec!(62_500));
    }

    // ------------------------------------------------------------------
    // 26. Sub-fund allocation — equal weighted
    // ------------------------------------------------------------------
    #[test]
    fn test_sub_fund_allocation_equal_weighted() {
        let mut input = sub_fund_allocation_input();
        input.allocation_method = "EqualWeighted".to_string();
        let result = vcc_sub_fund_allocation(&input).unwrap();

        // Each gets 1/3 of 125k shared
        let per_fund_shared = dec!(125_000) / dec!(3);
        for sf in &result.per_sub_fund_economics {
            // Allow for rounding
            let diff = (sf.allocated_shared_cost - per_fund_shared).abs();
            assert!(
                diff < dec!(1),
                "Expected ~{}, got {}",
                per_fund_shared,
                sf.allocated_shared_cost
            );
        }
    }

    // ------------------------------------------------------------------
    // 27. Sub-fund allocation — hybrid (50/50)
    // ------------------------------------------------------------------
    #[test]
    fn test_sub_fund_allocation_hybrid() {
        let mut input = sub_fund_allocation_input();
        input.allocation_method = "Hybrid".to_string();
        let result = vcc_sub_fund_allocation(&input).unwrap();

        // Equity: 50% AUM-weighted portion + 1/3 equal portion
        // = 125k * 0.5 * 0.5 + 125k * 0.5 / 3 = 31,250 + 20,833.33
        let equity = &result.per_sub_fund_economics[0];
        let expected_aum_part = dec!(125_000) * dec!(0.5) * dec!(0.5);
        let expected_equal_part = dec!(125_000) * dec!(0.5) / dec!(3);
        let expected = expected_aum_part + expected_equal_part;
        let diff = (equity.allocated_shared_cost - expected).abs();
        assert!(
            diff < dec!(1),
            "Expected ~{}, got {}",
            expected,
            equity.allocated_shared_cost
        );
    }

    // ------------------------------------------------------------------
    // 28. Sub-fund TER calculation
    // ------------------------------------------------------------------
    #[test]
    fn test_sub_fund_ter_calculation() {
        let input = sub_fund_allocation_input();
        let result = vcc_sub_fund_allocation(&input).unwrap();

        for sf in &result.per_sub_fund_economics {
            assert!(sf.ter > Decimal::ZERO);
            let expected_ter = sf.total_cost / sf.aum;
            assert_eq!(sf.ter, expected_ter);
        }
    }

    // ------------------------------------------------------------------
    // 29. Sub-fund aggregate TER
    // ------------------------------------------------------------------
    #[test]
    fn test_aggregate_ter() {
        let input = sub_fund_allocation_input();
        let result = vcc_sub_fund_allocation(&input).unwrap();

        let expected = result.aggregate_economics.total_costs / dec!(100_000_000);
        assert_eq!(result.aggregate_economics.aggregate_ter, expected);
    }

    // ------------------------------------------------------------------
    // 30. Sub-fund marginal cost
    // ------------------------------------------------------------------
    #[test]
    fn test_marginal_cost_per_sub_fund() {
        let input = sub_fund_allocation_input();
        let result = vcc_sub_fund_allocation(&input).unwrap();

        assert!(result.marginal_cost_per_sub_fund > Decimal::ZERO);
    }

    // ------------------------------------------------------------------
    // 31. Sub-fund allocation validation — empty sub-funds
    // ------------------------------------------------------------------
    #[test]
    fn test_allocation_validation_empty_sub_funds() {
        let mut input = sub_fund_allocation_input();
        input.sub_funds = vec![];
        let result = vcc_sub_fund_allocation(&input);
        assert!(result.is_err());
    }

    // ------------------------------------------------------------------
    // 32. Sub-fund allocation validation — zero total size
    // ------------------------------------------------------------------
    #[test]
    fn test_allocation_validation_zero_total_size() {
        let mut input = sub_fund_allocation_input();
        input.total_fund_size = Decimal::ZERO;
        let result = vcc_sub_fund_allocation(&input);
        assert!(result.is_err());
    }

    // ------------------------------------------------------------------
    // 33. Tax incentive — S13O eligible
    // ------------------------------------------------------------------
    #[test]
    fn test_s13o_eligible() {
        let input = tax_incentive_s13o_input();
        let result = tax_incentive_analysis(&input).unwrap();

        assert!(result.scheme_eligibility.eligible);
        assert_eq!(result.scheme_eligibility.scheme, "S13O");
        // Tax savings = 5M * 17% = 850k
        assert_eq!(result.tax_savings, dec!(850_000));
    }

    // ------------------------------------------------------------------
    // 34. Tax incentive — S13O conditions all pass
    // ------------------------------------------------------------------
    #[test]
    fn test_s13o_all_conditions_pass() {
        let input = tax_incentive_s13o_input();
        let result = tax_incentive_analysis(&input).unwrap();

        assert_eq!(
            result.scheme_eligibility.conditions_met,
            result.scheme_eligibility.conditions_total
        );
        assert!(result.conditions.iter().all(|c| c.passed));
    }

    // ------------------------------------------------------------------
    // 35. Tax incentive — S13O ineligible (fund too small)
    // ------------------------------------------------------------------
    #[test]
    fn test_s13o_ineligible_small_fund() {
        let mut input = tax_incentive_s13o_input();
        input.fund_size = dec!(5_000_000);
        let result = tax_incentive_analysis(&input).unwrap();

        assert!(!result.scheme_eligibility.eligible);
        assert_eq!(result.tax_savings, Decimal::ZERO);
    }

    // ------------------------------------------------------------------
    // 36. Tax incentive — S13O ineligible (no IPs)
    // ------------------------------------------------------------------
    #[test]
    fn test_s13o_ineligible_no_ips() {
        let mut input = tax_incentive_s13o_input();
        input.investment_professionals_sg = 0;
        let result = tax_incentive_analysis(&input).unwrap();

        assert!(!result.scheme_eligibility.eligible);
    }

    // ------------------------------------------------------------------
    // 37. Tax incentive — S13O ineligible (low spending)
    // ------------------------------------------------------------------
    #[test]
    fn test_s13o_ineligible_low_spending() {
        let mut input = tax_incentive_s13o_input();
        input.local_business_spending = dec!(100_000);
        let result = tax_incentive_analysis(&input).unwrap();

        assert!(!result.scheme_eligibility.eligible);
    }

    // ------------------------------------------------------------------
    // 38. Tax incentive — S13U eligible
    // ------------------------------------------------------------------
    #[test]
    fn test_s13u_eligible() {
        let mut input = tax_incentive_s13o_input();
        input.scheme = "S13U".to_string();
        input.fund_size = dec!(100_000_000);
        input.investment_professionals_sg = 3;
        input.local_business_spending = dec!(500_000);
        let result = tax_incentive_analysis(&input).unwrap();

        assert!(result.scheme_eligibility.eligible);
    }

    // ------------------------------------------------------------------
    // 39. Tax incentive — S13U threshold boundary (exactly 50M)
    // ------------------------------------------------------------------
    #[test]
    fn test_s13u_threshold_boundary() {
        let mut input = tax_incentive_s13o_input();
        input.scheme = "S13U".to_string();
        input.fund_size = dec!(50_000_000);
        input.investment_professionals_sg = 3;
        input.local_business_spending = dec!(200_000);
        let result = tax_incentive_analysis(&input).unwrap();

        assert!(result.scheme_eligibility.eligible);
    }

    // ------------------------------------------------------------------
    // 40. Tax incentive — S13U below threshold (49M)
    // ------------------------------------------------------------------
    #[test]
    fn test_s13u_below_threshold() {
        let mut input = tax_incentive_s13o_input();
        input.scheme = "S13U".to_string();
        input.fund_size = dec!(49_000_000);
        input.investment_professionals_sg = 3;
        input.local_business_spending = dec!(200_000);
        let result = tax_incentive_analysis(&input).unwrap();

        assert!(!result.scheme_eligibility.eligible);
    }

    // ------------------------------------------------------------------
    // 41. Tax incentive — S13U ineligible (only 2 IPs)
    // ------------------------------------------------------------------
    #[test]
    fn test_s13u_ineligible_insufficient_ips() {
        let mut input = tax_incentive_s13o_input();
        input.scheme = "S13U".to_string();
        input.fund_size = dec!(100_000_000);
        input.investment_professionals_sg = 2;
        let result = tax_incentive_analysis(&input).unwrap();

        assert!(!result.scheme_eligibility.eligible);
    }

    // ------------------------------------------------------------------
    // 42. Tax incentive — S13D eligible (non-resident)
    // ------------------------------------------------------------------
    #[test]
    fn test_s13d_eligible_non_resident() {
        let mut input = tax_incentive_s13o_input();
        input.scheme = "S13D".to_string();
        input.is_resident = false;
        let result = tax_incentive_analysis(&input).unwrap();

        assert!(result.scheme_eligibility.eligible);
        assert!(result.tax_savings > Decimal::ZERO);
    }

    // ------------------------------------------------------------------
    // 43. Tax incentive — S13D ineligible (resident)
    // ------------------------------------------------------------------
    #[test]
    fn test_s13d_ineligible_resident() {
        let mut input = tax_incentive_s13o_input();
        input.scheme = "S13D".to_string();
        input.is_resident = true;
        let result = tax_incentive_analysis(&input).unwrap();

        assert!(!result.scheme_eligibility.eligible);
    }

    // ------------------------------------------------------------------
    // 44. Tax incentive — non-qualifying income always taxed
    // ------------------------------------------------------------------
    #[test]
    fn test_non_qualifying_income_taxed() {
        let input = tax_incentive_s13o_input();
        let result = tax_incentive_analysis(&input).unwrap();

        // 500k * 17% = 85k
        assert_eq!(result.non_qualifying_income_tax, dec!(85_000));
    }

    // ------------------------------------------------------------------
    // 45. Tax incentive — comparison includes all 3 schemes
    // ------------------------------------------------------------------
    #[test]
    fn test_scheme_comparison_all_three() {
        let input = tax_incentive_s13o_input();
        let result = tax_incentive_analysis(&input).unwrap();

        assert_eq!(result.comparison_all_schemes.len(), 3);
        let schemes: Vec<&str> = result
            .comparison_all_schemes
            .iter()
            .map(|s| s.scheme.as_str())
            .collect();
        assert!(schemes.contains(&"S13O"));
        assert!(schemes.contains(&"S13U"));
        assert!(schemes.contains(&"S13D"));
    }

    // ------------------------------------------------------------------
    // 46. Tax incentive validation — invalid scheme
    // ------------------------------------------------------------------
    #[test]
    fn test_tax_validation_invalid_scheme() {
        let mut input = tax_incentive_s13o_input();
        input.scheme = "S13Z".to_string();
        let result = tax_incentive_analysis(&input);
        assert!(result.is_err());
    }

    // ------------------------------------------------------------------
    // 47. VCC vs Cayman SPC — basic comparison
    // ------------------------------------------------------------------
    #[test]
    fn test_vcc_vs_cayman_spc_basic() {
        let input = comparison_input();
        let result = vcc_vs_cayman_spc(&input).unwrap();

        assert_eq!(result.vcc_metrics.jurisdiction, "Singapore");
        assert_eq!(result.cayman_spc_metrics.jurisdiction, "Cayman Islands");
        assert_eq!(result.vcc_metrics.corporate_tax_rate, dec!(0.17));
        assert_eq!(result.cayman_spc_metrics.corporate_tax_rate, Decimal::ZERO);
    }

    // ------------------------------------------------------------------
    // 48. VCC vs SPC — cost differential
    // ------------------------------------------------------------------
    #[test]
    fn test_cost_differential() {
        let input = comparison_input();
        let result = vcc_vs_cayman_spc(&input).unwrap();

        // VCC should be more expensive
        assert!(result.cost_differential_setup > Decimal::ZERO);
        assert!(result.cost_differential_annual > Decimal::ZERO);
    }

    // ------------------------------------------------------------------
    // 49. VCC vs SPC — FATCA/CRS compliance
    // ------------------------------------------------------------------
    #[test]
    fn test_both_fatca_crs_compliant() {
        let input = comparison_input();
        let result = vcc_vs_cayman_spc(&input).unwrap();

        assert!(result.vcc_metrics.fatca_crs_compliant);
        assert!(result.cayman_spc_metrics.fatca_crs_compliant);
    }

    // ------------------------------------------------------------------
    // 50. VCC vs SPC — comparison notes populated
    // ------------------------------------------------------------------
    #[test]
    fn test_comparison_notes_populated() {
        let input = comparison_input();
        let result = vcc_vs_cayman_spc(&input).unwrap();

        assert!(!result.comparison_notes.is_empty());
        assert!(result.comparison_notes.len() >= 5);
    }

    // ------------------------------------------------------------------
    // 51. VCC vs SPC — Asia-focused recommendation
    // ------------------------------------------------------------------
    #[test]
    fn test_asia_focused_recommendation() {
        let mut input = comparison_input();
        input.target_investors = vec!["AsiaInstitutional".to_string()];
        let result = vcc_vs_cayman_spc(&input).unwrap();

        assert!(result.recommendation.contains("Singapore VCC"));
    }

    // ------------------------------------------------------------------
    // 52. VCC vs SPC — US-focused large fund recommendation
    // ------------------------------------------------------------------
    #[test]
    fn test_us_focused_recommendation() {
        let mut input = comparison_input();
        input.fund_size = dec!(500_000_000);
        input.target_investors = vec!["USInstitutional".to_string()];
        input.investment_professionals_sg = 0;
        let result = vcc_vs_cayman_spc(&input).unwrap();

        assert!(result.recommendation.contains("Cayman SPC"));
    }

    // ------------------------------------------------------------------
    // 53. VCC vs SPC — substance scores
    // ------------------------------------------------------------------
    #[test]
    fn test_substance_scores_comparison() {
        let input = comparison_input();
        let result = vcc_vs_cayman_spc(&input).unwrap();

        // VCC with IPs and spending should have higher substance
        assert!(
            result.vcc_metrics.substance_score >= result.cayman_spc_metrics.substance_score,
            "VCC substance {} should be >= Cayman SPC substance {}",
            result.vcc_metrics.substance_score,
            result.cayman_spc_metrics.substance_score
        );
    }

    // ------------------------------------------------------------------
    // 54. Comparison validation — empty fund name
    // ------------------------------------------------------------------
    #[test]
    fn test_comparison_validation_empty_name() {
        let mut input = comparison_input();
        input.fund_name = "".to_string();
        let result = vcc_vs_cayman_spc(&input);
        assert!(result.is_err());
    }

    // ------------------------------------------------------------------
    // 55. Comparison validation — zero fund size
    // ------------------------------------------------------------------
    #[test]
    fn test_comparison_validation_zero_size() {
        let mut input = comparison_input();
        input.fund_size = Decimal::ZERO;
        let result = vcc_vs_cayman_spc(&input);
        assert!(result.is_err());
    }

    // ------------------------------------------------------------------
    // 56. Umbrella VCC with < 2 sub-funds generates warning
    // ------------------------------------------------------------------
    #[test]
    fn test_umbrella_single_sub_fund_warning() {
        let mut input = umbrella_vcc_input();
        input.sub_funds = vec![SubFundInfo {
            name: "Only Fund".to_string(),
            strategy: "Equity".to_string(),
            target_aum: dec!(100_000_000),
            currency: "SGD".to_string(),
        }];
        let result = analyze_vcc_structure(&input).unwrap();

        assert!(result
            .warnings
            .iter()
            .any(|w| w.contains("fewer than 2 sub-funds")));
    }

    // ------------------------------------------------------------------
    // 57. Sub-fund AUM mismatch > 1% triggers validation error
    // ------------------------------------------------------------------
    #[test]
    fn test_sub_fund_aum_mismatch() {
        let mut input = umbrella_vcc_input();
        // Change sub-fund AUMs to sum to 120M but fund_size is 100M
        input.sub_funds[0].target_aum = dec!(70_000_000);
        // Total: 70M + 30M + 20M = 120M vs 100M → 20% off
        let result = analyze_vcc_structure(&input);
        assert!(result.is_err());
    }

    // ------------------------------------------------------------------
    // 58. Sub-fund AUM within 1% tolerance is OK
    // ------------------------------------------------------------------
    #[test]
    fn test_sub_fund_aum_within_tolerance() {
        let mut input = umbrella_vcc_input();
        // Sum = 50.5M + 30M + 20M = 100.5M vs 100M → 0.5%
        input.sub_funds[0].target_aum = dec!(50_500_000);
        input.fund_size = dec!(100_500_000);
        let result = analyze_vcc_structure(&input);
        assert!(result.is_ok());
    }

    // ------------------------------------------------------------------
    // 59. S13D no AUM minimum
    // ------------------------------------------------------------------
    #[test]
    fn test_s13d_no_aum_minimum() {
        let mut input = tax_incentive_s13o_input();
        input.scheme = "S13D".to_string();
        input.fund_size = dec!(1_000_000); // 1M — very small
        input.is_resident = false;
        let result = tax_incentive_analysis(&input).unwrap();

        assert!(result.scheme_eligibility.eligible);
    }

    // ------------------------------------------------------------------
    // 60. Tax incentive recommendation text for eligible
    // ------------------------------------------------------------------
    #[test]
    fn test_tax_recommendation_eligible_text() {
        let input = tax_incentive_s13o_input();
        let result = tax_incentive_analysis(&input).unwrap();

        assert!(result.recommendation.contains("eligible"));
        assert!(result.recommendation.contains("850000"));
    }

    // ------------------------------------------------------------------
    // 61. Tax incentive recommendation text for ineligible
    // ------------------------------------------------------------------
    #[test]
    fn test_tax_recommendation_ineligible_text() {
        let mut input = tax_incentive_s13o_input();
        input.fund_size = dec!(5_000_000);
        let result = tax_incentive_analysis(&input).unwrap();

        assert!(result.recommendation.contains("not eligible"));
    }

    // ------------------------------------------------------------------
    // 62. VCC regulatory timeline
    // ------------------------------------------------------------------
    #[test]
    fn test_regulatory_timeline() {
        let input = comparison_input();
        let result = vcc_vs_cayman_spc(&input).unwrap();

        assert_eq!(result.vcc_metrics.regulatory_timeline_weeks, 8);
        assert_eq!(result.cayman_spc_metrics.regulatory_timeline_weeks, 4);
    }
}
