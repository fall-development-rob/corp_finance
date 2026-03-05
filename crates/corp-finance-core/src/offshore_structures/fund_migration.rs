use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

use crate::error::CorpFinanceError;
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Static Data — Migration Corridors
// ---------------------------------------------------------------------------

struct MigrationCorridor {
    source: &'static str,
    target: &'static str,
    mechanism: &'static str,
    statutory_basis: &'static str,
    typical_timeline_weeks: u32,
    typical_cost_usd: u32,
    investor_consent_required: bool,
    consent_threshold_pct: u32,
}

const MIGRATION_CORRIDORS: &[MigrationCorridor] = &[
    MigrationCorridor {
        source: "BVI",
        target: "Cayman",
        mechanism: "StatutoryContinuation",
        statutory_basis: "BVI BCA Part XI / Cayman Companies Act Part XVII",
        typical_timeline_weeks: 8,
        typical_cost_usd: 150_000,
        investor_consent_required: false,
        consent_threshold_pct: 0,
    },
    MigrationCorridor {
        source: "Cayman",
        target: "BVI",
        mechanism: "StatutoryContinuation",
        statutory_basis: "Cayman Companies Act Part XVII / BVI BCA Part XI",
        typical_timeline_weeks: 8,
        typical_cost_usd: 120_000,
        investor_consent_required: false,
        consent_threshold_pct: 0,
    },
    MigrationCorridor {
        source: "Cayman",
        target: "Luxembourg",
        mechanism: "SchemeOfArrangement",
        statutory_basis: "Cayman Companies Act s.86 / Lux 1915 Law",
        typical_timeline_weeks: 24,
        typical_cost_usd: 500_000,
        investor_consent_required: true,
        consent_threshold_pct: 75,
    },
    MigrationCorridor {
        source: "Cayman",
        target: "Singapore",
        mechanism: "Redomiciliation",
        statutory_basis: "VCC Act 2022 Amendment / MAS Guidelines",
        typical_timeline_weeks: 16,
        typical_cost_usd: 300_000,
        investor_consent_required: true,
        consent_threshold_pct: 75,
    },
    MigrationCorridor {
        source: "Cayman",
        target: "HongKong",
        mechanism: "Redomiciliation",
        statutory_basis: "OFC Re-domiciliation regime / SFC Code",
        typical_timeline_weeks: 20,
        typical_cost_usd: 350_000,
        investor_consent_required: true,
        consent_threshold_pct: 75,
    },
    MigrationCorridor {
        source: "Ireland",
        target: "Luxembourg",
        mechanism: "CrossBorderMerger",
        statutory_basis: "EU Cross-Border Mergers Directive 2005/56/EC / UCITS Directive",
        typical_timeline_weeks: 20,
        typical_cost_usd: 400_000,
        investor_consent_required: true,
        consent_threshold_pct: 50,
    },
    MigrationCorridor {
        source: "Luxembourg",
        target: "Ireland",
        mechanism: "CrossBorderMerger",
        statutory_basis: "EU Cross-Border Mergers Directive 2005/56/EC / UCITS Directive",
        typical_timeline_weeks: 20,
        typical_cost_usd: 400_000,
        investor_consent_required: true,
        consent_threshold_pct: 50,
    },
    MigrationCorridor {
        source: "BVI",
        target: "Luxembourg",
        mechanism: "ParallelFund",
        statutory_basis: "No direct statutory path; parallel fund + asset transfer",
        typical_timeline_weeks: 28,
        typical_cost_usd: 600_000,
        investor_consent_required: true,
        consent_threshold_pct: 100,
    },
    MigrationCorridor {
        source: "Jersey",
        target: "Luxembourg",
        mechanism: "SchemeOfArrangement",
        statutory_basis: "Jersey Companies Law Art. 125 / Lux 1915 Law",
        typical_timeline_weeks: 22,
        typical_cost_usd: 450_000,
        investor_consent_required: true,
        consent_threshold_pct: 75,
    },
    MigrationCorridor {
        source: "Guernsey",
        target: "Luxembourg",
        mechanism: "SchemeOfArrangement",
        statutory_basis: "Guernsey Companies Law s.107 / Lux 1915 Law",
        typical_timeline_weeks: 22,
        typical_cost_usd: 450_000,
        investor_consent_required: true,
        consent_threshold_pct: 75,
    },
];

// ---------------------------------------------------------------------------
// Static Data — Exit Tax Rates
// ---------------------------------------------------------------------------

struct ExitTaxRule {
    jurisdiction: &'static str,
    exit_tax_rate: u32, // basis points (2500 = 25%)
    note: &'static str,
}

const EXIT_TAX_RULES: &[ExitTaxRule] = &[
    ExitTaxRule {
        jurisdiction: "Cayman",
        exit_tax_rate: 0,
        note: "No exit tax — tax-neutral jurisdiction",
    },
    ExitTaxRule {
        jurisdiction: "BVI",
        exit_tax_rate: 0,
        note: "No exit tax — tax-neutral jurisdiction",
    },
    ExitTaxRule {
        jurisdiction: "Jersey",
        exit_tax_rate: 0,
        note: "No exit tax for exempt funds",
    },
    ExitTaxRule {
        jurisdiction: "Guernsey",
        exit_tax_rate: 0,
        note: "No exit tax for exempt funds",
    },
    ExitTaxRule {
        jurisdiction: "Singapore",
        exit_tax_rate: 0,
        note: "No exit tax under S13O/S13U exemption",
    },
    ExitTaxRule {
        jurisdiction: "HongKong",
        exit_tax_rate: 0,
        note: "No exit tax under UFE regime",
    },
    ExitTaxRule {
        jurisdiction: "DIFC",
        exit_tax_rate: 0,
        note: "No exit tax — zero tax jurisdiction",
    },
    ExitTaxRule {
        jurisdiction: "ADGM",
        exit_tax_rate: 0,
        note: "No exit tax — zero tax jurisdiction",
    },
    ExitTaxRule {
        jurisdiction: "Luxembourg",
        exit_tax_rate: 2500,
        note: "Potential CGT at 25% on unrealized gains for corporate vehicles",
    },
    ExitTaxRule {
        jurisdiction: "Ireland",
        exit_tax_rate: 2500,
        note: "Exit charge at 25% on unrealized gains",
    },
];

// ---------------------------------------------------------------------------
// Static Data — Investor Deemed Disposal Rules
// ---------------------------------------------------------------------------

struct InvestorDeemedDisposalRule {
    investor_type: &'static str,
    triggers_deemed_disposal: bool,
    note: &'static str,
}

const INVESTOR_DEEMED_DISPOSAL_RULES: &[InvestorDeemedDisposalRule] = &[
    InvestorDeemedDisposalRule {
        investor_type: "USTaxExempt",
        triggers_deemed_disposal: false,
        note: "Tax-exempt status unaffected by fund domicile change",
    },
    InvestorDeemedDisposalRule {
        investor_type: "USTaxable",
        triggers_deemed_disposal: true,
        note: "IRC Section 1248 may trigger deemed disposal on domicile change",
    },
    InvestorDeemedDisposalRule {
        investor_type: "EU_Institutional",
        triggers_deemed_disposal: false,
        note: "UCITS/AIFMD merger protections generally avoid deemed disposal",
    },
    InvestorDeemedDisposalRule {
        investor_type: "GCC_SWF",
        triggers_deemed_disposal: false,
        note: "Sovereign wealth funds typically exempt from capital gains",
    },
    InvestorDeemedDisposalRule {
        investor_type: "UK_Taxable",
        triggers_deemed_disposal: true,
        note: "HMRC may treat migration as disposal event under TCGA 1992",
    },
];

