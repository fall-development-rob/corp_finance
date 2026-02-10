use rust_decimal::Decimal;
use rust_decimal::MathematicalOps;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::error::CorpFinanceError;
use crate::types::{with_metadata, ComputationOutput, Money, Rate};
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Input types
// ---------------------------------------------------------------------------

/// Cash flow from the collateral pool for a single period.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeriodCashflow {
    /// Period number (1-indexed)
    pub period: u32,
    /// Interest collected from collateral in this period
    pub interest: Money,
    /// Principal collected from collateral in this period
    pub principal: Money,
    /// Losses (defaults) in this period
    pub losses: Money,
}

/// Specification for a single tranche in the structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrancheSpec {
    /// Human-readable name (e.g. "AAA", "BBB", "Equity")
    pub name: String,
    /// Par/face amount of the tranche
    pub balance: Money,
    /// Annual coupon rate (decimal, e.g. 0.05 = 5%)
    pub coupon_rate: Rate,
    /// Seniority: 1 = most senior, higher = more subordinated
    pub seniority: u32,
    /// Whether the coupon is fixed-rate (true) or floating (false)
    pub is_fixed_rate: bool,
    /// Number of coupon payments per year (4 = quarterly, 12 = monthly)
    pub payment_frequency: u32,
}

/// Input for CDO/CLO tranching analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranchingInput {
    /// Deal name / identifier
    pub deal_name: String,
    /// Total collateral pool balance
    pub collateral_balance: Money,
    /// Period-by-period cash flows from the collateral
    pub collateral_cashflows: Vec<PeriodCashflow>,
    /// Tranches ordered by seniority (will be sorted internally)
    pub tranches: Vec<TrancheSpec>,
    /// Initial cash reserve account balance
    pub reserve_account: Money,
    /// Overcollateralisation trigger ratio (e.g. 1.20 = 120%)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oc_trigger: Option<Decimal>,
    /// Interest coverage trigger ratio (e.g. 1.05)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ic_trigger: Option<Decimal>,
    /// Months during which principal can be reinvested (CLO feature)
    pub reinvestment_period_months: u32,
}

// ---------------------------------------------------------------------------
// Output types
// ---------------------------------------------------------------------------

/// Result for a single tranche across the life of the deal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrancheResult {
    pub name: String,
    pub original_balance: Money,
    pub ending_balance: Money,
    pub total_interest_received: Money,
    pub total_principal_received: Money,
    pub loss_allocated: Money,
    /// IRR of tranche cash flows
    pub yield_to_maturity: Rate,
    /// Weighted average life in years
    pub weighted_average_life: Decimal,
    /// Subordination percentage (junior tranches / total deal)
    pub credit_enhancement_pct: Rate,
}

/// Subordination level for a single tranche.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubordinationLevel {
    pub tranche_name: String,
    /// Sum of balances junior to this tranche / total tranche balance
    pub subordination_pct: Rate,
}

/// Credit enhancement metrics for the deal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreditEnhancement {
    /// Subordination for each tranche
    pub subordination: Vec<SubordinationLevel>,
    /// Initial OC ratio: collateral / total tranches
    pub overcollateralisation_initial: Decimal,
    /// Final OC ratio after waterfall
    pub overcollateralisation_final: Decimal,
    /// Excess spread: WAC of collateral - weighted avg tranche coupon
    pub excess_spread: Rate,
    /// Reserve / collateral balance
    pub reserve_account_pct: Rate,
}

/// Payment detail for a single tranche in a single period.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranchePayment {
    pub tranche_name: String,
    pub interest_paid: Money,
    pub principal_paid: Money,
    pub interest_shortfall: Money,
}

/// Waterfall detail for a single period.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaterfallPeriod {
    pub period: u32,
    pub available_interest: Money,
    pub available_principal: Money,
    pub losses: Money,
    pub tranche_payments: Vec<TranchePayment>,
    pub reserve_balance: Money,
    pub oc_test_result: Option<bool>,
    pub ic_test_result: Option<bool>,
}

/// Summary metrics for the entire deal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DealSummary {
    pub total_collateral: Money,
    pub total_tranches: Money,
    pub excess_collateral: Money,
    pub weighted_avg_tranche_cost: Rate,
    pub total_losses: Money,
    pub total_interest_distributed: Money,
    pub total_principal_distributed: Money,
}

/// Full output of the tranching analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranchingOutput {
    pub tranche_results: Vec<TrancheResult>,
    pub credit_enhancement: CreditEnhancement,
    pub waterfall_periods: Vec<WaterfallPeriod>,
    pub deal_summary: DealSummary,
}

// ---------------------------------------------------------------------------
// Internal state for waterfall simulation
// ---------------------------------------------------------------------------

/// Internal mutable state for a tranche during waterfall execution.
#[derive(Debug, Clone)]
struct TrancheState {
    name: String,
    seniority: u32,
    original_balance: Money,
    current_balance: Money,
    coupon_rate: Rate,
    payment_frequency: u32,
    total_interest_received: Money,
    total_principal_received: Money,
    loss_allocated: Money,
    /// Per-period cash flows for IRR: negative at t=0 (purchase), positive each period
    cash_flows: Vec<Money>,
    /// For WAL: sum of (period * principal_paid)
    wal_numerator: Decimal,
}

// ---------------------------------------------------------------------------
// Main analysis function
// ---------------------------------------------------------------------------

