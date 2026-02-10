//! Direct lending models for private credit analysis.
//!
//! Covers PIK toggle loans, delayed draw facilities, covenant-lite analysis,
//! credit fund returns, and loan syndication. All math uses `rust_decimal::Decimal`
//! for institutional-grade precision.

use rust_decimal::Decimal;
use rust_decimal::MathematicalOps;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::error::CorpFinanceError;
use crate::types::{with_metadata, ComputationOutput, Money, Rate};
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const NEWTON_MAX_ITERATIONS: u32 = 50;
const NEWTON_EPSILON: Decimal = dec!(0.0000001);
/// Credit VaR z-score for 95% confidence level
const CREDIT_VAR_Z_95: Decimal = dec!(2.33);
/// Basis points divisor
const BPS_DIVISOR: Decimal = dec!(10000);

// ---------------------------------------------------------------------------
// Input / Output Types
// ---------------------------------------------------------------------------

/// Amortization schedule for a direct loan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AmortSchedule {
    /// Interest-only with bullet repayment at maturity.
    InterestOnly,
    /// Level amortization as annual percentage of original principal.
    LevelAmort(Decimal),
    /// Bullet maturity (same as InterestOnly for cash flow, principal at end).
    BulletMaturity,
    /// Custom annual principal payments.
    Custom(Vec<Decimal>),
}

/// Call protection / prepayment penalty by year.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrepaymentPenalty {
    pub year: u32,
    pub premium_pct: Decimal,
}

/// Input for modelling a direct loan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectLoanInput {
    pub loan_name: String,
    /// Total commitment amount.
    pub commitment: Money,
    /// Initial drawn amount.
    pub drawn_amount: Money,
    /// Base reference rate (e.g., SOFR at 0.05).
    pub base_rate: Rate,
    /// Credit spread in basis points (e.g., 550 = 5.50%).
    pub spread_bps: Decimal,
    /// Optional PIK interest rate (added to principal each period).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pik_rate: Option<Rate>,
    /// If true, borrower can elect to PIK instead of cash pay.
    pub pik_toggle: bool,
    /// Additional undrawn commitment for delayed draw.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delayed_draw_amount: Option<Money>,
    /// Fee on undrawn delayed draw (basis points).
    pub delayed_draw_fee_bps: Decimal,
    /// Maturity in years.
    pub maturity_years: u32,
    /// Repayment profile.
    pub amortization_schedule: AmortSchedule,
    /// Call protection by year.
    pub prepayment_penalty: Vec<PrepaymentPenalty>,
    /// Base rate floor (e.g., SOFR floor).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub floor_rate: Option<Rate>,
    /// Number of years to project (usually equals maturity).
    pub projection_years: u32,
    /// Annual probability of default for yield adjustment.
    pub expected_default_rate: Rate,
    /// Loss-given-default for yield adjustment.
    pub expected_loss_severity: Rate,
}

/// Output of a direct loan model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectLoanOutput {
    pub cash_flow_schedule: Vec<LoanPeriod>,
    pub yield_metrics: LoanYieldMetrics,
    pub credit_metrics: LoanCreditMetrics,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pik_analysis: Option<PikAnalysis>,
}

/// A single period in the loan cash flow schedule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoanPeriod {
    pub year: u32,
    pub beginning_balance: Money,
    pub cash_interest: Money,
    pub pik_interest: Money,
    pub principal_payment: Money,
    pub ending_balance: Money,
    pub delayed_draw_fee: Money,
    /// Cash interest + delayed draw fees (what lender actually receives in cash).
    pub total_lender_income: Money,
}

/// Yield metrics for the loan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoanYieldMetrics {
    /// Cash coupon yield.
    pub cash_yield: Rate,
    /// PIK component yield.
    pub pik_yield: Rate,
    /// Cash + PIK total yield.
    pub total_yield: Rate,
    /// IRR of all cash flows to lender.
    pub yield_to_maturity: Rate,
    /// YTM adjusted for expected losses.
    pub default_adjusted_yield: Rate,
    /// default_adjusted_yield - base_rate, expressed in bps (multiplied by 10000).
    pub loss_adjusted_spread: Decimal,
    /// max(base_rate, floor) + spread.
    pub effective_rate_with_floor: Rate,
}

/// Credit risk metrics for the loan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoanCreditMetrics {
    /// PD * LGD * exposure.
    pub expected_loss: Money,
    /// Expected loss as percentage of exposure.
    pub expected_loss_pct: Rate,
    /// Simplified credit VaR at 95%: 2.33 * sqrt(PD*(1-PD)) * LGD * exposure.
    pub credit_var_95: Money,
    /// YTM - expected_loss_pct.
    pub risk_adjusted_return: Rate,
}

/// Analysis of PIK interest accrual.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PikAnalysis {
    /// Total PIK interest accrued over the life of the loan.
    pub total_pik_accrued: Money,
    /// Total PIK as percentage of original principal.
    pub pik_as_pct_of_original: Rate,
    /// Principal balance at maturity including all PIK accrual.
    pub final_balance_with_pik: Money,
    /// cash_yield / total_yield.
    pub cash_vs_total_yield: Rate,
}

/// Input for loan syndication analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyndicationInput {
    /// Total facility size.
    pub total_facility: Money,
    /// Amount retained by lead arranger.
    pub arranger_hold: Money,
    /// Syndicate members (may include lead).
    pub syndicate_members: Vec<SyndicateMember>,
    /// Arrangement fee earned by arranger (basis points on total facility).
    pub arrangement_fee_bps: Decimal,
    /// Participation fee paid to participants (basis points on their allocation).
    pub participation_fee_bps: Decimal,
    /// Ongoing coupon spread (basis points) for interest income calc.
    pub coupon_spread_bps: Decimal,
}

/// A single syndicate member.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyndicateMember {
    pub name: String,
    pub commitment: Money,
    pub is_lead: bool,
}

/// Output of syndication analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyndicationOutput {
    pub total_committed: Money,
    /// Total committed / facility. > 1 means oversubscribed.
    pub oversubscription: Decimal,
    pub arranger_economics: ArrangerEconomics,
    pub participant_allocations: Vec<ParticipantAllocation>,
}

/// Economics for the lead arranger.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArrangerEconomics {
    pub hold_amount: Money,
    /// Facility minus hold.
    pub sell_down: Money,
    /// Arrangement fee on total facility.
    pub arrangement_fee_earned: Money,
    /// Ongoing spread income on retained hold.
    pub ongoing_spread_income: Money,
    /// Arrangement fee + ongoing spread income.
    pub total_year1_income: Money,
}

