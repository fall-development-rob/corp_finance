use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

use crate::error::CorpFinanceError;
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Types — OFC
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OfcInput {
    pub fund_name: String,
    /// "Public" (SFC-authorized, retail) or "Private" (professional investors only)
    pub ofc_type: String,
    pub umbrella: bool,
    pub sub_fund_count: u32,
    pub fund_strategy: String,
    pub fund_size: Decimal,
    pub management_fee_rate: Decimal,
    pub performance_fee_rate: Decimal,
    /// SFC Type 9 licensed investment manager required
    pub type9_licensed_manager: bool,
    /// Eligible for OFC grant scheme (up to HKD 1M, 70% of eligible expenses)
    pub grant_scheme_eligible: bool,
    pub target_investors: Vec<String>,
}

// ---------------------------------------------------------------------------
// Types — LPF
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LpfInput {
    pub fund_name: String,
    /// "PE", "VC", "RealEstate", "Infrastructure", "Credit"
    pub fund_purpose: String,
    pub fund_size: Decimal,
    pub gp_jurisdiction: String,
    pub management_fee_rate: Decimal,
    pub carried_interest_rate: Decimal,
    pub fund_term_years: u32,
    /// Responsible person (investment manager or custodian)
    pub responsible_person: String,
    pub audit_waiver: bool,
    pub target_investors: Vec<String>,
}

// ---------------------------------------------------------------------------
// Types — Carried Interest
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CarriedInterestInput {
    /// "OFC", "LPF", "UnitTrust"
    pub fund_type: String,
    pub carried_interest_amount: Decimal,
    /// Certified investment fund by HKMA
    pub fund_certified: bool,
    pub avg_holding_period_months: u32,
    pub arms_length_terms: bool,
    /// Minimum 2 full-time HK employees for concession
    pub hk_employees: u32,
    /// Percentage of qualifying transactions (0-1)
    pub qualifying_transactions_pct: Decimal,
}

// ---------------------------------------------------------------------------
// Types — HK vs Singapore Comparison
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HkSgCompInput {
    pub fund_size: Decimal,
    /// "PE", "VC", "Hedge", "RealEstate", "Infrastructure", "Credit"
    pub fund_strategy: String,
    pub target_market: String,
    pub management_fee_rate: Decimal,
    pub performance_fee_rate: Decimal,
    pub carried_interest_amount: Decimal,
}

