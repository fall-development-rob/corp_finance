//! Mortgage prepayment models: PSA, CPR, and Refinancing Incentive.
//!
//! Provides institutional-grade prepayment modelling with full amortisation
//! schedules, SMM conversion, WAL computation, and burnout-adjusted
//! refinancing incentive analysis. All math in `rust_decimal::Decimal`.

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::error::CorpFinanceError;
use crate::types::{with_metadata, ComputationOutput, Money, Rate};
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// PSA base CPR at month 30 (6% annual).
const PSA_BASE_CPR_30: Decimal = dec!(0.06);

/// Minimum balance threshold below which the loan is considered fully paid.
const BALANCE_EPSILON: Decimal = dec!(0.01);

// ---------------------------------------------------------------------------
// Input types
// ---------------------------------------------------------------------------

/// PSA (Public Securities Association) prepayment model input.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PsaInput {
    /// PSA speed as percentage (e.g., 150 for 150% PSA).
    pub psa_speed: Decimal,
    /// Current loan age in months.
    pub loan_age_months: u32,
    /// Remaining months to maturity.
    pub remaining_months: u32,
    /// Original loan balance.
    pub original_balance: Money,
    /// Current outstanding balance.
    pub current_balance: Money,
    /// Annual mortgage rate (e.g., 0.06 = 6%).
    pub mortgage_rate: Rate,
}

/// Constant CPR prepayment model input.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CprInput {
    /// Annual conditional prepayment rate (e.g., 0.06 = 6% CPR).
    pub annual_cpr: Rate,
    /// Current loan age in months.
    pub loan_age_months: u32,
    /// Remaining months to maturity.
    pub remaining_months: u32,
    /// Original loan balance.
    pub original_balance: Money,
    /// Current outstanding balance.
    pub current_balance: Money,
    /// Annual mortgage rate (e.g., 0.06 = 6%).
    pub mortgage_rate: Rate,
}

/// Refinancing incentive prepayment model input.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefinancingInput {
    /// Annual mortgage rate on existing loans.
    pub mortgage_rate: Rate,
    /// Current market rate for new mortgages.
    pub market_rate: Rate,
    /// Base CPR when no incentive exists.
    pub base_cpr: Rate,
    /// Multiplier applied to the rate differential.
    pub incentive_multiplier: Decimal,
    /// Burnout factor: fraction by which CPR decays per month (e.g., 0.01 = 1%).
    pub burnout_factor: Decimal,
    /// Current loan age in months.
    pub loan_age_months: u32,
    /// Remaining months to maturity.
    pub remaining_months: u32,
    /// Original loan balance.
    pub original_balance: Money,
    /// Current outstanding balance.
    pub current_balance: Money,
}

/// Prepayment model selector.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PrepaymentModel {
    Psa(PsaInput),
    Cpr(CprInput),
    Refinancing(RefinancingInput),
}

/// Top-level prepayment analysis input.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrepaymentInput {
    pub model: PrepaymentModel,
}

// ---------------------------------------------------------------------------
// Output types
// ---------------------------------------------------------------------------

/// PSA model output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PsaOutput {
    /// Monthly CPR values.
    pub cpr_schedule: Vec<Rate>,
    /// Monthly SMM values.
    pub smm_schedule: Vec<Rate>,
    /// Projected remaining balance at end of each month.
    pub projected_balances: Vec<Money>,
    /// Monthly prepayment amounts.
    pub projected_prepayments: Vec<Money>,
    /// Total prepayment over the projection.
    pub total_prepayment: Money,
    /// Weighted average life in years.
    pub weighted_average_life: Decimal,
    /// Month at which the balance reaches zero (or remaining_months if not).
    pub expected_maturity_months: u32,
}

/// Constant CPR model output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CprOutput {
    /// Monthly CPR values (constant).
    pub cpr_schedule: Vec<Rate>,
    /// Monthly SMM values (constant).
    pub smm_schedule: Vec<Rate>,
    /// Projected remaining balance at end of each month.
    pub projected_balances: Vec<Money>,
    /// Monthly prepayment amounts.
    pub projected_prepayments: Vec<Money>,
    /// Total prepayment over the projection.
    pub total_prepayment: Money,
    /// Weighted average life in years.
    pub weighted_average_life: Decimal,
    /// Month at which the balance reaches zero (or remaining_months if not).
    pub expected_maturity_months: u32,
}

/// Refinancing incentive model output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefinancingOutput {
    /// Monthly CPR values (burnout-adjusted).
    pub cpr_schedule: Vec<Rate>,
    /// Monthly SMM values.
    pub smm_schedule: Vec<Rate>,
    /// Projected remaining balance at end of each month.
    pub projected_balances: Vec<Money>,
    /// Monthly prepayment amounts (burnout-adjusted).
    pub projected_prepayments: Vec<Money>,
    /// Total prepayment over the projection.
    pub total_prepayment: Money,
    /// Weighted average life in years.
    pub weighted_average_life: Decimal,
    /// Month at which the balance reaches zero (or remaining_months if not).
    pub expected_maturity_months: u32,
    /// The unadjusted incentive CPR before burnout.
    pub base_incentive_cpr: Rate,
}