/// Analyse a CDO/CLO tranching structure with waterfall distribution.
///
/// Runs the full sequential/turbo waterfall for each period of collateral
/// cash flows, applying OC/IC tests, loss allocation, and reinvestment logic.
pub fn analyze_tranching(
    input: &TranchingInput,
) -> CorpFinanceResult<ComputationOutput<TranchingOutput>> {
    let start = Instant::now();
    let mut warnings: Vec<String> = Vec::new();

    // --- Validate inputs ---
    validate_input(input)?;

    // --- Sort tranches by seniority (ascending = most senior first) ---
    let mut sorted_specs: Vec<&TrancheSpec> = input.tranches.iter().collect();
    sorted_specs.sort_by_key(|t| t.seniority);

    let total_tranche_balance: Money = sorted_specs.iter().map(|t| t.balance).sum();

    // --- Build initial tranche state ---
    let mut tranche_states: Vec<TrancheState> = sorted_specs
        .iter()
        .map(|spec| {
            // First cash flow is the negative purchase price (par)
            TrancheState {
                name: spec.name.clone(),
                seniority: spec.seniority,
                original_balance: spec.balance,
                current_balance: spec.balance,
                coupon_rate: spec.coupon_rate,
                payment_frequency: spec.payment_frequency,
                total_interest_received: Decimal::ZERO,
                total_principal_received: Decimal::ZERO,
                loss_allocated: Decimal::ZERO,
                cash_flows: vec![-spec.balance],
                wal_numerator: Decimal::ZERO,
            }
        })
        .collect();

    // --- Compute initial credit enhancement ---
    let subordination = compute_subordination(&sorted_specs, total_tranche_balance);

    let oc_initial = if total_tranche_balance.is_zero() {
        Decimal::ZERO
    } else {
        input.collateral_balance / total_tranche_balance
    };

    let reserve_pct = if input.collateral_balance.is_zero() {
        Decimal::ZERO
    } else {
        input.reserve_account / input.collateral_balance
    };

    // --- Determine periods_per_year from the most senior tranche ---
    let periods_per_year = if sorted_specs.is_empty() {
        Decimal::from(12u32)
    } else {
        let freq = sorted_specs[0].payment_frequency;
        if freq == 0 {
            Decimal::from(12u32)
        } else {
            Decimal::from(freq)
        }
    };

    // --- Waterfall execution ---
    let mut waterfall_periods: Vec<WaterfallPeriod> = Vec::new();
    let mut reserve_balance = input.reserve_account;
    let mut collateral_balance = input.collateral_balance;
    let mut cumulative_losses = Decimal::ZERO;

    for cf in &input.collateral_cashflows {
        let period = cf.period;
        let period_losses = cf.losses;

        // 1. Allocate losses bottom-up
        cumulative_losses += period_losses;
        allocate_losses(&mut tranche_states, period_losses);

        // Update collateral balance
        collateral_balance = collateral_balance - cf.principal - period_losses;

        // 2. Available cash
        let mut available_interest = cf.interest;
        let available_principal = cf.principal;

        // Draw from reserve if interest is insufficient to cover senior coupon
        let total_interest_due: Money = tranche_states
            .iter()
            .filter(|t| t.current_balance > Decimal::ZERO)
            .map(compute_period_coupon)
            .sum();

        if available_interest < total_interest_due && reserve_balance > Decimal::ZERO {
            let shortfall = total_interest_due - available_interest;
            let draw = shortfall.min(reserve_balance);
            available_interest += draw;
            reserve_balance -= draw;
        }

        // 3. OC and IC tests
        let oc_test = input.oc_trigger.map(|trigger| {
            let senior_balance = tranche_states
                .first()
                .map(|t| t.current_balance)
                .unwrap_or(Decimal::ZERO);
            if senior_balance.is_zero() {
                true
            } else {
                let _oc_ratio = (collateral_balance - cumulative_losses
                    + collateral_balance.min(Decimal::ZERO).abs())
                .max(Decimal::ZERO);
                // Simpler: use remaining collateral / senior outstanding
                let effective_collateral = collateral_balance.max(Decimal::ZERO) + reserve_balance;
                let ratio = effective_collateral / senior_balance;
                ratio >= trigger
            }
        });

        let ic_test = input.ic_trigger.map(|trigger| {
            let senior_interest_due = tranche_states
                .first()
                .map(|t| {
                    if t.current_balance > Decimal::ZERO {
                        compute_period_coupon(t)
                    } else {
                        Decimal::ZERO
                    }
                })
                .unwrap_or(Decimal::ZERO);
            if senior_interest_due.is_zero() {
                true
            } else {
                cf.interest / senior_interest_due >= trigger
            }
        });

        let oc_passed = oc_test.unwrap_or(true);
        let ic_passed = ic_test.unwrap_or(true);

        // 4. Interest waterfall (pay in seniority order)
        let mut tranche_payments: Vec<TranchePayment> = Vec::new();
        let mut remaining_interest = available_interest;

        for (idx, state) in tranche_states.iter_mut().enumerate() {
            if state.current_balance <= Decimal::ZERO {
                tranche_payments.push(TranchePayment {
                    tranche_name: state.name.clone(),
                    interest_paid: Decimal::ZERO,
                    principal_paid: Decimal::ZERO,
                    interest_shortfall: Decimal::ZERO,
                });
                continue;
            }

            let interest_due = compute_period_coupon(state);
            let interest_paid = interest_due.min(remaining_interest);
            let interest_shortfall = interest_due - interest_paid;
            remaining_interest -= interest_paid;

            state.total_interest_received += interest_paid;

            tranche_payments.push(TranchePayment {
                tranche_name: state.name.clone(),
                interest_paid,
                principal_paid: Decimal::ZERO, // filled in principal waterfall
                interest_shortfall,
            });

            // If OC or IC test failed, divert remaining junior interest
            // to senior principal acceleration
            if (!oc_passed || !ic_passed) && idx == 0 {
                // After paying senior interest, any interest that would go
                // to junior tranches gets redirected to senior principal
                // We handle this after the interest loop
            }
        }

        // If OC/IC test failed, remaining interest (that would go to juniors)
        // is redirected to senior principal paydown
        let diverted_interest = if !oc_passed || !ic_passed {
            remaining_interest
        } else {
            Decimal::ZERO
        };

        // 5. Principal waterfall
        let in_reinvestment =
            period <= input.reinvestment_period_months && input.reinvestment_period_months > 0;

        let mut remaining_principal = if in_reinvestment {
            // During reinvestment period, principal is reinvested (returned to pool).
            // Only diverted interest (from test failures) is used for principal paydown.
            collateral_balance += available_principal;
            diverted_interest
        } else {
            available_principal + diverted_interest
        };

        // Sequential pay: most senior first
        for (idx, state) in tranche_states.iter_mut().enumerate() {
            if state.current_balance <= Decimal::ZERO || remaining_principal <= Decimal::ZERO {
                continue;
            }

            // If OC test failed and this is not the most senior tranche,
            // skip principal (turbo amortisation directs all to senior)
            if (!oc_passed || !ic_passed) && idx > 0 {
                continue;
            }

            let principal_paid = state.current_balance.min(remaining_principal);
            state.current_balance -= principal_paid;
            state.total_principal_received += principal_paid;
            remaining_principal -= principal_paid;

            // WAL numerator: period * principal_paid
            state.wal_numerator += Decimal::from(period) * principal_paid;

            // Update the payment record
            if let Some(payment) = tranche_payments.get_mut(idx) {
                payment.principal_paid = principal_paid;
            }
        }

        // If OC/IC passed and not in reinvestment, do sequential for remaining tranches
        if oc_passed && ic_passed && !in_reinvestment && remaining_principal > Decimal::ZERO {
            for (idx, state) in tranche_states.iter_mut().enumerate() {
                if state.current_balance <= Decimal::ZERO || remaining_principal <= Decimal::ZERO {
                    continue;
                }
                let principal_paid = state.current_balance.min(remaining_principal);
                state.current_balance -= principal_paid;
                state.total_principal_received += principal_paid;
                remaining_principal -= principal_paid;
                state.wal_numerator += Decimal::from(period) * principal_paid;

                if let Some(payment) = tranche_payments.get_mut(idx) {
                    payment.principal_paid += principal_paid;
                }
            }
        }

        // 6. Replenish reserve from any remaining interest
        if oc_passed && ic_passed && remaining_interest > Decimal::ZERO {
            let max_reserve = input.reserve_account; // cap at initial level
            if reserve_balance < max_reserve {
                let top_up = (max_reserve - reserve_balance).min(remaining_interest);
                reserve_balance += top_up;
                remaining_interest -= top_up;
            }
        }

        // 7. Record per-period cash flows for each tranche (for IRR calc)
        for (idx, state) in tranche_states.iter_mut().enumerate() {
            let payment = &tranche_payments[idx];
            let total_cf = payment.interest_paid + payment.principal_paid;
            state.cash_flows.push(total_cf);
        }

        waterfall_periods.push(WaterfallPeriod {
            period,
            available_interest: cf.interest,
            available_principal: cf.principal,
            losses: period_losses,
            tranche_payments,
            reserve_balance,
            oc_test_result: oc_test,
            ic_test_result: ic_test,
        });
    }

    // --- Compute tranche results ---
    let mut tranche_results: Vec<TrancheResult> = Vec::new();
    for (idx, state) in tranche_states.iter().enumerate() {
        // Credit enhancement: sum of junior tranche original balances / total
        let junior_balance: Money = tranche_states
            .iter()
            .filter(|t| t.seniority > state.seniority)
            .map(|t| t.original_balance)
            .sum();
        let ce_pct = if total_tranche_balance.is_zero() {
            Decimal::ZERO
        } else {
            junior_balance / total_tranche_balance
        };

        // WAL = sum(period_i * principal_i) / (total_principal * periods_per_year)
        // This converts period-denominated time to years.
        let total_principal = state.total_principal_received;
        let tranche_periods_per_year = Decimal::from(
            sorted_specs
                .get(idx)
                .map(|s| s.payment_frequency)
                .unwrap_or(4),
        );
        let wal = if total_principal.is_zero() {
            Decimal::ZERO
        } else {
            state.wal_numerator / (total_principal * tranche_periods_per_year)
        };

        // YTM via Newton-Raphson on tranche cash flows
        let ytm = compute_tranche_irr(&state.cash_flows, periods_per_year, &mut warnings);

        tranche_results.push(TrancheResult {
            name: state.name.clone(),
            original_balance: state.original_balance,
            ending_balance: state.current_balance,
            total_interest_received: state.total_interest_received,
            total_principal_received: state.total_principal_received,
            loss_allocated: state.loss_allocated,
            yield_to_maturity: ytm,
            weighted_average_life: wal,
            credit_enhancement_pct: ce_pct,
        });
    }

    // --- Deal summary ---
    let total_interest_distributed: Money = tranche_results
        .iter()
        .map(|t| t.total_interest_received)
        .sum();
    let total_principal_distributed: Money = tranche_results
        .iter()
        .map(|t| t.total_principal_received)
        .sum();
    let total_losses: Money = tranche_results.iter().map(|t| t.loss_allocated).sum();

    // Weighted average tranche cost
    let weighted_avg_cost = if total_tranche_balance.is_zero() {
        Decimal::ZERO
    } else {
        sorted_specs
            .iter()
            .map(|t| t.balance * t.coupon_rate)
            .sum::<Decimal>()
            / total_tranche_balance
    };

    // Compute excess spread: approx WAC of collateral interest vs tranche cost
    let total_collateral_interest: Money =
        input.collateral_cashflows.iter().map(|c| c.interest).sum();
    let _total_collateral_principal: Money =
        input.collateral_cashflows.iter().map(|c| c.principal).sum();
    let num_periods = Decimal::from(input.collateral_cashflows.len() as u32);
    let wac_estimate = if input.collateral_balance.is_zero() || num_periods.is_zero() {
        Decimal::ZERO
    } else {
        (total_collateral_interest / num_periods) / input.collateral_balance * periods_per_year
    };
    let excess_spread = wac_estimate - weighted_avg_cost;

    // Final OC ratio
    let final_tranche_balance: Money = tranche_states.iter().map(|t| t.current_balance).sum();
    let final_collateral = collateral_balance.max(Decimal::ZERO);
    let oc_final = if final_tranche_balance.is_zero() {
        if final_collateral > Decimal::ZERO {
            dec!(999.99) // fully paid, infinite OC
        } else {
            Decimal::ZERO
        }
    } else {
        final_collateral / final_tranche_balance
    };

    let credit_enhancement = CreditEnhancement {
        subordination,
        overcollateralisation_initial: oc_initial,
        overcollateralisation_final: oc_final,
        excess_spread,
        reserve_account_pct: reserve_pct,
    };

    let deal_summary = DealSummary {
        total_collateral: input.collateral_balance,
        total_tranches: total_tranche_balance,
        excess_collateral: input.collateral_balance - total_tranche_balance,
        weighted_avg_tranche_cost: weighted_avg_cost,
        total_losses,
        total_interest_distributed,
        total_principal_distributed,
    };

    let output = TranchingOutput {
        tranche_results,
        credit_enhancement,
        waterfall_periods,
        deal_summary,
    };

    let elapsed = start.elapsed().as_micros() as u64;
    Ok(with_metadata(
        "CDO/CLO Tranching: waterfall distribution with OC/IC tests",
        &serde_json::json!({
            "deal_name": input.deal_name,
            "collateral_balance": input.collateral_balance.to_string(),
            "num_tranches": input.tranches.len(),
            "num_periods": input.collateral_cashflows.len(),
            "reinvestment_months": input.reinvestment_period_months,
        }),
        warnings,
        elapsed,
        output,
    ))
}