// ---------------------------------------------------------------------------
// Input Types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationFeasibilityInput {
    pub source_jurisdiction: String,
    pub source_vehicle_type: String,
    pub target_jurisdiction: String,
    pub target_vehicle_type: String,
    pub fund_size: Decimal,
    pub investor_count: u32,
    pub fund_remaining_life_years: Option<u32>,
    pub migration_driver: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostBenefitInput {
    pub source_jurisdiction: String,
    pub target_jurisdiction: String,
    pub fund_size: Decimal,
    pub one_time_migration_cost: Decimal,
    pub annual_cost_current: Decimal,
    pub annual_cost_target: Decimal,
    pub tax_cost_of_migration: Decimal,
    pub new_distribution_aum: Decimal,
    pub management_fee_rate: Decimal,
    pub remaining_fund_life_years: u32,
    pub discount_rate: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationTimelineInput {
    pub source_jurisdiction: String,
    pub target_jurisdiction: String,
    pub mechanism: String,
    pub investor_consent_required: bool,
    pub investor_count: u32,
    pub fund_size: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaxConsequenceInput {
    pub source_jurisdiction: String,
    pub target_jurisdiction: String,
    pub fund_nav: Decimal,
    pub unrealized_gains: Decimal,
    pub investor_profiles: Vec<MigrationInvestorProfile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationInvestorProfile {
    pub investor_type: String,
    pub residence: String,
    pub allocation_pct: Decimal,
}

// ---------------------------------------------------------------------------
// Output Types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationFeasibilityOutput {
    pub feasible: bool,
    pub mechanism: String,
    pub statutory_basis: String,
    pub regulatory_approvals: Vec<String>,
    pub estimated_timeline_weeks: u32,
    pub estimated_cost_usd: u32,
    pub investor_consent_required: bool,
    pub consent_threshold_pct: u32,
    pub risks: Vec<String>,
    pub alternatives_if_infeasible: Vec<String>,
    pub migration_driver_alignment: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostBenefitOutput {
    pub one_time_costs: Decimal,
    pub annual_cost_delta: Decimal,
    pub tax_cost: Decimal,
    pub new_aum_benefit_annual: Decimal,
    pub npv: Decimal,
    pub payback_period_years: Decimal,
    pub irr_of_migration: Decimal,
    pub go_no_go_recommendation: String,
    pub sensitivity: CostBenefitSensitivity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostBenefitSensitivity {
    pub npv_if_no_new_aum: Decimal,
    pub npv_if_double_costs: Decimal,
    pub breakeven_new_aum: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationTimelineOutput {
    pub phases: Vec<MigrationPhase>,
    pub total_weeks: u32,
    pub critical_path: Vec<String>,
    pub parallel_activities: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationPhase {
    pub phase_number: u32,
    pub name: String,
    pub description: String,
    pub duration_weeks: u32,
    pub dependencies: Vec<String>,
    pub key_risks: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaxConsequenceOutput {
    pub source_exit_tax: Decimal,
    pub source_exit_tax_rate_bps: u32,
    pub source_exit_note: String,
    pub target_entry_tax: Decimal,
    pub step_up_available: bool,
    pub per_investor_impact: Vec<InvestorTaxImpact>,
    pub treaty_impact_analysis: String,
    pub net_tax_cost: Decimal,
    pub timing_recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvestorTaxImpact {
    pub investor_type: String,
    pub residence: String,
    pub allocation_pct: Decimal,
    pub deemed_disposal_triggered: bool,
    pub estimated_tax_cost: Decimal,
    pub note: String,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Evaluates whether fund migration from source to target jurisdiction is
/// legally possible and identifies the appropriate mechanism.
pub fn migration_feasibility(
    input: &MigrationFeasibilityInput,
) -> CorpFinanceResult<MigrationFeasibilityOutput> {
    validate_feasibility_input(input)?;

    let mut risks: Vec<String> = Vec::new();
    let mut alternatives: Vec<String> = Vec::new();

    // Look up known corridor
    let corridor = find_corridor(&input.source_jurisdiction, &input.target_jurisdiction);

    let (
        feasible,
        mechanism,
        statutory_basis,
        timeline_weeks,
        cost_usd,
        consent_required,
        consent_threshold,
    ) = if let Some(c) = corridor {
        (
            true,
            c.mechanism.to_string(),
            c.statutory_basis.to_string(),
            c.typical_timeline_weeks,
            c.typical_cost_usd,
            c.investor_consent_required,
            c.consent_threshold_pct,
        )
    } else {
        // No known corridor — suggest alternatives
        alternatives.push(format!(
            "Parallel fund: Establish new {} fund and transfer assets",
            input.target_jurisdiction
        ));
        alternatives.push(format!(
            "Side-by-side: Run {} and {} funds concurrently during wind-down",
            input.source_jurisdiction, input.target_jurisdiction
        ));
        alternatives.push("Scheme of arrangement: Court-supervised reorganization".to_string());
        (
            false,
            "None".to_string(),
            "No statutory continuation or redomiciliation path available".to_string(),
            0,
            0,
            true,
            100,
        )
    };

    // Build regulatory approvals
    let regulatory_approvals = build_regulatory_approvals(
        &input.source_jurisdiction,
        &input.target_jurisdiction,
        &mechanism,
    );

    // Assess risks
    if input.investor_count > 100 {
        risks.push("Large investor base increases consent coordination complexity".to_string());
    }
    if input.fund_size > dec!(1_000_000_000) {
        risks.push("Fund size >$1B may attract heightened regulatory scrutiny".to_string());
    }
    if mechanism == "SchemeOfArrangement" {
        risks.push("Court approval required — timeline may extend significantly".to_string());
    }
    if mechanism == "ParallelFund" {
        risks.push(
            "Parallel fund involves asset transfer — potential tax crystallization".to_string(),
        );
    }
    if consent_required && input.investor_count > 50 {
        risks.push(format!(
            "Investor consent at {}% threshold with {} investors may be challenging",
            consent_threshold, input.investor_count
        ));
    }
    if let Some(years) = input.fund_remaining_life_years {
        if years <= 2 {
            risks.push("Short remaining fund life may not justify migration costs".to_string());
        }
    }

    // Driver alignment
    let driver_alignment = assess_driver_alignment(
        &input.migration_driver,
        &input.target_jurisdiction,
        feasible,
    );

    Ok(MigrationFeasibilityOutput {
        feasible,
        mechanism,
        statutory_basis,
        regulatory_approvals,
        estimated_timeline_weeks: timeline_weeks,
        estimated_cost_usd: cost_usd,
        investor_consent_required: consent_required,
        consent_threshold_pct: consent_threshold,
        risks,
        alternatives_if_infeasible: alternatives,
        migration_driver_alignment: driver_alignment,
    })
}

/// Calculates NPV cost-benefit of migration including tax costs,
/// annual savings, and new distribution benefits.
pub fn redomiciliation_cost_benefit(
    input: &CostBenefitInput,
) -> CorpFinanceResult<CostBenefitOutput> {
    validate_cost_benefit_input(input)?;

    let annual_cost_delta = input.annual_cost_current - input.annual_cost_target;
    let new_aum_benefit_annual = input.new_distribution_aum * input.management_fee_rate;
    let total_annual_benefit = annual_cost_delta + new_aum_benefit_annual;

    // NPV using iterative discount factor (no powd)
    let mut npv = Decimal::ZERO;
    let one_plus_r = Decimal::ONE + input.discount_rate;
    let mut discount_factor = Decimal::ONE;

    for _year in 1..=input.remaining_fund_life_years {
        discount_factor /= one_plus_r;
        npv += total_annual_benefit * discount_factor;
    }
    npv -= input.one_time_migration_cost + input.tax_cost_of_migration;

    // NPV with no new AUM (sensitivity)
    let mut npv_no_new_aum = Decimal::ZERO;
    let mut df2 = Decimal::ONE;
    for _year in 1..=input.remaining_fund_life_years {
        df2 /= one_plus_r;
        npv_no_new_aum += annual_cost_delta * df2;
    }
    npv_no_new_aum -= input.one_time_migration_cost + input.tax_cost_of_migration;

    // NPV with double costs (sensitivity)
    let double_costs = input.one_time_migration_cost * dec!(2);
    // Recalculate NPV with doubled one-time costs
    let mut npv_double = Decimal::ZERO;
    let mut df3 = Decimal::ONE;
    for _year in 1..=input.remaining_fund_life_years {
        df3 /= one_plus_r;
        npv_double += total_annual_benefit * df3;
    }
    npv_double -= double_costs + input.tax_cost_of_migration;

    // Breakeven new AUM: the new_distribution_aum that makes NPV = 0
    // NPV = PV(cost_delta + new_aum * mgmt_fee) - upfront = 0
    // PV_annuity * (cost_delta + X * mgmt_fee) = upfront
    // X = (upfront / PV_annuity - cost_delta) / mgmt_fee
    let mut pv_annuity = Decimal::ZERO;
    let mut df4 = Decimal::ONE;
    for _year in 1..=input.remaining_fund_life_years {
        df4 /= one_plus_r;
        pv_annuity += df4;
    }
    let upfront = input.one_time_migration_cost + input.tax_cost_of_migration;
    let breakeven_new_aum =
        if input.management_fee_rate > Decimal::ZERO && pv_annuity > Decimal::ZERO {
            let required_annual = upfront / pv_annuity - annual_cost_delta;
            if required_annual > Decimal::ZERO {
                required_annual / input.management_fee_rate
            } else {
                Decimal::ZERO // cost savings alone cover migration
            }
        } else {
            Decimal::ZERO
        };

    // Payback period (simple: upfront / annual benefit)
    let payback_period_years = if total_annual_benefit > Decimal::ZERO {
        upfront / total_annual_benefit
    } else {
        dec!(999) // zero or negative annual benefit => never pays back
    };

    // IRR approximation via Newton-Raphson
    let irr = compute_migration_irr(
        upfront,
        total_annual_benefit,
        input.remaining_fund_life_years,
    );

    // Recommendation
    let recommendation = if npv > Decimal::ZERO && payback_period_years < dec!(3) {
        "Go".to_string()
    } else if npv > Decimal::ZERO {
        "Conditional".to_string()
    } else {
        "NoGo".to_string()
    };

    Ok(CostBenefitOutput {
        one_time_costs: input.one_time_migration_cost,
        annual_cost_delta,
        tax_cost: input.tax_cost_of_migration,
        new_aum_benefit_annual,
        npv,
        payback_period_years,
        irr_of_migration: irr,
        go_no_go_recommendation: recommendation,
        sensitivity: CostBenefitSensitivity {
            npv_if_no_new_aum: npv_no_new_aum,
            npv_if_double_costs: npv_double,
            breakeven_new_aum,
        },
    })
}

/// Builds a phase-by-phase migration timeline with dependencies and risks.
pub fn migration_timeline(
    input: &MigrationTimelineInput,
) -> CorpFinanceResult<MigrationTimelineOutput> {
    validate_timeline_input(input)?;

    let mut phases: Vec<MigrationPhase> = Vec::new();

    // Phase 1: Preparation
    let prep_weeks = 4;
    phases.push(MigrationPhase {
        phase_number: 1,
        name: "Preparation".to_string(),
        description: "Engage legal and tax advisors in source and target jurisdictions; \
                       conduct feasibility analysis; draft migration plan"
            .to_string(),
        duration_weeks: prep_weeks,
        dependencies: vec![],
        key_risks: vec![
            "Advisor availability may delay kick-off".to_string(),
            "Hidden tax consequences discovered during due diligence".to_string(),
        ],
    });

    // Phase 2: Regulatory Application
    let reg_weeks = match input.mechanism.as_str() {
        "StatutoryContinuation" => 4,
        "Redomiciliation" => 8,
        "CrossBorderMerger" => 10,
        "SchemeOfArrangement" => 12,
        "ParallelFund" => 10,
        _ => 8,
    };
    phases.push(MigrationPhase {
        phase_number: 2,
        name: "Regulatory Application".to_string(),
        description: format!(
            "File applications with {} (outbound) and {} (inbound) regulators; \
             obtain necessary approvals and no-objection letters",
            input.source_jurisdiction, input.target_jurisdiction
        ),
        duration_weeks: reg_weeks,
        dependencies: vec!["Phase 1: Preparation".to_string()],
        key_risks: vec![
            "Regulatory review may require additional information".to_string(),
            "Processing backlog at target regulator".to_string(),
        ],
    });

    // Phase 3: Investor Notification & Consent
    let consent_weeks = if input.investor_consent_required {
        if input.investor_count > 100 {
            8
        } else if input.investor_count > 25 {
            6
        } else {
            4
        }
    } else {
        2 // notification only
    };
    let consent_desc = if input.investor_consent_required {
        format!(
            "Distribute investor notice and consent materials to {} investors; \
             collect written consents; manage objections and redemption requests",
            input.investor_count
        )
    } else {
        format!(
            "Send notification letters to {} investors; \
             allow statutory notice period for objections",
            input.investor_count
        )
    };
    phases.push(MigrationPhase {
        phase_number: 3,
        name: "Investor Notification & Consent".to_string(),
        description: consent_desc,
        duration_weeks: consent_weeks,
        dependencies: vec!["Phase 2: Regulatory Application".to_string()],
        key_risks: vec![
            "Investor objections may trigger redemptions".to_string(),
            "Consent threshold may not be met".to_string(),
        ],
    });

    // Phase 4: Operational Transition
    let ops_weeks = if input.fund_size > dec!(500_000_000) {
        6
    } else {
        4
    };
    phases.push(MigrationPhase {
        phase_number: 4,
        name: "Operational Transition".to_string(),
        description: "Appoint new service providers (administrator, auditor, custodian, \
                       legal counsel) in target jurisdiction; migrate books and records; \
                       transfer accounts and custody arrangements"
            .to_string(),
        duration_weeks: ops_weeks,
        dependencies: vec!["Phase 3: Investor Notification & Consent".to_string()],
        key_risks: vec![
            "Service provider onboarding delays".to_string(),
            "Data migration errors in NAV records".to_string(),
            "Custody transfer settlement risk".to_string(),
        ],
    });

    // Phase 5: Completion & Registration
    let completion_weeks = 2;
    phases.push(MigrationPhase {
        phase_number: 5,
        name: "Completion & Registration".to_string(),
        description: format!(
            "File final documents with {} registrar; deregister from {}; \
             issue new constitutional documents; confirm registration in {}",
            input.target_jurisdiction, input.source_jurisdiction, input.target_jurisdiction
        ),
        duration_weeks: completion_weeks,
        dependencies: vec!["Phase 4: Operational Transition".to_string()],
        key_risks: vec![
            "Final registration delays".to_string(),
            "Gap in regulatory status between deregistration and registration".to_string(),
        ],
    });

    let total_weeks = prep_weeks + reg_weeks + consent_weeks + ops_weeks + completion_weeks;

    let critical_path = phases
        .iter()
        .map(|p| format!("{} ({} weeks)", p.name, p.duration_weeks))
        .collect();

    // Parallel activities (can overlap with sequential phases)
    let mut parallel_activities = Vec::new();
    parallel_activities
        .push("Service provider RFP (can begin during Phase 2 Regulatory Application)".to_string());
    parallel_activities.push("Tax structuring advice (can run parallel to Phase 2)".to_string());
    if input.investor_consent_required {
        parallel_activities.push("Investor Q&A preparation (can begin during Phase 2)".to_string());
    }
    parallel_activities
        .push("IT/systems migration planning (can begin during Phase 3)".to_string());

    Ok(MigrationTimelineOutput {
        phases,
        total_weeks,
        critical_path,
        parallel_activities,
    })
}

/// Analyses tax consequences of fund migration for both the fund and
/// its investors across multiple residence jurisdictions.
pub fn tax_consequence_analysis(
    input: &TaxConsequenceInput,
) -> CorpFinanceResult<TaxConsequenceOutput> {
    validate_tax_input(input)?;

    // Source exit tax
    let (exit_tax_rate_bps, exit_note) = lookup_exit_tax(&input.source_jurisdiction);
    let exit_tax_rate_decimal = Decimal::from(exit_tax_rate_bps) / dec!(10_000);
    let source_exit_tax = input.unrealized_gains * exit_tax_rate_decimal;

    // Target entry tax (generally zero for fund-level)
    let target_entry_tax = Decimal::ZERO;

    // Step-up availability (generally yes for most target jurisdictions)
    let step_up_available = exit_tax_rate_bps > 0
        || matches!(
            input.target_jurisdiction.as_str(),
            "Luxembourg" | "Ireland" | "Singapore" | "HongKong" | "Cayman" | "BVI"
        );

    // Per-investor tax impact
    let per_investor_impact: Vec<InvestorTaxImpact> = input
        .investor_profiles
        .iter()
        .map(|ip| {
            let (deemed_disposal, note) = lookup_investor_deemed_disposal(&ip.investor_type);
            let investor_gains = input.unrealized_gains * ip.allocation_pct;
            let estimated_tax = if deemed_disposal {
                // Assume standard CGT rate by investor type
                let cgt_rate = estimate_investor_cgt_rate(&ip.investor_type, &ip.residence);
                investor_gains * cgt_rate
            } else {
                Decimal::ZERO
            };
            InvestorTaxImpact {
                investor_type: ip.investor_type.clone(),
                residence: ip.residence.clone(),
                allocation_pct: ip.allocation_pct,
                deemed_disposal_triggered: deemed_disposal,
                estimated_tax_cost: estimated_tax,
                note: note.to_string(),
            }
        })
        .collect();

    let total_investor_tax: Decimal = per_investor_impact
        .iter()
        .map(|i| i.estimated_tax_cost)
        .sum();

    let net_tax_cost = source_exit_tax + target_entry_tax + total_investor_tax;

    // Treaty impact analysis
    let treaty_analysis =
        build_treaty_analysis(&input.source_jurisdiction, &input.target_jurisdiction);

    // Timing recommendations
    let mut timing_recs = Vec::new();
    if source_exit_tax > Decimal::ZERO {
        timing_recs
            .push("Consider realizing losses before migration to offset exit tax".to_string());
        timing_recs.push(
            "Evaluate deferral mechanisms (e.g., contribution-in-kind to new vehicle)".to_string(),
        );
    }
    if total_investor_tax > Decimal::ZERO {
        timing_recs.push("Notify affected investors early to allow tax planning".to_string());
        timing_recs.push("Consider phased migration to spread investor tax impact".to_string());
    }
    if step_up_available {
        timing_recs.push(
            "Step-up basis available in target — future gains measured from migration date"
                .to_string(),
        );
    }
    timing_recs.push("Complete migration before fiscal year-end for clean reporting".to_string());

    Ok(TaxConsequenceOutput {
        source_exit_tax,
        source_exit_tax_rate_bps: exit_tax_rate_bps,
        source_exit_note: exit_note.to_string(),
        target_entry_tax,
        step_up_available,
        per_investor_impact,
        treaty_impact_analysis: treaty_analysis,
        net_tax_cost,
        timing_recommendations: timing_recs,
    })
}

// ---------------------------------------------------------------------------
// Validation Helpers
// ---------------------------------------------------------------------------

fn validate_feasibility_input(input: &MigrationFeasibilityInput) -> CorpFinanceResult<()> {
    if input.source_jurisdiction == input.target_jurisdiction {
        return Err(CorpFinanceError::InvalidInput {
            field: "source_jurisdiction / target_jurisdiction".to_string(),
            reason: "Source and target jurisdictions must be different".to_string(),
        });
    }
    if input.fund_size <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "fund_size".to_string(),
            reason: "Fund size must be positive".to_string(),
        });
    }
    if input.source_jurisdiction.is_empty() {
        return Err(CorpFinanceError::InvalidInput {
            field: "source_jurisdiction".to_string(),
            reason: "Source jurisdiction must not be empty".to_string(),
        });
    }
    if input.target_jurisdiction.is_empty() {
        return Err(CorpFinanceError::InvalidInput {
            field: "target_jurisdiction".to_string(),
            reason: "Target jurisdiction must not be empty".to_string(),
        });
    }
    Ok(())
}

fn validate_cost_benefit_input(input: &CostBenefitInput) -> CorpFinanceResult<()> {
    if input.source_jurisdiction == input.target_jurisdiction {
        return Err(CorpFinanceError::InvalidInput {
            field: "source_jurisdiction / target_jurisdiction".to_string(),
            reason: "Source and target jurisdictions must be different".to_string(),
        });
    }
    if input.fund_size <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "fund_size".to_string(),
            reason: "Fund size must be positive".to_string(),
        });
    }
    if input.remaining_fund_life_years < 1 {
        return Err(CorpFinanceError::InvalidInput {
            field: "remaining_fund_life_years".to_string(),
            reason: "Remaining fund life must be at least 1 year".to_string(),
        });
    }
    if input.discount_rate <= Decimal::ZERO || input.discount_rate >= Decimal::ONE {
        return Err(CorpFinanceError::InvalidInput {
            field: "discount_rate".to_string(),
            reason: "Discount rate must be between 0 and 1 (exclusive)".to_string(),
        });
    }
    Ok(())
}

fn validate_timeline_input(input: &MigrationTimelineInput) -> CorpFinanceResult<()> {
    if input.source_jurisdiction == input.target_jurisdiction {
        return Err(CorpFinanceError::InvalidInput {
            field: "source_jurisdiction / target_jurisdiction".to_string(),
            reason: "Source and target jurisdictions must be different".to_string(),
        });
    }
    if input.fund_size <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "fund_size".to_string(),
            reason: "Fund size must be positive".to_string(),
        });
    }
    if input.mechanism.is_empty() {
        return Err(CorpFinanceError::InvalidInput {
            field: "mechanism".to_string(),
            reason: "Migration mechanism must be specified".to_string(),
        });
    }
    Ok(())
}

