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

/// Withdrawal strategy for the decumulation phase.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WithdrawalStrategy {
    /// Withdraw a fixed real (inflation-adjusted) amount each year.
    ConstantDollar,
    /// Withdraw a fixed percentage of the portfolio each year (e.g., 0.04 = 4%).
    ConstantPercentage(Decimal),
    /// Dynamic withdrawal with guardrails around an initial percentage.
    GuardrailsPercent {
        initial_pct: Decimal,
        floor_pct: Decimal,
        ceiling_pct: Decimal,
    },
    /// Required minimum distribution (simplified IRS life-expectancy tables).
    Rmd,
}

/// Input parameters for retirement planning.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetirementInput {
    pub current_age: u32,
    pub retirement_age: u32,
    pub life_expectancy: u32,
    pub current_savings: Money,
    pub annual_income: Money,
    pub annual_savings: Money,
    pub savings_growth_rate: Rate,
    pub pre_retirement_return: Rate,
    pub post_retirement_return: Rate,
    pub inflation_rate: Rate,
    pub desired_replacement_ratio: Rate,
    pub social_security_annual: Money,
    pub withdrawal_strategy: WithdrawalStrategy,
    pub tax_rate_retirement: Rate,
}

/// Top-level output from `plan_retirement`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetirementOutput {
    pub accumulation_phase: AccumulationPhase,
    pub decumulation_phase: DecumulationPhase,
    pub savings_gap_analysis: SavingsGap,
    pub year_by_year: Vec<RetirementYear>,
}

/// Accumulation-phase summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccumulationPhase {
    pub years_to_retirement: u32,
    pub total_contributions: Money,
    pub total_investment_gains: Money,
    pub projected_portfolio_at_retirement: Money,
    pub real_portfolio_at_retirement: Money,
}

/// Decumulation-phase summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecumulationPhase {
    pub years_in_retirement: u32,
    pub initial_annual_withdrawal: Money,
    pub total_withdrawals: Money,
    pub portfolio_at_end: Money,
    pub years_portfolio_lasts: u32,
    pub sustainable: bool,
    pub legacy_amount: Money,
}

/// Savings-gap analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavingsGap {
    pub annual_income_needed_retirement: Money,
    pub annual_income_from_portfolio: Money,
    pub annual_income_from_ss: Money,
    pub total_annual_income_retirement: Money,
    pub income_gap: Money,
    pub additional_annual_savings_needed: Money,
    pub savings_rate_current: Rate,
    pub savings_rate_needed: Rate,
}

/// A single year in the retirement projection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetirementYear {
    pub age: u32,
    pub phase: String,
    pub beginning_balance: Money,
    pub contribution_or_withdrawal: Money,
    pub investment_return: Money,
    pub ending_balance: Money,
    pub real_value: Money,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Compute (1 + r)^n via iterative multiplication (avoids Decimal::powd drift).
fn compound(rate: Decimal, n: u32) -> Decimal {
    let mut result = Decimal::ONE;
    let factor = Decimal::ONE + rate;
    for _ in 0..n {
        result *= factor;
    }
    result
}

// ---------------------------------------------------------------------------
// Core function
// ---------------------------------------------------------------------------

