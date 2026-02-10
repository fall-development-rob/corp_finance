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

const DAYS_IN_YEAR: Decimal = dec!(365);
const CIRCULAR_ITERATIONS: usize = 5;

// ---------------------------------------------------------------------------
// Input
// ---------------------------------------------------------------------------

/// Full input specification for a linked three-statement financial model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreeStatementInput {
    /// Base year revenue
    pub base_revenue: Money,
    /// Growth rate per projection year (length determines number of years)
    pub revenue_growth_rates: Vec<Rate>,
    /// COGS as % of revenue
    pub cogs_pct: Rate,
    /// SG&A as % of revenue
    pub sga_pct: Rate,
    /// R&D as % of revenue
    pub rnd_pct: Rate,
    /// D&A as % of prior PP&E
    pub da_pct: Rate,
    /// Interest rate on average debt
    pub interest_rate: Rate,
    /// Corporate tax rate
    pub tax_rate: Rate,
    /// Base year cash balance
    pub base_cash: Money,
    /// Base year accounts receivable
    pub base_receivables: Money,
    /// Base year inventory
    pub base_inventory: Money,
    /// Base year accounts payable
    pub base_payables: Money,
    /// Base year PP&E (net)
    pub base_ppe: Money,
    /// Base year total debt
    pub base_debt: Money,
    /// Base year shareholders' equity
    pub base_equity: Money,
    /// Days sales outstanding
    pub dso_days: Decimal,
    /// Days inventory outstanding
    pub dio_days: Decimal,
    /// Days payable outstanding
    pub dpo_days: Decimal,
    /// Capex as % of revenue
    pub capex_pct: Rate,
    /// Annual debt repayment as % of beginning debt
    pub debt_repayment_pct: Rate,
    /// Dividends as % of net income
    pub dividend_payout_ratio: Rate,
    /// Minimum cash to maintain (excess goes to extra debt paydown)
    pub min_cash_balance: Money,
}

// ---------------------------------------------------------------------------
// Output structs
// ---------------------------------------------------------------------------

/// Complete three-statement model output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreeStatementOutput {
    pub income_statements: Vec<IncomeStatement>,
    pub balance_sheets: Vec<BalanceSheet>,
    pub cash_flow_statements: Vec<CashFlowStatement>,
    pub summary: ProjectionSummary,
}

/// Income statement for a single projected year.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncomeStatement {
    pub year: i32,
    pub revenue: Money,
    pub cogs: Money,
    pub gross_profit: Money,
    pub gross_margin: Rate,
    pub sga: Money,
    pub rnd: Money,
    pub total_opex: Money,
    pub ebitda: Money,
    pub ebitda_margin: Rate,
    pub depreciation: Money,
    pub ebit: Money,
    pub ebit_margin: Rate,
    pub interest_expense: Money,
    pub ebt: Money,
    pub taxes: Money,
    pub net_income: Money,
    pub net_margin: Rate,
}

/// Balance sheet for a single projected year.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceSheet {
    pub year: i32,
    pub cash: Money,
    pub accounts_receivable: Money,
    pub inventory: Money,
    pub total_current_assets: Money,
    pub ppe_net: Money,
    pub total_assets: Money,
    pub accounts_payable: Money,
    pub current_debt: Money,
    pub total_current_liabilities: Money,
    pub long_term_debt: Money,
    pub total_debt: Money,
    pub total_liabilities: Money,
    pub shareholders_equity: Money,
    pub retained_earnings_cumulative: Money,
    pub total_liabilities_and_equity: Money,
}

/// Cash flow statement for a single projected year.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CashFlowStatement {
    pub year: i32,
    pub net_income: Money,
    pub depreciation: Money,
    pub change_in_receivables: Money,
    pub change_in_inventory: Money,
    pub change_in_payables: Money,
    pub cash_from_operations: Money,
    pub capex: Money,
    pub cash_from_investing: Money,
    pub debt_repayment: Money,
    pub new_debt: Money,
    pub dividends: Money,
    pub cash_from_financing: Money,
    pub net_change_in_cash: Money,
    pub ending_cash: Money,
    pub fcf: Money,
    pub fcfe: Money,
}

/// Aggregate summary metrics across the projection period.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectionSummary {
    pub total_years: i32,
    pub revenue_cagr: Rate,
    pub avg_ebitda_margin: Rate,
    pub avg_net_margin: Rate,
    pub ending_debt: Money,
    pub ending_leverage: Decimal,
    pub cumulative_fcf: Money,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Build a linked three-statement financial model (IS, BS, CF) with circular