fn validate_tax_input(input: &TaxConsequenceInput) -> CorpFinanceResult<()> {
    if input.source_jurisdiction == input.target_jurisdiction {
        return Err(CorpFinanceError::InvalidInput {
            field: "source_jurisdiction / target_jurisdiction".to_string(),
            reason: "Source and target jurisdictions must be different".to_string(),
        });
    }
    if input.fund_nav <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "fund_nav".to_string(),
            reason: "Fund NAV must be positive".to_string(),
        });
    }
    if input.unrealized_gains < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "unrealized_gains".to_string(),
            reason: "Unrealized gains cannot be negative".to_string(),
        });
    }
    // Validate investor allocation sums to ~1.0
    if !input.investor_profiles.is_empty() {
        let total_alloc: Decimal = input
            .investor_profiles
            .iter()
            .map(|ip| ip.allocation_pct)
            .sum();
        let diff = (total_alloc - Decimal::ONE).abs();
        if diff > dec!(0.01) {
            return Err(CorpFinanceError::InvalidInput {
                field: "investor_profiles.allocation_pct".to_string(),
                reason: format!(
                    "Investor allocation percentages sum to {} (must be ~1.0, tolerance 0.01)",
                    total_alloc
                ),
            });
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Private Helpers
// ---------------------------------------------------------------------------

fn find_corridor(source: &str, target: &str) -> Option<&'static MigrationCorridor> {
    MIGRATION_CORRIDORS
        .iter()
        .find(|c| c.source == source && c.target == target)
}

fn build_regulatory_approvals(source: &str, target: &str, mechanism: &str) -> Vec<String> {
    let mut approvals = Vec::new();

    // Source jurisdiction outbound
    match source {
        "Cayman" => approvals.push("CIMA no-objection letter for outbound migration".to_string()),
        "BVI" => approvals.push("BVI FSC approval for discontinuation".to_string()),
        "Ireland" => approvals.push("Central Bank of Ireland deauthorization".to_string()),
        "Luxembourg" => approvals.push("CSSF notification of fund closure/migration".to_string()),
        "Jersey" => approvals.push("JFSC consent for migration".to_string()),
        "Guernsey" => approvals.push("GFSC consent for migration".to_string()),
        _ => approvals.push(format!("{} regulator outbound approval", source)),
    }

    // Target jurisdiction inbound
    match target {
        "Luxembourg" => {
            approvals.push("CSSF authorization for new/continuing fund".to_string());
            approvals.push("CSSF approval of service providers".to_string());
        }
        "Singapore" => {
            approvals.push("MAS authorization for VCC registration".to_string());
            approvals.push("MAS approval of fund manager".to_string());
        }
        "HongKong" => {
            approvals.push("SFC authorization for OFC".to_string());
            approvals.push("SFC approval of investment manager".to_string());
        }
        "Cayman" => approvals.push("CIMA registration of incoming fund".to_string()),
        "BVI" => approvals.push("BVI FSC registration of continued entity".to_string()),
        "Ireland" => {
            approvals.push("Central Bank of Ireland authorization".to_string());
            approvals.push("CBI approval of AIFM/management company".to_string());
        }
        _ => approvals.push(format!("{} regulator inbound approval", target)),
    }

    // Mechanism-specific
    match mechanism {
        "SchemeOfArrangement" => {
            approvals.push("Court approval of scheme of arrangement".to_string());
        }
        "CrossBorderMerger" => {
            approvals
                .push("EU cross-border merger certificate from both jurisdictions".to_string());
        }
        _ => {}
    }

    approvals
}

fn assess_driver_alignment(driver: &str, target: &str, feasible: bool) -> String {
    if !feasible {
        return "Migration not feasible — driver cannot be achieved through this corridor"
            .to_string();
    }
    match driver {
        "EUPassport" => {
            if matches!(target, "Luxembourg" | "Ireland") {
                "Strong alignment: EU passport available in target jurisdiction".to_string()
            } else {
                "Weak alignment: target jurisdiction does not provide EU passport".to_string()
            }
        }
        "CostReduction" => {
            "Moderate alignment: cost benefit depends on specific fee comparison".to_string()
        }
        "SubstanceUpgrade" => {
            if matches!(target, "Luxembourg" | "Singapore" | "HongKong" | "Ireland") {
                "Strong alignment: target offers robust substance infrastructure".to_string()
            } else {
                "Moderate alignment: evaluate substance requirements in target".to_string()
            }
        }
        "TaxOptimization" => {
            "Moderate alignment: tax benefit requires detailed analysis of investor base"
                .to_string()
        }
        _ => format!(
            "Driver '{}' alignment requires case-specific evaluation",
            driver
        ),
    }
}

fn lookup_exit_tax(jurisdiction: &str) -> (u32, &'static str) {
    EXIT_TAX_RULES
        .iter()
        .find(|r| r.jurisdiction == jurisdiction)
        .map(|r| (r.exit_tax_rate, r.note))
        .unwrap_or((0, "No specific exit tax data — assume zero"))
}

fn lookup_investor_deemed_disposal(investor_type: &str) -> (bool, &'static str) {
    INVESTOR_DEEMED_DISPOSAL_RULES
        .iter()
        .find(|r| r.investor_type == investor_type)
        .map(|r| (r.triggers_deemed_disposal, r.note))
        .unwrap_or((false, "Unknown investor type — no deemed disposal assumed"))
}

fn estimate_investor_cgt_rate(investor_type: &str, _residence: &str) -> Decimal {
    match investor_type {
        "USTaxable" => dec!(0.238), // 20% + 3.8% NIIT
        "UK_Taxable" => dec!(0.20), // UK CGT rate for non-residential assets
        _ => dec!(0.15),            // conservative default
    }
}

fn build_treaty_analysis(source: &str, target: &str) -> String {
    match (source, target) {
        ("Cayman", "Luxembourg") => {
            "Cayman has no tax treaties; Luxembourg has 80+ treaties. Migration \
             may improve WHT rates on dividends from portfolio companies in treaty \
             jurisdictions. EU Parent-Subsidiary Directive may eliminate WHT on \
             EU-source dividends."
                .to_string()
        }
        ("Cayman", "Singapore") => {
            "Singapore has 90+ treaties with competitive WHT rates. Migration \
             may improve tax efficiency for Asia-Pacific investments."
                .to_string()
        }
        ("Cayman", "HongKong") => "Hong Kong has limited but growing treaty network (40+). \
             No tax on capital gains or dividends. Migration mainly benefits \
             investors seeking onshore Asian domicile."
            .to_string(),
        ("BVI", "Cayman") => "Both BVI and Cayman are tax-neutral. No material treaty impact. \
             Migration is primarily for regulatory or operational reasons."
            .to_string(),
        ("Ireland", "Luxembourg") | ("Luxembourg", "Ireland") => {
            "Both are EU member states with extensive treaty networks. \
             EU directives (Parent-Subsidiary, Interest & Royalties) apply equally. \
             Minimal treaty impact from migration."
                .to_string()
        }
        _ => {
            format!(
                "Treaty impact of {} to {} migration requires specific analysis \
                 based on portfolio composition and investor residence jurisdictions",
                source, target
            )
        }
    }
}

/// Newton-Raphson IRR for migration cash flows:
/// Year 0: -(one_time_costs + tax_cost), Years 1..N: annual_benefit
fn compute_migration_irr(upfront: Decimal, annual_benefit: Decimal, years: u32) -> Decimal {
    if annual_benefit <= Decimal::ZERO || upfront <= Decimal::ZERO {
        return Decimal::ZERO;
    }

    let mut r = dec!(0.10); // initial guess 10%

    for _iter in 0..50 {
        let one_plus_r = Decimal::ONE + r;
        if one_plus_r <= Decimal::ZERO {
            r = dec!(0.10);
            continue;
        }

        let mut npv = -upfront;
        let mut dnpv = Decimal::ZERO; // derivative
        let mut df = Decimal::ONE;

        for t in 1..=years {
            df /= one_plus_r;
            npv += annual_benefit * df;
            // d(CF*df)/dr = -t * CF * df / (1+r)
            let t_dec = Decimal::from(t);
            dnpv -= t_dec * annual_benefit * df / one_plus_r;
        }

        if dnpv.abs() < dec!(0.000001) {
            break;
        }

        let step = npv / dnpv;
        r -= step;

        // Clamp to reasonable range
        if r < dec!(-0.50) {
            r = dec!(-0.50);
        }
        if r > dec!(10.0) {
            r = dec!(10.0);
        }

        if step.abs() < dec!(0.00001) {
            break;
        }
    }

    // Round to 6 decimal places
    r.round_dp(6)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    // ------------------------------------------------------------------
    // Helper constructors
    // ------------------------------------------------------------------

    fn basic_feasibility_input() -> MigrationFeasibilityInput {
        MigrationFeasibilityInput {
            source_jurisdiction: "Cayman".to_string(),
            source_vehicle_type: "ExemptedLP".to_string(),
            target_jurisdiction: "Luxembourg".to_string(),
            target_vehicle_type: "SCSp".to_string(),
            fund_size: dec!(500_000_000),
            investor_count: 50,
            fund_remaining_life_years: Some(7),
            migration_driver: "EUPassport".to_string(),
        }
    }

    fn basic_cost_benefit_input() -> CostBenefitInput {
        CostBenefitInput {
            source_jurisdiction: "Cayman".to_string(),
            target_jurisdiction: "Luxembourg".to_string(),
            fund_size: dec!(500_000_000),
            one_time_migration_cost: dec!(500_000),
            annual_cost_current: dec!(1_200_000),
            annual_cost_target: dec!(900_000),
            tax_cost_of_migration: dec!(0),
            new_distribution_aum: dec!(100_000_000),
            management_fee_rate: dec!(0.015),
            remaining_fund_life_years: 7,
            discount_rate: dec!(0.08),
        }
    }

    fn basic_timeline_input() -> MigrationTimelineInput {
        MigrationTimelineInput {
            source_jurisdiction: "Cayman".to_string(),
            target_jurisdiction: "Luxembourg".to_string(),
            mechanism: "SchemeOfArrangement".to_string(),
            investor_consent_required: true,
            investor_count: 50,
            fund_size: dec!(500_000_000),
        }
    }

    fn basic_tax_input() -> TaxConsequenceInput {
        TaxConsequenceInput {
            source_jurisdiction: "Cayman".to_string(),
            target_jurisdiction: "Luxembourg".to_string(),
            fund_nav: dec!(500_000_000),
            unrealized_gains: dec!(50_000_000),
            investor_profiles: vec![
                MigrationInvestorProfile {
                    investor_type: "USTaxExempt".to_string(),
                    residence: "US".to_string(),
                    allocation_pct: dec!(0.40),
                },
                MigrationInvestorProfile {
                    investor_type: "EU_Institutional".to_string(),
                    residence: "Germany".to_string(),
                    allocation_pct: dec!(0.35),
                },
                MigrationInvestorProfile {
                    investor_type: "USTaxable".to_string(),
                    residence: "US".to_string(),
                    allocation_pct: dec!(0.25),
                },
            ],
        }
    }

    // ==================================================================
    // migration_feasibility tests
    // ==================================================================

    // ------------------------------------------------------------------
    // 1. Cayman -> Luxembourg feasibility (scheme of arrangement)
    // ------------------------------------------------------------------
    #[test]
    fn test_cayman_to_lux_feasibility() {
        let input = basic_feasibility_input();
        let result = migration_feasibility(&input).unwrap();

        assert!(result.feasible);
        assert_eq!(result.mechanism, "SchemeOfArrangement");
        assert_eq!(result.estimated_timeline_weeks, 24);
        assert!(result.investor_consent_required);
        assert_eq!(result.consent_threshold_pct, 75);
        assert!(result.alternatives_if_infeasible.is_empty());
    }

    // ------------------------------------------------------------------
    // 2. BVI -> Cayman statutory continuation
    // ------------------------------------------------------------------
    #[test]
    fn test_bvi_to_cayman_continuation() {
        let mut input = basic_feasibility_input();
        input.source_jurisdiction = "BVI".to_string();
        input.target_jurisdiction = "Cayman".to_string();
        let result = migration_feasibility(&input).unwrap();

        assert!(result.feasible);
        assert_eq!(result.mechanism, "StatutoryContinuation");
        assert_eq!(result.estimated_timeline_weeks, 8);
        assert!(!result.investor_consent_required);
    }

    // ------------------------------------------------------------------
    // 3. Cayman -> Singapore VCC redomiciliation
    // ------------------------------------------------------------------
    #[test]
    fn test_cayman_to_singapore_redomiciliation() {
        let mut input = basic_feasibility_input();
        input.target_jurisdiction = "Singapore".to_string();
        input.target_vehicle_type = "VCC".to_string();
        let result = migration_feasibility(&input).unwrap();

        assert!(result.feasible);
        assert_eq!(result.mechanism, "Redomiciliation");
        assert_eq!(result.estimated_timeline_weeks, 16);
        assert!(result.investor_consent_required);
    }

    // ------------------------------------------------------------------
    // 4. Cayman -> Hong Kong OFC redomiciliation
    // ------------------------------------------------------------------
    #[test]
    fn test_cayman_to_hk_redomiciliation() {
        let mut input = basic_feasibility_input();
        input.target_jurisdiction = "HongKong".to_string();
        let result = migration_feasibility(&input).unwrap();

        assert!(result.feasible);
        assert_eq!(result.mechanism, "Redomiciliation");
        assert_eq!(result.estimated_timeline_weeks, 20);
    }

    // ------------------------------------------------------------------
    // 5. Ireland -> Luxembourg cross-border merger
    // ------------------------------------------------------------------
    #[test]
    fn test_ireland_to_lux_cross_border_merger() {
        let mut input = basic_feasibility_input();
        input.source_jurisdiction = "Ireland".to_string();
        input.source_vehicle_type = "ICAV".to_string();
        input.target_jurisdiction = "Luxembourg".to_string();
        let result = migration_feasibility(&input).unwrap();

        assert!(result.feasible);
        assert_eq!(result.mechanism, "CrossBorderMerger");
        assert_eq!(result.estimated_timeline_weeks, 20);
        assert_eq!(result.consent_threshold_pct, 50);
    }

    // ------------------------------------------------------------------
    // 6. Unknown corridor — infeasible with alternatives
    // ------------------------------------------------------------------
    #[test]
    fn test_unknown_corridor_infeasible() {
        let mut input = basic_feasibility_input();
        input.source_jurisdiction = "Bermuda".to_string();
        input.target_jurisdiction = "Mauritius".to_string();
        let result = migration_feasibility(&input).unwrap();

        assert!(!result.feasible);
        assert_eq!(result.mechanism, "None");
        assert!(!result.alternatives_if_infeasible.is_empty());
        assert!(result.alternatives_if_infeasible.len() >= 2);
    }

    // ------------------------------------------------------------------
    // 7. Same jurisdiction error
    // ------------------------------------------------------------------
    #[test]
    fn test_same_jurisdiction_error() {
        let mut input = basic_feasibility_input();
        input.target_jurisdiction = "Cayman".to_string();
        let result = migration_feasibility(&input);

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("must be different"));
    }

    // ------------------------------------------------------------------
    // 8. Zero fund size error
    // ------------------------------------------------------------------
    #[test]
    fn test_zero_fund_size_error() {
        let mut input = basic_feasibility_input();
        input.fund_size = Decimal::ZERO;
        let result = migration_feasibility(&input);

        assert!(result.is_err());
    }

    // ------------------------------------------------------------------
    // 9. Large investor count risk warning
    // ------------------------------------------------------------------
    #[test]
    fn test_large_investor_count_risk() {
        let mut input = basic_feasibility_input();
        input.investor_count = 200;
        let result = migration_feasibility(&input).unwrap();

        assert!(result
            .risks
            .iter()
            .any(|r| r.contains("Large investor base")));
    }

    // ------------------------------------------------------------------
    // 10. Large fund size risk warning
    // ------------------------------------------------------------------
    #[test]
    fn test_large_fund_size_risk() {
        let mut input = basic_feasibility_input();
        input.fund_size = dec!(2_000_000_000);
        let result = migration_feasibility(&input).unwrap();

        assert!(result.risks.iter().any(|r| r.contains(">$1B")));
    }

    // ------------------------------------------------------------------
    // 11. Short remaining life risk
    // ------------------------------------------------------------------
    #[test]
    fn test_short_remaining_life_risk() {
        let mut input = basic_feasibility_input();
        input.fund_remaining_life_years = Some(1);
        let result = migration_feasibility(&input).unwrap();

        assert!(result
            .risks
            .iter()
            .any(|r| r.contains("Short remaining fund life")));
    }

    // ------------------------------------------------------------------
    // 12. EU passport driver alignment
    // ------------------------------------------------------------------
    #[test]
    fn test_eu_passport_driver_alignment() {
        let input = basic_feasibility_input();
        let result = migration_feasibility(&input).unwrap();

        assert!(result
            .migration_driver_alignment
            .contains("Strong alignment"));
    }

    // ------------------------------------------------------------------
    // 13. EU passport weak alignment (non-EU target)
    // ------------------------------------------------------------------
    #[test]
    fn test_eu_passport_weak_alignment_non_eu() {
        let mut input = basic_feasibility_input();
        input.target_jurisdiction = "Singapore".to_string();
        let result = migration_feasibility(&input).unwrap();

        assert!(result.migration_driver_alignment.contains("Weak alignment"));
    }

    // ------------------------------------------------------------------
    // 14. Regulatory approvals include source and target
    // ------------------------------------------------------------------
    #[test]
    fn test_regulatory_approvals_source_and_target() {
        let input = basic_feasibility_input();
        let result = migration_feasibility(&input).unwrap();

        assert!(result
            .regulatory_approvals
            .iter()
            .any(|a| a.contains("CIMA")));
        assert!(result
            .regulatory_approvals
            .iter()
            .any(|a| a.contains("CSSF")));
    }

    // ------------------------------------------------------------------
    // 15. Scheme of arrangement court approval
    // ------------------------------------------------------------------
    #[test]
    fn test_scheme_requires_court_approval() {
        let input = basic_feasibility_input();
        let result = migration_feasibility(&input).unwrap();

        assert!(result
            .regulatory_approvals
            .iter()
            .any(|a| a.contains("Court approval")));
    }

    // ------------------------------------------------------------------
    // 16. BVI -> Luxembourg parallel fund
    // ------------------------------------------------------------------
    #[test]
    fn test_bvi_to_lux_parallel_fund() {
        let mut input = basic_feasibility_input();
        input.source_jurisdiction = "BVI".to_string();
        input.target_jurisdiction = "Luxembourg".to_string();
        let result = migration_feasibility(&input).unwrap();

        assert!(result.feasible);
        assert_eq!(result.mechanism, "ParallelFund");
        assert_eq!(result.consent_threshold_pct, 100);
    }

    // ==================================================================
    // redomiciliation_cost_benefit tests
    // ==================================================================

    // ------------------------------------------------------------------
    // 17. Positive NPV — Go recommendation
    // ------------------------------------------------------------------
    #[test]
    fn test_cost_benefit_positive_npv_go() {
        let input = basic_cost_benefit_input();
        let result = redomiciliation_cost_benefit(&input).unwrap();

        assert!(result.npv > Decimal::ZERO);
        assert_eq!(result.go_no_go_recommendation, "Go");
        assert!(result.annual_cost_delta > Decimal::ZERO);
        assert!(result.new_aum_benefit_annual > Decimal::ZERO);
    }

    // ------------------------------------------------------------------
    // 18. Negative NPV — NoGo recommendation
    // ------------------------------------------------------------------
    #[test]
    fn test_cost_benefit_negative_npv_nogo() {
        let mut input = basic_cost_benefit_input();
        input.one_time_migration_cost = dec!(50_000_000);
        input.tax_cost_of_migration = dec!(20_000_000);
        input.new_distribution_aum = Decimal::ZERO;
        input.annual_cost_target = dec!(1_200_000); // same as current = no savings
        let result = redomiciliation_cost_benefit(&input).unwrap();

        assert!(result.npv < Decimal::ZERO);
        assert_eq!(result.go_no_go_recommendation, "NoGo");
    }

    // ------------------------------------------------------------------
    // 19. Conditional recommendation (positive NPV, long payback)
    // ------------------------------------------------------------------
    #[test]
    fn test_cost_benefit_conditional() {
        let mut input = basic_cost_benefit_input();
        input.one_time_migration_cost = dec!(5_000_000);
        input.new_distribution_aum = dec!(50_000_000);
        input.remaining_fund_life_years = 7;
        let result = redomiciliation_cost_benefit(&input).unwrap();

        // Should be positive NPV but payback > 3 years
        if result.npv > Decimal::ZERO && result.payback_period_years > dec!(3) {
            assert_eq!(result.go_no_go_recommendation, "Conditional");
        }
    }

    // ------------------------------------------------------------------
    // 20. Annual cost delta calculation
    // ------------------------------------------------------------------
    #[test]
    fn test_annual_cost_delta() {
        let input = basic_cost_benefit_input();
        let result = redomiciliation_cost_benefit(&input).unwrap();

        let expected_delta = dec!(1_200_000) - dec!(900_000);
        assert_eq!(result.annual_cost_delta, expected_delta);
    }

    // ------------------------------------------------------------------
    // 21. New AUM benefit annual
    // ------------------------------------------------------------------
    #[test]
    fn test_new_aum_benefit_annual() {
        let input = basic_cost_benefit_input();
        let result = redomiciliation_cost_benefit(&input).unwrap();

        let expected = dec!(100_000_000) * dec!(0.015);
        assert_eq!(result.new_aum_benefit_annual, expected);
    }

    // ------------------------------------------------------------------
    // 22. NPV sensitivity — no new AUM
    // ------------------------------------------------------------------
    #[test]
    fn test_npv_sensitivity_no_new_aum() {
        let input = basic_cost_benefit_input();
        let result = redomiciliation_cost_benefit(&input).unwrap();

        // NPV without new AUM should be less than full NPV
        assert!(result.sensitivity.npv_if_no_new_aum < result.npv);
    }

    // ------------------------------------------------------------------
    // 23. NPV sensitivity — double costs
    // ------------------------------------------------------------------
    #[test]
    fn test_npv_sensitivity_double_costs() {
        let input = basic_cost_benefit_input();
        let result = redomiciliation_cost_benefit(&input).unwrap();

        // Double costs NPV should be less than base NPV
        assert!(result.sensitivity.npv_if_double_costs < result.npv);
        let diff = result.npv - result.sensitivity.npv_if_double_costs;
        assert_eq!(diff, input.one_time_migration_cost);
    }

    // ------------------------------------------------------------------
    // 24. Payback period
    // ------------------------------------------------------------------
    #[test]
    fn test_payback_period() {
        let input = basic_cost_benefit_input();
        let result = redomiciliation_cost_benefit(&input).unwrap();

        assert!(result.payback_period_years > Decimal::ZERO);
        assert!(result.payback_period_years < dec!(10));
    }

    // ------------------------------------------------------------------
    // 25. IRR positive for good deal
    // ------------------------------------------------------------------
    #[test]
    fn test_irr_positive() {
        let input = basic_cost_benefit_input();
        let result = redomiciliation_cost_benefit(&input).unwrap();

        assert!(result.irr_of_migration > Decimal::ZERO);
    }

    // ------------------------------------------------------------------
    // 26. Discount rate validation
    // ------------------------------------------------------------------
    #[test]
    fn test_discount_rate_zero_error() {
        let mut input = basic_cost_benefit_input();
        input.discount_rate = Decimal::ZERO;
        let result = redomiciliation_cost_benefit(&input);

        assert!(result.is_err());
    }

    // ------------------------------------------------------------------
    // 27. Discount rate >= 1 error
    // ------------------------------------------------------------------
    #[test]
    fn test_discount_rate_one_error() {
        let mut input = basic_cost_benefit_input();
        input.discount_rate = Decimal::ONE;
        let result = redomiciliation_cost_benefit(&input);

        assert!(result.is_err());
    }

    // ------------------------------------------------------------------
    // 28. Remaining life years zero error
    // ------------------------------------------------------------------
    #[test]
    fn test_remaining_life_zero_error() {
        let mut input = basic_cost_benefit_input();
        input.remaining_fund_life_years = 0;
        let result = redomiciliation_cost_benefit(&input);

        assert!(result.is_err());
    }

    // ------------------------------------------------------------------
    // 29. Breakeven new AUM
    // ------------------------------------------------------------------
    #[test]
    fn test_breakeven_new_aum() {
        let mut input = basic_cost_benefit_input();
        // Scenario where cost savings alone do not cover
        input.annual_cost_target = input.annual_cost_current; // no savings
        input.new_distribution_aum = dec!(100_000_000);
        let result = redomiciliation_cost_benefit(&input).unwrap();

        assert!(result.sensitivity.breakeven_new_aum > Decimal::ZERO);
    }

    // ==================================================================
    // migration_timeline tests
    // ==================================================================

    // ------------------------------------------------------------------
    // 30. Timeline has 5 phases
    // ------------------------------------------------------------------
    #[test]
    fn test_timeline_five_phases() {
        let input = basic_timeline_input();
        let result = migration_timeline(&input).unwrap();

        assert_eq!(result.phases.len(), 5);
    }

    // ------------------------------------------------------------------
    // 31. Phase ordering
    // ------------------------------------------------------------------
    #[test]
    fn test_timeline_phase_ordering() {
        let input = basic_timeline_input();
        let result = migration_timeline(&input).unwrap();

        for (i, phase) in result.phases.iter().enumerate() {
            assert_eq!(phase.phase_number, (i + 1) as u32);
        }
    }

    // ------------------------------------------------------------------
    // 32. Total weeks sum
    // ------------------------------------------------------------------
    #[test]
    fn test_timeline_total_weeks() {
        let input = basic_timeline_input();
        let result = migration_timeline(&input).unwrap();

        let sum: u32 = result.phases.iter().map(|p| p.duration_weeks).sum();
        assert_eq!(result.total_weeks, sum);
    }

    // ------------------------------------------------------------------
    // 33. Statutory continuation shorter timeline
    // ------------------------------------------------------------------
    #[test]
    fn test_timeline_statutory_continuation_shorter() {
        let mut input = basic_timeline_input();
        input.mechanism = "StatutoryContinuation".to_string();
        input.investor_consent_required = false;
        let result = migration_timeline(&input).unwrap();

        // Statutory continuation should be faster than scheme
        assert!(result.total_weeks < 24);
    }

    // ------------------------------------------------------------------
    // 34. Large investor count increases consent phase
    // ------------------------------------------------------------------
    #[test]
    fn test_timeline_large_investor_consent() {
        let mut input = basic_timeline_input();
        input.investor_count = 200;
        let result_large = migration_timeline(&input).unwrap();

        input.investor_count = 10;
        let result_small = migration_timeline(&input).unwrap();

        // Phase 3 (consent) should be longer for large investor base
        assert!(result_large.phases[2].duration_weeks > result_small.phases[2].duration_weeks);
    }

    // ------------------------------------------------------------------
    // 35. Parallel activities populated
    // ------------------------------------------------------------------
    #[test]
    fn test_timeline_parallel_activities() {
        let input = basic_timeline_input();
        let result = migration_timeline(&input).unwrap();

        assert!(!result.parallel_activities.is_empty());
    }

    // ------------------------------------------------------------------
    // 36. Phase dependencies chain correctly
    // ------------------------------------------------------------------
    #[test]
    fn test_timeline_dependencies() {
        let input = basic_timeline_input();
        let result = migration_timeline(&input).unwrap();

        // Phase 1 has no deps
        assert!(result.phases[0].dependencies.is_empty());
        // Each subsequent phase depends on previous
        for i in 1..result.phases.len() {
            assert!(!result.phases[i].dependencies.is_empty());
        }
    }

    // ------------------------------------------------------------------
    // 37. Timeline same jurisdiction error
    // ------------------------------------------------------------------
    #[test]
    fn test_timeline_same_jurisdiction_error() {
        let mut input = basic_timeline_input();
        input.target_jurisdiction = input.source_jurisdiction.clone();
        let result = migration_timeline(&input);

        assert!(result.is_err());
    }

    // ------------------------------------------------------------------
    // 38. Large fund extends ops phase
    // ------------------------------------------------------------------
    #[test]
    fn test_timeline_large_fund_ops_phase() {
        let mut input = basic_timeline_input();
        input.fund_size = dec!(1_000_000_000);
        let result_large = migration_timeline(&input).unwrap();

        input.fund_size = dec!(100_000_000);
        let result_small = migration_timeline(&input).unwrap();

        assert!(result_large.phases[3].duration_weeks >= result_small.phases[3].duration_weeks);
    }

    // ==================================================================
    // tax_consequence_analysis tests
    // ==================================================================

    // ------------------------------------------------------------------
    // 39. Cayman exit tax is zero
    // ------------------------------------------------------------------
    #[test]
    fn test_cayman_exit_tax_zero() {
        let input = basic_tax_input();
        let result = tax_consequence_analysis(&input).unwrap();

        assert_eq!(result.source_exit_tax, Decimal::ZERO);
        assert_eq!(result.source_exit_tax_rate_bps, 0);
    }

    // ------------------------------------------------------------------
    // 40. Luxembourg exit tax on unrealized gains
    // ------------------------------------------------------------------
    #[test]
    fn test_lux_exit_tax() {
        let mut input = basic_tax_input();
        input.source_jurisdiction = "Luxembourg".to_string();
        input.target_jurisdiction = "Ireland".to_string();
        input.unrealized_gains = dec!(50_000_000);
        let result = tax_consequence_analysis(&input).unwrap();

        let expected_tax = dec!(50_000_000) * dec!(0.25);
        assert_eq!(result.source_exit_tax, expected_tax);
        assert_eq!(result.source_exit_tax_rate_bps, 2500);
    }

    // ------------------------------------------------------------------
    // 41. Ireland exit charge
    // ------------------------------------------------------------------
    #[test]
    fn test_ireland_exit_charge() {
        let mut input = basic_tax_input();
        input.source_jurisdiction = "Ireland".to_string();
        input.target_jurisdiction = "Luxembourg".to_string();
        input.unrealized_gains = dec!(30_000_000);
        let result = tax_consequence_analysis(&input).unwrap();

        let expected_tax = dec!(30_000_000) * dec!(0.25);
        assert_eq!(result.source_exit_tax, expected_tax);
    }

    // ------------------------------------------------------------------
    // 42. US tax-exempt no deemed disposal
    // ------------------------------------------------------------------
    #[test]
    fn test_us_tax_exempt_no_deemed_disposal() {
        let input = basic_tax_input();
        let result = tax_consequence_analysis(&input).unwrap();

        let us_exempt = result
            .per_investor_impact
            .iter()
            .find(|i| i.investor_type == "USTaxExempt")
            .unwrap();
        assert!(!us_exempt.deemed_disposal_triggered);
        assert_eq!(us_exempt.estimated_tax_cost, Decimal::ZERO);
    }

    // ------------------------------------------------------------------
    // 43. US taxable deemed disposal triggered
    // ------------------------------------------------------------------
    #[test]
    fn test_us_taxable_deemed_disposal() {
        let input = basic_tax_input();
        let result = tax_consequence_analysis(&input).unwrap();

        let us_taxable = result
            .per_investor_impact
            .iter()
            .find(|i| i.investor_type == "USTaxable")
            .unwrap();
        assert!(us_taxable.deemed_disposal_triggered);
        assert!(us_taxable.estimated_tax_cost > Decimal::ZERO);
    }

    // ------------------------------------------------------------------
    // 44. EU institutional no deemed disposal
    // ------------------------------------------------------------------
    #[test]
    fn test_eu_institutional_no_deemed_disposal() {
        let input = basic_tax_input();
        let result = tax_consequence_analysis(&input).unwrap();

        let eu_inst = result
            .per_investor_impact
            .iter()
            .find(|i| i.investor_type == "EU_Institutional")
            .unwrap();
        assert!(!eu_inst.deemed_disposal_triggered);
    }

    // ------------------------------------------------------------------
    // 45. Step-up available
    // ------------------------------------------------------------------
    #[test]
    fn test_step_up_available() {
        let input = basic_tax_input();
        let result = tax_consequence_analysis(&input).unwrap();

        assert!(result.step_up_available);
    }

    // ------------------------------------------------------------------
    // 46. Treaty impact analysis populated
    // ------------------------------------------------------------------
    #[test]
    fn test_treaty_impact_analysis() {
        let input = basic_tax_input();
        let result = tax_consequence_analysis(&input).unwrap();

        assert!(!result.treaty_impact_analysis.is_empty());
        assert!(
            result.treaty_impact_analysis.contains("treaty")
                || result.treaty_impact_analysis.contains("Treaty")
        );
    }

    // ------------------------------------------------------------------
    // 47. Investor allocation validation
    // ------------------------------------------------------------------
    #[test]
    fn test_investor_allocation_validation() {
        let mut input = basic_tax_input();
        input.investor_profiles[0].allocation_pct = dec!(0.80);
        // Now sums to 0.80 + 0.35 + 0.25 = 1.40
        let result = tax_consequence_analysis(&input);

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("allocation"));
    }

    // ------------------------------------------------------------------
    // 48. Timing recommendations with exit tax
    // ------------------------------------------------------------------
    #[test]
    fn test_timing_recommendations_with_exit_tax() {
        let mut input = basic_tax_input();
        input.source_jurisdiction = "Luxembourg".to_string();
        input.target_jurisdiction = "Ireland".to_string();
        let result = tax_consequence_analysis(&input).unwrap();

        assert!(result
            .timing_recommendations
            .iter()
            .any(|r| r.contains("losses")));
    }

    // ------------------------------------------------------------------
    // 49. Net tax cost aggregation
    // ------------------------------------------------------------------
    #[test]
    fn test_net_tax_cost_aggregation() {
        let input = basic_tax_input();
        let result = tax_consequence_analysis(&input).unwrap();

        let investor_tax: Decimal = result
            .per_investor_impact
            .iter()
            .map(|i| i.estimated_tax_cost)
            .sum();
        let expected_net = result.source_exit_tax + result.target_entry_tax + investor_tax;
        assert_eq!(result.net_tax_cost, expected_net);
    }

    // ------------------------------------------------------------------
    // 50. BVI exit tax is zero
    // ------------------------------------------------------------------
    #[test]
    fn test_bvi_exit_tax_zero() {
        let mut input = basic_tax_input();
        input.source_jurisdiction = "BVI".to_string();
        input.target_jurisdiction = "Cayman".to_string();
        let result = tax_consequence_analysis(&input).unwrap();

        assert_eq!(result.source_exit_tax, Decimal::ZERO);
    }

    // ------------------------------------------------------------------
    // 51. Negative unrealized gains error
    // ------------------------------------------------------------------
    #[test]
    fn test_negative_unrealized_gains_error() {
        let mut input = basic_tax_input();
        input.unrealized_gains = dec!(-1_000_000);
        let result = tax_consequence_analysis(&input);

        assert!(result.is_err());
    }

    // ------------------------------------------------------------------
    // 52. Zero NAV error
    // ------------------------------------------------------------------
    #[test]
    fn test_zero_nav_error() {
        let mut input = basic_tax_input();
        input.fund_nav = Decimal::ZERO;
        let result = tax_consequence_analysis(&input);

        assert!(result.is_err());
    }

    // ------------------------------------------------------------------
    // 53. Empty jurisdiction error
    // ------------------------------------------------------------------
    #[test]
    fn test_empty_source_jurisdiction_error() {
        let mut input = basic_feasibility_input();
        input.source_jurisdiction = "".to_string();
        let result = migration_feasibility(&input);

        assert!(result.is_err());
    }

    // ------------------------------------------------------------------
    // 54. GCC SWF no deemed disposal
    // ------------------------------------------------------------------
    #[test]
    fn test_gcc_swf_no_deemed_disposal() {
        let mut input = basic_tax_input();
        input.investor_profiles = vec![MigrationInvestorProfile {
            investor_type: "GCC_SWF".to_string(),
            residence: "UAE".to_string(),
            allocation_pct: dec!(1.0),
        }];
        let result = tax_consequence_analysis(&input).unwrap();

        let swf = &result.per_investor_impact[0];
        assert!(!swf.deemed_disposal_triggered);
        assert_eq!(swf.estimated_tax_cost, Decimal::ZERO);
    }

    // ------------------------------------------------------------------
    // 55. US taxable CGT rate 23.8%
    // ------------------------------------------------------------------
    #[test]
    fn test_us_taxable_cgt_rate() {
        let mut input = basic_tax_input();
        input.unrealized_gains = dec!(100_000_000);
        input.investor_profiles = vec![MigrationInvestorProfile {
            investor_type: "USTaxable".to_string(),
            residence: "US".to_string(),
            allocation_pct: dec!(1.0),
        }];
        let result = tax_consequence_analysis(&input).unwrap();

        let us_taxable = &result.per_investor_impact[0];
        let expected = dec!(100_000_000) * dec!(0.238);
        assert_eq!(us_taxable.estimated_tax_cost, expected);
    }

    // ------------------------------------------------------------------
    // 56. Cayman to Lux treaty analysis mentions WHT
    // ------------------------------------------------------------------
    #[test]
    fn test_cayman_lux_treaty_mentions_wht() {
        let input = basic_tax_input();
        let result = tax_consequence_analysis(&input).unwrap();

        assert!(result.treaty_impact_analysis.contains("WHT"));
    }

    // ------------------------------------------------------------------
    // 57. BVI to Cayman treaty analysis neutral
    // ------------------------------------------------------------------
    #[test]
    fn test_bvi_cayman_treaty_neutral() {
        let mut input = basic_tax_input();
        input.source_jurisdiction = "BVI".to_string();
        input.target_jurisdiction = "Cayman".to_string();
        let result = tax_consequence_analysis(&input).unwrap();

        assert!(result.treaty_impact_analysis.contains("tax-neutral"));
    }

    // ------------------------------------------------------------------
    // 58. Timing recommendation always includes year-end
    // ------------------------------------------------------------------
    #[test]
    fn test_timing_recommendation_year_end() {
        let input = basic_tax_input();
        let result = tax_consequence_analysis(&input).unwrap();

        assert!(result
            .timing_recommendations
            .iter()
            .any(|r| r.contains("fiscal year-end")));
    }
}