// ---------------------------------------------------------------------------
// Helper functions
// ---------------------------------------------------------------------------

/// Validate the tranching input.
fn validate_input(input: &TranchingInput) -> CorpFinanceResult<()> {
    if input.tranches.is_empty() {
        return Err(CorpFinanceError::InvalidInput {
            field: "tranches".into(),
            reason: "At least one tranche is required".into(),
        });
    }

    if input.collateral_cashflows.is_empty() {
        return Err(CorpFinanceError::InvalidInput {
            field: "collateral_cashflows".into(),
            reason: "At least one period of cash flows is required".into(),
        });
    }

    if input.collateral_balance <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "collateral_balance".into(),
            reason: "Collateral balance must be positive".into(),
        });
    }

    let total_tranches: Money = input.tranches.iter().map(|t| t.balance).sum();
    if total_tranches > input.collateral_balance {
        return Err(CorpFinanceError::InvalidInput {
            field: "tranches".into(),
            reason: "Total tranche balance exceeds collateral balance".into(),
        });
    }

    for tranche in &input.tranches {
        if tranche.balance <= Decimal::ZERO {
            return Err(CorpFinanceError::InvalidInput {
                field: format!("tranche[{}].balance", tranche.name),
                reason: "Tranche balance must be positive".into(),
            });
        }
        if tranche.payment_frequency == 0 {
            return Err(CorpFinanceError::InvalidInput {
                field: format!("tranche[{}].payment_frequency", tranche.name),
                reason: "Payment frequency must be > 0".into(),
            });
        }
        if tranche.coupon_rate < Decimal::ZERO {
            return Err(CorpFinanceError::InvalidInput {
                field: format!("tranche[{}].coupon_rate", tranche.name),
                reason: "Coupon rate cannot be negative".into(),
            });
        }
    }

    Ok(())
}

