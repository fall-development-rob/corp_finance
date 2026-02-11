//! International tax treaty network analysis module.
//!
//! Models treaty networks, withholding tax optimization through direct and
//! conduit routes, anti-treaty-shopping risk assessment, and entity-specific
//! treaty benefits.

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

use crate::error::CorpFinanceError;
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// A single income flow subject to withholding tax analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncomeFlow {
    /// Income type: "Dividends", "Interest", "Royalties", "ManagementFees",
    /// "CapitalGains", or "Services"
    pub income_type: String,
    /// Gross amount of the income flow
    pub amount: Decimal,
    /// Domestic WHT rate (statutory rate without treaty, as decimal 0-1)
    pub domestic_wht_rate: Decimal,
}

/// Treaty rate override for a specific income type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreatyRate {
    /// Income type this rate applies to
    pub income_type: String,
    /// Treaty WHT rate (as decimal 0-1)
    pub treaty_rate: Decimal,
    /// Conditions required to qualify for this rate
    pub qualifying_conditions: Vec<String>,
}

/// Input for treaty network analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreatyNetworkInput {
    /// Jurisdiction where income originates (e.g. "US")
    pub source_jurisdiction: String,
    /// Jurisdiction where income is received (e.g. "UK")
    pub recipient_jurisdiction: String,
    /// Income flows to analyze
    pub income_types: Vec<IncomeFlow>,
    /// Optional treaty rate overrides (if not provided, built-in rates used)
    pub treaty_rates: Option<Vec<TreatyRate>>,
    /// Potential intermediary jurisdictions for conduit analysis
    pub intermediary_jurisdictions: Vec<String>,
    /// Entity type of the recipient
    pub recipient_entity_type: String,
    /// Whether beneficial ownership test is met
    pub beneficial_owner: bool,
    /// LOB (Limitation on Benefits) qualified — US treaty specific
    pub lob_qualified: bool,
    /// PPT (Principal Purpose Test) met — MLI Article 7
    pub ppt_met: bool,
}

// ---------------------------------------------------------------------------
// Output types
// ---------------------------------------------------------------------------

/// Per-flow result in the direct route analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowResult {
    pub income_type: String,
    pub amount: Decimal,
    pub domestic_rate: Decimal,
    pub treaty_rate: Decimal,
    pub tax_domestic: Decimal,
    pub tax_treaty: Decimal,
    pub treaty_savings: Decimal,
}

/// Direct route treaty result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectTreatyResult {
    pub income_flows: Vec<FlowResult>,
    pub total_domestic_tax: Decimal,
    pub total_treaty_tax: Decimal,
    pub total_savings: Decimal,
}

/// A conduit route analysis result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConduitRoute {
    pub intermediary: String,
    pub leg1_rate: Decimal,
    pub leg2_rate: Decimal,
    pub combined_effective_rate: Decimal,
    pub net_tax: Decimal,
    pub savings_vs_direct: Decimal,
    pub viable: bool,
}

/// The optimal route (direct or conduit).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimalRoute {
    pub route_description: String,
    pub effective_rate: Decimal,
    pub total_tax: Decimal,
    pub savings_vs_domestic: Decimal,
}

/// Anti-avoidance risk assessment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AntiAvoidanceAssessment {
    /// "Low", "Medium", or "High"
    pub lob_risk: String,
    /// "Low", "Medium", or "High"
    pub ppt_risk: String,
    /// "Low", "Medium", or "High"
    pub beneficial_ownership_risk: String,
    /// 0-100 overall risk score
    pub overall_treaty_shopping_risk: u32,
    /// "Unlikely", "Possible", "Likely"
    pub challenge_likelihood: String,
}

/// Entity-specific treaty benefits.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityBenefits {
    pub pension_exemption: bool,
    pub swf_exemption: bool,
    pub participation_exemption: bool,
    pub participation_threshold: Option<Decimal>,
    pub look_through_available: bool,
}

/// Output for treaty network analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreatyNetworkOutput {
    pub direct_route: DirectTreatyResult,
    pub conduit_analysis: Vec<ConduitRoute>,
    pub optimal_route: OptimalRoute,
    pub anti_avoidance: AntiAvoidanceAssessment,
    pub entity_benefits: EntityBenefits,
    pub recommendations: Vec<String>,
    pub warnings: Vec<String>,
}

// ---------------------------------------------------------------------------
// Built-in treaty rate data
// ---------------------------------------------------------------------------

