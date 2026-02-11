//! Multi-jurisdiction holding structure optimization and PE risk assessment.
//!
//! Models optimal tax-efficient holding structures, multi-tier analysis,
//! permanent establishment (PE) risk scoring, and substance cost-benefit.

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

use crate::error::CorpFinanceError;
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// An operating entity in the group.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperatingEntity {
    pub name: String,
    pub jurisdiction: String,
    pub annual_profit: Decimal,
    pub annual_dividends_up: Decimal,
    pub annual_royalties_out: Decimal,
    pub annual_interest_out: Decimal,
    pub annual_management_fees_out: Decimal,
    pub corporate_tax_rate: Decimal,
}

/// A candidate holding jurisdiction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HoldingCandidate {
    pub jurisdiction: String,
    pub corporate_tax_rate: Decimal,
    pub participation_exemption: bool,
    pub participation_threshold_pct: Decimal,
    pub ip_box_rate: Option<Decimal>,
    /// "Low", "Medium", or "High"
    pub cfc_rules_risk: String,
    pub substance_cost_annual: Decimal,
    pub treaty_network_size: u32,
}

/// The ultimate parent entity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParentEntity {
    pub jurisdiction: String,
    pub corporate_tax_rate: Decimal,
}

/// PE risk factors for a jurisdiction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeRiskFactor {
    pub jurisdiction: String,
    pub has_fixed_place: bool,
    pub has_dependent_agent: bool,
    pub employees_in_jurisdiction: u32,
    pub contracts_concluded_locally: bool,
    pub server_or_warehouse: bool,
    pub duration_months: u32,
}

/// Input for treaty structure optimization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreatyOptInput {
    pub group_name: String,
    pub operating_jurisdictions: Vec<OperatingEntity>,
    pub holding_jurisdiction_candidates: Vec<HoldingCandidate>,
    pub ultimate_parent: ParentEntity,
    pub pe_risk_factors: Vec<PeRiskFactor>,
}

// ---------------------------------------------------------------------------
// Output types
// ---------------------------------------------------------------------------

/// A structure option with tax cost analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructureOption {
    pub holding_jurisdiction: String,
    pub total_tax_cost: Decimal,
    pub dividend_tax: Decimal,
    pub royalty_tax: Decimal,
    pub interest_tax: Decimal,
    pub mgmt_fee_tax: Decimal,
    pub substance_cost: Decimal,
    pub net_cost: Decimal,
    pub effective_tax_rate: Decimal,
    pub rank: u32,
}

/// The recommended optimal structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimalStructure {
    pub holding_jurisdiction: String,
    pub effective_tax_rate: Decimal,
    pub total_annual_tax: Decimal,
    pub total_substance_cost: Decimal,
    pub annual_savings_vs_direct: Decimal,
    pub payback_period_years: Decimal,
    pub key_benefits: Vec<String>,
    pub key_risks: Vec<String>,
}

/// Multi-tier analysis result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiTierAnalysis {
    pub recommended: bool,
    pub tier1: String,
    pub tier2: String,
    pub combined_etr: Decimal,
    pub savings_vs_single_tier: Decimal,
}

/// PE assessment for a single jurisdiction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeAssessment {
    pub jurisdiction: String,
    pub fixed_place_risk: bool,
    pub dependent_agent_risk: bool,
    pub service_pe_risk: bool,
    pub overall_risk_score: u32,
    pub risk_level: String,
    pub tax_exposure_if_pe: Decimal,
    pub mitigation: Vec<String>,
}

/// Cost-benefit for substance in a jurisdiction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostBenefit {
    pub jurisdiction: String,
    pub annual_substance_cost: Decimal,
    pub annual_tax_saving: Decimal,
    pub net_benefit: Decimal,
    pub roi_pct: Decimal,
}

/// Output for treaty structure optimization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreatyOptOutput {
    pub structure_options: Vec<StructureOption>,
    pub optimal_structure: OptimalStructure,
    pub multi_tier: Option<MultiTierAnalysis>,
    pub pe_assessment: Vec<PeAssessment>,
    pub cost_benefit: Vec<CostBenefit>,
    pub recommendations: Vec<String>,
    pub warnings: Vec<String>,
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