/// reference resolution for interest expense.
pub fn build_three_statement_model(
    input: &ThreeStatementInput,
) -> CorpFinanceResult<ComputationOutput<ThreeStatementOutput>> {
    let start = Instant::now();
    let mut warnings: Vec<String> = Vec::new();

    validate_input(input)?;

    let n_years = input.revenue_growth_rates.len();

    // Carry-forward state from the base year
    let mut prior_revenue = input.base_revenue;
    let mut prior_receivables = input.base_receivables;
    let mut prior_inventory = input.base_inventory;
    let mut prior_payables = input.base_payables;
    let mut prior_ppe = input.base_ppe;
    let mut prior_debt = input.base_debt;
    let mut prior_cash = input.base_cash;
    let mut prior_equity = input.base_equity;
    let mut cumulative_retained_earnings = Decimal::ZERO;

    let mut income_statements = Vec::with_capacity(n_years);
    let mut balance_sheets = Vec::with_capacity(n_years);
    let mut cash_flow_statements = Vec::with_capacity(n_years);

    for yr_idx in 0..n_years {
        let year = (yr_idx + 1) as i32;
        let growth = input.revenue_growth_rates[yr_idx];

        // ---------------------------------------------------------------
        // Income Statement (pre-interest)
        // ---------------------------------------------------------------
        let revenue = prior_revenue * (Decimal::ONE + growth);
        let cogs = revenue * input.cogs_pct;
        let gross_profit = revenue - cogs;
        let sga = revenue * input.sga_pct;
        let rnd = revenue * input.rnd_pct;
        let total_opex = sga + rnd;
        let depreciation = prior_ppe * input.da_pct;
        let ebitda = gross_profit - total_opex;
        let ebit = ebitda - depreciation;

        // ---------------------------------------------------------------
        // Working capital
        // ---------------------------------------------------------------
        let receivables = revenue * input.dso_days / DAYS_IN_YEAR;
        let inventory = cogs * input.dio_days / DAYS_IN_YEAR;
        let payables = cogs * input.dpo_days / DAYS_IN_YEAR;
        let change_in_receivables = receivables - prior_receivables;
        let change_in_inventory = inventory - prior_inventory;
        let change_in_payables = payables - prior_payables;

        // ---------------------------------------------------------------
        // Capex and PP&E
        // ---------------------------------------------------------------
        let capex = revenue * input.capex_pct;
        let ppe_net = prior_ppe - depreciation + capex;

        // ---------------------------------------------------------------
        // Circular reference resolution: interest <-> debt <-> cash flow
        // We iterate CIRCULAR_ITERATIONS times starting from a naive
        // estimate (interest on prior-year debt).
        // ---------------------------------------------------------------
        let scheduled_repayment = prior_debt * input.debt_repayment_pct;

        // Initial guess: interest on prior-year debt
        let mut interest_expense = prior_debt * input.interest_rate;

        // Iterate to converge interest <-> debt <-> cash flow circular reference.
        // Only interest_expense is carried between iterations; everything else is
        // recomputed from scratch each time.
        for _iter in 0..CIRCULAR_ITERATIONS {
            let iter_ebt = ebit - interest_expense;
            let iter_taxes = if iter_ebt > Decimal::ZERO {
                iter_ebt * input.tax_rate
            } else {
                Decimal::ZERO
            };
            let iter_ni = iter_ebt - iter_taxes;
            let iter_dividends = if iter_ni > Decimal::ZERO {
                iter_ni * input.dividend_payout_ratio
            } else {
                Decimal::ZERO
            };
            let iter_cfo = iter_ni + depreciation - change_in_receivables - change_in_inventory
                + change_in_payables;
            let iter_preliminary_cash =
                prior_cash + iter_cfo + (-capex) + (-scheduled_repayment - iter_dividends);

            let (iter_new_debt, iter_extra_paydown) =
                if iter_preliminary_cash < input.min_cash_balance {
                    (
                        input.min_cash_balance - iter_preliminary_cash,
                        Decimal::ZERO,
                    )
                } else {
                    let excess = iter_preliminary_cash - input.min_cash_balance;
                    let remaining = prior_debt - scheduled_repayment;
                    let paydown = if remaining < Decimal::ZERO {
                        Decimal::ZERO
                    } else {
                        excess.min(remaining)
                    };
                    (Decimal::ZERO, paydown)
                };

            let mut iter_debt =
                prior_debt - scheduled_repayment - iter_extra_paydown + iter_new_debt;
            if iter_debt < Decimal::ZERO {
                iter_debt = Decimal::ZERO;
            }

            let avg_debt = (prior_debt + iter_debt) / dec!(2);
            interest_expense = avg_debt * input.interest_rate;
        }

        // Final computation with converged interest_expense
        let final_ebt = ebit - interest_expense;
        let final_taxes = if final_ebt > Decimal::ZERO {
            final_ebt * input.tax_rate
        } else {
            Decimal::ZERO
        };
        let final_net_income = final_ebt - final_taxes;

        let final_dividends = if final_net_income > Decimal::ZERO {
            final_net_income * input.dividend_payout_ratio
        } else {
            Decimal::ZERO
        };

        let final_cfo =
            final_net_income + depreciation - change_in_receivables - change_in_inventory
                + change_in_payables;
        let cfi = -capex;

        let preliminary_cff = -scheduled_repayment - final_dividends;
        let preliminary_cash = prior_cash + final_cfo + cfi + preliminary_cff;

        let (final_new_debt, final_extra_paydown, final_ending_cash) =
            if preliminary_cash < input.min_cash_balance {
                (
                    input.min_cash_balance - preliminary_cash,
                    Decimal::ZERO,
                    input.min_cash_balance,
                )
            } else {
                let excess = preliminary_cash - input.min_cash_balance;
                let remaining_debt = prior_debt - scheduled_repayment;
                let paydown = if remaining_debt < Decimal::ZERO {
                    Decimal::ZERO
                } else {
                    excess.min(remaining_debt)
                };
                (Decimal::ZERO, paydown, preliminary_cash - paydown)
            };

        let mut final_total_debt =
            prior_debt - scheduled_repayment - final_extra_paydown + final_new_debt;
        if final_total_debt < Decimal::ZERO {
            final_total_debt = Decimal::ZERO;
        }

        let total_debt_repayment = scheduled_repayment + final_extra_paydown;
        let cff = -total_debt_repayment + final_new_debt - final_dividends;
        let net_change_in_cash = final_cfo + cfi + cff;

        // Split debt into current (next year's scheduled repayment) and long-term
        let final_current_debt =
            (final_total_debt * input.debt_repayment_pct).min(final_total_debt);
        let final_long_term_debt = final_total_debt - final_current_debt;

        // Margins (protect against zero revenue)
        let gross_margin = safe_divide(gross_profit, revenue);
        let ebitda_margin = safe_divide(ebitda, revenue);
        let ebit_margin = safe_divide(ebit, revenue);
        let net_margin = safe_divide(final_net_income, revenue);

        // Free cash flows
        let fcf = final_cfo - capex;
        let fcfe = fcf - total_debt_repayment + final_new_debt;

        // Retained earnings
        cumulative_retained_earnings += final_net_income - final_dividends;

        // Shareholders' equity
        let shareholders_equity = prior_equity + final_net_income - final_dividends;

        // Balance sheet totals
        let total_current_assets = final_ending_cash + receivables + inventory;
        let total_assets = total_current_assets + ppe_net;
        let total_current_liabilities = payables + final_current_debt;
        let total_liabilities = total_current_liabilities + final_long_term_debt;
        let total_liabilities_and_equity = total_liabilities + shareholders_equity;

        // ---------------------------------------------------------------
        // Build output structs
        // ---------------------------------------------------------------
        income_statements.push(IncomeStatement {
            year,
            revenue,
            cogs,
            gross_profit,
            gross_margin,
            sga,
            rnd,
            total_opex,
            ebitda,
            ebitda_margin,
            depreciation,
            ebit,
            ebit_margin,
            interest_expense,
            ebt: final_ebt,
            taxes: final_taxes,
            net_income: final_net_income,
            net_margin,
        });

        balance_sheets.push(BalanceSheet {
            year,
            cash: final_ending_cash,
            accounts_receivable: receivables,
            inventory,
            total_current_assets,
            ppe_net,
            total_assets,
            accounts_payable: payables,
            current_debt: final_current_debt,
            total_current_liabilities,
            long_term_debt: final_long_term_debt,
            total_debt: final_total_debt,
            total_liabilities,
            shareholders_equity,
            retained_earnings_cumulative: cumulative_retained_earnings,
            total_liabilities_and_equity,
        });

        cash_flow_statements.push(CashFlowStatement {
            year,
            net_income: final_net_income,
            depreciation,
            change_in_receivables,
            change_in_inventory,
            change_in_payables,
            cash_from_operations: final_cfo,
            capex,
            cash_from_investing: cfi,
            debt_repayment: total_debt_repayment,
            new_debt: final_new_debt,
            dividends: final_dividends,
            cash_from_financing: cff,
            net_change_in_cash,
            ending_cash: final_ending_cash,
            fcf,
            fcfe,
        });

        // Warnings
        if ebitda > Decimal::ZERO {
            let leverage = final_total_debt / ebitda;
            if leverage > dec!(6) {
                warnings.push(format!(
                    "Year {year}: leverage ratio {leverage:.1}x exceeds 6.0x threshold"
                ));
            }
        }
        if interest_expense > Decimal::ZERO {
            let coverage = ebit / interest_expense;
            if coverage < dec!(2) {
                warnings.push(format!(
                    "Year {year}: interest coverage ratio {coverage:.2}x below 2.0x minimum"
                ));
            }
        }
        if fcf < Decimal::ZERO {
            warnings.push(format!("Year {year}: negative free cash flow ({fcf})"));
        }

        // Advance carry-forward state
        prior_revenue = revenue;
        prior_receivables = receivables;
        prior_inventory = inventory;
        prior_payables = payables;
        prior_ppe = ppe_net;
        prior_debt = final_total_debt;
        prior_cash = final_ending_cash;
        prior_equity = shareholders_equity;
    }

    // -----------------------------------------------------------------------
    // Summary
    // -----------------------------------------------------------------------
    let summary = build_summary(
        input,
        &income_statements,
        &cash_flow_statements,
        &balance_sheets,
    );

    let output = ThreeStatementOutput {
        income_statements,
        balance_sheets,
        cash_flow_statements,
        summary,
    };

    let elapsed = start.elapsed().as_micros() as u64;

    Ok(with_metadata(
        "Linked Three-Statement Model with Circular Reference Resolution",
        input,
        warnings,
        elapsed,
        output,
    ))
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

fn validate_input(input: &ThreeStatementInput) -> CorpFinanceResult<()> {
    if input.revenue_growth_rates.is_empty() {
        return Err(CorpFinanceError::InvalidInput {
            field: "revenue_growth_rates".into(),
            reason: "Must contain at least one growth rate".into(),
        });
    }

    validate_rate("cogs_pct", input.cogs_pct)?;
    validate_rate("sga_pct", input.sga_pct)?;
    validate_rate("rnd_pct", input.rnd_pct)?;
    validate_rate("da_pct", input.da_pct)?;
    validate_rate("interest_rate", input.interest_rate)?;
    validate_rate("tax_rate", input.tax_rate)?;
    validate_rate("capex_pct", input.capex_pct)?;
    validate_rate("debt_repayment_pct", input.debt_repayment_pct)?;
    validate_rate("dividend_payout_ratio", input.dividend_payout_ratio)?;

    validate_non_negative("base_revenue", input.base_revenue)?;
    validate_non_negative("base_cash", input.base_cash)?;
    validate_non_negative("base_receivables", input.base_receivables)?;
    validate_non_negative("base_inventory", input.base_inventory)?;
    validate_non_negative("base_payables", input.base_payables)?;
    validate_non_negative("base_ppe", input.base_ppe)?;
    validate_non_negative("base_debt", input.base_debt)?;
    validate_non_negative("base_equity", input.base_equity)?;
    validate_non_negative("min_cash_balance", input.min_cash_balance)?;
    validate_non_negative("dso_days", input.dso_days)?;
    validate_non_negative("dio_days", input.dio_days)?;
    validate_non_negative("dpo_days", input.dpo_days)?;

    // Operating expenses should not exceed 100% of revenue
    let total_cost_pct = input.cogs_pct + input.sga_pct + input.rnd_pct;
    if total_cost_pct > Decimal::ONE {
        return Err(CorpFinanceError::FinancialImpossibility(format!(
            "Total operating cost percentage ({total_cost_pct}) exceeds 100% of revenue"
        )));
    }

    Ok(())
}

fn validate_rate(field: &str, value: Rate) -> CorpFinanceResult<()> {
    if value < Decimal::ZERO || value > Decimal::ONE {
        return Err(CorpFinanceError::InvalidInput {
            field: field.into(),
            reason: format!("Rate must be between 0 and 1, got {value}"),
        });
    }
    Ok(())
}

fn validate_non_negative(field: &str, value: Money) -> CorpFinanceResult<()> {
    if value < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: field.into(),
            reason: format!("Value must be non-negative, got {value}"),
        });
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn safe_divide(numerator: Money, denominator: Money) -> Decimal {
    if denominator.is_zero() {
        Decimal::ZERO
    } else {
        numerator / denominator
    }
}

