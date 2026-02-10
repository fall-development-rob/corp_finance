use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::error::CorpFinanceError;
use crate::types::*;
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

/// Whether to run the waterfall on going-concern value, liquidation value,
/// or both (producing a comparison).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ValuationType {
    GoingConcern,
    Liquidation,
    Both,
}

/// Priority ranking of a claim in the capital structure.  Variants are
/// ordered from highest priority (first to be paid) to lowest.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ClaimPriority {
    SuperPriority,
    Administrative,
    Priority,
    SecuredFirst,
    SecuredSecond,
    Senior,
    SeniorSubordinated,
    Subordinated,
    Mezzanine,
    Equity,
}

// ---------------------------------------------------------------------------
// Input types
// ---------------------------------------------------------------------------

/// A single claim against the debtor estate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claim {
    /// Human-readable identifier, e.g. "1st Lien Term Loan"
    pub name: String,
    /// Face (par) value of the claim
    pub amount: Money,
    /// Priority class in the absolute priority rule
    pub priority: ClaimPriority,
    /// Whether the claim is backed by collateral
    pub is_secured: bool,
    /// Value of collateral backing the claim (secured claims only)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub collateral_value: Option<Money>,
    /// Contractual interest rate (for accrued interest calculation)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interest_rate: Option<Rate>,
    /// Number of months of unpaid interest to accrue
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accrued_months: Option<u32>,
}

/// DIP (debtor-in-possession) financing facility.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DipFacility {
    /// Total DIP commitment drawn
    pub amount: Money,
    /// Whether the DIP primes existing secured debt
    pub priming: bool,
    /// Portion of DIP that rolls up pre-petition secured claims
    pub roll_up_amount: Money,
}

/// Full input for a restructuring recovery analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryAnalysisInput {
    /// Going-concern enterprise value
    pub enterprise_value: Money,
    /// Liquidation (fire-sale) value of assets
    pub liquidation_value: Money,
    /// Whether to run GC, liquidation, or both waterfalls
    pub valuation_type: ValuationType,
    /// Capital structure claims ordered by priority (highest first)
    pub claims: Vec<Claim>,
    /// Chapter 11 administrative costs (super-priority by statute)
    pub administrative_costs: Money,
    /// Optional DIP financing facility
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dip_facility: Option<DipFacility>,
    /// Cash on hand available for distribution
    pub cash_on_hand: Money,
}

// ---------------------------------------------------------------------------
// Output types
// ---------------------------------------------------------------------------

/// Recovery detail for a single claim.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaimRecovery {
    /// Claim identifier
    pub name: String,
    /// Face (par) value
    pub claim_amount: Money,
    /// Accrued but unpaid interest
    pub accrued_interest: Money,
    /// Face + accrued
    pub total_claim: Money,
    /// Amount actually recovered
    pub recovery_amount: Money,
    /// Recovery as a decimal fraction (0.0 to 1.0)
    pub recovery_rate: Decimal,
    /// Recovery expressed in cents on the dollar (0 to 100)
    pub recovery_cents_on_dollar: Decimal,
    /// True if recovery_rate < 1.0
    pub is_impaired: bool,
}

/// Detailed liquidation-scenario breakdown (only populated when
/// `ValuationType` is `Liquidation` or `Both`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiquidationDetail {
    /// Total distributable under liquidation
    pub liquidation_distributable: Money,
    /// Per-claim recoveries under liquidation
    pub claim_recoveries: Vec<ClaimRecovery>,
    /// Shortfall under liquidation
    pub shortfall: Money,
}

/// Top-level output of the recovery analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryAnalysisOutput {
    /// Total value available for distribution (EV + cash - admin costs)
    pub total_distributable: Money,
    /// Per-claim recovery breakdown
    pub claim_recoveries: Vec<ClaimRecovery>,
    /// First claim class that receives less than 100% recovery
    pub fulcrum_security: Option<String>,
    /// Sum of all claims (face + accrued)
    pub total_claims: Money,
    /// Total claims minus total distributable (zero-floored)
    pub shortfall: Money,
    /// (GC value - liquidation value) / liquidation value
    pub going_concern_premium: Option<Decimal>,
    /// Liquidation waterfall detail (when ValuationType includes liquidation)
    pub liquidation_analysis: Option<LiquidationDetail>,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Run a restructuring recovery analysis using the Absolute Priority Rule.
