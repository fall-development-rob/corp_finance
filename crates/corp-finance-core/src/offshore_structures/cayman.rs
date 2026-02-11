use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

use crate::error::CorpFinanceError;
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeederInfo {
    pub jurisdiction: String,
    /// "Cayman", "Delaware", "BVI", "Ireland"
    pub feeder_type: String,
    /// Allocation percentage as a decimal (0.40 = 40%)
    pub allocation_pct: Decimal,
    /// "USTaxExempt", "USTaxable", "NonUS"
    pub investor_profile: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceProviders {
    pub administrator: String,
    pub auditor: String,
    pub legal_counsel: String,
    pub prime_broker: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaymanFundInput {
    pub fund_name: String,
    /// "ExemptedLP", "SPC", "UnitTrust", "LLC", "BVI_BCA", "BVI_LP"
    pub structure_type: String,
    /// "Hedge", "PE", "VC", "RealEstate", "Credit", "FundOfFunds"
    pub fund_strategy: String,
    pub fund_size: Decimal,
    pub management_fee_rate: Decimal,
    /// Performance fee rate (hedge) or carried interest rate (PE/VC)
    pub performance_fee_rate: Decimal,
    pub hurdle_rate: Decimal,
    pub high_water_mark: bool,
    /// None for open-ended funds
    pub fund_term_years: Option<u32>,
    /// If true, this is a master-feeder structure
    pub master_feeder: bool,
    pub feeder_jurisdictions: Vec<FeederInfo>,
    pub service_providers: ServiceProviders,
    /// Cayman Islands Monetary Authority registered
    pub cima_registered: bool,
    pub annual_government_fees: Option<Decimal>,
}

// ---------------------------------------------------------------------------
// Output types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructureAnalysis {
    pub tax_status: String,
    pub exemption_period_years: u32,
    pub liability_protection: String,
    pub suitable_strategies: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeEconomics {
    pub gross_management_fee: Decimal,
    pub gross_performance_fee: Decimal,
    pub master_level_expenses: Decimal,
    pub feeder_level_expenses: Decimal,
    pub total_expense_ratio: Decimal,
    pub net_fee_to_manager: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeederEconomics {
    pub jurisdiction: String,
    pub feeder_type: String,
    pub allocation_pct: Decimal,
    pub feeder_aum: Decimal,
    pub feeder_admin_cost: Decimal,
    pub feeder_audit_cost: Decimal,
    pub feeder_legal_cost: Decimal,
    pub feeder_total_cost: Decimal,
    pub feeder_expense_ratio: Decimal,
    pub investor_profile: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MasterFeederAnalysis {
    pub feeders: Vec<FeederEconomics>,
    pub blocker_recommended: Vec<String>,
    pub total_aum_by_feeder: Vec<(String, Decimal)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegulatoryAnalysis {
    pub registration_category: String,
    pub cima_annual_fee: Decimal,
    pub audit_required: bool,
    pub nav_reporting: String,
    pub minimum_investment: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubstanceAnalysis {
    pub substance_score: u32,
    pub ciga_met: bool,
    pub local_directors_required: u32,
    pub board_meetings_required: u32,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostAnalysis {
    pub government_fees: Decimal,
    pub service_provider_costs: Decimal,
    pub total_annual_cost: Decimal,
    pub cost_pct_of_aum: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaymanFundOutput {
    pub structure_type: String,
    pub jurisdiction: String,
    pub structure_analysis: StructureAnalysis,
    pub fee_economics: FeeEconomics,
    pub master_feeder: Option<MasterFeederAnalysis>,
    pub regulatory: RegulatoryAnalysis,
    pub substance: SubstanceAnalysis,
    pub cost_analysis: CostAnalysis,
    pub recommendations: Vec<String>,
    pub warnings: Vec<String>,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

pub fn analyze_cayman_structure(input: &CaymanFundInput) -> CorpFinanceResult<CaymanFundOutput> {
    validate_input(input)?;

    let mut warnings: Vec<String> = Vec::new();
    let mut recommendations: Vec<String> = Vec::new();

    // ------------------------------------------------------------------
    // 1. Structure Selection Analysis
    // ------------------------------------------------------------------
    let structure_analysis = build_structure_analysis(
        &input.structure_type,
        &input.fund_strategy,
        &mut recommendations,
        &mut warnings,
    )?;

    let jurisdiction = determine_jurisdiction(&input.structure_type);

    // ------------------------------------------------------------------
    // 2. Fee Economics & Master-Feeder
    // ------------------------------------------------------------------
    let gross_mgmt_fee = input.fund_size * input.management_fee_rate;
    let gross_perf_fee = input.fund_size * input.performance_fee_rate;

    // Master-level annual expenses (admin, audit, legal, prime broker)
    let master_admin_cost = estimate_admin_cost(input.fund_size);
    let master_audit_cost = estimate_audit_cost(input.fund_size);
    let master_legal_cost = estimate_legal_cost(input.fund_size);
    let master_pb_cost = estimate_prime_broker_cost(input.fund_size);
    let master_level_expenses =
        master_admin_cost + master_audit_cost + master_legal_cost + master_pb_cost;

    let master_feeder = if input.master_feeder && !input.feeder_jurisdictions.is_empty() {
        let mf = build_master_feeder_analysis(input, &mut recommendations, &mut warnings);
        Some(mf)
    } else {
        None
    };

    let feeder_level_expenses = master_feeder
        .as_ref()
        .map(|mf| {
            mf.feeders
                .iter()
                .map(|f| f.feeder_total_cost)
                .sum::<Decimal>()
        })
        .unwrap_or(Decimal::ZERO);

    let total_expenses = master_level_expenses + feeder_level_expenses;
    let total_expense_ratio = if input.fund_size > Decimal::ZERO {
        total_expenses / input.fund_size
    } else {
        Decimal::ZERO
    };

    let net_fee_to_manager = gross_mgmt_fee + gross_perf_fee - total_expenses;

    let fee_economics = FeeEconomics {
        gross_management_fee: gross_mgmt_fee,
        gross_performance_fee: gross_perf_fee,
        master_level_expenses,
        feeder_level_expenses,
        total_expense_ratio,
        net_fee_to_manager,
    };

    // ------------------------------------------------------------------
    // 3. Regulatory Analysis
    // ------------------------------------------------------------------
    let regulatory = build_regulatory_analysis(input, &mut recommendations, &mut warnings);

    // ------------------------------------------------------------------
    // 4. Economic Substance Assessment
    // ------------------------------------------------------------------
    let substance = build_substance_analysis(input, &mut recommendations, &mut warnings);

    // ------------------------------------------------------------------
    // 5. Cost Analysis
    // ------------------------------------------------------------------
    let government_fees = input
        .annual_government_fees
        .unwrap_or_else(|| default_government_fees(&input.structure_type));

    let cima_fee = regulatory.cima_annual_fee;
    let total_govt = government_fees + cima_fee;

    // Service provider costs = master expenses (admin/audit/legal/PB)
    let service_provider_costs = master_level_expenses;
    let total_annual_cost = total_govt + service_provider_costs + feeder_level_expenses;
    let cost_pct_of_aum = if input.fund_size > Decimal::ZERO {
        total_annual_cost / input.fund_size
    } else {
        Decimal::ZERO
    };

    let cost_analysis = CostAnalysis {
        government_fees: total_govt,
        service_provider_costs,
        total_annual_cost,
        cost_pct_of_aum,
    };

    // ------------------------------------------------------------------
    // 6. Final recommendations
    // ------------------------------------------------------------------
    if cost_pct_of_aum > dec!(0.005) {
        warnings.push(format!(
            "Total annual cost is {:.2}% of AUM, which is above the typical 0.50% threshold",
            cost_pct_of_aum * dec!(100)
        ));
    }

    if input.high_water_mark && matches!(input.fund_strategy.as_str(), "PE" | "VC" | "RealEstate") {
        warnings.push(
            "High water mark is unusual for closed-end PE/VC/RE funds; \
             typically used for hedge funds"
                .to_string(),
        );
    }

    if !input.master_feeder && !input.feeder_jurisdictions.is_empty() {
        warnings.push(
            "Feeder jurisdictions provided but master_feeder is false; \
             feeder data ignored"
                .to_string(),
        );
    }

    if input.fund_term_years.is_none()
        && matches!(input.fund_strategy.as_str(), "PE" | "VC" | "RealEstate")
    {
        recommendations.push(
            "Consider specifying fund_term_years for closed-end PE/VC/RE strategies".to_string(),
        );
    }

    Ok(CaymanFundOutput {
        structure_type: input.structure_type.clone(),
        jurisdiction,
        structure_analysis,
        fee_economics,
        master_feeder,
        regulatory,
        substance,
        cost_analysis,
        recommendations,
        warnings,
    })
}

// ---------------------------------------------------------------------------
// Helpers — Structure Analysis
// ---------------------------------------------------------------------------

fn determine_jurisdiction(structure_type: &str) -> String {
    match structure_type {
        "BVI_BCA" | "BVI_LP" => "British Virgin Islands".to_string(),
        _ => "Cayman Islands".to_string(),
    }
}

fn build_structure_analysis(
    structure_type: &str,
    fund_strategy: &str,
    recommendations: &mut Vec<String>,
    warnings: &mut Vec<String>,
) -> CorpFinanceResult<StructureAnalysis> {
    let (tax_status, exemption_period, liability_protection, suitable) = match structure_type {
        "ExemptedLP" => (
            "Tax exempt — no Cayman income, capital gains, or withholding tax".to_string(),
            50u32,
            "LP limited liability; GP unlimited liability unless LLC GP".to_string(),
            vec![
                "PE".to_string(),
                "VC".to_string(),
                "RealEstate".to_string(),
                "Credit".to_string(),
                "FundOfFunds".to_string(),
            ],
        ),
        "SPC" => (
            "Tax exempt — segregated portfolios with statutory ring-fencing".to_string(),
            50,
            "Statutory segregation between portfolios; creditors of one SP \
                 cannot access assets of another"
                .to_string(),
            vec![
                "Hedge".to_string(),
                "FundOfFunds".to_string(),
                "Credit".to_string(),
            ],
        ),
        "UnitTrust" => (
            "Tax exempt — unit trust structure with trustee oversight".to_string(),
            50,
            "Trustee holds assets; investors have beneficial interest".to_string(),
            vec!["Hedge".to_string(), "FundOfFunds".to_string()],
        ),
        "LLC" => (
            "Tax exempt — limited liability company".to_string(),
            50,
            "Members have limited liability".to_string(),
            vec![
                "Hedge".to_string(),
                "Credit".to_string(),
                "FundOfFunds".to_string(),
            ],
        ),
        "BVI_BCA" => (
            "No BVI income tax, capital gains tax, or withholding tax".to_string(),
            0,
            "Limited liability; directors personally liable for fraud".to_string(),
            vec![
                "Hedge".to_string(),
                "PE".to_string(),
                "VC".to_string(),
                "Credit".to_string(),
            ],
        ),
        "BVI_LP" => (
            "Tax transparent — no BVI entity-level tax".to_string(),
            0,
            "LP limited liability; GP unlimited liability".to_string(),
            vec!["PE".to_string(), "VC".to_string(), "RealEstate".to_string()],
        ),
        other => {
            return Err(CorpFinanceError::InvalidInput {
                field: "structure_type".into(),
                reason: format!(
                    "Unknown structure type '{}'. Expected one of: \
                         ExemptedLP, SPC, UnitTrust, LLC, BVI_BCA, BVI_LP",
                    other
                ),
            });
        }
    };

    if !suitable.contains(&fund_strategy.to_string()) {
        warnings.push(format!(
            "Strategy '{}' is not typically associated with {} structure; \
             consider: {:?}",
            fund_strategy, structure_type, suitable
        ));
    }

    // Strategy-specific recommendations
    match fund_strategy {
        "Hedge" => {
            if structure_type != "SPC" && structure_type != "LLC" {
                recommendations.push(
                    "For multi-strategy hedge funds, SPC provides \
                     portfolio segregation benefits"
                        .to_string(),
                );
            }
        }
        "PE" | "VC" => {
            if structure_type != "ExemptedLP" && structure_type != "BVI_LP" {
                recommendations.push(
                    "ExemptedLP is the most common structure for PE/VC \
                     funds in Cayman"
                        .to_string(),
                );
            }
        }
        _ => {}
    }

    Ok(StructureAnalysis {
        tax_status,
        exemption_period_years: exemption_period,
        liability_protection,
        suitable_strategies: suitable,
    })
}

// ---------------------------------------------------------------------------
// Helpers — Master-Feeder
// ---------------------------------------------------------------------------

fn build_master_feeder_analysis(
    input: &CaymanFundInput,
    recommendations: &mut Vec<String>,
    warnings: &mut Vec<String>,
) -> MasterFeederAnalysis {
    let mut feeders = Vec::new();
    let mut total_aum_by_feeder = Vec::new();
    let mut blocker_recommended = Vec::new();

    let mut total_alloc = Decimal::ZERO;

    for fi in &input.feeder_jurisdictions {
        total_alloc += fi.allocation_pct;

        let feeder_aum = input.fund_size * fi.allocation_pct;

        // Feeder-level costs scale with feeder AUM
        let feeder_admin = estimate_feeder_admin_cost(feeder_aum);
        let feeder_audit = estimate_feeder_audit_cost(feeder_aum);
        let feeder_legal = estimate_feeder_legal_cost(feeder_aum);
        let feeder_total = feeder_admin + feeder_audit + feeder_legal;
        let feeder_expense_ratio = if feeder_aum > Decimal::ZERO {
            feeder_total / feeder_aum
        } else {
            Decimal::ZERO
        };

        // US blocker analysis
        if fi.investor_profile == "USTaxExempt"
            && matches!(input.fund_strategy.as_str(), "Credit" | "RealEstate" | "PE")
        {
            blocker_recommended.push(format!(
                "{} feeder ({}) — US tax-exempt investors may need a blocker \
                 corporation to avoid UBTI from debt-financed income",
                fi.jurisdiction, fi.feeder_type
            ));
        }

        if fi.investor_profile == "USTaxable" {
            recommendations.push(format!(
                "Consider Delaware LP feeder for US taxable investors in {} \
                 for pass-through tax treatment",
                fi.jurisdiction
            ));
        }

        total_aum_by_feeder.push((fi.jurisdiction.clone(), feeder_aum));

        feeders.push(FeederEconomics {
            jurisdiction: fi.jurisdiction.clone(),
            feeder_type: fi.feeder_type.clone(),
            allocation_pct: fi.allocation_pct,
            feeder_aum,
            feeder_admin_cost: feeder_admin,
            feeder_audit_cost: feeder_audit,
            feeder_legal_cost: feeder_legal,
            feeder_total_cost: feeder_total,
            feeder_expense_ratio,
            investor_profile: fi.investor_profile.clone(),
        });
    }

    if (total_alloc - Decimal::ONE).abs() > dec!(0.01) {
        warnings.push(format!(
            "Feeder allocation percentages sum to {}, not 100%",
            total_alloc * dec!(100)
        ));
    }

    MasterFeederAnalysis {
        feeders,
        blocker_recommended,
        total_aum_by_feeder,
    }
}

// ---------------------------------------------------------------------------
// Helpers — Regulatory
// ---------------------------------------------------------------------------

fn build_regulatory_analysis(
    input: &CaymanFundInput,
    recommendations: &mut Vec<String>,
    warnings: &mut [String],
) -> RegulatoryAnalysis {
    // BVI structures have their own regulatory framework
    if input.structure_type == "BVI_BCA" || input.structure_type == "BVI_LP" {
        return build_bvi_regulatory(input, recommendations, warnings);
    }

    // CIMA registration categories
    let (category, cima_fee, min_investment, nav_reporting) = if !input.cima_registered {
        // Not registered — private fund exemption
        recommendations.push(
            "Consider CIMA registration for broader investor access \
                 and institutional credibility"
                .to_string(),
        );
        (
            "Unregistered (private fund exemption)".to_string(),
            Decimal::ZERO,
            Decimal::ZERO,
            "Not required".to_string(),
        )
    } else if input.fund_size >= dec!(500_000_000) {
        // Large fund — Licensed
        (
            "Licensed (CIMA full oversight)".to_string(),
            dec!(4_878),
            dec!(100_000),
            "Monthly".to_string(),
        )
    } else if input.fund_size >= dec!(100_000_000) {
        // Administered fund
        (
            "Administered".to_string(),
            dec!(4_268),
            Decimal::ZERO,
            "Quarterly".to_string(),
        )
    } else {
        // Registered fund (s.4(3))
        (
            "Registered (s.4(3))".to_string(),
            dec!(3_658),
            Decimal::ZERO,
            "Annually".to_string(),
        )
    };

    if input.cima_registered && input.structure_type == "SPC" {
        recommendations
            .push("SPC must register each segregated portfolio separately with CIMA".to_string());
    }

    RegulatoryAnalysis {
        registration_category: category,
        cima_annual_fee: cima_fee,
        audit_required: input.cima_registered,
        nav_reporting,
        minimum_investment: min_investment,
    }
}

fn build_bvi_regulatory(
    input: &CaymanFundInput,
    recommendations: &mut Vec<String>,
    _warnings: &mut [String],
) -> RegulatoryAnalysis {
    // BVI Financial Services Commission (FSC)
    let (category, fee, min_inv) = if input.cima_registered {
        // "cima_registered" reused for BVI FSC registration
        (
            "BVI FSC Registered Fund".to_string(),
            dec!(1_500),
            Decimal::ZERO,
        )
    } else {
        recommendations.push(
            "BVI funds should consider FSC registration for institutional \
             investor access"
                .to_string(),
        );
        (
            "BVI Private Fund (unregistered)".to_string(),
            dec!(350),
            Decimal::ZERO,
        )
    };

    recommendations.push(
        "Ensure compliance with BVI Economic Substance Act 2018 — \
         fund management is a relevant activity"
            .to_string(),
    );

    RegulatoryAnalysis {
        registration_category: category,
        cima_annual_fee: fee,
        audit_required: input.cima_registered,
        nav_reporting: "Annually".to_string(),
        minimum_investment: min_inv,
    }
}

// ---------------------------------------------------------------------------
// Helpers — Substance
// ---------------------------------------------------------------------------

fn build_substance_analysis(
    input: &CaymanFundInput,
    recommendations: &mut Vec<String>,
    _warnings: &mut [String],
) -> SubstanceAnalysis {
    let mut score: u32 = 0;
    let mut recs: Vec<String> = Vec::new();

    let is_bvi = input.structure_type == "BVI_BCA" || input.structure_type == "BVI_LP";

    // Board meetings in jurisdiction
    let board_meetings_required = if is_bvi { 1 } else { 2 };
    // Assume 2 local board meetings adds +2 to score
    score += board_meetings_required;

    // Local directors
    let local_directors_required = if is_bvi { 1 } else { 2 };
    score += local_directors_required;

    // Fund size determines expected substance level
    if input.fund_size >= dec!(500_000_000) {
        // Large funds expected to have significant local substance
        recs.push(
            "Large fund (>$500M): ensure adequate local employees and \
             expenditure in jurisdiction"
                .to_string(),
        );
        score += 2;
    } else if input.fund_size >= dec!(100_000_000) {
        score += 1;
    }

    // CIMA-registered funds have higher substance expectations
    if input.cima_registered {
        score += 1;
        recs.push(
            "CIMA-registered: maintain records of board decision-making \
             in Cayman"
                .to_string(),
        );
    }

    // Service providers in jurisdiction
    if !input.service_providers.administrator.is_empty() {
        score += 1;
    }

    // CIGA test: are core income-generating activities directed from
    // the jurisdiction?
    let ciga_met = score >= 5;

    if !ciga_met {
        recs.push(
            "Substance score is below 5/10 — increase local presence: \
             more board meetings, local directors, or local staff"
                .to_string(),
        );
    }

    // Cap score at 10
    let final_score = score.min(10);

    recommendations.append(&mut recs);

    SubstanceAnalysis {
        substance_score: final_score,
        ciga_met,
        local_directors_required,
        board_meetings_required,
        recommendations: Vec::new(),
    }
}

// ---------------------------------------------------------------------------
// Helpers — Cost Estimation
// ---------------------------------------------------------------------------

/// Estimate annual fund administrator cost based on AUM tier
fn estimate_admin_cost(fund_size: Decimal) -> Decimal {
    if fund_size >= dec!(1_000_000_000) {
        dec!(250_000)
    } else if fund_size >= dec!(500_000_000) {
        dec!(175_000)
    } else if fund_size >= dec!(100_000_000) {
        dec!(120_000)
    } else {
        dec!(75_000)
    }
}

fn estimate_audit_cost(fund_size: Decimal) -> Decimal {
    if fund_size >= dec!(1_000_000_000) {
        dec!(150_000)
    } else if fund_size >= dec!(500_000_000) {
        dec!(100_000)
    } else if fund_size >= dec!(100_000_000) {
        dec!(75_000)
    } else {
        dec!(50_000)
    }
}

fn estimate_legal_cost(fund_size: Decimal) -> Decimal {
    if fund_size >= dec!(500_000_000) {
        dec!(100_000)
    } else if fund_size >= dec!(100_000_000) {
        dec!(60_000)
    } else {
        dec!(40_000)
    }
}

fn estimate_prime_broker_cost(fund_size: Decimal) -> Decimal {
    if fund_size >= dec!(1_000_000_000) {
        dec!(200_000)
    } else if fund_size >= dec!(500_000_000) {
        dec!(125_000)
    } else if fund_size >= dec!(100_000_000) {
        dec!(80_000)
    } else {
        dec!(50_000)
    }
}

/// Feeder-level admin costs (typically lower than master)
fn estimate_feeder_admin_cost(feeder_aum: Decimal) -> Decimal {
    if feeder_aum >= dec!(500_000_000) {
        dec!(80_000)
    } else if feeder_aum >= dec!(100_000_000) {
        dec!(50_000)
    } else {
        dec!(30_000)
    }
}

fn estimate_feeder_audit_cost(feeder_aum: Decimal) -> Decimal {
    if feeder_aum >= dec!(500_000_000) {
        dec!(60_000)
    } else if feeder_aum >= dec!(100_000_000) {
        dec!(40_000)
    } else {
        dec!(25_000)
    }
}

fn estimate_feeder_legal_cost(feeder_aum: Decimal) -> Decimal {
    if feeder_aum >= dec!(500_000_000) {
        dec!(40_000)
    } else if feeder_aum >= dec!(100_000_000) {
        dec!(25_000)
    } else {
        dec!(15_000)
    }
}

/// Default government fees by structure type
fn default_government_fees(structure_type: &str) -> Decimal {
    match structure_type {
        "ExemptedLP" => dec!(3_660),
        "SPC" => dec!(4_270),
        "UnitTrust" => dec!(3_050),
        "LLC" => dec!(2_745),
        "BVI_BCA" => dec!(1_600),
        "BVI_LP" => dec!(1_800),
        _ => dec!(3_000),
    }
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

fn validate_input(input: &CaymanFundInput) -> CorpFinanceResult<()> {
    if input.fund_name.trim().is_empty() {
        return Err(CorpFinanceError::InvalidInput {
            field: "fund_name".into(),
            reason: "Fund name cannot be empty".into(),
        });
    }

    let valid_structures = ["ExemptedLP", "SPC", "UnitTrust", "LLC", "BVI_BCA", "BVI_LP"];
    if !valid_structures.contains(&input.structure_type.as_str()) {
        return Err(CorpFinanceError::InvalidInput {
            field: "structure_type".into(),
            reason: format!(
                "Unknown structure type '{}'. Valid: {:?}",
                input.structure_type, valid_structures
            ),
        });
    }

    let valid_strategies = ["Hedge", "PE", "VC", "RealEstate", "Credit", "FundOfFunds"];
    if !valid_strategies.contains(&input.fund_strategy.as_str()) {
        return Err(CorpFinanceError::InvalidInput {
            field: "fund_strategy".into(),
            reason: format!(
                "Unknown fund strategy '{}'. Valid: {:?}",
                input.fund_strategy, valid_strategies
            ),
        });
    }

    if input.fund_size <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "fund_size".into(),
            reason: "Fund size must be greater than zero".into(),
        });
    }

    if input.management_fee_rate < Decimal::ZERO || input.management_fee_rate > Decimal::ONE {
        return Err(CorpFinanceError::InvalidInput {
            field: "management_fee_rate".into(),
            reason: "Management fee rate must be between 0 and 1".into(),
        });
    }

    if input.performance_fee_rate < Decimal::ZERO || input.performance_fee_rate > Decimal::ONE {
        return Err(CorpFinanceError::InvalidInput {
            field: "performance_fee_rate".into(),
            reason: "Performance fee rate must be between 0 and 1".into(),
        });
    }

    if input.hurdle_rate < Decimal::ZERO || input.hurdle_rate > Decimal::ONE {
        return Err(CorpFinanceError::InvalidInput {
            field: "hurdle_rate".into(),
            reason: "Hurdle rate must be between 0 and 1".into(),
        });
    }

    if input.master_feeder {
        for (i, fi) in input.feeder_jurisdictions.iter().enumerate() {
            if fi.allocation_pct < Decimal::ZERO || fi.allocation_pct > Decimal::ONE {
                return Err(CorpFinanceError::InvalidInput {
                    field: format!("feeder_jurisdictions[{}].allocation_pct", i),
                    reason: "Allocation percentage must be between 0 and 1".into(),
                });
            }
            let valid_feeder_types = ["Cayman", "Delaware", "BVI", "Ireland"];
            if !valid_feeder_types.contains(&fi.feeder_type.as_str()) {
                return Err(CorpFinanceError::InvalidInput {
                    field: format!("feeder_jurisdictions[{}].feeder_type", i),
                    reason: format!(
                        "Unknown feeder type '{}'. Valid: {:?}",
                        fi.feeder_type, valid_feeder_types
                    ),
                });
            }
            let valid_profiles = ["USTaxExempt", "USTaxable", "NonUS"];
            if !valid_profiles.contains(&fi.investor_profile.as_str()) {
                return Err(CorpFinanceError::InvalidInput {
                    field: format!("feeder_jurisdictions[{}].investor_profile", i),
                    reason: format!(
                        "Unknown investor profile '{}'. Valid: {:?}",
                        fi.investor_profile, valid_profiles
                    ),
                });
            }
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
    use rust_decimal_macros::dec;

    fn default_service_providers() -> ServiceProviders {
        ServiceProviders {
            administrator: "Citco Fund Services".to_string(),
            auditor: "KPMG".to_string(),
            legal_counsel: "Maples & Calder".to_string(),
            prime_broker: "Goldman Sachs".to_string(),
        }
    }

    fn hedge_fund_input() -> CaymanFundInput {
        CaymanFundInput {
            fund_name: "Alpha Offshore Fund".to_string(),
            structure_type: "SPC".to_string(),
            fund_strategy: "Hedge".to_string(),
            fund_size: dec!(500_000_000),
            management_fee_rate: dec!(0.02),
            performance_fee_rate: dec!(0.20),
            hurdle_rate: dec!(0.0),
            high_water_mark: true,
            fund_term_years: None,
            master_feeder: false,
            feeder_jurisdictions: vec![],
            service_providers: default_service_providers(),
            cima_registered: true,
            annual_government_fees: None,
        }
    }

    fn pe_fund_input() -> CaymanFundInput {
        CaymanFundInput {
            fund_name: "Cayman PE Fund I".to_string(),
            structure_type: "ExemptedLP".to_string(),
            fund_strategy: "PE".to_string(),
            fund_size: dec!(1_000_000_000),
            management_fee_rate: dec!(0.015),
            performance_fee_rate: dec!(0.20),
            hurdle_rate: dec!(0.08),
            high_water_mark: false,
            fund_term_years: Some(10),
            master_feeder: false,
            feeder_jurisdictions: vec![],
            service_providers: default_service_providers(),
            cima_registered: true,
            annual_government_fees: None,
        }
    }

    fn master_feeder_input() -> CaymanFundInput {
        CaymanFundInput {
            fund_name: "Global Master Fund".to_string(),
            structure_type: "SPC".to_string(),
            fund_strategy: "Hedge".to_string(),
            fund_size: dec!(750_000_000),
            management_fee_rate: dec!(0.02),
            performance_fee_rate: dec!(0.20),
            hurdle_rate: dec!(0.0),
            high_water_mark: true,
            fund_term_years: None,
            master_feeder: true,
            feeder_jurisdictions: vec![
                FeederInfo {
                    jurisdiction: "Cayman Islands".to_string(),
                    feeder_type: "Cayman".to_string(),
                    allocation_pct: dec!(0.40),
                    investor_profile: "NonUS".to_string(),
                },
                FeederInfo {
                    jurisdiction: "Delaware".to_string(),
                    feeder_type: "Delaware".to_string(),
                    allocation_pct: dec!(0.35),
                    investor_profile: "USTaxable".to_string(),
                },
                FeederInfo {
                    jurisdiction: "Cayman Islands".to_string(),
                    feeder_type: "Cayman".to_string(),
                    allocation_pct: dec!(0.25),
                    investor_profile: "USTaxExempt".to_string(),
                },
            ],
            service_providers: default_service_providers(),
            cima_registered: true,
            annual_government_fees: None,
        }
    }

    // ------------------------------------------------------------------
    // 1. Basic SPC hedge fund
    // ------------------------------------------------------------------
    #[test]
    fn test_basic_spc_hedge_fund() {
        let input = hedge_fund_input();
        let result = analyze_cayman_structure(&input).unwrap();

        assert_eq!(result.structure_type, "SPC");
        assert_eq!(result.jurisdiction, "Cayman Islands");
        assert!(result
            .structure_analysis
            .suitable_strategies
            .contains(&"Hedge".to_string()));
    }

    // ------------------------------------------------------------------
    // 2. ExemptedLP PE fund
    // ------------------------------------------------------------------
    #[test]
    fn test_exempted_lp_pe_fund() {
        let input = pe_fund_input();
        let result = analyze_cayman_structure(&input).unwrap();

        assert_eq!(result.structure_type, "ExemptedLP");
        assert_eq!(result.structure_analysis.exemption_period_years, 50);
        assert!(result
            .structure_analysis
            .suitable_strategies
            .contains(&"PE".to_string()));
    }

    // ------------------------------------------------------------------
    // 3. Fee economics — management fee
    // ------------------------------------------------------------------
    #[test]
    fn test_fee_economics_management_fee() {
        let input = hedge_fund_input();
        let result = analyze_cayman_structure(&input).unwrap();

        let expected_mgmt = dec!(500_000_000) * dec!(0.02);
        assert_eq!(result.fee_economics.gross_management_fee, expected_mgmt);
    }

    // ------------------------------------------------------------------
    // 4. Fee economics — performance fee
    // ------------------------------------------------------------------
    #[test]
    fn test_fee_economics_performance_fee() {
        let input = hedge_fund_input();
        let result = analyze_cayman_structure(&input).unwrap();

        let expected_perf = dec!(500_000_000) * dec!(0.20);
        assert_eq!(result.fee_economics.gross_performance_fee, expected_perf);
    }

    // ------------------------------------------------------------------
    // 5. Total expense ratio is positive
    // ------------------------------------------------------------------
    #[test]
    fn test_total_expense_ratio_positive() {
        let input = hedge_fund_input();
        let result = analyze_cayman_structure(&input).unwrap();

        assert!(result.fee_economics.total_expense_ratio > Decimal::ZERO);
    }

    // ------------------------------------------------------------------
    // 6. Master-feeder with feeders
    // ------------------------------------------------------------------
    #[test]
    fn test_master_feeder_structure() {
        let input = master_feeder_input();
        let result = analyze_cayman_structure(&input).unwrap();

        assert!(result.master_feeder.is_some());
        let mf = result.master_feeder.unwrap();
        assert_eq!(mf.feeders.len(), 3);
    }

    // ------------------------------------------------------------------
    // 7. Master-feeder AUM allocation
    // ------------------------------------------------------------------
    #[test]
    fn test_master_feeder_aum_allocation() {
        let input = master_feeder_input();
        let result = analyze_cayman_structure(&input).unwrap();

        let mf = result.master_feeder.unwrap();
        let total_feeder_aum: Decimal = mf.total_aum_by_feeder.iter().map(|(_, v)| *v).sum();

        assert_eq!(total_feeder_aum, dec!(750_000_000));
    }

    // ------------------------------------------------------------------
    // 8. US blocker recommendation for USTaxExempt in credit
    // ------------------------------------------------------------------
    #[test]
    fn test_us_blocker_recommendation() {
        let mut input = master_feeder_input();
        input.fund_strategy = "Credit".to_string();
        let result = analyze_cayman_structure(&input).unwrap();

        let mf = result.master_feeder.unwrap();
        assert!(
            !mf.blocker_recommended.is_empty(),
            "Should recommend blocker for US tax-exempt in credit strategy"
        );
    }

    // ------------------------------------------------------------------
    // 9. No master-feeder when flag is false
    // ------------------------------------------------------------------
    #[test]
    fn test_no_master_feeder_when_disabled() {
        let input = hedge_fund_input();
        let result = analyze_cayman_structure(&input).unwrap();

        assert!(result.master_feeder.is_none());
    }

    // ------------------------------------------------------------------
    // 10. CIMA regulatory — registered fund
    // ------------------------------------------------------------------
    #[test]
    fn test_cima_registered_fund() {
        let mut input = hedge_fund_input();
        input.fund_size = dec!(80_000_000);
        input.cima_registered = true;
        let result = analyze_cayman_structure(&input).unwrap();

        assert_eq!(
            result.regulatory.registration_category,
            "Registered (s.4(3))"
        );
        assert_eq!(result.regulatory.cima_annual_fee, dec!(3_658));
        assert!(result.regulatory.audit_required);
    }

    // ------------------------------------------------------------------
    // 11. CIMA regulatory — administered fund
    // ------------------------------------------------------------------
    #[test]
    fn test_cima_administered_fund() {
        let mut input = hedge_fund_input();
        input.fund_size = dec!(200_000_000);
        input.cima_registered = true;
        let result = analyze_cayman_structure(&input).unwrap();

        assert_eq!(result.regulatory.registration_category, "Administered");
        assert_eq!(result.regulatory.cima_annual_fee, dec!(4_268));
    }

    // ------------------------------------------------------------------
    // 12. CIMA regulatory — licensed fund
    // ------------------------------------------------------------------
    #[test]
    fn test_cima_licensed_fund() {
        let mut input = hedge_fund_input();
        input.fund_size = dec!(500_000_000);
        input.cima_registered = true;
        let result = analyze_cayman_structure(&input).unwrap();

        assert_eq!(
            result.regulatory.registration_category,
            "Licensed (CIMA full oversight)"
        );
        assert_eq!(result.regulatory.cima_annual_fee, dec!(4_878));
        assert_eq!(result.regulatory.minimum_investment, dec!(100_000));
    }

    // ------------------------------------------------------------------
    // 13. Unregistered fund — no CIMA fee
    // ------------------------------------------------------------------
    #[test]
    fn test_unregistered_fund() {
        let mut input = hedge_fund_input();
        input.cima_registered = false;
        let result = analyze_cayman_structure(&input).unwrap();

        assert_eq!(result.regulatory.cima_annual_fee, Decimal::ZERO);
        assert!(!result.regulatory.audit_required);
    }

    // ------------------------------------------------------------------
    // 14. BVI BCA structure
    // ------------------------------------------------------------------
    #[test]
    fn test_bvi_bca_structure() {
        let mut input = hedge_fund_input();
        input.structure_type = "BVI_BCA".to_string();
        let result = analyze_cayman_structure(&input).unwrap();

        assert_eq!(result.jurisdiction, "British Virgin Islands");
        assert_eq!(result.structure_analysis.exemption_period_years, 0);
    }

    // ------------------------------------------------------------------
    // 15. BVI LP structure
    // ------------------------------------------------------------------
    #[test]
    fn test_bvi_lp_structure() {
        let mut input = pe_fund_input();
        input.structure_type = "BVI_LP".to_string();
        let result = analyze_cayman_structure(&input).unwrap();

        assert_eq!(result.jurisdiction, "British Virgin Islands");
        assert!(result
            .structure_analysis
            .suitable_strategies
            .contains(&"PE".to_string()));
    }

    // ------------------------------------------------------------------
    // 16. Substance score >= 5 for large registered fund
    // ------------------------------------------------------------------
    #[test]
    fn test_substance_score_large_fund() {
        let input = pe_fund_input();
        let result = analyze_cayman_structure(&input).unwrap();

        // Large ($1B), CIMA registered, with service providers
        assert!(
            result.substance.substance_score >= 5,
            "Substance score should be >= 5 for large registered fund, got {}",
            result.substance.substance_score
        );
        assert!(result.substance.ciga_met);
    }

    // ------------------------------------------------------------------
    // 17. Cost analysis — government fees default
    // ------------------------------------------------------------------
    #[test]
    fn test_cost_analysis_default_government_fees() {
        let input = hedge_fund_input();
        let result = analyze_cayman_structure(&input).unwrap();

        // SPC default government fee is 4270
        assert!(result.cost_analysis.government_fees > Decimal::ZERO);
    }

    // ------------------------------------------------------------------
    // 18. Cost analysis — custom government fees
    // ------------------------------------------------------------------
    #[test]
    fn test_cost_analysis_custom_government_fees() {
        let mut input = hedge_fund_input();
        input.annual_government_fees = Some(dec!(10_000));
        let result = analyze_cayman_structure(&input).unwrap();

        // Government fees include custom + CIMA
        assert!(result.cost_analysis.government_fees >= dec!(10_000));
    }

    // ------------------------------------------------------------------
    // 19. Cost as pct of AUM
    // ------------------------------------------------------------------
    #[test]
    fn test_cost_pct_of_aum() {
        let input = hedge_fund_input();
        let result = analyze_cayman_structure(&input).unwrap();

        assert!(result.cost_analysis.cost_pct_of_aum > Decimal::ZERO);
        assert!(
            result.cost_analysis.cost_pct_of_aum < dec!(0.05),
            "Cost should be less than 5% of AUM"
        );
    }

    // ------------------------------------------------------------------
    // 20. Validation — empty fund name
    // ------------------------------------------------------------------
    #[test]
    fn test_validation_empty_fund_name() {
        let mut input = hedge_fund_input();
        input.fund_name = "".to_string();
        let result = analyze_cayman_structure(&input);
        assert!(result.is_err());

        match result.unwrap_err() {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "fund_name");
            }
            other => panic!("Expected InvalidInput, got: {other}"),
        }
    }

    // ------------------------------------------------------------------
    // 21. Validation — invalid structure type
    // ------------------------------------------------------------------
    #[test]
    fn test_validation_invalid_structure_type() {
        let mut input = hedge_fund_input();
        input.structure_type = "InvalidType".to_string();
        let result = analyze_cayman_structure(&input);
        assert!(result.is_err());
    }

    // ------------------------------------------------------------------
    // 22. Validation — invalid fund strategy
    // ------------------------------------------------------------------
    #[test]
    fn test_validation_invalid_fund_strategy() {
        let mut input = hedge_fund_input();
        input.fund_strategy = "Crypto".to_string();
        let result = analyze_cayman_structure(&input);
        assert!(result.is_err());
    }

    // ------------------------------------------------------------------
    // 23. Validation — negative fund size
    // ------------------------------------------------------------------
    #[test]
    fn test_validation_negative_fund_size() {
        let mut input = hedge_fund_input();
        input.fund_size = dec!(-100_000);
        let result = analyze_cayman_structure(&input);
        assert!(result.is_err());
    }

    // ------------------------------------------------------------------
    // 24. Validation — zero fund size
    // ------------------------------------------------------------------
    #[test]
    fn test_validation_zero_fund_size() {
        let mut input = hedge_fund_input();
        input.fund_size = Decimal::ZERO;
        let result = analyze_cayman_structure(&input);
        assert!(result.is_err());
    }

    // ------------------------------------------------------------------
    // 25. Validation — management fee rate out of range
    // ------------------------------------------------------------------
    #[test]
    fn test_validation_mgmt_fee_rate_high() {
        let mut input = hedge_fund_input();
        input.management_fee_rate = dec!(1.5);
        let result = analyze_cayman_structure(&input);
        assert!(result.is_err());
    }

    // ------------------------------------------------------------------
    // 26. Validation — negative performance fee rate
    // ------------------------------------------------------------------
    #[test]
    fn test_validation_negative_perf_fee() {
        let mut input = hedge_fund_input();
        input.performance_fee_rate = dec!(-0.10);
        let result = analyze_cayman_structure(&input);
        assert!(result.is_err());
    }

    // ------------------------------------------------------------------
    // 27. Validation — hurdle rate out of range
    // ------------------------------------------------------------------
    #[test]
    fn test_validation_hurdle_rate_high() {
        let mut input = hedge_fund_input();
        input.hurdle_rate = dec!(2.0);
        let result = analyze_cayman_structure(&input);
        assert!(result.is_err());
    }

    // ------------------------------------------------------------------
    // 28. Validation — invalid feeder type
    // ------------------------------------------------------------------
    #[test]
    fn test_validation_invalid_feeder_type() {
        let mut input = master_feeder_input();
        input.feeder_jurisdictions[0].feeder_type = "Jersey".to_string();
        let result = analyze_cayman_structure(&input);
        assert!(result.is_err());
    }

    // ------------------------------------------------------------------
    // 29. Validation — invalid investor profile
    // ------------------------------------------------------------------
    #[test]
    fn test_validation_invalid_investor_profile() {
        let mut input = master_feeder_input();
        input.feeder_jurisdictions[1].investor_profile = "RetailEU".to_string();
        let result = analyze_cayman_structure(&input);
        assert!(result.is_err());
    }

    // ------------------------------------------------------------------
    // 30. Validation — feeder alloc out of range
    // ------------------------------------------------------------------
    #[test]
    fn test_validation_feeder_alloc_out_of_range() {
        let mut input = master_feeder_input();
        input.feeder_jurisdictions[0].allocation_pct = dec!(1.5);
        let result = analyze_cayman_structure(&input);
        assert!(result.is_err());
    }

    // ------------------------------------------------------------------
    // 31. UnitTrust structure
    // ------------------------------------------------------------------
    #[test]
    fn test_unit_trust_structure() {
        let mut input = hedge_fund_input();
        input.structure_type = "UnitTrust".to_string();
        let result = analyze_cayman_structure(&input).unwrap();

        assert_eq!(result.structure_type, "UnitTrust");
        assert_eq!(result.structure_analysis.exemption_period_years, 50);
    }

    // ------------------------------------------------------------------
    // 32. LLC structure
    // ------------------------------------------------------------------
    #[test]
    fn test_llc_structure() {
        let mut input = hedge_fund_input();
        input.structure_type = "LLC".to_string();
        let result = analyze_cayman_structure(&input).unwrap();

        assert_eq!(result.structure_type, "LLC");
        assert!(result
            .structure_analysis
            .suitable_strategies
            .contains(&"Hedge".to_string()));
    }

    // ------------------------------------------------------------------
    // 33. High water mark warning for PE
    // ------------------------------------------------------------------
    #[test]
    fn test_hwm_warning_for_pe() {
        let mut input = pe_fund_input();
        input.high_water_mark = true;
        let result = analyze_cayman_structure(&input).unwrap();

        assert!(result
            .warnings
            .iter()
            .any(|w| w.contains("High water mark")));
    }

    // ------------------------------------------------------------------
    // 34. No HWM warning for hedge
    // ------------------------------------------------------------------
    #[test]
    fn test_no_hwm_warning_for_hedge() {
        let input = hedge_fund_input();
        let result = analyze_cayman_structure(&input).unwrap();

        assert!(!result
            .warnings
            .iter()
            .any(|w| w.contains("High water mark")));
    }

    // ------------------------------------------------------------------
    // 35. Strategy mismatch warning
    // ------------------------------------------------------------------
    #[test]
    fn test_strategy_mismatch_warning() {
        let mut input = hedge_fund_input();
        // ExemptedLP is not ideal for Hedge
        input.structure_type = "ExemptedLP".to_string();
        let result = analyze_cayman_structure(&input).unwrap();

        assert!(result
            .warnings
            .iter()
            .any(|w| w.contains("not typically associated")));
    }

    // ------------------------------------------------------------------
    // 36. Open-ended PE recommendation
    // ------------------------------------------------------------------
    #[test]
    fn test_open_ended_pe_recommendation() {
        let mut input = pe_fund_input();
        input.fund_term_years = None;
        let result = analyze_cayman_structure(&input).unwrap();

        assert!(result
            .recommendations
            .iter()
            .any(|r| r.contains("fund_term_years")));
    }

    // ------------------------------------------------------------------
    // 37. Feeder jurisdictions ignored when master_feeder false
    // ------------------------------------------------------------------
    #[test]
    fn test_feeder_data_ignored_when_not_master_feeder() {
        let mut input = hedge_fund_input();
        input.master_feeder = false;
        input.feeder_jurisdictions = vec![FeederInfo {
            jurisdiction: "Cayman Islands".to_string(),
            feeder_type: "Cayman".to_string(),
            allocation_pct: dec!(1.0),
            investor_profile: "NonUS".to_string(),
        }];
        let result = analyze_cayman_structure(&input).unwrap();

        assert!(result.master_feeder.is_none());
        assert!(result
            .warnings
            .iter()
            .any(|w| w.contains("feeder data ignored")));
    }

    // ------------------------------------------------------------------
    // 38. Feeder allocation warning (not summing to 100%)
    // ------------------------------------------------------------------
    #[test]
    fn test_feeder_allocation_warning() {
        let mut input = master_feeder_input();
        // Remove one feeder so they don't sum to 1.0
        input.feeder_jurisdictions.pop();
        let result = analyze_cayman_structure(&input).unwrap();

        assert!(result.warnings.iter().any(|w| w.contains("not 100%")));
    }

    // ------------------------------------------------------------------
    // 39. Net fee to manager
    // ------------------------------------------------------------------
    #[test]
    fn test_net_fee_to_manager() {
        let input = hedge_fund_input();
        let result = analyze_cayman_structure(&input).unwrap();

        let expected = result.fee_economics.gross_management_fee
            + result.fee_economics.gross_performance_fee
            - result.fee_economics.master_level_expenses
            - result.fee_economics.feeder_level_expenses;
        assert_eq!(result.fee_economics.net_fee_to_manager, expected);
    }

    // ------------------------------------------------------------------
    // 40. Service provider costs in cost analysis
    // ------------------------------------------------------------------
    #[test]
    fn test_service_provider_costs() {
        let input = hedge_fund_input();
        let result = analyze_cayman_structure(&input).unwrap();

        assert_eq!(
            result.cost_analysis.service_provider_costs,
            result.fee_economics.master_level_expenses
        );
    }

    // ------------------------------------------------------------------
    // 41. BVI regulatory — FSC registered
    // ------------------------------------------------------------------
    #[test]
    fn test_bvi_fsc_registered() {
        let mut input = hedge_fund_input();
        input.structure_type = "BVI_BCA".to_string();
        input.cima_registered = true;
        let result = analyze_cayman_structure(&input).unwrap();

        assert_eq!(
            result.regulatory.registration_category,
            "BVI FSC Registered Fund"
        );
        assert_eq!(result.regulatory.cima_annual_fee, dec!(1_500));
    }

    // ------------------------------------------------------------------
    // 42. BVI regulatory — unregistered
    // ------------------------------------------------------------------
    #[test]
    fn test_bvi_unregistered() {
        let mut input = hedge_fund_input();
        input.structure_type = "BVI_BCA".to_string();
        input.cima_registered = false;
        let result = analyze_cayman_structure(&input).unwrap();

        assert_eq!(
            result.regulatory.registration_category,
            "BVI Private Fund (unregistered)"
        );
        assert_eq!(result.regulatory.cima_annual_fee, dec!(350));
    }

    // ------------------------------------------------------------------
    // 43. Substance — BVI fewer requirements
    // ------------------------------------------------------------------
    #[test]
    fn test_substance_bvi_fewer_requirements() {
        let mut input = hedge_fund_input();
        input.structure_type = "BVI_BCA".to_string();
        let result = analyze_cayman_structure(&input).unwrap();

        assert_eq!(result.substance.local_directors_required, 1);
        assert_eq!(result.substance.board_meetings_required, 1);
    }

    // ------------------------------------------------------------------
    // 44. Substance — Cayman higher requirements
    // ------------------------------------------------------------------
    #[test]
    fn test_substance_cayman_higher_requirements() {
        let input = hedge_fund_input();
        let result = analyze_cayman_structure(&input).unwrap();

        assert_eq!(result.substance.local_directors_required, 2);
        assert_eq!(result.substance.board_meetings_required, 2);
    }

    // ------------------------------------------------------------------
    // 45. VC strategy with ExemptedLP
    // ------------------------------------------------------------------
    #[test]
    fn test_vc_exempted_lp() {
        let mut input = pe_fund_input();
        input.fund_strategy = "VC".to_string();
        let result = analyze_cayman_structure(&input).unwrap();

        assert!(result
            .structure_analysis
            .suitable_strategies
            .contains(&"VC".to_string()));
    }

    // ------------------------------------------------------------------
    // 46. FundOfFunds strategy
    // ------------------------------------------------------------------
    #[test]
    fn test_fund_of_funds_strategy() {
        let mut input = hedge_fund_input();
        input.fund_strategy = "FundOfFunds".to_string();
        let result = analyze_cayman_structure(&input).unwrap();

        assert_eq!(result.structure_type, "SPC");
        assert!(result
            .structure_analysis
            .suitable_strategies
            .contains(&"FundOfFunds".to_string()));
    }

    // ------------------------------------------------------------------
    // 47. RealEstate strategy
    // ------------------------------------------------------------------
    #[test]
    fn test_real_estate_strategy() {
        let mut input = pe_fund_input();
        input.fund_strategy = "RealEstate".to_string();
        let result = analyze_cayman_structure(&input).unwrap();

        assert!(result
            .structure_analysis
            .suitable_strategies
            .contains(&"RealEstate".to_string()));
    }

    // ------------------------------------------------------------------
    // 48. Credit strategy
    // ------------------------------------------------------------------
    #[test]
    fn test_credit_strategy() {
        let mut input = pe_fund_input();
        input.fund_strategy = "Credit".to_string();
        let result = analyze_cayman_structure(&input).unwrap();

        assert!(result
            .structure_analysis
            .suitable_strategies
            .contains(&"Credit".to_string()));
    }

    // ------------------------------------------------------------------
    // 49. Small fund cost scaling
    // ------------------------------------------------------------------
    #[test]
    fn test_small_fund_lower_costs() {
        let mut small = hedge_fund_input();
        small.fund_size = dec!(50_000_000);
        let small_result = analyze_cayman_structure(&small).unwrap();

        let mut large = hedge_fund_input();
        large.fund_size = dec!(1_000_000_000);
        let large_result = analyze_cayman_structure(&large).unwrap();

        assert!(
            small_result.cost_analysis.service_provider_costs
                < large_result.cost_analysis.service_provider_costs,
            "Small fund should have lower service provider costs"
        );
    }

    // ------------------------------------------------------------------
    // 50. Output serialization round-trip
    // ------------------------------------------------------------------
    #[test]
    fn test_output_serialization() {
        let input = hedge_fund_input();
        let result = analyze_cayman_structure(&input).unwrap();

        let json = serde_json::to_string(&result).unwrap();
        let deserialized: CaymanFundOutput = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.structure_type, result.structure_type);
        assert_eq!(deserialized.jurisdiction, result.jurisdiction);
        assert_eq!(
            deserialized.fee_economics.gross_management_fee,
            result.fee_economics.gross_management_fee
        );
    }
}