/// Unified prepayment output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PrepaymentOutput {
    Psa(PsaOutput),
    Cpr(CprOutput),
    Refinancing(RefinancingOutput),
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Analyse mortgage prepayment using the specified model.
pub fn analyze_prepayment(
    input: &PrepaymentInput,
) -> CorpFinanceResult<ComputationOutput<PrepaymentOutput>> {
    let start = Instant::now();

    let (output, methodology, warnings) = match &input.model {
        PrepaymentModel::Psa(psa) => {
            let (out, w) = compute_psa(psa)?;
            (PrepaymentOutput::Psa(out), "PSA Prepayment Model", w)
        }
        PrepaymentModel::Cpr(cpr) => {
            let (out, w) = compute_cpr_model(cpr)?;
            (
                PrepaymentOutput::Cpr(out),
                "Constant CPR Prepayment Model",
                w,
            )
        }
        PrepaymentModel::Refinancing(refi) => {
            let (out, w) = compute_refinancing(refi)?;
            (
                PrepaymentOutput::Refinancing(out),
                "Refinancing Incentive Prepayment Model",
                w,
            )
        }
    };

    let elapsed = start.elapsed().as_micros() as u64;
    Ok(with_metadata(methodology, input, warnings, elapsed, output))
}

// ---------------------------------------------------------------------------
// PSA model
// ---------------------------------------------------------------------------

fn compute_psa(input: &PsaInput) -> CorpFinanceResult<(PsaOutput, Vec<String>)> {
    let mut warnings: Vec<String> = Vec::new();
    validate_psa(input)?;

    if input.psa_speed > dec!(500) {
        warnings.push(format!(
            "PSA speed {}% is unusually high; results may be unreliable",
            input.psa_speed
        ));
    }

    let monthly_rate = input.mortgage_rate / dec!(12);
    let mut balance = input.current_balance;
    let mut remaining = input.remaining_months;

    let mut cpr_schedule = Vec::with_capacity(input.remaining_months as usize);
    let mut smm_schedule = Vec::with_capacity(input.remaining_months as usize);
    let mut projected_balances = Vec::with_capacity(input.remaining_months as usize);
    let mut projected_prepayments = Vec::with_capacity(input.remaining_months as usize);

    let mut total_prepayment = Decimal::ZERO;
    let mut wal_numerator = Decimal::ZERO;
    let mut total_principal = Decimal::ZERO;
    let mut expected_maturity = input.remaining_months;
    let mut maturity_found = false;

    for month_idx in 0..input.remaining_months {
        let age = input.loan_age_months + month_idx + 1;

        if balance < BALANCE_EPSILON || remaining == 0 {
            cpr_schedule.push(Decimal::ZERO);
            smm_schedule.push(Decimal::ZERO);
            projected_balances.push(balance);
            projected_prepayments.push(Decimal::ZERO);
            if !maturity_found {
                expected_maturity = month_idx;
                maturity_found = true;
            }
            continue;
        }

        // PSA CPR: ramps 0.2%/month for months 1-30, flat 6% thereafter, scaled by speed.
        let base_cpr = if age <= 30 {
            PSA_BASE_CPR_30 * Decimal::from(age) / dec!(30)
        } else {
            PSA_BASE_CPR_30
        };
        let cpr = base_cpr * input.psa_speed / dec!(100);
        let cpr_capped = if cpr > Decimal::ONE {
            Decimal::ONE
        } else {
            cpr
        };
        let smm = cpr_to_smm(cpr_capped);

        // Scheduled principal from level-pay amortisation.
        let sched_principal = compute_scheduled_principal(balance, monthly_rate, remaining);

        // Prepayment on remaining balance after scheduled principal.
        let prepay_base = balance - sched_principal;
        let prepayment = if prepay_base > Decimal::ZERO {
            prepay_base * smm
        } else {
            Decimal::ZERO
        };

        let month_principal = sched_principal + prepayment;
        total_prepayment += prepayment;
        total_principal += month_principal;

        // WAL: weight by month in years.
        wal_numerator += Decimal::from(month_idx + 1) * month_principal / dec!(12);

        balance -= sched_principal + prepayment;
        if balance < Decimal::ZERO {
            balance = Decimal::ZERO;
        }
        remaining = remaining.saturating_sub(1);

        cpr_schedule.push(cpr_capped);
        smm_schedule.push(smm);
        projected_balances.push(balance);
        projected_prepayments.push(prepayment);

        if balance < BALANCE_EPSILON && !maturity_found {
            expected_maturity = month_idx + 1;
            maturity_found = true;
        }
    }

    let weighted_average_life = if total_principal > Decimal::ZERO {
        wal_numerator / total_principal
    } else {
        Decimal::ZERO
    };

    Ok((
        PsaOutput {
            cpr_schedule,
            smm_schedule,
            projected_balances,
            projected_prepayments,
            total_prepayment,
            weighted_average_life,
            expected_maturity_months: expected_maturity,
        },
        warnings,
    ))
}

fn validate_psa(input: &PsaInput) -> CorpFinanceResult<()> {
    if input.psa_speed < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "psa_speed".into(),
            reason: "PSA speed must be non-negative".into(),
        });
    }
    if input.current_balance <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "current_balance".into(),
            reason: "Current balance must be positive".into(),
        });
    }
    if input.remaining_months == 0 {
        return Err(CorpFinanceError::InvalidInput {
            field: "remaining_months".into(),
            reason: "Remaining months must be greater than zero".into(),
        });
    }
    if input.mortgage_rate < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "mortgage_rate".into(),
            reason: "Mortgage rate cannot be negative".into(),
        });
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// CPR model
// ---------------------------------------------------------------------------

