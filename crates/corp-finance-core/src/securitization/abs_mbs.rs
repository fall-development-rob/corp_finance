//! ABS/MBS cash flow modelling with prepayment and default models.
//!
//! Provides institutional-grade securitization analytics including PSA/CPR/SMM
//! prepayment models, CDR/SDA default models, loss severity, recovery lag,
//! servicing fees, and weighted average life (WAL) computation.

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::error::CorpFinanceError;
use crate::types::{with_metadata, ComputationOutput, Money, Rate};
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Prepayment model specification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PrepaymentModel {
    /// Constant Prepayment Rate (annual). E.g., 0.06 = 6% CPR.
    Cpr(Rate),
    /// PSA speed. 100 = 100% PSA (ramps to 6% CPR at month 30, then flat).
    Psa(Decimal),
    /// Single Monthly Mortality (already monthly). E.g., 0.005 = 0.5% SMM.
    Smm(Rate),
}

/// Default model specification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DefaultModel {
    /// Constant Default Rate (annual). E.g., 0.02 = 2% CDR.
    Cdr(Rate),
    /// Standard Default Assumption speed. 100 = 100% SDA.
    Sda(Decimal),
    /// No defaults.
    None,
}

/// Input parameters for ABS/MBS cash flow modelling.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbsMbsInput {
    /// Initial pool balance (unpaid principal balance).
    pub pool_balance: Money,
    /// Weighted average coupon (e.g., 0.055 = 5.5%).
    pub weighted_avg_coupon: Rate,
    /// Weighted average maturity in months.
    pub weighted_avg_maturity_months: u32,
    /// Weighted average loan age in months (WALA).
    pub weighted_avg_age_months: u32,
    /// Number of loans in pool.
    pub num_loans: u32,
    /// Prepayment model.
    pub prepayment_model: PrepaymentModel,
    /// Default model.
    pub default_model: DefaultModel,
    /// Loss given default (e.g., 0.40 = 40%).
    pub loss_severity: Rate,
    /// Months to recover from defaulted loans.
    pub recovery_lag_months: u32,
    /// Annual servicing fee rate (e.g., 0.0025 = 25bps).
    pub servicing_fee_rate: Rate,
    /// Number of months to project.
    pub projection_months: u32,
}

/// A single period in the ABS/MBS cash flow projection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbsPeriod {
    pub month: u32,
    pub beginning_balance: Money,
    pub scheduled_principal: Money,
    pub scheduled_interest: Money,
    pub prepayment: Money,
    pub defaults: Money,
    pub loss: Money,
    pub recovery: Money,
    pub servicing_fee: Money,
    pub total_principal: Money,
    pub total_cashflow: Money,
    pub ending_balance: Money,
    pub smm: Rate,
    pub cpr: Rate,
    pub mdr: Rate,
}

/// Summary statistics for the ABS/MBS projection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbsSummary {
    pub total_principal_collected: Money,
    pub total_interest_collected: Money,
    pub total_prepayments: Money,
    pub total_defaults: Money,
    pub total_losses: Money,
    pub total_recoveries: Money,
    pub total_servicing_fees: Money,
    pub weighted_average_life_years: Decimal,
    pub pool_factor_at_end: Rate,
    pub cumulative_loss_rate: Rate,
    pub total_cashflows: Money,
}

/// Complete ABS/MBS cash flow output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbsMbsOutput {
    pub periods: Vec<AbsPeriod>,
    pub summary: AbsSummary,
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// PSA base CPR at month 30 (6% annual).
const PSA_BASE_CPR_30: Decimal = dec!(0.06);

/// SDA peak CDR at month 30 (0.6% annual).
const SDA_PEAK_CDR_30: Decimal = dec!(0.006);

/// SDA floor CDR from month 121 onward (0.03% annual).
const SDA_FLOOR_CDR: Decimal = dec!(0.0003);