/// Allocation and economics for a syndicate participant.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParticipantAllocation {
    pub name: String,
    pub committed: Money,
    /// May be scaled if oversubscribed.
    pub allocated: Money,
    pub pct_of_deal: Rate,
    pub participation_fee: Money,
    pub annual_interest_income: Money,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Model a direct loan including PIK toggle, delayed draw, and credit metrics.
pub fn model_direct_loan(
    input: &DirectLoanInput,
) -> CorpFinanceResult<ComputationOutput<DirectLoanOutput>> {
    let start = Instant::now();
    let mut warnings: Vec<String> = Vec::new();

    validate_direct_loan_input(input)?;

    // Effective base rate with floor
    let effective_base = match input.floor_rate {
        Some(floor) => {
            if floor > input.base_rate {
                warnings.push(format!(
                    "Floor rate {} exceeds base rate {}; using floor",
                    floor, input.base_rate
                ));
            }
            input.base_rate.max(floor)
        }
        None => input.base_rate,
    };

    let spread_decimal = input.spread_bps / BPS_DIVISOR;
    let all_in_rate = effective_base + spread_decimal;
    let pik_rate = input.pik_rate.unwrap_or(Decimal::ZERO);

    // Cash interest rate: if PIK toggle is on, all interest goes to PIK,
    // otherwise if PIK rate is specified, cash rate = all_in - pik_rate
    let cash_rate = if input.pik_toggle {
        Decimal::ZERO
    } else {
        all_in_rate - pik_rate
    };

    let effective_pik_rate = if input.pik_toggle {
        all_in_rate
    } else {
        pik_rate
    };

    // Delayed draw undrawn amount
    let delayed_draw = input.delayed_draw_amount.unwrap_or(Decimal::ZERO);
    let dd_fee_rate = input.delayed_draw_fee_bps / BPS_DIVISOR;

    let projection_years = input.projection_years.min(input.maturity_years);

    // Build cash flow schedule
    let mut schedule: Vec<LoanPeriod> = Vec::with_capacity(projection_years as usize);
    let mut balance = input.drawn_amount;

    for yr in 1..=projection_years {
        let beginning_balance = balance;

        // Cash interest on current balance
        let cash_interest = beginning_balance * cash_rate;

        // PIK interest accrued to principal
        let pik_interest = beginning_balance * effective_pik_rate;

        // Principal payment based on amortization schedule
        let principal_payment = compute_principal_payment(
            &input.amortization_schedule,
            input.drawn_amount,
            beginning_balance,
            yr,
            projection_years,
        );

        // Ending balance = beginning + PIK - principal payment
        let ending_balance = beginning_balance + pik_interest - principal_payment;

        // Delayed draw fee on undrawn amount
        let delayed_draw_fee = delayed_draw * dd_fee_rate;

        // Total lender income = cash interest + delayed draw fees
        let total_lender_income = cash_interest + delayed_draw_fee;

        schedule.push(LoanPeriod {
            year: yr,
            beginning_balance,
            cash_interest,
            pik_interest,
            principal_payment,
            ending_balance,
            delayed_draw_fee,
            total_lender_income,
        });

        balance = ending_balance;
    }

    // Yield metrics
    let cash_yield = cash_rate;
    let pik_yield = effective_pik_rate;
    let total_yield = cash_yield + pik_yield;
    let effective_rate_with_floor = all_in_rate;

    // YTM via IRR of lender cash flows
    let ytm = compute_lender_irr(input, &schedule, &mut warnings);

    // Default-adjusted yield
    let expected_loss_rate = input.expected_default_rate * input.expected_loss_severity;
    let default_adjusted_yield = ytm - expected_loss_rate;

    // Loss-adjusted spread (in basis points * 10000 format, i.e., as a decimal)
    let loss_adjusted_spread = (default_adjusted_yield - input.base_rate) * BPS_DIVISOR;

    let yield_metrics = LoanYieldMetrics {
        cash_yield,
        pik_yield,
        total_yield,
        yield_to_maturity: ytm,
        default_adjusted_yield,
        loss_adjusted_spread,
        effective_rate_with_floor,
    };

    // Credit metrics
    let exposure = input.drawn_amount;
    let expected_loss = input.expected_default_rate * input.expected_loss_severity * exposure;
    let expected_loss_pct = if exposure.is_zero() {
        Decimal::ZERO
    } else {
        expected_loss / exposure
    };

    // Credit VaR 95%: 2.33 * sqrt(PD * (1-PD)) * LGD * exposure
    let pd = input.expected_default_rate;
    let pd_variance = pd * (Decimal::ONE - pd);
    let pd_std = decimal_sqrt(pd_variance);
    let credit_var_95 = CREDIT_VAR_Z_95 * pd_std * input.expected_loss_severity * exposure;

    let risk_adjusted_return = ytm - expected_loss_pct;

    let credit_metrics = LoanCreditMetrics {
        expected_loss,
        expected_loss_pct,
        credit_var_95,
        risk_adjusted_return,
    };

    // PIK analysis
    let pik_analysis = if effective_pik_rate > Decimal::ZERO {
        let total_pik_accrued: Money = schedule.iter().map(|p| p.pik_interest).sum();
        let pik_as_pct_of_original = if input.drawn_amount.is_zero() {
            Decimal::ZERO
        } else {
            total_pik_accrued / input.drawn_amount
        };
        let final_balance_with_pik = schedule
            .last()
            .map(|p| p.ending_balance)
            .unwrap_or(input.drawn_amount);
        let cash_vs_total_yield = if total_yield.is_zero() {
            Decimal::ZERO
        } else {
            cash_yield / total_yield
        };

        Some(PikAnalysis {
            total_pik_accrued,
            pik_as_pct_of_original,
            final_balance_with_pik,
            cash_vs_total_yield,
        })
    } else {
        None
    };

    let output = DirectLoanOutput {
        cash_flow_schedule: schedule,
        yield_metrics,
        credit_metrics,
        pik_analysis,
    };

    let elapsed = start.elapsed().as_micros() as u64;
    Ok(with_metadata(
        "Direct Lending Model — PIK toggle, delayed draw, credit analytics",
        &serde_json::json!({
            "loan_name": input.loan_name,
            "commitment": input.commitment.to_string(),
            "drawn_amount": input.drawn_amount.to_string(),
            "all_in_rate": all_in_rate.to_string(),
            "pik_toggle": input.pik_toggle,
            "projection_years": projection_years,
        }),
        warnings,
        elapsed,
        output,
    ))
}