fn compute_cpr_model(input: &CprInput) -> CorpFinanceResult<(CprOutput, Vec<String>)> {
    let mut warnings: Vec<String> = Vec::new();
    validate_cpr(input)?;

    if input.annual_cpr > dec!(0.50) {
        warnings.push(format!(
            "CPR of {}% is unusually high",
            input.annual_cpr * dec!(100)
        ));
    }

    let monthly_rate = input.mortgage_rate / dec!(12);
    let smm = cpr_to_smm(input.annual_cpr);
    let mut balance = input.current_balance;
    let mut remaining = input.remaining_months;

    let mut cpr_schedule = Vec::with_capacity(input.remaining_months as usize);
    let mut smm_schedule = Vec::with_capacity(input.remaining_months as usize);
    let mut projected_balances = Vec::with_capacity(input.remaining_months as usize);
    let mut projected_prepayments = Vec::with_capacity(input.remaining_months as usize);

    let mut total_prepayment = Decimal::ZERO;
    let mut wal_numerator = Decimal::ZERO;
    let mut total_principal = Decimal::ZERO;
    let mut expected_maturity = input.remaining_months;
    let mut maturity_found = false;

    for month_idx in 0..input.remaining_months {
        if balance < BALANCE_EPSILON || remaining == 0 {
            cpr_schedule.push(Decimal::ZERO);
            smm_schedule.push(Decimal::ZERO);
            projected_balances.push(balance);
            projected_prepayments.push(Decimal::ZERO);
            if !maturity_found {
                expected_maturity = month_idx;
                maturity_found = true;
            }
            continue;
        }

        let sched_principal = compute_scheduled_principal(balance, monthly_rate, remaining);

        let prepay_base = balance - sched_principal;
        let prepayment = if prepay_base > Decimal::ZERO {
            prepay_base * smm
        } else {
            Decimal::ZERO
        };

        let month_principal = sched_principal + prepayment;
        total_prepayment += prepayment;
        total_principal += month_principal;
        wal_numerator += Decimal::from(month_idx + 1) * month_principal / dec!(12);

        balance -= sched_principal + prepayment;
        if balance < Decimal::ZERO {
            balance = Decimal::ZERO;
        }
        remaining = remaining.saturating_sub(1);

        cpr_schedule.push(input.annual_cpr);
        smm_schedule.push(smm);
        projected_balances.push(balance);
        projected_prepayments.push(prepayment);

        if balance < BALANCE_EPSILON && !maturity_found {
            expected_maturity = month_idx + 1;
            maturity_found = true;
        }
    }

    let weighted_average_life = if total_principal > Decimal::ZERO {
        wal_numerator / total_principal
    } else {
        Decimal::ZERO
    };

    Ok((
        CprOutput {
            cpr_schedule,
            smm_schedule,
            projected_balances,
            projected_prepayments,
            total_prepayment,
            weighted_average_life,
            expected_maturity_months: expected_maturity,
        },
        warnings,
    ))
}

fn validate_cpr(input: &CprInput) -> CorpFinanceResult<()> {
    if input.annual_cpr < Decimal::ZERO || input.annual_cpr > Decimal::ONE {
        return Err(CorpFinanceError::InvalidInput {
            field: "annual_cpr".into(),
            reason: "Annual CPR must be between 0 and 1".into(),
        });
    }
    if input.current_balance <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "current_balance".into(),
            reason: "Current balance must be positive".into(),
        });
    }
    if input.remaining_months == 0 {
        return Err(CorpFinanceError::InvalidInput {
            field: "remaining_months".into(),
            reason: "Remaining months must be greater than zero".into(),
        });
    }
    if input.mortgage_rate < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "mortgage_rate".into(),
            reason: "Mortgage rate cannot be negative".into(),
        });
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Refinancing incentive model
// ---------------------------------------------------------------------------

fn compute_refinancing(
    input: &RefinancingInput,
) -> CorpFinanceResult<(RefinancingOutput, Vec<String>)> {
    let mut warnings: Vec<String> = Vec::new();
    validate_refinancing(input)?;

    // Unadjusted incentive CPR: base + multiplier * max(0, mortgage_rate - market_rate).
    let rate_diff = input.mortgage_rate - input.market_rate;
    let incentive = if rate_diff > Decimal::ZERO {
        rate_diff
    } else {
        Decimal::ZERO
    };
    let base_incentive_cpr = input.base_cpr + input.incentive_multiplier * incentive;
    let base_incentive_cpr_capped = if base_incentive_cpr > Decimal::ONE {
        warnings.push("Base incentive CPR exceeds 100%; capped at 1.0".into());
        Decimal::ONE
    } else {
        base_incentive_cpr
    };

    // We need a rate for scheduled amortisation. Use the mortgage_rate.
    let monthly_rate = input.mortgage_rate / dec!(12);
    let mut balance = input.current_balance;
    let mut remaining = input.remaining_months;

    let mut cpr_schedule = Vec::with_capacity(input.remaining_months as usize);
    let mut smm_schedule = Vec::with_capacity(input.remaining_months as usize);
    let mut projected_balances = Vec::with_capacity(input.remaining_months as usize);
    let mut projected_prepayments = Vec::with_capacity(input.remaining_months as usize);

    let mut total_prepayment = Decimal::ZERO;
    let mut wal_numerator = Decimal::ZERO;
    let mut total_principal = Decimal::ZERO;
    let mut expected_maturity = input.remaining_months;
    let mut maturity_found = false;

    for month_idx in 0..input.remaining_months {
        if balance < BALANCE_EPSILON || remaining == 0 {
            cpr_schedule.push(Decimal::ZERO);
            smm_schedule.push(Decimal::ZERO);
            projected_balances.push(balance);
            projected_prepayments.push(Decimal::ZERO);
            if !maturity_found {
                expected_maturity = month_idx;
                maturity_found = true;
            }
            continue;
        }

        // Burnout: the incentive CPR decays over time as easy refinancers leave.
        // burnout_adjusted_cpr = base_incentive_cpr * (1 - burnout_factor)^(loan_age + month)
        let age = input.loan_age_months + month_idx + 1;
        let burnout_decay = iterative_pow(Decimal::ONE - input.burnout_factor, age);
        let adjusted_cpr = base_incentive_cpr_capped * burnout_decay;
        let adjusted_cpr_clamped = if adjusted_cpr < Decimal::ZERO {
            Decimal::ZERO
        } else if adjusted_cpr > Decimal::ONE {
            Decimal::ONE
        } else {
            adjusted_cpr
        };

        let smm = cpr_to_smm(adjusted_cpr_clamped);

        let sched_principal = compute_scheduled_principal(balance, monthly_rate, remaining);

        let prepay_base = balance - sched_principal;
        let prepayment = if prepay_base > Decimal::ZERO {
            prepay_base * smm
        } else {
            Decimal::ZERO
        };

        let month_principal = sched_principal + prepayment;
        total_prepayment += prepayment;
        total_principal += month_principal;
        wal_numerator += Decimal::from(month_idx + 1) * month_principal / dec!(12);

        balance -= sched_principal + prepayment;
        if balance < Decimal::ZERO {
            balance = Decimal::ZERO;
        }
        remaining = remaining.saturating_sub(1);

        cpr_schedule.push(adjusted_cpr_clamped);
        smm_schedule.push(smm);
        projected_balances.push(balance);
        projected_prepayments.push(prepayment);

        if balance < BALANCE_EPSILON && !maturity_found {
            expected_maturity = month_idx + 1;
            maturity_found = true;
        }
    }

    let weighted_average_life = if total_principal > Decimal::ZERO {
        wal_numerator / total_principal
    } else {
        Decimal::ZERO
    };

    Ok((
        RefinancingOutput {
            cpr_schedule,
            smm_schedule,
            projected_balances,
            projected_prepayments,
            total_prepayment,
            weighted_average_life,
            expected_maturity_months: expected_maturity,
            base_incentive_cpr: base_incentive_cpr_capped,
        },
        warnings,
    ))
}