fn validate_input(input: &TreatyOptInput) -> CorpFinanceResult<()> {
    if input.group_name.trim().is_empty() {
        return Err(CorpFinanceError::InvalidInput {
            field: "group_name".into(),
            reason: "Group name must not be empty".into(),
        });
    }
    if input.operating_jurisdictions.is_empty() {
        return Err(CorpFinanceError::InvalidInput {
            field: "operating_jurisdictions".into(),
            reason: "At least one operating entity is required".into(),
        });
    }
    if input.holding_jurisdiction_candidates.is_empty() {
        return Err(CorpFinanceError::InvalidInput {
            field: "holding_jurisdiction_candidates".into(),
            reason: "At least one holding candidate is required".into(),
        });
    }
    for (i, ent) in input.operating_jurisdictions.iter().enumerate() {
        if ent.name.trim().is_empty() {
            return Err(CorpFinanceError::InvalidInput {
                field: format!("operating_jurisdictions[{}].name", i),
                reason: "Entity name must not be empty".into(),
            });
        }
        if ent.jurisdiction.trim().is_empty() {
            return Err(CorpFinanceError::InvalidInput {
                field: format!("operating_jurisdictions[{}].jurisdiction", i),
                reason: "Jurisdiction must not be empty".into(),
            });
        }
        if ent.corporate_tax_rate < dec!(0) || ent.corporate_tax_rate > dec!(1) {
            return Err(CorpFinanceError::InvalidInput {
                field: format!("operating_jurisdictions[{}].corporate_tax_rate", i),
                reason: "Tax rate must be between 0 and 1".into(),
            });
        }
        if ent.annual_profit < dec!(0) {
            return Err(CorpFinanceError::InvalidInput {
                field: format!("operating_jurisdictions[{}].annual_profit", i),
                reason: "Annual profit must be non-negative".into(),
            });
        }
        if ent.annual_dividends_up < dec!(0) {
            return Err(CorpFinanceError::InvalidInput {
                field: format!("operating_jurisdictions[{}].annual_dividends_up", i),
                reason: "Annual dividends must be non-negative".into(),
            });
        }
        if ent.annual_royalties_out < dec!(0) {
            return Err(CorpFinanceError::InvalidInput {
                field: format!("operating_jurisdictions[{}].annual_royalties_out", i),
                reason: "Annual royalties must be non-negative".into(),
            });
        }
        if ent.annual_interest_out < dec!(0) {
            return Err(CorpFinanceError::InvalidInput {
                field: format!("operating_jurisdictions[{}].annual_interest_out", i),
                reason: "Annual interest must be non-negative".into(),
            });
        }
        if ent.annual_management_fees_out < dec!(0) {
            return Err(CorpFinanceError::InvalidInput {
                field: format!("operating_jurisdictions[{}].annual_management_fees_out", i),
                reason: "Annual management fees must be non-negative".into(),
            });
        }
    }
    for (i, hc) in input.holding_jurisdiction_candidates.iter().enumerate() {
        if hc.jurisdiction.trim().is_empty() {
            return Err(CorpFinanceError::InvalidInput {
                field: format!("holding_jurisdiction_candidates[{}].jurisdiction", i),
                reason: "Jurisdiction must not be empty".into(),
            });
        }
        if hc.corporate_tax_rate < dec!(0) || hc.corporate_tax_rate > dec!(1) {
            return Err(CorpFinanceError::InvalidInput {
                field: format!("holding_jurisdiction_candidates[{}].corporate_tax_rate", i),
                reason: "Tax rate must be between 0 and 1".into(),
            });
        }
        if hc.participation_threshold_pct < dec!(0) || hc.participation_threshold_pct > dec!(100) {
            return Err(CorpFinanceError::InvalidInput {
                field: format!(
                    "holding_jurisdiction_candidates[{}].participation_threshold_pct",
                    i
                ),
                reason: "Participation threshold must be between 0 and 100".into(),
            });
        }
        if hc.substance_cost_annual < dec!(0) {
            return Err(CorpFinanceError::InvalidInput {
                field: format!(
                    "holding_jurisdiction_candidates[{}].substance_cost_annual",
                    i
                ),
                reason: "Substance cost must be non-negative".into(),
            });
        }
        if let Some(ip_rate) = hc.ip_box_rate {
            if ip_rate < dec!(0) || ip_rate > dec!(1) {
                return Err(CorpFinanceError::InvalidInput {
                    field: format!("holding_jurisdiction_candidates[{}].ip_box_rate", i),
                    reason: "IP box rate must be between 0 and 1".into(),
                });
            }
        }
        let valid_cfc = ["Low", "Medium", "High"];
        if !valid_cfc.contains(&hc.cfc_rules_risk.as_str()) {
            return Err(CorpFinanceError::InvalidInput {
                field: format!("holding_jurisdiction_candidates[{}].cfc_rules_risk", i),
                reason: format!(
                    "Invalid CFC risk '{}'. Valid: {:?}",
                    hc.cfc_rules_risk, valid_cfc
                ),
            });
        }
    }
    if input.ultimate_parent.jurisdiction.trim().is_empty() {
        return Err(CorpFinanceError::InvalidInput {
            field: "ultimate_parent.jurisdiction".into(),
            reason: "Parent jurisdiction must not be empty".into(),
        });
    }
    if input.ultimate_parent.corporate_tax_rate < dec!(0)
        || input.ultimate_parent.corporate_tax_rate > dec!(1)
    {
        return Err(CorpFinanceError::InvalidInput {
            field: "ultimate_parent.corporate_tax_rate".into(),
            reason: "Tax rate must be between 0 and 1".into(),
        });
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Internal calculation helpers
// ---------------------------------------------------------------------------

/// EBITDA-based interest deduction limit (typically 30% of EBITDA).
const INTEREST_DEDUCTION_LIMIT: Decimal = dec!(0.30);

/// Compute tax cost for routing all flows through a given holding candidate.
fn compute_structure_cost(
    candidate: &HoldingCandidate,
    ops: &[OperatingEntity],
    parent: &ParentEntity,
) -> (Decimal, Decimal, Decimal, Decimal, Decimal) {
    let mut dividend_tax = dec!(0);
    let mut royalty_tax = dec!(0);
    let mut interest_tax = dec!(0);
    let mut mgmt_fee_tax = dec!(0);

    for op in ops {
        // -- Dividends --
        // If participation exemption applies, dividends are exempt at the holding level
        if candidate.participation_exemption {
            // Assume threshold is met for simplicity; dividends pass through tax-free
            // Potential WHT from operating jurisdiction (simplified: assume treaty
            // eliminates or reduces; use a default rate)
            let wht_on_div = op.annual_dividends_up * estimate_wht_rate(op, candidate, "Dividends");
            dividend_tax += wht_on_div;
        } else {
            // Dividends taxed at holding corporate rate
            let corp_tax_on_div = op.annual_dividends_up * candidate.corporate_tax_rate;
            let wht_on_div = op.annual_dividends_up * estimate_wht_rate(op, candidate, "Dividends");
            dividend_tax += corp_tax_on_div + wht_on_div;
        }

        // -- Royalties --
        let royalty_rate = if let Some(ip_rate) = candidate.ip_box_rate {
            ip_rate
        } else {
            candidate.corporate_tax_rate
        };
        let corp_tax_on_royalty = op.annual_royalties_out * royalty_rate;
        let wht_on_royalty =
            op.annual_royalties_out * estimate_wht_rate(op, candidate, "Royalties");
        royalty_tax += corp_tax_on_royalty + wht_on_royalty;

        // -- Interest --
        // Interest deduction may be limited (30% of EBITDA)
        let deductible_interest = op
            .annual_interest_out
            .min(op.annual_profit * INTEREST_DEDUCTION_LIMIT);
        let non_deductible = op.annual_interest_out - deductible_interest;
        // Non-deductible portion is effectively taxed at operating rate
        let extra_tax_non_deductible = non_deductible * op.corporate_tax_rate;
        // WHT on interest from operating entity to holding
        let wht_on_interest = op.annual_interest_out * estimate_wht_rate(op, candidate, "Interest");
        // Tax at holding level on interest income
        let corp_tax_on_interest = op.annual_interest_out * candidate.corporate_tax_rate;
        interest_tax += wht_on_interest + corp_tax_on_interest + extra_tax_non_deductible;

        // -- Management fees --
        let wht_on_mgmt =
            op.annual_management_fees_out * estimate_wht_rate(op, candidate, "ManagementFees");
        let corp_tax_on_mgmt = op.annual_management_fees_out * candidate.corporate_tax_rate;
        mgmt_fee_tax += wht_on_mgmt + corp_tax_on_mgmt;
    }

    // Add upstream tax: holding -> parent dividend (on accumulated earnings)
    let total_earnings: Decimal = ops
        .iter()
        .map(|o| {
            o.annual_dividends_up
                + o.annual_royalties_out
                + o.annual_interest_out
                + o.annual_management_fees_out
        })
        .sum();

    // Simplified: assume parent taxes incremental income at its rate minus foreign tax credit
    let holding_to_parent_wht = total_earnings * estimate_upstream_wht(candidate, parent);
    dividend_tax += holding_to_parent_wht;

    let total_tax = dividend_tax + royalty_tax + interest_tax + mgmt_fee_tax;

    (
        total_tax,
        dividend_tax,
        royalty_tax,
        interest_tax,
        mgmt_fee_tax,
    )
}

/// Simplified WHT rate estimate based on treaty network size heuristic.
fn estimate_wht_rate(
    _op: &OperatingEntity,
    candidate: &HoldingCandidate,
    income_type: &str,
) -> Decimal {
    // Jurisdictions with large treaty networks typically have lower effective WHT
    let base = match income_type {
        "Dividends" => {
            if candidate.participation_exemption {
                dec!(0.05) // Low rate with participation exemption treaties
            } else {
                dec!(0.15) // Standard treaty rate
            }
        }
        "Interest" => dec!(0.0),        // Most EU/treaty jurisdictions: 0%
        "Royalties" => dec!(0.05),      // Typical treaty rate for royalties
        "ManagementFees" => dec!(0.10), // Often not covered by treaties
        _ => dec!(0.15),
    };

    // Adjust based on treaty network size
    if candidate.treaty_network_size >= 80 {
        base * dec!(0.5) // Extensive network -> lower effective rates
    } else if candidate.treaty_network_size >= 50 {
        base * dec!(0.7)
    } else {
        base
    }
}

/// Estimate upstream WHT from holding to parent jurisdiction.
fn estimate_upstream_wht(candidate: &HoldingCandidate, _parent: &ParentEntity) -> Decimal {
    if candidate.participation_exemption {
        dec!(0.0) // Participation exemption eliminates upstream WHT on dividends
    } else {
        dec!(0.05) // Default treaty rate
    }
}

/// Compute direct repatriation cost (no intermediate holding).
fn compute_direct_cost(ops: &[OperatingEntity], parent: &ParentEntity) -> Decimal {
    let mut total = dec!(0);
    for op in ops {
        // Dividends: assume 15% WHT (no treaty optimization)
        total += op.annual_dividends_up * dec!(0.15);
        // Royalties: 15% WHT
        total += op.annual_royalties_out * dec!(0.15);
        // Interest: 10% WHT
        total += op.annual_interest_out * dec!(0.10);
        // Management fees: 15% WHT
        total += op.annual_management_fees_out * dec!(0.15);
        // Parent taxes remaining at its rate (simplified)
        let total_flow = op.annual_dividends_up
            + op.annual_royalties_out
            + op.annual_interest_out
            + op.annual_management_fees_out;
        total += total_flow * parent.corporate_tax_rate * dec!(0.5); // Partial credit
    }
    total
}

// ---------------------------------------------------------------------------
// PE assessment
// ---------------------------------------------------------------------------

fn assess_pe_risk(factor: &PeRiskFactor, ops: &[OperatingEntity]) -> PeAssessment {
    let mut risk_score: u32 = 0;
    let mut mitigation: Vec<String> = Vec::new();

    // Fixed place of business (Article 5(1))
    let fixed_place_risk = factor.has_fixed_place;
    if fixed_place_risk {
        risk_score += 30;
        mitigation
            .push("Ensure fixed place is auxiliary/preparatory (Article 5(4) exception)".into());
    }

    // Dependent agent PE (Article 5(5))
    let dependent_agent_risk = factor.has_dependent_agent || factor.contracts_concluded_locally;
    if dependent_agent_risk {
        risk_score += 25;
        mitigation.push("Limit agent authority — ensure no habitually concluding contracts".into());
    }
    if factor.contracts_concluded_locally && !factor.has_dependent_agent {
        risk_score += 10;
    }

    // Service PE: employees present > 183 days
    let service_pe_risk = factor.employees_in_jurisdiction > 0 && factor.duration_months > 6;
    if service_pe_risk {
        risk_score += 20;
        mitigation.push(format!(
            "Limit employee presence to <183 days in 12 months (currently {} employees, {} months)",
            factor.employees_in_jurisdiction, factor.duration_months
        ));
    } else if factor.employees_in_jurisdiction > 0 {
        risk_score += 5;
    }

    // Server/warehouse presence
    if factor.server_or_warehouse {
        risk_score += 10;
        mitigation.push("Ensure server/warehouse is auxiliary/preparatory only".into());
    }

    // Digital PE considerations (post-BEPS)
    if factor.server_or_warehouse && factor.duration_months > 12 {
        risk_score += 5;
        mitigation.push("Consider digital PE risk under BEPS Action 7 / Pillar One".into());
    }

    risk_score = risk_score.min(100);

    let risk_level = if risk_score <= 20 {
        "Low".to_string()
    } else if risk_score <= 50 {
        "Medium".to_string()
    } else {
        "High".to_string()
    };

    // Estimated tax exposure if PE is found
    let jurisdiction_profit: Decimal = ops
        .iter()
        .filter(|o| o.jurisdiction == factor.jurisdiction)
        .map(|o| o.annual_profit)
        .sum();

    // If no operating entity in this jurisdiction, estimate based on employee presence
    let estimated_profit = if jurisdiction_profit > dec!(0) {
        jurisdiction_profit
    } else {
        // Estimate: proportional to employee presence
        let total_profit: Decimal = ops.iter().map(|o| o.annual_profit).sum();
        let total_employees: u32 = ops.iter().map(|_o| 10u32).sum(); // Rough estimate
        if total_employees > 0 {
            total_profit * Decimal::from(factor.employees_in_jurisdiction)
                / Decimal::from(total_employees)
        } else {
            dec!(0)
        }
    };

    // Assume local tax rate ~25% if PE found (conservative)
    let tax_exposure_if_pe = estimated_profit * dec!(0.25);

    if mitigation.is_empty() {
        mitigation.push("Low PE risk — maintain current posture".into());
    }

    PeAssessment {
        jurisdiction: factor.jurisdiction.clone(),
        fixed_place_risk,
        dependent_agent_risk,
        service_pe_risk,
        overall_risk_score: risk_score,
        risk_level,
        tax_exposure_if_pe,
        mitigation,
    }
}

// ---------------------------------------------------------------------------
// Multi-tier analysis
// ---------------------------------------------------------------------------

fn analyze_multi_tier(
    candidates: &[HoldingCandidate],
    ops: &[OperatingEntity],
    parent: &ParentEntity,
    best_single_tier_cost: Decimal,
) -> Option<MultiTierAnalysis> {
    if candidates.len() < 2 {
        return None;
    }

    let mut best: Option<MultiTierAnalysis> = None;
    let mut best_combined_cost = best_single_tier_cost;

    // Try all pairs as tier1 (regional) and tier2 (intermediate)
    for tier1 in candidates {
        for tier2 in candidates {
            if tier1.jurisdiction == tier2.jurisdiction {
                continue;
            }

            // Tier 1: Operating -> Regional Holding
            let (tier1_tax, ..) = compute_structure_cost(tier1, ops, parent);

            // Tier 2: Regional Holding -> Intermediate -> Parent
            // Simplified: tier2 adds substance cost but may reduce upstream WHT
            let upstream_benefit = if tier2.participation_exemption {
                // Eliminate upstream dividend tax
                let total_div: Decimal = ops.iter().map(|o| o.annual_dividends_up).sum();
                total_div * dec!(0.05) // Estimated savings on upstream WHT
            } else {
                dec!(0)
            };

            let combined_cost = tier1_tax + tier2.substance_cost_annual - upstream_benefit;

            let total_flows: Decimal = ops
                .iter()
                .map(|o| {
                    o.annual_dividends_up
                        + o.annual_royalties_out
                        + o.annual_interest_out
                        + o.annual_management_fees_out
                })
                .sum();

            let combined_etr = if total_flows > dec!(0) {
                combined_cost / total_flows
            } else {
                dec!(0)
            };

            let savings = best_single_tier_cost - combined_cost;

            if combined_cost < best_combined_cost && savings > dec!(0) {
                best_combined_cost = combined_cost;
                best = Some(MultiTierAnalysis {
                    recommended: true,
                    tier1: tier1.jurisdiction.clone(),
                    tier2: tier2.jurisdiction.clone(),
                    combined_etr,
                    savings_vs_single_tier: savings,
                });
            }
        }
    }

    // If no multi-tier is better, return non-recommended analysis
    if best.is_none() && candidates.len() >= 2 {
        let total_flows: Decimal = ops
            .iter()
            .map(|o| {
                o.annual_dividends_up
                    + o.annual_royalties_out
                    + o.annual_interest_out
                    + o.annual_management_fees_out
            })
            .sum();

        let etr = if total_flows > dec!(0) {
            best_single_tier_cost / total_flows
        } else {
            dec!(0)
        };

        best = Some(MultiTierAnalysis {
            recommended: false,
            tier1: candidates[0].jurisdiction.clone(),
            tier2: candidates[1].jurisdiction.clone(),
            combined_etr: etr,
            savings_vs_single_tier: dec!(0),
        });
    }

    best
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Optimize multi-jurisdiction holding structure for tax efficiency.
///
/// Models holding structure options, multi-tier analysis, PE risk assessment,
/// and substance cost-benefit for each candidate jurisdiction.
pub fn optimize_treaty_structure(input: &TreatyOptInput) -> CorpFinanceResult<TreatyOptOutput> {
    validate_input(input)?;

    let mut recommendations: Vec<String> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();

    // --- 1. Compute direct cost baseline ---
    let direct_cost = compute_direct_cost(&input.operating_jurisdictions, &input.ultimate_parent);

    // --- 2. Evaluate each holding structure ---
    let mut structure_options: Vec<StructureOption> = Vec::new();

    for candidate in &input.holding_jurisdiction_candidates {
        let (total_tax, div_tax, roy_tax, int_tax, mgmt_tax) = compute_structure_cost(
            candidate,
            &input.operating_jurisdictions,
            &input.ultimate_parent,
        );

        let net_cost = total_tax + candidate.substance_cost_annual;

        let total_flows: Decimal = input
            .operating_jurisdictions
            .iter()
            .map(|o| {
                o.annual_dividends_up
                    + o.annual_royalties_out
                    + o.annual_interest_out
                    + o.annual_management_fees_out
            })
            .sum();

        let effective_tax_rate = if total_flows > dec!(0) {
            net_cost / total_flows
        } else {
            dec!(0)
        };

        structure_options.push(StructureOption {
            holding_jurisdiction: candidate.jurisdiction.clone(),
            total_tax_cost: total_tax,
            dividend_tax: div_tax,
            royalty_tax: roy_tax,
            interest_tax: int_tax,
            mgmt_fee_tax: mgmt_tax,
            substance_cost: candidate.substance_cost_annual,
            net_cost,
            effective_tax_rate,
            rank: 0, // Will be set after sorting
        });
    }

    // Sort by net_cost ascending and assign ranks
    structure_options.sort_by(|a, b| {
        a.net_cost
            .partial_cmp(&b.net_cost)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    for (i, opt) in structure_options.iter_mut().enumerate() {
        opt.rank = (i + 1) as u32;
    }

    // --- 3. Determine optimal structure ---
    let best = &structure_options[0]; // Already sorted

    let savings_vs_direct = direct_cost - best.net_cost;

    let payback_period = if savings_vs_direct > dec!(0) {
        best.substance_cost / savings_vs_direct
    } else {
        dec!(999) // No payback
    };

    let mut key_benefits: Vec<String> = Vec::new();
    let mut key_risks: Vec<String> = Vec::new();

    // Find the matching candidate
    if let Some(cand) = input
        .holding_jurisdiction_candidates
        .iter()
        .find(|c| c.jurisdiction == best.holding_jurisdiction)
    {
        if cand.participation_exemption {
            key_benefits.push("Participation exemption on dividends".into());
        }
        if let Some(ip_rate) = cand.ip_box_rate {
            key_benefits.push(format!("IP box rate: {}%", ip_rate * dec!(100)));
        }
        if cand.treaty_network_size >= 80 {
            key_benefits.push(format!(
                "Extensive treaty network ({} treaties)",
                cand.treaty_network_size
            ));
        }
        match cand.cfc_rules_risk.as_str() {
            "High" => {
                key_risks.push("High CFC rules risk — parent may tax passively".into());
                warnings.push(format!(
                    "{}: High CFC risk could negate holding benefits",
                    cand.jurisdiction
                ));
            }
            "Medium" => {
                key_risks.push("Medium CFC rules risk — monitor compliance".into());
            }
            _ => {}
        }
    }

    if savings_vs_direct <= dec!(0) {
        warnings.push("No holding structure provides net savings vs direct repatriation".into());
    }

    let optimal_structure = OptimalStructure {
        holding_jurisdiction: best.holding_jurisdiction.clone(),
        effective_tax_rate: best.effective_tax_rate,
        total_annual_tax: best.total_tax_cost,
        total_substance_cost: best.substance_cost,
        annual_savings_vs_direct: savings_vs_direct,
        payback_period_years: payback_period,
        key_benefits,
        key_risks,
    };

    // --- 4. Multi-tier analysis ---
    let multi_tier = analyze_multi_tier(
        &input.holding_jurisdiction_candidates,
        &input.operating_jurisdictions,
        &input.ultimate_parent,
        best.net_cost,
    );

    if let Some(ref mt) = multi_tier {
        if mt.recommended {
            recommendations.push(format!(
                "Consider two-tier structure: {} (regional) -> {} (intermediate) for additional savings of {}",
                mt.tier1, mt.tier2, mt.savings_vs_single_tier
            ));
        }
    }

    // --- 5. PE Assessment ---
    let pe_assessment: Vec<PeAssessment> = input
        .pe_risk_factors
        .iter()
        .map(|f| assess_pe_risk(f, &input.operating_jurisdictions))
        .collect();

    for pa in &pe_assessment {
        if pa.overall_risk_score > 50 {
            warnings.push(format!(
                "High PE risk in {} (score {}) — tax exposure if PE found: {}",
                pa.jurisdiction, pa.overall_risk_score, pa.tax_exposure_if_pe
            ));
        }
    }

    // --- 6. Cost-Benefit ---
    let cost_benefit: Vec<CostBenefit> = input
        .holding_jurisdiction_candidates
        .iter()
        .map(|cand| {
            let (tax_cost, ..) = compute_structure_cost(
                cand,
                &input.operating_jurisdictions,
                &input.ultimate_parent,
            );
            let tax_saving = direct_cost - tax_cost;
            let net_benefit = tax_saving - cand.substance_cost_annual;
            let roi_pct = if cand.substance_cost_annual > dec!(0) {
                (net_benefit / cand.substance_cost_annual) * dec!(100)
            } else if net_benefit > dec!(0) {
                dec!(999) // Infinite ROI (no substance cost)
            } else {
                dec!(0)
            };
            CostBenefit {
                jurisdiction: cand.jurisdiction.clone(),
                annual_substance_cost: cand.substance_cost_annual,
                annual_tax_saving: tax_saving,
                net_benefit,
                roi_pct,
            }
        })
        .collect();

    // General recommendations
    if savings_vs_direct > dec!(0) {
        recommendations.push(format!(
            "Recommended holding jurisdiction: {} (saves {} annually vs direct repatriation)",
            best.holding_jurisdiction, savings_vs_direct
        ));
    }
    recommendations
        .push("Ensure sufficient economic substance in chosen holding jurisdiction".into());
    recommendations
        .push("Review BEPS Action 6 (treaty abuse) and Action 7 (PE avoidance) compliance".into());

    Ok(TreatyOptOutput {
        structure_options,
        optimal_structure,
        multi_tier,
        pe_assessment,
        cost_benefit,
        recommendations,
        warnings,
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn make_basic_input() -> TreatyOptInput {
        TreatyOptInput {
            group_name: "TestGroup".to_string(),
            operating_jurisdictions: vec![
                OperatingEntity {
                    name: "OpCo Germany".to_string(),
                    jurisdiction: "Germany".to_string(),
                    annual_profit: dec!(10_000_000),
                    annual_dividends_up: dec!(5_000_000),
                    annual_royalties_out: dec!(1_000_000),
                    annual_interest_out: dec!(500_000),
                    annual_management_fees_out: dec!(200_000),
                    corporate_tax_rate: dec!(0.30),
                },
                OperatingEntity {
                    name: "OpCo UK".to_string(),
                    jurisdiction: "UK".to_string(),
                    annual_profit: dec!(8_000_000),
                    annual_dividends_up: dec!(4_000_000),
                    annual_royalties_out: dec!(800_000),
                    annual_interest_out: dec!(300_000),
                    annual_management_fees_out: dec!(150_000),
                    corporate_tax_rate: dec!(0.25),
                },
            ],
            holding_jurisdiction_candidates: vec![
                HoldingCandidate {
                    jurisdiction: "Netherlands".to_string(),
                    corporate_tax_rate: dec!(0.2569),
                    participation_exemption: true,
                    participation_threshold_pct: dec!(5),
                    ip_box_rate: Some(dec!(0.09)),
                    cfc_rules_risk: "Low".to_string(),
                    substance_cost_annual: dec!(200_000),
                    treaty_network_size: 95,
                },
                HoldingCandidate {
                    jurisdiction: "Luxembourg".to_string(),
                    corporate_tax_rate: dec!(0.2494),
                    participation_exemption: true,
                    participation_threshold_pct: dec!(10),
                    ip_box_rate: Some(dec!(0.0528)),
                    cfc_rules_risk: "Low".to_string(),
                    substance_cost_annual: dec!(250_000),
                    treaty_network_size: 83,
                },
                HoldingCandidate {
                    jurisdiction: "Ireland".to_string(),
                    corporate_tax_rate: dec!(0.125),
                    participation_exemption: true,
                    participation_threshold_pct: dec!(5),
                    ip_box_rate: Some(dec!(0.0625)),
                    cfc_rules_risk: "Medium".to_string(),
                    substance_cost_annual: dec!(180_000),
                    treaty_network_size: 74,
                },
            ],
            ultimate_parent: ParentEntity {
                jurisdiction: "US".to_string(),
                corporate_tax_rate: dec!(0.21),
            },
            pe_risk_factors: vec![
                PeRiskFactor {
                    jurisdiction: "Germany".to_string(),
                    has_fixed_place: false,
                    has_dependent_agent: false,
                    employees_in_jurisdiction: 0,
                    contracts_concluded_locally: false,
                    server_or_warehouse: false,
                    duration_months: 0,
                },
                PeRiskFactor {
                    jurisdiction: "UK".to_string(),
                    has_fixed_place: true,
                    has_dependent_agent: true,
                    employees_in_jurisdiction: 5,
                    contracts_concluded_locally: true,
                    server_or_warehouse: false,
                    duration_months: 12,
                },
            ],
        }
    }

    // --- Validation tests ---

    #[test]
    fn test_empty_group_name() {
        let mut input = make_basic_input();
        input.group_name = "".to_string();
        assert!(optimize_treaty_structure(&input).is_err());
    }

    #[test]
    fn test_empty_operating_jurisdictions() {
        let mut input = make_basic_input();
        input.operating_jurisdictions = vec![];
        assert!(optimize_treaty_structure(&input).is_err());
    }

    #[test]
    fn test_empty_holding_candidates() {
        let mut input = make_basic_input();
        input.holding_jurisdiction_candidates = vec![];
        assert!(optimize_treaty_structure(&input).is_err());
    }

    #[test]
    fn test_empty_entity_name() {
        let mut input = make_basic_input();
        input.operating_jurisdictions[0].name = "".to_string();
        assert!(optimize_treaty_structure(&input).is_err());
    }

    #[test]
    fn test_empty_entity_jurisdiction() {
        let mut input = make_basic_input();
        input.operating_jurisdictions[0].jurisdiction = "".to_string();
        assert!(optimize_treaty_structure(&input).is_err());
    }

    #[test]
    fn test_negative_profit() {
        let mut input = make_basic_input();
        input.operating_jurisdictions[0].annual_profit = dec!(-1);
        assert!(optimize_treaty_structure(&input).is_err());
    }

    #[test]
    fn test_negative_dividends() {
        let mut input = make_basic_input();
        input.operating_jurisdictions[0].annual_dividends_up = dec!(-1);
        assert!(optimize_treaty_structure(&input).is_err());
    }

    #[test]
    fn test_negative_royalties() {
        let mut input = make_basic_input();
        input.operating_jurisdictions[0].annual_royalties_out = dec!(-1);
        assert!(optimize_treaty_structure(&input).is_err());
    }

    #[test]
    fn test_negative_interest() {
        let mut input = make_basic_input();
        input.operating_jurisdictions[0].annual_interest_out = dec!(-1);
        assert!(optimize_treaty_structure(&input).is_err());
    }

    #[test]
    fn test_negative_mgmt_fees() {
        let mut input = make_basic_input();
        input.operating_jurisdictions[0].annual_management_fees_out = dec!(-1);
        assert!(optimize_treaty_structure(&input).is_err());
    }

    #[test]
    fn test_invalid_corporate_tax_rate() {
        let mut input = make_basic_input();
        input.operating_jurisdictions[0].corporate_tax_rate = dec!(1.5);
        assert!(optimize_treaty_structure(&input).is_err());
    }

    #[test]
    fn test_negative_corporate_tax_rate() {
        let mut input = make_basic_input();
        input.operating_jurisdictions[0].corporate_tax_rate = dec!(-0.1);
        assert!(optimize_treaty_structure(&input).is_err());
    }

    #[test]
    fn test_invalid_holding_tax_rate() {
        let mut input = make_basic_input();
        input.holding_jurisdiction_candidates[0].corporate_tax_rate = dec!(2.0);
        assert!(optimize_treaty_structure(&input).is_err());
    }

    #[test]
    fn test_invalid_participation_threshold() {
        let mut input = make_basic_input();
        input.holding_jurisdiction_candidates[0].participation_threshold_pct = dec!(101);
        assert!(optimize_treaty_structure(&input).is_err());
    }

    #[test]
    fn test_negative_substance_cost() {
        let mut input = make_basic_input();
        input.holding_jurisdiction_candidates[0].substance_cost_annual = dec!(-1);
        assert!(optimize_treaty_structure(&input).is_err());
    }

    #[test]
    fn test_invalid_ip_box_rate() {
        let mut input = make_basic_input();
        input.holding_jurisdiction_candidates[0].ip_box_rate = Some(dec!(1.5));
        assert!(optimize_treaty_structure(&input).is_err());
    }

    #[test]
    fn test_invalid_cfc_risk() {
        let mut input = make_basic_input();
        input.holding_jurisdiction_candidates[0].cfc_rules_risk = "Invalid".to_string();
        assert!(optimize_treaty_structure(&input).is_err());
    }

    #[test]
    fn test_empty_parent_jurisdiction() {
        let mut input = make_basic_input();
        input.ultimate_parent.jurisdiction = "".to_string();
        assert!(optimize_treaty_structure(&input).is_err());
    }

    #[test]
    fn test_invalid_parent_tax_rate() {
        let mut input = make_basic_input();
        input.ultimate_parent.corporate_tax_rate = dec!(1.5);
        assert!(optimize_treaty_structure(&input).is_err());
    }

    // --- Structure option tests ---

    #[test]
    fn test_structure_options_generated() {
        let input = make_basic_input();
        let output = optimize_treaty_structure(&input).unwrap();
        assert_eq!(output.structure_options.len(), 3);
    }

    #[test]
    fn test_structure_options_ranked() {
        let input = make_basic_input();
        let output = optimize_treaty_structure(&input).unwrap();
        for (i, opt) in output.structure_options.iter().enumerate() {
            assert_eq!(opt.rank, (i + 1) as u32);
        }
    }

    #[test]
    fn test_structure_options_sorted_by_net_cost() {
        let input = make_basic_input();
        let output = optimize_treaty_structure(&input).unwrap();
        for i in 1..output.structure_options.len() {
            assert!(
                output.structure_options[i].net_cost >= output.structure_options[i - 1].net_cost
            );
        }
    }

    #[test]
    fn test_structure_option_tax_components() {
        let input = make_basic_input();
        let output = optimize_treaty_structure(&input).unwrap();
        for opt in &output.structure_options {
            assert!(opt.dividend_tax >= dec!(0));
            assert!(opt.royalty_tax >= dec!(0));
            assert!(opt.interest_tax >= dec!(0));
            assert!(opt.mgmt_fee_tax >= dec!(0));
            // Net cost = total_tax + substance
            assert_eq!(opt.net_cost, opt.total_tax_cost + opt.substance_cost);
        }
    }

    #[test]
    fn test_effective_tax_rate_positive() {
        let input = make_basic_input();
        let output = optimize_treaty_structure(&input).unwrap();
        for opt in &output.structure_options {
            assert!(opt.effective_tax_rate >= dec!(0));
        }
    }

    // --- Optimal structure tests ---

    #[test]
    fn test_optimal_is_rank_1() {
        let input = make_basic_input();
        let output = optimize_treaty_structure(&input).unwrap();
        assert_eq!(
            output.optimal_structure.holding_jurisdiction,
            output.structure_options[0].holding_jurisdiction
        );
    }

    #[test]
    fn test_optimal_savings_vs_direct() {
        let input = make_basic_input();
        let output = optimize_treaty_structure(&input).unwrap();
        // Savings should be computed relative to direct cost
        // The value should be a Decimal, positive means saving
        assert!(
            output
                .optimal_structure
                .annual_savings_vs_direct
                .is_sign_positive()
                || output.optimal_structure.annual_savings_vs_direct == dec!(0)
        );
    }

    #[test]
    fn test_payback_period_positive() {
        let input = make_basic_input();
        let output = optimize_treaty_structure(&input).unwrap();
        assert!(output.optimal_structure.payback_period_years >= dec!(0));
    }

    #[test]
    fn test_optimal_key_benefits() {
        let input = make_basic_input();
        let output = optimize_treaty_structure(&input).unwrap();
        // The optimal structure should list at least one benefit
        assert!(!output.optimal_structure.key_benefits.is_empty());
    }

    // --- PE assessment tests ---

    #[test]
    fn test_pe_assessments_generated() {
        let input = make_basic_input();
        let output = optimize_treaty_structure(&input).unwrap();
        assert_eq!(output.pe_assessment.len(), 2);
    }

    #[test]
    fn test_pe_low_risk_germany() {
        let input = make_basic_input();
        let output = optimize_treaty_structure(&input).unwrap();
        let de = output
            .pe_assessment
            .iter()
            .find(|p| p.jurisdiction == "Germany")
            .unwrap();
        assert_eq!(de.risk_level, "Low");
        assert!(!de.fixed_place_risk);
        assert!(!de.dependent_agent_risk);
        assert!(!de.service_pe_risk);
    }

    #[test]
    fn test_pe_high_risk_uk() {
        let input = make_basic_input();
        let output = optimize_treaty_structure(&input).unwrap();
        let uk = output
            .pe_assessment
            .iter()
            .find(|p| p.jurisdiction == "UK")
            .unwrap();
        assert_eq!(uk.risk_level, "High");
        assert!(uk.fixed_place_risk);
        assert!(uk.dependent_agent_risk);
        assert!(uk.service_pe_risk);
    }

    #[test]
    fn test_pe_risk_score_capped_at_100() {
        let mut input = make_basic_input();
        input.pe_risk_factors = vec![PeRiskFactor {
            jurisdiction: "UK".to_string(),
            has_fixed_place: true,
            has_dependent_agent: true,
            employees_in_jurisdiction: 100,
            contracts_concluded_locally: true,
            server_or_warehouse: true,
            duration_months: 24,
        }];
        let output = optimize_treaty_structure(&input).unwrap();
        assert!(output.pe_assessment[0].overall_risk_score <= 100);
    }

    #[test]
    fn test_pe_tax_exposure_non_negative() {
        let input = make_basic_input();
        let output = optimize_treaty_structure(&input).unwrap();
        for pa in &output.pe_assessment {
            assert!(pa.tax_exposure_if_pe >= dec!(0));
        }
    }

    #[test]
    fn test_pe_mitigation_always_present() {
        let input = make_basic_input();
        let output = optimize_treaty_structure(&input).unwrap();
        for pa in &output.pe_assessment {
            assert!(!pa.mitigation.is_empty());
        }
    }

    // --- Multi-tier tests ---

    #[test]
    fn test_multi_tier_generated() {
        let input = make_basic_input();
        let output = optimize_treaty_structure(&input).unwrap();
        assert!(output.multi_tier.is_some());
    }

    #[test]
    fn test_multi_tier_different_tiers() {
        let input = make_basic_input();
        let output = optimize_treaty_structure(&input).unwrap();
        if let Some(ref mt) = output.multi_tier {
            assert_ne!(mt.tier1, mt.tier2);
        }
    }

    #[test]
    fn test_multi_tier_single_candidate_none() {
        let mut input = make_basic_input();
        input.holding_jurisdiction_candidates =
            vec![input.holding_jurisdiction_candidates[0].clone()];
        let output = optimize_treaty_structure(&input).unwrap();
        assert!(output.multi_tier.is_none());
    }

    // --- Cost-benefit tests ---

    #[test]
    fn test_cost_benefit_generated() {
        let input = make_basic_input();
        let output = optimize_treaty_structure(&input).unwrap();
        assert_eq!(output.cost_benefit.len(), 3);
    }

    #[test]
    fn test_cost_benefit_substance_matches() {
        let input = make_basic_input();
        let output = optimize_treaty_structure(&input).unwrap();
        for (i, cb) in output.cost_benefit.iter().enumerate() {
            assert_eq!(
                cb.annual_substance_cost,
                input.holding_jurisdiction_candidates[i].substance_cost_annual
            );
        }
    }

    #[test]
    fn test_cost_benefit_net_equals_saving_minus_cost() {
        let input = make_basic_input();
        let output = optimize_treaty_structure(&input).unwrap();
        for cb in &output.cost_benefit {
            assert_eq!(
                cb.net_benefit,
                cb.annual_tax_saving - cb.annual_substance_cost
            );
        }
    }

    #[test]
    fn test_cost_benefit_roi_calculation() {
        let input = make_basic_input();
        let output = optimize_treaty_structure(&input).unwrap();
        for cb in &output.cost_benefit {
            if cb.annual_substance_cost > dec!(0) {
                let expected_roi = (cb.net_benefit / cb.annual_substance_cost) * dec!(100);
                assert_eq!(cb.roi_pct, expected_roi);
            }
        }
    }

    // --- Warnings and recommendations tests ---

    #[test]
    fn test_recommendations_present() {
        let input = make_basic_input();
        let output = optimize_treaty_structure(&input).unwrap();
        assert!(!output.recommendations.is_empty());
    }

    #[test]
    fn test_substance_recommendation() {
        let input = make_basic_input();
        let output = optimize_treaty_structure(&input).unwrap();
        assert!(output
            .recommendations
            .iter()
            .any(|r| r.contains("substance")));
    }

    #[test]
    fn test_beps_recommendation() {
        let input = make_basic_input();
        let output = optimize_treaty_structure(&input).unwrap();
        assert!(output.recommendations.iter().any(|r| r.contains("BEPS")));
    }

    #[test]
    fn test_high_pe_warning() {
        let input = make_basic_input();
        let output = optimize_treaty_structure(&input).unwrap();
        // UK has high PE risk, should produce a warning
        assert!(output.warnings.iter().any(|w| w.contains("PE risk")));
    }

    #[test]
    fn test_high_cfc_risk_warning() {
        let mut input = make_basic_input();
        input.holding_jurisdiction_candidates[0].cfc_rules_risk = "High".to_string();
        // Make this candidate the cheapest so it becomes optimal
        input.holding_jurisdiction_candidates[0].substance_cost_annual = dec!(1);
        let output = optimize_treaty_structure(&input).unwrap();
        assert!(output.warnings.iter().any(|w| w.contains("CFC")));
    }

    // --- Edge cases ---

    #[test]
    fn test_zero_flows() {
        let mut input = make_basic_input();
        for op in input.operating_jurisdictions.iter_mut() {
            op.annual_dividends_up = dec!(0);
            op.annual_royalties_out = dec!(0);
            op.annual_interest_out = dec!(0);
            op.annual_management_fees_out = dec!(0);
        }
        let output = optimize_treaty_structure(&input).unwrap();
        for opt in &output.structure_options {
            assert_eq!(opt.total_tax_cost, dec!(0));
        }
    }

    #[test]
    fn test_single_operating_entity() {
        let mut input = make_basic_input();
        input.operating_jurisdictions = vec![input.operating_jurisdictions[0].clone()];
        let output = optimize_treaty_structure(&input).unwrap();
        assert_eq!(output.structure_options.len(), 3);
    }

    #[test]
    fn test_no_pe_risk_factors() {
        let mut input = make_basic_input();
        input.pe_risk_factors = vec![];
        let output = optimize_treaty_structure(&input).unwrap();
        assert!(output.pe_assessment.is_empty());
    }

    #[test]
    fn test_no_ip_box_rate() {
        let mut input = make_basic_input();
        for cand in input.holding_jurisdiction_candidates.iter_mut() {
            cand.ip_box_rate = None;
        }
        let output = optimize_treaty_structure(&input).unwrap();
        assert!(!output.structure_options.is_empty());
    }

    #[test]
    fn test_no_participation_exemption() {
        let mut input = make_basic_input();
        for cand in input.holding_jurisdiction_candidates.iter_mut() {
            cand.participation_exemption = false;
        }
        let output = optimize_treaty_structure(&input).unwrap();
        // Without participation exemption, dividend tax should be higher
        for opt in &output.structure_options {
            assert!(opt.dividend_tax > dec!(0));
        }
    }

    #[test]
    fn test_interest_deduction_limit() {
        // Interest > 30% of profit should trigger extra tax
        let mut input = make_basic_input();
        input.operating_jurisdictions = vec![OperatingEntity {
            name: "HighDebt OpCo".to_string(),
            jurisdiction: "Germany".to_string(),
            annual_profit: dec!(1_000_000),
            annual_dividends_up: dec!(0),
            annual_royalties_out: dec!(0),
            annual_interest_out: dec!(500_000), // 50% of profit, exceeds 30% limit
            annual_management_fees_out: dec!(0),
            corporate_tax_rate: dec!(0.30),
        }];
        let output = optimize_treaty_structure(&input).unwrap();
        // Interest tax should be positive due to non-deductible portion
        for opt in &output.structure_options {
            assert!(opt.interest_tax > dec!(0));
        }
    }

    #[test]
    fn test_interest_within_limit() {
        let mut input = make_basic_input();
        input.operating_jurisdictions = vec![OperatingEntity {
            name: "LowDebt OpCo".to_string(),
            jurisdiction: "Germany".to_string(),
            annual_profit: dec!(10_000_000),
            annual_dividends_up: dec!(0),
            annual_royalties_out: dec!(0),
            annual_interest_out: dec!(100_000), // Well within 30% limit
            annual_management_fees_out: dec!(0),
            corporate_tax_rate: dec!(0.30),
        }];
        let output = optimize_treaty_structure(&input).unwrap();
        // Interest tax should still be positive (WHT + corp tax at holding level)
        for opt in &output.structure_options {
            assert!(opt.interest_tax >= dec!(0));
        }
    }
}