/// Minimum balance threshold below which the pool is considered fully paid.
const BALANCE_EPSILON: Decimal = dec!(0.01);

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Model ABS/MBS cash flows with prepayment and default projections.
///
/// Projects monthly cash flows for a mortgage or asset-backed securities pool,
/// applying the specified prepayment model (CPR/PSA/SMM), default model
/// (CDR/SDA/None), loss severity, recovery lag, and servicing fees.
pub fn model_abs_cashflows(
    input: &AbsMbsInput,
) -> CorpFinanceResult<ComputationOutput<AbsMbsOutput>> {
    let start = Instant::now();
    let mut warnings: Vec<String> = Vec::new();

    validate_input(input)?;

    let wac_monthly = input.weighted_avg_coupon / dec!(12);
    let mut balance = input.pool_balance;
    let mut remaining_months = input.weighted_avg_maturity_months;

    let mut periods: Vec<AbsPeriod> = Vec::with_capacity(input.projection_months as usize);
    // Track defaults per month for lagged recovery.
    let mut defaults_history: Vec<Money> = Vec::with_capacity(input.projection_months as usize);

    // Accumulators for summary.
    let mut total_principal_collected = Decimal::ZERO;
    let mut total_interest_collected = Decimal::ZERO;
    let mut total_prepayments = Decimal::ZERO;
    let mut total_defaults = Decimal::ZERO;
    let mut total_losses = Decimal::ZERO;
    let mut total_recoveries = Decimal::ZERO;
    let mut total_servicing_fees = Decimal::ZERO;
    let mut total_cashflows = Decimal::ZERO;

    // For WAL computation: sum(t * principal_t).
    let mut wal_numerator = Decimal::ZERO;

    for month_idx in 0..input.projection_months {
        let month = month_idx + 1;
        let age = input.weighted_avg_age_months + month;

        if balance < BALANCE_EPSILON || remaining_months == 0 {
            // Pool is exhausted; emit a zero period.
            let period = zero_period(month, balance);
            defaults_history.push(Decimal::ZERO);
            periods.push(period);
            continue;
        }

        let beginning_balance = balance;

        // --- Prepayment rates ---
        let cpr_annual = compute_cpr(age, &input.prepayment_model);
        let smm = cpr_to_smm(cpr_annual);

        // --- Default rates ---
        let cdr_annual = compute_cdr(age, &input.default_model);
        let mdr = cdr_to_mdr(cdr_annual);

        // --- Scheduled payment (standard level-pay amortisation) ---
        let scheduled_interest = beginning_balance * wac_monthly;

        let scheduled_payment = if wac_monthly > Decimal::ZERO {
            let denom =
                Decimal::ONE - iterative_pow_recip(Decimal::ONE + wac_monthly, remaining_months);
            if denom > Decimal::ZERO {
                beginning_balance * wac_monthly / denom
            } else {
                beginning_balance
            }
        } else {
            // Zero coupon: just return principal evenly.
            beginning_balance / Decimal::from(remaining_months)
        };

        let mut scheduled_principal = scheduled_payment - scheduled_interest;
        // Guard: scheduled principal cannot exceed balance.
        if scheduled_principal > beginning_balance {
            scheduled_principal = beginning_balance;
        }
        if scheduled_principal < Decimal::ZERO {
            scheduled_principal = Decimal::ZERO;
        }

        // --- Prepayment ---
        let prepay_base = beginning_balance - scheduled_principal;
        let prepayment = if prepay_base > Decimal::ZERO {
            prepay_base * smm
        } else {
            Decimal::ZERO
        };

        // --- Defaults ---
        let default_base = beginning_balance - scheduled_principal - prepayment;
        let defaults = if default_base > Decimal::ZERO {
            default_base * mdr
        } else {
            Decimal::ZERO
        };

        // --- Loss and recovery ---
        let loss = defaults * input.loss_severity;

        // Recovery comes from defaults that occurred recovery_lag_months ago.
        let recovery = if input.recovery_lag_months > 0 && month > input.recovery_lag_months {
            let lag_idx = (month - input.recovery_lag_months - 1) as usize;
            if lag_idx < defaults_history.len() {
                defaults_history[lag_idx] * (Decimal::ONE - input.loss_severity)
            } else {
                Decimal::ZERO
            }
        } else if input.recovery_lag_months == 0 {
            defaults * (Decimal::ONE - input.loss_severity)
        } else {
            Decimal::ZERO
        };

        defaults_history.push(defaults);

        // --- Servicing fee ---
        let servicing_fee = beginning_balance * input.servicing_fee_rate / dec!(12);

        // --- Total principal ---
        let total_principal = scheduled_principal + prepayment;

        // --- Total cashflow ---
        let total_cashflow = scheduled_interest + total_principal - servicing_fee + recovery;

        // --- Ending balance ---
        let mut ending_balance = beginning_balance - scheduled_principal - prepayment - defaults;
        if ending_balance < Decimal::ZERO {
            if ending_balance.abs() < BALANCE_EPSILON {
                ending_balance = Decimal::ZERO;
            } else {
                warnings.push(format!(
                    "Month {}: ending balance went negative ({}) — clamped to zero",
                    month, ending_balance
                ));
                ending_balance = Decimal::ZERO;
            }
        }

        // --- Accumulate summary ---
        total_principal_collected += total_principal;
        total_interest_collected += scheduled_interest;
        total_prepayments += prepayment;
        total_defaults += defaults;
        total_losses += loss;
        total_recoveries += recovery;
        total_servicing_fees += servicing_fee;
        total_cashflows += total_cashflow;

        // WAL: weight by month in years.
        wal_numerator += Decimal::from(month) * total_principal / dec!(12);

        periods.push(AbsPeriod {
            month,
            beginning_balance,
            scheduled_principal,
            scheduled_interest,
            prepayment,
            defaults,
            loss,
            recovery,
            servicing_fee,
            total_principal,
            total_cashflow,
            ending_balance,
            smm,
            cpr: cpr_annual,
            mdr,
        });

        balance = ending_balance;
        remaining_months = remaining_months.saturating_sub(1);
    }

    // --- WAL ---
    let weighted_average_life_years = if total_principal_collected > Decimal::ZERO {
        wal_numerator / total_principal_collected
    } else {
        Decimal::ZERO
    };

    // --- Pool factor ---
    let pool_factor_at_end = if input.pool_balance > Decimal::ZERO {
        balance / input.pool_balance
    } else {
        Decimal::ZERO
    };

    // --- Cumulative loss rate ---
    let cumulative_loss_rate = if input.pool_balance > Decimal::ZERO {
        total_losses / input.pool_balance
    } else {
        Decimal::ZERO
    };

    let summary = AbsSummary {
        total_principal_collected,
        total_interest_collected,
        total_prepayments,
        total_defaults,
        total_losses,
        total_recoveries,
        total_servicing_fees,
        weighted_average_life_years,
        pool_factor_at_end,
        cumulative_loss_rate,
        total_cashflows,
    };

    let output = AbsMbsOutput { periods, summary };

    let elapsed = start.elapsed().as_micros() as u64;

    Ok(with_metadata(
        "ABS/MBS Cash Flow Model — Amortisation with prepayment/default/recovery projections",
        input,
        warnings,
        elapsed,
        output,
    ))
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

fn validate_input(input: &AbsMbsInput) -> CorpFinanceResult<()> {
    if input.pool_balance <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "pool_balance".into(),
            reason: "Pool balance must be positive".into(),
        });
    }
    if input.weighted_avg_coupon < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "weighted_avg_coupon".into(),
            reason: "WAC cannot be negative".into(),
        });
    }
    if input.weighted_avg_maturity_months == 0 {
        return Err(CorpFinanceError::InvalidInput {
            field: "weighted_avg_maturity_months".into(),
            reason: "WAM must be greater than zero".into(),
        });
    }
    if input.loss_severity < Decimal::ZERO || input.loss_severity > Decimal::ONE {
        return Err(CorpFinanceError::InvalidInput {
            field: "loss_severity".into(),
            reason: "Loss severity must be between 0 and 1".into(),
        });
    }
    if input.servicing_fee_rate < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "servicing_fee_rate".into(),
            reason: "Servicing fee rate cannot be negative".into(),
        });
    }
    if input.projection_months == 0 {
        return Err(CorpFinanceError::InvalidInput {
            field: "projection_months".into(),
            reason: "Projection months must be greater than zero".into(),
        });
    }

    // Validate prepayment model rates.
    match &input.prepayment_model {
        PrepaymentModel::Cpr(rate) => {
            if *rate < Decimal::ZERO || *rate > Decimal::ONE {
                return Err(CorpFinanceError::InvalidInput {
                    field: "prepayment_model.Cpr".into(),
                    reason: "CPR must be between 0 and 1".into(),
                });
            }
        }
        PrepaymentModel::Psa(speed) => {
            if *speed < Decimal::ZERO {
                return Err(CorpFinanceError::InvalidInput {
                    field: "prepayment_model.Psa".into(),
                    reason: "PSA speed must be non-negative".into(),
                });
            }
        }
        PrepaymentModel::Smm(rate) => {
            if *rate < Decimal::ZERO || *rate > Decimal::ONE {
                return Err(CorpFinanceError::InvalidInput {
                    field: "prepayment_model.Smm".into(),
                    reason: "SMM must be between 0 and 1".into(),
                });
            }
        }
    }

    // Validate default model rates.
    match &input.default_model {
        DefaultModel::Cdr(rate) => {
            if *rate < Decimal::ZERO || *rate > Decimal::ONE {
                return Err(CorpFinanceError::InvalidInput {
                    field: "default_model.Cdr".into(),
                    reason: "CDR must be between 0 and 1".into(),
                });
            }
        }
        DefaultModel::Sda(speed) => {
            if *speed < Decimal::ZERO {
                return Err(CorpFinanceError::InvalidInput {
                    field: "default_model.Sda".into(),
                    reason: "SDA speed must be non-negative".into(),
                });
            }
        }
        DefaultModel::None => {}
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Prepayment model helpers
// ---------------------------------------------------------------------------

/// Compute the annualised CPR for a given loan age and prepayment model.
fn compute_cpr(age: u32, model: &PrepaymentModel) -> Rate {
    match model {
        PrepaymentModel::Cpr(cpr) => *cpr,
        PrepaymentModel::Psa(speed) => {
            // PSA: CPR ramps linearly from 0 at month 0 to 6% at month 30, then flat.
            let base_cpr = if age <= 30 {
                PSA_BASE_CPR_30 * Decimal::from(age) / dec!(30)
            } else {
                PSA_BASE_CPR_30
            };
            base_cpr * *speed / dec!(100)
        }
        PrepaymentModel::Smm(smm) => {
            // Convert SMM back to CPR for reporting: CPR = 1 - (1 - SMM)^12.
            smm_to_cpr(*smm)
        }
    }
}

/// Convert annual CPR to single monthly mortality (SMM).
/// SMM = 1 - (1 - CPR)^(1/12)
fn cpr_to_smm(cpr: Rate) -> Rate {
    if cpr <= Decimal::ZERO {
        return Decimal::ZERO;
    }
    if cpr >= Decimal::ONE {
        return Decimal::ONE;
    }
    // SMM = 1 - twelfth_root(1 - CPR)
    let base = Decimal::ONE - cpr;
    Decimal::ONE - nth_root(base, 12)
}

/// Convert SMM back to annualised CPR.
/// CPR = 1 - (1 - SMM)^12
fn smm_to_cpr(smm: Rate) -> Rate {
    if smm <= Decimal::ZERO {
        return Decimal::ZERO;
    }
    if smm >= Decimal::ONE {
        return Decimal::ONE;
    }
    Decimal::ONE - iterative_pow(Decimal::ONE - smm, 12)
}

// ---------------------------------------------------------------------------
// Default model helpers
// ---------------------------------------------------------------------------

/// Compute the annualised CDR for a given loan age and default model.
fn compute_cdr(age: u32, model: &DefaultModel) -> Rate {
    match model {
        DefaultModel::Cdr(cdr) => *cdr,
        DefaultModel::Sda(speed) => {
            // SDA curve:
            //   Months 1-30:   CDR ramps from 0 to 0.6%
            //   Months 31-60:  flat at 0.6%
            //   Months 61-120: declines linearly to 0.03%
            //   Months 121+:   flat at 0.03%
            let base_cdr = if age <= 30 {
                SDA_PEAK_CDR_30 * Decimal::from(age) / dec!(30)
            } else if age <= 60 {
                SDA_PEAK_CDR_30
            } else if age <= 120 {
                // Linear decline from 0.006 at month 60 to 0.0003 at month 120.
                let months_into_decline = Decimal::from(age - 60);
                let decline_range = SDA_PEAK_CDR_30 - SDA_FLOOR_CDR;
                SDA_PEAK_CDR_30 - decline_range * months_into_decline / dec!(60)
            } else {
                SDA_FLOOR_CDR
            };
            base_cdr * *speed / dec!(100)
        }
        DefaultModel::None => Decimal::ZERO,
    }
}

/// Convert annual CDR to monthly default rate (MDR).
/// MDR = 1 - (1 - CDR)^(1/12)
fn cdr_to_mdr(cdr: Rate) -> Rate {
    if cdr <= Decimal::ZERO {
        return Decimal::ZERO;
    }
    if cdr >= Decimal::ONE {
        return Decimal::ONE;
    }
    let base = Decimal::ONE - cdr;
    Decimal::ONE - nth_root(base, 12)
}

// ---------------------------------------------------------------------------
// Decimal math helpers (no f64, no powd)
// ---------------------------------------------------------------------------

/// Compute base^n for a positive integer exponent via iterative multiplication.
fn iterative_pow(base: Decimal, n: u32) -> Decimal {
    let mut result = Decimal::ONE;
    for _ in 0..n {
        result *= base;
    }
    result
}

/// Compute 1 / base^n for a positive integer exponent via iterative multiplication.
/// Returns zero if base^n overflows or is zero.
fn iterative_pow_recip(base: Decimal, n: u32) -> Decimal {
    let pow = iterative_pow(base, n);
    if pow.is_zero() {
        Decimal::ZERO
    } else {
        Decimal::ONE / pow
    }
}

/// Compute the nth root of x using Newton's method.
/// x^(1/n) where n is a positive integer.
///
/// Newton iteration: g_{k+1} = g_k - (g_k^n - x) / (n * g_k^{n-1})
fn nth_root(x: Decimal, n: u32) -> Decimal {
    if x == Decimal::ONE {
        return Decimal::ONE;
    }
    if x == Decimal::ZERO {
        return Decimal::ZERO;
    }
    if n == 0 {
        return Decimal::ONE;
    }
    if n == 1 {
        return x;
    }

    let n_dec = Decimal::from(n);
    let n_minus_1 = n - 1;

    // Initial guess: start near 1 since our inputs are always close to 1
    // (they are of the form (1 - small_rate)).
    let mut guess = Decimal::ONE;

    for _ in 0..40 {
        let g_n_minus_1 = iterative_pow(guess, n_minus_1);
        let g_n = g_n_minus_1 * guess;

        if g_n_minus_1.is_zero() {
            break;
        }

        let delta = (g_n - x) / (n_dec * g_n_minus_1);
        guess -= delta;

        if delta.abs() < dec!(0.0000000000001) {
            break;
        }
    }

    guess
}

/// Create a zero-amount period for when the pool is exhausted.
fn zero_period(month: u32, balance: Money) -> AbsPeriod {
    AbsPeriod {
        month,
        beginning_balance: balance,
        scheduled_principal: Decimal::ZERO,
        scheduled_interest: Decimal::ZERO,
        prepayment: Decimal::ZERO,
        defaults: Decimal::ZERO,
        loss: Decimal::ZERO,
        recovery: Decimal::ZERO,
        servicing_fee: Decimal::ZERO,
        total_principal: Decimal::ZERO,
        total_cashflow: Decimal::ZERO,
        ending_balance: balance,
        smm: Decimal::ZERO,
        cpr: Decimal::ZERO,
        mdr: Decimal::ZERO,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    /// Tolerance for financial comparisons.
    const TOL: Decimal = dec!(0.01);

    /// Helper: build a standard input for testing.
    fn standard_input() -> AbsMbsInput {
        AbsMbsInput {
            pool_balance: dec!(1_000_000),
            weighted_avg_coupon: dec!(0.06),
            weighted_avg_maturity_months: 360,
            weighted_avg_age_months: 0,
            num_loans: 1000,
            prepayment_model: PrepaymentModel::Cpr(dec!(0.0)),
            default_model: DefaultModel::None,
            loss_severity: dec!(0.40),
            recovery_lag_months: 6,
            servicing_fee_rate: dec!(0.0025),
            projection_months: 360,
        }
    }

    fn assert_close(actual: Decimal, expected: Decimal, tol: Decimal, msg: &str) {
        let diff = (actual - expected).abs();
        assert!(
            diff <= tol,
            "{}: expected ~{}, got {} (diff = {})",
            msg,
            expected,
            actual,
            diff
        );
    }

    // -----------------------------------------------------------------------
    // 1. Basic amortisation — no prepayments, no defaults
    // -----------------------------------------------------------------------
    #[test]
    fn test_basic_amortisation_no_prepay_no_defaults() {
        let input = standard_input();
        let result = model_abs_cashflows(&input).unwrap();
        let out = &result.result;

        // First period should have the full starting balance.
        assert_eq!(out.periods[0].beginning_balance, dec!(1_000_000));

        // Ending balance of last period should be near zero.
        let last = out.periods.last().unwrap();
        assert!(
            last.ending_balance < dec!(1.0),
            "Final balance should be near zero, got {}",
            last.ending_balance
        );

        // Total principal collected should equal pool balance (no defaults).
        assert_close(
            out.summary.total_principal_collected,
            dec!(1_000_000),
            dec!(1.0),
            "Total principal collected should equal pool balance",
        );

        // No prepayments.
        assert_eq!(out.summary.total_prepayments, Decimal::ZERO);
        // No defaults.
        assert_eq!(out.summary.total_defaults, Decimal::ZERO);
    }

    // -----------------------------------------------------------------------
    // 2. CPR prepayment model — basic
    // -----------------------------------------------------------------------
    #[test]
    fn test_cpr_prepayment_model() {
        let mut input = standard_input();
        input.prepayment_model = PrepaymentModel::Cpr(dec!(0.06));
        input.projection_months = 120;

        let result = model_abs_cashflows(&input).unwrap();
        let out = &result.result;

        // There should be prepayments in every period.
        assert!(out.summary.total_prepayments > Decimal::ZERO);

        // SMM should be constant across all periods with non-zero balance.
        let first_smm = out.periods[0].smm;
        for p in &out.periods {
            if p.beginning_balance > BALANCE_EPSILON {
                assert_close(
                    p.smm,
                    first_smm,
                    dec!(0.000001),
                    "SMM should be constant for CPR",
                );
            }
        }

        // CPR should be reported as 6%.
        assert_close(
            out.periods[0].cpr,
            dec!(0.06),
            dec!(0.0001),
            "CPR should be 6%",
        );
    }

    // -----------------------------------------------------------------------
    // 3. CPR at various speeds
    // -----------------------------------------------------------------------
    #[test]
    fn test_cpr_various_speeds() {
        for cpr_val in [dec!(0.02), dec!(0.10), dec!(0.20)] {
            let mut input = standard_input();
            input.prepayment_model = PrepaymentModel::Cpr(cpr_val);
            input.projection_months = 60;

            let result = model_abs_cashflows(&input).unwrap();
            let out = &result.result;

            assert!(
                out.summary.total_prepayments > Decimal::ZERO,
                "CPR {} should produce prepayments",
                cpr_val
            );
            assert_close(
                out.periods[0].cpr,
                cpr_val,
                dec!(0.0001),
                &format!("CPR should be {}", cpr_val),
            );
        }
    }

    // -----------------------------------------------------------------------
    // 4. PSA 100% model
    // -----------------------------------------------------------------------
    #[test]
    fn test_psa_100_model() {
        let mut input = standard_input();
        input.prepayment_model = PrepaymentModel::Psa(dec!(100));
        input.projection_months = 60;

        let result = model_abs_cashflows(&input).unwrap();
        let out = &result.result;

        // At month 1 (age=1), CPR = 0.06 * 1/30 = 0.002
        assert_close(
            out.periods[0].cpr,
            dec!(0.002),
            dec!(0.0001),
            "PSA 100 month 1 CPR",
        );

        // At month 30 (age=30), CPR = 0.06
        assert_close(
            out.periods[29].cpr,
            dec!(0.06),
            dec!(0.0001),
            "PSA 100 month 30 CPR",
        );

        // At month 40 (age=40), CPR still = 0.06 (flat after 30)
        assert_close(
            out.periods[39].cpr,
            dec!(0.06),
            dec!(0.0001),
            "PSA 100 month 40 CPR",
        );
    }

    // -----------------------------------------------------------------------
    // 5. PSA 200% model (faster prepayments)
    // -----------------------------------------------------------------------
    #[test]
    fn test_psa_200_model() {
        let mut input = standard_input();
        input.prepayment_model = PrepaymentModel::Psa(dec!(200));
        input.projection_months = 60;

        let result = model_abs_cashflows(&input).unwrap();
        let out = &result.result;

        // At month 30 (age=30), CPR = 0.06 * 200/100 = 0.12
        assert_close(
            out.periods[29].cpr,
            dec!(0.12),
            dec!(0.0001),
            "PSA 200 month 30 CPR",
        );

        // Higher PSA should produce more prepayments than PSA 100.
        let mut input_100 = standard_input();
        input_100.prepayment_model = PrepaymentModel::Psa(dec!(100));
        input_100.projection_months = 60;
        let result_100 = model_abs_cashflows(&input_100).unwrap();

        assert!(
            out.summary.total_prepayments > result_100.result.summary.total_prepayments,
            "PSA 200 should have more prepayments than PSA 100"
        );
    }

    // -----------------------------------------------------------------------
    // 6. PSA 50% model (slower prepayments)
    // -----------------------------------------------------------------------
    #[test]
    fn test_psa_50_model() {
        let mut input = standard_input();
        input.prepayment_model = PrepaymentModel::Psa(dec!(50));
        input.projection_months = 60;

        let result = model_abs_cashflows(&input).unwrap();
        let out = &result.result;

        // At month 30 (age=30), CPR = 0.06 * 50/100 = 0.03
        assert_close(
            out.periods[29].cpr,
            dec!(0.03),
            dec!(0.0001),
            "PSA 50 month 30 CPR",
        );
    }

    // -----------------------------------------------------------------------
    // 7. SDA default model with loss severity
    // -----------------------------------------------------------------------
    #[test]
    fn test_sda_default_model() {
        let mut input = standard_input();
        input.default_model = DefaultModel::Sda(dec!(100));
        input.projection_months = 120;

        let result = model_abs_cashflows(&input).unwrap();
        let out = &result.result;

        assert!(
            out.summary.total_defaults > Decimal::ZERO,
            "SDA 100 should produce defaults"
        );
        assert!(
            out.summary.total_losses > Decimal::ZERO,
            "SDA 100 should produce losses"
        );

        // Losses = defaults * severity, accumulated.
        // Check that total_losses / total_defaults is close to loss severity.
        if out.summary.total_defaults > Decimal::ZERO {
            let avg_severity = out.summary.total_losses / out.summary.total_defaults;
            assert_close(
                avg_severity,
                dec!(0.40),
                dec!(0.01),
                "Average loss severity should equal input loss severity",
            );
        }
    }

    // -----------------------------------------------------------------------
    // 8. CDR constant default model
    // -----------------------------------------------------------------------
    #[test]
    fn test_cdr_constant_default_model() {
        let mut input = standard_input();
        input.default_model = DefaultModel::Cdr(dec!(0.02));
        input.projection_months = 60;

        let result = model_abs_cashflows(&input).unwrap();
        let out = &result.result;

        assert!(
            out.summary.total_defaults > Decimal::ZERO,
            "CDR 2% should produce defaults"
        );

        // MDR should be constant.
        let first_mdr = out.periods[0].mdr;
        for p in &out.periods {
            if p.beginning_balance > BALANCE_EPSILON {
                assert_close(
                    p.mdr,
                    first_mdr,
                    dec!(0.000001),
                    "MDR should be constant for CDR",
                );
            }
        }
    }

    // -----------------------------------------------------------------------
    // 9. Combined prepayment + default models
    // -----------------------------------------------------------------------
    #[test]
    fn test_combined_prepayment_and_default() {
        let mut input = standard_input();
        input.prepayment_model = PrepaymentModel::Psa(dec!(150));
        input.default_model = DefaultModel::Cdr(dec!(0.03));
        input.projection_months = 120;

        let result = model_abs_cashflows(&input).unwrap();
        let out = &result.result;

        assert!(out.summary.total_prepayments > Decimal::ZERO);
        assert!(out.summary.total_defaults > Decimal::ZERO);
        assert!(out.summary.total_losses > Decimal::ZERO);

        // Balance should decrease monotonically.
        for window in out.periods.windows(2) {
            assert!(
                window[1].beginning_balance <= window[0].beginning_balance + TOL,
                "Balance should decrease: month {} = {}, month {} = {}",
                window[0].month,
                window[0].beginning_balance,
                window[1].month,
                window[1].beginning_balance,
            );
        }
    }

    // -----------------------------------------------------------------------
    // 10. WAL calculation accuracy
    // -----------------------------------------------------------------------
    #[test]
    fn test_wal_calculation() {
        let input = standard_input();
        let result = model_abs_cashflows(&input).unwrap();
        let out = &result.result;

        // For a standard 30-year level-pay mortgage at 6%, WAL is around 14 years.
        // Without prepayments or defaults, WAL should be a reasonable value.
        assert!(
            out.summary.weighted_average_life_years > dec!(10),
            "WAL should be > 10 years, got {}",
            out.summary.weighted_average_life_years
        );
        assert!(
            out.summary.weighted_average_life_years < dec!(20),
            "WAL should be < 20 years, got {}",
            out.summary.weighted_average_life_years
        );

        // With higher prepayment, WAL should be shorter.
        let mut input_fast = standard_input();
        input_fast.prepayment_model = PrepaymentModel::Cpr(dec!(0.15));
        let result_fast = model_abs_cashflows(&input_fast).unwrap();
        assert!(
            result_fast.result.summary.weighted_average_life_years
                < out.summary.weighted_average_life_years,
            "Higher prepayment should produce shorter WAL"
        );
    }

    // -----------------------------------------------------------------------
    // 11. Pool factor calculation
    // -----------------------------------------------------------------------
    #[test]
    fn test_pool_factor() {
        let input = standard_input();
        let result = model_abs_cashflows(&input).unwrap();
        let out = &result.result;

        // After full amortisation, pool factor should be near zero.
        assert!(
            out.summary.pool_factor_at_end < dec!(0.001),
            "Pool factor at end should be near zero, got {}",
            out.summary.pool_factor_at_end
        );
    }

    // -----------------------------------------------------------------------
    // 12. Pool factor with early projection cutoff
    // -----------------------------------------------------------------------
    #[test]
    fn test_pool_factor_partial_projection() {
        let mut input = standard_input();
        input.projection_months = 60;

        let result = model_abs_cashflows(&input).unwrap();
        let out = &result.result;

        // After only 60 months of a 360-month pool, significant balance remains.
        assert!(
            out.summary.pool_factor_at_end > dec!(0.5),
            "Pool factor after 60 months should be > 50%, got {}",
            out.summary.pool_factor_at_end
        );
    }

    // -----------------------------------------------------------------------
    // 13. Recovery lag mechanics
    // -----------------------------------------------------------------------
    #[test]
    fn test_recovery_lag() {
        let mut input = standard_input();
        input.default_model = DefaultModel::Cdr(dec!(0.05));
        input.recovery_lag_months = 6;
        input.projection_months = 24;

        let result = model_abs_cashflows(&input).unwrap();
        let out = &result.result;

        // Recoveries should be zero for the first 6 months.
        for i in 0..6 {
            assert_eq!(
                out.periods[i].recovery,
                Decimal::ZERO,
                "Month {} should have zero recovery (lag = 6)",
                i + 1
            );
        }

        // After the lag, recoveries should appear.
        assert!(
            out.periods[6].recovery > Decimal::ZERO,
            "Month 7 should have non-zero recovery"
        );
    }

    // -----------------------------------------------------------------------
    // 14. Recovery lag of zero means immediate recovery
    // -----------------------------------------------------------------------
    #[test]
    fn test_recovery_lag_zero() {
        let mut input = standard_input();
        input.default_model = DefaultModel::Cdr(dec!(0.05));
        input.recovery_lag_months = 0;
        input.projection_months = 12;

        let result = model_abs_cashflows(&input).unwrap();
        let out = &result.result;

        // With zero lag, recoveries should appear in month 1.
        assert!(
            out.periods[0].recovery > Decimal::ZERO,
            "Month 1 should have immediate recovery with lag=0"
        );

        // Recovery should be defaults * (1 - loss_severity) for each period.
        for p in &out.periods {
            if p.defaults > Decimal::ZERO {
                let expected_recovery = p.defaults * (Decimal::ONE - dec!(0.40));
                assert_close(
                    p.recovery,
                    expected_recovery,
                    dec!(0.01),
                    &format!("Month {} recovery", p.month),
                );
            }
        }
    }

    // -----------------------------------------------------------------------
    // 15. Servicing fee deduction
    // -----------------------------------------------------------------------
    #[test]
    fn test_servicing_fee() {
        let input = standard_input();
        let result = model_abs_cashflows(&input).unwrap();
        let out = &result.result;

        // First month servicing fee = 1,000,000 * 0.0025 / 12 = ~208.33
        let expected_fee = dec!(1_000_000) * dec!(0.0025) / dec!(12);
        assert_close(
            out.periods[0].servicing_fee,
            expected_fee,
            dec!(0.01),
            "First month servicing fee",
        );

        // Total servicing fees should be positive.
        assert!(
            out.summary.total_servicing_fees > Decimal::ZERO,
            "Total servicing fees should be positive"
        );
    }

    // -----------------------------------------------------------------------
    // 16. Zero prepayment rate
    // -----------------------------------------------------------------------
    #[test]
    fn test_zero_prepayment() {
        let mut input = standard_input();
        input.prepayment_model = PrepaymentModel::Cpr(dec!(0.0));
        input.projection_months = 360;

        let result = model_abs_cashflows(&input).unwrap();
        let out = &result.result;

        assert_eq!(out.summary.total_prepayments, Decimal::ZERO);
        for p in &out.periods {
            assert_eq!(p.prepayment, Decimal::ZERO);
            assert_eq!(p.smm, Decimal::ZERO);
        }
    }

    // -----------------------------------------------------------------------
    // 17. Very high prepayment (near 100% CPR)
    // -----------------------------------------------------------------------
    #[test]
    fn test_very_high_prepayment() {
        let mut input = standard_input();
        input.prepayment_model = PrepaymentModel::Cpr(dec!(0.95));
        input.projection_months = 60;

        let result = model_abs_cashflows(&input).unwrap();
        let out = &result.result;

        // Pool should be mostly paid off quickly.
        let last_period = out.periods.last().unwrap();
        assert!(
            last_period.ending_balance < dec!(100),
            "Very high CPR should exhaust pool quickly, got {}",
            last_period.ending_balance
        );

        // WAL should be very short.
        assert!(
            out.summary.weighted_average_life_years < dec!(2.0),
            "WAL with 95% CPR should be very short, got {}",
            out.summary.weighted_average_life_years
        );
    }

    // -----------------------------------------------------------------------
    // 18. High default rate
    // -----------------------------------------------------------------------
    #[test]
    fn test_high_default_rate() {
        let mut input = standard_input();
        input.default_model = DefaultModel::Cdr(dec!(0.20));
        input.projection_months = 60;

        let result = model_abs_cashflows(&input).unwrap();
        let out = &result.result;

        assert!(
            out.summary.total_defaults > Decimal::ZERO,
            "20% CDR should produce significant defaults"
        );
        assert!(
            out.summary.cumulative_loss_rate > dec!(0.01),
            "Cumulative loss rate should be significant"
        );
    }

    // -----------------------------------------------------------------------
    // 19. Balance should never go negative
    // -----------------------------------------------------------------------
    #[test]
    fn test_balance_never_negative() {
        // Test with aggressive combined prepayment and defaults.
        let mut input = standard_input();
        input.prepayment_model = PrepaymentModel::Cpr(dec!(0.30));
        input.default_model = DefaultModel::Cdr(dec!(0.10));
        input.projection_months = 120;

        let result = model_abs_cashflows(&input).unwrap();
        let out = &result.result;

        for p in &out.periods {
            assert!(
                p.ending_balance >= Decimal::ZERO,
                "Month {}: ending balance should not be negative, got {}",
                p.month,
                p.ending_balance
            );
            assert!(
                p.beginning_balance >= Decimal::ZERO,
                "Month {}: beginning balance should not be negative, got {}",
                p.month,
                p.beginning_balance
            );
        }
    }

    // -----------------------------------------------------------------------
    // 20. Sum of principal + defaults = starting balance
    // -----------------------------------------------------------------------
    #[test]
    fn test_principal_plus_defaults_equals_starting_balance() {
        let mut input = standard_input();
        input.prepayment_model = PrepaymentModel::Psa(dec!(150));
        input.default_model = DefaultModel::Cdr(dec!(0.03));
        input.projection_months = 360;

        let result = model_abs_cashflows(&input).unwrap();
        let out = &result.result;

        // Total principal collected + total defaults + remaining balance = pool balance.
        let total_reduction = out.summary.total_principal_collected
            + out.summary.total_defaults
            + out.periods.last().unwrap().ending_balance;

        assert_close(
            total_reduction,
            dec!(1_000_000),
            dec!(1.0),
            "Principal + defaults + remaining balance should equal pool balance",
        );
    }

    // -----------------------------------------------------------------------
    // 21. SMM model — direct monthly mortality
    // -----------------------------------------------------------------------
    #[test]
    fn test_smm_prepayment_model() {
        let mut input = standard_input();
        input.prepayment_model = PrepaymentModel::Smm(dec!(0.005));
        input.projection_months = 24;

        let result = model_abs_cashflows(&input).unwrap();
        let out = &result.result;

        // SMM should be the direct value.
        assert_close(
            out.periods[0].smm,
            dec!(0.005),
            dec!(0.0001),
            "SMM should be 0.005",
        );

        assert!(out.summary.total_prepayments > Decimal::ZERO);
    }

    // -----------------------------------------------------------------------
    // 22. Cumulative loss rate
    // -----------------------------------------------------------------------
    #[test]
    fn test_cumulative_loss_rate() {
        let mut input = standard_input();
        input.default_model = DefaultModel::Cdr(dec!(0.04));
        input.projection_months = 120;

        let result = model_abs_cashflows(&input).unwrap();
        let out = &result.result;

        // cumulative_loss_rate = total_losses / pool_balance.
        let expected = out.summary.total_losses / dec!(1_000_000);
        assert_close(
            out.summary.cumulative_loss_rate,
            expected,
            dec!(0.0001),
            "Cumulative loss rate",
        );
    }

    // -----------------------------------------------------------------------
    // 23. Interest calculation in first month
    // -----------------------------------------------------------------------
    #[test]
    fn test_interest_first_month() {
        let input = standard_input();
        let result = model_abs_cashflows(&input).unwrap();
        let out = &result.result;

        // First month interest = 1,000,000 * 0.06 / 12 = 5,000
        let expected_interest = dec!(1_000_000) * dec!(0.06) / dec!(12);
        assert_close(
            out.periods[0].scheduled_interest,
            expected_interest,
            dec!(0.01),
            "First month scheduled interest",
        );
    }

    // -----------------------------------------------------------------------
    // 24. Total cashflow includes all components
    // -----------------------------------------------------------------------
    #[test]
    fn test_total_cashflow_composition() {
        let mut input = standard_input();
        input.default_model = DefaultModel::Cdr(dec!(0.02));
        input.recovery_lag_months = 0;
        input.prepayment_model = PrepaymentModel::Cpr(dec!(0.05));
        input.projection_months = 24;

        let result = model_abs_cashflows(&input).unwrap();
        let out = &result.result;

        for p in &out.periods {
            let expected_cf =
                p.scheduled_interest + p.total_principal - p.servicing_fee + p.recovery;
            assert_close(
                p.total_cashflow,
                expected_cf,
                dec!(0.01),
                &format!("Month {} total cashflow composition", p.month),
            );
        }
    }

    // -----------------------------------------------------------------------
    // 25. Validation: negative pool balance
    // -----------------------------------------------------------------------
    #[test]
    fn test_validation_negative_pool_balance() {
        let mut input = standard_input();
        input.pool_balance = dec!(-100);

        let result = model_abs_cashflows(&input);
        assert!(result.is_err());
        match result.unwrap_err() {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "pool_balance");
            }
            other => panic!("Expected InvalidInput, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // 26. Validation: invalid loss severity
    // -----------------------------------------------------------------------
    #[test]
    fn test_validation_loss_severity_out_of_range() {
        let mut input = standard_input();
        input.loss_severity = dec!(1.5);

        let result = model_abs_cashflows(&input);
        assert!(result.is_err());
        match result.unwrap_err() {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "loss_severity");
            }
            other => panic!("Expected InvalidInput, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // 27. Metadata is populated
    // -----------------------------------------------------------------------
    #[test]
    fn test_metadata_populated() {
        let input = standard_input();
        let result = model_abs_cashflows(&input).unwrap();

        assert!(!result.methodology.is_empty());
        assert!(result.methodology.contains("ABS/MBS"));
        assert_eq!(result.metadata.precision, "rust_decimal_128bit");
    }

    // -----------------------------------------------------------------------
    // 28. Correct number of periods
    // -----------------------------------------------------------------------
    #[test]
    fn test_correct_number_of_periods() {
        let mut input = standard_input();
        input.projection_months = 120;

        let result = model_abs_cashflows(&input).unwrap();
        let out = &result.result;

        assert_eq!(out.periods.len(), 120);
        assert_eq!(out.periods[0].month, 1);
        assert_eq!(out.periods[119].month, 120);
    }

    // -----------------------------------------------------------------------
    // 29. SDA curve shape verification
    // -----------------------------------------------------------------------
    #[test]
    fn test_sda_curve_shape() {
        // Verify the SDA curve ramps, peaks, and declines correctly.
        // Age 1: CDR = 0.006 * 1/30 = 0.0002
        let cdr_1 = compute_cdr(1, &DefaultModel::Sda(dec!(100)));
        assert_close(cdr_1, dec!(0.0002), dec!(0.00001), "SDA age 1 CDR");

        // Age 30: CDR = 0.006
        let cdr_30 = compute_cdr(30, &DefaultModel::Sda(dec!(100)));
        assert_close(cdr_30, dec!(0.006), dec!(0.00001), "SDA age 30 CDR");

        // Age 45: CDR = 0.006 (flat 31-60)
        let cdr_45 = compute_cdr(45, &DefaultModel::Sda(dec!(100)));
        assert_close(cdr_45, dec!(0.006), dec!(0.00001), "SDA age 45 CDR");

        // Age 90: CDR = 0.006 - (0.006 - 0.0003) * 30/60 = 0.006 - 0.00285 = 0.00315
        let cdr_90 = compute_cdr(90, &DefaultModel::Sda(dec!(100)));
        assert_close(cdr_90, dec!(0.00315), dec!(0.0001), "SDA age 90 CDR");

        // Age 120: CDR = 0.0003
        let cdr_120 = compute_cdr(120, &DefaultModel::Sda(dec!(100)));
        assert_close(cdr_120, dec!(0.0003), dec!(0.00001), "SDA age 120 CDR");

        // Age 150: CDR = 0.0003 (floor)
        let cdr_150 = compute_cdr(150, &DefaultModel::Sda(dec!(100)));
        assert_close(cdr_150, dec!(0.0003), dec!(0.00001), "SDA age 150 CDR");
    }

    // -----------------------------------------------------------------------
    // 30. WALA offset works correctly
    // -----------------------------------------------------------------------
    #[test]
    fn test_wala_offset() {
        let mut input = standard_input();
        input.weighted_avg_age_months = 24;
        input.prepayment_model = PrepaymentModel::Psa(dec!(100));
        input.projection_months = 36;

        let result = model_abs_cashflows(&input).unwrap();
        let out = &result.result;

        // Month 1 of projection, age = 24 + 1 = 25.
        // CPR at age 25 = 0.06 * 25/30 = 0.05
        assert_close(
            out.periods[0].cpr,
            dec!(0.05),
            dec!(0.001),
            "PSA CPR at age 25",
        );

        // Month 6 of projection, age = 24 + 6 = 30 => CPR = 0.06
        assert_close(
            out.periods[5].cpr,
            dec!(0.06),
            dec!(0.001),
            "PSA CPR at age 30",
        );
    }

    // -----------------------------------------------------------------------
    // 31. Zero WAC pool (interest-free)
    // -----------------------------------------------------------------------
    #[test]
    fn test_zero_coupon_pool() {
        let mut input = standard_input();
        input.weighted_avg_coupon = dec!(0.0);
        input.projection_months = 360;

        let result = model_abs_cashflows(&input).unwrap();
        let out = &result.result;

        // All interest should be zero.
        assert_eq!(out.summary.total_interest_collected, Decimal::ZERO);
        for p in &out.periods {
            assert_eq!(p.scheduled_interest, Decimal::ZERO);
        }

        // But principal should still be amortised.
        assert_close(
            out.summary.total_principal_collected,
            dec!(1_000_000),
            dec!(1.0),
            "Zero coupon pool should still amortise",
        );
    }

    // -----------------------------------------------------------------------
    // 32. Validation: zero projection months
    // -----------------------------------------------------------------------
    #[test]
    fn test_validation_zero_projection_months() {
        let mut input = standard_input();
        input.projection_months = 0;

        let result = model_abs_cashflows(&input);
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // 33. Nth root helper — 12th root of common values
    // -----------------------------------------------------------------------
    #[test]
    fn test_nth_root_precision() {
        // 12th root of 0.94 (i.e., (1-0.06)^(1/12))
        let root = nth_root(dec!(0.94), 12);
        // Expected: 0.94^(1/12) ~ 0.994845
        // Verify: root^12 should be close to 0.94
        let reconstructed = iterative_pow(root, 12);
        assert_close(
            reconstructed,
            dec!(0.94),
            dec!(0.000001),
            "12th root of 0.94 reconstruction",
        );

        // 12th root of 0.98
        let root2 = nth_root(dec!(0.98), 12);
        let reconstructed2 = iterative_pow(root2, 12);
        assert_close(
            reconstructed2,
            dec!(0.98),
            dec!(0.000001),
            "12th root of 0.98 reconstruction",
        );
    }

    // -----------------------------------------------------------------------
    // 34. CPR to SMM and back round-trip
    // -----------------------------------------------------------------------
    #[test]
    fn test_cpr_smm_round_trip() {
        for cpr in [dec!(0.02), dec!(0.06), dec!(0.10), dec!(0.20)] {
            let smm = cpr_to_smm(cpr);
            let cpr_back = smm_to_cpr(smm);
            assert_close(
                cpr_back,
                cpr,
                dec!(0.0001),
                &format!("CPR-SMM round trip for CPR={}", cpr),
            );
        }
    }
}
