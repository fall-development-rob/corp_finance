use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

use crate::error::CorpFinanceError;
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Types — Jersey
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JerseyFundInput {
    pub fund_name: String,
    /// "JPF", "ExpertFund", "ListedFund", "QIF"
    pub structure_type: String,
    /// "Hedge", "PE", "VC", "RealEstate", "Credit"
    pub fund_strategy: String,
    pub fund_size: Decimal,
    pub management_fee_rate: Decimal,
    pub performance_fee_rate: Decimal,
    pub investor_count: u32,
    pub jersey_directors_count: u32,
    pub local_admin: bool,
    pub aif_designation: bool,
    pub target_investors: Vec<String>,
}

// ---------------------------------------------------------------------------
// Types — Guernsey
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuernseyFundInput {
    pub fund_name: String,
    /// "PIF", "QIF", "RQIF", "AuthorisedFund"
    pub structure_type: String,
    /// "Hedge", "PE", "VC", "RealEstate", "Credit"
    pub fund_strategy: String,
    pub fund_size: Decimal,
    pub management_fee_rate: Decimal,
    pub performance_fee_rate: Decimal,
    pub investor_count: u32,
    pub guernsey_directors_count: u32,
    pub local_admin: bool,
    pub licensed_manager: bool,
    pub target_investors: Vec<String>,
}

// ---------------------------------------------------------------------------
// Types — Comparison
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelIslandsCompInput {
    pub fund_name: String,
    /// "Hedge", "PE", "VC", "RealEstate", "Credit"
    pub fund_strategy: String,
    pub fund_size: Decimal,
    pub investor_count: u32,
    pub require_eu_nppr: bool,
    pub require_retail: bool,
    /// Weights for comparison scoring (setup_cost, annual_cost, speed,
    /// investor_access, substance, regulatory_burden).
    /// Should sum to 1.0; defaults applied if empty.
    pub weights: Vec<Decimal>,
}