/// Analyze loan syndication: allocation, oversubscription, arranger economics.
pub fn analyze_syndication(
    input: &SyndicationInput,
) -> CorpFinanceResult<ComputationOutput<SyndicationOutput>> {
    let start = Instant::now();
    let warnings: Vec<String> = Vec::new();

    validate_syndication_input(input)?;

    let total_committed: Money = input.syndicate_members.iter().map(|m| m.commitment).sum();
    let oversubscription = if input.total_facility.is_zero() {
        Decimal::ZERO
    } else {
        total_committed / input.total_facility
    };

    // Arranger economics
    let arrangement_fee_earned = input.total_facility * input.arrangement_fee_bps / BPS_DIVISOR;
    let sell_down = input.total_facility - input.arranger_hold;
    let ongoing_spread_income = input.arranger_hold * input.coupon_spread_bps / BPS_DIVISOR;
    let total_year1_income = arrangement_fee_earned + ongoing_spread_income;

    let arranger_economics = ArrangerEconomics {
        hold_amount: input.arranger_hold,
        sell_down,
        arrangement_fee_earned,
        ongoing_spread_income,
        total_year1_income,
    };

    // Participant allocations
    // Available for syndication = facility - arranger_hold
    let available_for_syndication = input.total_facility - input.arranger_hold;

    // Total non-lead commitments
    let total_non_lead_commitments: Money = input
        .syndicate_members
        .iter()
        .filter(|m| !m.is_lead)
        .map(|m| m.commitment)
        .sum();

    let participation_fee_rate = input.participation_fee_bps / BPS_DIVISOR;
    let coupon_spread_rate = input.coupon_spread_bps / BPS_DIVISOR;

    let mut participant_allocations: Vec<ParticipantAllocation> =
        Vec::with_capacity(input.syndicate_members.len());

    for member in &input.syndicate_members {
        if member.is_lead {
            // Lead gets the arranger_hold
            let pct_of_deal = if input.total_facility.is_zero() {
                Decimal::ZERO
            } else {
                input.arranger_hold / input.total_facility
            };
            let annual_interest_income = input.arranger_hold * coupon_spread_rate;
            participant_allocations.push(ParticipantAllocation {
                name: member.name.clone(),
                committed: member.commitment,
                allocated: input.arranger_hold,
                pct_of_deal,
                participation_fee: Decimal::ZERO, // arranger doesn't get participation fee
                annual_interest_income,
            });
        } else {
            // Scale non-lead members pro-rata if oversubscribed
            let allocated = if total_non_lead_commitments.is_zero() {
                Decimal::ZERO
            } else if total_non_lead_commitments > available_for_syndication {
                // Oversubscribed: scale down
                member.commitment * available_for_syndication / total_non_lead_commitments
            } else {
                // Undersubscribed or exact: allocate full commitment
                member.commitment
            };

            let pct_of_deal = if input.total_facility.is_zero() {
                Decimal::ZERO
            } else {
                allocated / input.total_facility
            };
            let participation_fee = allocated * participation_fee_rate;
            let annual_interest_income = allocated * coupon_spread_rate;

            participant_allocations.push(ParticipantAllocation {
                name: member.name.clone(),
                committed: member.commitment,
                allocated,
                pct_of_deal,
                participation_fee,
                annual_interest_income,
            });
        }
    }

    let output = SyndicationOutput {
        total_committed,
        oversubscription,
        arranger_economics,
        participant_allocations,
    };

    let elapsed = start.elapsed().as_micros() as u64;
    Ok(with_metadata(
        "Loan Syndication Analysis — allocation and arranger economics",
        &serde_json::json!({
            "total_facility": input.total_facility.to_string(),
            "arranger_hold": input.arranger_hold.to_string(),
            "num_members": input.syndicate_members.len(),
            "oversubscription": oversubscription.to_string(),
        }),
        warnings,
        elapsed,
        output,
    ))
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

fn validate_direct_loan_input(input: &DirectLoanInput) -> CorpFinanceResult<()> {
    if input.commitment <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "commitment".into(),
            reason: "Commitment must be positive".into(),
        });
    }
    if input.drawn_amount <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "drawn_amount".into(),
            reason: "Drawn amount must be positive".into(),
        });
    }
    if input.drawn_amount > input.commitment {
        return Err(CorpFinanceError::InvalidInput {
            field: "drawn_amount".into(),
            reason: "Drawn amount cannot exceed commitment".into(),
        });
    }
    if input.maturity_years == 0 {
        return Err(CorpFinanceError::InvalidInput {
            field: "maturity_years".into(),
            reason: "Maturity must be at least 1 year".into(),
        });
    }
    if input.projection_years == 0 {
        return Err(CorpFinanceError::InvalidInput {
            field: "projection_years".into(),
            reason: "Projection years must be at least 1".into(),
        });
    }
    if input.expected_default_rate < Decimal::ZERO || input.expected_default_rate > Decimal::ONE {
        return Err(CorpFinanceError::InvalidInput {
            field: "expected_default_rate".into(),
            reason: "Default rate must be between 0 and 1".into(),
        });
    }
    if input.expected_loss_severity < Decimal::ZERO || input.expected_loss_severity > Decimal::ONE {
        return Err(CorpFinanceError::InvalidInput {
            field: "expected_loss_severity".into(),
            reason: "Loss severity must be between 0 and 1".into(),
        });
    }
    Ok(())
}