// ---------------------------------------------------------------------------
// Output types — OFC
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OfcStructureAnalysis {
    pub ofc_type: String,
    pub description: String,
    pub investor_eligibility: String,
    pub umbrella_structure: bool,
    pub sub_fund_count: u32,
    pub suitable_strategies: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OfcRegulatoryAnalysis {
    pub sfc_authorization_required: bool,
    pub type9_license_required: bool,
    pub type9_license_held: bool,
    pub custodian_required: bool,
    pub audit_required: bool,
    pub reporting_frequency: String,
    pub requirements: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrantSchemeAnalysis {
    pub eligible: bool,
    pub max_grant_hkd: Decimal,
    pub reimbursement_rate: Decimal,
    pub eligible_expenses: Vec<String>,
    pub estimated_grant: Decimal,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OfcCostAnalysis {
    pub setup_cost_low: Decimal,
    pub setup_cost_high: Decimal,
    pub annual_cost_low: Decimal,
    pub annual_cost_high: Decimal,
    pub annual_management_fee: Decimal,
    pub annual_performance_fee: Decimal,
    pub cost_pct_of_aum: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OfcSubstanceAnalysis {
    pub substance_score: u32,
    pub type9_manager_in_hk: bool,
    pub local_directors_recommended: u32,
    pub board_meetings_in_hk: u32,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OfcOutput {
    pub fund_name: String,
    pub structure_analysis: OfcStructureAnalysis,
    pub regulatory: OfcRegulatoryAnalysis,
    pub grant_scheme_analysis: GrantSchemeAnalysis,
    pub cost_analysis: OfcCostAnalysis,
    pub substance_analysis: OfcSubstanceAnalysis,
    pub recommendations: Vec<String>,
    pub warnings: Vec<String>,
}

// ---------------------------------------------------------------------------
// Output types — LPF
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LpfStructureAnalysis {
    pub fund_purpose: String,
    pub description: String,
    pub ordinance: String,
    pub eligible_purpose: bool,
    pub gp_liability: String,
    pub lp_liability: String,
    pub suitable_purposes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpAnalysis {
    pub gp_jurisdiction: String,
    pub gp_unlimited_liability: bool,
    pub responsible_person: String,
    pub responsible_person_requirement: String,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LpfRegulatoryAnalysis {
    pub registration_body: String,
    pub sfc_authorization_required: bool,
    pub audit_required: bool,
    pub audit_waiver_available: bool,
    pub annual_return_required: bool,
    pub requirements: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LpfCostAnalysis {
    pub setup_cost_low: Decimal,
    pub setup_cost_high: Decimal,
    pub annual_cost_low: Decimal,
    pub annual_cost_high: Decimal,
    pub annual_management_fee: Decimal,
    pub annual_carried_interest: Decimal,
    pub cost_pct_of_aum: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaymanLpComparison {
    pub dimension: String,
    pub hk_lpf: String,
    pub cayman_elp: String,
    pub advantage: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LpfOutput {
    pub fund_name: String,
    pub structure_analysis: LpfStructureAnalysis,
    pub gp_analysis: GpAnalysis,
    pub regulatory: LpfRegulatoryAnalysis,
    pub cost_analysis: LpfCostAnalysis,
    pub comparison_to_cayman_lp: Vec<CaymanLpComparison>,
    pub recommendations: Vec<String>,
    pub warnings: Vec<String>,
}

// ---------------------------------------------------------------------------
// Output types — Carried Interest
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CarryCondition {
    pub condition: String,
    pub met: bool,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JurisdictionComparison {
    pub jurisdiction: String,
    pub carry_tax_rate: Decimal,
    pub tax_on_carry: Decimal,
    pub notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CarriedInterestOutput {
    pub eligible: bool,
    pub conditions: Vec<CarryCondition>,
    pub effective_tax_rate: Decimal,
    pub tax_savings: Decimal,
    pub carried_interest_amount: Decimal,
    pub tax_payable: Decimal,
    pub comparison_to_other_jurisdictions: Vec<JurisdictionComparison>,
    pub recommendations: Vec<String>,
    pub warnings: Vec<String>,
}

// ---------------------------------------------------------------------------
// Output types — HK vs Singapore
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparisonDimension {
    pub dimension: String,
    pub hk_ofc: String,
    pub hk_lpf: String,
    pub sg_vcc: String,
    pub best: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HkSgCompOutput {
    pub comparison_matrix: Vec<ComparisonDimension>,
    pub hk_ofc_score: u32,
    pub hk_lpf_score: u32,
    pub sg_vcc_score: u32,
    pub recommendation: String,
    pub strategy_specific_notes: Vec<String>,
    pub warnings: Vec<String>,
}

// ---------------------------------------------------------------------------
// Public API — OFC
// ---------------------------------------------------------------------------

pub fn analyze_ofc_structure(input: &OfcInput) -> CorpFinanceResult<OfcOutput> {
    validate_ofc_input(input)?;

    let mut warnings: Vec<String> = Vec::new();
    let mut recommendations: Vec<String> = Vec::new();

    // ------------------------------------------------------------------
    // 1. Structure Analysis
    // ------------------------------------------------------------------
    let is_public = input.ofc_type == "Public";

    let investor_eligibility = if is_public {
        "Retail and professional investors (SFC-authorized)".to_string()
    } else {
        "Professional investors only (not SFC-authorized)".to_string()
    };

    let suitable_strategies = vec![
        "Hedge".to_string(),
        "PE".to_string(),
        "VC".to_string(),
        "RealEstate".to_string(),
        "Credit".to_string(),
        "FundOfFunds".to_string(),
    ];

    let description = if input.umbrella {
        format!(
            "{} OFC with umbrella structure ({} sub-fund(s))",
            input.ofc_type, input.sub_fund_count
        )
    } else {
        format!("{} OFC — standalone fund", input.ofc_type)
    };

    let structure_analysis = OfcStructureAnalysis {
        ofc_type: input.ofc_type.clone(),
        description,
        investor_eligibility,
        umbrella_structure: input.umbrella,
        sub_fund_count: if input.umbrella {
            input.sub_fund_count
        } else {
            1
        },
        suitable_strategies,
    };

    if is_public {
        recommendations.push(
            "Public OFC requires SFC authorization — longer setup timeline \
             (3-6 months) and ongoing compliance obligations"
                .to_string(),
        );
    } else {
        recommendations.push(
            "Private OFC does not require SFC authorization but must \
             appoint SFC Type 9 licensed investment manager"
                .to_string(),
        );
    }

    if input.umbrella && input.sub_fund_count > 5 {
        recommendations.push(
            "Large umbrella OFC (>5 sub-funds): consider dedicated compliance \
             officer and enhanced reporting infrastructure"
                .to_string(),
        );
    }

    // ------------------------------------------------------------------
    // 2. Regulatory Analysis
    // ------------------------------------------------------------------
    let mut requirements = Vec::new();
    requirements.push("SFC Type 9 licensed investment manager required".to_string());
    requirements.push("Custodian must be a qualifying custodian".to_string());
    requirements.push("Annual audited financial statements required".to_string());

    if is_public {
        requirements.push("SFC authorization required for public offering".to_string());
        requirements.push("Offering document must be registered with SFC".to_string());
    } else {
        requirements
            .push("No SFC authorization required for professional investors only".to_string());
    }

    if input.umbrella {
        requirements.push(format!(
            "Each sub-fund ({}) must maintain separate accounts and records",
            input.sub_fund_count
        ));
    }

    if !input.type9_licensed_manager {
        warnings.push(
            "OFC structure requires a Type 9 licensed investment manager — \
             this condition is not met"
                .to_string(),
        );
    }

    let regulatory = OfcRegulatoryAnalysis {
        sfc_authorization_required: is_public,
        type9_license_required: true,
        type9_license_held: input.type9_licensed_manager,
        custodian_required: true,
        audit_required: true,
        reporting_frequency: if is_public {
            "Semi-annual".to_string()
        } else {
            "Annual".to_string()
        },
        requirements,
    };

    // ------------------------------------------------------------------
    // 3. Grant Scheme Analysis
    // ------------------------------------------------------------------
    let max_grant = dec!(1_000_000);
    let reimbursement_rate = dec!(0.70);

    let eligible_expenses = vec![
        "Legal fees for fund setup".to_string(),
        "Audit and accounting fees".to_string(),
        "Regulatory filing fees".to_string(),
        "Custodian setup fees".to_string(),
        "Tax advisory fees".to_string(),
    ];

    let (estimated_grant, grant_notes) = if input.grant_scheme_eligible {
        // Estimate eligible setup costs based on OFC type
        let estimated_eligible_costs = if is_public {
            dec!(1_200_000)
        } else {
            dec!(800_000)
        };
        let raw_grant = estimated_eligible_costs * reimbursement_rate;
        let capped_grant = raw_grant.min(max_grant);
        let mut notes = vec![format!(
            "Estimated eligible expenses: HKD {}, 70% reimbursement = HKD {}, capped at HKD {}",
            estimated_eligible_costs, raw_grant, capped_grant
        )];
        notes.push("Grant application to InvestHK within 2 years of fund launch".to_string());
        (capped_grant, notes)
    } else {
        let notes = vec![
            "Fund not marked as eligible for OFC grant scheme".to_string(),
            "Check eligibility: must be a new OFC registered in Hong Kong".to_string(),
        ];
        (Decimal::ZERO, notes)
    };

    let grant_scheme_analysis = GrantSchemeAnalysis {
        eligible: input.grant_scheme_eligible,
        max_grant_hkd: max_grant,
        reimbursement_rate,
        eligible_expenses,
        estimated_grant,
        notes: grant_notes,
    };

    // ------------------------------------------------------------------
    // 4. Cost Analysis
    // ------------------------------------------------------------------
    let (setup_low, setup_high) = if is_public {
        (dec!(150_000), dec!(250_000))
    } else {
        (dec!(100_000), dec!(200_000))
    };

    let sub_fund_multiplier = if input.umbrella {
        Decimal::from(input.sub_fund_count.max(1))
    } else {
        Decimal::ONE
    };

    let (annual_low, annual_high) = if is_public {
        (
            dec!(300_000) + (sub_fund_multiplier - Decimal::ONE) * dec!(50_000),
            dec!(500_000) + (sub_fund_multiplier - Decimal::ONE) * dec!(80_000),
        )
    } else {
        (
            dec!(200_000) + (sub_fund_multiplier - Decimal::ONE) * dec!(40_000),
            dec!(400_000) + (sub_fund_multiplier - Decimal::ONE) * dec!(60_000),
        )
    };

    let annual_mgmt_fee = input.fund_size * input.management_fee_rate;
    let annual_perf_fee = input.fund_size * input.performance_fee_rate;

    let mid_annual_cost = (annual_low + annual_high) / dec!(2);
    let cost_pct = if input.fund_size > Decimal::ZERO {
        mid_annual_cost / input.fund_size
    } else {
        Decimal::ZERO
    };

    let cost_analysis = OfcCostAnalysis {
        setup_cost_low: setup_low,
        setup_cost_high: setup_high,
        annual_cost_low: annual_low,
        annual_cost_high: annual_high,
        annual_management_fee: annual_mgmt_fee,
        annual_performance_fee: annual_perf_fee,
        cost_pct_of_aum: cost_pct,
    };

    if cost_pct > dec!(0.005) {
        warnings.push(format!(
            "Mid-range annual cost is {:.2}% of AUM, above the typical 0.50% threshold",
            cost_pct * dec!(100)
        ));
    }

    // ------------------------------------------------------------------
    // 5. Substance Analysis
    // ------------------------------------------------------------------
    let mut substance_score: u32 = 0;
    let mut substance_recs: Vec<String> = Vec::new();

    if input.type9_licensed_manager {
        substance_score += 3;
    } else {
        substance_recs.push("Appoint SFC Type 9 licensed investment manager in HK".to_string());
    }

    // Fund size substance expectations
    if input.fund_size >= dec!(500_000_000) {
        substance_score += 2;
        substance_recs.push(
            "Large fund (>HKD 500M): ensure adequate local staff and \
             decision-making in Hong Kong"
                .to_string(),
        );
    } else if input.fund_size >= dec!(100_000_000) {
        substance_score += 1;
    }

    // Public OFC has higher substance by default (SFC oversight)
    if is_public {
        substance_score += 2;
    } else {
        substance_score += 1;
    }

    // Board meetings
    let board_meetings = if is_public { 4 } else { 2 };
    substance_score += if board_meetings >= 4 { 2 } else { 1 };

    let substance_analysis = OfcSubstanceAnalysis {
        substance_score: substance_score.min(10),
        type9_manager_in_hk: input.type9_licensed_manager,
        local_directors_recommended: 2,
        board_meetings_in_hk: board_meetings,
        recommendations: substance_recs,
    };

    // ------------------------------------------------------------------
    // 6. Strategy-specific recommendations
    // ------------------------------------------------------------------
    match input.fund_strategy.as_str() {
        "Hedge" => {
            recommendations.push(
                "OFC is well-suited for hedge fund strategies with \
                 unified fund entity regime (UFE) tax exemption"
                    .to_string(),
            );
        }
        "PE" | "VC" => {
            recommendations.push(
                "Consider LPF structure as alternative for PE/VC — \
                 simpler setup via Companies Registry, lower cost"
                    .to_string(),
            );
        }
        "RealEstate" => {
            recommendations.push(
                "OFC for real estate: leverage Stock Connect/Bond Connect \
                 for Greater China exposure"
                    .to_string(),
            );
        }
        _ => {}
    }

    Ok(OfcOutput {
        fund_name: input.fund_name.clone(),
        structure_analysis,
        regulatory,
        grant_scheme_analysis,
        cost_analysis,
        substance_analysis,
        recommendations,
        warnings,
    })
}

// ---------------------------------------------------------------------------
// Public API — LPF
// ---------------------------------------------------------------------------

pub fn analyze_lpf_structure(input: &LpfInput) -> CorpFinanceResult<LpfOutput> {
    validate_lpf_input(input)?;

    let mut warnings: Vec<String> = Vec::new();
    let mut recommendations: Vec<String> = Vec::new();

    // ------------------------------------------------------------------
    // 1. Structure Analysis
    // ------------------------------------------------------------------
    let eligible_purposes = vec![
        "PE".to_string(),
        "VC".to_string(),
        "RealEstate".to_string(),
        "Infrastructure".to_string(),
        "Credit".to_string(),
    ];

    let eligible_purpose = eligible_purposes.contains(&input.fund_purpose);

    if !eligible_purpose {
        warnings.push(format!(
            "Fund purpose '{}' may not qualify as an eligible purpose \
             under the LPF Ordinance (Cap 637)",
            input.fund_purpose
        ));
    }

    let description = format!(
        "Limited Partnership Fund for {} — {}-year term, GP in {}",
        input.fund_purpose, input.fund_term_years, input.gp_jurisdiction
    );

    let structure_analysis = LpfStructureAnalysis {
        fund_purpose: input.fund_purpose.clone(),
        description,
        ordinance: "Limited Partnership Fund Ordinance (Cap 637)".to_string(),
        eligible_purpose,
        gp_liability: "Unlimited liability".to_string(),
        lp_liability: "Limited to capital contribution (safe harbour provisions)".to_string(),
        suitable_purposes: eligible_purposes,
    };

    // ------------------------------------------------------------------
    // 2. GP Analysis
    // ------------------------------------------------------------------
    let mut gp_recs = Vec::new();

    let rp_requirement = if input.responsible_person.is_empty() {
        warnings.push(
            "Responsible person must be appointed — SFC Type 9 licensee \
             or authorized institution"
                .to_string(),
        );
        "Not appointed — REQUIRED".to_string()
    } else {
        format!("Appointed: {}", input.responsible_person)
    };

    if input.gp_jurisdiction != "HongKong" && input.gp_jurisdiction != "Hong Kong" {
        gp_recs.push(format!(
            "GP is incorporated in {} — ensure compliance with HK \
             substance requirements and responsible person appointment",
            input.gp_jurisdiction
        ));
    } else {
        gp_recs.push("HK-incorporated GP provides strong substance position".to_string());
    }

    let gp_analysis = GpAnalysis {
        gp_jurisdiction: input.gp_jurisdiction.clone(),
        gp_unlimited_liability: true,
        responsible_person: input.responsible_person.clone(),
        responsible_person_requirement: rp_requirement,
        recommendations: gp_recs,
    };

    // ------------------------------------------------------------------
    // 3. Regulatory Analysis
    // ------------------------------------------------------------------
    let mut requirements = vec![
        "Registration with Companies Registry required".to_string(),
        "Responsible person (Type 9 licensee or authorized institution)".to_string(),
        "Registered office in Hong Kong required".to_string(),
        "Must maintain register of partners".to_string(),
    ];

    let audit_waiver_available = input.fund_size < dec!(100_000_000);
    let audit_required = if input.audit_waiver && audit_waiver_available {
        requirements.push("Audit waiver granted (fund size < HKD 100M)".to_string());
        false
    } else {
        if input.audit_waiver && !audit_waiver_available {
            warnings.push("Audit waiver not available for funds >= HKD 100M".to_string());
        }
        requirements.push("Annual audited financial statements required".to_string());
        true
    };

    let regulatory = LpfRegulatoryAnalysis {
        registration_body: "Companies Registry (Hong Kong)".to_string(),
        sfc_authorization_required: false,
        audit_required,
        audit_waiver_available,
        annual_return_required: true,
        requirements,
    };

    // ------------------------------------------------------------------
    // 4. Cost Analysis
    // ------------------------------------------------------------------
    let (setup_low, setup_high) = (dec!(50_000), dec!(150_000));
    let (annual_low, annual_high) = (dec!(100_000), dec!(300_000));

    let annual_mgmt_fee = input.fund_size * input.management_fee_rate;
    let annual_carry = input.fund_size * input.carried_interest_rate;

    let mid_annual_cost = (annual_low + annual_high) / dec!(2);
    let cost_pct = if input.fund_size > Decimal::ZERO {
        mid_annual_cost / input.fund_size
    } else {
        Decimal::ZERO
    };

    let cost_analysis = LpfCostAnalysis {
        setup_cost_low: setup_low,
        setup_cost_high: setup_high,
        annual_cost_low: annual_low,
        annual_cost_high: annual_high,
        annual_management_fee: annual_mgmt_fee,
        annual_carried_interest: annual_carry,
        cost_pct_of_aum: cost_pct,
    };

    if cost_pct > dec!(0.005) {
        warnings.push(format!(
            "Mid-range annual cost is {:.2}% of AUM, above 0.50% threshold",
            cost_pct * dec!(100)
        ));
    }

    // ------------------------------------------------------------------
    // 5. Comparison to Cayman LP
    // ------------------------------------------------------------------
    let comparison_to_cayman_lp = build_cayman_lp_comparison(input);

    // ------------------------------------------------------------------
    // 6. Recommendations
    // ------------------------------------------------------------------
    recommendations.push(
        "LPF provides cost-effective alternative to Cayman Exempted LP \
         with HK substance benefits"
            .to_string(),
    );

    if matches!(input.fund_purpose.as_str(), "PE" | "VC") {
        recommendations.push(
            "Consider applying for carried interest tax concession \
             (0% profits tax on qualifying carry)"
                .to_string(),
        );
    }

    if input.fund_term_years > 12 {
        recommendations.push(
            "Fund term exceeds typical 10+2 structure — ensure LP \
             agreement provides for extension mechanics"
                .to_string(),
        );
    }

    match input.fund_purpose.as_str() {
        "RealEstate" | "Infrastructure" => {
            recommendations.push(
                "Greater China real asset strategies benefit from HK's \
                 proximity and Stock/Bond Connect access"
                    .to_string(),
            );
        }
        _ => {}
    }

    Ok(LpfOutput {
        fund_name: input.fund_name.clone(),
        structure_analysis,
        gp_analysis,
        regulatory,
        cost_analysis,
        comparison_to_cayman_lp,
        recommendations,
        warnings,
    })
}

// ---------------------------------------------------------------------------
// Public API — Carried Interest Concession
// ---------------------------------------------------------------------------

pub fn carried_interest_concession(
    input: &CarriedInterestInput,
) -> CorpFinanceResult<CarriedInterestOutput> {
    validate_carry_input(input)?;

    let mut warnings: Vec<String> = Vec::new();
    let mut recommendations: Vec<String> = Vec::new();

    let standard_rate = dec!(0.165);

    // ------------------------------------------------------------------
    // 1. Eligibility Conditions
    // ------------------------------------------------------------------
    let cond_certified = CarryCondition {
        condition: "Fund certified by HKMA as qualifying investment fund".to_string(),
        met: input.fund_certified,
        detail: if input.fund_certified {
            "Fund is HKMA-certified".to_string()
        } else {
            "Fund must obtain HKMA certification to qualify".to_string()
        },
    };

    let cond_holding = CarryCondition {
        condition: "Average holding period >= 24 months".to_string(),
        met: input.avg_holding_period_months >= 24,
        detail: format!(
            "Average holding period: {} months (minimum 24 required)",
            input.avg_holding_period_months
        ),
    };

    let cond_arms_length = CarryCondition {
        condition: "Carried interest on arm's length terms".to_string(),
        met: input.arms_length_terms,
        detail: if input.arms_length_terms {
            "Arm's length terms confirmed".to_string()
        } else {
            "Must demonstrate arm's length terms for concession".to_string()
        },
    };

    let cond_employees = CarryCondition {
        condition: "Minimum 2 full-time HK employees providing investment management".to_string(),
        met: input.hk_employees >= 2,
        detail: format!("HK employees: {} (minimum 2 required)", input.hk_employees),
    };

    let cond_qualifying_txn = CarryCondition {
        condition: "Qualifying transactions percentage".to_string(),
        met: input.qualifying_transactions_pct >= dec!(0.50),
        detail: format!(
            "Qualifying transactions: {:.0}% (recommend >= 50%)",
            input.qualifying_transactions_pct * dec!(100)
        ),
    };

    let conditions = vec![
        cond_certified,
        cond_holding,
        cond_arms_length,
        cond_employees,
        cond_qualifying_txn,
    ];

    // All core conditions must be met (certified, holding, arms_length, employees)
    let eligible = input.fund_certified
        && input.avg_holding_period_months >= 24
        && input.arms_length_terms
        && input.hk_employees >= 2;

    let effective_tax_rate = if eligible {
        Decimal::ZERO
    } else {
        standard_rate
    };

    let tax_payable = input.carried_interest_amount * effective_tax_rate;
    let tax_savings = if eligible {
        input.carried_interest_amount * standard_rate
    } else {
        Decimal::ZERO
    };

    // ------------------------------------------------------------------
    // 2. Jurisdiction Comparison
    // ------------------------------------------------------------------
    let carry_amount = input.carried_interest_amount;

    let comparison_to_other_jurisdictions = vec![
        JurisdictionComparison {
            jurisdiction: "Hong Kong (with concession)".to_string(),
            carry_tax_rate: Decimal::ZERO,
            tax_on_carry: Decimal::ZERO,
            notes: "0% profits tax on qualifying carried interest".to_string(),
        },
        JurisdictionComparison {
            jurisdiction: "Hong Kong (without concession)".to_string(),
            carry_tax_rate: dec!(0.165),
            tax_on_carry: carry_amount * dec!(0.165),
            notes: "Standard 16.5% profits tax".to_string(),
        },
        JurisdictionComparison {
            jurisdiction: "Singapore".to_string(),
            carry_tax_rate: dec!(0.10),
            tax_on_carry: carry_amount * dec!(0.10),
            notes: "10% concessionary rate under S13O/U/D incentives".to_string(),
        },
        JurisdictionComparison {
            jurisdiction: "Cayman Islands".to_string(),
            carry_tax_rate: Decimal::ZERO,
            tax_on_carry: Decimal::ZERO,
            notes: "No income tax, capital gains tax, or withholding tax".to_string(),
        },
        JurisdictionComparison {
            jurisdiction: "United Kingdom".to_string(),
            carry_tax_rate: dec!(0.28),
            tax_on_carry: carry_amount * dec!(0.28),
            notes: "28% CGT on carried interest".to_string(),
        },
        JurisdictionComparison {
            jurisdiction: "United States".to_string(),
            carry_tax_rate: dec!(0.238),
            tax_on_carry: carry_amount * dec!(0.238),
            notes: "20% LTCG + 3.8% NIIT if held > 3 years".to_string(),
        },
    ];

    // ------------------------------------------------------------------
    // 3. Recommendations
    // ------------------------------------------------------------------
    if !eligible {
        let failed: Vec<String> = conditions
            .iter()
            .filter(|c| !c.met)
            .map(|c| c.condition.clone())
            .collect();
        warnings.push(format!(
            "Carried interest concession NOT available — failed conditions: {}",
            failed.join("; ")
        ));
        recommendations
            .push("Address failed conditions to qualify for 0% tax concession".to_string());
    } else {
        recommendations.push(format!(
            "Qualifying carried interest concession saves HKD {} in profits tax",
            tax_savings
        ));
    }

    if input.avg_holding_period_months >= 24 && input.avg_holding_period_months < 30 {
        recommendations.push(
            "Holding period is close to the 24-month minimum — consider \
             extending to provide buffer against disqualification"
                .to_string(),
        );
    }

    if input.hk_employees == 2 {
        recommendations.push(
            "At minimum employee threshold (2) — consider additional HK \
             staff for operational resilience and substance"
                .to_string(),
        );
    }

    Ok(CarriedInterestOutput {
        eligible,
        conditions,
        effective_tax_rate,
        tax_savings,
        carried_interest_amount: input.carried_interest_amount,
        tax_payable,
        comparison_to_other_jurisdictions,
        recommendations,
        warnings,
    })
}

// ---------------------------------------------------------------------------
// Public API — HK vs Singapore Comparison
// ---------------------------------------------------------------------------

pub fn hk_vs_singapore(input: &HkSgCompInput) -> CorpFinanceResult<HkSgCompOutput> {
    validate_hk_sg_input(input)?;

    let mut warnings: Vec<String> = Vec::new();

    let is_closed_end = matches!(
        input.fund_strategy.as_str(),
        "PE" | "VC" | "RealEstate" | "Infrastructure" | "Credit"
    );

    // ------------------------------------------------------------------
    // 1. Build Comparison Matrix
    // ------------------------------------------------------------------

    // Setup cost
    let (ofc_setup, lpf_setup, vcc_setup) = if is_closed_end {
        ("HKD 100-200K", "HKD 50-150K", "SGD 50-150K")
    } else {
        ("HKD 150-250K", "HKD 80-180K", "SGD 80-200K")
    };

    // Annual cost
    let (ofc_annual, lpf_annual, vcc_annual) = if is_closed_end {
        ("HKD 200-400K", "HKD 100-300K", "SGD 100-300K")
    } else {
        ("HKD 300-500K", "HKD 150-350K", "SGD 150-400K")
    };

    // Tax treatment
    let (ofc_tax, lpf_tax, vcc_tax) = (
        "UFE: 0% on qualifying profits; carry concession 0%",
        "UFE: 0% on qualifying profits; carry concession 0%",
        "S13O/U/D: 0% on specified income; 10% concessionary carry",
    );

    // Substance
    let (ofc_sub, lpf_sub, vcc_sub) = (
        "Type 9 SFC license; 2+ HK employees for carry concession",
        "Responsible person (Type 9); Companies Registry filing",
        "MAS licensed fund manager; Singapore-based directors",
    );

    // Greater China access
    let (ofc_cn, lpf_cn, vcc_cn) = (
        "Strong: Stock Connect, Bond Connect, QFII/RQFII",
        "Strong: Stock Connect, Bond Connect, QFII/RQFII",
        "Moderate: QFII/RQFII only, no direct Connect access",
    );

    // ASEAN access
    let (ofc_asean, lpf_asean, vcc_asean) = (
        "Limited: no ASEAN passport",
        "Limited: no ASEAN passport",
        "Strong: ASEAN CIS Framework, regional hub",
    );

    // Regulatory burden
    let (ofc_reg, lpf_reg, vcc_reg) = (
        "Medium-High: SFC oversight, Type 9 license",
        "Low-Medium: Companies Registry only, no SFC authorization",
        "Medium: MAS oversight, VCC Act compliance",
    );

    let comparison_matrix = vec![
        ComparisonDimension {
            dimension: "Setup Cost".to_string(),
            hk_ofc: ofc_setup.to_string(),
            hk_lpf: lpf_setup.to_string(),
            sg_vcc: vcc_setup.to_string(),
            best: if is_closed_end {
                "HK LPF / SG VCC (comparable)".to_string()
            } else {
                "SG VCC".to_string()
            },
        },
        ComparisonDimension {
            dimension: "Annual Cost".to_string(),
            hk_ofc: ofc_annual.to_string(),
            hk_lpf: lpf_annual.to_string(),
            sg_vcc: vcc_annual.to_string(),
            best: if is_closed_end {
                "HK LPF".to_string()
            } else {
                "SG VCC".to_string()
            },
        },
        ComparisonDimension {
            dimension: "Tax Treatment".to_string(),
            hk_ofc: ofc_tax.to_string(),
            hk_lpf: lpf_tax.to_string(),
            sg_vcc: vcc_tax.to_string(),
            best: "HK OFC / HK LPF (0% carry vs SG 10%)".to_string(),
        },
        ComparisonDimension {
            dimension: "Substance Requirements".to_string(),
            hk_ofc: ofc_sub.to_string(),
            hk_lpf: lpf_sub.to_string(),
            sg_vcc: vcc_sub.to_string(),
            best: "HK LPF (lightest)".to_string(),
        },
        ComparisonDimension {
            dimension: "Greater China Access".to_string(),
            hk_ofc: ofc_cn.to_string(),
            hk_lpf: lpf_cn.to_string(),
            sg_vcc: vcc_cn.to_string(),
            best: "HK OFC / HK LPF (Stock/Bond Connect)".to_string(),
        },
        ComparisonDimension {
            dimension: "ASEAN Access".to_string(),
            hk_ofc: ofc_asean.to_string(),
            hk_lpf: lpf_asean.to_string(),
            sg_vcc: vcc_asean.to_string(),
            best: "SG VCC (ASEAN CIS Framework)".to_string(),
        },
        ComparisonDimension {
            dimension: "Regulatory Burden".to_string(),
            hk_ofc: ofc_reg.to_string(),
            hk_lpf: lpf_reg.to_string(),
            sg_vcc: vcc_reg.to_string(),
            best: "HK LPF (lightest regulatory)".to_string(),
        },
    ];

    // ------------------------------------------------------------------
    // 2. Score each structure
    // ------------------------------------------------------------------
    let (mut ofc_score, mut lpf_score, mut vcc_score): (u32, u32, u32) = (0, 0, 0);

    // Cost advantage
    if is_closed_end {
        lpf_score += 2;
        vcc_score += 1;
        ofc_score += 0;
    } else {
        vcc_score += 2;
        lpf_score += 1;
        ofc_score += 0;
    }

    // Tax advantage (HK carry concession is 0% vs SG 10%)
    ofc_score += 2;
    lpf_score += 2;
    vcc_score += 1;

    // Greater China access
    ofc_score += 2;
    lpf_score += 2;
    vcc_score += 1;

    // ASEAN access
    vcc_score += 2;
    ofc_score += 0;
    lpf_score += 0;

    // Regulatory simplicity
    lpf_score += 2;
    vcc_score += 1;
    ofc_score += 1;

    // Strategy-specific adjustments
    let mut strategy_notes = Vec::new();
    match input.fund_strategy.as_str() {
        "PE" | "VC" => {
            lpf_score += 2;
            strategy_notes.push(
                "PE/VC: HK LPF is purpose-built for closed-end PE/VC, \
                 with carried interest concession providing 0% tax"
                    .to_string(),
            );
            strategy_notes
                .push("PE/VC: Greater China deal flow advantages via HK base".to_string());
        }
        "Hedge" => {
            ofc_score += 2;
            vcc_score += 1;
            strategy_notes.push(
                "Hedge: OFC umbrella structure allows multi-strategy \
                 with sub-funds; VCC offers similar flexibility"
                    .to_string(),
            );
            strategy_notes.push(
                "Hedge: Both HK UFE and SG S13O provide fund-level tax exemptions".to_string(),
            );
        }
        "RealEstate" | "Infrastructure" => {
            lpf_score += 1;
            ofc_score += 1;
            strategy_notes.push(
                "Real assets: HK provides superior Greater China market access \
                 via Stock Connect and Bond Connect"
                    .to_string(),
            );
        }
        "Credit" => {
            lpf_score += 1;
            strategy_notes.push(
                "Credit: HK LPF suitable for credit funds; consider \
                 Bond Connect for onshore China fixed income exposure"
                    .to_string(),
            );
        }
        _ => {}
    }

    // Fund size adjustment
    if input.fund_size < dec!(50_000_000) {
        lpf_score += 1;
        strategy_notes.push(
            "Small fund (<HKD 50M): LPF is most cost-effective; \
             audit waiver may be available"
                .to_string(),
        );
    }

    // Carry amount matters
    if input.carried_interest_amount > Decimal::ZERO {
        let hk_savings = input.carried_interest_amount * dec!(0.165);
        let sg_savings = input.carried_interest_amount * dec!(0.065);
        strategy_notes.push(format!(
            "Carry tax savings: HK concession saves HKD {} vs standard rate; \
             HK saves HKD {} more than SG concessionary rate",
            hk_savings,
            hk_savings - sg_savings
        ));
    }

    // ------------------------------------------------------------------
    // 3. Recommendation
    // ------------------------------------------------------------------
    let recommendation = if lpf_score >= ofc_score && lpf_score >= vcc_score {
        format!(
            "Recommended: HK LPF (score {}/{}/{}) — best combination of \
             cost, tax efficiency, and Greater China access for {} strategy",
            lpf_score, ofc_score, vcc_score, input.fund_strategy
        )
    } else if ofc_score >= lpf_score && ofc_score >= vcc_score {
        format!(
            "Recommended: HK OFC (score {}/{}/{}) — best for {} strategy \
             with umbrella flexibility and UFE tax exemption",
            ofc_score, lpf_score, vcc_score, input.fund_strategy
        )
    } else {
        format!(
            "Recommended: SG VCC (score {}/{}/{}) — best for {} strategy \
             with ASEAN access and regulatory framework",
            vcc_score, ofc_score, lpf_score, input.fund_strategy
        )
    };

    if input.fund_strategy == "Hedge" && ofc_score == vcc_score {
        warnings.push(
            "HK OFC and SG VCC are closely matched for hedge strategies — \
             decision may depend on target investor base and market access needs"
                .to_string(),
        );
    }

    Ok(HkSgCompOutput {
        comparison_matrix,
        hk_ofc_score: ofc_score,
        hk_lpf_score: lpf_score,
        sg_vcc_score: vcc_score,
        recommendation,
        strategy_specific_notes: strategy_notes,
        warnings,
    })
}

// ---------------------------------------------------------------------------
// Helpers — Cayman LP Comparison
// ---------------------------------------------------------------------------

fn build_cayman_lp_comparison(input: &LpfInput) -> Vec<CaymanLpComparison> {
    vec![
        CaymanLpComparison {
            dimension: "Legal Framework".to_string(),
            hk_lpf: "LPF Ordinance (Cap 637), 2020".to_string(),
            cayman_elp: "Exempted Limited Partnership Act (as revised)".to_string(),
            advantage: "Cayman (more established, wider market acceptance)".to_string(),
        },
        CaymanLpComparison {
            dimension: "Registration".to_string(),
            hk_lpf: "Companies Registry (HK)".to_string(),
            cayman_elp: "Registrar of Exempted Limited Partnerships (Cayman)".to_string(),
            advantage: "Neutral".to_string(),
        },
        CaymanLpComparison {
            dimension: "Setup Cost".to_string(),
            hk_lpf: "HKD 50-150K".to_string(),
            cayman_elp: "USD 30-80K (HKD 230-620K equiv)".to_string(),
            advantage: "HK LPF (significantly lower)".to_string(),
        },
        CaymanLpComparison {
            dimension: "Annual Cost".to_string(),
            hk_lpf: "HKD 100-300K".to_string(),
            cayman_elp: "USD 50-150K (HKD 390-1,170K equiv)".to_string(),
            advantage: "HK LPF (significantly lower)".to_string(),
        },
        CaymanLpComparison {
            dimension: "Tax Treatment".to_string(),
            hk_lpf: "UFE 0% + carry concession 0%".to_string(),
            cayman_elp: "No Cayman tax; investors taxed in home jurisdiction".to_string(),
            advantage: "Comparable (both effectively 0% at fund level)".to_string(),
        },
        CaymanLpComparison {
            dimension: "Carried Interest".to_string(),
            hk_lpf: "0% with concession (vs 16.5% standard)".to_string(),
            cayman_elp: "No Cayman tax on carry".to_string(),
            advantage: "Comparable (0% in both, but HK requires concession application)"
                .to_string(),
        },
        CaymanLpComparison {
            dimension: "Greater China Access".to_string(),
            hk_lpf: "Stock Connect, Bond Connect, direct access".to_string(),
            cayman_elp: "QFII/RQFII only, no Connect programs".to_string(),
            advantage: "HK LPF (significant advantage for China strategies)".to_string(),
        },
        CaymanLpComparison {
            dimension: "Investor Familiarity".to_string(),
            hk_lpf: "Growing acceptance, especially among Asian LPs".to_string(),
            cayman_elp: "Gold standard for institutional investors globally".to_string(),
            advantage: if input
                .target_investors
                .iter()
                .any(|i| i.contains("Asia") || i.contains("China") || i.contains("HK"))
            {
                "HK LPF (Asian investor preference)".to_string()
            } else {
                "Cayman ELP (global institutional standard)".to_string()
            },
        },
        CaymanLpComparison {
            dimension: "Substance Requirements".to_string(),
            hk_lpf: "Responsible person required; Companies Registry filing".to_string(),
            cayman_elp: "Economic Substance Act; CIGA test".to_string(),
            advantage: "Neutral (both require substance)".to_string(),
        },
        CaymanLpComparison {
            dimension: "GP Flexibility".to_string(),
            hk_lpf: "GP can be any entity; no HK incorporation required".to_string(),
            cayman_elp: "GP typically Cayman or BVI entity".to_string(),
            advantage: format!("HK LPF (GP in {} is acceptable)", input.gp_jurisdiction),
        },
    ]
}

// ---------------------------------------------------------------------------
// Validation — OFC
// ---------------------------------------------------------------------------

fn validate_ofc_input(input: &OfcInput) -> CorpFinanceResult<()> {
    if input.fund_name.trim().is_empty() {
        return Err(CorpFinanceError::InvalidInput {
            field: "fund_name".into(),
            reason: "Fund name cannot be empty".into(),
        });
    }

    let valid_types = ["Public", "Private"];
    if !valid_types.contains(&input.ofc_type.as_str()) {
        return Err(CorpFinanceError::InvalidInput {
            field: "ofc_type".into(),
            reason: format!(
                "Unknown OFC type '{}'. Valid: {:?}",
                input.ofc_type, valid_types
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

    if !input.type9_licensed_manager {
        // Warning but not hard error — output will flag it
    }

    if input.umbrella && input.sub_fund_count == 0 {
        return Err(CorpFinanceError::InvalidInput {
            field: "sub_fund_count".into(),
            reason: "Umbrella OFC must have at least 1 sub-fund".into(),
        });
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Validation — LPF
// ---------------------------------------------------------------------------

fn validate_lpf_input(input: &LpfInput) -> CorpFinanceResult<()> {
    if input.fund_name.trim().is_empty() {
        return Err(CorpFinanceError::InvalidInput {
            field: "fund_name".into(),
            reason: "Fund name cannot be empty".into(),
        });
    }

    let valid_purposes = ["PE", "VC", "RealEstate", "Infrastructure", "Credit"];
    if !valid_purposes.contains(&input.fund_purpose.as_str()) {
        return Err(CorpFinanceError::InvalidInput {
            field: "fund_purpose".into(),
            reason: format!(
                "Unknown fund purpose '{}'. Valid: {:?}",
                input.fund_purpose, valid_purposes
            ),
        });
    }

    if input.fund_size <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "fund_size".into(),
            reason: "Fund size must be greater than zero".into(),
        });
    }

    if input.fund_term_years < 1 {
        return Err(CorpFinanceError::InvalidInput {
            field: "fund_term_years".into(),
            reason: "Fund term must be at least 1 year".into(),
        });
    }

    if input.management_fee_rate < Decimal::ZERO || input.management_fee_rate >= Decimal::ONE {
        return Err(CorpFinanceError::InvalidInput {
            field: "management_fee_rate".into(),
            reason: "Management fee rate must be >= 0 and < 1".into(),
        });
    }

    if input.carried_interest_rate < Decimal::ZERO || input.carried_interest_rate >= Decimal::ONE {
        return Err(CorpFinanceError::InvalidInput {
            field: "carried_interest_rate".into(),
            reason: "Carried interest rate must be >= 0 and < 1".into(),
        });
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Validation — Carried Interest
// ---------------------------------------------------------------------------

fn validate_carry_input(input: &CarriedInterestInput) -> CorpFinanceResult<()> {
    let valid_fund_types = ["OFC", "LPF", "UnitTrust"];
    if !valid_fund_types.contains(&input.fund_type.as_str()) {
        return Err(CorpFinanceError::InvalidInput {
            field: "fund_type".into(),
            reason: format!(
                "Unknown fund type '{}'. Valid: {:?}",
                input.fund_type, valid_fund_types
            ),
        });
    }

    if input.carried_interest_amount <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "carried_interest_amount".into(),
            reason: "Carried interest amount must be greater than zero".into(),
        });
    }

    if input.qualifying_transactions_pct < Decimal::ZERO
        || input.qualifying_transactions_pct > Decimal::ONE
    {
        return Err(CorpFinanceError::InvalidInput {
            field: "qualifying_transactions_pct".into(),
            reason: "Qualifying transactions percentage must be between 0 and 1".into(),
        });
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Validation — HK vs Singapore
// ---------------------------------------------------------------------------

fn validate_hk_sg_input(input: &HkSgCompInput) -> CorpFinanceResult<()> {
    if input.fund_size <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "fund_size".into(),
            reason: "Fund size must be greater than zero".into(),
        });
    }

    let valid_strategies = [
        "PE",
        "VC",
        "Hedge",
        "RealEstate",
        "Infrastructure",
        "Credit",
    ];
    if !valid_strategies.contains(&input.fund_strategy.as_str()) {
        return Err(CorpFinanceError::InvalidInput {
            field: "fund_strategy".into(),
            reason: format!(
                "Unknown fund strategy '{}'. Valid: {:?}",
                input.fund_strategy, valid_strategies
            ),
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
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    // ======================================================================
    // Test Helpers
    // ======================================================================

    fn default_ofc_private() -> OfcInput {
        OfcInput {
            fund_name: "Asia Alpha OFC".to_string(),
            ofc_type: "Private".to_string(),
            umbrella: false,
            sub_fund_count: 0,
            fund_strategy: "Hedge".to_string(),
            fund_size: dec!(500_000_000),
            management_fee_rate: dec!(0.02),
            performance_fee_rate: dec!(0.20),
            type9_licensed_manager: true,
            grant_scheme_eligible: true,
            target_investors: vec!["Professional".to_string()],
        }
    }

    fn default_ofc_public() -> OfcInput {
        OfcInput {
            fund_name: "Asia Retail OFC".to_string(),
            ofc_type: "Public".to_string(),
            umbrella: true,
            sub_fund_count: 3,
            fund_strategy: "Hedge".to_string(),
            fund_size: dec!(1_000_000_000),
            management_fee_rate: dec!(0.015),
            performance_fee_rate: dec!(0.15),
            type9_licensed_manager: true,
            grant_scheme_eligible: true,
            target_investors: vec!["Retail".to_string(), "Professional".to_string()],
        }
    }

    fn default_lpf_pe() -> LpfInput {
        LpfInput {
            fund_name: "Greater China PE Fund I".to_string(),
            fund_purpose: "PE".to_string(),
            fund_size: dec!(500_000_000),
            gp_jurisdiction: "HongKong".to_string(),
            management_fee_rate: dec!(0.015),
            carried_interest_rate: dec!(0.20),
            fund_term_years: 10,
            responsible_person: "Asia Capital Management".to_string(),
            audit_waiver: false,
            target_investors: vec!["Institutional".to_string(), "Asia".to_string()],
        }
    }

    fn default_carry_eligible() -> CarriedInterestInput {
        CarriedInterestInput {
            fund_type: "LPF".to_string(),
            carried_interest_amount: dec!(50_000_000),
            fund_certified: true,
            avg_holding_period_months: 36,
            arms_length_terms: true,
            hk_employees: 3,
            qualifying_transactions_pct: dec!(0.80),
        }
    }

    fn default_hk_sg_comp() -> HkSgCompInput {
        HkSgCompInput {
            fund_size: dec!(500_000_000),
            fund_strategy: "PE".to_string(),
            target_market: "Greater China".to_string(),
            management_fee_rate: dec!(0.015),
            performance_fee_rate: dec!(0.20),
            carried_interest_amount: dec!(50_000_000),
        }
    }

    // ======================================================================
    // OFC Tests
    // ======================================================================

    #[test]
    fn test_ofc_private_basic() {
        let input = default_ofc_private();
        let result = analyze_ofc_structure(&input).unwrap();

        assert_eq!(result.fund_name, "Asia Alpha OFC");
        assert_eq!(result.structure_analysis.ofc_type, "Private");
        assert!(!result.regulatory.sfc_authorization_required);
        assert!(result.regulatory.type9_license_held);
    }

    #[test]
    fn test_ofc_public_basic() {
        let input = default_ofc_public();
        let result = analyze_ofc_structure(&input).unwrap();

        assert_eq!(result.structure_analysis.ofc_type, "Public");
        assert!(result.regulatory.sfc_authorization_required);
        assert!(result.structure_analysis.umbrella_structure);
        assert_eq!(result.structure_analysis.sub_fund_count, 3);
    }

    #[test]
    fn test_ofc_grant_scheme_eligible() {
        let input = default_ofc_private();
        let result = analyze_ofc_structure(&input).unwrap();

        assert!(result.grant_scheme_analysis.eligible);
        assert!(result.grant_scheme_analysis.estimated_grant > Decimal::ZERO);
        assert!(result.grant_scheme_analysis.estimated_grant <= dec!(1_000_000));
        assert_eq!(result.grant_scheme_analysis.reimbursement_rate, dec!(0.70));
    }

    #[test]
    fn test_ofc_grant_scheme_not_eligible() {
        let mut input = default_ofc_private();
        input.grant_scheme_eligible = false;
        let result = analyze_ofc_structure(&input).unwrap();

        assert!(!result.grant_scheme_analysis.eligible);
        assert_eq!(result.grant_scheme_analysis.estimated_grant, Decimal::ZERO);
    }

    #[test]
    fn test_ofc_grant_scheme_capped_at_1m() {
        let input = default_ofc_public();
        let result = analyze_ofc_structure(&input).unwrap();

        // Public OFC has estimated eligible costs of HKD 1.2M * 0.70 = 840K
        assert!(result.grant_scheme_analysis.estimated_grant <= dec!(1_000_000));
    }

    #[test]
    fn test_ofc_private_grant_estimate() {
        let input = default_ofc_private();
        let result = analyze_ofc_structure(&input).unwrap();

        // Private: 800K * 0.70 = 560K
        assert_eq!(result.grant_scheme_analysis.estimated_grant, dec!(560_000));
    }

    #[test]
    fn test_ofc_management_fee() {
        let input = default_ofc_private();
        let result = analyze_ofc_structure(&input).unwrap();

        let expected = dec!(500_000_000) * dec!(0.02);
        assert_eq!(result.cost_analysis.annual_management_fee, expected);
    }

    #[test]
    fn test_ofc_performance_fee() {
        let input = default_ofc_private();
        let result = analyze_ofc_structure(&input).unwrap();

        let expected = dec!(500_000_000) * dec!(0.20);
        assert_eq!(result.cost_analysis.annual_performance_fee, expected);
    }

    #[test]
    fn test_ofc_cost_range() {
        let input = default_ofc_private();
        let result = analyze_ofc_structure(&input).unwrap();

        // Private OFC: setup 100-200K, annual 200-400K
        assert_eq!(result.cost_analysis.setup_cost_low, dec!(100_000));
        assert_eq!(result.cost_analysis.setup_cost_high, dec!(200_000));
        assert!(result.cost_analysis.annual_cost_low >= dec!(200_000));
        assert!(result.cost_analysis.annual_cost_high <= dec!(500_000));
    }

    #[test]
    fn test_ofc_public_cost_higher() {
        let private_result = analyze_ofc_structure(&default_ofc_private()).unwrap();
        let public_result = analyze_ofc_structure(&default_ofc_public()).unwrap();

        // Public OFC has higher setup costs
        assert!(
            public_result.cost_analysis.setup_cost_low
                >= private_result.cost_analysis.setup_cost_low
        );
    }

    #[test]
    fn test_ofc_no_type9_warning() {
        let mut input = default_ofc_private();
        input.type9_licensed_manager = false;
        let result = analyze_ofc_structure(&input).unwrap();

        assert!(result.warnings.iter().any(|w| w.contains("Type 9")));
    }

    #[test]
    fn test_ofc_substance_with_type9() {
        let input = default_ofc_private();
        let result = analyze_ofc_structure(&input).unwrap();

        assert!(result.substance_analysis.type9_manager_in_hk);
        assert!(result.substance_analysis.substance_score >= 5);
    }

    #[test]
    fn test_ofc_substance_without_type9() {
        let mut input = default_ofc_private();
        input.type9_licensed_manager = false;
        let result = analyze_ofc_structure(&input).unwrap();

        assert!(!result.substance_analysis.type9_manager_in_hk);
        assert!(result.substance_analysis.substance_score < 10);
    }

    #[test]
    fn test_ofc_umbrella_sub_funds() {
        let mut input = default_ofc_private();
        input.umbrella = true;
        input.sub_fund_count = 4;
        let result = analyze_ofc_structure(&input).unwrap();

        assert!(result.structure_analysis.umbrella_structure);
        assert_eq!(result.structure_analysis.sub_fund_count, 4);
    }

    #[test]
    fn test_ofc_umbrella_zero_sub_funds_error() {
        let mut input = default_ofc_private();
        input.umbrella = true;
        input.sub_fund_count = 0;
        let result = analyze_ofc_structure(&input);

        assert!(result.is_err());
    }

    #[test]
    fn test_ofc_invalid_type() {
        let mut input = default_ofc_private();
        input.ofc_type = "Invalid".to_string();
        let result = analyze_ofc_structure(&input);

        assert!(result.is_err());
    }

    #[test]
    fn test_ofc_zero_fund_size() {
        let mut input = default_ofc_private();
        input.fund_size = Decimal::ZERO;
        let result = analyze_ofc_structure(&input);

        assert!(result.is_err());
    }

    #[test]
    fn test_ofc_negative_fee_rate() {
        let mut input = default_ofc_private();
        input.management_fee_rate = dec!(-0.01);
        assert!(analyze_ofc_structure(&input).is_err());
    }

    #[test]
    fn test_ofc_fee_rate_too_high() {
        let mut input = default_ofc_private();
        input.management_fee_rate = dec!(1.0);
        assert!(analyze_ofc_structure(&input).is_err());
    }

    #[test]
    fn test_ofc_empty_fund_name() {
        let mut input = default_ofc_private();
        input.fund_name = "".to_string();
        assert!(analyze_ofc_structure(&input).is_err());
    }

    #[test]
    fn test_ofc_pe_strategy_recommends_lpf() {
        let mut input = default_ofc_private();
        input.fund_strategy = "PE".to_string();
        let result = analyze_ofc_structure(&input).unwrap();

        assert!(result.recommendations.iter().any(|r| r.contains("LPF")));
    }

    #[test]
    fn test_ofc_reporting_frequency() {
        let public_result = analyze_ofc_structure(&default_ofc_public()).unwrap();
        let private_result = analyze_ofc_structure(&default_ofc_private()).unwrap();

        assert_eq!(public_result.regulatory.reporting_frequency, "Semi-annual");
        assert_eq!(private_result.regulatory.reporting_frequency, "Annual");
    }

    // ======================================================================
    // LPF Tests
    // ======================================================================

    #[test]
    fn test_lpf_pe_basic() {
        let input = default_lpf_pe();
        let result = analyze_lpf_structure(&input).unwrap();

        assert_eq!(result.fund_name, "Greater China PE Fund I");
        assert_eq!(result.structure_analysis.fund_purpose, "PE");
        assert!(result.structure_analysis.eligible_purpose);
        assert_eq!(
            result.structure_analysis.ordinance,
            "Limited Partnership Fund Ordinance (Cap 637)"
        );
    }

    #[test]
    fn test_lpf_gp_analysis() {
        let input = default_lpf_pe();
        let result = analyze_lpf_structure(&input).unwrap();

        assert_eq!(result.gp_analysis.gp_jurisdiction, "HongKong");
        assert!(result.gp_analysis.gp_unlimited_liability);
        assert!(!result.gp_analysis.responsible_person.is_empty());
    }

    #[test]
    fn test_lpf_regulatory_no_sfc() {
        let input = default_lpf_pe();
        let result = analyze_lpf_structure(&input).unwrap();

        assert!(!result.regulatory.sfc_authorization_required);
        assert_eq!(
            result.regulatory.registration_body,
            "Companies Registry (Hong Kong)"
        );
    }

    #[test]
    fn test_lpf_cost_range() {
        let input = default_lpf_pe();
        let result = analyze_lpf_structure(&input).unwrap();

        assert_eq!(result.cost_analysis.setup_cost_low, dec!(50_000));
        assert_eq!(result.cost_analysis.setup_cost_high, dec!(150_000));
        assert_eq!(result.cost_analysis.annual_cost_low, dec!(100_000));
        assert_eq!(result.cost_analysis.annual_cost_high, dec!(300_000));
    }

    #[test]
    fn test_lpf_management_fee() {
        let input = default_lpf_pe();
        let result = analyze_lpf_structure(&input).unwrap();

        let expected = dec!(500_000_000) * dec!(0.015);
        assert_eq!(result.cost_analysis.annual_management_fee, expected);
    }

    #[test]
    fn test_lpf_carried_interest() {
        let input = default_lpf_pe();
        let result = analyze_lpf_structure(&input).unwrap();

        let expected = dec!(500_000_000) * dec!(0.20);
        assert_eq!(result.cost_analysis.annual_carried_interest, expected);
    }

    #[test]
    fn test_lpf_cayman_comparison_has_entries() {
        let input = default_lpf_pe();
        let result = analyze_lpf_structure(&input).unwrap();

        assert!(!result.comparison_to_cayman_lp.is_empty());
        assert!(result.comparison_to_cayman_lp.len() >= 8);
    }

    #[test]
    fn test_lpf_cayman_comparison_cost_advantage() {
        let input = default_lpf_pe();
        let result = analyze_lpf_structure(&input).unwrap();

        let setup_comp = result
            .comparison_to_cayman_lp
            .iter()
            .find(|c| c.dimension == "Setup Cost")
            .unwrap();
        assert!(setup_comp.advantage.contains("HK LPF"));
    }

    #[test]
    fn test_lpf_carry_recommendation() {
        let input = default_lpf_pe();
        let result = analyze_lpf_structure(&input).unwrap();

        assert!(result
            .recommendations
            .iter()
            .any(|r| r.contains("carried interest")));
    }

    #[test]
    fn test_lpf_audit_waiver_small_fund() {
        let mut input = default_lpf_pe();
        input.fund_size = dec!(50_000_000);
        input.audit_waiver = true;
        let result = analyze_lpf_structure(&input).unwrap();

        assert!(!result.regulatory.audit_required);
        assert!(result.regulatory.audit_waiver_available);
    }

    #[test]
    fn test_lpf_audit_waiver_large_fund_rejected() {
        let mut input = default_lpf_pe();
        input.fund_size = dec!(200_000_000);
        input.audit_waiver = true;
        let result = analyze_lpf_structure(&input).unwrap();

        assert!(result.regulatory.audit_required);
        assert!(!result.regulatory.audit_waiver_available);
        assert!(result.warnings.iter().any(|w| w.contains("Audit waiver")));
    }

    #[test]
    fn test_lpf_long_term_warning() {
        let mut input = default_lpf_pe();
        input.fund_term_years = 15;
        let result = analyze_lpf_structure(&input).unwrap();

        assert!(result.recommendations.iter().any(|r| r.contains("exceeds")));
    }

    #[test]
    fn test_lpf_zero_term_error() {
        let mut input = default_lpf_pe();
        input.fund_term_years = 0;
        assert!(analyze_lpf_structure(&input).is_err());
    }

    #[test]
    fn test_lpf_invalid_purpose() {
        let mut input = default_lpf_pe();
        input.fund_purpose = "Hedge".to_string();
        assert!(analyze_lpf_structure(&input).is_err());
    }

    #[test]
    fn test_lpf_empty_responsible_person_warning() {
        let mut input = default_lpf_pe();
        input.responsible_person = "".to_string();
        let result = analyze_lpf_structure(&input).unwrap();

        assert!(result
            .warnings
            .iter()
            .any(|w| w.contains("Responsible person")));
    }

    #[test]
    fn test_lpf_foreign_gp_recommendation() {
        let mut input = default_lpf_pe();
        input.gp_jurisdiction = "Cayman".to_string();
        let result = analyze_lpf_structure(&input).unwrap();

        assert!(result
            .gp_analysis
            .recommendations
            .iter()
            .any(|r| r.contains("Cayman")));
    }

    #[test]
    fn test_lpf_asian_investor_comparison() {
        let input = default_lpf_pe();
        let result = analyze_lpf_structure(&input).unwrap();

        let investor_comp = result
            .comparison_to_cayman_lp
            .iter()
            .find(|c| c.dimension == "Investor Familiarity")
            .unwrap();
        assert!(investor_comp.advantage.contains("Asian"));
    }

    // ======================================================================
    // Carried Interest Tests
    // ======================================================================

    #[test]
    fn test_carry_eligible_zero_tax() {
        let input = default_carry_eligible();
        let result = carried_interest_concession(&input).unwrap();

        assert!(result.eligible);
        assert_eq!(result.effective_tax_rate, Decimal::ZERO);
        assert_eq!(result.tax_payable, Decimal::ZERO);
    }

    #[test]
    fn test_carry_eligible_tax_savings() {
        let input = default_carry_eligible();
        let result = carried_interest_concession(&input).unwrap();

        let expected_savings = dec!(50_000_000) * dec!(0.165);
        assert_eq!(result.tax_savings, expected_savings);
    }

    #[test]
    fn test_carry_ineligible_standard_rate() {
        let mut input = default_carry_eligible();
        input.fund_certified = false;
        let result = carried_interest_concession(&input).unwrap();

        assert!(!result.eligible);
        assert_eq!(result.effective_tax_rate, dec!(0.165));
        assert_eq!(result.tax_savings, Decimal::ZERO);
    }

    #[test]
    fn test_carry_ineligible_tax_payable() {
        let mut input = default_carry_eligible();
        input.fund_certified = false;
        let result = carried_interest_concession(&input).unwrap();

        let expected_tax = dec!(50_000_000) * dec!(0.165);
        assert_eq!(result.tax_payable, expected_tax);
    }

    #[test]
    fn test_carry_23_months_fails() {
        let mut input = default_carry_eligible();
        input.avg_holding_period_months = 23;
        let result = carried_interest_concession(&input).unwrap();

        assert!(!result.eligible);
        assert_eq!(result.effective_tax_rate, dec!(0.165));

        let holding_cond = result
            .conditions
            .iter()
            .find(|c| c.condition.contains("holding period"))
            .unwrap();
        assert!(!holding_cond.met);
    }

    #[test]
    fn test_carry_24_months_passes() {
        let mut input = default_carry_eligible();
        input.avg_holding_period_months = 24;
        let result = carried_interest_concession(&input).unwrap();

        assert!(result.eligible);
        assert_eq!(result.effective_tax_rate, Decimal::ZERO);

        let holding_cond = result
            .conditions
            .iter()
            .find(|c| c.condition.contains("holding period"))
            .unwrap();
        assert!(holding_cond.met);
    }

    #[test]
    fn test_carry_1_employee_fails() {
        let mut input = default_carry_eligible();
        input.hk_employees = 1;
        let result = carried_interest_concession(&input).unwrap();

        assert!(!result.eligible);
        assert_eq!(result.effective_tax_rate, dec!(0.165));

        let emp_cond = result
            .conditions
            .iter()
            .find(|c| c.condition.contains("employees"))
            .unwrap();
        assert!(!emp_cond.met);
    }

    #[test]
    fn test_carry_2_employees_passes() {
        let mut input = default_carry_eligible();
        input.hk_employees = 2;
        let result = carried_interest_concession(&input).unwrap();

        assert!(result.eligible);
        assert_eq!(result.effective_tax_rate, Decimal::ZERO);
    }

    #[test]
    fn test_carry_not_arms_length_fails() {
        let mut input = default_carry_eligible();
        input.arms_length_terms = false;
        let result = carried_interest_concession(&input).unwrap();

        assert!(!result.eligible);
        assert_eq!(result.effective_tax_rate, dec!(0.165));
    }

    #[test]
    fn test_carry_jurisdiction_comparison() {
        let input = default_carry_eligible();
        let result = carried_interest_concession(&input).unwrap();

        assert!(result.comparison_to_other_jurisdictions.len() >= 5);

        let hk_conc = result
            .comparison_to_other_jurisdictions
            .iter()
            .find(|j| j.jurisdiction.contains("with concession"))
            .unwrap();
        assert_eq!(hk_conc.carry_tax_rate, Decimal::ZERO);

        let uk = result
            .comparison_to_other_jurisdictions
            .iter()
            .find(|j| j.jurisdiction.contains("United Kingdom"))
            .unwrap();
        assert_eq!(uk.carry_tax_rate, dec!(0.28));
    }

    #[test]
    fn test_carry_cayman_zero_tax() {
        let input = default_carry_eligible();
        let result = carried_interest_concession(&input).unwrap();

        let cayman = result
            .comparison_to_other_jurisdictions
            .iter()
            .find(|j| j.jurisdiction.contains("Cayman"))
            .unwrap();
        assert_eq!(cayman.carry_tax_rate, Decimal::ZERO);
        assert_eq!(cayman.tax_on_carry, Decimal::ZERO);
    }

    #[test]
    fn test_carry_invalid_fund_type() {
        let mut input = default_carry_eligible();
        input.fund_type = "Invalid".to_string();
        assert!(carried_interest_concession(&input).is_err());
    }

    #[test]
    fn test_carry_zero_amount_error() {
        let mut input = default_carry_eligible();
        input.carried_interest_amount = Decimal::ZERO;
        assert!(carried_interest_concession(&input).is_err());
    }

    #[test]
    fn test_carry_conditions_all_met() {
        let input = default_carry_eligible();
        let result = carried_interest_concession(&input).unwrap();

        let core_conditions: Vec<&CarryCondition> = result
            .conditions
            .iter()
            .filter(|c| {
                c.condition.contains("certified")
                    || c.condition.contains("holding")
                    || c.condition.contains("arm's length")
                    || c.condition.contains("employees")
            })
            .collect();
        assert!(core_conditions.iter().all(|c| c.met));
    }

    #[test]
    fn test_carry_multiple_failures() {
        let mut input = default_carry_eligible();
        input.fund_certified = false;
        input.hk_employees = 1;
        input.avg_holding_period_months = 20;
        let result = carried_interest_concession(&input).unwrap();

        assert!(!result.eligible);
        let failed_count = result.conditions.iter().filter(|c| !c.met).count();
        assert!(failed_count >= 3);
    }

    // ======================================================================
    // HK vs Singapore Tests
    // ======================================================================

    #[test]
    fn test_hk_sg_pe_strategy() {
        let input = default_hk_sg_comp();
        let result = hk_vs_singapore(&input).unwrap();

        assert!(!result.comparison_matrix.is_empty());
        // For PE strategy, HK LPF should score highest
        assert!(result.hk_lpf_score >= result.sg_vcc_score);
        assert!(result.recommendation.contains("LPF"));
    }

    #[test]
    fn test_hk_sg_hedge_strategy() {
        let mut input = default_hk_sg_comp();
        input.fund_strategy = "Hedge".to_string();
        let result = hk_vs_singapore(&input).unwrap();

        // For hedge, OFC should get bonus points
        assert!(result.hk_ofc_score > 0);
        assert!(result
            .strategy_specific_notes
            .iter()
            .any(|n| n.contains("Hedge") || n.contains("OFC")));
    }

    #[test]
    fn test_hk_sg_comparison_matrix_dimensions() {
        let input = default_hk_sg_comp();
        let result = hk_vs_singapore(&input).unwrap();

        let dimensions: Vec<&str> = result
            .comparison_matrix
            .iter()
            .map(|c| c.dimension.as_str())
            .collect();
        assert!(dimensions.contains(&"Setup Cost"));
        assert!(dimensions.contains(&"Annual Cost"));
        assert!(dimensions.contains(&"Tax Treatment"));
        assert!(dimensions.contains(&"Greater China Access"));
        assert!(dimensions.contains(&"ASEAN Access"));
        assert!(dimensions.contains(&"Regulatory Burden"));
    }

    #[test]
    fn test_hk_sg_tax_advantage_hk() {
        let input = default_hk_sg_comp();
        let result = hk_vs_singapore(&input).unwrap();

        let tax_dim = result
            .comparison_matrix
            .iter()
            .find(|c| c.dimension == "Tax Treatment")
            .unwrap();
        assert!(tax_dim.best.contains("HK"));
    }

    #[test]
    fn test_hk_sg_asean_advantage_sg() {
        let input = default_hk_sg_comp();
        let result = hk_vs_singapore(&input).unwrap();

        let asean_dim = result
            .comparison_matrix
            .iter()
            .find(|c| c.dimension == "ASEAN Access")
            .unwrap();
        assert!(asean_dim.best.contains("SG VCC"));
    }

    #[test]
    fn test_hk_sg_greater_china_advantage_hk() {
        let input = default_hk_sg_comp();
        let result = hk_vs_singapore(&input).unwrap();

        let china_dim = result
            .comparison_matrix
            .iter()
            .find(|c| c.dimension == "Greater China Access")
            .unwrap();
        assert!(china_dim.best.contains("HK"));
    }

    #[test]
    fn test_hk_sg_carry_savings_note() {
        let input = default_hk_sg_comp();
        let result = hk_vs_singapore(&input).unwrap();

        assert!(result
            .strategy_specific_notes
            .iter()
            .any(|n| n.contains("Carry tax savings")));
    }

    #[test]
    fn test_hk_sg_invalid_strategy() {
        let mut input = default_hk_sg_comp();
        input.fund_strategy = "Invalid".to_string();
        assert!(hk_vs_singapore(&input).is_err());
    }

    #[test]
    fn test_hk_sg_zero_fund_size() {
        let mut input = default_hk_sg_comp();
        input.fund_size = Decimal::ZERO;
        assert!(hk_vs_singapore(&input).is_err());
    }

    #[test]
    fn test_hk_sg_small_fund_lpf_bonus() {
        let mut input = default_hk_sg_comp();
        input.fund_size = dec!(30_000_000);
        let result = hk_vs_singapore(&input).unwrap();

        assert!(result
            .strategy_specific_notes
            .iter()
            .any(|n| n.contains("Small fund")));
    }

    #[test]
    fn test_hk_sg_scores_positive() {
        let input = default_hk_sg_comp();
        let result = hk_vs_singapore(&input).unwrap();

        assert!(result.hk_ofc_score > 0);
        assert!(result.hk_lpf_score > 0);
        assert!(result.sg_vcc_score > 0);
    }
}