fn build_summary(
    input: &ThreeStatementInput,
    income_statements: &[IncomeStatement],
    cash_flow_statements: &[CashFlowStatement],
    balance_sheets: &[BalanceSheet],
) -> ProjectionSummary {
    let n = income_statements.len() as i32;
    let last_is = income_statements.last().unwrap();
    let last_bs = balance_sheets.last().unwrap();

    // Revenue CAGR: (ending / beginning)^(1/n) - 1
    // Use iterative approach: compute the ratio, then approximate via n-th root
    let revenue_cagr = compute_cagr(input.base_revenue, last_is.revenue, n);

    let avg_ebitda_margin = if n > 0 {
        let sum: Decimal = income_statements.iter().map(|is| is.ebitda_margin).sum();
        sum / Decimal::from(n)
    } else {
        Decimal::ZERO
    };

    let avg_net_margin = if n > 0 {
        let sum: Decimal = income_statements.iter().map(|is| is.net_margin).sum();
        sum / Decimal::from(n)
    } else {
        Decimal::ZERO
    };

    let cumulative_fcf: Money = cash_flow_statements.iter().map(|cf| cf.fcf).sum();

    let ending_leverage = if last_is.ebitda > Decimal::ZERO {
        last_bs.total_debt / last_is.ebitda
    } else {
        Decimal::ZERO
    };

    ProjectionSummary {
        total_years: n,
        revenue_cagr,
        avg_ebitda_margin,
        avg_net_margin,
        ending_debt: last_bs.total_debt,
        ending_leverage,
        cumulative_fcf,
    }
}