/// Returns a default treaty rate for Source -> Recipient, given income type.
/// This is a simplified lookup; real-world treaty networks are far more complex.
/// Returns None if no treaty rate is known.
fn builtin_treaty_rate(source: &str, recipient: &str, income_type: &str) -> Option<Decimal> {
    // Normalize to uppercase for matching
    let s = source.to_uppercase();
    let r = recipient.to_uppercase();
    let it = income_type.to_lowercase();

    // Order-independent pair matching
    let pair = if s <= r { (&s, &r) } else { (&r, &s) };

    match (pair.0.as_str(), pair.1.as_str()) {
        // US-UK
        ("UK", "US") => match it.as_str() {
            "dividends" => Some(dec!(0.15)),
            "interest" => Some(dec!(0.0)),
            "royalties" => Some(dec!(0.0)),
            _ => None,
        },
        // US-Netherlands
        ("NL" | "NETHERLANDS", "US") => match it.as_str() {
            "dividends" => Some(dec!(0.15)),
            "interest" => Some(dec!(0.0)),
            "royalties" => Some(dec!(0.0)),
            _ => None,
        },
        // US-Luxembourg
        ("LU" | "LUXEMBOURG", "US") => match it.as_str() {
            "dividends" => Some(dec!(0.15)),
            "interest" => Some(dec!(0.0)),
            "royalties" => Some(dec!(0.0)),
            _ => None,
        },
        // US-Ireland
        ("IE" | "IRELAND", "US") => match it.as_str() {
            "dividends" => Some(dec!(0.15)),
            "interest" => Some(dec!(0.0)),
            "royalties" => Some(dec!(0.0)),
            _ => None,
        },
        // US-Switzerland
        ("CH" | "SWITZERLAND", "US") => match it.as_str() {
            "dividends" => Some(dec!(0.15)),
            "interest" => Some(dec!(0.0)),
            "royalties" => Some(dec!(0.0)),
            _ => None,
        },
        // US-Singapore
        ("SG" | "SINGAPORE", "US") => match it.as_str() {
            "dividends" => Some(dec!(0.15)),
            "interest" => Some(dec!(0.0)),
            "royalties" => Some(dec!(0.0)),
            _ => None,
        },
        // US-Germany
        ("DE" | "GERMANY", "US") => match it.as_str() {
            "dividends" => Some(dec!(0.15)),
            "interest" => Some(dec!(0.0)),
            "royalties" => Some(dec!(0.0)),
            _ => None,
        },
        // UK-Netherlands
        ("NL" | "NETHERLANDS", "UK") => match it.as_str() {
            "dividends" => Some(dec!(0.10)),
            "interest" => Some(dec!(0.0)),
            "royalties" => Some(dec!(0.0)),
            _ => None,
        },
        // UK-Ireland
        ("IE" | "IRELAND", "UK") => match it.as_str() {
            "dividends" => Some(dec!(0.15)),
            "interest" => Some(dec!(0.0)),
            "royalties" => Some(dec!(0.0)),
            _ => None,
        },
        // UK-Luxembourg
        ("LU" | "LUXEMBOURG", "UK") => match it.as_str() {
            "dividends" => Some(dec!(0.10)),
            "interest" => Some(dec!(0.0)),
            "royalties" => Some(dec!(0.0)),
            _ => None,
        },
        // UK-Singapore
        ("SG" | "SINGAPORE", "UK") => match it.as_str() {
            "dividends" => Some(dec!(0.15)),
            "interest" => Some(dec!(0.0)),
            "royalties" => Some(dec!(0.0)),
            _ => None,
        },
        // UK-Switzerland
        ("CH" | "SWITZERLAND", "UK") => match it.as_str() {
            "dividends" => Some(dec!(0.15)),
            "interest" => Some(dec!(0.0)),
            "royalties" => Some(dec!(0.0)),
            _ => None,
        },
        // Germany-Netherlands
        ("DE" | "GERMANY", "NL" | "NETHERLANDS") => match it.as_str() {
            "dividends" => Some(dec!(0.10)),
            "interest" => Some(dec!(0.0)),
            "royalties" => Some(dec!(0.0)),
            _ => None,
        },
        // Germany-Luxembourg
        ("DE" | "GERMANY", "LU" | "LUXEMBOURG") => match it.as_str() {
            "dividends" => Some(dec!(0.10)),
            "interest" => Some(dec!(0.0)),
            "royalties" => Some(dec!(0.0)),
            _ => None,
        },
        // Germany-Switzerland
        ("CH" | "SWITZERLAND", "DE" | "GERMANY") => match it.as_str() {
            "dividends" => Some(dec!(0.15)),
            "interest" => Some(dec!(0.0)),
            "royalties" => Some(dec!(0.0)),
            _ => None,
        },
        // Netherlands-Luxembourg (EU Parent-Sub Directive: 0% dividends)
        ("LU" | "LUXEMBOURG", "NL" | "NETHERLANDS") => match it.as_str() {
            "dividends" => Some(dec!(0.0)),
            "interest" => Some(dec!(0.0)),
            "royalties" => Some(dec!(0.0)),
            _ => None,
        },
        // Netherlands-Singapore
        ("NL" | "NETHERLANDS", "SG" | "SINGAPORE") => match it.as_str() {
            "dividends" => Some(dec!(0.0)),
            "interest" => Some(dec!(0.0)),
            "royalties" => Some(dec!(0.0)),
            _ => None,
        },
        // Ireland-Luxembourg
        ("IE" | "IRELAND", "LU" | "LUXEMBOURG") => match it.as_str() {
            "dividends" => Some(dec!(0.0)),
            "interest" => Some(dec!(0.0)),
            "royalties" => Some(dec!(0.0)),
            _ => None,
        },
        // Singapore-Switzerland
        ("CH" | "SWITZERLAND", "SG" | "SINGAPORE") => match it.as_str() {
            "dividends" => Some(dec!(0.10)),
            "interest" => Some(dec!(0.0)),
            "royalties" => Some(dec!(0.05)),
            _ => None,
        },
        // Ireland-Switzerland
        ("CH" | "SWITZERLAND", "IE" | "IRELAND") => match it.as_str() {
            "dividends" => Some(dec!(0.0)),
            "interest" => Some(dec!(0.0)),
            "royalties" => Some(dec!(0.0)),
            _ => None,
        },
        _ => None,
    }
}