/// Compute the period coupon for a tranche (annual rate / payment_frequency * balance).
fn compute_period_coupon(state: &TrancheState) -> Money {
    if state.payment_frequency == 0 || state.current_balance <= Decimal::ZERO {
        return Decimal::ZERO;
    }
    state.current_balance * state.coupon_rate / Decimal::from(state.payment_frequency)
}

/// Allocate losses bottom-up (most junior tranche absorbs first).
fn allocate_losses(tranche_states: &mut [TrancheState], mut losses: Money) {
    // Iterate from most junior (highest seniority number) to most senior
    for state in tranche_states.iter_mut().rev() {
        if losses <= Decimal::ZERO {
            break;
        }
        let absorbed = losses.min(state.current_balance);
        state.current_balance -= absorbed;
        state.loss_allocated += absorbed;
        losses -= absorbed;
    }
}

/// Compute subordination levels for each tranche.
fn compute_subordination(
    sorted_specs: &[&TrancheSpec],
    total_balance: Money,
) -> Vec<SubordinationLevel> {
    sorted_specs
        .iter()
        .map(|spec| {
            let junior_sum: Money = sorted_specs
                .iter()
                .filter(|t| t.seniority > spec.seniority)
                .map(|t| t.balance)
                .sum();
            let pct = if total_balance.is_zero() {
                Decimal::ZERO
            } else {
                junior_sum / total_balance
            };
            SubordinationLevel {
                tranche_name: spec.name.clone(),
                subordination_pct: pct,
            }
        })
        .collect()
}