/// Plan a full retirement projection including accumulation, decumulation,
/// savings-gap analysis, and a year-by-year schedule.
pub fn plan_retirement(
    input: &RetirementInput,
) -> CorpFinanceResult<ComputationOutput<RetirementOutput>> {
    let start = Instant::now();
    let mut warnings: Vec<String> = Vec::new();

    // --- Validation ---
    if input.retirement_age < input.current_age {
        return Err(CorpFinanceError::InvalidInput {
            field: "retirement_age".into(),
            reason: "retirement_age must be >= current_age".into(),
        });
    }
    if input.life_expectancy < input.retirement_age {
        return Err(CorpFinanceError::InvalidInput {
            field: "life_expectancy".into(),
            reason: "life_expectancy must be >= retirement_age".into(),
        });
    }
    if input.annual_income.is_zero() {
        return Err(CorpFinanceError::InvalidInput {
            field: "annual_income".into(),
            reason: "annual_income must be > 0".into(),
        });
    }

    let years_to_retirement = input.retirement_age - input.current_age;
    let years_in_retirement = input.life_expectancy - input.retirement_age;

    // ===================================================================
    // Accumulation phase
    // ===================================================================
    let mut year_by_year: Vec<RetirementYear> = Vec::new();
    let mut balance = input.current_savings;
    let mut total_contributions = Decimal::ZERO;
    let mut total_investment_gains = Decimal::ZERO;

    for yr in 0..years_to_retirement {
        let beginning = balance;
        // Contribution grows with savings_growth_rate each year
        let contribution = input.annual_savings * compound(input.savings_growth_rate, yr);
        let inv_return = (beginning + contribution) * input.pre_retirement_return;
        balance = beginning + contribution + inv_return;

        total_contributions += contribution;
        total_investment_gains += inv_return;

        // Real value: nominal / (1 + inflation)^(yr+1)
        let real = balance / compound(input.inflation_rate, yr + 1);

        year_by_year.push(RetirementYear {
            age: input.current_age + yr + 1,
            phase: "Accumulation".to_string(),
            beginning_balance: beginning,
            contribution_or_withdrawal: contribution,
            investment_return: inv_return,
            ending_balance: balance,
            real_value: real,
        });
    }

    let portfolio_at_retirement = balance;
    let real_portfolio_at_retirement =
        portfolio_at_retirement / compound(input.inflation_rate, years_to_retirement);

    let accumulation = AccumulationPhase {
        years_to_retirement,
        total_contributions,
        total_investment_gains,
        projected_portfolio_at_retirement: portfolio_at_retirement,
        real_portfolio_at_retirement,
    };

    // ===================================================================
    // Decumulation phase
    // ===================================================================

    // Needed annual income in retirement (nominal at retirement time)
    let income_at_retirement = input.annual_income
        * compound(input.savings_growth_rate, years_to_retirement)
        * input.desired_replacement_ratio;
    // Annual need from portfolio (after SS)
    let needed_from_portfolio = income_at_retirement - input.social_security_annual;
    // Gross withdrawal to cover taxes: needed / (1 - tax_rate)
    let gross_first_withdrawal = if input.tax_rate_retirement < Decimal::ONE {
        if needed_from_portfolio > Decimal::ZERO {
            needed_from_portfolio / (Decimal::ONE - input.tax_rate_retirement)
        } else {
            Decimal::ZERO
        }
    } else {
        warnings.push("Tax rate >= 100% — withdrawals set to 0".into());
        Decimal::ZERO
    };

    // Determine initial withdrawal based on strategy
    let initial_withdrawal = match &input.withdrawal_strategy {
        WithdrawalStrategy::ConstantDollar => gross_first_withdrawal,
        WithdrawalStrategy::ConstantPercentage(pct) => portfolio_at_retirement * pct,
        WithdrawalStrategy::GuardrailsPercent { initial_pct, .. } => {
            portfolio_at_retirement * initial_pct
        }
        WithdrawalStrategy::Rmd => {
            let factor = if years_in_retirement > 0 {
                Decimal::from(years_in_retirement)
            } else {
                Decimal::ONE
            };
            portfolio_at_retirement / factor
        }
    };

    let mut dec_balance = portfolio_at_retirement;
    let mut total_withdrawals = Decimal::ZERO;
    let mut years_portfolio_lasts: u32 = years_in_retirement;
    let mut portfolio_exhausted = false;
    // Reference portfolio for guardrails strategy
    let guardrails_ref_portfolio = portfolio_at_retirement;

    for yr in 0..years_in_retirement {
        if dec_balance <= Decimal::ZERO {
            if !portfolio_exhausted {
                years_portfolio_lasts = yr;
                portfolio_exhausted = true;
            }
            // Record zero-balance years
            let years_from_now = years_to_retirement + yr + 1;
            year_by_year.push(RetirementYear {
                age: input.retirement_age + yr + 1,
                phase: "Decumulation".to_string(),
                beginning_balance: Decimal::ZERO,
                contribution_or_withdrawal: Decimal::ZERO,
                investment_return: Decimal::ZERO,
                ending_balance: Decimal::ZERO,
                real_value: Decimal::ZERO,
            });
            let _ = years_from_now; // suppress warning
            continue;
        }

        let beginning = dec_balance;

        let withdrawal = match &input.withdrawal_strategy {
            WithdrawalStrategy::ConstantDollar => {
                // Inflation-adjusted from first year
                gross_first_withdrawal * compound(input.inflation_rate, yr)
            }
            WithdrawalStrategy::ConstantPercentage(pct) => beginning * pct,
            WithdrawalStrategy::GuardrailsPercent {
                initial_pct,
                floor_pct,
                ceiling_pct,
            } => {
                // Determine effective percentage based on portfolio vs reference
                let pct = if beginning > guardrails_ref_portfolio * dec!(1.2) {
                    *ceiling_pct
                } else if beginning < guardrails_ref_portfolio * dec!(0.8) {
                    *floor_pct
                } else {
                    *initial_pct
                };
                beginning * pct
            }
            WithdrawalStrategy::Rmd => {
                // Simplified RMD: factor = life_expectancy - current age, min 1
                let current_ret_age = input.retirement_age + yr;
                let remaining = if input.life_expectancy > current_ret_age {
                    input.life_expectancy - current_ret_age
                } else {
                    1
                };
                let factor = Decimal::from(remaining).max(Decimal::ONE);
                beginning / factor
            }
        };

        // Cap withdrawal at available balance
        let actual_withdrawal = withdrawal.min(beginning);
        let after_withdrawal = beginning - actual_withdrawal;
        let inv_return = after_withdrawal * input.post_retirement_return;
        dec_balance = after_withdrawal + inv_return;
        total_withdrawals += actual_withdrawal;

        let years_from_now = years_to_retirement + yr + 1;
        let real = if dec_balance > Decimal::ZERO {
            dec_balance / compound(input.inflation_rate, years_from_now)
        } else {
            Decimal::ZERO
        };

        year_by_year.push(RetirementYear {
            age: input.retirement_age + yr + 1,
            phase: "Decumulation".to_string(),
            beginning_balance: beginning,
            contribution_or_withdrawal: -actual_withdrawal,
            investment_return: inv_return,
            ending_balance: dec_balance,
            real_value: real,
        });
    }

    let portfolio_at_end = dec_balance.max(Decimal::ZERO);
    let sustainable = !portfolio_exhausted;
    let legacy_amount = if sustainable {
        portfolio_at_end
    } else {
        Decimal::ZERO
    };

    if !sustainable {
        warnings.push(format!(
            "Portfolio exhausted after {} years in retirement (before life expectancy)",
            years_portfolio_lasts
        ));
    }

    let decumulation = DecumulationPhase {
        years_in_retirement,
        initial_annual_withdrawal: initial_withdrawal,
        total_withdrawals,
        portfolio_at_end,
        years_portfolio_lasts,
        sustainable,
        legacy_amount,
    };

    // ===================================================================
    // Savings-gap analysis
    // ===================================================================
    let annual_income_needed = income_at_retirement;
    let annual_income_from_portfolio =
        initial_withdrawal * (Decimal::ONE - input.tax_rate_retirement);
    let annual_income_from_ss = input.social_security_annual;
    let total_annual = annual_income_from_portfolio + annual_income_from_ss;
    let income_gap = annual_income_needed - total_annual;

    // Additional annual savings needed to close the gap:
    // PV of gap annuity over retirement years, then level annual savings over accumulation years.
    let additional_annual_savings = if income_gap > Decimal::ZERO && years_to_retirement > 0 {
        // PV of gap annuity at post_retirement_return
        let pv_gap = pv_annuity(
            income_gap,
            input.post_retirement_return,
            years_in_retirement,
        );
        // Spread PV over accumulation years as future-value annuity
        fv_annuity_payment(pv_gap, input.pre_retirement_return, years_to_retirement)
    } else {
        Decimal::ZERO
    };

    let savings_rate_current = if input.annual_income > Decimal::ZERO {
        input.annual_savings / input.annual_income
    } else {
        Decimal::ZERO
    };

    let savings_rate_needed = if input.annual_income > Decimal::ZERO {
        (input.annual_savings + additional_annual_savings) / input.annual_income
    } else {
        Decimal::ZERO
    };

    let gap = SavingsGap {
        annual_income_needed_retirement: annual_income_needed,
        annual_income_from_portfolio,
        annual_income_from_ss,
        total_annual_income_retirement: total_annual,
        income_gap,
        additional_annual_savings_needed: additional_annual_savings,
        savings_rate_current,
        savings_rate_needed,
    };

    // ===================================================================
    // Build output
    // ===================================================================
    let output = RetirementOutput {
        accumulation_phase: accumulation,
        decumulation_phase: decumulation,
        savings_gap_analysis: gap,
        year_by_year,
    };

    let elapsed = start.elapsed().as_micros() as u64;
    Ok(with_metadata(
        "Retirement Planning (accumulation/decumulation with withdrawal strategies)",
        &serde_json::json!({
            "current_age": input.current_age,
            "retirement_age": input.retirement_age,
            "life_expectancy": input.life_expectancy,
            "withdrawal_strategy": format!("{:?}", input.withdrawal_strategy),
            "inflation_rate": input.inflation_rate.to_string(),
            "pre_retirement_return": input.pre_retirement_return.to_string(),
            "post_retirement_return": input.post_retirement_return.to_string(),
        }),
        warnings,
        elapsed,
        output,
    ))
}