// ---------------------------------------------------------------------------
// Types — Cell Company
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CellInfo {
    pub cell_name: String,
    pub cell_aum: Decimal,
    pub expense_ratio: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CellCompanyInput {
    pub company_name: String,
    /// "PCC" or "ICC"
    pub cell_type: String,
    /// "Jersey" or "Guernsey"
    pub jurisdiction: String,
    pub core_aum: Decimal,
    pub cells: Vec<CellInfo>,
    pub core_annual_cost: Decimal,
    /// Allocation method for core costs: "EqualSplit" or "ProRataAUM"
    pub cost_allocation_method: String,
}

// ---------------------------------------------------------------------------
// Output types — Shared
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructureAnalysis {
    pub tax_status: String,
    pub liability_protection: String,
    pub suitable_strategies: Vec<String>,
    pub max_investors: Option<u32>,
    pub approval_timeline: String,
    pub minimum_investment: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegulatoryAnalysis {
    pub registration_category: String,
    pub regulator: String,
    pub annual_fee: Decimal,
    pub audit_required: bool,
    pub aml_handbook_applies: bool,
    pub minimum_investment: Decimal,
    pub approval_days: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubstanceAnalysis {
    pub substance_score: u32,
    pub ciga_met: bool,
    pub local_directors_required: u32,
    pub local_admin_required: bool,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostAnalysis {
    pub setup_cost_low: Decimal,
    pub setup_cost_high: Decimal,
    pub annual_cost_low: Decimal,
    pub annual_cost_high: Decimal,
    pub government_fees: Decimal,
    pub service_provider_costs: Decimal,
    pub total_annual_cost: Decimal,
    pub cost_pct_of_aum: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributionAnalysis {
    pub eu_nppr_available: Vec<String>,
    pub reverse_solicitation_risk: String,
    pub passport_available: bool,
    pub private_placement_jurisdictions: Vec<String>,
}

// ---------------------------------------------------------------------------
// Output types — Jersey
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JerseyFundOutput {
    pub fund_name: String,
    pub structure_type: String,
    pub jurisdiction: String,
    pub structure_analysis: StructureAnalysis,
    pub regulatory: RegulatoryAnalysis,
    pub substance: SubstanceAnalysis,
    pub cost_analysis: CostAnalysis,
    pub distribution: DistributionAnalysis,
    pub recommendations: Vec<String>,
    pub warnings: Vec<String>,
}

// ---------------------------------------------------------------------------
// Output types — Guernsey
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuernseyFundOutput {
    pub fund_name: String,
    pub structure_type: String,
    pub jurisdiction: String,
    pub structure_analysis: StructureAnalysis,
    pub regulatory: RegulatoryAnalysis,
    pub substance: SubstanceAnalysis,
    pub cost_analysis: CostAnalysis,
    pub distribution: DistributionAnalysis,
    pub recommendations: Vec<String>,
    pub warnings: Vec<String>,
}

// ---------------------------------------------------------------------------
// Output types — Comparison
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparisonEntry {
    pub jurisdiction: String,
    pub structure_type: String,
    pub setup_timeline_days: u32,
    pub setup_cost: Decimal,
    pub annual_cost: Decimal,
    pub investor_limit: Option<u32>,
    pub minimum_investment: Decimal,
    pub eu_nppr: bool,
    pub substance_score: u32,
    pub regulatory_burden_score: u32,
    pub weighted_score: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelIslandsCompOutput {
    pub fund_name: String,
    pub entries: Vec<ComparisonEntry>,
    pub recommended: String,
    pub recommendation_rationale: String,
}

// ---------------------------------------------------------------------------
// Output types — Cell Company
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CellEconomics {
    pub cell_name: String,
    pub cell_aum: Decimal,
    pub allocated_core_cost: Decimal,
    pub cell_direct_cost: Decimal,
    pub total_cell_cost: Decimal,
    pub cell_expense_ratio: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CellCompanyOutput {
    pub company_name: String,
    pub cell_type: String,
    pub jurisdiction: String,
    pub total_aum: Decimal,
    pub cell_count: u32,
    pub cells: Vec<CellEconomics>,
    pub total_annual_cost: Decimal,
    pub breakeven_cell_count: u32,
    pub pcc_vs_icc: String,
    pub recommendations: Vec<String>,
    pub warnings: Vec<String>,
}

// ---------------------------------------------------------------------------
// Public API — 1. Jersey Fund
// ---------------------------------------------------------------------------

pub fn analyze_jersey_fund(input: &JerseyFundInput) -> CorpFinanceResult<JerseyFundOutput> {
    validate_jersey_input(input)?;

    let mut warnings: Vec<String> = Vec::new();
    let mut recommendations: Vec<String> = Vec::new();

    let structure_analysis = build_jersey_structure(input, &mut recommendations, &mut warnings)?;

    let regulatory = build_jersey_regulatory(input, &mut recommendations, &mut warnings);

    let substance = build_jersey_substance(input, &mut recommendations);

    let cost_analysis = build_jersey_costs(input, &regulatory);

    let distribution = build_jersey_distribution(input, &mut recommendations);

    // Final recommendations
    if cost_analysis.cost_pct_of_aum > dec!(0.005) {
        warnings.push(format!(
            "Total annual cost is {:.2}% of AUM, above the typical 0.50% threshold",
            cost_analysis.cost_pct_of_aum * dec!(100)
        ));
    }

    if input.investor_count > 15 && input.structure_type == "JPF" {
        recommendations.push(
            "JPF with >15 investors: ensure designated service provider \
             (DSP) is appointed as required by JFSC"
                .to_string(),
        );
    }

    Ok(JerseyFundOutput {
        fund_name: input.fund_name.clone(),
        structure_type: input.structure_type.clone(),
        jurisdiction: "Jersey".to_string(),
        structure_analysis,
        regulatory,
        substance,
        cost_analysis,
        distribution,
        recommendations,
        warnings,
    })
}

// ---------------------------------------------------------------------------
// Public API — 2. Guernsey Fund
// ---------------------------------------------------------------------------

pub fn analyze_guernsey_fund(input: &GuernseyFundInput) -> CorpFinanceResult<GuernseyFundOutput> {
    validate_guernsey_input(input)?;

    let mut warnings: Vec<String> = Vec::new();
    let mut recommendations: Vec<String> = Vec::new();

    let structure_analysis = build_guernsey_structure(input, &mut recommendations, &mut warnings)?;

    let regulatory = build_guernsey_regulatory(input, &mut recommendations, &mut warnings);

    let substance = build_guernsey_substance(input, &mut recommendations);

    let cost_analysis = build_guernsey_costs(input, &regulatory);

    let distribution = build_guernsey_distribution(input, &mut recommendations);

    if cost_analysis.cost_pct_of_aum > dec!(0.005) {
        warnings.push(format!(
            "Total annual cost is {:.2}% of AUM, above the typical 0.50% threshold",
            cost_analysis.cost_pct_of_aum * dec!(100)
        ));
    }

    Ok(GuernseyFundOutput {
        fund_name: input.fund_name.clone(),
        structure_type: input.structure_type.clone(),
        jurisdiction: "Guernsey".to_string(),
        structure_analysis,
        regulatory,
        substance,
        cost_analysis,
        distribution,
        recommendations,
        warnings,
    })
}

// ---------------------------------------------------------------------------
// Public API — 3. Channel Islands Comparison
// ---------------------------------------------------------------------------

pub fn channel_islands_comparison(
    input: &ChannelIslandsCompInput,
) -> CorpFinanceResult<ChannelIslandsCompOutput> {
    validate_comparison_input(input)?;

    // Default weights: setup_cost, annual_cost, speed, investor_access,
    //                  substance, regulatory_burden
    let weights = if input.weights.len() == 6 {
        input.weights.clone()
    } else {
        vec![
            dec!(0.15), // setup_cost
            dec!(0.20), // annual_cost
            dec!(0.20), // speed
            dec!(0.15), // investor_access
            dec!(0.15), // substance
            dec!(0.15), // regulatory_burden
        ]
    };

    let candidates = [
        ComparisonCandidate {
            jurisdiction: "Jersey",
            structure_type: "JPF",
            setup_days: 2,
            setup_cost: dec!(15_000),
            annual_cost: dec!(60_000),
            investor_limit: Some(50),
            minimum_investment: Decimal::ZERO,
            eu_nppr: true,
        },
        ComparisonCandidate {
            jurisdiction: "Jersey",
            structure_type: "ExpertFund",
            setup_days: 5,
            setup_cost: dec!(25_000),
            annual_cost: dec!(80_000),
            investor_limit: None,
            minimum_investment: dec!(100_000),
            eu_nppr: true,
        },
        ComparisonCandidate {
            jurisdiction: "Guernsey",
            structure_type: "PIF",
            setup_days: 1,
            setup_cost: dec!(10_000),
            annual_cost: dec!(50_000),
            investor_limit: Some(50),
            minimum_investment: dec!(100_000),
            eu_nppr: true,
        },
        ComparisonCandidate {
            jurisdiction: "Guernsey",
            structure_type: "QIF",
            setup_days: 10,
            setup_cost: dec!(20_000),
            annual_cost: dec!(70_000),
            investor_limit: None,
            minimum_investment: dec!(100_000),
            eu_nppr: true,
        },
    ];

    let mut entries: Vec<ComparisonEntry> = candidates
        .iter()
        .map(|c| build_comparison_entry(c, input, &weights))
        .collect();

    // Sort by weighted score descending (higher = better)
    entries.sort_by(|a, b| b.weighted_score.cmp(&a.weighted_score));

    let recommended = format!("{} {}", entries[0].jurisdiction, entries[0].structure_type);
    let rationale = format!(
        "{} {} scores highest ({:.2}) based on weighted criteria: \
         cost efficiency, speed to market, investor access, substance, \
         and regulatory burden",
        entries[0].jurisdiction, entries[0].structure_type, entries[0].weighted_score
    );

    Ok(ChannelIslandsCompOutput {
        fund_name: input.fund_name.clone(),
        entries,
        recommended,
        recommendation_rationale: rationale,
    })
}

// ---------------------------------------------------------------------------
// Public API — 4. Cell Company Analysis
// ---------------------------------------------------------------------------

pub fn cell_company_analysis(input: &CellCompanyInput) -> CorpFinanceResult<CellCompanyOutput> {
    validate_cell_input(input)?;

    let mut warnings: Vec<String> = Vec::new();
    let mut recommendations: Vec<String> = Vec::new();

    let cell_count = input.cells.len() as u32;
    let total_cell_aum: Decimal = input.cells.iter().map(|c| c.cell_aum).sum();
    let total_aum = input.core_aum + total_cell_aum;

    let mut cells_out = Vec::new();
    let mut total_annual_cost = input.core_annual_cost;

    for cell in &input.cells {
        let allocated_core = match input.cost_allocation_method.as_str() {
            "EqualSplit" => {
                if cell_count > 0 {
                    input.core_annual_cost / Decimal::from(cell_count)
                } else {
                    Decimal::ZERO
                }
            }
            _ => {
                // ProRataAUM
                if total_cell_aum > Decimal::ZERO {
                    input.core_annual_cost * cell.cell_aum / total_cell_aum
                } else {
                    Decimal::ZERO
                }
            }
        };

        let cell_direct_cost = cell.cell_aum * cell.expense_ratio;
        let total_cell_cost = allocated_core + cell_direct_cost;
        let cell_expense_ratio = if cell.cell_aum > Decimal::ZERO {
            total_cell_cost / cell.cell_aum
        } else {
            Decimal::ZERO
        };

        total_annual_cost += cell_direct_cost;

        cells_out.push(CellEconomics {
            cell_name: cell.cell_name.clone(),
            cell_aum: cell.cell_aum,
            allocated_core_cost: allocated_core,
            cell_direct_cost,
            total_cell_cost,
            cell_expense_ratio,
        });
    }

    // Break-even: minimum cells where per-cell core allocation < standalone cost
    // Standalone fund setup ~ GBP 15,000/yr minimum; core cost splits at N cells
    let standalone_annual = dec!(15_000);
    let breakeven_cell_count = if input.core_annual_cost > Decimal::ZERO {
        let mut n = 1u32;
        loop {
            let per_cell = input.core_annual_cost / Decimal::from(n);
            if per_cell <= standalone_annual || n > 100 {
                break n;
            }
            n += 1;
        }
    } else {
        1
    };

    // PCC vs ICC comparison
    let pcc_vs_icc = if input.cell_type == "PCC" {
        if cell_count > 10 {
            recommendations.push(
                "Consider ICC for large number of cells: separate legal \
                 personality provides stronger ring-fencing"
                    .to_string(),
            );
        }
        "PCC: statutory ring-fencing of cell assets; cells do not have \
         separate legal personality. Lower cost but weaker segregation \
         than ICC."
            .to_string()
    } else {
        if cell_count < 3 {
            recommendations.push(
                "ICC may be over-engineered for fewer than 3 cells; \
                 consider PCC for cost efficiency"
                    .to_string(),
            );
        }
        "ICC: each cell is a separate legal entity with its own legal \
         personality. Stronger ring-fencing but higher formation and \
         ongoing costs than PCC."
            .to_string()
    };

    // Jurisdiction-specific notes
    match input.jurisdiction.as_str() {
        "Jersey" => {
            recommendations.push(
                "Jersey PCCs governed by Companies (Jersey) Law 1991; \
                 ICCs available under same framework"
                    .to_string(),
            );
        }
        "Guernsey" => {
            recommendations.push(
                "Guernsey PCCs governed by Companies (Guernsey) Law 2008; \
                 ICC available as Incorporated Cell Company"
                    .to_string(),
            );
        }
        _ => {}
    }

    if cell_count < breakeven_cell_count {
        warnings.push(format!(
            "Current cell count ({}) is below break-even ({}); \
             standalone funds may be more cost-effective",
            cell_count, breakeven_cell_count
        ));
    }

    Ok(CellCompanyOutput {
        company_name: input.company_name.clone(),
        cell_type: input.cell_type.clone(),
        jurisdiction: input.jurisdiction.clone(),
        total_aum,
        cell_count,
        cells: cells_out,
        total_annual_cost,
        breakeven_cell_count,
        pcc_vs_icc,
        recommendations,
        warnings,
    })
}

// ---------------------------------------------------------------------------
// Helpers — Jersey Structure
// ---------------------------------------------------------------------------

fn build_jersey_structure(
    input: &JerseyFundInput,
    recommendations: &mut Vec<String>,
    warnings: &mut Vec<String>,
) -> CorpFinanceResult<StructureAnalysis> {
    let (tax_status, liability, suitable, max_inv, timeline, min_inv) =
        match input.structure_type.as_str() {
            "JPF" => (
                "Jersey — no income tax, no capital gains tax, no GST on \
                 fund management fees"
                    .to_string(),
                "Limited partnership or company with limited liability".to_string(),
                vec![
                    "Hedge".to_string(),
                    "PE".to_string(),
                    "VC".to_string(),
                    "RealEstate".to_string(),
                    "Credit".to_string(),
                ],
                Some(50u32),
                "48 hours JFSC consent".to_string(),
                Decimal::ZERO,
            ),
            "ExpertFund" => (
                "Jersey — no income tax, no capital gains tax, no GST on \
                 fund management fees"
                    .to_string(),
                "Limited liability — professional investors only".to_string(),
                vec![
                    "Hedge".to_string(),
                    "PE".to_string(),
                    "VC".to_string(),
                    "Credit".to_string(),
                ],
                None,
                "5 business days JFSC approval".to_string(),
                dec!(100_000),
            ),
            "ListedFund" => (
                "Jersey — no income tax, no capital gains tax; listed on \
                 CISE or equivalent exchange"
                    .to_string(),
                "Listed vehicle — regulated by JFSC and exchange rules".to_string(),
                vec![
                    "RealEstate".to_string(),
                    "PE".to_string(),
                    "Credit".to_string(),
                ],
                None,
                "8 weeks JFSC + listing approval".to_string(),
                Decimal::ZERO,
            ),
            "QIF" => (
                "Jersey — no income tax, no capital gains tax; JFSC \
                 regulated qualifying investor fund"
                    .to_string(),
                "JFSC regulated — institutional investors only".to_string(),
                vec![
                    "Hedge".to_string(),
                    "PE".to_string(),
                    "VC".to_string(),
                    "RealEstate".to_string(),
                    "Credit".to_string(),
                ],
                None,
                "JFSC regulated — ongoing oversight".to_string(),
                Decimal::ZERO,
            ),
            other => {
                return Err(CorpFinanceError::InvalidInput {
                    field: "structure_type".into(),
                    reason: format!(
                        "Unknown Jersey structure '{}'. Valid: JPF, ExpertFund, \
                         ListedFund, QIF",
                        other
                    ),
                });
            }
        };

    if !suitable.contains(&input.fund_strategy) {
        warnings.push(format!(
            "Strategy '{}' is not typically associated with Jersey {} \
             structure; consider: {:?}",
            input.fund_strategy, input.structure_type, suitable
        ));
    }

    match input.fund_strategy.as_str() {
        "PE" | "VC" => {
            if input.structure_type != "JPF" {
                recommendations.push(
                    "JPF is the most popular Jersey structure for PE/VC \
                     funds due to fast 48h setup and flexible terms"
                        .to_string(),
                );
            }
        }
        "Hedge" => {
            if input.structure_type == "ListedFund" {
                recommendations.push(
                    "Listed funds are uncommon for hedge strategies; \
                     consider JPF or Expert Fund"
                        .to_string(),
                );
            }
        }
        _ => {}
    }

    Ok(StructureAnalysis {
        tax_status,
        liability_protection: liability,
        suitable_strategies: suitable,
        max_investors: max_inv,
        approval_timeline: timeline,
        minimum_investment: min_inv,
    })
}

// ---------------------------------------------------------------------------
// Helpers — Jersey Regulatory
// ---------------------------------------------------------------------------

fn build_jersey_regulatory(
    input: &JerseyFundInput,
    recommendations: &mut Vec<String>,
    _warnings: &mut [String],
) -> RegulatoryAnalysis {
    let (category, annual_fee, audit_req, approval_days, min_inv) =
        match input.structure_type.as_str() {
            "JPF" => (
                "Jersey Private Fund — JFSC consent".to_string(),
                dec!(1_580),
                true,
                2u32,
                Decimal::ZERO,
            ),
            "ExpertFund" => {
                recommendations.push(
                    "Expert Fund requires a Jersey-regulated functionary \
                     as designated service provider"
                        .to_string(),
                );
                (
                    "Expert Fund — JFSC regulated".to_string(),
                    dec!(3_420),
                    true,
                    5,
                    dec!(100_000),
                )
            }
            "ListedFund" => (
                "Listed Fund — JFSC + exchange listing".to_string(),
                dec!(5_780),
                true,
                56,
                Decimal::ZERO,
            ),
            "QIF" => (
                "Qualifying Investor Fund — JFSC full regulation".to_string(),
                dec!(4_620),
                true,
                10,
                Decimal::ZERO,
            ),
            _ => ("Unknown".to_string(), Decimal::ZERO, true, 0, Decimal::ZERO),
        };

    if input.aif_designation {
        recommendations.push(
            "AIF designation: ensure compliance with AIFMD-equivalent \
             Jersey requirements under JFSC Code of Practice"
                .to_string(),
        );
    }

    RegulatoryAnalysis {
        registration_category: category,
        regulator: "Jersey Financial Services Commission (JFSC)".to_string(),
        annual_fee,
        audit_required: audit_req,
        aml_handbook_applies: true,
        minimum_investment: min_inv,
        approval_days,
    }
}

// ---------------------------------------------------------------------------
// Helpers — Jersey Substance
// ---------------------------------------------------------------------------

fn build_jersey_substance(
    input: &JerseyFundInput,
    recommendations: &mut Vec<String>,
) -> SubstanceAnalysis {
    let mut score: u32 = 0;
    let mut recs: Vec<String> = Vec::new();

    // Dimension 1: Local directors (0-20 points)
    let local_directors_required = 2u32;
    let director_score = match input.jersey_directors_count {
        0 => {
            recs.push(
                "JFSC requires at least 2 Jersey-resident directors; \
                 appoint local directors"
                    .to_string(),
            );
            0
        }
        1 => {
            recs.push("Only 1 Jersey director; JFSC standard is minimum 2".to_string());
            10
        }
        _ => 20,
    };
    score += director_score;

    // Dimension 2: Local administration (0-20 points)
    let admin_score = if input.local_admin {
        20
    } else {
        recs.push(
            "Appoint a Jersey-based administrator for stronger \
             substance position"
                .to_string(),
        );
        0
    };
    score += admin_score;

    // Dimension 3: Decision-making in jurisdiction (0-20 points)
    // Proxy: if >= 2 local directors and local admin, decisions likely local
    let decision_score = if input.jersey_directors_count >= 2 && input.local_admin {
        20
    } else if input.jersey_directors_count >= 1 {
        10
    } else {
        0
    };
    score += decision_score;

    // Dimension 4: Expenditure in jurisdiction (0-20 points)
    // Proxy: fund size > 100M and local admin => significant local spend
    let expenditure_score = if input.fund_size >= dec!(100_000_000) && input.local_admin {
        20
    } else if input.fund_size >= dec!(50_000_000) {
        10
    } else {
        5
    };
    score += expenditure_score;

    // Dimension 5: CIGA (Core Income-Generating Activities) (0-20 points)
    let ciga_score = if input.jersey_directors_count >= 2
        && input.local_admin
        && input.fund_size >= dec!(50_000_000)
    {
        20
    } else if input.jersey_directors_count >= 1 && input.local_admin {
        10
    } else {
        recs.push(
            "Increase local substance to meet CIGA requirements: \
             local directors, admin, and decision-making"
                .to_string(),
        );
        0
    };
    score += ciga_score;

    let ciga_met = score >= 60;

    if !ciga_met {
        recs.push(
            "Substance score below 60/100 — consider increasing local \
             directors, administration, and expenditure"
                .to_string(),
        );
    }

    recommendations.append(&mut recs);

    SubstanceAnalysis {
        substance_score: score.min(100),
        ciga_met,
        local_directors_required,
        local_admin_required: true,
        recommendations: Vec::new(),
    }
}

// ---------------------------------------------------------------------------
// Helpers — Jersey Costs
// ---------------------------------------------------------------------------

fn build_jersey_costs(input: &JerseyFundInput, regulatory: &RegulatoryAnalysis) -> CostAnalysis {
    let (setup_low, setup_high, annual_low, annual_high) = match input.structure_type.as_str() {
        "JPF" => (dec!(15_000), dec!(25_000), dec!(60_000), dec!(90_000)),
        "ExpertFund" => (dec!(25_000), dec!(35_000), dec!(80_000), dec!(120_000)),
        "ListedFund" => (dec!(30_000), dec!(50_000), dec!(100_000), dec!(150_000)),
        "QIF" => (dec!(20_000), dec!(30_000), dec!(70_000), dec!(110_000)),
        _ => (dec!(15_000), dec!(30_000), dec!(60_000), dec!(120_000)),
    };

    let government_fees = regulatory.annual_fee;
    let service_provider_costs = estimate_ci_service_costs(input.fund_size);
    let total_annual_cost = government_fees + service_provider_costs;
    let cost_pct_of_aum = if input.fund_size > Decimal::ZERO {
        total_annual_cost / input.fund_size
    } else {
        Decimal::ZERO
    };

    CostAnalysis {
        setup_cost_low: setup_low,
        setup_cost_high: setup_high,
        annual_cost_low: annual_low,
        annual_cost_high: annual_high,
        government_fees,
        service_provider_costs,
        total_annual_cost,
        cost_pct_of_aum,
    }
}

// ---------------------------------------------------------------------------
// Helpers — Jersey Distribution
// ---------------------------------------------------------------------------

fn build_jersey_distribution(
    input: &JerseyFundInput,
    recommendations: &mut Vec<String>,
) -> DistributionAnalysis {
    // Jersey has NPPR agreements with most EU member states
    let eu_nppr = vec![
        "UK".to_string(),
        "Germany".to_string(),
        "France".to_string(),
        "Netherlands".to_string(),
        "Luxembourg".to_string(),
        "Ireland".to_string(),
        "Sweden".to_string(),
        "Denmark".to_string(),
        "Finland".to_string(),
    ];

    let reverse_solicitation_risk = if input.aif_designation {
        "Low — AIF designation enables structured NPPR marketing".to_string()
    } else {
        recommendations.push(
            "Without AIF designation, EU distribution relies on reverse \
             solicitation which carries regulatory risk"
                .to_string(),
        );
        "High — no AIF designation; reliance on reverse solicitation".to_string()
    };

    let private_placement = vec![
        "UK".to_string(),
        "US (Reg D/S)".to_string(),
        "Switzerland".to_string(),
        "Singapore".to_string(),
        "Hong Kong".to_string(),
    ];

    DistributionAnalysis {
        eu_nppr_available: eu_nppr,
        reverse_solicitation_risk,
        passport_available: false,
        private_placement_jurisdictions: private_placement,
    }
}

// ---------------------------------------------------------------------------
// Helpers — Guernsey Structure
// ---------------------------------------------------------------------------

fn build_guernsey_structure(
    input: &GuernseyFundInput,
    recommendations: &mut Vec<String>,
    warnings: &mut Vec<String>,
) -> CorpFinanceResult<StructureAnalysis> {
    let (tax_status, liability, suitable, max_inv, timeline, min_inv) =
        match input.structure_type.as_str() {
            "PIF" => (
                "Guernsey — 0% corporate tax rate; no capital gains tax, \
                 no VAT on fund management"
                    .to_string(),
                "Limited partnership or company with limited liability".to_string(),
                vec![
                    "Hedge".to_string(),
                    "PE".to_string(),
                    "VC".to_string(),
                    "RealEstate".to_string(),
                    "Credit".to_string(),
                ],
                Some(50u32),
                "1 business day GFSC registration".to_string(),
                dec!(100_000),
            ),
            "QIF" => (
                "Guernsey — 0% corporate tax rate; no capital gains tax".to_string(),
                "Licensed local manager required; institutional investors".to_string(),
                vec!["Hedge".to_string(), "PE".to_string(), "Credit".to_string()],
                None,
                "10 business days GFSC registration".to_string(),
                dec!(100_000),
            ),
            "RQIF" => (
                "Guernsey — 0% corporate tax rate; fast-track registration".to_string(),
                "Restricted — qualifying investors, GFSC-licensed manager".to_string(),
                vec![
                    "Hedge".to_string(),
                    "PE".to_string(),
                    "VC".to_string(),
                    "Credit".to_string(),
                ],
                None,
                "3 business days GFSC fast-track".to_string(),
                dec!(100_000),
            ),
            "AuthorisedFund" => (
                "Guernsey — 0% corporate tax rate; full GFSC authorization, \
                 retail-capable"
                    .to_string(),
                "Full GFSC regulation — retail investor protection \
                 requirements apply"
                    .to_string(),
                vec!["RealEstate".to_string(), "Credit".to_string()],
                None,
                "12 weeks full GFSC authorization".to_string(),
                Decimal::ZERO,
            ),
            other => {
                return Err(CorpFinanceError::InvalidInput {
                    field: "structure_type".into(),
                    reason: format!(
                        "Unknown Guernsey structure '{}'. Valid: PIF, QIF, \
                         RQIF, AuthorisedFund",
                        other
                    ),
                });
            }
        };

    if !suitable.contains(&input.fund_strategy) {
        warnings.push(format!(
            "Strategy '{}' is not typically associated with Guernsey {} \
             structure; consider: {:?}",
            input.fund_strategy, input.structure_type, suitable
        ));
    }

    match input.fund_strategy.as_str() {
        "PE" | "VC" => {
            if input.structure_type == "AuthorisedFund" {
                recommendations.push(
                    "Authorised Fund is uncommon for PE/VC; consider PIF \
                     or RQIF for faster setup and lower cost"
                        .to_string(),
                );
            }
        }
        "RealEstate" => {
            if input.structure_type == "AuthorisedFund" {
                recommendations.push(
                    "Authorised Fund with retail capability is suitable \
                     for listed real estate vehicles"
                        .to_string(),
                );
            }
        }
        _ => {}
    }

    Ok(StructureAnalysis {
        tax_status,
        liability_protection: liability,
        suitable_strategies: suitable,
        max_investors: max_inv,
        approval_timeline: timeline,
        minimum_investment: min_inv,
    })
}

// ---------------------------------------------------------------------------
// Helpers — Guernsey Regulatory
// ---------------------------------------------------------------------------

fn build_guernsey_regulatory(
    input: &GuernseyFundInput,
    recommendations: &mut Vec<String>,
    _warnings: &mut [String],
) -> RegulatoryAnalysis {
    let (category, annual_fee, audit_req, approval_days, min_inv) =
        match input.structure_type.as_str() {
            "PIF" => (
                "Private Investment Fund — GFSC registered".to_string(),
                dec!(1_300),
                true,
                1u32,
                dec!(100_000),
            ),
            "QIF" => {
                if !input.licensed_manager {
                    recommendations.push(
                        "QIF requires a Guernsey-licensed fund manager; \
                         appoint a local licensed manager"
                            .to_string(),
                    );
                }
                (
                    "Qualifying Investor Fund — GFSC registered".to_string(),
                    dec!(2_850),
                    true,
                    10,
                    dec!(100_000),
                )
            }
            "RQIF" => {
                if !input.licensed_manager {
                    recommendations.push(
                        "RQIF requires a GFSC-licensed manager for \
                         fast-track approval"
                            .to_string(),
                    );
                }
                (
                    "Registered Qualifying Investor Fund — GFSC fast-track".to_string(),
                    dec!(2_850),
                    true,
                    3,
                    dec!(100_000),
                )
            }
            "AuthorisedFund" => {
                recommendations.push(
                    "Authorised Fund: full GFSC review including investor \
                     protection, custody, and valuation rules"
                        .to_string(),
                );
                (
                    "Authorised Fund — full GFSC authorization".to_string(),
                    dec!(5_200),
                    true,
                    84,
                    Decimal::ZERO,
                )
            }
            _ => ("Unknown".to_string(), Decimal::ZERO, true, 0, Decimal::ZERO),
        };

    RegulatoryAnalysis {
        registration_category: category,
        regulator: "Guernsey Financial Services Commission (GFSC)".to_string(),
        annual_fee,
        audit_required: audit_req,
        aml_handbook_applies: true,
        minimum_investment: min_inv,
        approval_days,
    }
}

// ---------------------------------------------------------------------------
// Helpers — Guernsey Substance
// ---------------------------------------------------------------------------

fn build_guernsey_substance(
    input: &GuernseyFundInput,
    recommendations: &mut Vec<String>,
) -> SubstanceAnalysis {
    let mut score: u32 = 0;
    let mut recs: Vec<String> = Vec::new();

    let local_directors_required = 2u32;

    // Dimension 1: Local directors (0-20)
    let director_score = match input.guernsey_directors_count {
        0 => {
            recs.push("GFSC requires at least 2 Guernsey-resident directors".to_string());
            0
        }
        1 => {
            recs.push("Only 1 Guernsey director; standard requires minimum 2".to_string());
            10
        }
        _ => 20,
    };
    score += director_score;

    // Dimension 2: Local administration (0-20)
    let admin_score = if input.local_admin {
        20
    } else {
        recs.push("Appoint Guernsey-based administrator for substance".to_string());
        0
    };
    score += admin_score;

    // Dimension 3: Decision-making (0-20)
    let decision_score = if input.guernsey_directors_count >= 2 && input.local_admin {
        20
    } else if input.guernsey_directors_count >= 1 {
        10
    } else {
        0
    };
    score += decision_score;

    // Dimension 4: Expenditure (0-20)
    let expenditure_score = if input.fund_size >= dec!(100_000_000) && input.local_admin {
        20
    } else if input.fund_size >= dec!(50_000_000) {
        10
    } else {
        5
    };
    score += expenditure_score;

    // Dimension 5: CIGA (0-20)
    let ciga_score =
        if input.guernsey_directors_count >= 2 && input.local_admin && input.licensed_manager {
            20
        } else if input.guernsey_directors_count >= 1 && input.local_admin {
            10
        } else {
            recs.push(
                "Increase Guernsey substance: local directors, licensed \
             manager, and local admin"
                    .to_string(),
            );
            0
        };
    score += ciga_score;

    let ciga_met = score >= 60;

    if !ciga_met {
        recs.push("Substance score below 60/100 — increase local presence".to_string());
    }

    recommendations.append(&mut recs);

    SubstanceAnalysis {
        substance_score: score.min(100),
        ciga_met,
        local_directors_required,
        local_admin_required: true,
        recommendations: Vec::new(),
    }
}

// ---------------------------------------------------------------------------
// Helpers — Guernsey Costs
// ---------------------------------------------------------------------------

fn build_guernsey_costs(
    input: &GuernseyFundInput,
    regulatory: &RegulatoryAnalysis,
) -> CostAnalysis {
    let (setup_low, setup_high, annual_low, annual_high) = match input.structure_type.as_str() {
        "PIF" => (dec!(10_000), dec!(20_000), dec!(50_000), dec!(80_000)),
        "QIF" => (dec!(20_000), dec!(30_000), dec!(70_000), dec!(100_000)),
        "RQIF" => (dec!(18_000), dec!(28_000), dec!(65_000), dec!(95_000)),
        "AuthorisedFund" => (dec!(30_000), dec!(50_000), dec!(100_000), dec!(150_000)),
        _ => (dec!(15_000), dec!(30_000), dec!(60_000), dec!(120_000)),
    };

    let government_fees = regulatory.annual_fee;
    let service_provider_costs = estimate_ci_service_costs(input.fund_size);
    let total_annual_cost = government_fees + service_provider_costs;
    let cost_pct_of_aum = if input.fund_size > Decimal::ZERO {
        total_annual_cost / input.fund_size
    } else {
        Decimal::ZERO
    };

    CostAnalysis {
        setup_cost_low: setup_low,
        setup_cost_high: setup_high,
        annual_cost_low: annual_low,
        annual_cost_high: annual_high,
        government_fees,
        service_provider_costs,
        total_annual_cost,
        cost_pct_of_aum,
    }
}

// ---------------------------------------------------------------------------
// Helpers — Guernsey Distribution
// ---------------------------------------------------------------------------

fn build_guernsey_distribution(
    input: &GuernseyFundInput,
    recommendations: &mut Vec<String>,
) -> DistributionAnalysis {
    let eu_nppr = vec![
        "UK".to_string(),
        "Germany".to_string(),
        "France".to_string(),
        "Netherlands".to_string(),
        "Luxembourg".to_string(),
        "Ireland".to_string(),
        "Sweden".to_string(),
        "Denmark".to_string(),
    ];

    let reverse_solicitation_risk = if input.structure_type == "AuthorisedFund" {
        "Low — full GFSC authorization supports structured marketing".to_string()
    } else {
        recommendations.push(
            "For broad EU distribution, consider NPPR registration \
             in target member states"
                .to_string(),
        );
        "Medium — NPPR available but requires per-state registration".to_string()
    };

    let passport_available = input.structure_type == "AuthorisedFund";

    let private_placement = vec![
        "UK".to_string(),
        "US (Reg D/S)".to_string(),
        "Switzerland".to_string(),
        "Singapore".to_string(),
        "Hong Kong".to_string(),
    ];

    DistributionAnalysis {
        eu_nppr_available: eu_nppr,
        reverse_solicitation_risk,
        passport_available,
        private_placement_jurisdictions: private_placement,
    }
}

// ---------------------------------------------------------------------------
// Helpers — Comparison
// ---------------------------------------------------------------------------

struct ComparisonCandidate<'a> {
    jurisdiction: &'a str,
    structure_type: &'a str,
    setup_days: u32,
    setup_cost: Decimal,
    annual_cost: Decimal,
    investor_limit: Option<u32>,
    minimum_investment: Decimal,
    eu_nppr: bool,
}

fn build_comparison_entry(
    c: &ComparisonCandidate<'_>,
    input: &ChannelIslandsCompInput,
    weights: &[Decimal],
) -> ComparisonEntry {
    // Substance score heuristic (higher for Guernsey due to licensed manager req)
    let substance_score: u32 = match (c.jurisdiction, c.structure_type) {
        ("Jersey", "JPF") => 65,
        ("Jersey", "ExpertFund") => 70,
        ("Guernsey", "PIF") => 60,
        ("Guernsey", "QIF") => 75,
        _ => 60,
    };

    // Regulatory burden 1-5 (1 = lightest)
    let regulatory_burden: u32 = match (c.jurisdiction, c.structure_type) {
        ("Jersey", "JPF") => 1,
        ("Jersey", "ExpertFund") => 2,
        ("Guernsey", "PIF") => 1,
        ("Guernsey", "QIF") => 3,
        _ => 3,
    };

    // Weighted scoring: higher = better
    // Normalize each dimension to 0-1 scale, invert where lower is better

    // Setup cost score: lower cost = higher score
    let max_setup = dec!(30_000);
    let cost_score = (max_setup - c.setup_cost.min(max_setup)) / max_setup;

    // Annual cost score
    let max_annual = dec!(100_000);
    let annual_score = (max_annual - c.annual_cost.min(max_annual)) / max_annual;

    // Speed score: fewer days = higher score
    let max_days = dec!(60);
    let speed_score = (max_days - Decimal::from(c.setup_days).min(max_days)) / max_days;

    // Investor access: considers investor limit and retail flag
    let access_score = if input.require_retail {
        Decimal::ZERO // Only AuthorisedFund supports retail
    } else if let Some(limit) = c.investor_limit {
        if input.investor_count <= limit {
            dec!(0.8)
        } else {
            dec!(0.2)
        }
    } else {
        Decimal::ONE // No limit
    };

    // Substance score normalized to 0-1
    let substance_norm = Decimal::from(substance_score) / dec!(100);

    // Regulatory burden: lower = better
    let reg_score = (dec!(5) - Decimal::from(regulatory_burden)) / dec!(5);

    let weighted_score = weights[0] * cost_score
        + weights[1] * annual_score
        + weights[2] * speed_score
        + weights[3] * access_score
        + weights[4] * substance_norm
        + weights[5] * reg_score;

    ComparisonEntry {
        jurisdiction: c.jurisdiction.to_string(),
        structure_type: c.structure_type.to_string(),
        setup_timeline_days: c.setup_days,
        setup_cost: c.setup_cost,
        annual_cost: c.annual_cost,
        investor_limit: c.investor_limit,
        minimum_investment: c.minimum_investment,
        eu_nppr: c.eu_nppr,
        substance_score,
        regulatory_burden_score: regulatory_burden,
        weighted_score,
    }
}

// ---------------------------------------------------------------------------
// Helpers — Shared Cost Estimation
// ---------------------------------------------------------------------------

fn estimate_ci_service_costs(fund_size: Decimal) -> Decimal {
    // Channel Islands service provider costs (admin + audit + legal)
    let admin = if fund_size >= dec!(500_000_000) {
        dec!(120_000)
    } else if fund_size >= dec!(100_000_000) {
        dec!(80_000)
    } else if fund_size >= dec!(50_000_000) {
        dec!(55_000)
    } else {
        dec!(35_000)
    };

    let audit = if fund_size >= dec!(500_000_000) {
        dec!(75_000)
    } else if fund_size >= dec!(100_000_000) {
        dec!(50_000)
    } else {
        dec!(30_000)
    };

    let legal = if fund_size >= dec!(500_000_000) {
        dec!(60_000)
    } else if fund_size >= dec!(100_000_000) {
        dec!(40_000)
    } else {
        dec!(25_000)
    };

    admin + audit + legal
}

// ---------------------------------------------------------------------------
// Validation — Jersey
// ---------------------------------------------------------------------------

fn validate_jersey_input(input: &JerseyFundInput) -> CorpFinanceResult<()> {
    if input.fund_name.trim().is_empty() {
        return Err(CorpFinanceError::InvalidInput {
            field: "fund_name".into(),
            reason: "Fund name cannot be empty".into(),
        });
    }

    let valid_structures = ["JPF", "ExpertFund", "ListedFund", "QIF"];
    if !valid_structures.contains(&input.structure_type.as_str()) {
        return Err(CorpFinanceError::InvalidInput {
            field: "structure_type".into(),
            reason: format!(
                "Unknown Jersey structure '{}'. Valid: {:?}",
                input.structure_type, valid_structures
            ),
        });
    }

    let valid_strategies = ["Hedge", "PE", "VC", "RealEstate", "Credit"];
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

    // JPF: max 50 investors
    if input.structure_type == "JPF" && input.investor_count > 50 {
        return Err(CorpFinanceError::InvalidInput {
            field: "investor_count".into(),
            reason: format!(
                "JPF is limited to 50 investors; got {}",
                input.investor_count
            ),
        });
    }

    // Expert Fund: GBP 100k minimum per investor
    if input.structure_type == "ExpertFund" && input.investor_count > 0 {
        let per_investor = input.fund_size / Decimal::from(input.investor_count);
        if per_investor < dec!(100_000) {
            return Err(CorpFinanceError::InvalidInput {
                field: "fund_size".into(),
                reason: format!(
                    "Expert Fund requires GBP 100,000 minimum per investor; \
                     average is {:.0} with {} investors",
                    per_investor, input.investor_count
                ),
            });
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Validation — Guernsey
// ---------------------------------------------------------------------------

fn validate_guernsey_input(input: &GuernseyFundInput) -> CorpFinanceResult<()> {
    if input.fund_name.trim().is_empty() {
        return Err(CorpFinanceError::InvalidInput {
            field: "fund_name".into(),
            reason: "Fund name cannot be empty".into(),
        });
    }

    let valid_structures = ["PIF", "QIF", "RQIF", "AuthorisedFund"];
    if !valid_structures.contains(&input.structure_type.as_str()) {
        return Err(CorpFinanceError::InvalidInput {
            field: "structure_type".into(),
            reason: format!(
                "Unknown Guernsey structure '{}'. Valid: {:?}",
                input.structure_type, valid_structures
            ),
        });
    }

    let valid_strategies = ["Hedge", "PE", "VC", "RealEstate", "Credit"];
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

    // PIF: max 50 investors
    if input.structure_type == "PIF" && input.investor_count > 50 {
        return Err(CorpFinanceError::InvalidInput {
            field: "investor_count".into(),
            reason: format!(
                "PIF is limited to 50 investors; got {}",
                input.investor_count
            ),
        });
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Validation — Comparison
// ---------------------------------------------------------------------------

fn validate_comparison_input(input: &ChannelIslandsCompInput) -> CorpFinanceResult<()> {
    if input.fund_name.trim().is_empty() {
        return Err(CorpFinanceError::InvalidInput {
            field: "fund_name".into(),
            reason: "Fund name cannot be empty".into(),
        });
    }

    let valid_strategies = ["Hedge", "PE", "VC", "RealEstate", "Credit"];
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

    if !input.weights.is_empty() && input.weights.len() != 6 {
        return Err(CorpFinanceError::InvalidInput {
            field: "weights".into(),
            reason: format!(
                "Weights must have exactly 6 elements or be empty; got {}",
                input.weights.len()
            ),
        });
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Validation — Cell Company
// ---------------------------------------------------------------------------

fn validate_cell_input(input: &CellCompanyInput) -> CorpFinanceResult<()> {
    if input.company_name.trim().is_empty() {
        return Err(CorpFinanceError::InvalidInput {
            field: "company_name".into(),
            reason: "Company name cannot be empty".into(),
        });
    }

    let valid_types = ["PCC", "ICC"];
    if !valid_types.contains(&input.cell_type.as_str()) {
        return Err(CorpFinanceError::InvalidInput {
            field: "cell_type".into(),
            reason: format!("Unknown cell type '{}'. Valid: PCC, ICC", input.cell_type),
        });
    }

    let valid_jurisdictions = ["Jersey", "Guernsey"];
    if !valid_jurisdictions.contains(&input.jurisdiction.as_str()) {
        return Err(CorpFinanceError::InvalidInput {
            field: "jurisdiction".into(),
            reason: format!(
                "Unknown jurisdiction '{}'. Valid: Jersey, Guernsey",
                input.jurisdiction
            ),
        });
    }

    let valid_methods = ["EqualSplit", "ProRataAUM"];
    if !valid_methods.contains(&input.cost_allocation_method.as_str()) {
        return Err(CorpFinanceError::InvalidInput {
            field: "cost_allocation_method".into(),
            reason: format!(
                "Unknown method '{}'. Valid: EqualSplit, ProRataAUM",
                input.cost_allocation_method
            ),
        });
    }

    if input.cells.is_empty() {
        return Err(CorpFinanceError::InvalidInput {
            field: "cells".into(),
            reason: "At least one cell is required".into(),
        });
    }

    for (i, cell) in input.cells.iter().enumerate() {
        if cell.cell_aum <= Decimal::ZERO {
            return Err(CorpFinanceError::InvalidInput {
                field: format!("cells[{}].cell_aum", i),
                reason: "Cell AUM must be greater than zero".into(),
            });
        }
        if cell.expense_ratio < Decimal::ZERO || cell.expense_ratio >= Decimal::ONE {
            return Err(CorpFinanceError::InvalidInput {
                field: format!("cells[{}].expense_ratio", i),
                reason: "Expense ratio must be >= 0 and < 1".into(),
            });
        }
    }

    if input.core_annual_cost < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "core_annual_cost".into(),
            reason: "Core annual cost cannot be negative".into(),
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

    fn jersey_jpf_input() -> JerseyFundInput {
        JerseyFundInput {
            fund_name: "CI PE Fund I".to_string(),
            structure_type: "JPF".to_string(),
            fund_strategy: "PE".to_string(),
            fund_size: dec!(200_000_000),
            management_fee_rate: dec!(0.015),
            performance_fee_rate: dec!(0.20),
            investor_count: 25,
            jersey_directors_count: 2,
            local_admin: true,
            aif_designation: true,
            target_investors: vec!["Institutional".to_string()],
        }
    }

    fn jersey_expert_input() -> JerseyFundInput {
        JerseyFundInput {
            fund_name: "Expert Hedge Fund".to_string(),
            structure_type: "ExpertFund".to_string(),
            fund_strategy: "Hedge".to_string(),
            fund_size: dec!(500_000_000),
            management_fee_rate: dec!(0.02),
            performance_fee_rate: dec!(0.20),
            investor_count: 100,
            jersey_directors_count: 3,
            local_admin: true,
            aif_designation: true,
            target_investors: vec!["Professional".to_string()],
        }
    }

    fn guernsey_pif_input() -> GuernseyFundInput {
        GuernseyFundInput {
            fund_name: "Guernsey PE Fund".to_string(),
            structure_type: "PIF".to_string(),
            fund_strategy: "PE".to_string(),
            fund_size: dec!(150_000_000),
            management_fee_rate: dec!(0.015),
            performance_fee_rate: dec!(0.20),
            investor_count: 30,
            guernsey_directors_count: 2,
            local_admin: true,
            licensed_manager: true,
            target_investors: vec!["Institutional".to_string()],
        }
    }

    fn guernsey_qif_input() -> GuernseyFundInput {
        GuernseyFundInput {
            fund_name: "Guernsey QIF Hedge".to_string(),
            structure_type: "QIF".to_string(),
            fund_strategy: "Hedge".to_string(),
            fund_size: dec!(300_000_000),
            management_fee_rate: dec!(0.02),
            performance_fee_rate: dec!(0.20),
            investor_count: 80,
            guernsey_directors_count: 2,
            local_admin: true,
            licensed_manager: true,
            target_investors: vec!["Institutional".to_string()],
        }
    }

    fn comparison_input() -> ChannelIslandsCompInput {
        ChannelIslandsCompInput {
            fund_name: "CI Comparison Fund".to_string(),
            fund_strategy: "PE".to_string(),
            fund_size: dec!(200_000_000),
            investor_count: 25,
            require_eu_nppr: true,
            require_retail: false,
            weights: vec![],
        }
    }

    fn cell_pcc_input() -> CellCompanyInput {
        CellCompanyInput {
            company_name: "Jersey PCC Ltd".to_string(),
            cell_type: "PCC".to_string(),
            jurisdiction: "Jersey".to_string(),
            core_aum: dec!(50_000_000),
            cells: vec![
                CellInfo {
                    cell_name: "Cell A".to_string(),
                    cell_aum: dec!(100_000_000),
                    expense_ratio: dec!(0.005),
                },
                CellInfo {
                    cell_name: "Cell B".to_string(),
                    cell_aum: dec!(75_000_000),
                    expense_ratio: dec!(0.006),
                },
                CellInfo {
                    cell_name: "Cell C".to_string(),
                    cell_aum: dec!(50_000_000),
                    expense_ratio: dec!(0.008),
                },
            ],
            core_annual_cost: dec!(60_000),
            cost_allocation_method: "ProRataAUM".to_string(),
        }
    }

    // ======================================================================
    // Jersey Fund — Structure Tests
    // ======================================================================

    #[test]
    fn test_jersey_jpf_basic() {
        let input = jersey_jpf_input();
        let result = analyze_jersey_fund(&input).unwrap();
        assert_eq!(result.structure_type, "JPF");
        assert_eq!(result.jurisdiction, "Jersey");
        assert!(result
            .structure_analysis
            .suitable_strategies
            .contains(&"PE".to_string()));
        assert_eq!(result.structure_analysis.max_investors, Some(50));
    }

    #[test]
    fn test_jersey_expert_fund() {
        let input = jersey_expert_input();
        let result = analyze_jersey_fund(&input).unwrap();
        assert_eq!(result.structure_type, "ExpertFund");
        assert_eq!(result.structure_analysis.minimum_investment, dec!(100_000));
    }

    #[test]
    fn test_jersey_listed_fund() {
        let input = JerseyFundInput {
            fund_name: "Listed RE Fund".to_string(),
            structure_type: "ListedFund".to_string(),
            fund_strategy: "RealEstate".to_string(),
            fund_size: dec!(300_000_000),
            management_fee_rate: dec!(0.01),
            performance_fee_rate: dec!(0.15),
            investor_count: 200,
            jersey_directors_count: 3,
            local_admin: true,
            aif_designation: false,
            target_investors: vec![],
        };
        let result = analyze_jersey_fund(&input).unwrap();
        assert_eq!(result.structure_type, "ListedFund");
        assert!(result
            .structure_analysis
            .approval_timeline
            .contains("8 weeks"));
    }

    #[test]
    fn test_jersey_qif() {
        let input = JerseyFundInput {
            fund_name: "QIF Fund".to_string(),
            structure_type: "QIF".to_string(),
            fund_strategy: "Credit".to_string(),
            fund_size: dec!(100_000_000),
            management_fee_rate: dec!(0.01),
            performance_fee_rate: dec!(0.15),
            investor_count: 20,
            jersey_directors_count: 2,
            local_admin: true,
            aif_designation: true,
            target_investors: vec![],
        };
        let result = analyze_jersey_fund(&input).unwrap();
        assert_eq!(result.structure_type, "QIF");
        assert_eq!(
            result.regulatory.regulator,
            "Jersey Financial Services Commission (JFSC)"
        );
    }

    // ======================================================================
    // Jersey Fund — Regulatory Tests
    // ======================================================================

    #[test]
    fn test_jersey_jpf_regulatory_fee() {
        let input = jersey_jpf_input();
        let result = analyze_jersey_fund(&input).unwrap();
        assert_eq!(result.regulatory.annual_fee, dec!(1_580));
        assert!(result.regulatory.audit_required);
        assert!(result.regulatory.aml_handbook_applies);
        assert_eq!(result.regulatory.approval_days, 2);
    }

    #[test]
    fn test_jersey_expert_regulatory_fee() {
        let input = jersey_expert_input();
        let result = analyze_jersey_fund(&input).unwrap();
        assert_eq!(result.regulatory.annual_fee, dec!(3_420));
        assert_eq!(result.regulatory.minimum_investment, dec!(100_000));
    }

    // ======================================================================
    // Jersey Fund — Substance Tests
    // ======================================================================

    #[test]
    fn test_jersey_full_substance() {
        let input = jersey_jpf_input();
        let result = analyze_jersey_fund(&input).unwrap();
        // 2 directors (20) + local admin (20) + decision (20) +
        // expenditure (20, >100M + admin) + CIGA (20, 2 dirs + admin + >50M)
        assert_eq!(result.substance.substance_score, 100);
        assert!(result.substance.ciga_met);
    }

    #[test]
    fn test_jersey_no_substance() {
        let mut input = jersey_jpf_input();
        input.jersey_directors_count = 0;
        input.local_admin = false;
        input.fund_size = dec!(10_000_000);
        let result = analyze_jersey_fund(&input).unwrap();
        assert!(result.substance.substance_score < 60);
        assert!(!result.substance.ciga_met);
    }

    #[test]
    fn test_jersey_partial_substance() {
        let mut input = jersey_jpf_input();
        input.jersey_directors_count = 1;
        input.local_admin = false;
        input.fund_size = dec!(30_000_000);
        let result = analyze_jersey_fund(&input).unwrap();
        // 1 director (10) + no admin (0) + decision (10) +
        // expenditure (5, <50M) + CIGA (0)
        assert_eq!(result.substance.substance_score, 25);
        assert!(!result.substance.ciga_met);
    }

    // ======================================================================
    // Jersey Fund — Cost Tests
    // ======================================================================

    #[test]
    fn test_jersey_jpf_costs() {
        let input = jersey_jpf_input();
        let result = analyze_jersey_fund(&input).unwrap();
        assert_eq!(result.cost_analysis.setup_cost_low, dec!(15_000));
        assert_eq!(result.cost_analysis.setup_cost_high, dec!(25_000));
        assert!(result.cost_analysis.total_annual_cost > Decimal::ZERO);
        assert!(result.cost_analysis.cost_pct_of_aum > Decimal::ZERO);
    }

    #[test]
    fn test_jersey_cost_pct_large_fund() {
        let mut input = jersey_jpf_input();
        input.fund_size = dec!(1_000_000_000);
        let result = analyze_jersey_fund(&input).unwrap();
        // Large fund should have low cost % of AUM
        assert!(result.cost_analysis.cost_pct_of_aum < dec!(0.005));
    }

    // ======================================================================
    // Jersey Fund — Distribution Tests
    // ======================================================================

    #[test]
    fn test_jersey_distribution_nppr() {
        let input = jersey_jpf_input();
        let result = analyze_jersey_fund(&input).unwrap();
        assert!(!result.distribution.eu_nppr_available.is_empty());
        assert!(result
            .distribution
            .eu_nppr_available
            .contains(&"UK".to_string()));
        assert!(!result.distribution.passport_available);
    }

    #[test]
    fn test_jersey_distribution_no_aif() {
        let mut input = jersey_jpf_input();
        input.aif_designation = false;
        let result = analyze_jersey_fund(&input).unwrap();
        assert!(result
            .distribution
            .reverse_solicitation_risk
            .contains("High"));
    }

    // ======================================================================
    // Jersey Fund — Validation Tests
    // ======================================================================

    #[test]
    fn test_jersey_empty_name_rejected() {
        let mut input = jersey_jpf_input();
        input.fund_name = "".to_string();
        let err = analyze_jersey_fund(&input).unwrap_err();
        assert!(err.to_string().contains("fund_name"));
    }

    #[test]
    fn test_jersey_invalid_structure_rejected() {
        let mut input = jersey_jpf_input();
        input.structure_type = "InvalidType".to_string();
        let err = analyze_jersey_fund(&input).unwrap_err();
        assert!(err.to_string().contains("structure_type"));
    }

    #[test]
    fn test_jersey_invalid_strategy_rejected() {
        let mut input = jersey_jpf_input();
        input.fund_strategy = "Crypto".to_string();
        let err = analyze_jersey_fund(&input).unwrap_err();
        assert!(err.to_string().contains("fund_strategy"));
    }

    #[test]
    fn test_jersey_zero_fund_size_rejected() {
        let mut input = jersey_jpf_input();
        input.fund_size = Decimal::ZERO;
        let err = analyze_jersey_fund(&input).unwrap_err();
        assert!(err.to_string().contains("fund_size"));
    }

    #[test]
    fn test_jersey_negative_fee_rejected() {
        let mut input = jersey_jpf_input();
        input.management_fee_rate = dec!(-0.01);
        let err = analyze_jersey_fund(&input).unwrap_err();
        assert!(err.to_string().contains("management_fee_rate"));
    }

    #[test]
    fn test_jersey_fee_rate_one_rejected() {
        let mut input = jersey_jpf_input();
        input.performance_fee_rate = Decimal::ONE;
        let err = analyze_jersey_fund(&input).unwrap_err();
        assert!(err.to_string().contains("performance_fee_rate"));
    }

    #[test]
    fn test_jersey_jpf_over_50_investors_rejected() {
        let mut input = jersey_jpf_input();
        input.investor_count = 51;
        let err = analyze_jersey_fund(&input).unwrap_err();
        assert!(err.to_string().contains("50 investors"));
    }

    #[test]
    fn test_jersey_jpf_50_investors_accepted() {
        let mut input = jersey_jpf_input();
        input.investor_count = 50;
        let result = analyze_jersey_fund(&input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_jersey_expert_low_per_investor_rejected() {
        let mut input = jersey_expert_input();
        input.fund_size = dec!(5_000_000); // 5M / 100 investors = 50k < 100k
        let err = analyze_jersey_fund(&input).unwrap_err();
        assert!(err.to_string().contains("100,000"));
    }

    // ======================================================================
    // Guernsey Fund — Structure Tests
    // ======================================================================

    #[test]
    fn test_guernsey_pif_basic() {
        let input = guernsey_pif_input();
        let result = analyze_guernsey_fund(&input).unwrap();
        assert_eq!(result.structure_type, "PIF");
        assert_eq!(result.jurisdiction, "Guernsey");
        assert_eq!(result.structure_analysis.max_investors, Some(50));
        assert!(result
            .structure_analysis
            .approval_timeline
            .contains("1 business day"));
    }

    #[test]
    fn test_guernsey_qif() {
        let input = guernsey_qif_input();
        let result = analyze_guernsey_fund(&input).unwrap();
        assert_eq!(result.structure_type, "QIF");
        assert_eq!(result.structure_analysis.minimum_investment, dec!(100_000));
    }

    #[test]
    fn test_guernsey_rqif() {
        let input = GuernseyFundInput {
            fund_name: "RQIF Fund".to_string(),
            structure_type: "RQIF".to_string(),
            fund_strategy: "VC".to_string(),
            fund_size: dec!(100_000_000),
            management_fee_rate: dec!(0.02),
            performance_fee_rate: dec!(0.20),
            investor_count: 40,
            guernsey_directors_count: 2,
            local_admin: true,
            licensed_manager: true,
            target_investors: vec![],
        };
        let result = analyze_guernsey_fund(&input).unwrap();
        assert_eq!(result.structure_type, "RQIF");
        assert!(result
            .structure_analysis
            .approval_timeline
            .contains("3 business days"));
    }

    #[test]
    fn test_guernsey_authorised_fund() {
        let input = GuernseyFundInput {
            fund_name: "Authorised RE Fund".to_string(),
            structure_type: "AuthorisedFund".to_string(),
            fund_strategy: "RealEstate".to_string(),
            fund_size: dec!(500_000_000),
            management_fee_rate: dec!(0.01),
            performance_fee_rate: dec!(0.15),
            investor_count: 500,
            guernsey_directors_count: 3,
            local_admin: true,
            licensed_manager: true,
            target_investors: vec![],
        };
        let result = analyze_guernsey_fund(&input).unwrap();
        assert_eq!(result.structure_type, "AuthorisedFund");
        assert!(result
            .structure_analysis
            .approval_timeline
            .contains("12 weeks"));
        assert_eq!(result.structure_analysis.minimum_investment, Decimal::ZERO);
        assert!(result.distribution.passport_available);
    }

    // ======================================================================
    // Guernsey Fund — Regulatory Tests
    // ======================================================================

    #[test]
    fn test_guernsey_pif_regulatory() {
        let input = guernsey_pif_input();
        let result = analyze_guernsey_fund(&input).unwrap();
        assert_eq!(result.regulatory.annual_fee, dec!(1_300));
        assert_eq!(result.regulatory.approval_days, 1);
        assert!(result.regulatory.aml_handbook_applies);
    }

    #[test]
    fn test_guernsey_qif_no_manager_warning() {
        let mut input = guernsey_qif_input();
        input.licensed_manager = false;
        let result = analyze_guernsey_fund(&input).unwrap();
        assert!(result
            .recommendations
            .iter()
            .any(|r| r.contains("licensed manager")));
    }

    #[test]
    fn test_guernsey_authorised_regulatory_fee() {
        let input = GuernseyFundInput {
            fund_name: "Auth Fund".to_string(),
            structure_type: "AuthorisedFund".to_string(),
            fund_strategy: "RealEstate".to_string(),
            fund_size: dec!(200_000_000),
            management_fee_rate: dec!(0.01),
            performance_fee_rate: dec!(0.10),
            investor_count: 100,
            guernsey_directors_count: 2,
            local_admin: true,
            licensed_manager: true,
            target_investors: vec![],
        };
        let result = analyze_guernsey_fund(&input).unwrap();
        assert_eq!(result.regulatory.annual_fee, dec!(5_200));
        assert_eq!(result.regulatory.approval_days, 84);
    }

    // ======================================================================
    // Guernsey Fund — Substance Tests
    // ======================================================================

    #[test]
    fn test_guernsey_full_substance() {
        let input = guernsey_pif_input();
        let result = analyze_guernsey_fund(&input).unwrap();
        // 2 dirs (20) + admin (20) + decision (20) + expenditure (20) + CIGA (20)
        assert_eq!(result.substance.substance_score, 100);
        assert!(result.substance.ciga_met);
    }

    #[test]
    fn test_guernsey_no_substance() {
        let mut input = guernsey_pif_input();
        input.guernsey_directors_count = 0;
        input.local_admin = false;
        input.licensed_manager = false;
        input.fund_size = dec!(10_000_000);
        let result = analyze_guernsey_fund(&input).unwrap();
        assert!(result.substance.substance_score < 60);
        assert!(!result.substance.ciga_met);
    }

    // ======================================================================
    // Guernsey Fund — Cost Tests
    // ======================================================================

    #[test]
    fn test_guernsey_pif_cheapest_setup() {
        let input = guernsey_pif_input();
        let result = analyze_guernsey_fund(&input).unwrap();
        assert_eq!(result.cost_analysis.setup_cost_low, dec!(10_000));
        assert_eq!(result.cost_analysis.setup_cost_high, dec!(20_000));
    }

    #[test]
    fn test_guernsey_costs_positive() {
        let input = guernsey_qif_input();
        let result = analyze_guernsey_fund(&input).unwrap();
        assert!(result.cost_analysis.total_annual_cost > Decimal::ZERO);
        assert!(result.cost_analysis.government_fees > Decimal::ZERO);
        assert!(result.cost_analysis.service_provider_costs > Decimal::ZERO);
    }

    // ======================================================================
    // Guernsey Fund — Validation Tests
    // ======================================================================

    #[test]
    fn test_guernsey_empty_name_rejected() {
        let mut input = guernsey_pif_input();
        input.fund_name = "  ".to_string();
        let err = analyze_guernsey_fund(&input).unwrap_err();
        assert!(err.to_string().contains("fund_name"));
    }

    #[test]
    fn test_guernsey_invalid_structure_rejected() {
        let mut input = guernsey_pif_input();
        input.structure_type = "SPC".to_string();
        let err = analyze_guernsey_fund(&input).unwrap_err();
        assert!(err.to_string().contains("structure_type"));
    }

    #[test]
    fn test_guernsey_pif_over_50_rejected() {
        let mut input = guernsey_pif_input();
        input.investor_count = 51;
        let err = analyze_guernsey_fund(&input).unwrap_err();
        assert!(err.to_string().contains("50 investors"));
    }

    #[test]
    fn test_guernsey_negative_fund_size_rejected() {
        let mut input = guernsey_pif_input();
        input.fund_size = dec!(-1);
        let err = analyze_guernsey_fund(&input).unwrap_err();
        assert!(err.to_string().contains("fund_size"));
    }

    // ======================================================================
    // Channel Islands Comparison Tests
    // ======================================================================

    #[test]
    fn test_comparison_basic() {
        let input = comparison_input();
        let result = channel_islands_comparison(&input).unwrap();
        assert_eq!(result.entries.len(), 4);
        assert!(!result.recommended.is_empty());
    }

    #[test]
    fn test_comparison_sorted_by_score() {
        let input = comparison_input();
        let result = channel_islands_comparison(&input).unwrap();
        for i in 0..result.entries.len() - 1 {
            assert!(result.entries[i].weighted_score >= result.entries[i + 1].weighted_score);
        }
    }

    #[test]
    fn test_comparison_contains_all_structures() {
        let input = comparison_input();
        let result = channel_islands_comparison(&input).unwrap();
        let types: Vec<&str> = result
            .entries
            .iter()
            .map(|e| e.structure_type.as_str())
            .collect();
        assert!(types.contains(&"JPF"));
        assert!(types.contains(&"ExpertFund"));
        assert!(types.contains(&"PIF"));
        assert!(types.contains(&"QIF"));
    }

    #[test]
    fn test_comparison_custom_weights() {
        let mut input = comparison_input();
        // Heavily weight speed
        input.weights = vec![
            dec!(0.0),
            dec!(0.0),
            dec!(1.0),
            dec!(0.0),
            dec!(0.0),
            dec!(0.0),
        ];
        let result = channel_islands_comparison(&input).unwrap();
        // PIF has 1-day setup, should rank first with pure speed weight
        assert_eq!(result.entries[0].structure_type, "PIF");
    }

    #[test]
    fn test_comparison_invalid_strategy() {
        let mut input = comparison_input();
        input.fund_strategy = "Unknown".to_string();
        let err = channel_islands_comparison(&input).unwrap_err();
        assert!(err.to_string().contains("fund_strategy"));
    }

    #[test]
    fn test_comparison_invalid_weights_length() {
        let mut input = comparison_input();
        input.weights = vec![dec!(0.5), dec!(0.5)]; // only 2
        let err = channel_islands_comparison(&input).unwrap_err();
        assert!(err.to_string().contains("weights"));
    }

    #[test]
    fn test_comparison_zero_fund_size_rejected() {
        let mut input = comparison_input();
        input.fund_size = Decimal::ZERO;
        let err = channel_islands_comparison(&input).unwrap_err();
        assert!(err.to_string().contains("fund_size"));
    }

    // ======================================================================
    // Cell Company Tests
    // ======================================================================

    #[test]
    fn test_cell_pcc_basic() {
        let input = cell_pcc_input();
        let result = cell_company_analysis(&input).unwrap();
        assert_eq!(result.cell_type, "PCC");
        assert_eq!(result.jurisdiction, "Jersey");
        assert_eq!(result.cell_count, 3);
        assert!(result.total_aum > Decimal::ZERO);
    }

    #[test]
    fn test_cell_pcc_pro_rata_allocation() {
        let input = cell_pcc_input();
        let result = cell_company_analysis(&input).unwrap();
        // Cell A: 100M out of 225M total cell AUM
        let cell_a = &result.cells[0];
        let expected_alloc = dec!(60_000) * dec!(100_000_000) / dec!(225_000_000);
        assert_eq!(cell_a.allocated_core_cost, expected_alloc);
    }

    #[test]
    fn test_cell_pcc_equal_split() {
        let mut input = cell_pcc_input();
        input.cost_allocation_method = "EqualSplit".to_string();
        let result = cell_company_analysis(&input).unwrap();
        let expected_per_cell = dec!(60_000) / dec!(3);
        for cell in &result.cells {
            assert_eq!(cell.allocated_core_cost, expected_per_cell);
        }
    }

    #[test]
    fn test_cell_icc() {
        let mut input = cell_pcc_input();
        input.cell_type = "ICC".to_string();
        let result = cell_company_analysis(&input).unwrap();
        assert_eq!(result.cell_type, "ICC");
        assert!(result.pcc_vs_icc.contains("ICC"));
        assert!(result.pcc_vs_icc.contains("separate legal entity"));
    }

    #[test]
    fn test_cell_breakeven() {
        let input = cell_pcc_input();
        let result = cell_company_analysis(&input).unwrap();
        // core_annual_cost = 60,000; standalone = 15,000
        // breakeven: 60,000 / N <= 15,000 => N >= 4
        assert_eq!(result.breakeven_cell_count, 4);
    }

    #[test]
    fn test_cell_below_breakeven_warning() {
        let mut input = cell_pcc_input();
        input.core_annual_cost = dec!(100_000);
        // 100,000 / 3 = 33,333 > 15,000; breakeven = 7
        let result = cell_company_analysis(&input).unwrap();
        assert!(result.warnings.iter().any(|w| w.contains("break-even")));
    }

    #[test]
    fn test_cell_guernsey_jurisdiction() {
        let mut input = cell_pcc_input();
        input.jurisdiction = "Guernsey".to_string();
        let result = cell_company_analysis(&input).unwrap();
        assert_eq!(result.jurisdiction, "Guernsey");
        assert!(result
            .recommendations
            .iter()
            .any(|r| r.contains("Guernsey")));
    }

    #[test]
    fn test_cell_expense_ratio() {
        let input = cell_pcc_input();
        let result = cell_company_analysis(&input).unwrap();
        for cell in &result.cells {
            assert!(cell.cell_expense_ratio > Decimal::ZERO);
            assert!(cell.cell_expense_ratio < Decimal::ONE);
        }
    }

    #[test]
    fn test_cell_many_cells_icc_recommendation() {
        let mut input = cell_pcc_input();
        // Add cells to exceed 10
        for i in 0..10 {
            input.cells.push(CellInfo {
                cell_name: format!("Cell {}", i + 4),
                cell_aum: dec!(20_000_000),
                expense_ratio: dec!(0.005),
            });
        }
        let result = cell_company_analysis(&input).unwrap();
        assert!(result.recommendations.iter().any(|r| r.contains("ICC")));
    }

    #[test]
    fn test_cell_few_cells_icc_warning() {
        let mut input = cell_pcc_input();
        input.cell_type = "ICC".to_string();
        input.cells = vec![CellInfo {
            cell_name: "Single Cell".to_string(),
            cell_aum: dec!(50_000_000),
            expense_ratio: dec!(0.005),
        }];
        let result = cell_company_analysis(&input).unwrap();
        assert!(result.recommendations.iter().any(|r| r.contains("PCC")));
    }

    // ======================================================================
    // Cell Company — Validation Tests
    // ======================================================================

    #[test]
    fn test_cell_empty_name_rejected() {
        let mut input = cell_pcc_input();
        input.company_name = "".to_string();
        let err = cell_company_analysis(&input).unwrap_err();
        assert!(err.to_string().contains("company_name"));
    }

    #[test]
    fn test_cell_invalid_type_rejected() {
        let mut input = cell_pcc_input();
        input.cell_type = "SPC".to_string();
        let err = cell_company_analysis(&input).unwrap_err();
        assert!(err.to_string().contains("cell_type"));
    }

    #[test]
    fn test_cell_invalid_jurisdiction_rejected() {
        let mut input = cell_pcc_input();
        input.jurisdiction = "Cayman".to_string();
        let err = cell_company_analysis(&input).unwrap_err();
        assert!(err.to_string().contains("jurisdiction"));
    }

    #[test]
    fn test_cell_invalid_method_rejected() {
        let mut input = cell_pcc_input();
        input.cost_allocation_method = "Random".to_string();
        let err = cell_company_analysis(&input).unwrap_err();
        assert!(err.to_string().contains("cost_allocation_method"));
    }

    #[test]
    fn test_cell_empty_cells_rejected() {
        let mut input = cell_pcc_input();
        input.cells = vec![];
        let err = cell_company_analysis(&input).unwrap_err();
        assert!(err.to_string().contains("cells"));
    }

    #[test]
    fn test_cell_zero_aum_rejected() {
        let mut input = cell_pcc_input();
        input.cells[0].cell_aum = Decimal::ZERO;
        let err = cell_company_analysis(&input).unwrap_err();
        assert!(err.to_string().contains("cell_aum"));
    }

    #[test]
    fn test_cell_negative_expense_rejected() {
        let mut input = cell_pcc_input();
        input.cells[0].expense_ratio = dec!(-0.01);
        let err = cell_company_analysis(&input).unwrap_err();
        assert!(err.to_string().contains("expense_ratio"));
    }

    #[test]
    fn test_cell_negative_core_cost_rejected() {
        let mut input = cell_pcc_input();
        input.core_annual_cost = dec!(-1);
        let err = cell_company_analysis(&input).unwrap_err();
        assert!(err.to_string().contains("core_annual_cost"));
    }

    // ======================================================================
    // Edge Cases
    // ======================================================================

    #[test]
    fn test_jersey_jpf_boundary_50_investors() {
        let mut input = jersey_jpf_input();
        input.investor_count = 50;
        assert!(analyze_jersey_fund(&input).is_ok());
    }

    #[test]
    fn test_guernsey_pif_boundary_50_investors() {
        let mut input = guernsey_pif_input();
        input.investor_count = 50;
        assert!(analyze_guernsey_fund(&input).is_ok());
    }

    #[test]
    fn test_jersey_small_fund_high_cost_warning() {
        let mut input = jersey_jpf_input();
        input.fund_size = dec!(5_000_000);
        input.investor_count = 5;
        let result = analyze_jersey_fund(&input).unwrap();
        assert!(result
            .warnings
            .iter()
            .any(|w| w.contains("0.50% threshold")));
    }

    #[test]
    fn test_comparison_empty_name_rejected() {
        let mut input = comparison_input();
        input.fund_name = "".to_string();
        let err = channel_islands_comparison(&input).unwrap_err();
        assert!(err.to_string().contains("fund_name"));
    }
}