fn validate_refinancing(input: &RefinancingInput) -> CorpFinanceResult<()> {
    if input.current_balance <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "current_balance".into(),
            reason: "Current balance must be positive".into(),
        });
    }
    if input.remaining_months == 0 {
        return Err(CorpFinanceError::InvalidInput {
            field: "remaining_months".into(),
            reason: "Remaining months must be greater than zero".into(),
        });
    }
    if input.base_cpr < Decimal::ZERO || input.base_cpr > Decimal::ONE {
        return Err(CorpFinanceError::InvalidInput {
            field: "base_cpr".into(),
            reason: "Base CPR must be between 0 and 1".into(),
        });
    }
    if input.burnout_factor < Decimal::ZERO || input.burnout_factor > Decimal::ONE {
        return Err(CorpFinanceError::InvalidInput {
            field: "burnout_factor".into(),
            reason: "Burnout factor must be between 0 and 1".into(),
        });
    }
    if input.incentive_multiplier < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "incentive_multiplier".into(),
            reason: "Incentive multiplier must be non-negative".into(),
        });
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Decimal math helpers (no f64, no powd)
// ---------------------------------------------------------------------------

/// Taylor series expansion for e^x, 30 terms.
pub fn decimal_exp(x: Decimal) -> Decimal {
    let mut sum = Decimal::ONE;
    let mut term = Decimal::ONE;
    for n in 1..=30u32 {
        term *= x / Decimal::from(n);
        sum += term;
        if term.abs() < dec!(0.00000000000001) {
            break;
        }
    }
    sum
}

/// Natural logarithm via Newton's method, 20 iterations.
/// Uses the identity: solve f(y) = e^y - x = 0.
pub fn decimal_ln(x: Decimal) -> Decimal {
    if x <= Decimal::ZERO {
        return Decimal::ZERO; // undefined; guard
    }
    if x == Decimal::ONE {
        return Decimal::ZERO;
    }

    // Initial guess: use a rough estimate.
    // For x close to 1, ln(x) ~ x - 1.
    // For larger x, start with a reasonable guess.
    let mut guess = x - Decimal::ONE;
    if guess.abs() > dec!(2) {
        // Better initial guess for values far from 1.
        // Count how many times we can divide by e (~2.718).
        guess = Decimal::ZERO;
        let mut temp = x;
        let e_approx = dec!(2.718281828);
        if temp > Decimal::ONE {
            while temp > e_approx {
                temp /= e_approx;
                guess += Decimal::ONE;
            }
            guess += temp - Decimal::ONE;
        } else {
            while temp < Decimal::ONE / e_approx {
                temp *= e_approx;
                guess -= Decimal::ONE;
            }
            guess += temp - Decimal::ONE;
        }
    }

    for _ in 0..20 {
        let exp_guess = decimal_exp(guess);
        if exp_guess.is_zero() {
            break;
        }
        let delta = (exp_guess - x) / exp_guess;
        guess -= delta;
        if delta.abs() < dec!(0.00000000000001) {
            break;
        }
    }

    guess
}

/// Compute base^exp for arbitrary Decimal exponent via exp(exp * ln(base)).
pub fn power_decimal(base: Decimal, exp: Decimal) -> Decimal {
    if base.is_zero() {
        return Decimal::ZERO;
    }
    if exp.is_zero() {
        return Decimal::ONE;
    }
    if base == Decimal::ONE {
        return Decimal::ONE;
    }
    decimal_exp(exp * decimal_ln(base))
}

/// Compute base^n for a positive integer exponent via iterative multiplication.
fn iterative_pow(base: Decimal, n: u32) -> Decimal {
    let mut result = Decimal::ONE;
    for _ in 0..n {
        result *= base;
    }
    result
}

/// Compute 1 / base^n via iterative multiplication.
fn iterative_pow_recip(base: Decimal, n: u32) -> Decimal {
    let pow = iterative_pow(base, n);
    if pow.is_zero() {
        Decimal::ZERO
    } else {
        Decimal::ONE / pow
    }
}