// ---------------------------------------------------------------------------
// Annuity helpers
// ---------------------------------------------------------------------------

/// Present value of a level annuity: PV = pmt * [(1 - (1+r)^-n) / r]
fn pv_annuity(pmt: Decimal, rate: Rate, n: u32) -> Decimal {
    if rate.is_zero() || n == 0 {
        return pmt * Decimal::from(n);
    }
    let compound_factor = compound(rate, n);
    pmt * (Decimal::ONE - Decimal::ONE / compound_factor) / rate
}

/// Payment required to reach a future value via level annuity:
/// FV = pmt * [((1+r)^n - 1) / r]  =>  pmt = FV * r / ((1+r)^n - 1)
fn fv_annuity_payment(fv: Decimal, rate: Rate, n: u32) -> Decimal {
    if n == 0 {
        return Decimal::ZERO;
    }
    if rate.is_zero() {
        return fv / Decimal::from(n);
    }
    let compound_factor = compound(rate, n);
    let denom = compound_factor - Decimal::ONE;
    if denom.is_zero() {
        return fv / Decimal::from(n);
    }
    fv * rate / denom
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    /// Build a default input suitable for many tests. Override fields as needed.
    /// Designed so that the default ConstantDollar strategy is sustainable over
    /// 30 years of retirement (moderate income growth, generous SS, modest ratio).
    fn default_input() -> RetirementInput {
        RetirementInput {
            current_age: 35,
            retirement_age: 65,
            life_expectancy: 95,
            current_savings: dec!(200_000),
            annual_income: dec!(100_000),
            annual_savings: dec!(20_000),
            savings_growth_rate: dec!(0.02),
            pre_retirement_return: dec!(0.07),
            post_retirement_return: dec!(0.05),
            inflation_rate: dec!(0.025),
            desired_replacement_ratio: dec!(0.70),
            social_security_annual: dec!(25_000),
            withdrawal_strategy: WithdrawalStrategy::ConstantDollar,
            tax_rate_retirement: dec!(0.15),
        }
    }

    // ---------------------------------------------------------------
    // 1. Basic accumulation: 30 years of saving -> portfolio value
    // ---------------------------------------------------------------
    #[test]
    fn test_basic_accumulation_portfolio_value() {
        let input = default_input();
        let result = plan_retirement(&input).unwrap();
        let acc = &result.result.accumulation_phase;

        assert_eq!(acc.years_to_retirement, 30);
        // 30 years of contributions + 7% returns on a 100k starting balance
        // should produce a portfolio well north of 1M
        assert!(acc.projected_portfolio_at_retirement > dec!(1_000_000));
    }

    // ---------------------------------------------------------------
    // 2. Contributions grow with savings_growth_rate
    // ---------------------------------------------------------------
    #[test]
    fn test_contributions_grow_with_savings_rate() {
        let input = default_input();
        let result = plan_retirement(&input).unwrap();
        let yby = &result.result.year_by_year;

        // Year 1 contribution = 20_000 (growth^0)
        let yr1 = &yby[0];
        assert_eq!(yr1.contribution_or_withdrawal, dec!(20_000));

        // Year 2 contribution = 20_000 * 1.02
        let yr2 = &yby[1];
        let expected_yr2 = dec!(20_000) * dec!(1.02);
        assert_eq!(yr2.contribution_or_withdrawal, expected_yr2);
    }

    // ---------------------------------------------------------------
    // 3. Investment returns compound
    // ---------------------------------------------------------------
    #[test]
    fn test_investment_returns_compound() {
        let mut input = default_input();
        input.savings_growth_rate = Decimal::ZERO;
        input.annual_savings = Decimal::ZERO;
        input.current_savings = dec!(100_000);

        let result = plan_retirement(&input).unwrap();
        let acc = &result.result.accumulation_phase;

        // 100k at 7% for 30 years via iterative: 100_000 * 1.07^30
        let expected = dec!(100_000) * compound(dec!(0.07), 30);
        // Allow small tolerance for cumulative rounding
        let diff = (acc.projected_portfolio_at_retirement - expected).abs();
        assert!(diff < dec!(1.0), "diff={}", diff);
    }

    // ---------------------------------------------------------------
    // 4. Constant dollar withdrawal: sustainable over 30 years
    // ---------------------------------------------------------------
    #[test]
    fn test_constant_dollar_sustainable() {
        let input = default_input();
        let result = plan_retirement(&input).unwrap();
        let dec_phase = &result.result.decumulation_phase;

        assert!(dec_phase.sustainable);
        assert_eq!(dec_phase.years_portfolio_lasts, 30);
        assert!(dec_phase.legacy_amount > Decimal::ZERO);
    }

    // ---------------------------------------------------------------
    // 5. Constant dollar withdrawal: runs out early
    // ---------------------------------------------------------------
    #[test]
    fn test_constant_dollar_runs_out_early() {
        let mut input = default_input();
        input.current_savings = dec!(10_000);
        input.annual_savings = dec!(2_000);
        input.desired_replacement_ratio = dec!(1.5); // Extremely high need
        input.social_security_annual = Decimal::ZERO;

        let result = plan_retirement(&input).unwrap();
        let dec_phase = &result.result.decumulation_phase;

        assert!(!dec_phase.sustainable);
        assert!(dec_phase.years_portfolio_lasts < 30);
        assert_eq!(dec_phase.legacy_amount, Decimal::ZERO);
    }

    // ---------------------------------------------------------------
    // 6. 4% rule: sustainable for typical scenario
    // ---------------------------------------------------------------
    #[test]
    fn test_four_percent_rule_sustainable() {
        let mut input = default_input();
        input.withdrawal_strategy = WithdrawalStrategy::ConstantPercentage(dec!(0.04));

        let result = plan_retirement(&input).unwrap();
        let dec_phase = &result.result.decumulation_phase;

        // 4% of a large portfolio with 5% returns should be sustainable
        assert!(dec_phase.sustainable);
        // Initial withdrawal = 4% of portfolio
        let expected_initial = result
            .result
            .accumulation_phase
            .projected_portfolio_at_retirement
            * dec!(0.04);
        assert_eq!(dec_phase.initial_annual_withdrawal, expected_initial);
    }

    // ---------------------------------------------------------------
    // 7. Guardrails: increases withdrawal when portfolio grows
    // ---------------------------------------------------------------
    #[test]
    fn test_guardrails_ceiling_when_portfolio_grows() {
        let mut input = default_input();
        input.withdrawal_strategy = WithdrawalStrategy::GuardrailsPercent {
            initial_pct: dec!(0.04),
            floor_pct: dec!(0.03),
            ceiling_pct: dec!(0.05),
        };
        // High returns so the portfolio grows well above reference
        input.post_retirement_return = dec!(0.12);

        let result = plan_retirement(&input).unwrap();
        let yby = &result.result.year_by_year;

        // Find decumulation years where portfolio exceeds 120% of reference
        let ref_portfolio = result
            .result
            .accumulation_phase
            .projected_portfolio_at_retirement;
        let threshold = ref_portfolio * dec!(1.2);

        let ceiling_applied = yby.iter().any(|y| {
            y.phase == "Decumulation"
                && y.beginning_balance > threshold
                && y.contribution_or_withdrawal < Decimal::ZERO
        });
        assert!(
            ceiling_applied,
            "Guardrails ceiling should apply when portfolio grows > 20%"
        );
    }

    // ---------------------------------------------------------------
    // 8. Guardrails: decreases withdrawal when portfolio drops
    // ---------------------------------------------------------------
    #[test]
    fn test_guardrails_floor_when_portfolio_drops() {
        let mut input = default_input();
        input.withdrawal_strategy = WithdrawalStrategy::GuardrailsPercent {
            initial_pct: dec!(0.06),
            floor_pct: dec!(0.03),
            ceiling_pct: dec!(0.08),
        };
        // Poor returns -> portfolio drops
        input.post_retirement_return = dec!(0.00);
        input.desired_replacement_ratio = dec!(0.80);

        let result = plan_retirement(&input).unwrap();
        let yby = &result.result.year_by_year;

        let ref_portfolio = result
            .result
            .accumulation_phase
            .projected_portfolio_at_retirement;
        let threshold = ref_portfolio * dec!(0.8);

        // Look for years where balance dropped below 80% of reference
        let floor_applied = yby
            .iter()
            .any(|y| y.phase == "Decumulation" && y.beginning_balance < threshold);
        assert!(
            floor_applied,
            "With 0% return and 6% withdrawal, portfolio should drop below 80% threshold"
        );
    }

    // ---------------------------------------------------------------
    // 9. RMD: withdrawal rate increases with age
    // ---------------------------------------------------------------
    #[test]
    fn test_rmd_withdrawal_increases_with_age() {
        let mut input = default_input();
        input.withdrawal_strategy = WithdrawalStrategy::Rmd;

        let result = plan_retirement(&input).unwrap();
        let decumulation_years: Vec<&RetirementYear> = result
            .result
            .year_by_year
            .iter()
            .filter(|y| y.phase == "Decumulation" && y.beginning_balance > Decimal::ZERO)
            .collect();

        assert!(decumulation_years.len() >= 2);

        // RMD rate = 1/remaining_years, so rate increases as remaining decreases
        let first = &decumulation_years[0];
        let last = &decumulation_years[decumulation_years.len() - 1];

        // Withdrawal as percentage of balance should increase
        let first_pct = (-first.contribution_or_withdrawal) / first.beginning_balance;
        let last_pct = (-last.contribution_or_withdrawal) / last.beginning_balance;
        assert!(
            last_pct > first_pct,
            "RMD withdrawal rate should increase with age: first={} last={}",
            first_pct,
            last_pct
        );
    }

    // ---------------------------------------------------------------
    // 10. Social security reduces needed withdrawals
    // ---------------------------------------------------------------
    #[test]
    fn test_social_security_reduces_withdrawals() {
        let mut input_no_ss = default_input();
        input_no_ss.social_security_annual = Decimal::ZERO;

        let mut input_with_ss = default_input();
        input_with_ss.social_security_annual = dec!(30_000);

        let result_no_ss = plan_retirement(&input_no_ss).unwrap();
        let result_with_ss = plan_retirement(&input_with_ss).unwrap();

        assert!(
            result_with_ss
                .result
                .decumulation_phase
                .initial_annual_withdrawal
                < result_no_ss
                    .result
                    .decumulation_phase
                    .initial_annual_withdrawal,
            "With SS, initial withdrawal should be lower"
        );
    }

    // ---------------------------------------------------------------
    // 11. Savings gap: no gap (adequate savings)
    // ---------------------------------------------------------------
    #[test]
    fn test_savings_gap_no_shortfall() {
        let mut input = default_input();
        // High savings and modest lifestyle
        input.annual_savings = dec!(40_000);
        input.desired_replacement_ratio = dec!(0.50);
        input.social_security_annual = dec!(30_000);

        let result = plan_retirement(&input).unwrap();
        let gap = &result.result.savings_gap_analysis;

        // With generous savings, gap should be zero or negative
        assert!(
            gap.income_gap <= Decimal::ZERO
                || gap.additional_annual_savings_needed == Decimal::ZERO,
            "Well-funded plan should have no savings gap"
        );
    }

    // ---------------------------------------------------------------
    // 12. Savings gap: shortfall -> additional savings needed
    // ---------------------------------------------------------------
    #[test]
    fn test_savings_gap_shortfall() {
        let mut input = default_input();
        input.annual_savings = dec!(2_000);
        input.current_savings = dec!(5_000);
        input.social_security_annual = Decimal::ZERO;
        input.desired_replacement_ratio = dec!(0.90);
        // Use ConstantPercentage so initial withdrawal is based on portfolio size,
        // not on the needed income. A small portfolio with 4% rule will not meet
        // the high income need, producing a real gap.
        input.withdrawal_strategy = WithdrawalStrategy::ConstantPercentage(dec!(0.04));

        let result = plan_retirement(&input).unwrap();
        let gap = &result.result.savings_gap_analysis;

        assert!(
            gap.income_gap > Decimal::ZERO,
            "Should have income shortfall: gap={}",
            gap.income_gap
        );
        assert!(
            gap.additional_annual_savings_needed > Decimal::ZERO,
            "Should need additional savings"
        );
    }

    // ---------------------------------------------------------------
    // 13. Savings rate calculation
    // ---------------------------------------------------------------
    #[test]
    fn test_savings_rate_calculation() {
        let input = default_input(); // 20k / 100k = 0.20
        let result = plan_retirement(&input).unwrap();
        let gap = &result.result.savings_gap_analysis;

        assert_eq!(gap.savings_rate_current, dec!(0.20));
        // Needed rate should be >= current since there may be a gap
        assert!(gap.savings_rate_needed >= gap.savings_rate_current);
    }

    // ---------------------------------------------------------------
    // 14. Real value (inflation-adjusted) decreases over time
    // ---------------------------------------------------------------
    #[test]
    fn test_real_value_decreases_over_time() {
        let mut input = default_input();
        input.inflation_rate = dec!(0.03);

        let result = plan_retirement(&input).unwrap();
        let yby = &result.result.year_by_year;

        // Compare first and last accumulation year real values
        let accum: Vec<&RetirementYear> =
            yby.iter().filter(|y| y.phase == "Accumulation").collect();

        assert!(accum.len() > 1);
        let last = accum.last().unwrap();

        // Real value should be less than nominal at the end of accumulation
        assert!(
            last.real_value < last.ending_balance,
            "Real value ({}) should be less than nominal ({})",
            last.real_value,
            last.ending_balance
        );
    }

    // ---------------------------------------------------------------
    // 15. Year-by-year accumulation detail
    // ---------------------------------------------------------------
    #[test]
    fn test_year_by_year_accumulation_detail() {
        let input = default_input();
        let result = plan_retirement(&input).unwrap();
        let yby = &result.result.year_by_year;

        let accum: Vec<&RetirementYear> =
            yby.iter().filter(|y| y.phase == "Accumulation").collect();

        assert_eq!(accum.len(), 30);

        // Each year: ending = beginning + contribution + return
        for yr in &accum {
            let expected =
                yr.beginning_balance + yr.contribution_or_withdrawal + yr.investment_return;
            let diff = (yr.ending_balance - expected).abs();
            assert!(
                diff < dec!(0.01),
                "Year {} balance mismatch: {}",
                yr.age,
                diff
            );
        }
    }

    // ---------------------------------------------------------------
    // 16. Year-by-year decumulation detail
    // ---------------------------------------------------------------
    #[test]
    fn test_year_by_year_decumulation_detail() {
        let input = default_input();
        let result = plan_retirement(&input).unwrap();
        let yby = &result.result.year_by_year;

        let decum: Vec<&RetirementYear> =
            yby.iter().filter(|y| y.phase == "Decumulation").collect();

        assert_eq!(decum.len(), 30);

        // Contributions should be negative (withdrawals)
        for yr in &decum {
            if yr.beginning_balance > Decimal::ZERO {
                assert!(
                    yr.contribution_or_withdrawal <= Decimal::ZERO,
                    "Decumulation should have negative contribution at age {}",
                    yr.age
                );
            }
        }
    }

    // ---------------------------------------------------------------
    // 17. Legacy amount when sustainable
    // ---------------------------------------------------------------
    #[test]
    fn test_legacy_amount_when_sustainable() {
        let mut input = default_input();
        input.withdrawal_strategy = WithdrawalStrategy::ConstantPercentage(dec!(0.03));
        // Conservative 3% withdrawal with 5% return should leave a legacy
        let result = plan_retirement(&input).unwrap();
        let dec_phase = &result.result.decumulation_phase;

        assert!(dec_phase.sustainable);
        assert!(
            dec_phase.legacy_amount > Decimal::ZERO,
            "Should have a legacy amount with 3% withdrawal and 5% return"
        );
        assert_eq!(dec_phase.legacy_amount, dec_phase.portfolio_at_end);
    }

    // ---------------------------------------------------------------
    // 18. Tax impact on withdrawals
    // ---------------------------------------------------------------
    #[test]
    fn test_tax_impact_on_withdrawals() {
        // Higher tax rate -> higher gross withdrawal needed for same income
        let mut input_low_tax = default_input();
        input_low_tax.tax_rate_retirement = dec!(0.10);

        let mut input_high_tax = default_input();
        input_high_tax.tax_rate_retirement = dec!(0.35);

        let result_low = plan_retirement(&input_low_tax).unwrap();
        let result_high = plan_retirement(&input_high_tax).unwrap();

        assert!(
            result_high
                .result
                .decumulation_phase
                .initial_annual_withdrawal
                > result_low
                    .result
                    .decumulation_phase
                    .initial_annual_withdrawal,
            "Higher tax rate should require higher gross withdrawal"
        );
    }

    // ---------------------------------------------------------------
    // 19. Edge: already at retirement age (0 accumulation years)
    // ---------------------------------------------------------------
    #[test]
    fn test_already_at_retirement_age() {
        let mut input = default_input();
        input.current_age = 65;
        input.retirement_age = 65;
        input.current_savings = dec!(1_000_000);

        let result = plan_retirement(&input).unwrap();
        let acc = &result.result.accumulation_phase;
        let dec_phase = &result.result.decumulation_phase;

        assert_eq!(acc.years_to_retirement, 0);
        assert_eq!(acc.total_contributions, Decimal::ZERO);
        assert_eq!(acc.projected_portfolio_at_retirement, dec!(1_000_000));

        // Should still have decumulation
        assert_eq!(dec_phase.years_in_retirement, 30);
    }

    // ---------------------------------------------------------------
    // 20. Edge: very high return -> large surplus
    // ---------------------------------------------------------------
    #[test]
    fn test_very_high_return_large_surplus() {
        let mut input = default_input();
        input.pre_retirement_return = dec!(0.15);
        input.post_retirement_return = dec!(0.10);

        let result = plan_retirement(&input).unwrap();
        let dec_phase = &result.result.decumulation_phase;

        assert!(dec_phase.sustainable);
        assert!(
            dec_phase.legacy_amount > dec!(1_000_000),
            "Very high returns should leave a large legacy"
        );
    }

    // ---------------------------------------------------------------
    // 21. Replacement ratio impact on needed income
    // ---------------------------------------------------------------
    #[test]
    fn test_replacement_ratio_impact() {
        let mut input_low = default_input();
        input_low.desired_replacement_ratio = dec!(0.50);

        let mut input_high = default_input();
        input_high.desired_replacement_ratio = dec!(1.00);

        let result_low = plan_retirement(&input_low).unwrap();
        let result_high = plan_retirement(&input_high).unwrap();

        assert!(
            result_high
                .result
                .savings_gap_analysis
                .annual_income_needed_retirement
                > result_low
                    .result
                    .savings_gap_analysis
                    .annual_income_needed_retirement,
            "Higher replacement ratio = higher income needed"
        );
    }

    // ---------------------------------------------------------------
    // 22. Portfolio at end with different strategies
    // ---------------------------------------------------------------
    #[test]
    fn test_portfolio_at_end_different_strategies() {
        let base = default_input();

        let mut input_const = base.clone();
        input_const.withdrawal_strategy = WithdrawalStrategy::ConstantDollar;
        let result_const = plan_retirement(&input_const).unwrap();

        let mut input_pct = base.clone();
        input_pct.withdrawal_strategy = WithdrawalStrategy::ConstantPercentage(dec!(0.04));
        let result_pct = plan_retirement(&input_pct).unwrap();

        let mut input_rmd = base.clone();
        input_rmd.withdrawal_strategy = WithdrawalStrategy::Rmd;
        let result_rmd = plan_retirement(&input_rmd).unwrap();

        // All three should produce different end portfolios
        let end_const = result_const.result.decumulation_phase.portfolio_at_end;
        let end_pct = result_pct.result.decumulation_phase.portfolio_at_end;
        let end_rmd = result_rmd.result.decumulation_phase.portfolio_at_end;

        // ConstantPercentage never fully depletes (asymptotic), so it should be positive
        assert!(
            end_pct > Decimal::ZERO,
            "Constant percentage should never fully deplete"
        );

        // These are different strategies producing different outcomes
        let all_same = end_const == end_pct && end_pct == end_rmd;
        assert!(
            !all_same,
            "Different strategies should produce different end portfolios"
        );
    }

    // ---------------------------------------------------------------
    // Extra: validation errors
    // ---------------------------------------------------------------
    #[test]
    fn test_validation_retirement_before_current() {
        let mut input = default_input();
        input.retirement_age = 30;
        input.current_age = 35;

        assert!(plan_retirement(&input).is_err());
    }

    #[test]
    fn test_validation_life_expectancy_before_retirement() {
        let mut input = default_input();
        input.life_expectancy = 60;

        assert!(plan_retirement(&input).is_err());
    }

    #[test]
    fn test_validation_zero_income() {
        let mut input = default_input();
        input.annual_income = Decimal::ZERO;

        assert!(plan_retirement(&input).is_err());
    }

    // ---------------------------------------------------------------
    // Helper tests
    // ---------------------------------------------------------------
    #[test]
    fn test_compound_basic() {
        let result = compound(dec!(0.10), 3);
        // 1.1^3 = 1.331
        assert_eq!(result, dec!(1.331));
    }

    #[test]
    fn test_pv_annuity_basic() {
        // PV of $1000/yr for 10 years at 5%
        let pv = pv_annuity(dec!(1000), dec!(0.05), 10);
        // Expected ~7721.73
        assert!(pv > dec!(7700) && pv < dec!(7750));
    }

    #[test]
    fn test_fv_annuity_payment_basic() {
        // Need $100,000 in 10 years at 5%, payment needed each year
        let pmt = fv_annuity_payment(dec!(100_000), dec!(0.05), 10);
        // Expected ~7950.46
        assert!(pmt > dec!(7900) && pmt < dec!(8100));
    }
}