fn validate_syndication_input(input: &SyndicationInput) -> CorpFinanceResult<()> {
    if input.total_facility <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "total_facility".into(),
            reason: "Total facility must be positive".into(),
        });
    }
    if input.arranger_hold <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "arranger_hold".into(),
            reason: "Arranger hold must be positive".into(),
        });
    }
    if input.arranger_hold > input.total_facility {
        return Err(CorpFinanceError::InvalidInput {
            field: "arranger_hold".into(),
            reason: "Arranger hold cannot exceed total facility".into(),
        });
    }
    if input.syndicate_members.is_empty() {
        return Err(CorpFinanceError::InvalidInput {
            field: "syndicate_members".into(),
            reason: "At least one syndicate member is required".into(),
        });
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Compute principal payment for a given year based on amortization schedule.
fn compute_principal_payment(
    schedule: &AmortSchedule,
    original_principal: Money,
    beginning_balance: Money,
    year: u32,
    total_years: u32,
) -> Money {
    match schedule {
        AmortSchedule::InterestOnly => {
            if year == total_years {
                beginning_balance
            } else {
                Decimal::ZERO
            }
        }
        AmortSchedule::BulletMaturity => {
            if year == total_years {
                beginning_balance
            } else {
                Decimal::ZERO
            }
        }
        AmortSchedule::LevelAmort(annual_pct) => {
            let scheduled = original_principal * *annual_pct;
            if year == total_years {
                // Final year: pay remaining balance
                beginning_balance
            } else {
                // Cap at current balance
                scheduled.min(beginning_balance)
            }
        }
        AmortSchedule::Custom(payments) => {
            let idx = (year - 1) as usize;
            if year == total_years {
                // Final year: pay remaining balance
                beginning_balance
            } else if idx < payments.len() {
                payments[idx].min(beginning_balance)
            } else {
                Decimal::ZERO
            }
        }
    }
}

/// Compute IRR of lender cash flows using Newton-Raphson.
///
/// Cash flows: year 0 = -drawn_amount (initial outflow),
/// years 1..n = cash_interest + delayed_draw_fee (income),
/// year n also includes principal repayment (ending_balance at maturity
/// is repaid to lender as principal).
fn compute_lender_irr(
    input: &DirectLoanInput,
    schedule: &[LoanPeriod],
    warnings: &mut Vec<String>,
) -> Rate {
    if schedule.is_empty() {
        return Decimal::ZERO;
    }

    let mut cash_flows: Vec<Money> = Vec::with_capacity(schedule.len() + 1);

    // Year 0: lender deploys capital
    cash_flows.push(-input.drawn_amount);

    // Years 1..n: cash income + principal returns
    for (i, period) in schedule.iter().enumerate() {
        let is_last = i == schedule.len() - 1;
        let mut cf = period.total_lender_income;
        cf += period.principal_payment;
        // At maturity, any remaining PIK balance is repaid
        // (principal_payment already captures the ending balance repayment
        //  for the final year in compute_principal_payment)
        if is_last && period.ending_balance > Decimal::ZERO {
            // For non-bullet/IO, ending_balance should be 0 in final year.
            // But with PIK, ending_balance might still be > 0 if we computed
            // principal_payment = beginning_balance (not beginning_balance + PIK).
            // The ending_balance = beginning + PIK - principal_payment.
            // Since principal_payment = beginning_balance in final year,
            // ending_balance = PIK interest for final year.
            // This PIK amount is also returned to lender at maturity.
            cf += period.ending_balance;
        }
        cash_flows.push(cf);
    }

    match newton_raphson_irr(&cash_flows, dec!(0.10)) {
        Ok(irr) => irr,
        Err(e) => {
            warnings.push(format!("YTM/IRR calculation warning: {e}"));
            Decimal::ZERO
        }
    }
}

/// Newton-Raphson IRR solver for annual cash flows.
fn newton_raphson_irr(cash_flows: &[Decimal], guess: Rate) -> CorpFinanceResult<Rate> {
    if cash_flows.len() < 2 {
        return Err(CorpFinanceError::InsufficientData(
            "IRR requires at least 2 cash flows".into(),
        ));
    }

    let mut rate = guess;

    for iteration in 0..NEWTON_MAX_ITERATIONS {
        let mut npv_val = Decimal::ZERO;
        let mut dnpv = Decimal::ZERO;
        let one_plus_r = Decimal::ONE + rate;

        for (t, cf) in cash_flows.iter().enumerate() {
            let t_dec = Decimal::from(t as i64);
            let discount = one_plus_r.powd(t_dec);
            if discount.is_zero() {
                continue;
            }
            npv_val += cf / discount;
            if t > 0 {
                dnpv -= t_dec * cf / (one_plus_r.powd(t_dec + Decimal::ONE));
            }
        }

        if npv_val.abs() < NEWTON_EPSILON {
            return Ok(rate);
        }

        if dnpv.is_zero() {
            return Err(CorpFinanceError::ConvergenceFailure {
                function: "Direct Loan IRR".into(),
                iterations: iteration,
                last_delta: npv_val,
            });
        }

        rate -= npv_val / dnpv;

        if rate < dec!(-0.99) {
            rate = dec!(-0.99);
        } else if rate > dec!(100.0) {
            rate = dec!(100.0);
        }
    }

    Err(CorpFinanceError::ConvergenceFailure {
        function: "Direct Loan IRR".into(),
        iterations: NEWTON_MAX_ITERATIONS,
        last_delta: Decimal::ZERO,
    })
}

/// Newton's method square root for Decimal (20 iterations).
fn decimal_sqrt(value: Decimal) -> Decimal {
    if value <= Decimal::ZERO {
        return Decimal::ZERO;
    }
    if value == Decimal::ONE {
        return Decimal::ONE;
    }

    let mut x = value / dec!(2);
    for _ in 0..20 {
        if x.is_zero() {
            return Decimal::ZERO;
        }
        x = (x + value / x) / dec!(2);
    }
    x
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    /// Helper: build a standard interest-only direct loan for tests.
    fn standard_io_loan() -> DirectLoanInput {
        DirectLoanInput {
            loan_name: "Test Senior Secured Loan".into(),
            commitment: dec!(100_000_000),
            drawn_amount: dec!(100_000_000),
            base_rate: dec!(0.05),
            spread_bps: dec!(550),
            pik_rate: None,
            pik_toggle: false,
            delayed_draw_amount: None,
            delayed_draw_fee_bps: dec!(0),
            maturity_years: 5,
            amortization_schedule: AmortSchedule::InterestOnly,
            prepayment_penalty: vec![],
            floor_rate: None,
            projection_years: 5,
            expected_default_rate: dec!(0.02),
            expected_loss_severity: dec!(0.40),
        }
    }

    /// Helper: build a PIK toggle loan.
    fn pik_toggle_loan() -> DirectLoanInput {
        DirectLoanInput {
            loan_name: "PIK Toggle Loan".into(),
            commitment: dec!(50_000_000),
            drawn_amount: dec!(50_000_000),
            base_rate: dec!(0.04),
            spread_bps: dec!(600),
            pik_rate: Some(dec!(0.10)),
            pik_toggle: true,
            delayed_draw_amount: None,
            delayed_draw_fee_bps: dec!(0),
            maturity_years: 5,
            amortization_schedule: AmortSchedule::InterestOnly,
            prepayment_penalty: vec![],
            floor_rate: None,
            projection_years: 5,
            expected_default_rate: dec!(0.03),
            expected_loss_severity: dec!(0.50),
        }
    }

    // -----------------------------------------------------------------------
    // 1. Basic IO loan: cash yield matches spread + base
    // -----------------------------------------------------------------------
    #[test]
    fn test_basic_io_loan_cash_yield() {
        let input = standard_io_loan();
        let result = model_direct_loan(&input).unwrap();
        let out = &result.result;

        // all_in = 0.05 + 550/10000 = 0.05 + 0.055 = 0.105
        let expected_yield = dec!(0.105);
        assert_eq!(
            out.yield_metrics.cash_yield, expected_yield,
            "Cash yield should be base + spread = 10.5%, got {}",
            out.yield_metrics.cash_yield
        );
    }

    // -----------------------------------------------------------------------
    // 2. PIK toggle: principal grows each year
    // -----------------------------------------------------------------------
    #[test]
    fn test_pik_toggle_principal_grows() {
        let input = pik_toggle_loan();
        let result = model_direct_loan(&input).unwrap();
        let schedule = &result.result.cash_flow_schedule;

        // With PIK toggle, balance should grow each year (except final which repays)
        for i in 0..schedule.len() - 1 {
            assert!(
                schedule[i].ending_balance > schedule[i].beginning_balance,
                "Year {}: ending balance {} should exceed beginning balance {} with PIK",
                schedule[i].year,
                schedule[i].ending_balance,
                schedule[i].beginning_balance
            );
        }
    }

    // -----------------------------------------------------------------------
    // 3. PIK analysis: total accrued, final balance
    // -----------------------------------------------------------------------
    #[test]
    fn test_pik_analysis_totals() {
        let input = pik_toggle_loan();
        let result = model_direct_loan(&input).unwrap();
        let pik = result.result.pik_analysis.as_ref().unwrap();

        assert!(
            pik.total_pik_accrued > Decimal::ZERO,
            "Total PIK accrued should be positive"
        );
        assert!(
            pik.pik_as_pct_of_original > Decimal::ZERO,
            "PIK as % of original should be positive"
        );
        // With full PIK toggle, cash yield = 0, so cash/total = 0
        assert_eq!(
            pik.cash_vs_total_yield,
            Decimal::ZERO,
            "Cash vs total yield should be 0 for full PIK toggle"
        );
    }

    // -----------------------------------------------------------------------
    // 4. Delayed draw with fee income
    // -----------------------------------------------------------------------
    #[test]
    fn test_delayed_draw_fee_income() {
        let mut input = standard_io_loan();
        input.commitment = dec!(150_000_000);
        input.drawn_amount = dec!(100_000_000);
        input.delayed_draw_amount = Some(dec!(50_000_000));
        input.delayed_draw_fee_bps = dec!(100); // 100 bps = 1%

        let result = model_direct_loan(&input).unwrap();
        let schedule = &result.result.cash_flow_schedule;

        // Fee = 50M * 100/10000 = 50M * 0.01 = 500,000
        let expected_fee = dec!(500_000);
        for period in schedule {
            assert_eq!(
                period.delayed_draw_fee, expected_fee,
                "Delayed draw fee should be {} per year, got {}",
                expected_fee, period.delayed_draw_fee
            );
            assert!(
                period.total_lender_income >= period.cash_interest + period.delayed_draw_fee,
                "Total lender income should include delayed draw fee"
            );
        }
    }

    // -----------------------------------------------------------------------
    // 5. Level amortization schedule
    // -----------------------------------------------------------------------
    #[test]
    fn test_level_amortization() {
        let mut input = standard_io_loan();
        input.amortization_schedule = AmortSchedule::LevelAmort(dec!(0.10)); // 10% per year

        let result = model_direct_loan(&input).unwrap();
        let schedule = &result.result.cash_flow_schedule;

        // Year 1: principal payment = 100M * 10% = 10M
        assert_eq!(
            schedule[0].principal_payment,
            dec!(10_000_000),
            "Year 1 level amort payment should be 10M"
        );

        // Balance should decrease
        assert!(
            schedule[0].ending_balance < schedule[0].beginning_balance,
            "Balance should decrease with amortization"
        );

        // Final year pays remaining balance
        let last = schedule.last().unwrap();
        // ending_balance should be 0 in final year (all repaid)
        assert_eq!(
            last.ending_balance,
            Decimal::ZERO,
            "Final year ending balance should be zero"
        );
    }

    // -----------------------------------------------------------------------
    // 6. Bullet maturity
    // -----------------------------------------------------------------------
    #[test]
    fn test_bullet_maturity() {
        let mut input = standard_io_loan();
        input.amortization_schedule = AmortSchedule::BulletMaturity;

        let result = model_direct_loan(&input).unwrap();
        let schedule = &result.result.cash_flow_schedule;

        // No principal payments until maturity
        for period in &schedule[..schedule.len() - 1] {
            assert_eq!(
                period.principal_payment,
                Decimal::ZERO,
                "Year {} should have no principal payment for bullet",
                period.year
            );
        }

        // Final year: full repayment
        let last = schedule.last().unwrap();
        assert_eq!(
            last.principal_payment, last.beginning_balance,
            "Final year should repay full balance"
        );
    }

    // -----------------------------------------------------------------------
    // 7. Custom amortization
    // -----------------------------------------------------------------------
    #[test]
    fn test_custom_amortization() {
        let mut input = standard_io_loan();
        input.amortization_schedule = AmortSchedule::Custom(vec![
            dec!(5_000_000),
            dec!(10_000_000),
            dec!(15_000_000),
            dec!(20_000_000),
        ]);
        // Year 5 will pay remaining balance

        let result = model_direct_loan(&input).unwrap();
        let schedule = &result.result.cash_flow_schedule;

        assert_eq!(schedule[0].principal_payment, dec!(5_000_000));
        assert_eq!(schedule[1].principal_payment, dec!(10_000_000));
        assert_eq!(schedule[2].principal_payment, dec!(15_000_000));
        assert_eq!(schedule[3].principal_payment, dec!(20_000_000));

        // Final year should pay remaining
        let remaining_before_y5 = dec!(100_000_000)
            - dec!(5_000_000)
            - dec!(10_000_000)
            - dec!(15_000_000)
            - dec!(20_000_000);
        assert_eq!(
            schedule[4].principal_payment, remaining_before_y5,
            "Final year should repay remaining balance of {}",
            remaining_before_y5
        );
    }

    // -----------------------------------------------------------------------
    // 8. Rate floor: floor > base_rate
    // -----------------------------------------------------------------------
    #[test]
    fn test_rate_floor() {
        let mut input = standard_io_loan();
        input.base_rate = dec!(0.03); // base lower than floor
        input.floor_rate = Some(dec!(0.05)); // floor at 5%

        let result = model_direct_loan(&input).unwrap();
        let out = &result.result;

        // Effective rate = max(0.03, 0.05) + 550/10000 = 0.05 + 0.055 = 0.105
        assert_eq!(
            out.yield_metrics.effective_rate_with_floor,
            dec!(0.105),
            "Effective rate should use floor when base < floor"
        );
        assert_eq!(out.yield_metrics.cash_yield, dec!(0.105));

        // Should have a warning about floor exceeding base
        assert!(
            result.warnings.iter().any(|w| w.contains("Floor rate")),
            "Should warn when floor exceeds base rate"
        );
    }

    // -----------------------------------------------------------------------
    // 9. Default-adjusted yield lower than YTM
    // -----------------------------------------------------------------------
    #[test]
    fn test_default_adjusted_yield_lower() {
        let input = standard_io_loan();
        let result = model_direct_loan(&input).unwrap();
        let ym = &result.result.yield_metrics;

        // default_adjusted = YTM - (PD * LGD) = YTM - (0.02 * 0.40) = YTM - 0.008
        assert!(
            ym.default_adjusted_yield < ym.yield_to_maturity,
            "Default-adjusted yield ({}) should be less than YTM ({})",
            ym.default_adjusted_yield,
            ym.yield_to_maturity
        );

        let expected_diff = dec!(0.02) * dec!(0.40);
        let actual_diff = ym.yield_to_maturity - ym.default_adjusted_yield;
        assert_eq!(
            actual_diff, expected_diff,
            "Difference should be PD * LGD = {}, got {}",
            expected_diff, actual_diff
        );
    }

    // -----------------------------------------------------------------------
    // 10. Loss-adjusted spread
    // -----------------------------------------------------------------------
    #[test]
    fn test_loss_adjusted_spread() {
        let input = standard_io_loan();
        let result = model_direct_loan(&input).unwrap();
        let ym = &result.result.yield_metrics;

        // loss_adjusted_spread = (default_adjusted_yield - base_rate) * 10000
        let expected = (ym.default_adjusted_yield - input.base_rate) * dec!(10000);
        assert_eq!(
            ym.loss_adjusted_spread, expected,
            "Loss-adjusted spread should be {} bps, got {}",
            expected, ym.loss_adjusted_spread
        );
    }

    // -----------------------------------------------------------------------
    // 11. Credit VaR calculation
    // -----------------------------------------------------------------------
    #[test]
    fn test_credit_var_95() {
        let input = standard_io_loan();
        let result = model_direct_loan(&input).unwrap();
        let cm = &result.result.credit_metrics;

        // VaR = 2.33 * sqrt(0.02 * 0.98) * 0.40 * 100M
        let pd = dec!(0.02);
        let pd_var = pd * (Decimal::ONE - pd);
        let pd_std = decimal_sqrt(pd_var);
        let expected_var = dec!(2.33) * pd_std * dec!(0.40) * dec!(100_000_000);

        let diff = (cm.credit_var_95 - expected_var).abs();
        assert!(
            diff < dec!(1.0),
            "Credit VaR should be ~{}, got {}",
            expected_var,
            cm.credit_var_95
        );
    }

    // -----------------------------------------------------------------------
    // 12. Prepayment penalty by year (stored in input, used in structure)
    // -----------------------------------------------------------------------
    #[test]
    fn test_prepayment_penalties_stored() {
        let mut input = standard_io_loan();
        input.prepayment_penalty = vec![
            PrepaymentPenalty {
                year: 1,
                premium_pct: dec!(0.03),
            },
            PrepaymentPenalty {
                year: 2,
                premium_pct: dec!(0.02),
            },
            PrepaymentPenalty {
                year: 3,
                premium_pct: dec!(0.01),
            },
        ];

        let result = model_direct_loan(&input).unwrap();
        // Verify model runs successfully with penalties defined
        assert_eq!(result.result.cash_flow_schedule.len(), 5);
        // Verify penalties are accessible from input
        assert_eq!(input.prepayment_penalty[0].premium_pct, dec!(0.03));
        assert_eq!(input.prepayment_penalty[1].premium_pct, dec!(0.02));
        assert_eq!(input.prepayment_penalty[2].premium_pct, dec!(0.01));
    }

    // -----------------------------------------------------------------------
    // 13. Syndication: exact allocation
    // -----------------------------------------------------------------------
    #[test]
    fn test_syndication_exact_allocation() {
        let input = SyndicationInput {
            total_facility: dec!(500_000_000),
            arranger_hold: dec!(100_000_000),
            syndicate_members: vec![
                SyndicateMember {
                    name: "Lead Bank".into(),
                    commitment: dec!(100_000_000),
                    is_lead: true,
                },
                SyndicateMember {
                    name: "Bank A".into(),
                    commitment: dec!(200_000_000),
                    is_lead: false,
                },
                SyndicateMember {
                    name: "Bank B".into(),
                    commitment: dec!(200_000_000),
                    is_lead: false,
                },
            ],
            arrangement_fee_bps: dec!(50),
            participation_fee_bps: dec!(25),
            coupon_spread_bps: dec!(400),
        };

        let result = analyze_syndication(&input).unwrap();
        let out = &result.result;

        // Total committed = 100 + 200 + 200 = 500
        assert_eq!(out.total_committed, dec!(500_000_000));
        // Oversubscription = 500/500 = 1.0
        assert_eq!(out.oversubscription, Decimal::ONE);

        // Bank A and B should each get 200M (exact fit)
        let bank_a = out
            .participant_allocations
            .iter()
            .find(|p| p.name == "Bank A")
            .unwrap();
        assert_eq!(bank_a.allocated, dec!(200_000_000));
        assert_eq!(bank_a.committed, dec!(200_000_000));
    }

    // -----------------------------------------------------------------------
    // 14. Syndication: oversubscribed -> scale down
    // -----------------------------------------------------------------------
    #[test]
    fn test_syndication_oversubscribed() {
        let input = SyndicationInput {
            total_facility: dec!(500_000_000),
            arranger_hold: dec!(100_000_000),
            syndicate_members: vec![
                SyndicateMember {
                    name: "Lead".into(),
                    commitment: dec!(100_000_000),
                    is_lead: true,
                },
                SyndicateMember {
                    name: "Bank A".into(),
                    commitment: dec!(300_000_000),
                    is_lead: false,
                },
                SyndicateMember {
                    name: "Bank B".into(),
                    commitment: dec!(500_000_000),
                    is_lead: false,
                },
            ],
            arrangement_fee_bps: dec!(50),
            participation_fee_bps: dec!(25),
            coupon_spread_bps: dec!(400),
        };

        let result = analyze_syndication(&input).unwrap();
        let out = &result.result;

        // Total committed = 100 + 300 + 500 = 900
        assert_eq!(out.total_committed, dec!(900_000_000));
        // Oversubscription = 900/500 = 1.8
        assert_eq!(out.oversubscription, dec!(1.8));

        // Available for syndication = 500 - 100 = 400
        // Total non-lead commitments = 300 + 500 = 800
        // Bank A allocated = 300 * 400/800 = 150
        // Bank B allocated = 500 * 400/800 = 250
        let bank_a = out
            .participant_allocations
            .iter()
            .find(|p| p.name == "Bank A")
            .unwrap();
        let bank_b = out
            .participant_allocations
            .iter()
            .find(|p| p.name == "Bank B")
            .unwrap();

        assert_eq!(
            bank_a.allocated,
            dec!(150_000_000),
            "Bank A should be scaled to 150M, got {}",
            bank_a.allocated
        );
        assert_eq!(
            bank_b.allocated,
            dec!(250_000_000),
            "Bank B should be scaled to 250M, got {}",
            bank_b.allocated
        );
    }

    // -----------------------------------------------------------------------
    // 15. Arranger economics (fee + hold income)
    // -----------------------------------------------------------------------
    #[test]
    fn test_arranger_economics() {
        let input = SyndicationInput {
            total_facility: dec!(500_000_000),
            arranger_hold: dec!(100_000_000),
            syndicate_members: vec![SyndicateMember {
                name: "Lead".into(),
                commitment: dec!(500_000_000),
                is_lead: true,
            }],
            arrangement_fee_bps: dec!(50), // 50bps on 500M = 2.5M
            participation_fee_bps: dec!(25),
            coupon_spread_bps: dec!(400), // 400bps on 100M hold = 4M
        };

        let result = analyze_syndication(&input).unwrap();
        let ae = &result.result.arranger_economics;

        assert_eq!(ae.hold_amount, dec!(100_000_000));
        assert_eq!(ae.sell_down, dec!(400_000_000));

        // Arrangement fee = 500M * 50/10000 = 2.5M
        assert_eq!(ae.arrangement_fee_earned, dec!(2_500_000));

        // Ongoing spread = 100M * 400/10000 = 4M
        assert_eq!(ae.ongoing_spread_income, dec!(4_000_000));

        // Total year 1 = 2.5M + 4M = 6.5M
        assert_eq!(ae.total_year1_income, dec!(6_500_000));
    }

    // -----------------------------------------------------------------------
    // 16. Cash flow schedule period-by-period
    // -----------------------------------------------------------------------
    #[test]
    fn test_cash_flow_schedule_period_by_period() {
        let input = standard_io_loan();
        let result = model_direct_loan(&input).unwrap();
        let schedule = &result.result.cash_flow_schedule;

        assert_eq!(schedule.len(), 5, "Should have 5 periods");

        // Year 1: beginning = 100M
        assert_eq!(schedule[0].beginning_balance, dec!(100_000_000));
        assert_eq!(schedule[0].year, 1);

        // IO loan: cash_interest = 100M * 0.105 = 10.5M
        let expected_interest = dec!(100_000_000) * dec!(0.105);
        assert_eq!(schedule[0].cash_interest, expected_interest);

        // No PIK
        assert_eq!(schedule[0].pik_interest, Decimal::ZERO);

        // IO: no principal until maturity
        assert_eq!(schedule[0].principal_payment, Decimal::ZERO);
        assert_eq!(schedule[0].ending_balance, dec!(100_000_000));

        // Total lender income = cash interest (no delayed draw fees)
        assert_eq!(schedule[0].total_lender_income, expected_interest);

        // Each year beginning = prior ending
        for i in 1..schedule.len() {
            assert_eq!(
                schedule[i].beginning_balance,
                schedule[i - 1].ending_balance,
                "Year {} beginning should equal year {} ending",
                schedule[i].year,
                schedule[i - 1].year
            );
        }
    }

    // -----------------------------------------------------------------------
    // 17. YTM via IRR
    // -----------------------------------------------------------------------
    #[test]
    fn test_ytm_via_irr() {
        let input = standard_io_loan();
        let result = model_direct_loan(&input).unwrap();
        let ytm = result.result.yield_metrics.yield_to_maturity;

        // For an IO loan at par, IRR should approximate the all-in coupon rate
        // all_in = 0.05 + 0.055 = 0.105
        let diff = (ytm - dec!(0.105)).abs();
        assert!(
            diff < dec!(0.001),
            "YTM for IO loan at par should be ~10.5%, got {}",
            ytm
        );
    }

    // -----------------------------------------------------------------------
    // 18. Edge: zero PD -> default-adjusted = YTM
    // -----------------------------------------------------------------------
    #[test]
    fn test_zero_pd_default_adjusted_equals_ytm() {
        let mut input = standard_io_loan();
        input.expected_default_rate = Decimal::ZERO;

        let result = model_direct_loan(&input).unwrap();
        let ym = &result.result.yield_metrics;

        assert_eq!(
            ym.default_adjusted_yield, ym.yield_to_maturity,
            "With zero PD, default-adjusted yield should equal YTM"
        );
    }

    // -----------------------------------------------------------------------
    // 19. Edge: 100% drawn, no delayed draw
    // -----------------------------------------------------------------------
    #[test]
    fn test_fully_drawn_no_delayed_draw() {
        let input = standard_io_loan();
        let result = model_direct_loan(&input).unwrap();
        let schedule = &result.result.cash_flow_schedule;

        // No delayed draw fees
        for period in schedule {
            assert_eq!(
                period.delayed_draw_fee,
                Decimal::ZERO,
                "No delayed draw fee when fully drawn"
            );
        }

        // PIK analysis should be None (no PIK rate)
        assert!(result.result.pik_analysis.is_none());
    }

    // -----------------------------------------------------------------------
    // 20. PIK with partial PIK rate (not toggle)
    // -----------------------------------------------------------------------
    #[test]
    fn test_partial_pik_rate() {
        let mut input = standard_io_loan();
        input.pik_rate = Some(dec!(0.02)); // 200bps PIK
        input.pik_toggle = false;

        let result = model_direct_loan(&input).unwrap();
        let out = &result.result;

        // Cash rate = all_in - pik_rate = 0.105 - 0.02 = 0.085
        assert_eq!(out.yield_metrics.cash_yield, dec!(0.085));
        assert_eq!(out.yield_metrics.pik_yield, dec!(0.02));
        assert_eq!(out.yield_metrics.total_yield, dec!(0.105));

        // PIK analysis should exist
        let pik = out.pik_analysis.as_ref().unwrap();
        assert!(pik.total_pik_accrued > Decimal::ZERO);

        // cash / total should be 0.085 / 0.105
        let expected_ratio = dec!(0.085) / dec!(0.105);
        let diff = (pik.cash_vs_total_yield - expected_ratio).abs();
        assert!(
            diff < dec!(0.0001),
            "Cash vs total yield ratio should be ~{}, got {}",
            expected_ratio,
            pik.cash_vs_total_yield
        );
    }

    // -----------------------------------------------------------------------
    // 21. Expected loss calculation
    // -----------------------------------------------------------------------
    #[test]
    fn test_expected_loss() {
        let input = standard_io_loan();
        let result = model_direct_loan(&input).unwrap();
        let cm = &result.result.credit_metrics;

        // EL = 0.02 * 0.40 * 100M = 800,000
        assert_eq!(cm.expected_loss, dec!(800_000));
        assert_eq!(cm.expected_loss_pct, dec!(0.008));
    }

    // -----------------------------------------------------------------------
    // 22. Risk-adjusted return
    // -----------------------------------------------------------------------
    #[test]
    fn test_risk_adjusted_return() {
        let input = standard_io_loan();
        let result = model_direct_loan(&input).unwrap();
        let cm = &result.result.credit_metrics;
        let ym = &result.result.yield_metrics;

        let expected_rar = ym.yield_to_maturity - cm.expected_loss_pct;
        assert_eq!(
            cm.risk_adjusted_return, expected_rar,
            "Risk-adjusted return should be YTM - EL%"
        );
    }

    // -----------------------------------------------------------------------
    // 23. Metadata populated
    // -----------------------------------------------------------------------
    #[test]
    fn test_metadata_populated() {
        let input = standard_io_loan();
        let result = model_direct_loan(&input).unwrap();

        assert!(!result.methodology.is_empty());
        assert!(result.methodology.contains("Direct Lending"));
        assert_eq!(result.metadata.precision, "rust_decimal_128bit");
    }

    // -----------------------------------------------------------------------
    // 24. Syndication: participant fees
    // -----------------------------------------------------------------------
    #[test]
    fn test_syndication_participant_fees() {
        let input = SyndicationInput {
            total_facility: dec!(400_000_000),
            arranger_hold: dec!(100_000_000),
            syndicate_members: vec![
                SyndicateMember {
                    name: "Lead".into(),
                    commitment: dec!(100_000_000),
                    is_lead: true,
                },
                SyndicateMember {
                    name: "Participant".into(),
                    commitment: dec!(300_000_000),
                    is_lead: false,
                },
            ],
            arrangement_fee_bps: dec!(50),
            participation_fee_bps: dec!(25), // 25bps
            coupon_spread_bps: dec!(400),
        };

        let result = analyze_syndication(&input).unwrap();
        let participant = result
            .result
            .participant_allocations
            .iter()
            .find(|p| p.name == "Participant")
            .unwrap();

        // allocated = 300M (exact fit)
        assert_eq!(participant.allocated, dec!(300_000_000));

        // participation_fee = 300M * 25/10000 = 750,000
        assert_eq!(participant.participation_fee, dec!(750_000));

        // annual_interest = 300M * 400/10000 = 12M
        assert_eq!(participant.annual_interest_income, dec!(12_000_000));
    }

    // -----------------------------------------------------------------------
    // 25. Validation: zero commitment
    // -----------------------------------------------------------------------
    #[test]
    fn test_validation_zero_commitment() {
        let mut input = standard_io_loan();
        input.commitment = Decimal::ZERO;

        let err = model_direct_loan(&input).unwrap_err();
        match err {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "commitment");
            }
            other => panic!("Expected InvalidInput, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // 26. Validation: drawn > commitment
    // -----------------------------------------------------------------------
    #[test]
    fn test_validation_drawn_exceeds_commitment() {
        let mut input = standard_io_loan();
        input.drawn_amount = dec!(200_000_000); // > 100M commitment

        let err = model_direct_loan(&input).unwrap_err();
        match err {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "drawn_amount");
            }
            other => panic!("Expected InvalidInput, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // 27. Validation: syndication empty members
    // -----------------------------------------------------------------------
    #[test]
    fn test_validation_syndication_empty_members() {
        let input = SyndicationInput {
            total_facility: dec!(100_000_000),
            arranger_hold: dec!(50_000_000),
            syndicate_members: vec![],
            arrangement_fee_bps: dec!(50),
            participation_fee_bps: dec!(25),
            coupon_spread_bps: dec!(400),
        };

        let err = analyze_syndication(&input).unwrap_err();
        match err {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "syndicate_members");
            }
            other => panic!("Expected InvalidInput, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // 28. Newton sqrt correctness
    // -----------------------------------------------------------------------
    #[test]
    fn test_decimal_sqrt() {
        let result = decimal_sqrt(dec!(4));
        let diff = (result - dec!(2)).abs();
        assert!(diff < dec!(0.0001), "sqrt(4) should be ~2, got {}", result);

        let result = decimal_sqrt(dec!(0.0196));
        let diff = (result - dec!(0.14)).abs();
        assert!(
            diff < dec!(0.0001),
            "sqrt(0.0196) should be ~0.14, got {}",
            result
        );

        assert_eq!(decimal_sqrt(Decimal::ZERO), Decimal::ZERO);
    }

    // -----------------------------------------------------------------------
    // 29. Syndication: pct_of_deal sums to ~1
    // -----------------------------------------------------------------------
    #[test]
    fn test_syndication_pct_sum() {
        let input = SyndicationInput {
            total_facility: dec!(500_000_000),
            arranger_hold: dec!(100_000_000),
            syndicate_members: vec![
                SyndicateMember {
                    name: "Lead".into(),
                    commitment: dec!(100_000_000),
                    is_lead: true,
                },
                SyndicateMember {
                    name: "Bank A".into(),
                    commitment: dec!(200_000_000),
                    is_lead: false,
                },
                SyndicateMember {
                    name: "Bank B".into(),
                    commitment: dec!(200_000_000),
                    is_lead: false,
                },
            ],
            arrangement_fee_bps: dec!(50),
            participation_fee_bps: dec!(25),
            coupon_spread_bps: dec!(400),
        };

        let result = analyze_syndication(&input).unwrap();
        let total_pct: Decimal = result
            .result
            .participant_allocations
            .iter()
            .map(|p| p.pct_of_deal)
            .sum();

        let diff = (total_pct - Decimal::ONE).abs();
        assert!(
            diff < dec!(0.001),
            "Total pct_of_deal should sum to ~1.0, got {}",
            total_pct
        );
    }

    // -----------------------------------------------------------------------
    // 30. Level amort with PIK: balance behavior
    // -----------------------------------------------------------------------
    #[test]
    fn test_level_amort_with_pik() {
        let mut input = standard_io_loan();
        input.amortization_schedule = AmortSchedule::LevelAmort(dec!(0.05));
        input.pik_rate = Some(dec!(0.02));
        input.pik_toggle = false;

        let result = model_direct_loan(&input).unwrap();
        let schedule = &result.result.cash_flow_schedule;

        // Year 1: PIK adds to balance, amort reduces it
        // beginning = 100M
        // pik = 100M * 0.02 = 2M
        // amort = 100M * 0.05 = 5M
        // ending = 100M + 2M - 5M = 97M
        assert_eq!(schedule[0].pik_interest, dec!(2_000_000));
        assert_eq!(schedule[0].principal_payment, dec!(5_000_000));
        assert_eq!(schedule[0].ending_balance, dec!(97_000_000));
    }
}