/// Compute the nth root of x using Newton's method (40 iterations).
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

/// Convert annual CPR to single monthly mortality (SMM).
/// SMM = 1 - (1 - CPR)^(1/12)
pub fn cpr_to_smm(cpr: Rate) -> Rate {
    if cpr <= Decimal::ZERO {
        return Decimal::ZERO;
    }
    if cpr >= Decimal::ONE {
        return Decimal::ONE;
    }
    let base = Decimal::ONE - cpr;
    Decimal::ONE - nth_root(base, 12)
}

/// Convert SMM back to annualised CPR.
/// CPR = 1 - (1 - SMM)^12
pub fn smm_to_cpr(smm: Rate) -> Rate {
    if smm <= Decimal::ZERO {
        return Decimal::ZERO;
    }
    if smm >= Decimal::ONE {
        return Decimal::ONE;
    }
    Decimal::ONE - iterative_pow(Decimal::ONE - smm, 12)
}

/// Compute scheduled principal for a level-pay amortising loan.
fn compute_scheduled_principal(balance: Money, monthly_rate: Rate, remaining: u32) -> Money {
    if remaining == 0 {
        return balance;
    }
    if monthly_rate <= Decimal::ZERO {
        // Zero-rate: equal principal payments.
        return balance / Decimal::from(remaining);
    }

    let denom = Decimal::ONE - iterative_pow_recip(Decimal::ONE + monthly_rate, remaining);
    let payment = if denom > Decimal::ZERO {
        balance * monthly_rate / denom
    } else {
        balance
    };

    let interest = balance * monthly_rate;
    let mut principal = payment - interest;

    if principal > balance {
        principal = balance;
    }
    if principal < Decimal::ZERO {
        principal = Decimal::ZERO;
    }
    principal
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    const TOL: Decimal = dec!(0.01);
    const RATE_TOL: Decimal = dec!(0.0001);

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

    fn standard_psa_input() -> PsaInput {
        PsaInput {
            psa_speed: dec!(100),
            loan_age_months: 0,
            remaining_months: 360,
            original_balance: dec!(1_000_000),
            current_balance: dec!(1_000_000),
            mortgage_rate: dec!(0.06),
        }
    }

    fn standard_cpr_input() -> CprInput {
        CprInput {
            annual_cpr: dec!(0.06),
            loan_age_months: 0,
            remaining_months: 360,
            original_balance: dec!(1_000_000),
            current_balance: dec!(1_000_000),
            mortgage_rate: dec!(0.06),
        }
    }

    fn standard_refi_input() -> RefinancingInput {
        RefinancingInput {
            mortgage_rate: dec!(0.06),
            market_rate: dec!(0.04),
            base_cpr: dec!(0.02),
            incentive_multiplier: dec!(2.0),
            burnout_factor: dec!(0.01),
            loan_age_months: 0,
            remaining_months: 360,
            original_balance: dec!(1_000_000),
            current_balance: dec!(1_000_000),
        }
    }

    fn run_psa(input: &PsaInput) -> PsaOutput {
        let pi = PrepaymentInput {
            model: PrepaymentModel::Psa(input.clone()),
        };
        let result = analyze_prepayment(&pi).unwrap();
        match result.result {
            PrepaymentOutput::Psa(out) => out,
            _ => panic!("Expected PsaOutput"),
        }
    }

    fn run_cpr(input: &CprInput) -> CprOutput {
        let pi = PrepaymentInput {
            model: PrepaymentModel::Cpr(input.clone()),
        };
        let result = analyze_prepayment(&pi).unwrap();
        match result.result {
            PrepaymentOutput::Cpr(out) => out,
            _ => panic!("Expected CprOutput"),
        }
    }

    fn run_refi(input: &RefinancingInput) -> RefinancingOutput {
        let pi = PrepaymentInput {
            model: PrepaymentModel::Refinancing(input.clone()),
        };
        let result = analyze_prepayment(&pi).unwrap();
        match result.result {
            PrepaymentOutput::Refinancing(out) => out,
            _ => panic!("Expected RefinancingOutput"),
        }
    }

    // -----------------------------------------------------------------------
    // 1. PSA 100%: CPR at month 1 = 0.2%
    // -----------------------------------------------------------------------
    #[test]
    fn test_psa_100_cpr_month_1() {
        let out = run_psa(&standard_psa_input());
        // Month 1 (age=1): CPR = 0.06 * 1/30 = 0.002
        assert_close(
            out.cpr_schedule[0],
            dec!(0.002),
            RATE_TOL,
            "PSA 100 month 1 CPR",
        );
    }

    // -----------------------------------------------------------------------
    // 2. PSA 100%: CPR at month 30 = 6%
    // -----------------------------------------------------------------------
    #[test]
    fn test_psa_100_cpr_month_30() {
        let out = run_psa(&standard_psa_input());
        assert_close(
            out.cpr_schedule[29],
            dec!(0.06),
            RATE_TOL,
            "PSA 100 month 30 CPR",
        );
    }

    // -----------------------------------------------------------------------
    // 3. PSA 100%: CPR at month 31+ = 6%
    // -----------------------------------------------------------------------
    #[test]
    fn test_psa_100_cpr_month_31_plus() {
        let out = run_psa(&standard_psa_input());
        assert_close(
            out.cpr_schedule[30],
            dec!(0.06),
            RATE_TOL,
            "PSA 100 month 31 CPR",
        );
        assert_close(
            out.cpr_schedule[59],
            dec!(0.06),
            RATE_TOL,
            "PSA 100 month 60 CPR",
        );
    }

    // -----------------------------------------------------------------------
    // 4. PSA 150% = 1.5x the 100% PSA CPR
    // -----------------------------------------------------------------------
    #[test]
    fn test_psa_150_cpr_scaling() {
        let mut input = standard_psa_input();
        input.psa_speed = dec!(150);
        let out = run_psa(&input);

        // Month 1 (age=1): CPR = 0.06 * 1/30 * 150/100 = 0.003
        assert_close(
            out.cpr_schedule[0],
            dec!(0.003),
            RATE_TOL,
            "PSA 150 month 1 CPR",
        );
        // Month 30 (age=30): CPR = 0.06 * 150/100 = 0.09
        assert_close(
            out.cpr_schedule[29],
            dec!(0.09),
            RATE_TOL,
            "PSA 150 month 30 CPR",
        );
    }

    // -----------------------------------------------------------------------
    // 5. SMM conversion: SMM = 1 - (1-CPR)^(1/12)
    // -----------------------------------------------------------------------
    #[test]
    fn test_smm_conversion() {
        let cpr = dec!(0.06);
        let smm = cpr_to_smm(cpr);
        // SMM ~ 0.005143
        // Verify round-trip: (1-SMM)^12 = 1-CPR
        let reconstructed = iterative_pow(Decimal::ONE - smm, 12);
        assert_close(
            reconstructed,
            Decimal::ONE - cpr,
            dec!(0.000001),
            "SMM round-trip via (1-SMM)^12",
        );
    }

    // -----------------------------------------------------------------------
    // 6. SMM round-trip CPR -> SMM -> CPR
    // -----------------------------------------------------------------------
    #[test]
    fn test_smm_round_trip() {
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

    // -----------------------------------------------------------------------
    // 7. WAL shorter at higher PSA speeds
    // -----------------------------------------------------------------------
    #[test]
    fn test_wal_shorter_at_higher_psa() {
        let out_100 = run_psa(&standard_psa_input());

        let mut input_200 = standard_psa_input();
        input_200.psa_speed = dec!(200);
        let out_200 = run_psa(&input_200);

        assert!(
            out_200.weighted_average_life < out_100.weighted_average_life,
            "WAL at 200% PSA ({}) should be shorter than 100% PSA ({})",
            out_200.weighted_average_life,
            out_100.weighted_average_life
        );
    }

    // -----------------------------------------------------------------------
    // 8. CPR: constant prepayment rate across months
    // -----------------------------------------------------------------------
    #[test]
    fn test_cpr_constant_rate() {
        let input = standard_cpr_input();
        let out = run_cpr(&input);

        for (i, &cpr) in out.cpr_schedule.iter().enumerate() {
            if out.projected_balances.get(i).copied().unwrap_or_default() > BALANCE_EPSILON {
                assert_close(
                    cpr,
                    dec!(0.06),
                    RATE_TOL,
                    &format!("CPR should be constant at month {}", i + 1),
                );
            }
        }
    }

    // -----------------------------------------------------------------------
    // 9. CPR: balance amortisation (monotonically decreasing)
    // -----------------------------------------------------------------------
    #[test]
    fn test_cpr_balance_decreasing() {
        let out = run_cpr(&standard_cpr_input());

        for window in out.projected_balances.windows(2) {
            assert!(
                window[1] <= window[0] + TOL,
                "Balance should decrease: {} -> {}",
                window[0],
                window[1]
            );
        }
    }

    // -----------------------------------------------------------------------
    // 10. Refinancing: higher rate spread -> higher prepayment
    // -----------------------------------------------------------------------
    #[test]
    fn test_refi_higher_spread_higher_prepayment() {
        let input_small = RefinancingInput {
            mortgage_rate: dec!(0.06),
            market_rate: dec!(0.055), // small spread
            base_cpr: dec!(0.02),
            incentive_multiplier: dec!(2.0),
            burnout_factor: dec!(0.005),
            loan_age_months: 0,
            remaining_months: 120,
            original_balance: dec!(1_000_000),
            current_balance: dec!(1_000_000),
        };
        let out_small = run_refi(&input_small);

        let input_large = RefinancingInput {
            mortgage_rate: dec!(0.06),
            market_rate: dec!(0.03), // large spread
            base_cpr: dec!(0.02),
            incentive_multiplier: dec!(2.0),
            burnout_factor: dec!(0.005),
            loan_age_months: 0,
            remaining_months: 120,
            original_balance: dec!(1_000_000),
            current_balance: dec!(1_000_000),
        };
        let out_large = run_refi(&input_large);

        assert!(
            out_large.total_prepayment > out_small.total_prepayment,
            "Larger spread should produce more prepayment: {} vs {}",
            out_large.total_prepayment,
            out_small.total_prepayment,
        );
    }

    // -----------------------------------------------------------------------
    // 11. Burnout: prepayment speed decreases over time
    // -----------------------------------------------------------------------
    #[test]
    fn test_burnout_decreasing_cpr() {
        let input = standard_refi_input();
        let out = run_refi(&input);

        // With burnout_factor > 0, CPR should decline month-over-month.
        // Check first 30 months (before balance gets too small).
        for i in 1..30 {
            if out.projected_balances[i] > BALANCE_EPSILON
                && out.projected_balances[i - 1] > BALANCE_EPSILON
            {
                assert!(
                    out.cpr_schedule[i] <= out.cpr_schedule[i - 1] + RATE_TOL,
                    "CPR should decrease with burnout: month {} CPR {} > month {} CPR {}",
                    i + 1,
                    out.cpr_schedule[i],
                    i,
                    out.cpr_schedule[i - 1],
                );
            }
        }
    }

    // -----------------------------------------------------------------------
    // 12. Edge: 0% PSA = no prepayment
    // -----------------------------------------------------------------------
    #[test]
    fn test_psa_0_no_prepayment() {
        let mut input = standard_psa_input();
        input.psa_speed = dec!(0);
        input.remaining_months = 60;
        let out = run_psa(&input);

        assert_eq!(out.total_prepayment, Decimal::ZERO);
        for &p in &out.projected_prepayments {
            assert_eq!(p, Decimal::ZERO);
        }
    }

    // -----------------------------------------------------------------------
    // 13. Edge: 300%+ PSA extreme
    // -----------------------------------------------------------------------
    #[test]
    fn test_psa_300_extreme() {
        let mut input = standard_psa_input();
        input.psa_speed = dec!(300);
        let out = run_psa(&input);

        // Should still produce valid, non-negative results.
        assert!(out.total_prepayment > Decimal::ZERO);
        for &b in &out.projected_balances {
            assert!(b >= Decimal::ZERO, "Balance should never be negative");
        }
        // WAL should be shorter than 100% PSA.
        let out_100 = run_psa(&standard_psa_input());
        assert!(out.weighted_average_life < out_100.weighted_average_life);
    }

    // -----------------------------------------------------------------------
    // 14. Balance never goes negative (PSA)
    // -----------------------------------------------------------------------
    #[test]
    fn test_psa_balance_never_negative() {
        let mut input = standard_psa_input();
        input.psa_speed = dec!(400);
        let out = run_psa(&input);

        for (i, &b) in out.projected_balances.iter().enumerate() {
            assert!(
                b >= Decimal::ZERO,
                "Month {}: balance should not be negative, got {}",
                i + 1,
                b
            );
        }
    }

    // -----------------------------------------------------------------------
    // 15. Balance monotonically decreasing (PSA)
    // -----------------------------------------------------------------------
    #[test]
    fn test_psa_balance_monotonically_decreasing() {
        let out = run_psa(&standard_psa_input());

        let mut prev = dec!(1_000_000);
        for (i, &b) in out.projected_balances.iter().enumerate() {
            assert!(
                b <= prev + TOL,
                "Month {}: balance {} should be <= previous {}",
                i + 1,
                b,
                prev
            );
            prev = b;
        }
    }

    // -----------------------------------------------------------------------
    // 16. CPR: 0% CPR = no prepayment
    // -----------------------------------------------------------------------
    #[test]
    fn test_cpr_zero_no_prepayment() {
        let mut input = standard_cpr_input();
        input.annual_cpr = dec!(0.0);
        input.remaining_months = 60;
        let out = run_cpr(&input);

        assert_eq!(out.total_prepayment, Decimal::ZERO);
        for &p in &out.projected_prepayments {
            assert_eq!(p, Decimal::ZERO);
        }
    }

    // -----------------------------------------------------------------------
    // 17. PSA CPR ramp is linear
    // -----------------------------------------------------------------------
    #[test]
    fn test_psa_cpr_ramp_linear() {
        let out = run_psa(&standard_psa_input());

        // CPR at month 15 (age=15): 0.06 * 15/30 = 0.03
        assert_close(
            out.cpr_schedule[14],
            dec!(0.03),
            RATE_TOL,
            "PSA month 15 CPR",
        );
        // CPR at month 10 (age=10): 0.06 * 10/30 = 0.02
        assert_close(
            out.cpr_schedule[9],
            dec!(0.02),
            RATE_TOL,
            "PSA month 10 CPR",
        );
    }

    // -----------------------------------------------------------------------
    // 18. Refinancing: no incentive when market_rate >= mortgage_rate
    // -----------------------------------------------------------------------
    #[test]
    fn test_refi_no_incentive_when_rates_equal() {
        let mut input = standard_refi_input();
        input.market_rate = dec!(0.06); // equal to mortgage_rate
        input.remaining_months = 60;
        let out = run_refi(&input);

        // With no rate differential, only base_cpr applies.
        // base_incentive_cpr should equal base_cpr.
        assert_close(
            out.base_incentive_cpr,
            dec!(0.02),
            RATE_TOL,
            "No incentive: base_incentive_cpr should equal base_cpr",
        );
    }

    // -----------------------------------------------------------------------
    // 19. Refinancing: no incentive when market rate is higher
    // -----------------------------------------------------------------------
    #[test]
    fn test_refi_no_incentive_when_market_rate_higher() {
        let mut input = standard_refi_input();
        input.market_rate = dec!(0.08); // higher than mortgage_rate
        input.remaining_months = 60;
        let out = run_refi(&input);

        assert_close(
            out.base_incentive_cpr,
            dec!(0.02),
            RATE_TOL,
            "Market rate higher: base_incentive_cpr should equal base_cpr",
        );
    }

    // -----------------------------------------------------------------------
    // 20. Validation: negative PSA speed
    // -----------------------------------------------------------------------
    #[test]
    fn test_validation_negative_psa_speed() {
        let mut input = standard_psa_input();
        input.psa_speed = dec!(-50);
        let pi = PrepaymentInput {
            model: PrepaymentModel::Psa(input),
        };
        let result = analyze_prepayment(&pi);
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // 21. Validation: CPR out of range
    // -----------------------------------------------------------------------
    #[test]
    fn test_validation_cpr_out_of_range() {
        let mut input = standard_cpr_input();
        input.annual_cpr = dec!(1.5);
        let pi = PrepaymentInput {
            model: PrepaymentModel::Cpr(input),
        };
        let result = analyze_prepayment(&pi);
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // 22. Validation: zero remaining months
    // -----------------------------------------------------------------------
    #[test]
    fn test_validation_zero_remaining_months() {
        let mut input = standard_psa_input();
        input.remaining_months = 0;
        let pi = PrepaymentInput {
            model: PrepaymentModel::Psa(input),
        };
        let result = analyze_prepayment(&pi);
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // 23. Validation: negative balance
    // -----------------------------------------------------------------------
    #[test]
    fn test_validation_negative_balance() {
        let mut input = standard_cpr_input();
        input.current_balance = dec!(-100);
        let pi = PrepaymentInput {
            model: PrepaymentModel::Cpr(input),
        };
        let result = analyze_prepayment(&pi);
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // 24. Metadata is populated
    // -----------------------------------------------------------------------
    #[test]
    fn test_metadata_populated() {
        let pi = PrepaymentInput {
            model: PrepaymentModel::Psa(standard_psa_input()),
        };
        let result = analyze_prepayment(&pi).unwrap();
        assert!(!result.methodology.is_empty());
        assert!(result.methodology.contains("PSA"));
        assert_eq!(result.metadata.precision, "rust_decimal_128bit");
    }

    // -----------------------------------------------------------------------
    // 25. Schedule lengths match remaining_months
    // -----------------------------------------------------------------------
    #[test]
    fn test_schedule_lengths() {
        let input = standard_psa_input();
        let out = run_psa(&input);

        assert_eq!(out.cpr_schedule.len(), 360);
        assert_eq!(out.smm_schedule.len(), 360);
        assert_eq!(out.projected_balances.len(), 360);
        assert_eq!(out.projected_prepayments.len(), 360);
    }

    // -----------------------------------------------------------------------
    // 26. WAL is positive for non-zero prepayment
    // -----------------------------------------------------------------------
    #[test]
    fn test_wal_positive() {
        let out = run_psa(&standard_psa_input());
        assert!(
            out.weighted_average_life > Decimal::ZERO,
            "WAL should be positive"
        );
    }

    // -----------------------------------------------------------------------
    // 27. decimal_exp basic values
    // -----------------------------------------------------------------------
    #[test]
    fn test_decimal_exp_basic() {
        // e^0 = 1
        assert_close(
            decimal_exp(Decimal::ZERO),
            Decimal::ONE,
            dec!(0.0001),
            "e^0",
        );
        // e^1 ~ 2.71828
        assert_close(decimal_exp(Decimal::ONE), dec!(2.71828), dec!(0.001), "e^1");
    }

    // -----------------------------------------------------------------------
    // 28. decimal_ln basic values
    // -----------------------------------------------------------------------
    #[test]
    fn test_decimal_ln_basic() {
        // ln(1) = 0
        assert_eq!(decimal_ln(Decimal::ONE), Decimal::ZERO);
        // ln(e) ~ 1
        let e_val = dec!(2.718281828);
        assert_close(decimal_ln(e_val), Decimal::ONE, dec!(0.001), "ln(e)");
    }

    // -----------------------------------------------------------------------
    // 29. power_decimal round-trip
    // -----------------------------------------------------------------------
    #[test]
    fn test_power_decimal() {
        // 2^3 = 8
        let result = power_decimal(dec!(2), dec!(3));
        assert_close(result, dec!(8), dec!(0.01), "2^3");
    }

    // -----------------------------------------------------------------------
    // 30. PSA with non-zero loan age offset
    // -----------------------------------------------------------------------
    #[test]
    fn test_psa_with_loan_age_offset() {
        let mut input = standard_psa_input();
        input.loan_age_months = 24;
        input.remaining_months = 60;
        let out = run_psa(&input);

        // Month 1 of projection, age = 25: CPR = 0.06 * 25/30 = 0.05
        assert_close(
            out.cpr_schedule[0],
            dec!(0.05),
            dec!(0.001),
            "PSA age 25 CPR",
        );
        // Month 6 of projection, age = 30: CPR = 0.06
        assert_close(
            out.cpr_schedule[5],
            dec!(0.06),
            dec!(0.001),
            "PSA age 30 CPR",
        );
    }

    // -----------------------------------------------------------------------
    // 31. CPR balance never negative
    // -----------------------------------------------------------------------
    #[test]
    fn test_cpr_balance_never_negative() {
        let mut input = standard_cpr_input();
        input.annual_cpr = dec!(0.50);
        let out = run_cpr(&input);

        for (i, &b) in out.projected_balances.iter().enumerate() {
            assert!(
                b >= Decimal::ZERO,
                "Month {}: CPR balance should not be negative, got {}",
                i + 1,
                b
            );
        }
    }

    // -----------------------------------------------------------------------
    // 32. Burnout factor 0 = no decay
    // -----------------------------------------------------------------------
    #[test]
    fn test_burnout_factor_zero_no_decay() {
        let mut input = standard_refi_input();
        input.burnout_factor = dec!(0.0);
        input.remaining_months = 60;
        let out = run_refi(&input);

        // With no burnout, all CPRs should equal the base incentive CPR.
        let expected_cpr = out.base_incentive_cpr;
        for (i, &cpr) in out.cpr_schedule.iter().enumerate() {
            if out.projected_balances[i] > BALANCE_EPSILON {
                assert_close(
                    cpr,
                    expected_cpr,
                    dec!(0.001),
                    &format!("No burnout: month {} CPR should be constant", i + 1),
                );
            }
        }
    }

    // -----------------------------------------------------------------------
    // 33. Validation: burnout factor out of range
    // -----------------------------------------------------------------------
    #[test]
    fn test_validation_burnout_factor_out_of_range() {
        let mut input = standard_refi_input();
        input.burnout_factor = dec!(1.5);
        let pi = PrepaymentInput {
            model: PrepaymentModel::Refinancing(input),
        };
        let result = analyze_prepayment(&pi);
        assert!(result.is_err());
    }
}