/// Compute IRR for a tranche's cash flows using Newton-Raphson.
/// Cash flows are periodic (matching the tranche payment frequency).
/// Returns annualised rate.
fn compute_tranche_irr(
    cash_flows: &[Money],
    periods_per_year: Decimal,
    warnings: &mut Vec<String>,
) -> Rate {
    if cash_flows.len() < 2 {
        return Decimal::ZERO;
    }

    // Check if there are any positive flows
    let has_positive = cash_flows.iter().any(|cf| *cf > Decimal::ZERO);
    let has_negative = cash_flows.iter().any(|cf| *cf < Decimal::ZERO);
    if !has_positive || !has_negative {
        return Decimal::ZERO;
    }

    let max_iterations: u32 = 50;
    let threshold = dec!(0.0000001);
    let mut rate = dec!(0.05) / periods_per_year; // initial guess: 5% annualised

    for iteration in 0..max_iterations {
        let mut npv_val = Decimal::ZERO;
        let mut dnpv = Decimal::ZERO;
        let one_plus_r = Decimal::ONE + rate;

        if one_plus_r <= Decimal::ZERO {
            rate = dec!(0.01);
            continue;
        }

        // Use iterative multiplication for discount factors
        let mut discount = Decimal::ONE;
        for (t, cf) in cash_flows.iter().enumerate() {
            if t > 0 {
                discount *= one_plus_r;
            }
            if discount.is_zero() {
                break;
            }
            npv_val += cf / discount;
            if t > 0 {
                let t_dec = Decimal::from(t as i64);
                dnpv -= t_dec * cf / (discount * one_plus_r);
            }
        }

        if npv_val.abs() < threshold {
            // Annualise the periodic rate
            // (1 + periodic_rate)^periods_per_year - 1
            let annual = (Decimal::ONE + rate).powd(periods_per_year) - Decimal::ONE;
            return annual;
        }

        if dnpv.is_zero() {
            warnings.push(format!(
                "IRR derivative zero at iteration {iteration}, using last estimate"
            ));
            let annual = (Decimal::ONE + rate).powd(periods_per_year) - Decimal::ONE;
            return annual;
        }

        rate -= npv_val / dnpv;

        // Guard against divergence
        if rate < dec!(-0.99) {
            rate = dec!(-0.99);
        } else if rate > dec!(10.0) {
            rate = dec!(10.0);
        }
    }

    warnings.push("Tranche IRR did not converge within 50 iterations".into());
    (Decimal::ONE + rate).powd(periods_per_year) - Decimal::ONE
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    // -----------------------------------------------------------------------
    // Test helpers
    // -----------------------------------------------------------------------

    /// Create a simple set of collateral cash flows for testing.
    fn make_cashflows(
        num_periods: u32,
        interest_per_period: Decimal,
        principal_per_period: Decimal,
        loss_per_period: Decimal,
    ) -> Vec<PeriodCashflow> {
        (1..=num_periods)
            .map(|p| PeriodCashflow {
                period: p,
                interest: interest_per_period,
                principal: principal_per_period,
                losses: loss_per_period,
            })
            .collect()
    }

    /// Create a 2-tranche (senior/equity) test input.
    fn two_tranche_input() -> TranchingInput {
        TranchingInput {
            deal_name: "Test 2-Tranche".into(),
            collateral_balance: dec!(1000),
            collateral_cashflows: make_cashflows(4, dec!(25), dec!(200), dec!(0)),
            tranches: vec![
                TrancheSpec {
                    name: "Senior".into(),
                    balance: dec!(800),
                    coupon_rate: dec!(0.04),
                    seniority: 1,
                    is_fixed_rate: true,
                    payment_frequency: 4,
                },
                TrancheSpec {
                    name: "Equity".into(),
                    balance: dec!(150),
                    coupon_rate: dec!(0.10),
                    seniority: 2,
                    is_fixed_rate: true,
                    payment_frequency: 4,
                },
            ],
            reserve_account: dec!(10),
            oc_trigger: None,
            ic_trigger: None,
            reinvestment_period_months: 0,
        }
    }

    /// Create a 3-tranche (AAA/BBB/Equity) test input.
    fn three_tranche_input() -> TranchingInput {
        TranchingInput {
            deal_name: "Test 3-Tranche".into(),
            collateral_balance: dec!(1000),
            collateral_cashflows: make_cashflows(8, dec!(20), dec!(100), dec!(0)),
            tranches: vec![
                TrancheSpec {
                    name: "AAA".into(),
                    balance: dec!(600),
                    coupon_rate: dec!(0.03),
                    seniority: 1,
                    is_fixed_rate: true,
                    payment_frequency: 4,
                },
                TrancheSpec {
                    name: "BBB".into(),
                    balance: dec!(200),
                    coupon_rate: dec!(0.06),
                    seniority: 2,
                    is_fixed_rate: true,
                    payment_frequency: 4,
                },
                TrancheSpec {
                    name: "Equity".into(),
                    balance: dec!(100),
                    coupon_rate: dec!(0.12),
                    seniority: 3,
                    is_fixed_rate: true,
                    payment_frequency: 4,
                },
            ],
            reserve_account: dec!(20),
            oc_trigger: None,
            ic_trigger: None,
            reinvestment_period_months: 0,
        }
    }

    // -----------------------------------------------------------------------
    // Test 1: Simple 2-tranche interest allocation
    // -----------------------------------------------------------------------
    #[test]
    fn test_two_tranche_interest_allocation() {
        let input = two_tranche_input();
        let result = analyze_tranching(&input).unwrap();
        let output = &result.result;

        // Senior gets paid first in each period
        let senior = &output.tranche_results[0];
        let equity = &output.tranche_results[1];

        // Senior coupon per period: 800 * 0.04 / 4 = 8
        // Equity coupon per period: 150 * 0.10 / 4 = 3.75
        // Available interest per period: 25
        // Both should be paid in full (8 + 3.75 = 11.75 < 25)
        assert!(senior.total_interest_received > Decimal::ZERO);
        assert!(equity.total_interest_received > Decimal::ZERO);
    }

    // -----------------------------------------------------------------------
    // Test 2: Simple 2-tranche principal allocation (sequential pay)
    // -----------------------------------------------------------------------
    #[test]
    fn test_two_tranche_principal_sequential_pay() {
        let input = two_tranche_input();
        let result = analyze_tranching(&input).unwrap();
        let output = &result.result;

        // Total principal: 200 * 4 = 800
        // Senior has 800 balance, equity has 150
        // Sequential: senior gets principal first
        let senior = &output.tranche_results[0];
        assert_eq!(senior.original_balance, dec!(800));
        // Senior should receive all 800 in principal
        assert_eq!(senior.total_principal_received, dec!(800));
        assert_eq!(senior.ending_balance, dec!(0));
    }

    // -----------------------------------------------------------------------
    // Test 3: 3-tranche sequential pay
    // -----------------------------------------------------------------------
    #[test]
    fn test_three_tranche_sequential_pay() {
        let input = three_tranche_input();
        let result = analyze_tranching(&input).unwrap();
        let output = &result.result;

        let aaa = &output.tranche_results[0];
        let bbb = &output.tranche_results[1];
        let equity = &output.tranche_results[2];

        // Total principal = 100 * 8 = 800
        // AAA = 600, BBB = 200, Equity = 100
        // Sequential: AAA gets 600 first, then BBB gets 200, equity gets 0
        assert_eq!(aaa.total_principal_received, dec!(600));
        assert_eq!(aaa.ending_balance, dec!(0));
        assert_eq!(bbb.total_principal_received, dec!(200));
        assert_eq!(bbb.ending_balance, dec!(0));
        assert_eq!(equity.total_principal_received, dec!(0));
        assert_eq!(equity.ending_balance, dec!(100));
    }

    // -----------------------------------------------------------------------
    // Test 4: Loss allocation bottom-up
    // -----------------------------------------------------------------------
    #[test]
    fn test_loss_allocation_bottom_up() {
        let mut input = two_tranche_input();
        // Add 50 in losses per period for 4 periods = 200 total losses
        input.collateral_cashflows = make_cashflows(4, dec!(25), dec!(200), dec!(50));

        let result = analyze_tranching(&input).unwrap();
        let output = &result.result;

        let senior = &output.tranche_results[0];
        let equity = &output.tranche_results[1];

        // Equity has 150, so first 150 of losses go to equity, remaining 50 to senior
        assert_eq!(equity.loss_allocated, dec!(150));
        assert_eq!(senior.loss_allocated, dec!(50));
    }

    // -----------------------------------------------------------------------
    // Test 5: Losses absorbed entirely by equity, senior unimpaired
    // -----------------------------------------------------------------------
    #[test]
    fn test_losses_absorbed_by_equity_only() {
        let mut input = two_tranche_input();
        // 30 total losses (well within equity's 150 capacity)
        input.collateral_cashflows = make_cashflows(4, dec!(25), dec!(200), dec!(7.5));

        let result = analyze_tranching(&input).unwrap();
        let output = &result.result;

        let senior = &output.tranche_results[0];
        let equity = &output.tranche_results[1];

        assert_eq!(senior.loss_allocated, dec!(0));
        assert_eq!(equity.loss_allocated, dec!(30));
    }

    // -----------------------------------------------------------------------
    // Test 6: Losses exceeding equity, impairing mezzanine
    // -----------------------------------------------------------------------
    #[test]
    fn test_losses_exceed_equity_impair_mezzanine() {
        let mut input = three_tranche_input();
        // 200 total losses: 100 equity + 100 from BBB
        input.collateral_cashflows = make_cashflows(4, dec!(20), dec!(100), dec!(50));

        let result = analyze_tranching(&input).unwrap();
        let output = &result.result;

        let aaa = &output.tranche_results[0];
        let bbb = &output.tranche_results[1];
        let equity = &output.tranche_results[2];

        assert_eq!(equity.loss_allocated, dec!(100));
        assert_eq!(bbb.loss_allocated, dec!(100));
        assert_eq!(aaa.loss_allocated, dec!(0));
    }

    // -----------------------------------------------------------------------
    // Test 7: OC test trigger and turbo amortisation
    // -----------------------------------------------------------------------
    #[test]
    fn test_oc_trigger_turbo_amortisation() {
        let input = TranchingInput {
            deal_name: "OC Test Deal".into(),
            collateral_balance: dec!(1000),
            // 8 periods, moderate losses to trigger OC failure
            collateral_cashflows: make_cashflows(8, dec!(20), dec!(100), dec!(30)),
            tranches: vec![
                TrancheSpec {
                    name: "Senior".into(),
                    balance: dec!(700),
                    coupon_rate: dec!(0.04),
                    seniority: 1,
                    is_fixed_rate: true,
                    payment_frequency: 4,
                },
                TrancheSpec {
                    name: "Junior".into(),
                    balance: dec!(200),
                    coupon_rate: dec!(0.08),
                    seniority: 2,
                    is_fixed_rate: true,
                    payment_frequency: 4,
                },
            ],
            reserve_account: dec!(10),
            oc_trigger: Some(dec!(1.50)), // high trigger to force failure
            ic_trigger: None,
            reinvestment_period_months: 0,
        };

        let result = analyze_tranching(&input).unwrap();
        let output = &result.result;

        // With a high OC trigger, the test should fail in some periods
        let some_failed = output
            .waterfall_periods
            .iter()
            .any(|wp| wp.oc_test_result == Some(false));
        assert!(some_failed, "OC test should fail in at least one period");

        // When OC fails, principal should flow to senior (turbo)
        let senior = &output.tranche_results[0];
        assert!(
            senior.total_principal_received > Decimal::ZERO,
            "Senior should receive principal via turbo"
        );
    }

    // -----------------------------------------------------------------------
    // Test 8: IC test trigger with interest diversion
    // -----------------------------------------------------------------------
    #[test]
    fn test_ic_trigger_interest_diversion() {
        let input = TranchingInput {
            deal_name: "IC Test Deal".into(),
            collateral_balance: dec!(1000),
            // Very low interest to trigger IC failure
            collateral_cashflows: make_cashflows(4, dec!(5), dec!(200), dec!(0)),
            tranches: vec![
                TrancheSpec {
                    name: "Senior".into(),
                    balance: dec!(800),
                    coupon_rate: dec!(0.04),
                    seniority: 1,
                    is_fixed_rate: true,
                    payment_frequency: 4,
                },
                TrancheSpec {
                    name: "Equity".into(),
                    balance: dec!(100),
                    coupon_rate: dec!(0.10),
                    seniority: 2,
                    is_fixed_rate: true,
                    payment_frequency: 4,
                },
            ],
            reserve_account: dec!(10),
            oc_trigger: None,
            ic_trigger: Some(dec!(2.0)), // High IC trigger to force failure
            reinvestment_period_months: 0,
        };

        let result = analyze_tranching(&input).unwrap();
        let output = &result.result;

        // IC test: interest collected / senior interest due >= 2.0
        // Senior interest due = 800 * 0.04 / 4 = 8, available = 5
        // 5 / 8 = 0.625 < 2.0 => fail
        let failed = output
            .waterfall_periods
            .iter()
            .any(|wp| wp.ic_test_result == Some(false));
        assert!(failed, "IC test should fail");
    }

    // -----------------------------------------------------------------------
    // Test 9: Credit enhancement / subordination percentages
    // -----------------------------------------------------------------------
    #[test]
    fn test_credit_enhancement_subordination() {
        let input = three_tranche_input();
        let result = analyze_tranching(&input).unwrap();
        let ce = &result.result.credit_enhancement;

        // AAA subordination: (BBB 200 + Equity 100) / 900 = 33.33%
        let aaa_sub = ce
            .subordination
            .iter()
            .find(|s| s.tranche_name == "AAA")
            .unwrap();
        let expected = dec!(300) / dec!(900);
        assert!(
            (aaa_sub.subordination_pct - expected).abs() < dec!(0.001),
            "AAA subordination should be ~33.3%, got {}",
            aaa_sub.subordination_pct
        );

        // BBB subordination: Equity 100 / 900 = 11.11%
        let bbb_sub = ce
            .subordination
            .iter()
            .find(|s| s.tranche_name == "BBB")
            .unwrap();
        let expected_bbb = dec!(100) / dec!(900);
        assert!(
            (bbb_sub.subordination_pct - expected_bbb).abs() < dec!(0.001),
            "BBB subordination should be ~11.1%, got {}",
            bbb_sub.subordination_pct
        );

        // Equity subordination: 0%
        let eq_sub = ce
            .subordination
            .iter()
            .find(|s| s.tranche_name == "Equity")
            .unwrap();
        assert_eq!(eq_sub.subordination_pct, dec!(0));
    }

    // -----------------------------------------------------------------------
    // Test 10: Credit enhancement in tranche results
    // -----------------------------------------------------------------------
    #[test]
    fn test_tranche_result_credit_enhancement_pct() {
        let input = three_tranche_input();
        let result = analyze_tranching(&input).unwrap();

        let aaa = &result.result.tranche_results[0];
        let bbb = &result.result.tranche_results[1];
        let equity = &result.result.tranche_results[2];

        // AAA: (200 + 100) / 900 = 33.33%
        assert!((aaa.credit_enhancement_pct - dec!(300) / dec!(900)).abs() < dec!(0.001));
        // BBB: 100 / 900 = 11.11%
        assert!((bbb.credit_enhancement_pct - dec!(100) / dec!(900)).abs() < dec!(0.001));
        // Equity: 0%
        assert_eq!(equity.credit_enhancement_pct, dec!(0));
    }

    // -----------------------------------------------------------------------
    // Test 11: Reinvestment period behaviour
    // -----------------------------------------------------------------------
    #[test]
    fn test_reinvestment_period() {
        let mut input = two_tranche_input();
        input.reinvestment_period_months = 2; // First 2 periods reinvest principal

        let result = analyze_tranching(&input).unwrap();
        let output = &result.result;

        // During reinvestment (periods 1-2), principal should not go to tranches
        // (it's reinvested back into the pool)
        let p1_payments = &output.waterfall_periods[0].tranche_payments;
        let p2_payments = &output.waterfall_periods[1].tranche_payments;

        let p1_principal: Decimal = p1_payments.iter().map(|p| p.principal_paid).sum();
        let p2_principal: Decimal = p2_payments.iter().map(|p| p.principal_paid).sum();

        assert_eq!(p1_principal, dec!(0), "Period 1 should reinvest principal");
        assert_eq!(p2_principal, dec!(0), "Period 2 should reinvest principal");

        // After reinvestment, principal should flow to tranches
        let p3_payments = &output.waterfall_periods[2].tranche_payments;
        let p3_principal: Decimal = p3_payments.iter().map(|p| p.principal_paid).sum();
        assert!(
            p3_principal > Decimal::ZERO,
            "Period 3 should distribute principal"
        );
    }

    // -----------------------------------------------------------------------
    // Test 12: WAL calculation
    // -----------------------------------------------------------------------
    #[test]
    fn test_wal_calculation() {
        let input = two_tranche_input();
        let result = analyze_tranching(&input).unwrap();

        let senior = &result.result.tranche_results[0];
        // Senior gets all 800 in principal over 4 quarterly periods
        // WAL should be > 0 and reasonable (measured in years)
        assert!(
            senior.weighted_average_life > Decimal::ZERO,
            "Senior WAL should be positive"
        );
        // With quarterly payments over 4 periods (1 year), WAL should be roughly 0.5-1 years
        assert!(
            senior.weighted_average_life < dec!(2),
            "Senior WAL should be less than 2 years, got {}",
            senior.weighted_average_life
        );
    }

    // -----------------------------------------------------------------------
    // Test 13: YTM/IRR calculation for tranches
    // -----------------------------------------------------------------------
    #[test]
    fn test_tranche_ytm_positive() {
        let input = two_tranche_input();
        let result = analyze_tranching(&input).unwrap();

        let senior = &result.result.tranche_results[0];
        // Senior tranche is paid in full with coupon, so YTM should be close to coupon rate
        assert!(
            senior.yield_to_maturity > Decimal::ZERO,
            "Senior YTM should be positive, got {}",
            senior.yield_to_maturity
        );
        // Should be close to the 4% coupon
        assert!(
            (senior.yield_to_maturity - dec!(0.04)).abs() < dec!(0.02),
            "Senior YTM should be close to 4%, got {}",
            senior.yield_to_maturity
        );
    }

    // -----------------------------------------------------------------------
    // Test 14: Deal summary totals
    // -----------------------------------------------------------------------
    #[test]
    fn test_deal_summary_totals() {
        let input = two_tranche_input();
        let result = analyze_tranching(&input).unwrap();
        let summary = &result.result.deal_summary;

        assert_eq!(summary.total_collateral, dec!(1000));
        assert_eq!(summary.total_tranches, dec!(950)); // 800 + 150
        assert_eq!(summary.excess_collateral, dec!(50));
        assert_eq!(summary.total_losses, dec!(0));

        // Total interest + principal distributed should be positive
        assert!(summary.total_interest_distributed > Decimal::ZERO);
        assert!(summary.total_principal_distributed > Decimal::ZERO);
    }

    // -----------------------------------------------------------------------
    // Test 15: Zero losses scenario — all tranches paid in full
    // -----------------------------------------------------------------------
    #[test]
    fn test_zero_losses_all_paid() {
        // Enough principal to pay all tranches
        let input = TranchingInput {
            deal_name: "Zero Loss Deal".into(),
            collateral_balance: dec!(500),
            collateral_cashflows: make_cashflows(4, dec!(10), dec!(100), dec!(0)),
            tranches: vec![
                TrancheSpec {
                    name: "Senior".into(),
                    balance: dec!(300),
                    coupon_rate: dec!(0.04),
                    seniority: 1,
                    is_fixed_rate: true,
                    payment_frequency: 4,
                },
                TrancheSpec {
                    name: "Junior".into(),
                    balance: dec!(100),
                    coupon_rate: dec!(0.08),
                    seniority: 2,
                    is_fixed_rate: true,
                    payment_frequency: 4,
                },
            ],
            reserve_account: dec!(5),
            oc_trigger: None,
            ic_trigger: None,
            reinvestment_period_months: 0,
        };

        let result = analyze_tranching(&input).unwrap();
        let output = &result.result;

        for tr in &output.tranche_results {
            assert_eq!(
                tr.loss_allocated,
                dec!(0),
                "{} should have no losses",
                tr.name
            );
            assert_eq!(
                tr.ending_balance,
                dec!(0),
                "{} should be fully paid down",
                tr.name
            );
        }
    }

    // -----------------------------------------------------------------------
    // Test 16: Excess spread calculation
    // -----------------------------------------------------------------------
    #[test]
    fn test_excess_spread() {
        let input = two_tranche_input();
        let result = analyze_tranching(&input).unwrap();
        let ce = &result.result.credit_enhancement;

        // Excess spread = collateral WAC - weighted avg tranche cost
        // Should be some positive value for an overcollateralised deal
        // WAC estimate from collateral: 25 per quarter on 1000 = 10% annualised
        // Tranche cost: (800*0.04 + 150*0.10)/950 ≈ 0.0494
        // Excess spread ≈ 0.10 - 0.0494 ≈ 0.05+
        assert!(
            ce.excess_spread > Decimal::ZERO,
            "Excess spread should be positive, got {}",
            ce.excess_spread
        );
    }

    // -----------------------------------------------------------------------
    // Test 17: Reserve account drawdown
    // -----------------------------------------------------------------------
    #[test]
    fn test_reserve_account_drawdown() {
        // Create scenario where interest is just barely enough
        let input = TranchingInput {
            deal_name: "Reserve Draw Deal".into(),
            collateral_balance: dec!(1000),
            // Very low interest to force reserve draw
            collateral_cashflows: make_cashflows(4, dec!(6), dec!(200), dec!(0)),
            tranches: vec![
                TrancheSpec {
                    name: "Senior".into(),
                    balance: dec!(800),
                    coupon_rate: dec!(0.04),
                    seniority: 1,
                    is_fixed_rate: true,
                    payment_frequency: 4,
                },
                TrancheSpec {
                    name: "Equity".into(),
                    balance: dec!(100),
                    coupon_rate: dec!(0.10),
                    seniority: 2,
                    is_fixed_rate: true,
                    payment_frequency: 4,
                },
            ],
            reserve_account: dec!(50),
            oc_trigger: None,
            ic_trigger: None,
            reinvestment_period_months: 0,
        };

        let result = analyze_tranching(&input).unwrap();
        let output = &result.result;

        // Senior coupon = 800 * 0.04 / 4 = 8, Equity = 100 * 0.10 / 4 = 2.50
        // Total due = 10.5, available = 6 => shortfall = 4.5 => draw from reserve
        // After first period, reserve should be < 50
        let first_period_reserve = output.waterfall_periods[0].reserve_balance;
        assert!(
            first_period_reserve < dec!(50),
            "Reserve should be drawn down, got {}",
            first_period_reserve
        );
    }

    // -----------------------------------------------------------------------
    // Test 18: Overcollateralisation initial ratio
    // -----------------------------------------------------------------------
    #[test]
    fn test_oc_initial_ratio() {
        let input = two_tranche_input();
        let result = analyze_tranching(&input).unwrap();
        let ce = &result.result.credit_enhancement;

        // OC initial = 1000 / (800 + 150) = 1000/950 ≈ 1.0526
        let expected = dec!(1000) / dec!(950);
        assert!(
            (ce.overcollateralisation_initial - expected).abs() < dec!(0.001),
            "OC initial should be ~1.053, got {}",
            ce.overcollateralisation_initial
        );
    }

    // -----------------------------------------------------------------------
    // Test 19: Reserve account percentage
    // -----------------------------------------------------------------------
    #[test]
    fn test_reserve_account_pct() {
        let input = two_tranche_input();
        let result = analyze_tranching(&input).unwrap();
        let ce = &result.result.credit_enhancement;

        // Reserve pct = 10 / 1000 = 0.01
        assert_eq!(ce.reserve_account_pct, dec!(0.01));
    }

    // -----------------------------------------------------------------------
    // Test 20: Validation — no tranches
    // -----------------------------------------------------------------------
    #[test]
    fn test_validation_no_tranches() {
        let input = TranchingInput {
            deal_name: "Empty".into(),
            collateral_balance: dec!(1000),
            collateral_cashflows: make_cashflows(4, dec!(25), dec!(200), dec!(0)),
            tranches: vec![],
            reserve_account: dec!(0),
            oc_trigger: None,
            ic_trigger: None,
            reinvestment_period_months: 0,
        };

        let result = analyze_tranching(&input);
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // Test 21: Validation — no cashflows
    // -----------------------------------------------------------------------
    #[test]
    fn test_validation_no_cashflows() {
        let input = TranchingInput {
            deal_name: "No CFs".into(),
            collateral_balance: dec!(1000),
            collateral_cashflows: vec![],
            tranches: vec![TrancheSpec {
                name: "A".into(),
                balance: dec!(800),
                coupon_rate: dec!(0.04),
                seniority: 1,
                is_fixed_rate: true,
                payment_frequency: 4,
            }],
            reserve_account: dec!(0),
            oc_trigger: None,
            ic_trigger: None,
            reinvestment_period_months: 0,
        };

        assert!(analyze_tranching(&input).is_err());
    }

    // -----------------------------------------------------------------------
    // Test 22: Validation — tranche balance exceeds collateral
    // -----------------------------------------------------------------------
    #[test]
    fn test_validation_tranches_exceed_collateral() {
        let input = TranchingInput {
            deal_name: "Oversize".into(),
            collateral_balance: dec!(100),
            collateral_cashflows: make_cashflows(1, dec!(5), dec!(50), dec!(0)),
            tranches: vec![TrancheSpec {
                name: "A".into(),
                balance: dec!(200),
                coupon_rate: dec!(0.04),
                seniority: 1,
                is_fixed_rate: true,
                payment_frequency: 4,
            }],
            reserve_account: dec!(0),
            oc_trigger: None,
            ic_trigger: None,
            reinvestment_period_months: 0,
        };

        assert!(analyze_tranching(&input).is_err());
    }

    // -----------------------------------------------------------------------
    // Test 23: Waterfall period count matches input
    // -----------------------------------------------------------------------
    #[test]
    fn test_waterfall_period_count() {
        let input = two_tranche_input();
        let result = analyze_tranching(&input).unwrap();
        assert_eq!(result.result.waterfall_periods.len(), 4);
    }

    // -----------------------------------------------------------------------
    // Test 24: Interest shortfall tracked correctly
    // -----------------------------------------------------------------------
    #[test]
    fn test_interest_shortfall() {
        let input = TranchingInput {
            deal_name: "Shortfall Deal".into(),
            collateral_balance: dec!(1000),
            // Very low interest: 2 per period
            collateral_cashflows: make_cashflows(4, dec!(2), dec!(200), dec!(0)),
            tranches: vec![
                TrancheSpec {
                    name: "Senior".into(),
                    balance: dec!(800),
                    coupon_rate: dec!(0.04),
                    seniority: 1,
                    is_fixed_rate: true,
                    payment_frequency: 4,
                },
                TrancheSpec {
                    name: "Equity".into(),
                    balance: dec!(100),
                    coupon_rate: dec!(0.10),
                    seniority: 2,
                    is_fixed_rate: true,
                    payment_frequency: 4,
                },
            ],
            reserve_account: dec!(0), // no reserve to draw from
            oc_trigger: None,
            ic_trigger: None,
            reinvestment_period_months: 0,
        };

        let result = analyze_tranching(&input).unwrap();
        let first_period = &result.result.waterfall_periods[0];

        // Senior due: 800 * 0.04 / 4 = 8, available = 2, shortfall = 6
        let senior_payment = &first_period.tranche_payments[0];
        assert!(
            senior_payment.interest_shortfall > Decimal::ZERO,
            "Senior should have interest shortfall"
        );
    }

    // -----------------------------------------------------------------------
    // Test 25: Weighted average tranche cost
    // -----------------------------------------------------------------------
    #[test]
    fn test_weighted_avg_tranche_cost() {
        let input = two_tranche_input();
        let result = analyze_tranching(&input).unwrap();
        let summary = &result.result.deal_summary;

        // WAC = (800*0.04 + 150*0.10) / 950 = (32 + 15) / 950 = 47/950 ≈ 0.04947
        let expected = dec!(47) / dec!(950);
        assert!(
            (summary.weighted_avg_tranche_cost - expected).abs() < dec!(0.001),
            "Weighted avg cost should be ~4.95%, got {}",
            summary.weighted_avg_tranche_cost
        );
    }

    // -----------------------------------------------------------------------
    // Test 26: Methodology and metadata present
    // -----------------------------------------------------------------------
    #[test]
    fn test_methodology_metadata() {
        let input = two_tranche_input();
        let result = analyze_tranching(&input).unwrap();

        assert!(result.methodology.contains("CDO/CLO Tranching"));
        assert!(!result.metadata.version.is_empty());
    }

    // -----------------------------------------------------------------------
    // Test 27: Seniority ordering is respected regardless of input order
    // -----------------------------------------------------------------------
    #[test]
    fn test_seniority_ordering_independent_of_input() {
        let mut input = three_tranche_input();
        // Reverse the input order
        input.tranches.reverse();

        let result = analyze_tranching(&input).unwrap();
        let output = &result.result;

        // Results should still be AAA first (seniority 1)
        assert_eq!(output.tranche_results[0].name, "AAA");
        assert_eq!(output.tranche_results[1].name, "BBB");
        assert_eq!(output.tranche_results[2].name, "Equity");
    }

    // -----------------------------------------------------------------------
    // Test 28: Equity tranche has zero subordination
    // -----------------------------------------------------------------------
    #[test]
    fn test_equity_zero_subordination() {
        let input = three_tranche_input();
        let result = analyze_tranching(&input).unwrap();

        let equity = &result.result.tranche_results[2];
        assert_eq!(equity.credit_enhancement_pct, dec!(0));
    }

    // -----------------------------------------------------------------------
    // Test 29: Large loss scenario wipes all tranches
    // -----------------------------------------------------------------------
    #[test]
    fn test_total_wipeout() {
        let input = TranchingInput {
            deal_name: "Wipeout".into(),
            collateral_balance: dec!(1000),
            // 250 loss per period * 4 = 1000 total losses
            collateral_cashflows: make_cashflows(4, dec!(10), dec!(0), dec!(250)),
            tranches: vec![
                TrancheSpec {
                    name: "Senior".into(),
                    balance: dec!(600),
                    coupon_rate: dec!(0.04),
                    seniority: 1,
                    is_fixed_rate: true,
                    payment_frequency: 4,
                },
                TrancheSpec {
                    name: "Junior".into(),
                    balance: dec!(200),
                    coupon_rate: dec!(0.08),
                    seniority: 2,
                    is_fixed_rate: true,
                    payment_frequency: 4,
                },
                TrancheSpec {
                    name: "Equity".into(),
                    balance: dec!(100),
                    coupon_rate: dec!(0.12),
                    seniority: 3,
                    is_fixed_rate: true,
                    payment_frequency: 4,
                },
            ],
            reserve_account: dec!(0),
            oc_trigger: None,
            ic_trigger: None,
            reinvestment_period_months: 0,
        };

        let result = analyze_tranching(&input).unwrap();
        let summary = &result.result.deal_summary;

        // Total losses = 1000, all tranches total = 900
        // All tranches should be wiped: equity 100, junior 200, senior 600 -> only 900 absorbed
        assert_eq!(summary.total_losses, dec!(900));
    }

    // -----------------------------------------------------------------------
    // Test 30: Validation — negative collateral balance
    // -----------------------------------------------------------------------
    #[test]
    fn test_validation_negative_collateral() {
        let input = TranchingInput {
            deal_name: "Negative".into(),
            collateral_balance: dec!(-100),
            collateral_cashflows: make_cashflows(1, dec!(5), dec!(50), dec!(0)),
            tranches: vec![TrancheSpec {
                name: "A".into(),
                balance: dec!(50),
                coupon_rate: dec!(0.04),
                seniority: 1,
                is_fixed_rate: true,
                payment_frequency: 4,
            }],
            reserve_account: dec!(0),
            oc_trigger: None,
            ic_trigger: None,
            reinvestment_period_months: 0,
        };

        assert!(analyze_tranching(&input).is_err());
    }
}