///
/// Distributes enterprise value (or liquidation value) through the capital
/// structure, paying each priority class in full before moving to the next.
/// Within a class, claims are paid pro-rata if funds are insufficient.
///
/// Returns per-claim recoveries, the fulcrum security, and optional
/// going-concern vs. liquidation comparison.
pub fn analyze_recovery(
    input: &RecoveryAnalysisInput,
) -> CorpFinanceResult<ComputationOutput<RecoveryAnalysisOutput>> {
    let start = Instant::now();
    let mut warnings: Vec<String> = Vec::new();

    validate_input(input)?;

    // Compute accrued interest for each claim once
    let enriched_claims: Vec<EnrichedClaim> = input
        .claims
        .iter()
        .map(|c| {
            let accrued = compute_accrued_interest(c);
            EnrichedClaim {
                claim: c.clone(),
                accrued_interest: accrued,
                total_claim: c.amount + accrued,
            }
        })
        .collect();

    let total_claims: Money = enriched_claims.iter().map(|ec| ec.total_claim).sum();

    // ---- Going-concern waterfall --------------------------------------------
    let gc_distributable = compute_distributable(input.enterprise_value, input);
    let gc_recoveries = run_waterfall(gc_distributable, &enriched_claims, input);
    let gc_fulcrum = find_fulcrum_security(&gc_recoveries);
    let gc_shortfall = (total_claims - gc_distributable).max(Decimal::ZERO);

    // ---- Liquidation waterfall (when requested) -----------------------------
    let liquidation_analysis = match input.valuation_type {
        ValuationType::GoingConcern => None,
        ValuationType::Liquidation | ValuationType::Both => {
            let liq_distributable = compute_distributable(input.liquidation_value, input);
            let liq_recoveries = run_waterfall(liq_distributable, &enriched_claims, input);
            let liq_shortfall = (total_claims - liq_distributable).max(Decimal::ZERO);
            Some(LiquidationDetail {
                liquidation_distributable: liq_distributable,
                claim_recoveries: liq_recoveries,
                shortfall: liq_shortfall,
            })
        }
    };

    // ---- Choose primary outputs based on valuation type ---------------------
    let (primary_distributable, primary_recoveries, primary_shortfall, primary_fulcrum) =
        match input.valuation_type {
            ValuationType::GoingConcern | ValuationType::Both => {
                (gc_distributable, gc_recoveries, gc_shortfall, gc_fulcrum)
            }
            ValuationType::Liquidation => {
                let liq_dist = compute_distributable(input.liquidation_value, input);
                let liq_rec = run_waterfall(liq_dist, &enriched_claims, input);
                let liq_short = (total_claims - liq_dist).max(Decimal::ZERO);
                let liq_ful = find_fulcrum_security(&liq_rec);
                (liq_dist, liq_rec, liq_short, liq_ful)
            }
        };

    // ---- Going-concern premium ----------------------------------------------
    let going_concern_premium = if input.liquidation_value > Decimal::ZERO {
        Some((input.enterprise_value - input.liquidation_value) / input.liquidation_value)
    } else {
        None
    };

    // ---- Warnings -----------------------------------------------------------
    if primary_fulcrum.is_some() {
        warnings.push("Fulcrum security identified: not all classes are made whole.".into());
    }

    // Check if equity receives any recovery (unusual in impaired structures)
    for rec in &primary_recoveries {
        if input
            .claims
            .iter()
            .any(|c| c.name == rec.name && c.priority == ClaimPriority::Equity)
            && rec.recovery_amount > Decimal::ZERO
            && primary_fulcrum.is_some()
        {
            warnings.push(format!(
                "Equity class '{}' receives recovery despite senior impairment.",
                rec.name
            ));
        }
    }

    if input.liquidation_value > input.enterprise_value {
        warnings.push(
            "Liquidation value exceeds going-concern value; going-concern premium is negative."
                .into(),
        );
    }

    let admin_pct = if input.enterprise_value > Decimal::ZERO {
        input.administrative_costs / input.enterprise_value * dec!(100)
    } else {
        Decimal::ZERO
    };
    if admin_pct > dec!(10) {
        warnings.push(format!(
            "Administrative costs are {admin_pct:.1}% of enterprise value (>10%)."
        ));
    }

    let output = RecoveryAnalysisOutput {
        total_distributable: primary_distributable,
        claim_recoveries: primary_recoveries,
        fulcrum_security: primary_fulcrum,
        total_claims,
        shortfall: primary_shortfall,
        going_concern_premium,
        liquidation_analysis,
    };

    let elapsed = start.elapsed().as_micros() as u64;
    Ok(with_metadata(
        "Restructuring Recovery Analysis (Absolute Priority Rule)",
        &serde_json::json!({
            "enterprise_value": input.enterprise_value.to_string(),
            "liquidation_value": input.liquidation_value.to_string(),
            "valuation_type": format!("{:?}", input.valuation_type),
            "num_claims": input.claims.len(),
            "administrative_costs": input.administrative_costs.to_string(),
            "dip_facility": input.dip_facility.is_some(),
            "cash_on_hand": input.cash_on_hand.to_string(),
        }),
        warnings,
        elapsed,
        output,
    ))
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Intermediate representation that bundles a claim with its computed accrued
/// interest and total claim amount.
#[derive(Debug, Clone)]
struct EnrichedClaim {
    claim: Claim,
    accrued_interest: Money,
    total_claim: Money,
}

/// Compute accrued but unpaid interest for a claim.
fn compute_accrued_interest(claim: &Claim) -> Money {
    match (claim.interest_rate, claim.accrued_months) {
        (Some(rate), Some(months)) => {
            // Simple interest: principal * annual_rate * (months / 12)
            claim.amount * rate * Decimal::from(months) / dec!(12)
        }
        _ => Decimal::ZERO,
    }
}

/// Total value available for distribution after deducting admin costs.
fn compute_distributable(base_value: Money, input: &RecoveryAnalysisInput) -> Money {
    let gross = base_value + input.cash_on_hand;
    (gross - input.administrative_costs).max(Decimal::ZERO)
}

/// Run the APR waterfall over the enriched claims, returning per-claim
/// recovery results.
fn run_waterfall(
    total_distributable: Money,
    enriched_claims: &[EnrichedClaim],
    input: &RecoveryAnalysisInput,
) -> Vec<ClaimRecovery> {
    let mut remaining = total_distributable;
    let mut recoveries: Vec<ClaimRecovery> = Vec::with_capacity(enriched_claims.len());

    // Deficiency claims that arise from under-collateralised secured debt.
    // These are collected and paid as unsecured/senior claims later.
    let mut deficiency_claims: Vec<(String, Money)> = Vec::new();

    // --- Phase 1: DIP facility (super-priority after admin) ------------------
    if let Some(dip) = &input.dip_facility {
        if dip.amount > Decimal::ZERO {
            let dip_total = dip.amount;
            let paid = remaining.min(dip_total);
            remaining -= paid;
            let rate = safe_divide(paid, dip_total);
            recoveries.push(ClaimRecovery {
                name: "DIP Facility".into(),
                claim_amount: dip.amount,
                accrued_interest: Decimal::ZERO,
                total_claim: dip_total,
                recovery_amount: paid,
                recovery_rate: rate,
                recovery_cents_on_dollar: rate * dec!(100),
                is_impaired: rate < Decimal::ONE,
            });
        }
    }

    // --- Phase 2: Walk claims by priority ------------------------------------
    // Group claims by priority and process each group.
    // Within the same priority class, distribute pro-rata if funds
    // are insufficient.

    // Collect unique priority classes in order (ClaimPriority derives Ord).
    let mut priority_classes: Vec<ClaimPriority> =
        enriched_claims.iter().map(|ec| ec.claim.priority).collect();
    priority_classes.sort();
    priority_classes.dedup();

    for priority in &priority_classes {
        // Skip DIP-level SuperPriority claims if they were already handled
        // through the DIP facility. If there is no DIP facility, super-priority
        // claims are processed normally.

        let class_claims: Vec<&EnrichedClaim> = enriched_claims
            .iter()
            .filter(|ec| ec.claim.priority == *priority)
            .collect();

        if class_claims.is_empty() {
            continue;
        }

        // For secured claims, handle collateral-limited recovery and
        // generate deficiency claims for the unsecured shortfall.
        if is_secured_priority(*priority) {
            for ec in &class_claims {
                if ec.claim.is_secured {
                    // Secured portion: min(total_claim, collateral_value)
                    let collateral = ec.claim.collateral_value.unwrap_or(ec.total_claim);
                    let secured_amount = ec.total_claim.min(collateral);
                    let paid = remaining.min(secured_amount);
                    remaining -= paid;

                    // Deficiency: anything above the collateral value
                    let deficiency = ec.total_claim - secured_amount;
                    if deficiency > Decimal::ZERO {
                        deficiency_claims
                            .push((format!("{} (deficiency)", ec.claim.name), deficiency));
                    }

                    let rate = safe_divide(paid, ec.total_claim);
                    recoveries.push(ClaimRecovery {
                        name: ec.claim.name.clone(),
                        claim_amount: ec.claim.amount,
                        accrued_interest: ec.accrued_interest,
                        total_claim: ec.total_claim,
                        recovery_amount: paid,
                        recovery_rate: rate,
                        recovery_cents_on_dollar: rate * dec!(100),
                        is_impaired: rate < Decimal::ONE,
                    });
                } else {
                    // Unsecured claim in a "secured" priority bucket --
                    // just participate in pro-rata with other unsecured
                    let paid = remaining.min(ec.total_claim);
                    remaining -= paid;
                    let rate = safe_divide(paid, ec.total_claim);
                    recoveries.push(ClaimRecovery {
                        name: ec.claim.name.clone(),
                        claim_amount: ec.claim.amount,
                        accrued_interest: ec.accrued_interest,
                        total_claim: ec.total_claim,
                        recovery_amount: paid,
                        recovery_rate: rate,
                        recovery_cents_on_dollar: rate * dec!(100),
                        is_impaired: rate < Decimal::ONE,
                    });
                }
            }
        } else {
            // Unsecured / subordinated / equity classes: pro-rata within class
            let total_class_claims: Money = class_claims.iter().map(|ec| ec.total_claim).sum();

            // Include deficiency claims if this is the Senior class
            let deficiency_total: Money = if *priority == ClaimPriority::Senior {
                deficiency_claims.iter().map(|(_, amt)| *amt).sum()
            } else {
                Decimal::ZERO
            };

            let combined_class = total_class_claims + deficiency_total;
            let available_for_class = remaining.min(combined_class);

            // Pro-rata factor for the entire combined class
            let pro_rata = safe_divide(available_for_class, combined_class);

            for ec in &class_claims {
                let paid = ec.total_claim * pro_rata;
                let rate = safe_divide(paid, ec.total_claim);
                recoveries.push(ClaimRecovery {
                    name: ec.claim.name.clone(),
                    claim_amount: ec.claim.amount,
                    accrued_interest: ec.accrued_interest,
                    total_claim: ec.total_claim,
                    recovery_amount: paid,
                    recovery_rate: rate,
                    recovery_cents_on_dollar: rate * dec!(100),
                    is_impaired: rate < Decimal::ONE,
                });
            }

            // Pay deficiency claims pro-rata within Senior class
            if *priority == ClaimPriority::Senior && deficiency_total > Decimal::ZERO {
                for (def_name, def_amount) in &deficiency_claims {
                    let paid = *def_amount * pro_rata;
                    let rate = safe_divide(paid, *def_amount);
                    recoveries.push(ClaimRecovery {
                        name: def_name.clone(),
                        claim_amount: *def_amount,
                        accrued_interest: Decimal::ZERO,
                        total_claim: *def_amount,
                        recovery_amount: paid,
                        recovery_rate: rate,
                        recovery_cents_on_dollar: rate * dec!(100),
                        is_impaired: rate < Decimal::ONE,
                    });
                }
            }

            remaining -= available_for_class;
        }
    }

    recoveries
}

/// Determine whether a priority class corresponds to secured debt.
fn is_secured_priority(p: ClaimPriority) -> bool {
    matches!(
        p,
        ClaimPriority::SecuredFirst | ClaimPriority::SecuredSecond
    )
}

/// Find the first claim class with recovery < 100% (the fulcrum security).
fn find_fulcrum_security(recoveries: &[ClaimRecovery]) -> Option<String> {
    recoveries
        .iter()
        .find(|r| r.is_impaired)
        .map(|r| r.name.clone())
}

/// Safe division that returns ZERO when the denominator is zero.
fn safe_divide(numerator: Decimal, denominator: Decimal) -> Decimal {
    if denominator.is_zero() {
        Decimal::ZERO
    } else {
        numerator / denominator
    }
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

fn validate_input(input: &RecoveryAnalysisInput) -> CorpFinanceResult<()> {
    if input.enterprise_value < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "enterprise_value".into(),
            reason: "Enterprise value cannot be negative.".into(),
        });
    }
    if input.liquidation_value < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "liquidation_value".into(),
            reason: "Liquidation value cannot be negative.".into(),
        });
    }
    if input.claims.is_empty() {
        return Err(CorpFinanceError::InvalidInput {
            field: "claims".into(),
            reason: "At least one claim is required.".into(),
        });
    }
    if input.administrative_costs < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "administrative_costs".into(),
            reason: "Administrative costs cannot be negative.".into(),
        });
    }
    if input.cash_on_hand < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "cash_on_hand".into(),
            reason: "Cash on hand cannot be negative.".into(),
        });
    }
    for claim in &input.claims {
        if claim.amount < Decimal::ZERO {
            return Err(CorpFinanceError::InvalidInput {
                field: format!("claim[{}].amount", claim.name),
                reason: "Claim amount cannot be negative.".into(),
            });
        }
        if let Some(cv) = claim.collateral_value {
            if cv < Decimal::ZERO {
                return Err(CorpFinanceError::InvalidInput {
                    field: format!("claim[{}].collateral_value", claim.name),
                    reason: "Collateral value cannot be negative.".into(),
                });
            }
        }
    }
    if let Some(dip) = &input.dip_facility {
        if dip.amount < Decimal::ZERO {
            return Err(CorpFinanceError::InvalidInput {
                field: "dip_facility.amount".into(),
                reason: "DIP facility amount cannot be negative.".into(),
            });
        }
        if dip.roll_up_amount < Decimal::ZERO {
            return Err(CorpFinanceError::InvalidInput {
                field: "dip_facility.roll_up_amount".into(),
                reason: "DIP roll-up amount cannot be negative.".into(),
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
    use rust_decimal_macros::dec;

    // --- Helpers -------------------------------------------------------------

    fn simple_claim(name: &str, amount: Money, priority: ClaimPriority) -> Claim {
        Claim {
            name: name.into(),
            amount,
            priority,
            is_secured: false,
            collateral_value: None,
            interest_rate: None,
            accrued_months: None,
        }
    }

    fn secured_claim(
        name: &str,
        amount: Money,
        priority: ClaimPriority,
        collateral: Money,
    ) -> Claim {
        Claim {
            name: name.into(),
            amount,
            priority,
            is_secured: true,
            collateral_value: Some(collateral),
            interest_rate: None,
            accrued_months: None,
        }
    }

    fn base_input(claims: Vec<Claim>) -> RecoveryAnalysisInput {
        RecoveryAnalysisInput {
            enterprise_value: dec!(500),
            liquidation_value: dec!(300),
            valuation_type: ValuationType::GoingConcern,
            claims,
            administrative_costs: dec!(20),
            dip_facility: None,
            cash_on_hand: dec!(30),
        }
    }

    // --- Test cases ----------------------------------------------------------

    #[test]
    fn test_simple_two_claim_waterfall() {
        // Senior: 400, Equity: 200. EV=500, cash=30, admin=20 => distributable=510
        let input = base_input(vec![
            simple_claim("Senior Notes", dec!(400), ClaimPriority::Senior),
            simple_claim("Equity", dec!(200), ClaimPriority::Equity),
        ]);
        let result = analyze_recovery(&input).unwrap();
        let out = &result.result;

        // distributable = 500 + 30 - 20 = 510
        assert_eq!(out.total_distributable, dec!(510));

        // Senior: 400 fully recovered
        assert_eq!(out.claim_recoveries[0].name, "Senior Notes");
        assert_eq!(out.claim_recoveries[0].recovery_amount, dec!(400));
        assert_eq!(out.claim_recoveries[0].recovery_rate, Decimal::ONE);
        assert!(!out.claim_recoveries[0].is_impaired);

        // Equity: 510 - 400 = 110 remaining out of 200
        assert_eq!(out.claim_recoveries[1].name, "Equity");
        assert_eq!(out.claim_recoveries[1].recovery_amount, dec!(110));
        assert!(out.claim_recoveries[1].is_impaired);

        assert_eq!(out.fulcrum_security, Some("Equity".into()));
        assert_eq!(out.total_claims, dec!(600));
        assert_eq!(out.shortfall, dec!(90)); // 600 - 510
    }

    #[test]
    fn test_full_capital_structure() {
        // Full stack: DIP + secured + unsecured + mezz + equity
        let claims = vec![
            secured_claim(
                "1st Lien TL",
                dec!(200),
                ClaimPriority::SecuredFirst,
                dec!(250),
            ),
            secured_claim(
                "2nd Lien TL",
                dec!(100),
                ClaimPriority::SecuredSecond,
                dec!(80),
            ),
            simple_claim("Senior Notes", dec!(150), ClaimPriority::Senior),
            simple_claim("Mezz Notes", dec!(100), ClaimPriority::Mezzanine),
            simple_claim("Equity", dec!(300), ClaimPriority::Equity),
        ];
        let mut input = base_input(claims);
        input.enterprise_value = dec!(600);
        input.cash_on_hand = dec!(50);
        input.administrative_costs = dec!(30);
        input.dip_facility = Some(DipFacility {
            amount: dec!(50),
            priming: true,
            roll_up_amount: dec!(10),
        });
        // distributable = 600 + 50 - 30 = 620. DIP takes 50 => 570 remaining.

        let result = analyze_recovery(&input).unwrap();
        let out = &result.result;

        assert_eq!(out.total_distributable, dec!(620));

        // DIP: 50/50 = 100%
        let dip = out
            .claim_recoveries
            .iter()
            .find(|r| r.name == "DIP Facility")
            .unwrap();
        assert_eq!(dip.recovery_rate, Decimal::ONE);

        // 1st Lien: 200 claim, 250 collateral => secured portion = 200.
        // Remaining after DIP = 570. 1st lien gets 200 => 370 remaining.
        let first_lien = out
            .claim_recoveries
            .iter()
            .find(|r| r.name == "1st Lien TL")
            .unwrap();
        assert_eq!(first_lien.recovery_amount, dec!(200));
        assert!(!first_lien.is_impaired);

        // 2nd Lien: 100 claim, 80 collateral => secured = 80, deficiency = 20.
        // 370 remaining, pays 80 for secured portion => 290 remaining.
        let second_lien = out
            .claim_recoveries
            .iter()
            .find(|r| r.name == "2nd Lien TL")
            .unwrap();
        assert_eq!(second_lien.recovery_amount, dec!(80));
        assert!(second_lien.is_impaired);

        // Senior + deficiency (20): combined class = 150 + 20 = 170.
        // 290 remaining, pays 170 in full => 120 remaining.
        let senior = out
            .claim_recoveries
            .iter()
            .find(|r| r.name == "Senior Notes")
            .unwrap();
        assert_eq!(senior.recovery_amount, dec!(150));
        assert!(!senior.is_impaired);

        // Mezz: 120 remaining for 100 claim => fully recovered.
        let mezz = out
            .claim_recoveries
            .iter()
            .find(|r| r.name == "Mezz Notes")
            .unwrap();
        assert_eq!(mezz.recovery_amount, dec!(100));
        assert!(!mezz.is_impaired);

        // Equity: 120 - 100 = 20 remaining for 300 claim.
        // But we need to subtract deficiency payout too. Let's check.
        // After mezz: 120 - 100 = 20 remaining.
        // Wait -- let me recalculate. After senior class (which includes
        // deficiency): 290 - 170 = 120 remaining for mezz (100) => 20 remaining.
        let equity = out
            .claim_recoveries
            .iter()
            .find(|r| r.name == "Equity")
            .unwrap();
        assert!(equity.is_impaired);
        assert!(equity.recovery_amount < dec!(300));
    }

    #[test]
    fn test_fulcrum_security_identification() {
        // Two senior claims and one equity. Distributable just barely covers seniors.
        let claims = vec![
            simple_claim("Senior A", dec!(200), ClaimPriority::Senior),
            simple_claim("Senior B", dec!(200), ClaimPriority::Senior),
            simple_claim("Equity", dec!(100), ClaimPriority::Equity),
        ];
        let mut input = base_input(claims);
        input.enterprise_value = dec!(350);
        input.cash_on_hand = dec!(0);
        input.administrative_costs = dec!(0);
        // distributable = 350

        let result = analyze_recovery(&input).unwrap();
        let out = &result.result;

        // Both seniors share 350 pro-rata out of 400 total => 87.5% each
        assert!(out.claim_recoveries[0].is_impaired);
        assert!(out.claim_recoveries[1].is_impaired);
        // Fulcrum is the first impaired claim
        assert!(
            out.fulcrum_security == Some("Senior A".into())
                || out.fulcrum_security == Some("Senior B".into())
        );
        // Equity gets zero
        let equity = out
            .claim_recoveries
            .iter()
            .find(|r| r.name == "Equity")
            .unwrap();
        assert_eq!(equity.recovery_amount, Decimal::ZERO);
    }

    #[test]
    fn test_all_secured_scenario() {
        let claims = vec![
            secured_claim(
                "1st Lien",
                dec!(300),
                ClaimPriority::SecuredFirst,
                dec!(400),
            ),
            secured_claim(
                "2nd Lien",
                dec!(200),
                ClaimPriority::SecuredSecond,
                dec!(250),
            ),
        ];
        let mut input = base_input(claims);
        input.enterprise_value = dec!(600);
        input.cash_on_hand = dec!(0);
        input.administrative_costs = dec!(0);
        // distributable = 600

        let result = analyze_recovery(&input).unwrap();
        let out = &result.result;

        // 1st Lien: 300 claim, 400 collateral => secured = 300. Paid 300.
        let first = &out.claim_recoveries[0];
        assert_eq!(first.recovery_amount, dec!(300));
        assert!(!first.is_impaired);

        // 2nd Lien: 200 claim, 250 collateral => secured = 200. Paid 200.
        let second = &out.claim_recoveries[1];
        assert_eq!(second.recovery_amount, dec!(200));
        assert!(!second.is_impaired);

        assert!(out.fulcrum_security.is_none());
        assert_eq!(out.shortfall, Decimal::ZERO);
    }

    #[test]
    fn test_total_impairment_ev_zero() {
        let claims = vec![
            simple_claim("Senior", dec!(200), ClaimPriority::Senior),
            simple_claim("Equity", dec!(100), ClaimPriority::Equity),
        ];
        let mut input = base_input(claims);
        input.enterprise_value = Decimal::ZERO;
        input.cash_on_hand = Decimal::ZERO;
        input.administrative_costs = Decimal::ZERO;
        // distributable = 0

        let result = analyze_recovery(&input).unwrap();
        let out = &result.result;

        assert_eq!(out.total_distributable, Decimal::ZERO);
        for rec in &out.claim_recoveries {
            assert_eq!(rec.recovery_amount, Decimal::ZERO);
            assert!(rec.is_impaired);
        }
        assert_eq!(out.shortfall, dec!(300));
        assert_eq!(out.fulcrum_security, Some("Senior".into()));
    }

    #[test]
    fn test_dip_priming() {
        // DIP primes 1st lien. EV barely covers DIP + some 1st lien.
        let claims = vec![
            secured_claim(
                "1st Lien",
                dec!(200),
                ClaimPriority::SecuredFirst,
                dec!(200),
            ),
            simple_claim("Equity", dec!(100), ClaimPriority::Equity),
        ];
        let mut input = base_input(claims);
        input.enterprise_value = dec!(180);
        input.cash_on_hand = dec!(0);
        input.administrative_costs = dec!(0);
        input.dip_facility = Some(DipFacility {
            amount: dec!(50),
            priming: true,
            roll_up_amount: Decimal::ZERO,
        });
        // distributable = 180. DIP = 50 => 130 remaining.

        let result = analyze_recovery(&input).unwrap();
        let out = &result.result;

        let dip = out
            .claim_recoveries
            .iter()
            .find(|r| r.name == "DIP Facility")
            .unwrap();
        assert_eq!(dip.recovery_amount, dec!(50));
        assert!(!dip.is_impaired);

        let first_lien = out
            .claim_recoveries
            .iter()
            .find(|r| r.name == "1st Lien")
            .unwrap();
        assert_eq!(first_lien.recovery_amount, dec!(130));
        assert!(first_lien.is_impaired);

        let equity = out
            .claim_recoveries
            .iter()
            .find(|r| r.name == "Equity")
            .unwrap();
        assert_eq!(equity.recovery_amount, Decimal::ZERO);
    }

    #[test]
    fn test_collateral_deficiency() {
        // Secured claim with collateral < claim => deficiency treated as senior unsecured
        let claims = vec![
            secured_claim(
                "1st Lien",
                dec!(300),
                ClaimPriority::SecuredFirst,
                dec!(200),
            ),
            simple_claim("Senior Notes", dec!(100), ClaimPriority::Senior),
            simple_claim("Equity", dec!(50), ClaimPriority::Equity),
        ];
        let mut input = base_input(claims);
        input.enterprise_value = dec!(350);
        input.cash_on_hand = dec!(0);
        input.administrative_costs = dec!(0);
        // distributable = 350

        let result = analyze_recovery(&input).unwrap();
        let out = &result.result;

        // 1st Lien: 300 claim, 200 collateral => secured portion = 200.
        // Paid 200 from distributable. Deficiency = 100.
        let first_lien = out
            .claim_recoveries
            .iter()
            .find(|r| r.name == "1st Lien")
            .unwrap();
        assert_eq!(first_lien.recovery_amount, dec!(200));
        assert!(first_lien.is_impaired); // Only 200/300

        // Remaining = 350 - 200 = 150 for Senior class (100 notes + 100 deficiency = 200).
        // Pro-rata: 150/200 = 0.75
        let senior = out
            .claim_recoveries
            .iter()
            .find(|r| r.name == "Senior Notes")
            .unwrap();
        assert_eq!(senior.recovery_amount, dec!(75)); // 100 * 0.75
        assert!(senior.is_impaired);

        // Deficiency claim recovery
        let deficiency = out
            .claim_recoveries
            .iter()
            .find(|r| r.name == "1st Lien (deficiency)")
            .unwrap();
        assert_eq!(deficiency.recovery_amount, dec!(75)); // 100 * 0.75

        // Equity gets zero
        let equity = out
            .claim_recoveries
            .iter()
            .find(|r| r.name == "Equity")
            .unwrap();
        assert_eq!(equity.recovery_amount, Decimal::ZERO);
    }

    #[test]
    fn test_accrued_interest() {
        // Claim with 6 months of unpaid interest at 10% annual
        let claims = vec![
            Claim {
                name: "Senior Notes".into(),
                amount: dec!(1000),
                priority: ClaimPriority::Senior,
                is_secured: false,
                collateral_value: None,
                interest_rate: Some(dec!(0.10)),
                accrued_months: Some(6),
            },
            simple_claim("Equity", dec!(200), ClaimPriority::Equity),
        ];
        let mut input = base_input(claims);
        input.enterprise_value = dec!(1100);
        input.cash_on_hand = dec!(0);
        input.administrative_costs = dec!(0);
        // distributable = 1100

        let result = analyze_recovery(&input).unwrap();
        let out = &result.result;

        let senior = &out.claim_recoveries[0];
        // accrued = 1000 * 0.10 * 6/12 = 50
        assert_eq!(senior.accrued_interest, dec!(50));
        assert_eq!(senior.total_claim, dec!(1050));
        assert_eq!(senior.recovery_amount, dec!(1050));
        assert!(!senior.is_impaired);

        // Equity: 1100 - 1050 = 50 remaining for 200 claim
        let equity = &out.claim_recoveries[1];
        assert_eq!(equity.recovery_amount, dec!(50));
        assert!(equity.is_impaired);
    }

    #[test]
    fn test_going_concern_vs_liquidation_both() {
        let claims = vec![
            simple_claim("Senior", dec!(400), ClaimPriority::Senior),
            simple_claim("Equity", dec!(200), ClaimPriority::Equity),
        ];
        let mut input = base_input(claims);
        input.enterprise_value = dec!(600);
        input.liquidation_value = dec!(300);
        input.cash_on_hand = dec!(0);
        input.administrative_costs = dec!(0);
        input.valuation_type = ValuationType::Both;
        // GC distributable = 600, Liq distributable = 300

        let result = analyze_recovery(&input).unwrap();
        let out = &result.result;

        // Primary (GC): Senior fully recovered, Equity partial
        assert_eq!(out.total_distributable, dec!(600));
        let senior = &out.claim_recoveries[0];
        assert_eq!(senior.recovery_amount, dec!(400));
        let equity = &out.claim_recoveries[1];
        assert_eq!(equity.recovery_amount, dec!(200));
        assert!(!equity.is_impaired);

        // GC premium = (600 - 300) / 300 = 1.0 (100% premium)
        assert_eq!(out.going_concern_premium, Some(Decimal::ONE));

        // Liquidation analysis
        let liq = out.liquidation_analysis.as_ref().unwrap();
        assert_eq!(liq.liquidation_distributable, dec!(300));
        let liq_senior = &liq.claim_recoveries[0];
        assert_eq!(liq_senior.recovery_amount, dec!(300));
        assert!(liq_senior.is_impaired); // 300 / 400
        let liq_equity = &liq.claim_recoveries[1];
        assert_eq!(liq_equity.recovery_amount, Decimal::ZERO);
        assert_eq!(liq.shortfall, dec!(300)); // 600 - 300
    }

    #[test]
    fn test_pro_rata_within_class() {
        // Two claims at same priority, insufficient funds
        let claims = vec![
            simple_claim("Senior A", dec!(200), ClaimPriority::Senior),
            simple_claim("Senior B", dec!(300), ClaimPriority::Senior),
        ];
        let mut input = base_input(claims);
        input.enterprise_value = dec!(250);
        input.cash_on_hand = dec!(0);
        input.administrative_costs = dec!(0);
        // distributable = 250 for 500 total => 50% pro-rata

        let result = analyze_recovery(&input).unwrap();
        let out = &result.result;

        let a = out
            .claim_recoveries
            .iter()
            .find(|r| r.name == "Senior A")
            .unwrap();
        assert_eq!(a.recovery_amount, dec!(100)); // 200 * 0.5
        assert_eq!(a.recovery_rate, dec!(0.5));
        assert_eq!(a.recovery_cents_on_dollar, dec!(50));
        assert!(a.is_impaired);

        let b = out
            .claim_recoveries
            .iter()
            .find(|r| r.name == "Senior B")
            .unwrap();
        assert_eq!(b.recovery_amount, dec!(150)); // 300 * 0.5
        assert_eq!(b.recovery_rate, dec!(0.5));
        assert!(b.is_impaired);
    }

    #[test]
    fn test_full_recovery_all_claims() {
        let claims = vec![
            simple_claim("Senior", dec!(100), ClaimPriority::Senior),
            simple_claim("Sub", dec!(50), ClaimPriority::Subordinated),
            simple_claim("Equity", dec!(50), ClaimPriority::Equity),
        ];
        let mut input = base_input(claims);
        input.enterprise_value = dec!(500);
        input.cash_on_hand = dec!(0);
        input.administrative_costs = dec!(0);
        // distributable = 500 >= 200 total claims

        let result = analyze_recovery(&input).unwrap();
        let out = &result.result;

        for rec in &out.claim_recoveries {
            assert_eq!(rec.recovery_rate, Decimal::ONE);
            assert!(!rec.is_impaired);
        }
        assert!(out.fulcrum_security.is_none());
        assert_eq!(out.shortfall, Decimal::ZERO);
    }

    #[test]
    fn test_administrative_costs_deducted() {
        let claims = vec![simple_claim("Senior", dec!(100), ClaimPriority::Senior)];
        let mut input = base_input(claims);
        input.enterprise_value = dec!(150);
        input.cash_on_hand = dec!(0);
        input.administrative_costs = dec!(60);
        // distributable = 150 - 60 = 90

        let result = analyze_recovery(&input).unwrap();
        let out = &result.result;

        assert_eq!(out.total_distributable, dec!(90));
        assert_eq!(out.claim_recoveries[0].recovery_amount, dec!(90));
        assert!(out.claim_recoveries[0].is_impaired);
    }

    #[test]
    fn test_admin_costs_exceed_ev_warning() {
        let claims = vec![simple_claim("Senior", dec!(100), ClaimPriority::Senior)];
        let mut input = base_input(claims);
        input.enterprise_value = dec!(100);
        input.cash_on_hand = dec!(0);
        input.administrative_costs = dec!(15); // 15% of EV
                                               // distributable = max(100 - 15, 0) = 85

        let result = analyze_recovery(&input).unwrap();
        assert!(result
            .warnings
            .iter()
            .any(|w| w.contains("Administrative costs")));
    }

    #[test]
    fn test_cash_on_hand_increases_distributable() {
        let claims = vec![simple_claim("Senior", dec!(200), ClaimPriority::Senior)];
        let mut input = base_input(claims);
        input.enterprise_value = dec!(100);
        input.cash_on_hand = dec!(80);
        input.administrative_costs = dec!(0);
        // distributable = 100 + 80 = 180

        let result = analyze_recovery(&input).unwrap();
        let out = &result.result;
        assert_eq!(out.total_distributable, dec!(180));
        assert_eq!(out.claim_recoveries[0].recovery_amount, dec!(180));
    }

    #[test]
    fn test_liquidation_only_valuation_type() {
        let claims = vec![
            simple_claim("Senior", dec!(200), ClaimPriority::Senior),
            simple_claim("Equity", dec!(100), ClaimPriority::Equity),
        ];
        let mut input = base_input(claims);
        input.enterprise_value = dec!(500);
        input.liquidation_value = dec!(250);
        input.cash_on_hand = dec!(0);
        input.administrative_costs = dec!(0);
        input.valuation_type = ValuationType::Liquidation;

        let result = analyze_recovery(&input).unwrap();
        let out = &result.result;

        // Primary outputs based on liquidation value
        assert_eq!(out.total_distributable, dec!(250));

        let senior = &out.claim_recoveries[0];
        assert_eq!(senior.recovery_amount, dec!(200));
        assert!(!senior.is_impaired);

        let equity = &out.claim_recoveries[1];
        assert_eq!(equity.recovery_amount, dec!(50));
        assert!(equity.is_impaired);
    }

    #[test]
    fn test_negative_going_concern_premium_warning() {
        let claims = vec![simple_claim("Senior", dec!(100), ClaimPriority::Senior)];
        let mut input = base_input(claims);
        input.enterprise_value = dec!(200);
        input.liquidation_value = dec!(300); // Liq > GC (unusual)

        let result = analyze_recovery(&input).unwrap();
        assert!(result
            .warnings
            .iter()
            .any(|w| w.contains("going-concern premium is negative")));
    }

    #[test]
    fn test_going_concern_premium_calculation() {
        let claims = vec![simple_claim("Senior", dec!(100), ClaimPriority::Senior)];
        let mut input = base_input(claims);
        input.enterprise_value = dec!(400);
        input.liquidation_value = dec!(200);

        let result = analyze_recovery(&input).unwrap();
        // Premium = (400 - 200) / 200 = 1.0
        assert_eq!(result.result.going_concern_premium, Some(Decimal::ONE));
    }

    #[test]
    fn test_going_concern_premium_zero_liquidation() {
        let claims = vec![simple_claim("Senior", dec!(100), ClaimPriority::Senior)];
        let mut input = base_input(claims);
        input.enterprise_value = dec!(400);
        input.liquidation_value = Decimal::ZERO;

        let result = analyze_recovery(&input).unwrap();
        // Cannot compute premium with zero denominator
        assert_eq!(result.result.going_concern_premium, None);
    }

    #[test]
    fn test_priority_ordering() {
        // Claims in mixed order -- waterfall respects priority enum ordering
        let claims = vec![
            simple_claim("Equity", dec!(100), ClaimPriority::Equity),
            simple_claim("Senior", dec!(200), ClaimPriority::Senior),
            simple_claim("Priority", dec!(50), ClaimPriority::Priority),
        ];
        let mut input = base_input(claims);
        input.enterprise_value = dec!(300);
        input.cash_on_hand = dec!(0);
        input.administrative_costs = dec!(0);
        // distributable = 300

        let result = analyze_recovery(&input).unwrap();
        let out = &result.result;

        // Priority claims paid first: 50 => 250 remaining
        let priority_claim = out
            .claim_recoveries
            .iter()
            .find(|r| r.name == "Priority")
            .unwrap();
        assert_eq!(priority_claim.recovery_amount, dec!(50));
        assert!(!priority_claim.is_impaired);

        // Senior: 200 => 50 remaining
        let senior = out
            .claim_recoveries
            .iter()
            .find(|r| r.name == "Senior")
            .unwrap();
        assert_eq!(senior.recovery_amount, dec!(200));
        assert!(!senior.is_impaired);

        // Equity: 50 remaining for 100 claim
        let equity = out
            .claim_recoveries
            .iter()
            .find(|r| r.name == "Equity")
            .unwrap();
        assert_eq!(equity.recovery_amount, dec!(50));
        assert!(equity.is_impaired);
    }

    #[test]
    fn test_dip_with_no_remaining_for_others() {
        // DIP consumes all distributable
        let claims = vec![
            simple_claim("Senior", dec!(200), ClaimPriority::Senior),
            simple_claim("Equity", dec!(100), ClaimPriority::Equity),
        ];
        let mut input = base_input(claims);
        input.enterprise_value = dec!(40);
        input.cash_on_hand = dec!(10);
        input.administrative_costs = dec!(0);
        input.dip_facility = Some(DipFacility {
            amount: dec!(50),
            priming: true,
            roll_up_amount: Decimal::ZERO,
        });
        // distributable = 50. DIP takes 50 => 0 remaining.

        let result = analyze_recovery(&input).unwrap();
        let out = &result.result;

        let dip = out
            .claim_recoveries
            .iter()
            .find(|r| r.name == "DIP Facility")
            .unwrap();
        assert_eq!(dip.recovery_amount, dec!(50));
        assert!(!dip.is_impaired);

        for rec in out
            .claim_recoveries
            .iter()
            .filter(|r| r.name != "DIP Facility")
        {
            assert_eq!(rec.recovery_amount, Decimal::ZERO);
            assert!(rec.is_impaired);
        }
    }

    #[test]
    fn test_recovery_cents_on_dollar() {
        let claims = vec![simple_claim("Senior", dec!(200), ClaimPriority::Senior)];
        let mut input = base_input(claims);
        input.enterprise_value = dec!(100);
        input.cash_on_hand = dec!(0);
        input.administrative_costs = dec!(0);

        let result = analyze_recovery(&input).unwrap();
        let rec = &result.result.claim_recoveries[0];
        // 100/200 = 0.5 => 50 cents on dollar
        assert_eq!(rec.recovery_rate, dec!(0.5));
        assert_eq!(rec.recovery_cents_on_dollar, dec!(50));
    }

    // --- Validation tests ----------------------------------------------------

    #[test]
    fn test_validation_negative_ev() {
        let mut input = base_input(vec![simple_claim("S", dec!(100), ClaimPriority::Senior)]);
        input.enterprise_value = dec!(-10);
        let err = analyze_recovery(&input).unwrap_err();
        match err {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "enterprise_value");
            }
            other => panic!("Expected InvalidInput, got: {other:?}"),
        }
    }

    #[test]
    fn test_validation_negative_liquidation() {
        let mut input = base_input(vec![simple_claim("S", dec!(100), ClaimPriority::Senior)]);
        input.liquidation_value = dec!(-5);
        let err = analyze_recovery(&input).unwrap_err();
        match err {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "liquidation_value");
            }
            other => panic!("Expected InvalidInput, got: {other:?}"),
        }
    }

    #[test]
    fn test_validation_empty_claims() {
        let mut input = base_input(vec![]);
        input.claims = vec![];
        let err = analyze_recovery(&input).unwrap_err();
        match err {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "claims");
            }
            other => panic!("Expected InvalidInput, got: {other:?}"),
        }
    }

    #[test]
    fn test_validation_negative_admin_costs() {
        let mut input = base_input(vec![simple_claim("S", dec!(100), ClaimPriority::Senior)]);
        input.administrative_costs = dec!(-1);
        let err = analyze_recovery(&input).unwrap_err();
        match err {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "administrative_costs");
            }
            other => panic!("Expected InvalidInput, got: {other:?}"),
        }
    }

    #[test]
    fn test_validation_negative_claim_amount() {
        let input = base_input(vec![simple_claim("Bad", dec!(-50), ClaimPriority::Senior)]);
        let err = analyze_recovery(&input).unwrap_err();
        match err {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert!(field.contains("Bad"));
            }
            other => panic!("Expected InvalidInput, got: {other:?}"),
        }
    }

    #[test]
    fn test_validation_negative_cash() {
        let mut input = base_input(vec![simple_claim("S", dec!(100), ClaimPriority::Senior)]);
        input.cash_on_hand = dec!(-10);
        let err = analyze_recovery(&input).unwrap_err();
        match err {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "cash_on_hand");
            }
            other => panic!("Expected InvalidInput, got: {other:?}"),
        }
    }

    #[test]
    fn test_metadata_populated() {
        let input = base_input(vec![simple_claim("S", dec!(100), ClaimPriority::Senior)]);
        let result = analyze_recovery(&input).unwrap();
        assert!(!result.methodology.is_empty());
        assert!(result.methodology.contains("Absolute Priority Rule"));
        assert_eq!(result.metadata.precision, "rust_decimal_128bit");
    }

    #[test]
    fn test_multiple_priority_classes_sequential() {
        // Priority > Senior > Subordinated > Mezzanine > Equity
        let claims = vec![
            simple_claim("Wages", dec!(50), ClaimPriority::Priority),
            simple_claim("Senior", dec!(200), ClaimPriority::Senior),
            simple_claim("Sub", dec!(100), ClaimPriority::Subordinated),
            simple_claim("Mezz", dec!(80), ClaimPriority::Mezzanine),
            simple_claim("Equity", dec!(150), ClaimPriority::Equity),
        ];
        let mut input = base_input(claims);
        input.enterprise_value = dec!(400);
        input.cash_on_hand = dec!(0);
        input.administrative_costs = dec!(0);
        // distributable = 400

        let result = analyze_recovery(&input).unwrap();
        let out = &result.result;

        // Wages: 50 (full), Senior: 200 (full), Sub: 100 (full),
        // Mezz: 50/80 (partial), Equity: 0
        let wages = out
            .claim_recoveries
            .iter()
            .find(|r| r.name == "Wages")
            .unwrap();
        assert!(!wages.is_impaired);
        let senior = out
            .claim_recoveries
            .iter()
            .find(|r| r.name == "Senior")
            .unwrap();
        assert!(!senior.is_impaired);
        let sub = out
            .claim_recoveries
            .iter()
            .find(|r| r.name == "Sub")
            .unwrap();
        assert!(!sub.is_impaired);
        let mezz = out
            .claim_recoveries
            .iter()
            .find(|r| r.name == "Mezz")
            .unwrap();
        assert_eq!(mezz.recovery_amount, dec!(50));
        assert!(mezz.is_impaired);
        let equity = out
            .claim_recoveries
            .iter()
            .find(|r| r.name == "Equity")
            .unwrap();
        assert_eq!(equity.recovery_amount, Decimal::ZERO);
        assert!(equity.is_impaired);
        assert_eq!(out.fulcrum_security, Some("Mezz".into()));
    }

    #[test]
    fn test_senior_subordinated_class() {
        let claims = vec![
            simple_claim("Senior", dec!(100), ClaimPriority::Senior),
            simple_claim("Senior Sub", dec!(100), ClaimPriority::SeniorSubordinated),
            simple_claim("Equity", dec!(50), ClaimPriority::Equity),
        ];
        let mut input = base_input(claims);
        input.enterprise_value = dec!(180);
        input.cash_on_hand = dec!(0);
        input.administrative_costs = dec!(0);
        // distributable = 180

        let result = analyze_recovery(&input).unwrap();
        let out = &result.result;

        let senior = out
            .claim_recoveries
            .iter()
            .find(|r| r.name == "Senior")
            .unwrap();
        assert_eq!(senior.recovery_amount, dec!(100));
        assert!(!senior.is_impaired);

        let sub = out
            .claim_recoveries
            .iter()
            .find(|r| r.name == "Senior Sub")
            .unwrap();
        assert_eq!(sub.recovery_amount, dec!(80));
        assert!(sub.is_impaired);

        assert_eq!(out.fulcrum_security, Some("Senior Sub".into()));
    }
}
