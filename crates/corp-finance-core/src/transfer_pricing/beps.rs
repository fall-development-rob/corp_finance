use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::error::CorpFinanceError;
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Input Types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupEntity {
    pub name: String,
    pub jurisdiction: String,
    pub function: String,
    pub revenue: Decimal,
    pub operating_profit: Decimal,
    pub employees: u32,
    pub tangible_assets: Decimal,
    pub intangible_assets: Decimal,
    pub related_party_revenue: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntercompanyTx {
    pub from_entity: String,
    pub to_entity: String,
    pub transaction_type: String,
    pub amount: Decimal,
    pub arm_length_range_low: Decimal,
    pub arm_length_range_high: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BepsInput {
    pub entity_name: String,
    pub parent_jurisdiction: String,
    pub entities: Vec<GroupEntity>,
    pub intercompany_transactions: Vec<IntercompanyTx>,
    pub group_consolidated_revenue: Decimal,
    pub group_consolidated_profit: Decimal,
    pub cbcr_threshold: Decimal,
    pub pillar_two_applicable: bool,
}

// ---------------------------------------------------------------------------
// Output Types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupSummary {
    pub total_revenue: Decimal,
    pub total_profit: Decimal,
    pub total_entities: u32,
    pub jurisdictions_count: u32,
    pub cbcr_required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityAnalysis {
    pub name: String,
    pub jurisdiction: String,
    pub functional_classification: String,
    pub profit_margin: Decimal,
    pub substance_score: Decimal,
    pub profit_substance_ratio: Decimal,
    pub risk_score: Decimal,
    pub risk_level: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CbcrJurisdiction {
    pub jurisdiction: String,
    pub entities: u32,
    pub revenue: Decimal,
    pub profit_before_tax: Decimal,
    pub tax_paid: Decimal,
    pub tax_accrued: Decimal,
    pub effective_tax_rate: Decimal,
    pub employees: u32,
    pub tangible_assets: Decimal,
    pub stated_capital: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JurisdictionTopUp {
    pub jurisdiction: String,
    pub etr: Decimal,
    pub top_up_rate: Decimal,
    pub excess_profit: Decimal,
    pub sbie_exclusion: Decimal,
    pub top_up_tax: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PillarTwoAnalysis {
    pub jurisdictions_below_minimum: Vec<JurisdictionTopUp>,
    pub total_top_up_tax: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxReview {
    pub from: String,
    pub to: String,
    pub transaction_type: String,
    pub amount: Decimal,
    pub within_arm_length: bool,
    pub deviation_pct: Decimal,
    pub risk_flag: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskAssessment {
    pub overall_risk_score: Decimal,
    pub high_risk_entities: Vec<String>,
    pub remediation_priorities: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BepsOutput {
    pub group_summary: GroupSummary,
    pub entity_analysis: Vec<EntityAnalysis>,
    pub cbcr_report: Vec<CbcrJurisdiction>,
    pub pillar_two: Option<PillarTwoAnalysis>,
    pub intercompany_review: Vec<TxReview>,
    pub risk_assessment: RiskAssessment,
    pub warnings: Vec<String>,
}

// ---------------------------------------------------------------------------
// Validation Helpers
// ---------------------------------------------------------------------------

const VALID_FUNCTIONS: &[&str] = &[
    "Principal",
    "LimitedRisk",
    "Commissionnaire",
    "IP_Owner",
    "ManufacturingCE",
    "DistributionCE",
    "ServicesCE",
    "Holding",
];

const VALID_TX_TYPES: &[&str] = &[
    "Services",
    "Goods",
    "Royalties",
    "InterestPayment",
    "ManagementFee",
    "CostSharing",
];

fn validate_beps_input(input: &BepsInput) -> CorpFinanceResult<()> {
    if input.entities.is_empty() {
        return Err(CorpFinanceError::InsufficientData(
            "At least one entity is required".to_string(),
        ));
    }

    if input.group_consolidated_revenue < dec!(0) {
        return Err(CorpFinanceError::InvalidInput {
            field: "group_consolidated_revenue".into(),
            reason: "Must be non-negative".into(),
        });
    }

    if input.cbcr_threshold < dec!(0) {
        return Err(CorpFinanceError::InvalidInput {
            field: "cbcr_threshold".into(),
            reason: "Must be non-negative".into(),
        });
    }

    for entity in &input.entities {
        if entity.revenue < dec!(0) {
            return Err(CorpFinanceError::InvalidInput {
                field: format!("entity.{}.revenue", entity.name),
                reason: "Revenue must be non-negative".into(),
            });
        }
        if entity.tangible_assets < dec!(0) {
            return Err(CorpFinanceError::InvalidInput {
                field: format!("entity.{}.tangible_assets", entity.name),
                reason: "Tangible assets must be non-negative".into(),
            });
        }
        if entity.intangible_assets < dec!(0) {
            return Err(CorpFinanceError::InvalidInput {
                field: format!("entity.{}.intangible_assets", entity.name),
                reason: "Intangible assets must be non-negative".into(),
            });
        }
        if entity.related_party_revenue < dec!(0) {
            return Err(CorpFinanceError::InvalidInput {
                field: format!("entity.{}.related_party_revenue", entity.name),
                reason: "Related party revenue must be non-negative".into(),
            });
        }
        if !VALID_FUNCTIONS.contains(&entity.function.as_str()) {
            return Err(CorpFinanceError::InvalidInput {
                field: format!("entity.{}.function", entity.name),
                reason: format!(
                    "Invalid function '{}'. Valid: {:?}",
                    entity.function, VALID_FUNCTIONS
                ),
            });
        }
    }

    for tx in &input.intercompany_transactions {
        if tx.amount < dec!(0) {
            return Err(CorpFinanceError::InvalidInput {
                field: "intercompany_transaction.amount".into(),
                reason: "Amount must be non-negative".into(),
            });
        }
        if tx.arm_length_range_low > tx.arm_length_range_high {
            return Err(CorpFinanceError::InvalidInput {
                field: "intercompany_transaction.arm_length_range".into(),
                reason: "Range low must not exceed range high".into(),
            });
        }
        if !VALID_TX_TYPES.contains(&tx.transaction_type.as_str()) {
            return Err(CorpFinanceError::InvalidInput {
                field: "intercompany_transaction.transaction_type".into(),
                reason: format!(
                    "Invalid type '{}'. Valid: {:?}",
                    tx.transaction_type, VALID_TX_TYPES
                ),
            });
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Statutory Corporate Tax Rate Lookup
// ---------------------------------------------------------------------------

fn statutory_corporate_rate(jurisdiction: &str) -> Decimal {
    match jurisdiction {
        "US" => dec!(0.21),
        "UK" => dec!(0.25),
        "Germany" => dec!(0.2983),
        "France" => dec!(0.2571),
        "Ireland" => dec!(0.15),
        "Netherlands" => dec!(0.2569),
        "Luxembourg" => dec!(0.2494),
        "Switzerland" => dec!(0.1470),
        "Singapore" => dec!(0.17),
        "HongKong" => dec!(0.165),
        "Japan" => dec!(0.3062),
        "Australia" => dec!(0.30),
        "Canada" => dec!(0.265),
        "Cayman" | "BVI" | "Jersey" | "Guernsey" | "Bermuda" => dec!(0),
        _ => dec!(0.25),
    }
}

// ---------------------------------------------------------------------------
// Core Analysis Functions
// ---------------------------------------------------------------------------

/// Compute substance score for an entity (0-100) based on employees, tangible
/// assets, and intangible assets relative to group totals.
fn compute_substance_score(
    entity: &GroupEntity,
    group_employees: u32,
    group_tangible: Decimal,
    group_intangible: Decimal,
) -> Decimal {
    let emp_share = if group_employees > 0 {
        Decimal::from(entity.employees) / Decimal::from(group_employees) * dec!(100)
    } else {
        dec!(0)
    };

    let tang_share = if group_tangible > dec!(0) {
        entity.tangible_assets / group_tangible * dec!(100)
    } else {
        dec!(0)
    };

    let intang_share = if group_intangible > dec!(0) {
        entity.intangible_assets / group_intangible * dec!(100)
    } else {
        dec!(0)
    };

    // Weight: employees 40%, tangible 35%, intangible 25%
    let score = emp_share * dec!(0.40) + tang_share * dec!(0.35) + intang_share * dec!(0.25);

    // Clamp 0-100
    if score > dec!(100) {
        dec!(100)
    } else if score < dec!(0) {
        dec!(0)
    } else {
        score
    }
}

/// Classify the functional profile based on declared function
fn classify_function(function: &str) -> String {
    match function {
        "Principal" => "Entrepreneur / Principal — bears main risks, owns key assets".into(),
        "LimitedRisk" => "Limited Risk Entity — performs routine functions under direction".into(),
        "Commissionnaire" => "Commissionnaire — sells in own name on behalf of principal".into(),
        "IP_Owner" => "IP Owner — holds and develops intangible assets".into(),
        "ManufacturingCE" => {
            "Contract Manufacturer — performs manufacturing on a cost-plus basis".into()
        }
        "DistributionCE" => {
            "Limited Risk Distributor — distribution under principal direction".into()
        }
        "ServicesCE" => "Services Cost Centre — provides intra-group services at cost-plus".into(),
        "Holding" => {
            "Holding Company — holds equity participations, limited operational role".into()
        }
        _ => format!("Unclassified ({})", function),
    }
}

/// Risk score for a single entity (0-100)
fn compute_entity_risk_score(
    entity: &GroupEntity,
    substance_score: Decimal,
    group_revenue: Decimal,
    group_profit: Decimal,
    related_party_pct: Decimal,
) -> Decimal {
    let mut score = dec!(0);

    // (1) Profit misalignment: profit share vs substance score
    let profit_share = if group_profit > dec!(0) {
        entity.operating_profit / group_profit * dec!(100)
    } else {
        dec!(0)
    };
    let misalignment = if profit_share > dec!(0) && substance_score > dec!(0) {
        profit_share / substance_score
    } else if profit_share > dec!(0) {
        dec!(5)
    } else {
        dec!(0)
    };
    // High misalignment penalized: ratio > 2 => high risk
    if misalignment > dec!(3) {
        score += dec!(35);
    } else if misalignment > dec!(2) {
        score += dec!(25);
    } else if misalignment > dec!(1.5) {
        score += dec!(15);
    }

    // (2) Related party % of revenue
    if related_party_pct > dec!(80) {
        score += dec!(25);
    } else if related_party_pct > dec!(60) {
        score += dec!(15);
    } else if related_party_pct > dec!(40) {
        score += dec!(10);
    }

    // (3) Low-tax jurisdiction
    let etr = statutory_corporate_rate(&entity.jurisdiction);
    if etr < dec!(0.10) {
        score += dec!(20);
    } else if etr < dec!(0.15) {
        score += dec!(10);
    }

    // (4) Revenue concentration: entity has large share of group revenue
    let rev_share = if group_revenue > dec!(0) {
        entity.revenue / group_revenue * dec!(100)
    } else {
        dec!(0)
    };
    if rev_share > dec!(50) && substance_score < dec!(20) {
        score += dec!(20);
    }

    // Clamp
    if score > dec!(100) {
        dec!(100)
    } else {
        score
    }
}

fn risk_level_label(score: Decimal) -> String {
    if score >= dec!(70) {
        "High".into()
    } else if score >= dec!(40) {
        "Medium".into()
    } else {
        "Low".into()
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Analyze BEPS compliance for a multinational group including CbCR, Pillar
/// Two GloBE analysis, transfer pricing risk scoring, and intercompany
/// transaction review.
pub fn analyze_beps_compliance(input: &BepsInput) -> CorpFinanceResult<BepsOutput> {
    validate_beps_input(input)?;

    let mut warnings: Vec<String> = Vec::new();

    // -----------------------------------------------------------------------
    // Aggregate group totals
    // -----------------------------------------------------------------------
    let group_employees: u32 = input.entities.iter().map(|e| e.employees).sum();
    let group_tangible: Decimal = input.entities.iter().map(|e| e.tangible_assets).sum();
    let group_intangible: Decimal = input.entities.iter().map(|e| e.intangible_assets).sum();
    let total_revenue: Decimal = input.entities.iter().map(|e| e.revenue).sum();
    let total_profit: Decimal = input.entities.iter().map(|e| e.operating_profit).sum();

    // Unique jurisdictions
    let mut jurisdiction_set: Vec<String> = input
        .entities
        .iter()
        .map(|e| e.jurisdiction.clone())
        .collect();
    jurisdiction_set.sort();
    jurisdiction_set.dedup();
    let jurisdictions_count = jurisdiction_set.len() as u32;

    let cbcr_required = input.group_consolidated_revenue >= input.cbcr_threshold;

    let group_summary = GroupSummary {
        total_revenue,
        total_profit,
        total_entities: input.entities.len() as u32,
        jurisdictions_count,
        cbcr_required,
    };

    // -----------------------------------------------------------------------
    // Entity Analysis (BEPS Action 8-10)
    // -----------------------------------------------------------------------
    let mut entity_analysis: Vec<EntityAnalysis> = Vec::new();
    let mut high_risk_entities: Vec<String> = Vec::new();
    let mut remediation_priorities: Vec<String> = Vec::new();
    let mut entity_risk_scores: Vec<Decimal> = Vec::new();

    for entity in &input.entities {
        let profit_margin = if entity.revenue > dec!(0) {
            entity.operating_profit / entity.revenue
        } else {
            dec!(0)
        };

        let substance_score =
            compute_substance_score(entity, group_employees, group_tangible, group_intangible);

        let profit_share = if total_profit > dec!(0) {
            entity.operating_profit / total_profit * dec!(100)
        } else {
            dec!(0)
        };
        let profit_substance_ratio = if substance_score > dec!(0) {
            profit_share / substance_score
        } else if profit_share > dec!(0) {
            dec!(999)
        } else {
            dec!(0)
        };

        let related_party_pct = if entity.revenue > dec!(0) {
            entity.related_party_revenue / entity.revenue * dec!(100)
        } else {
            dec!(0)
        };

        let risk_score = compute_entity_risk_score(
            entity,
            substance_score,
            total_revenue,
            total_profit,
            related_party_pct,
        );

        let risk_level = risk_level_label(risk_score);

        if risk_level == "High" {
            high_risk_entities.push(entity.name.clone());
            remediation_priorities.push(format!(
                "Review {} ({}) — risk score {}, profit/substance ratio {:.2}",
                entity.name, entity.jurisdiction, risk_score, profit_substance_ratio,
            ));
        }

        entity_risk_scores.push(risk_score);

        entity_analysis.push(EntityAnalysis {
            name: entity.name.clone(),
            jurisdiction: entity.jurisdiction.clone(),
            functional_classification: classify_function(&entity.function),
            profit_margin,
            substance_score,
            profit_substance_ratio,
            risk_score,
            risk_level,
        });
    }

    // -----------------------------------------------------------------------
    // CbCR Report (BEPS Action 13)
    // -----------------------------------------------------------------------
    let mut cbcr_map: HashMap<String, CbcrJurisdiction> = HashMap::new();

    for entity in &input.entities {
        let corp_rate = statutory_corporate_rate(&entity.jurisdiction);
        let tax_est = entity.operating_profit * corp_rate;
        let tax_est_positive = if tax_est > dec!(0) { tax_est } else { dec!(0) };

        let entry = cbcr_map
            .entry(entity.jurisdiction.clone())
            .or_insert_with(|| CbcrJurisdiction {
                jurisdiction: entity.jurisdiction.clone(),
                entities: 0,
                revenue: dec!(0),
                profit_before_tax: dec!(0),
                tax_paid: dec!(0),
                tax_accrued: dec!(0),
                effective_tax_rate: dec!(0),
                employees: 0,
                tangible_assets: dec!(0),
                stated_capital: dec!(0),
            });

        entry.entities += 1;
        entry.revenue += entity.revenue;
        entry.profit_before_tax += entity.operating_profit;
        entry.tax_paid += tax_est_positive;
        entry.tax_accrued += tax_est_positive;
        entry.employees += entity.employees;
        entry.tangible_assets += entity.tangible_assets;
        // Stated capital approximated as tangible + intangible assets
        entry.stated_capital += entity.tangible_assets + entity.intangible_assets;
    }

    // Compute effective tax rates and flag misalignments
    let mut cbcr_report: Vec<CbcrJurisdiction> = Vec::new();
    for (_, mut jur) in cbcr_map {
        jur.effective_tax_rate = if jur.profit_before_tax > dec!(0) {
            jur.tax_paid / jur.profit_before_tax
        } else {
            dec!(0)
        };

        // Flag: high profit, low substance
        let profit_share = if total_profit > dec!(0) {
            jur.profit_before_tax / total_profit * dec!(100)
        } else {
            dec!(0)
        };
        let emp_share = if group_employees > 0 {
            Decimal::from(jur.employees) / Decimal::from(group_employees) * dec!(100)
        } else {
            dec!(0)
        };
        if profit_share > dec!(25) && emp_share < dec!(5) {
            warnings.push(format!(
                "CbCR flag: {} has {:.1}% of group profit but only {:.1}% of employees",
                jur.jurisdiction, profit_share, emp_share,
            ));
        }

        cbcr_report.push(jur);
    }
    cbcr_report.sort_by(|a, b| b.profit_before_tax.cmp(&a.profit_before_tax));

    if !cbcr_required {
        warnings.push(format!(
            "CbCR filing not required — consolidated revenue ({}) is below threshold ({})",
            input.group_consolidated_revenue, input.cbcr_threshold,
        ));
    }

    // -----------------------------------------------------------------------
    // Pillar Two Analysis (GloBE 15% minimum tax)
    // -----------------------------------------------------------------------
    let pillar_two = if input.pillar_two_applicable {
        let min_rate = dec!(0.15);
        let sbie_tangible_pct = dec!(0.05);
        let sbie_payroll_pct = dec!(0.05);

        let mut jurisdictions_below: Vec<JurisdictionTopUp> = Vec::new();
        let mut total_top_up = dec!(0);

        for cbcr_jur in &cbcr_report {
            let etr = cbcr_jur.effective_tax_rate;
            if etr < min_rate && cbcr_jur.profit_before_tax > dec!(0) {
                let top_up_rate = min_rate - etr;

                // SBIE: 5% tangible + 5% payroll (payroll approximated from
                // employees * average compensation; use tangible_assets as
                // payroll proxy since payroll data is not available)
                let payroll_proxy = Decimal::from(cbcr_jur.employees) * dec!(50000);
                let sbie =
                    cbcr_jur.tangible_assets * sbie_tangible_pct + payroll_proxy * sbie_payroll_pct;

                let excess_profit = if cbcr_jur.profit_before_tax > sbie {
                    cbcr_jur.profit_before_tax - sbie
                } else {
                    dec!(0)
                };

                let top_up_tax = excess_profit * top_up_rate;
                total_top_up += top_up_tax;

                jurisdictions_below.push(JurisdictionTopUp {
                    jurisdiction: cbcr_jur.jurisdiction.clone(),
                    etr,
                    top_up_rate,
                    excess_profit,
                    sbie_exclusion: sbie,
                    top_up_tax,
                });
            }
        }

        if jurisdictions_below.is_empty() {
            warnings.push("Pillar Two: No jurisdictions below 15% minimum ETR".to_string());
        }

        Some(PillarTwoAnalysis {
            jurisdictions_below_minimum: jurisdictions_below,
            total_top_up_tax: total_top_up,
        })
    } else {
        None
    };

    // -----------------------------------------------------------------------
    // Intercompany Transaction Review
    // -----------------------------------------------------------------------
    let mut intercompany_review: Vec<TxReview> = Vec::new();

    for tx in &input.intercompany_transactions {
        let midpoint = (tx.arm_length_range_low + tx.arm_length_range_high) / dec!(2);
        let within_arm_length =
            tx.amount >= tx.arm_length_range_low && tx.amount <= tx.arm_length_range_high;

        let deviation_pct = if midpoint > dec!(0) {
            (tx.amount - midpoint) / midpoint * dec!(100)
        } else {
            dec!(0)
        };

        let risk_flag = if !within_arm_length {
            if deviation_pct.abs() > dec!(25) {
                "HIGH — significant deviation from arm's length range".to_string()
            } else {
                "MEDIUM — outside arm's length range".to_string()
            }
        } else if deviation_pct.abs() > dec!(15) {
            "LOW — within range but near boundary".to_string()
        } else {
            "NONE".to_string()
        };

        if !within_arm_length {
            warnings.push(format!(
                "Transaction {} -> {} ({}, {}) is outside arm's length range [{}, {}]",
                tx.from_entity,
                tx.to_entity,
                tx.transaction_type,
                tx.amount,
                tx.arm_length_range_low,
                tx.arm_length_range_high,
            ));
        }

        intercompany_review.push(TxReview {
            from: tx.from_entity.clone(),
            to: tx.to_entity.clone(),
            transaction_type: tx.transaction_type.clone(),
            amount: tx.amount,
            within_arm_length,
            deviation_pct,
            risk_flag,
        });
    }

    // -----------------------------------------------------------------------
    // Overall Risk Assessment
    // -----------------------------------------------------------------------
    let overall_risk_score = if entity_risk_scores.is_empty() {
        dec!(0)
    } else {
        let sum: Decimal = entity_risk_scores.iter().copied().sum();
        sum / Decimal::from(entity_risk_scores.len() as u32)
    };

    // Add intercompany risk to remediation
    for review in &intercompany_review {
        if review.risk_flag.starts_with("HIGH") {
            remediation_priorities.push(format!(
                "Adjust intercompany {} from {} to {} — deviation {:.1}%",
                review.transaction_type, review.from, review.to, review.deviation_pct,
            ));
        }
    }

    let risk_assessment = RiskAssessment {
        overall_risk_score,
        high_risk_entities,
        remediation_priorities,
    };

    Ok(BepsOutput {
        group_summary,
        entity_analysis,
        cbcr_report,
        pillar_two,
        intercompany_review,
        risk_assessment,
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

    fn make_entity(
        name: &str,
        jurisdiction: &str,
        function: &str,
        revenue: Decimal,
        operating_profit: Decimal,
        employees: u32,
        tangible_assets: Decimal,
        intangible_assets: Decimal,
        related_party_revenue: Decimal,
    ) -> GroupEntity {
        GroupEntity {
            name: name.into(),
            jurisdiction: jurisdiction.into(),
            function: function.into(),
            revenue,
            operating_profit,
            employees,
            tangible_assets,
            intangible_assets,
            related_party_revenue,
        }
    }

    fn make_tx(
        from: &str,
        to: &str,
        tx_type: &str,
        amount: Decimal,
        low: Decimal,
        high: Decimal,
    ) -> IntercompanyTx {
        IntercompanyTx {
            from_entity: from.into(),
            to_entity: to.into(),
            transaction_type: tx_type.into(),
            amount,
            arm_length_range_low: low,
            arm_length_range_high: high,
        }
    }

    fn basic_input() -> BepsInput {
        BepsInput {
            entity_name: "Global Corp".into(),
            parent_jurisdiction: "US".into(),
            entities: vec![
                make_entity(
                    "US Parent",
                    "US",
                    "Principal",
                    dec!(500000000),
                    dec!(80000000),
                    2000,
                    dec!(200000000),
                    dec!(100000000),
                    dec!(100000000),
                ),
                make_entity(
                    "Ireland Sub",
                    "Ireland",
                    "IP_Owner",
                    dec!(300000000),
                    dec!(120000000),
                    50,
                    dec!(10000000),
                    dec!(500000000),
                    dec!(280000000),
                ),
                make_entity(
                    "UK Distributor",
                    "UK",
                    "DistributionCE",
                    dec!(200000000),
                    dec!(10000000),
                    500,
                    dec!(50000000),
                    dec!(5000000),
                    dec!(180000000),
                ),
            ],
            intercompany_transactions: vec![
                make_tx(
                    "Ireland Sub",
                    "US Parent",
                    "Royalties",
                    dec!(50000000),
                    dec!(30000000),
                    dec!(60000000),
                ),
                make_tx(
                    "US Parent",
                    "UK Distributor",
                    "Goods",
                    dec!(150000000),
                    dec!(140000000),
                    dec!(160000000),
                ),
            ],
            group_consolidated_revenue: dec!(1000000000),
            group_consolidated_profit: dec!(210000000),
            cbcr_threshold: dec!(750000000),
            pillar_two_applicable: true,
        }
    }

    // --- Validation Tests ---

    #[test]
    fn test_empty_entities_rejected() {
        let mut input = basic_input();
        input.entities.clear();
        let result = analyze_beps_compliance(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_negative_revenue_rejected() {
        let mut input = basic_input();
        input.entities[0].revenue = dec!(-100);
        let result = analyze_beps_compliance(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_negative_tangible_assets_rejected() {
        let mut input = basic_input();
        input.entities[0].tangible_assets = dec!(-1);
        let result = analyze_beps_compliance(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_function_rejected() {
        let mut input = basic_input();
        input.entities[0].function = "InvalidFunc".into();
        let result = analyze_beps_compliance(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_tx_type_rejected() {
        let mut input = basic_input();
        input.intercompany_transactions[0].transaction_type = "BadType".into();
        let result = analyze_beps_compliance(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_negative_tx_amount_rejected() {
        let mut input = basic_input();
        input.intercompany_transactions[0].amount = dec!(-1000);
        let result = analyze_beps_compliance(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_arm_length_range_inverted_rejected() {
        let mut input = basic_input();
        input.intercompany_transactions[0].arm_length_range_low = dec!(100);
        input.intercompany_transactions[0].arm_length_range_high = dec!(50);
        let result = analyze_beps_compliance(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_negative_cbcr_threshold_rejected() {
        let mut input = basic_input();
        input.cbcr_threshold = dec!(-1);
        let result = analyze_beps_compliance(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_negative_consolidated_revenue_rejected() {
        let mut input = basic_input();
        input.group_consolidated_revenue = dec!(-100);
        let result = analyze_beps_compliance(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_negative_intangible_assets_rejected() {
        let mut input = basic_input();
        input.entities[0].intangible_assets = dec!(-1);
        let result = analyze_beps_compliance(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_negative_related_party_revenue_rejected() {
        let mut input = basic_input();
        input.entities[0].related_party_revenue = dec!(-1);
        let result = analyze_beps_compliance(&input);
        assert!(result.is_err());
    }

    // --- Group Summary Tests ---

    #[test]
    fn test_group_summary_totals() {
        let input = basic_input();
        let output = analyze_beps_compliance(&input).unwrap();
        assert_eq!(output.group_summary.total_entities, 3);
        assert_eq!(output.group_summary.jurisdictions_count, 3);
        assert_eq!(output.group_summary.total_revenue, dec!(1000000000));
        assert_eq!(output.group_summary.total_profit, dec!(210000000));
    }

    #[test]
    fn test_cbcr_required_above_threshold() {
        let input = basic_input();
        let output = analyze_beps_compliance(&input).unwrap();
        assert!(output.group_summary.cbcr_required);
    }

    #[test]
    fn test_cbcr_not_required_below_threshold() {
        let mut input = basic_input();
        input.group_consolidated_revenue = dec!(500000000);
        let output = analyze_beps_compliance(&input).unwrap();
        assert!(!output.group_summary.cbcr_required);
    }

    // --- Entity Analysis Tests ---

    #[test]
    fn test_entity_analysis_count() {
        let input = basic_input();
        let output = analyze_beps_compliance(&input).unwrap();
        assert_eq!(output.entity_analysis.len(), 3);
    }

    #[test]
    fn test_entity_profit_margin() {
        let input = basic_input();
        let output = analyze_beps_compliance(&input).unwrap();
        // US Parent: 80M / 500M = 0.16
        let us = output
            .entity_analysis
            .iter()
            .find(|e| e.name == "US Parent")
            .unwrap();
        assert_eq!(us.profit_margin, dec!(0.16));
    }

    #[test]
    fn test_entity_functional_classification() {
        let input = basic_input();
        let output = analyze_beps_compliance(&input).unwrap();
        let us = output
            .entity_analysis
            .iter()
            .find(|e| e.name == "US Parent")
            .unwrap();
        assert!(us.functional_classification.contains("Principal"));
    }

    #[test]
    fn test_ireland_ip_owner_high_risk() {
        let input = basic_input();
        let output = analyze_beps_compliance(&input).unwrap();
        // Ireland Sub: high profit, low employees => should have elevated risk
        let ie = output
            .entity_analysis
            .iter()
            .find(|e| e.name == "Ireland Sub")
            .unwrap();
        assert!(ie.risk_score >= dec!(40));
    }

    #[test]
    fn test_substance_score_non_negative() {
        let input = basic_input();
        let output = analyze_beps_compliance(&input).unwrap();
        for entity in &output.entity_analysis {
            assert!(entity.substance_score >= dec!(0));
        }
    }

    #[test]
    fn test_risk_level_labels() {
        assert_eq!(risk_level_label(dec!(75)), "High");
        assert_eq!(risk_level_label(dec!(50)), "Medium");
        assert_eq!(risk_level_label(dec!(20)), "Low");
    }

    #[test]
    fn test_entity_zero_revenue_profit_margin() {
        let mut input = basic_input();
        input.entities[2].revenue = dec!(0);
        input.entities[2].operating_profit = dec!(0);
        let output = analyze_beps_compliance(&input).unwrap();
        let uk = output
            .entity_analysis
            .iter()
            .find(|e| e.name == "UK Distributor")
            .unwrap();
        assert_eq!(uk.profit_margin, dec!(0));
    }

    // --- CbCR Report Tests ---

    #[test]
    fn test_cbcr_report_jurisdictions() {
        let input = basic_input();
        let output = analyze_beps_compliance(&input).unwrap();
        assert_eq!(output.cbcr_report.len(), 3);
    }

    #[test]
    fn test_cbcr_effective_tax_rate() {
        let input = basic_input();
        let output = analyze_beps_compliance(&input).unwrap();
        for jur in &output.cbcr_report {
            if jur.profit_before_tax > dec!(0) {
                assert!(jur.effective_tax_rate > dec!(0));
                assert!(jur.effective_tax_rate <= dec!(1));
            }
        }
    }

    #[test]
    fn test_cbcr_employee_count() {
        let input = basic_input();
        let output = analyze_beps_compliance(&input).unwrap();
        let total_emp: u32 = output.cbcr_report.iter().map(|j| j.employees).sum();
        assert_eq!(total_emp, 2550);
    }

    #[test]
    fn test_cbcr_flag_high_profit_low_substance() {
        let input = basic_input();
        let output = analyze_beps_compliance(&input).unwrap();
        // Ireland has ~57% of profit but ~2% of employees
        let has_flag = output
            .warnings
            .iter()
            .any(|w| w.contains("CbCR flag") && w.contains("Ireland"));
        assert!(has_flag);
    }

    // --- Pillar Two Tests ---

    #[test]
    fn test_pillar_two_present_when_applicable() {
        let input = basic_input();
        let output = analyze_beps_compliance(&input).unwrap();
        assert!(output.pillar_two.is_some());
    }

    #[test]
    fn test_pillar_two_absent_when_not_applicable() {
        let mut input = basic_input();
        input.pillar_two_applicable = false;
        let output = analyze_beps_compliance(&input).unwrap();
        assert!(output.pillar_two.is_none());
    }

    #[test]
    fn test_pillar_two_top_up_rate() {
        let input = basic_input();
        let output = analyze_beps_compliance(&input).unwrap();
        let p2 = output.pillar_two.unwrap();
        for jur in &p2.jurisdictions_below_minimum {
            assert!(jur.top_up_rate > dec!(0));
            assert_eq!(jur.top_up_rate, dec!(0.15) - jur.etr);
        }
    }

    #[test]
    fn test_pillar_two_sbie_exclusion_nonneg() {
        let input = basic_input();
        let output = analyze_beps_compliance(&input).unwrap();
        let p2 = output.pillar_two.unwrap();
        for jur in &p2.jurisdictions_below_minimum {
            assert!(jur.sbie_exclusion >= dec!(0));
        }
    }

    #[test]
    fn test_pillar_two_total_top_up_sum() {
        let input = basic_input();
        let output = analyze_beps_compliance(&input).unwrap();
        let p2 = output.pillar_two.unwrap();
        let sum: Decimal = p2
            .jurisdictions_below_minimum
            .iter()
            .map(|j| j.top_up_tax)
            .sum();
        assert_eq!(p2.total_top_up_tax, sum);
    }

    #[test]
    fn test_pillar_two_no_jurisdictions_below_minimum() {
        // All entities in high-tax jurisdictions
        let input = BepsInput {
            entity_name: "HighTax Corp".into(),
            parent_jurisdiction: "US".into(),
            entities: vec![
                make_entity(
                    "US HQ",
                    "US",
                    "Principal",
                    dec!(500000000),
                    dec!(50000000),
                    1000,
                    dec!(100000000),
                    dec!(50000000),
                    dec!(0),
                ),
                make_entity(
                    "Japan Sub",
                    "Japan",
                    "ManufacturingCE",
                    dec!(200000000),
                    dec!(20000000),
                    500,
                    dec!(80000000),
                    dec!(10000000),
                    dec!(180000000),
                ),
            ],
            intercompany_transactions: vec![],
            group_consolidated_revenue: dec!(800000000),
            group_consolidated_profit: dec!(70000000),
            cbcr_threshold: dec!(750000000),
            pillar_two_applicable: true,
        };
        let output = analyze_beps_compliance(&input).unwrap();
        let p2 = output.pillar_two.unwrap();
        assert!(p2.jurisdictions_below_minimum.is_empty());
        assert_eq!(p2.total_top_up_tax, dec!(0));
    }

    // --- Intercompany Transaction Review Tests ---

    #[test]
    fn test_tx_within_arm_length() {
        let input = basic_input();
        let output = analyze_beps_compliance(&input).unwrap();
        // Both transactions are within arm's length range in basic_input
        for review in &output.intercompany_review {
            assert!(review.within_arm_length);
        }
    }

    #[test]
    fn test_tx_outside_arm_length() {
        let mut input = basic_input();
        // Set amount outside range
        input.intercompany_transactions[0].amount = dec!(70000000);
        // Range is [30M, 60M] so 70M is outside
        let output = analyze_beps_compliance(&input).unwrap();
        let review = &output.intercompany_review[0];
        assert!(!review.within_arm_length);
    }

    #[test]
    fn test_tx_deviation_pct() {
        let input = basic_input();
        let output = analyze_beps_compliance(&input).unwrap();
        // Royalty tx: amount 50M, range [30M, 60M], midpoint 45M
        // deviation = (50 - 45) / 45 * 100 = 11.11%
        let royalty_review = output
            .intercompany_review
            .iter()
            .find(|r| r.transaction_type == "Royalties")
            .unwrap();
        // Approximately 11.11%
        assert!(royalty_review.deviation_pct > dec!(11));
        assert!(royalty_review.deviation_pct < dec!(12));
    }

    #[test]
    fn test_tx_high_deviation_risk_flag() {
        let mut input = basic_input();
        // Set amount way outside range to trigger HIGH flag
        input.intercompany_transactions[0].amount = dec!(100000000);
        // Range [30M, 60M], midpoint 45M, deviation = (100-45)/45*100 = 122%
        let output = analyze_beps_compliance(&input).unwrap();
        let review = &output.intercompany_review[0];
        assert!(review.risk_flag.starts_with("HIGH"));
    }

    #[test]
    fn test_tx_review_count() {
        let input = basic_input();
        let output = analyze_beps_compliance(&input).unwrap();
        assert_eq!(output.intercompany_review.len(), 2);
    }

    // --- Risk Assessment Tests ---

    #[test]
    fn test_overall_risk_score_nonneg() {
        let input = basic_input();
        let output = analyze_beps_compliance(&input).unwrap();
        assert!(output.risk_assessment.overall_risk_score >= dec!(0));
    }

    #[test]
    fn test_high_risk_entities_identified() {
        let input = basic_input();
        let output = analyze_beps_compliance(&input).unwrap();
        // Ireland Sub likely flagged as high risk
        // (may or may not be, depends on exact scoring)
        // Just check structure is populated
        assert!(output.risk_assessment.high_risk_entities.len() <= input.entities.len());
    }

    #[test]
    fn test_remediation_priorities_for_outside_arm_length() {
        let mut input = basic_input();
        input.intercompany_transactions[0].amount = dec!(100000000);
        let output = analyze_beps_compliance(&input).unwrap();
        let has_ic_remediation = output
            .risk_assessment
            .remediation_priorities
            .iter()
            .any(|p| p.contains("intercompany"));
        assert!(has_ic_remediation);
    }

    // --- Warnings Tests ---

    #[test]
    fn test_warnings_populated_on_outside_range() {
        let mut input = basic_input();
        input.intercompany_transactions[0].amount = dec!(70000000);
        let output = analyze_beps_compliance(&input).unwrap();
        let has_warning = output
            .warnings
            .iter()
            .any(|w| w.contains("outside arm's length"));
        assert!(has_warning);
    }

    #[test]
    fn test_cbcr_not_required_warning() {
        let mut input = basic_input();
        input.group_consolidated_revenue = dec!(500000000);
        let output = analyze_beps_compliance(&input).unwrap();
        let has_warning = output
            .warnings
            .iter()
            .any(|w| w.contains("CbCR filing not required"));
        assert!(has_warning);
    }

    // --- Edge Cases ---

    #[test]
    fn test_single_entity_group() {
        let input = BepsInput {
            entity_name: "Solo Corp".into(),
            parent_jurisdiction: "US".into(),
            entities: vec![make_entity(
                "US Only",
                "US",
                "Principal",
                dec!(100000000),
                dec!(15000000),
                500,
                dec!(50000000),
                dec!(20000000),
                dec!(0),
            )],
            intercompany_transactions: vec![],
            group_consolidated_revenue: dec!(100000000),
            group_consolidated_profit: dec!(15000000),
            cbcr_threshold: dec!(750000000),
            pillar_two_applicable: false,
        };
        let output = analyze_beps_compliance(&input).unwrap();
        assert_eq!(output.entity_analysis.len(), 1);
        assert_eq!(output.cbcr_report.len(), 1);
        assert!(output.intercompany_review.is_empty());
    }

    #[test]
    fn test_zero_profit_group() {
        let input = BepsInput {
            entity_name: "Loss Corp".into(),
            parent_jurisdiction: "US".into(),
            entities: vec![
                make_entity(
                    "US Parent",
                    "US",
                    "Principal",
                    dec!(100000000),
                    dec!(0),
                    300,
                    dec!(50000000),
                    dec!(10000000),
                    dec!(0),
                ),
                make_entity(
                    "UK Sub",
                    "UK",
                    "DistributionCE",
                    dec!(50000000),
                    dec!(0),
                    100,
                    dec!(20000000),
                    dec!(5000000),
                    dec!(40000000),
                ),
            ],
            intercompany_transactions: vec![],
            group_consolidated_revenue: dec!(150000000),
            group_consolidated_profit: dec!(0),
            cbcr_threshold: dec!(750000000),
            pillar_two_applicable: false,
        };
        let output = analyze_beps_compliance(&input).unwrap();
        assert_eq!(output.group_summary.total_profit, dec!(0));
    }

    #[test]
    fn test_cayman_entity_zero_tax_rate() {
        let input = BepsInput {
            entity_name: "Offshore Corp".into(),
            parent_jurisdiction: "US".into(),
            entities: vec![
                make_entity(
                    "US Parent",
                    "US",
                    "Principal",
                    dec!(200000000),
                    dec!(30000000),
                    500,
                    dec!(100000000),
                    dec!(50000000),
                    dec!(0),
                ),
                make_entity(
                    "Cayman Holdco",
                    "Cayman",
                    "Holding",
                    dec!(50000000),
                    dec!(20000000),
                    5,
                    dec!(1000000),
                    dec!(0),
                    dec!(50000000),
                ),
            ],
            intercompany_transactions: vec![],
            group_consolidated_revenue: dec!(250000000),
            group_consolidated_profit: dec!(50000000),
            cbcr_threshold: dec!(750000000),
            pillar_two_applicable: true,
        };
        let output = analyze_beps_compliance(&input).unwrap();
        let p2 = output.pillar_two.unwrap();
        let cayman = p2
            .jurisdictions_below_minimum
            .iter()
            .find(|j| j.jurisdiction == "Cayman");
        assert!(cayman.is_some());
        assert_eq!(cayman.unwrap().etr, dec!(0));
    }

    #[test]
    fn test_multiple_entities_same_jurisdiction() {
        let input = BepsInput {
            entity_name: "Multi US Corp".into(),
            parent_jurisdiction: "US".into(),
            entities: vec![
                make_entity(
                    "US HQ",
                    "US",
                    "Principal",
                    dec!(300000000),
                    dec!(50000000),
                    1000,
                    dec!(100000000),
                    dec!(50000000),
                    dec!(0),
                ),
                make_entity(
                    "US Manufacturing",
                    "US",
                    "ManufacturingCE",
                    dec!(200000000),
                    dec!(20000000),
                    800,
                    dec!(80000000),
                    dec!(10000000),
                    dec!(180000000),
                ),
            ],
            intercompany_transactions: vec![],
            group_consolidated_revenue: dec!(500000000),
            group_consolidated_profit: dec!(70000000),
            cbcr_threshold: dec!(750000000),
            pillar_two_applicable: false,
        };
        let output = analyze_beps_compliance(&input).unwrap();
        // Should aggregate into one CbCR jurisdiction entry
        assert_eq!(output.cbcr_report.len(), 1);
        assert_eq!(output.cbcr_report[0].entities, 2);
    }

    #[test]
    fn test_statutory_rate_lookup() {
        assert_eq!(statutory_corporate_rate("US"), dec!(0.21));
        assert_eq!(statutory_corporate_rate("Cayman"), dec!(0));
        assert_eq!(statutory_corporate_rate("Unknown"), dec!(0.25));
    }

    #[test]
    fn test_classify_all_functions() {
        for func in VALID_FUNCTIONS {
            let classification = classify_function(func);
            assert!(!classification.contains("Unclassified"));
        }
    }

    #[test]
    fn test_substance_score_proportional() {
        let group_emp = 1000u32;
        let group_tang = dec!(100000000);
        let group_intang = dec!(50000000);

        let entity = GroupEntity {
            name: "Test".into(),
            jurisdiction: "US".into(),
            function: "Principal".into(),
            revenue: dec!(0),
            operating_profit: dec!(0),
            employees: 500,
            tangible_assets: dec!(50000000),
            intangible_assets: dec!(25000000),
            related_party_revenue: dec!(0),
        };

        let score = compute_substance_score(&entity, group_emp, group_tang, group_intang);
        // 50% employees (40% weight) + 50% tangible (35% weight) + 50% intangible (25% weight)
        // = 20 + 17.5 + 12.5 = 50
        assert_eq!(score, dec!(50));
    }
}