/// Normalize a jurisdiction name for matching (uppercase, common aliases).
fn normalize_jurisdiction(j: &str) -> String {
    j.trim().to_uppercase()
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

fn validate_input(input: &TreatyNetworkInput) -> CorpFinanceResult<()> {
    if input.source_jurisdiction.trim().is_empty() {
        return Err(CorpFinanceError::InvalidInput {
            field: "source_jurisdiction".into(),
            reason: "Source jurisdiction must not be empty".into(),
        });
    }
    if input.recipient_jurisdiction.trim().is_empty() {
        return Err(CorpFinanceError::InvalidInput {
            field: "recipient_jurisdiction".into(),
            reason: "Recipient jurisdiction must not be empty".into(),
        });
    }
    if input.income_types.is_empty() {
        return Err(CorpFinanceError::InvalidInput {
            field: "income_types".into(),
            reason: "At least one income flow is required".into(),
        });
    }
    for (i, flow) in input.income_types.iter().enumerate() {
        if flow.amount < dec!(0) {
            return Err(CorpFinanceError::InvalidInput {
                field: format!("income_types[{}].amount", i),
                reason: "Amount must be non-negative".into(),
            });
        }
        if flow.domestic_wht_rate < dec!(0) || flow.domestic_wht_rate > dec!(1) {
            return Err(CorpFinanceError::InvalidInput {
                field: format!("income_types[{}].domestic_wht_rate", i),
                reason: "WHT rate must be between 0 and 1".into(),
            });
        }
        let valid_types = [
            "Dividends",
            "Interest",
            "Royalties",
            "ManagementFees",
            "CapitalGains",
            "Services",
        ];
        if !valid_types.contains(&flow.income_type.as_str()) {
            return Err(CorpFinanceError::InvalidInput {
                field: format!("income_types[{}].income_type", i),
                reason: format!(
                    "Invalid income type '{}'. Valid: {:?}",
                    flow.income_type, valid_types
                ),
            });
        }
    }
    if let Some(ref rates) = input.treaty_rates {
        for (i, rate) in rates.iter().enumerate() {
            if rate.treaty_rate < dec!(0) || rate.treaty_rate > dec!(1) {
                return Err(CorpFinanceError::InvalidInput {
                    field: format!("treaty_rates[{}].treaty_rate", i),
                    reason: "Treaty rate must be between 0 and 1".into(),
                });
            }
        }
    }
    let valid_entities = [
        "Corporation",
        "Partnership",
        "Fund",
        "Trust",
        "Individual",
        "SWF",
        "PensionFund",
    ];
    if !valid_entities.contains(&input.recipient_entity_type.as_str()) {
        return Err(CorpFinanceError::InvalidInput {
            field: "recipient_entity_type".into(),
            reason: format!(
                "Invalid entity type '{}'. Valid: {:?}",
                input.recipient_entity_type, valid_entities
            ),
        });
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Treaty rate resolution
// ---------------------------------------------------------------------------

/// Resolve the treaty rate for a specific income type between two jurisdictions.
/// Uses user-supplied treaty_rates first, falls back to built-in data.
fn resolve_treaty_rate(
    source: &str,
    recipient: &str,
    income_type: &str,
    user_rates: &Option<Vec<TreatyRate>>,
) -> Option<Decimal> {
    // Check user-supplied rates first
    if let Some(ref rates) = user_rates {
        for tr in rates {
            if tr.income_type == income_type {
                return Some(tr.treaty_rate);
            }
        }
    }
    // Fall back to built-in
    builtin_treaty_rate(source, recipient, income_type)
}

// ---------------------------------------------------------------------------
// Anti-avoidance assessment
// ---------------------------------------------------------------------------

fn assess_anti_avoidance(
    input: &TreatyNetworkInput,
    conduit_used: bool,
) -> AntiAvoidanceAssessment {
    let mut risk_score: u32 = 0;

    // LOB risk
    let lob_risk = if input.lob_qualified {
        "Low".to_string()
    } else {
        let src = normalize_jurisdiction(&input.source_jurisdiction);
        if src == "US" {
            risk_score += 30;
            "High".to_string()
        } else {
            risk_score += 10;
            "Medium".to_string()
        }
    };

    // PPT risk
    let ppt_risk = if input.ppt_met {
        "Low".to_string()
    } else {
        risk_score += 25;
        if conduit_used {
            risk_score += 10;
            "High".to_string()
        } else {
            "Medium".to_string()
        }
    };

    // Beneficial ownership risk
    let bo_risk = if input.beneficial_owner {
        "Low".to_string()
    } else {
        risk_score += 25;
        if conduit_used {
            risk_score += 10;
            "High".to_string()
        } else {
            "Medium".to_string()
        }
    };

    // Conduit usage itself increases risk
    if conduit_used {
        risk_score += 15;
    }

    // Entity type adjustments
    match input.recipient_entity_type.as_str() {
        "PensionFund" | "SWF" => {
            // Generally lower risk
            risk_score = risk_score.saturating_sub(10);
        }
        "Trust" => {
            risk_score += 5;
        }
        _ => {}
    }

    // Cap at 100
    risk_score = risk_score.min(100);

    let challenge_likelihood = if risk_score <= 25 {
        "Unlikely".to_string()
    } else if risk_score <= 60 {
        "Possible".to_string()
    } else {
        "Likely".to_string()
    };

    AntiAvoidanceAssessment {
        lob_risk,
        ppt_risk,
        beneficial_ownership_risk: bo_risk,
        overall_treaty_shopping_risk: risk_score,
        challenge_likelihood,
    }
}

// ---------------------------------------------------------------------------
// Entity benefits assessment
// ---------------------------------------------------------------------------

fn assess_entity_benefits(input: &TreatyNetworkInput) -> EntityBenefits {
    let entity = input.recipient_entity_type.as_str();

    let pension_exemption = entity == "PensionFund";
    let swf_exemption = entity == "SWF";

    // Participation exemption: typically available for corporations with
    // sufficient ownership (10-25% thresholds depending on treaty)
    let (participation_exemption, participation_threshold) = if entity == "Corporation" {
        // Check if dividends are among the income types and beneficial owner
        let has_dividends = input
            .income_types
            .iter()
            .any(|f| f.income_type == "Dividends");
        if has_dividends && input.beneficial_owner {
            (true, Some(dec!(0.10)))
        } else {
            (false, None)
        }
    } else {
        (false, None)
    };

    // Look-through available for partnerships and some funds
    let look_through_available = matches!(entity, "Partnership" | "Fund");

    EntityBenefits {
        pension_exemption,
        swf_exemption,
        participation_exemption,
        participation_threshold,
        look_through_available,
    }
}

// ---------------------------------------------------------------------------
// Conduit analysis
// ---------------------------------------------------------------------------

/// Compute effective WHT for a two-hop route Source -> Intermediary -> Recipient
/// using the weighted average rate across all income flows.
fn compute_conduit_route(
    source: &str,
    intermediary: &str,
    recipient: &str,
    income_flows: &[IncomeFlow],
    user_rates: &Option<Vec<TreatyRate>>,
    direct_treaty_tax: Decimal,
) -> ConduitRoute {
    let total_amount: Decimal = income_flows.iter().map(|f| f.amount).sum();
    if total_amount == dec!(0) {
        return ConduitRoute {
            intermediary: intermediary.to_string(),
            leg1_rate: dec!(0),
            leg2_rate: dec!(0),
            combined_effective_rate: dec!(0),
            net_tax: dec!(0),
            savings_vs_direct: dec!(0),
            viable: false,
        };
    }

    let mut total_leg1_tax = dec!(0);
    let mut total_leg2_tax = dec!(0);

    for flow in income_flows {
        // Leg 1: Source -> Intermediary
        let leg1_rate = resolve_treaty_rate(source, intermediary, &flow.income_type, user_rates)
            .unwrap_or(flow.domestic_wht_rate);
        let leg1_tax = flow.amount * leg1_rate;

        // Amount after leg 1
        let net_after_leg1 = flow.amount - leg1_tax;

        // Leg 2: Intermediary -> Recipient
        let leg2_rate = resolve_treaty_rate(intermediary, recipient, &flow.income_type, user_rates)
            .unwrap_or(dec!(0));
        let leg2_tax = net_after_leg1 * leg2_rate;

        total_leg1_tax += leg1_tax;
        total_leg2_tax += leg2_tax;
    }

    let net_tax = total_leg1_tax + total_leg2_tax;
    let combined_effective_rate = if total_amount > dec!(0) {
        net_tax / total_amount
    } else {
        dec!(0)
    };

    // Weighted average leg rates
    let leg1_avg = if total_amount > dec!(0) {
        total_leg1_tax / total_amount
    } else {
        dec!(0)
    };
    let leg2_avg = if total_amount > dec!(0) {
        total_leg2_tax / total_amount
    } else {
        dec!(0)
    };

    let savings = direct_treaty_tax - net_tax;
    let viable = savings > dec!(0);

    ConduitRoute {
        intermediary: intermediary.to_string(),
        leg1_rate: leg1_avg,
        leg2_rate: leg2_avg,
        combined_effective_rate,
        net_tax,
        savings_vs_direct: savings,
        viable,
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Analyze a treaty network for optimal withholding tax structure.
///
/// Models direct treaty routes, conduit structures through intermediary
/// jurisdictions, anti-treaty-shopping risk, and entity-specific benefits.
pub fn analyze_treaty_network(
    input: &TreatyNetworkInput,
) -> CorpFinanceResult<TreatyNetworkOutput> {
    validate_input(input)?;

    let mut recommendations: Vec<String> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();

    // --- 1. Direct Treaty Analysis ---
    let mut flow_results: Vec<FlowResult> = Vec::new();
    let mut total_domestic_tax = dec!(0);
    let mut total_treaty_tax = dec!(0);

    for flow in &input.income_types {
        let tax_domestic = flow.amount * flow.domestic_wht_rate;

        let treaty_rate = resolve_treaty_rate(
            &input.source_jurisdiction,
            &input.recipient_jurisdiction,
            &flow.income_type,
            &input.treaty_rates,
        )
        .unwrap_or(flow.domestic_wht_rate);

        // If not beneficial owner, treaty benefits may be denied
        let effective_treaty_rate = if !input.beneficial_owner {
            flow.domestic_wht_rate
        } else {
            treaty_rate
        };

        let tax_treaty = flow.amount * effective_treaty_rate;
        let savings = tax_domestic - tax_treaty;

        total_domestic_tax += tax_domestic;
        total_treaty_tax += tax_treaty;

        flow_results.push(FlowResult {
            income_type: flow.income_type.clone(),
            amount: flow.amount,
            domestic_rate: flow.domestic_wht_rate,
            treaty_rate: effective_treaty_rate,
            tax_domestic,
            tax_treaty,
            treaty_savings: savings,
        });
    }

    let total_savings = total_domestic_tax - total_treaty_tax;

    let direct_route = DirectTreatyResult {
        income_flows: flow_results,
        total_domestic_tax,
        total_treaty_tax,
        total_savings,
    };

    // --- 2. Conduit Analysis ---
    let conduit_analysis: Vec<ConduitRoute> = input
        .intermediary_jurisdictions
        .iter()
        .map(|intermediary| {
            compute_conduit_route(
                &input.source_jurisdiction,
                intermediary,
                &input.recipient_jurisdiction,
                &input.income_types,
                &input.treaty_rates,
                total_treaty_tax,
            )
        })
        .collect();

    // --- 3. Optimal Route ---
    // Start with direct route as baseline
    let total_amount: Decimal = input.income_types.iter().map(|f| f.amount).sum();

    let mut best_tax = total_treaty_tax;
    let mut best_route_desc = format!(
        "Direct: {} -> {}",
        input.source_jurisdiction, input.recipient_jurisdiction
    );

    let conduit_used;

    // Check conduit routes
    let best_conduit = conduit_analysis.iter().filter(|c| c.viable).min_by(|a, b| {
        a.net_tax
            .partial_cmp(&b.net_tax)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    if let Some(bc) = best_conduit {
        if bc.net_tax < best_tax {
            best_tax = bc.net_tax;
            best_route_desc = format!(
                "Conduit: {} -> {} -> {}",
                input.source_jurisdiction, bc.intermediary, input.recipient_jurisdiction
            );
            conduit_used = true;
            recommendations.push(format!(
                "Consider routing through {} to reduce overall WHT from {} to {}",
                bc.intermediary, total_treaty_tax, bc.net_tax
            ));
        } else {
            conduit_used = false;
        }
    } else {
        conduit_used = false;
    }

    let effective_rate = if total_amount > dec!(0) {
        best_tax / total_amount
    } else {
        dec!(0)
    };

    let optimal_route = OptimalRoute {
        route_description: best_route_desc,
        effective_rate,
        total_tax: best_tax,
        savings_vs_domestic: total_domestic_tax - best_tax,
    };

    // --- 4. Anti-Avoidance Assessment ---
    let anti_avoidance = assess_anti_avoidance(input, conduit_used);

    if anti_avoidance.overall_treaty_shopping_risk > 50 {
        warnings.push(format!(
            "High treaty shopping risk (score {}). Ensure substance and documentation.",
            anti_avoidance.overall_treaty_shopping_risk
        ));
    }

    // --- 5. Entity Benefits ---
    let entity_benefits = assess_entity_benefits(input);

    if entity_benefits.pension_exemption {
        recommendations
            .push("Pension fund exemption may apply — confirm treaty Article coverage".to_string());
    }
    if entity_benefits.swf_exemption {
        recommendations
            .push("Sovereign wealth fund exemption may reduce or eliminate WHT".to_string());
    }
    if entity_benefits.participation_exemption {
        recommendations.push(format!(
            "Participation exemption available for dividends (threshold: {}%)",
            entity_benefits
                .participation_threshold
                .unwrap_or(dec!(0.10))
                * dec!(100)
        ));
    }
    if entity_benefits.look_through_available {
        recommendations.push(
            "Look-through treatment may be available — treaty benefits apply at partner/investor level"
                .to_string(),
        );
    }

    // Beneficial ownership warning
    if !input.beneficial_owner {
        warnings
            .push("Beneficial ownership test not met — treaty benefits may be denied".to_string());
    }
    if !input.lob_qualified {
        let src = normalize_jurisdiction(&input.source_jurisdiction);
        if src == "US" {
            warnings.push(
                "LOB not qualified for US treaty — may be denied treaty benefits".to_string(),
            );
        }
    }
    if !input.ppt_met {
        warnings.push(
            "PPT (Principal Purpose Test) not met — MLI may deny treaty benefits".to_string(),
        );
    }

    // General recommendations
    if total_savings > dec!(0) {
        recommendations.push(format!(
            "Direct treaty route saves {} vs domestic WHT",
            total_savings
        ));
    }

    Ok(TreatyNetworkOutput {
        direct_route,
        conduit_analysis,
        optimal_route,
        anti_avoidance,
        entity_benefits,
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

    fn make_basic_input() -> TreatyNetworkInput {
        TreatyNetworkInput {
            source_jurisdiction: "US".to_string(),
            recipient_jurisdiction: "UK".to_string(),
            income_types: vec![
                IncomeFlow {
                    income_type: "Dividends".to_string(),
                    amount: dec!(1_000_000),
                    domestic_wht_rate: dec!(0.30),
                },
                IncomeFlow {
                    income_type: "Interest".to_string(),
                    amount: dec!(500_000),
                    domestic_wht_rate: dec!(0.30),
                },
                IncomeFlow {
                    income_type: "Royalties".to_string(),
                    amount: dec!(200_000),
                    domestic_wht_rate: dec!(0.30),
                },
            ],
            treaty_rates: None,
            intermediary_jurisdictions: vec![
                "Netherlands".to_string(),
                "Luxembourg".to_string(),
                "Singapore".to_string(),
                "Ireland".to_string(),
                "Switzerland".to_string(),
            ],
            recipient_entity_type: "Corporation".to_string(),
            beneficial_owner: true,
            lob_qualified: true,
            ppt_met: true,
        }
    }

    // --- Validation tests ---

    #[test]
    fn test_empty_source_jurisdiction() {
        let mut input = make_basic_input();
        input.source_jurisdiction = "".to_string();
        let result = analyze_treaty_network(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_recipient_jurisdiction() {
        let mut input = make_basic_input();
        input.recipient_jurisdiction = "".to_string();
        let result = analyze_treaty_network(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_income_types() {
        let mut input = make_basic_input();
        input.income_types = vec![];
        let result = analyze_treaty_network(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_negative_amount() {
        let mut input = make_basic_input();
        input.income_types[0].amount = dec!(-100);
        let result = analyze_treaty_network(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_wht_rate_above_1() {
        let mut input = make_basic_input();
        input.income_types[0].domestic_wht_rate = dec!(1.5);
        let result = analyze_treaty_network(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_wht_rate_below_0() {
        let mut input = make_basic_input();
        input.income_types[0].domestic_wht_rate = dec!(-0.1);
        let result = analyze_treaty_network(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_income_type() {
        let mut input = make_basic_input();
        input.income_types[0].income_type = "Invalid".to_string();
        let result = analyze_treaty_network(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_entity_type() {
        let mut input = make_basic_input();
        input.recipient_entity_type = "Alien".to_string();
        let result = analyze_treaty_network(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_treaty_rate() {
        let mut input = make_basic_input();
        input.treaty_rates = Some(vec![TreatyRate {
            income_type: "Dividends".to_string(),
            treaty_rate: dec!(1.5),
            qualifying_conditions: vec![],
        }]);
        let result = analyze_treaty_network(&input);
        assert!(result.is_err());
    }

    // --- Direct treaty analysis tests ---

    #[test]
    fn test_basic_us_uk_direct_treaty() {
        let input = make_basic_input();
        let output = analyze_treaty_network(&input).unwrap();
        // US-UK treaty: dividends 15%, interest 0%, royalties 0%
        assert_eq!(output.direct_route.income_flows.len(), 3);

        let div = &output.direct_route.income_flows[0];
        assert_eq!(div.treaty_rate, dec!(0.15));
        assert_eq!(div.tax_domestic, dec!(300_000)); // 1M * 0.30
        assert_eq!(div.tax_treaty, dec!(150_000)); // 1M * 0.15
        assert_eq!(div.treaty_savings, dec!(150_000));

        let int = &output.direct_route.income_flows[1];
        assert_eq!(int.treaty_rate, dec!(0.0));
        assert_eq!(int.tax_treaty, dec!(0));
    }

    #[test]
    fn test_total_savings_calculation() {
        let input = make_basic_input();
        let output = analyze_treaty_network(&input).unwrap();
        // Domestic: 300k + 150k + 60k = 510k
        // Treaty: 150k + 0 + 0 = 150k
        // Savings: 360k
        assert_eq!(output.direct_route.total_domestic_tax, dec!(510_000));
        assert_eq!(output.direct_route.total_treaty_tax, dec!(150_000));
        assert_eq!(output.direct_route.total_savings, dec!(360_000));
    }

    #[test]
    fn test_user_supplied_treaty_rates_override() {
        let mut input = make_basic_input();
        input.treaty_rates = Some(vec![
            TreatyRate {
                income_type: "Dividends".to_string(),
                treaty_rate: dec!(0.05),
                qualifying_conditions: vec!["25% ownership".to_string()],
            },
            TreatyRate {
                income_type: "Interest".to_string(),
                treaty_rate: dec!(0.10),
                qualifying_conditions: vec![],
            },
        ]);
        let output = analyze_treaty_network(&input).unwrap();
        let div = &output.direct_route.income_flows[0];
        assert_eq!(div.treaty_rate, dec!(0.05));
        let int = &output.direct_route.income_flows[1];
        assert_eq!(int.treaty_rate, dec!(0.10));
    }

    #[test]
    fn test_no_treaty_falls_back_to_domestic() {
        let mut input = make_basic_input();
        input.source_jurisdiction = "XX".to_string(); // Unknown jurisdiction
        input.recipient_jurisdiction = "YY".to_string();
        input.income_types = vec![IncomeFlow {
            income_type: "Dividends".to_string(),
            amount: dec!(1_000_000),
            domestic_wht_rate: dec!(0.25),
        }];
        input.intermediary_jurisdictions = vec![];
        let output = analyze_treaty_network(&input).unwrap();
        // No treaty: treaty_rate == domestic_rate
        assert_eq!(output.direct_route.income_flows[0].treaty_rate, dec!(0.25));
        assert_eq!(output.direct_route.total_savings, dec!(0));
    }

    #[test]
    fn test_zero_amount_flow() {
        let mut input = make_basic_input();
        input.income_types = vec![IncomeFlow {
            income_type: "Dividends".to_string(),
            amount: dec!(0),
            domestic_wht_rate: dec!(0.30),
        }];
        input.intermediary_jurisdictions = vec![];
        let output = analyze_treaty_network(&input).unwrap();
        assert_eq!(output.direct_route.total_domestic_tax, dec!(0));
        assert_eq!(output.direct_route.total_treaty_tax, dec!(0));
    }

    // --- Conduit analysis tests ---

    #[test]
    fn test_conduit_routes_generated() {
        let input = make_basic_input();
        let output = analyze_treaty_network(&input).unwrap();
        assert_eq!(output.conduit_analysis.len(), 5);
    }

    #[test]
    fn test_conduit_netherlands_route() {
        let input = make_basic_input();
        let output = analyze_treaty_network(&input).unwrap();
        let nl = output
            .conduit_analysis
            .iter()
            .find(|c| c.intermediary == "Netherlands")
            .unwrap();
        // US->NL: dividends 15%, interest 0%, royalties 0%
        // NL->UK: dividends 10%, interest 0%, royalties 0%
        // Two-hop tax should be calculated
        assert!(nl.net_tax >= dec!(0));
    }

    #[test]
    fn test_conduit_viability_flag() {
        let input = make_basic_input();
        let output = analyze_treaty_network(&input).unwrap();
        // At least check that viable flag is set correctly
        for route in &output.conduit_analysis {
            if route.savings_vs_direct > dec!(0) {
                assert!(route.viable);
            } else {
                assert!(!route.viable);
            }
        }
    }

    #[test]
    fn test_no_intermediaries() {
        let mut input = make_basic_input();
        input.intermediary_jurisdictions = vec![];
        let output = analyze_treaty_network(&input).unwrap();
        assert!(output.conduit_analysis.is_empty());
    }

    #[test]
    fn test_conduit_combined_effective_rate() {
        let mut input = make_basic_input();
        input.income_types = vec![IncomeFlow {
            income_type: "Dividends".to_string(),
            amount: dec!(1_000_000),
            domestic_wht_rate: dec!(0.30),
        }];
        input.intermediary_jurisdictions = vec!["Netherlands".to_string()];
        let output = analyze_treaty_network(&input).unwrap();
        let nl = &output.conduit_analysis[0];
        // Combined rate should be >= 0 and <= 1
        assert!(nl.combined_effective_rate >= dec!(0));
        assert!(nl.combined_effective_rate <= dec!(1));
    }

    // --- Optimal route tests ---

    #[test]
    fn test_optimal_route_selects_lowest_tax() {
        let input = make_basic_input();
        let output = analyze_treaty_network(&input).unwrap();
        // Optimal route should have the lowest effective tax
        assert!(output.optimal_route.total_tax <= output.direct_route.total_treaty_tax);
        for route in &output.conduit_analysis {
            if route.viable {
                assert!(output.optimal_route.total_tax <= route.net_tax);
            }
        }
    }

    #[test]
    fn test_optimal_route_savings_vs_domestic() {
        let input = make_basic_input();
        let output = analyze_treaty_network(&input).unwrap();
        assert_eq!(
            output.optimal_route.savings_vs_domestic,
            output.direct_route.total_domestic_tax - output.optimal_route.total_tax
        );
    }

    #[test]
    fn test_optimal_route_effective_rate() {
        let input = make_basic_input();
        let output = analyze_treaty_network(&input).unwrap();
        let total_amount: Decimal = input.income_types.iter().map(|f| f.amount).sum();
        let expected_rate = output.optimal_route.total_tax / total_amount;
        assert_eq!(output.optimal_route.effective_rate, expected_rate);
    }

    // --- Anti-avoidance tests ---

    #[test]
    fn test_low_risk_all_qualified() {
        let input = make_basic_input();
        let output = analyze_treaty_network(&input).unwrap();
        assert_eq!(output.anti_avoidance.lob_risk, "Low");
        assert_eq!(output.anti_avoidance.ppt_risk, "Low");
        assert_eq!(output.anti_avoidance.beneficial_ownership_risk, "Low");
        assert!(output.anti_avoidance.overall_treaty_shopping_risk <= 25);
    }

    #[test]
    fn test_high_risk_no_lob_us_source() {
        let mut input = make_basic_input();
        input.lob_qualified = false;
        let output = analyze_treaty_network(&input).unwrap();
        assert_eq!(output.anti_avoidance.lob_risk, "High");
        assert!(output.anti_avoidance.overall_treaty_shopping_risk >= 30);
    }

    #[test]
    fn test_high_risk_no_ppt() {
        let mut input = make_basic_input();
        input.ppt_met = false;
        let output = analyze_treaty_network(&input).unwrap();
        assert_ne!(output.anti_avoidance.ppt_risk, "Low");
        assert!(output.anti_avoidance.overall_treaty_shopping_risk > 0);
    }

    #[test]
    fn test_high_risk_no_beneficial_owner() {
        let mut input = make_basic_input();
        input.beneficial_owner = false;
        let output = analyze_treaty_network(&input).unwrap();
        assert_ne!(output.anti_avoidance.beneficial_ownership_risk, "Low");
    }

    #[test]
    fn test_challenge_likelihood_mapping() {
        // Low risk
        let input = make_basic_input();
        let output = analyze_treaty_network(&input).unwrap();
        assert_eq!(output.anti_avoidance.challenge_likelihood, "Unlikely");

        // High risk
        let mut high_risk = make_basic_input();
        high_risk.lob_qualified = false;
        high_risk.ppt_met = false;
        high_risk.beneficial_owner = false;
        let output2 = analyze_treaty_network(&high_risk).unwrap();
        assert_eq!(output2.anti_avoidance.challenge_likelihood, "Likely");
    }

    #[test]
    fn test_risk_capped_at_100() {
        let mut input = make_basic_input();
        input.lob_qualified = false;
        input.ppt_met = false;
        input.beneficial_owner = false;
        input.recipient_entity_type = "Trust".to_string();
        let output = analyze_treaty_network(&input).unwrap();
        assert!(output.anti_avoidance.overall_treaty_shopping_risk <= 100);
    }

    // --- Entity benefits tests ---

    #[test]
    fn test_pension_fund_exemption() {
        let mut input = make_basic_input();
        input.recipient_entity_type = "PensionFund".to_string();
        let output = analyze_treaty_network(&input).unwrap();
        assert!(output.entity_benefits.pension_exemption);
        assert!(!output.entity_benefits.swf_exemption);
    }

    #[test]
    fn test_swf_exemption() {
        let mut input = make_basic_input();
        input.recipient_entity_type = "SWF".to_string();
        let output = analyze_treaty_network(&input).unwrap();
        assert!(output.entity_benefits.swf_exemption);
        assert!(!output.entity_benefits.pension_exemption);
    }

    #[test]
    fn test_corporation_participation_exemption() {
        let input = make_basic_input();
        let output = analyze_treaty_network(&input).unwrap();
        assert!(output.entity_benefits.participation_exemption);
        assert_eq!(
            output.entity_benefits.participation_threshold,
            Some(dec!(0.10))
        );
    }

    #[test]
    fn test_partnership_look_through() {
        let mut input = make_basic_input();
        input.recipient_entity_type = "Partnership".to_string();
        let output = analyze_treaty_network(&input).unwrap();
        assert!(output.entity_benefits.look_through_available);
        assert!(!output.entity_benefits.participation_exemption);
    }

    #[test]
    fn test_fund_look_through() {
        let mut input = make_basic_input();
        input.recipient_entity_type = "Fund".to_string();
        let output = analyze_treaty_network(&input).unwrap();
        assert!(output.entity_benefits.look_through_available);
    }

    #[test]
    fn test_individual_no_special_benefits() {
        let mut input = make_basic_input();
        input.recipient_entity_type = "Individual".to_string();
        let output = analyze_treaty_network(&input).unwrap();
        assert!(!output.entity_benefits.pension_exemption);
        assert!(!output.entity_benefits.swf_exemption);
        assert!(!output.entity_benefits.participation_exemption);
        assert!(!output.entity_benefits.look_through_available);
    }

    // --- Warnings and recommendations tests ---

    #[test]
    fn test_beneficial_owner_warning() {
        let mut input = make_basic_input();
        input.beneficial_owner = false;
        let output = analyze_treaty_network(&input).unwrap();
        assert!(output
            .warnings
            .iter()
            .any(|w| w.contains("Beneficial ownership")));
    }

    #[test]
    fn test_lob_warning_for_us() {
        let mut input = make_basic_input();
        input.lob_qualified = false;
        let output = analyze_treaty_network(&input).unwrap();
        assert!(output.warnings.iter().any(|w| w.contains("LOB")));
    }

    #[test]
    fn test_ppt_warning() {
        let mut input = make_basic_input();
        input.ppt_met = false;
        let output = analyze_treaty_network(&input).unwrap();
        assert!(output.warnings.iter().any(|w| w.contains("PPT")));
    }

    #[test]
    fn test_savings_recommendation() {
        let input = make_basic_input();
        let output = analyze_treaty_network(&input).unwrap();
        assert!(output.recommendations.iter().any(|r| r.contains("saves")));
    }

    // --- Built-in treaty rate tests ---

    #[test]
    fn test_builtin_us_uk_dividends() {
        assert_eq!(
            builtin_treaty_rate("US", "UK", "Dividends"),
            Some(dec!(0.15))
        );
    }

    #[test]
    fn test_builtin_us_uk_interest() {
        assert_eq!(builtin_treaty_rate("US", "UK", "Interest"), Some(dec!(0.0)));
    }

    #[test]
    fn test_builtin_symmetry() {
        assert_eq!(
            builtin_treaty_rate("US", "UK", "Dividends"),
            builtin_treaty_rate("UK", "US", "Dividends")
        );
    }

    #[test]
    fn test_builtin_unknown_pair() {
        assert_eq!(builtin_treaty_rate("XX", "YY", "Dividends"), None);
    }

    #[test]
    fn test_builtin_nl_lux_eu_directive() {
        // EU Parent-Sub: 0% dividends between NL and LU
        assert_eq!(
            builtin_treaty_rate("Netherlands", "Luxembourg", "Dividends"),
            Some(dec!(0.0))
        );
    }

    #[test]
    fn test_builtin_case_insensitive() {
        assert_eq!(
            builtin_treaty_rate("us", "uk", "dividends"),
            Some(dec!(0.15))
        );
    }

    // --- All income types tested ---

    #[test]
    fn test_management_fees_flow() {
        let mut input = make_basic_input();
        input.income_types = vec![IncomeFlow {
            income_type: "ManagementFees".to_string(),
            amount: dec!(100_000),
            domestic_wht_rate: dec!(0.20),
        }];
        input.intermediary_jurisdictions = vec![];
        let output = analyze_treaty_network(&input).unwrap();
        assert_eq!(output.direct_route.income_flows.len(), 1);
    }

    #[test]
    fn test_capital_gains_flow() {
        let mut input = make_basic_input();
        input.income_types = vec![IncomeFlow {
            income_type: "CapitalGains".to_string(),
            amount: dec!(500_000),
            domestic_wht_rate: dec!(0.0),
        }];
        input.intermediary_jurisdictions = vec![];
        let output = analyze_treaty_network(&input).unwrap();
        assert_eq!(output.direct_route.total_domestic_tax, dec!(0));
    }

    #[test]
    fn test_services_flow() {
        let mut input = make_basic_input();
        input.income_types = vec![IncomeFlow {
            income_type: "Services".to_string(),
            amount: dec!(200_000),
            domestic_wht_rate: dec!(0.15),
        }];
        input.intermediary_jurisdictions = vec![];
        let output = analyze_treaty_network(&input).unwrap();
        assert_eq!(
            output.direct_route.income_flows[0].tax_domestic,
            dec!(30_000)
        );
    }

    // --- Edge cases ---

    #[test]
    fn test_same_source_and_recipient() {
        let mut input = make_basic_input();
        input.recipient_jurisdiction = "US".to_string();
        input.intermediary_jurisdictions = vec![];
        // Should still work — same jurisdiction
        let output = analyze_treaty_network(&input).unwrap();
        assert!(output.direct_route.total_treaty_tax >= dec!(0));
    }

    #[test]
    fn test_single_intermediary() {
        let mut input = make_basic_input();
        input.intermediary_jurisdictions = vec!["Singapore".to_string()];
        let output = analyze_treaty_network(&input).unwrap();
        assert_eq!(output.conduit_analysis.len(), 1);
    }

    #[test]
    fn test_multiple_flows_same_type() {
        let mut input = make_basic_input();
        input.income_types = vec![
            IncomeFlow {
                income_type: "Dividends".to_string(),
                amount: dec!(500_000),
                domestic_wht_rate: dec!(0.30),
            },
            IncomeFlow {
                income_type: "Dividends".to_string(),
                amount: dec!(500_000),
                domestic_wht_rate: dec!(0.30),
            },
        ];
        input.intermediary_jurisdictions = vec![];
        let output = analyze_treaty_network(&input).unwrap();
        assert_eq!(output.direct_route.income_flows.len(), 2);
        assert_eq!(output.direct_route.total_domestic_tax, dec!(300_000));
    }

    #[test]
    fn test_pension_fund_lower_risk_score() {
        let mut input_corp = make_basic_input();
        input_corp.lob_qualified = false;
        input_corp.ppt_met = false;
        let out_corp = analyze_treaty_network(&input_corp).unwrap();

        let mut input_pension = make_basic_input();
        input_pension.recipient_entity_type = "PensionFund".to_string();
        input_pension.lob_qualified = false;
        input_pension.ppt_met = false;
        let out_pension = analyze_treaty_network(&input_pension).unwrap();

        assert!(
            out_pension.anti_avoidance.overall_treaty_shopping_risk
                <= out_corp.anti_avoidance.overall_treaty_shopping_risk
        );
    }

    #[test]
    fn test_trust_higher_risk() {
        let mut input_trust = make_basic_input();
        input_trust.recipient_entity_type = "Trust".to_string();
        input_trust.lob_qualified = false;
        input_trust.ppt_met = false;
        input_trust.beneficial_owner = false;
        let out = analyze_treaty_network(&input_trust).unwrap();
        // Trust should have elevated risk
        assert!(out.anti_avoidance.overall_treaty_shopping_risk >= 60);
    }

    #[test]
    fn test_high_risk_treaty_shopping_warning() {
        let mut input = make_basic_input();
        input.lob_qualified = false;
        input.ppt_met = false;
        input.beneficial_owner = false;
        let output = analyze_treaty_network(&input).unwrap();
        assert!(output
            .warnings
            .iter()
            .any(|w| w.contains("treaty shopping risk")));
    }
}