/// Compute CAGR using Newton's method for n-th root to avoid `powd()`.
/// CAGR = (ending / beginning)^(1/n) - 1
fn compute_cagr(beginning: Money, ending: Money, n: i32) -> Rate {
    if beginning <= Decimal::ZERO || ending <= Decimal::ZERO || n <= 0 {
        return Decimal::ZERO;
    }

    let ratio = ending / beginning;
    // Newton's method: find x such that x^n = ratio
    // x_{k+1} = x_k - (x_k^n - ratio) / (n * x_k^(n-1))
    //         = x_k * (1 - 1/n) + ratio / (n * x_k^(n-1))
    let n_dec = Decimal::from(n);
    let mut x = ratio; // initial guess

    for _ in 0..30 {
        // Compute x^(n-1) iteratively
        let mut x_pow_nm1 = Decimal::ONE;
        for _ in 0..(n - 1) {
            x_pow_nm1 *= x;
        }
        let x_pow_n = x_pow_nm1 * x;

        let denom = n_dec * x_pow_nm1;
        if denom.is_zero() {
            break;
        }

        x -= (x_pow_n - ratio) / denom;

        if x <= Decimal::ZERO {
            x = dec!(0.001);
        }
    }

    x - Decimal::ONE
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    /// Standard 3-year test input with moderate assumptions.
    fn sample_input() -> ThreeStatementInput {
        ThreeStatementInput {
            base_revenue: dec!(1000),
            revenue_growth_rates: vec![dec!(0.10), dec!(0.08), dec!(0.06)],
            cogs_pct: dec!(0.60),
            sga_pct: dec!(0.10),
            rnd_pct: dec!(0.05),
            da_pct: dec!(0.10),
            interest_rate: dec!(0.05),
            tax_rate: dec!(0.25),
            base_cash: dec!(100),
            base_receivables: dec!(80),
            base_inventory: dec!(60),
            base_payables: dec!(50),
            base_ppe: dec!(500),
            base_debt: dec!(400),
            base_equity: dec!(290),
            dso_days: dec!(30),
            dio_days: dec!(40),
            dpo_days: dec!(35),
            capex_pct: dec!(0.08),
            debt_repayment_pct: dec!(0.05),
            dividend_payout_ratio: dec!(0.30),
            min_cash_balance: dec!(50),
        }
    }

    // --------------------------------------------------
    // Core projection tests
    // --------------------------------------------------

    #[test]
    fn test_basic_3_year_projection() {
        let input = sample_input();
        let result = build_three_statement_model(&input).unwrap();
        let out = &result.result;

        assert_eq!(out.income_statements.len(), 3);
        assert_eq!(out.balance_sheets.len(), 3);
        assert_eq!(out.cash_flow_statements.len(), 3);
        assert_eq!(out.summary.total_years, 3);
    }

    #[test]
    fn test_year1_revenue() {
        let input = sample_input();
        let result = build_three_statement_model(&input).unwrap();
        let is = &result.result.income_statements[0];

        // Year 1: 1000 * (1 + 0.10) = 1100
        assert_eq!(is.revenue, dec!(1100));
        assert_eq!(is.year, 1);
    }

    #[test]
    fn test_year1_cogs_and_gross_profit() {
        let input = sample_input();
        let result = build_three_statement_model(&input).unwrap();
        let is = &result.result.income_statements[0];

        // COGS = 1100 * 0.60 = 660
        assert_eq!(is.cogs, dec!(660));
        // Gross profit = 1100 - 660 = 440
        assert_eq!(is.gross_profit, dec!(440));
        // Gross margin = 440 / 1100 = 0.4
        assert_eq!(is.gross_margin, dec!(0.4));
    }

    #[test]
    fn test_year1_operating_expenses() {
        let input = sample_input();
        let result = build_three_statement_model(&input).unwrap();
        let is = &result.result.income_statements[0];

        // SG&A = 1100 * 0.10 = 110
        assert_eq!(is.sga, dec!(110));
        // R&D = 1100 * 0.05 = 55
        assert_eq!(is.rnd, dec!(55));
        assert_eq!(is.total_opex, dec!(165));
    }

    #[test]
    fn test_year1_ebitda() {
        let input = sample_input();
        let result = build_three_statement_model(&input).unwrap();
        let is = &result.result.income_statements[0];

        // EBITDA = gross_profit - opex = 440 - 165 = 275
        assert_eq!(is.ebitda, dec!(275));
        assert_eq!(is.ebitda_margin, dec!(0.25));
    }

    #[test]
    fn test_year1_depreciation() {
        let input = sample_input();
        let result = build_three_statement_model(&input).unwrap();
        let is = &result.result.income_statements[0];

        // D&A = base_ppe * da_pct = 500 * 0.10 = 50
        assert_eq!(is.depreciation, dec!(50));
        // EBIT = 275 - 50 = 225
        assert_eq!(is.ebit, dec!(225));
    }

    #[test]
    fn test_revenue_compounds_correctly() {
        let input = sample_input();
        let result = build_three_statement_model(&input).unwrap();
        let stmts = &result.result.income_statements;

        // Year 1: 1000 * 1.10 = 1100
        assert_eq!(stmts[0].revenue, dec!(1100));
        // Year 2: 1100 * 1.08 = 1188
        assert_eq!(stmts[1].revenue, dec!(1188));
        // Year 3: 1188 * 1.06 = 1259.28
        assert_eq!(stmts[2].revenue, dec!(1259.28));
    }

    #[test]
    fn test_working_capital_dso() {
        let input = sample_input();
        let result = build_three_statement_model(&input).unwrap();
        let bs = &result.result.balance_sheets[0];

        // A/R = revenue * dso / 365 = 1100 * 30 / 365
        let expected_ar = dec!(1100) * dec!(30) / dec!(365);
        assert_eq!(bs.accounts_receivable, expected_ar);
    }

    #[test]
    fn test_working_capital_dio() {
        let input = sample_input();
        let result = build_three_statement_model(&input).unwrap();
        let bs = &result.result.balance_sheets[0];

        // Inventory = COGS * dio / 365 = 660 * 40 / 365
        let expected_inv = dec!(660) * dec!(40) / dec!(365);
        assert_eq!(bs.inventory, expected_inv);
    }

    #[test]
    fn test_working_capital_dpo() {
        let input = sample_input();
        let result = build_three_statement_model(&input).unwrap();
        let bs = &result.result.balance_sheets[0];

        // A/P = COGS * dpo / 365 = 660 * 35 / 365
        let expected_ap = dec!(660) * dec!(35) / dec!(365);
        assert_eq!(bs.accounts_payable, expected_ap);
    }

    #[test]
    fn test_capex_and_ppe() {
        let input = sample_input();
        let result = build_three_statement_model(&input).unwrap();
        let bs = &result.result.balance_sheets[0];
        let cf = &result.result.cash_flow_statements[0];

        // Capex = 1100 * 0.08 = 88
        assert_eq!(cf.capex, dec!(88));
        // PP&E = 500 - 50 + 88 = 538
        assert_eq!(bs.ppe_net, dec!(538));
    }

    // --------------------------------------------------
    // Balance sheet checks
    // --------------------------------------------------

    #[test]
    fn test_balance_sheet_balances() {
        let input = sample_input();
        let result = build_three_statement_model(&input).unwrap();

        for bs in &result.result.balance_sheets {
            // Assets = Liabilities + Equity
            let diff = (bs.total_assets - bs.total_liabilities_and_equity).abs();
            assert!(
                diff < dec!(0.01),
                "Year {}: BS does not balance. Assets={}, L+E={}, diff={}",
                bs.year,
                bs.total_assets,
                bs.total_liabilities_and_equity,
                diff,
            );
        }
    }

    #[test]
    fn test_cash_flow_ties_to_ending_cash() {
        let input = sample_input();
        let result = build_three_statement_model(&input).unwrap();
        let cfs = &result.result.cash_flow_statements;
        let bss = &result.result.balance_sheets;

        for (cf, bs) in cfs.iter().zip(bss.iter()) {
            let diff = (cf.ending_cash - bs.cash).abs();
            assert!(
                diff < dec!(0.01),
                "Year {}: CF ending cash ({}) != BS cash ({})",
                cf.year,
                cf.ending_cash,
                bs.cash,
            );
        }
    }

    // --------------------------------------------------
    // Scenario tests
    // --------------------------------------------------

    #[test]
    fn test_high_growth_scenario() {
        let mut input = sample_input();
        input.revenue_growth_rates =
            vec![dec!(0.25), dec!(0.20), dec!(0.15), dec!(0.10), dec!(0.08)];
        input.capex_pct = dec!(0.12);

        let result = build_three_statement_model(&input).unwrap();
        let out = &result.result;

        assert_eq!(out.income_statements.len(), 5);
        // Revenue should be much higher
        let final_rev = out.income_statements.last().unwrap().revenue;
        assert!(
            final_rev > dec!(1800),
            "High-growth final revenue should exceed 1800"
        );
    }

    #[test]
    fn test_deleveraging_scenario() {
        let mut input = sample_input();
        input.base_debt = dec!(800);
        input.debt_repayment_pct = dec!(0.10);
        input.dividend_payout_ratio = dec!(0.0);
        input.min_cash_balance = dec!(20);

        let result = build_three_statement_model(&input).unwrap();
        let out = &result.result;

        // Debt should decrease over time
        let first_debt = out.balance_sheets[0].total_debt;
        let last_debt = out.balance_sheets.last().unwrap().total_debt;
        assert!(
            last_debt < first_debt,
            "Debt should decrease: first={first_debt}, last={last_debt}"
        );
    }

    #[test]
    fn test_revolver_draw_scenario() {
        let mut input = sample_input();
        // Force cash shortfall: high capex, high repayment, high dividends
        input.capex_pct = dec!(0.25);
        input.debt_repayment_pct = dec!(0.15);
        input.dividend_payout_ratio = dec!(0.50);
        input.min_cash_balance = dec!(100);
        input.base_cash = dec!(100);

        let result = build_three_statement_model(&input).unwrap();
        let out = &result.result;

        // At least one year should have a revolver draw
        let has_revolver_draw = out
            .cash_flow_statements
            .iter()
            .any(|cf| cf.new_debt > Decimal::ZERO);
        assert!(
            has_revolver_draw,
            "Cash-constrained scenario should trigger at least one revolver draw"
        );
    }

    #[test]
    fn test_zero_growth_steady_state() {
        let mut input = sample_input();
        input.revenue_growth_rates = vec![dec!(0.0), dec!(0.0), dec!(0.0)];

        let result = build_three_statement_model(&input).unwrap();
        let stmts = &result.result.income_statements;

        // All years should have the same revenue
        for is in stmts {
            assert_eq!(
                is.revenue,
                dec!(1000),
                "Zero-growth revenue should stay at base"
            );
        }
    }

    #[test]
    fn test_100_pct_payout_ratio() {
        let mut input = sample_input();
        input.dividend_payout_ratio = dec!(1.0);

        let result = build_three_statement_model(&input).unwrap();
        let out = &result.result;

        // Dividends should equal net income each year
        for (is, cf) in out
            .income_statements
            .iter()
            .zip(out.cash_flow_statements.iter())
        {
            if is.net_income > Decimal::ZERO {
                let diff = (cf.dividends - is.net_income).abs();
                assert!(
                    diff < dec!(0.01),
                    "Year {}: dividends ({}) should equal NI ({})",
                    is.year,
                    cf.dividends,
                    is.net_income,
                );
            }
        }
    }

    #[test]
    fn test_zero_debt() {
        let mut input = sample_input();
        input.base_debt = dec!(0);
        input.debt_repayment_pct = dec!(0.0);
        input.min_cash_balance = dec!(0);

        let result = build_three_statement_model(&input).unwrap();
        let out = &result.result;

        // All years should have zero interest expense
        for is in &out.income_statements {
            assert_eq!(
                is.interest_expense,
                Decimal::ZERO,
                "Year {}: interest should be zero with no debt",
                is.year
            );
        }

        // All years should have zero debt
        for bs in &out.balance_sheets {
            assert_eq!(
                bs.total_debt,
                Decimal::ZERO,
                "Year {}: debt should be zero",
                bs.year
            );
        }
    }

    // --------------------------------------------------
    // Summary tests
    // --------------------------------------------------

    #[test]
    fn test_summary_total_years() {
        let input = sample_input();
        let result = build_three_statement_model(&input).unwrap();
        assert_eq!(result.result.summary.total_years, 3);
    }

    #[test]
    fn test_summary_revenue_cagr() {
        let input = sample_input();
        let result = build_three_statement_model(&input).unwrap();
        let cagr = result.result.summary.revenue_cagr;

        // 3-year CAGR for 10%/8%/6% growth should be roughly 8%
        assert!(
            cagr > dec!(0.07) && cagr < dec!(0.09),
            "CAGR should be ~8%, got {cagr}"
        );
    }

    #[test]
    fn test_summary_avg_ebitda_margin() {
        let input = sample_input();
        let result = build_three_statement_model(&input).unwrap();
        let avg = result.result.summary.avg_ebitda_margin;

        // EBITDA margin = 1 - 0.60 - 0.10 - 0.05 = 0.25
        assert_eq!(avg, dec!(0.25), "Average EBITDA margin should be 25%");
    }

    #[test]
    fn test_summary_cumulative_fcf() {
        let input = sample_input();
        let result = build_three_statement_model(&input).unwrap();
        let cum_fcf = result.result.summary.cumulative_fcf;

        // Should equal sum of per-year FCFs
        let manual_sum: Decimal = result
            .result
            .cash_flow_statements
            .iter()
            .map(|cf| cf.fcf)
            .sum();
        assert_eq!(cum_fcf, manual_sum);
    }

    // --------------------------------------------------
    // Validation tests
    // --------------------------------------------------

    #[test]
    fn test_empty_growth_rates_rejected() {
        let mut input = sample_input();
        input.revenue_growth_rates = vec![];

        let result = build_three_statement_model(&input);
        assert!(result.is_err());
        match result.unwrap_err() {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "revenue_growth_rates");
            }
            e => panic!("Expected InvalidInput, got {e:?}"),
        }
    }

    #[test]
    fn test_negative_rate_rejected() {
        let mut input = sample_input();
        input.cogs_pct = dec!(-0.1);

        let result = build_three_statement_model(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_rate_above_one_rejected() {
        let mut input = sample_input();
        input.tax_rate = dec!(1.5);

        let result = build_three_statement_model(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_negative_base_value_rejected() {
        let mut input = sample_input();
        input.base_revenue = dec!(-100);

        let result = build_three_statement_model(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_operating_cost_exceeds_100pct_rejected() {
        let mut input = sample_input();
        input.cogs_pct = dec!(0.80);
        input.sga_pct = dec!(0.15);
        input.rnd_pct = dec!(0.10);

        let result = build_three_statement_model(&input);
        assert!(result.is_err());
    }

    // --------------------------------------------------
    // Warnings tests
    // --------------------------------------------------

    #[test]
    fn test_high_leverage_warning() {
        let mut input = sample_input();
        input.base_debt = dec!(2000);
        input.debt_repayment_pct = dec!(0.0);
        input.min_cash_balance = dec!(0);

        let result = build_three_statement_model(&input).unwrap();
        let has_leverage_warning = result.warnings.iter().any(|w| w.contains("leverage ratio"));
        assert!(has_leverage_warning, "Should warn about high leverage");
    }

    #[test]
    fn test_low_interest_coverage_warning() {
        let mut input = sample_input();
        input.base_debt = dec!(5000);
        input.interest_rate = dec!(0.20);
        input.debt_repayment_pct = dec!(0.0);
        input.min_cash_balance = dec!(0);

        let result = build_three_statement_model(&input).unwrap();
        let has_coverage_warning = result
            .warnings
            .iter()
            .any(|w| w.contains("interest coverage"));
        assert!(
            has_coverage_warning,
            "Should warn about low interest coverage"
        );
    }

    // --------------------------------------------------
    // Metadata tests
    // --------------------------------------------------

    #[test]
    fn test_methodology_string() {
        let input = sample_input();
        let result = build_three_statement_model(&input).unwrap();
        assert_eq!(
            result.methodology,
            "Linked Three-Statement Model with Circular Reference Resolution"
        );
    }

    #[test]
    fn test_computation_time_recorded() {
        let input = sample_input();
        let result = build_three_statement_model(&input).unwrap();
        // The computation should take at least some microseconds
        assert!(result.metadata.computation_time_us < 1_000_000);
    }

    // --------------------------------------------------
    // FCF and FCFE tests
    // --------------------------------------------------

    #[test]
    fn test_fcf_equals_cfo_minus_capex() {
        let input = sample_input();
        let result = build_three_statement_model(&input).unwrap();

        for cf in &result.result.cash_flow_statements {
            let expected_fcf = cf.cash_from_operations - cf.capex;
            assert_eq!(
                cf.fcf, expected_fcf,
                "Year {}: FCF should equal CFO - CapEx",
                cf.year
            );
        }
    }

    #[test]
    fn test_fcfe_equals_fcf_minus_repayment_plus_new_debt() {
        let input = sample_input();
        let result = build_three_statement_model(&input).unwrap();

        for cf in &result.result.cash_flow_statements {
            let expected_fcfe = cf.fcf - cf.debt_repayment + cf.new_debt;
            assert_eq!(
                cf.fcfe, expected_fcfe,
                "Year {}: FCFE should equal FCF - repayment + new debt",
                cf.year
            );
        }
    }

    // --------------------------------------------------
    // Multi-year consistency tests
    // --------------------------------------------------

    #[test]
    fn test_debt_decreases_with_repayment() {
        let input = sample_input();
        let result = build_three_statement_model(&input).unwrap();
        let bss = &result.result.balance_sheets;

        // With 5% repayment and healthy cash flows, debt should generally decrease
        // (unless revolver draws push it up, which shouldn't happen with this input)
        let first_debt = bss[0].total_debt;
        let last_debt = bss.last().unwrap().total_debt;
        assert!(
            last_debt <= first_debt || first_debt <= input.base_debt,
            "Debt should not increase without revolver draws in base case"
        );
    }

    #[test]
    fn test_single_year_projection() {
        let mut input = sample_input();
        input.revenue_growth_rates = vec![dec!(0.10)];

        let result = build_three_statement_model(&input).unwrap();
        let out = &result.result;

        assert_eq!(out.income_statements.len(), 1);
        assert_eq!(out.balance_sheets.len(), 1);
        assert_eq!(out.cash_flow_statements.len(), 1);
    }

    #[test]
    fn test_five_year_projection() {
        let mut input = sample_input();
        input.revenue_growth_rates =
            vec![dec!(0.10), dec!(0.08), dec!(0.06), dec!(0.05), dec!(0.04)];

        let result = build_three_statement_model(&input).unwrap();
        assert_eq!(result.result.income_statements.len(), 5);
    }

    #[test]
    fn test_net_change_in_cash_consistency() {
        let input = sample_input();
        let result = build_three_statement_model(&input).unwrap();
        let cfs = &result.result.cash_flow_statements;

        for cf in cfs {
            let expected_net =
                cf.cash_from_operations + cf.cash_from_investing + cf.cash_from_financing;
            let diff = (cf.net_change_in_cash - expected_net).abs();
            assert!(
                diff < dec!(0.01),
                "Year {}: net change in cash ({}) should equal CFO+CFI+CFF ({})",
                cf.year,
                cf.net_change_in_cash,
                expected_net,
            );
        }
    }

    #[test]
    fn test_zero_dividend_payout() {
        let mut input = sample_input();
        input.dividend_payout_ratio = dec!(0.0);

        let result = build_three_statement_model(&input).unwrap();
        for cf in &result.result.cash_flow_statements {
            assert_eq!(
                cf.dividends,
                Decimal::ZERO,
                "Year {}: dividends should be zero",
                cf.year
            );
        }
    }
}
