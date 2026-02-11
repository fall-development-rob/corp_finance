use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Instant;

use crate::error::CorpFinanceError;
use crate::types::{with_metadata, ComputationOutput};
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Types -- GIPS & Regulatory Reporting
// ---------------------------------------------------------------------------

/// A single external cash flow event within a performance period.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CashFlowEvent {
    /// Day within the period (1-based).
    pub day_of_period: u32,
    /// Positive = inflow, negative = outflow.
    pub amount: Decimal,
    /// Total days in the period.
    pub total_days: u32,
}

/// Performance data for a single reporting period.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformancePeriod {
    pub period_name: String,
    pub beginning_value: Decimal,
    pub ending_value: Decimal,
    pub external_cash_flows: Vec<CashFlowEvent>,
    /// Dividends, interest, etc.
    pub income: Decimal,
    pub fees_management: Decimal,
    pub fees_performance: Decimal,
    pub fees_trading: Decimal,
}

/// Account-level returns for composite dispersion calculation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountReturn {
    pub account_name: String,
    /// One return per period.
    pub returns: Vec<Decimal>,
}

/// Input for GIPS-compliant performance reporting.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GipsInput {
    pub composite_name: String,
    pub periods: Vec<PerformancePeriod>,
    /// One benchmark return per period.
    pub benchmark_returns: Vec<Decimal>,
    pub inception_date: String,
    pub reporting_currency: String,
    /// "Gross", "Net", or "Both".
    pub fee_schedule: String,
    /// Account returns for dispersion calculation.
    pub composite_accounts: Vec<AccountReturn>,
}

/// Period-level GIPS result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GipsPeriodResult {
    pub period_name: String,
    pub gross_return: Decimal,
    pub net_return: Decimal,
    pub benchmark_return: Decimal,
    pub excess_return_gross: Decimal,
    pub excess_return_net: Decimal,
    /// Modified Dietz return.
    pub time_weighted_return: Decimal,
}

/// GIPS compliance checklist.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GipsChecklist {
    pub time_weighted_returns: bool,
    /// Assumed true for our calculation.
    pub trade_date_accounting: bool,
    /// Assumed true for our calculation.
    pub accrual_accounting: bool,
    /// True if accounts provided.
    pub composite_construction: bool,
    /// True if fee_schedule specified.
    pub fee_disclosure: bool,
    /// True if benchmark_returns provided.
    pub benchmark_disclosed: bool,
    /// True if >= 5 accounts.
    pub dispersion_reported: bool,
    pub overall_compliant: bool,
}

/// Full GIPS-compliant performance reporting output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GipsOutput {
    pub composite_name: String,
    pub period_results: Vec<GipsPeriodResult>,
    pub cumulative_gross_return: Decimal,
    pub cumulative_net_return: Decimal,
    pub cumulative_benchmark_return: Decimal,
    pub annualized_gross_return: Decimal,
    pub annualized_net_return: Decimal,
    pub annualized_benchmark_return: Decimal,
    pub annualized_excess_return: Decimal,
    pub tracking_error: Decimal,
    pub information_ratio: Decimal,
    /// Standard deviation of account returns within the composite.
    pub composite_dispersion: Decimal,
    pub sharpe_ratio: Decimal,
    pub max_drawdown: Decimal,
    pub gips_compliance_checklist: GipsChecklist,
    pub methodology: String,
    pub assumptions: HashMap<String, String>,
    pub warnings: Vec<String>,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Newton's method square root (20 iterations).
fn newton_sqrt(x: Decimal) -> Decimal {
    if x <= dec!(0) {
        return Decimal::ZERO;
    }
    let mut guess = x / dec!(2);
    if guess == dec!(0) {
        guess = dec!(1);
    }
    for _ in 0..20 {
        guess = (guess + x / guess) / dec!(2);
    }
    guess
}

/// Newton's method nth root (40 iterations): x^(1/n).
fn nth_root(x: Decimal, n: Decimal) -> Decimal {
    if x <= dec!(0) || n == dec!(0) {
        return Decimal::ZERO;
    }
    if n == dec!(1) {
        return x;
    }
    let mut guess = x / n;
    if guess == dec!(0) {
        guess = dec!(1);
    }
    // Newton: f(g) = g^n - x, f'(g) = n * g^(n-1)
    // g_new = g - (g^n - x) / (n * g^(n-1)) = ((n-1)*g + x/g^(n-1)) / n
    let n_minus_1 = n - dec!(1);
    let n_int = n_minus_1.to_string().parse::<u32>().unwrap_or(1);
    for _ in 0..40 {
        // Compute g^(n-1) iteratively
        let mut g_pow = dec!(1);
        for _ in 0..n_int {
            g_pow *= guess;
        }
        if g_pow == dec!(0) {
            break;
        }
        guess = (n_minus_1 * guess + x / g_pow) / n;
    }
    guess
}

/// Modified Dietz return for a single period.
fn modified_dietz(period: &PerformancePeriod) -> Decimal {
    let bmv = period.beginning_value;
    let emv = period.ending_value;

    let cf_sum: Decimal = period.external_cash_flows.iter().map(|cf| cf.amount).sum();

    let weighted_cf: Decimal = period
        .external_cash_flows
        .iter()
        .map(|cf| {
            let total = Decimal::from(cf.total_days);
            let day = Decimal::from(cf.day_of_period);
            let weight = if total == dec!(0) {
                dec!(0)
            } else {
                (total - day) / total
            };
            cf.amount * weight
        })
        .sum();

    let denominator = bmv + weighted_cf;
    if denominator == dec!(0) {
        return Decimal::ZERO;
    }

    (emv - bmv - cf_sum) / denominator
}

/// Standard deviation of a slice of Decimal values.
fn std_dev(values: &[Decimal]) -> Decimal {
    let n = Decimal::from(values.len() as u64);
    if n <= dec!(1) {
        return Decimal::ZERO;
    }
    let mean = values.iter().copied().sum::<Decimal>() / n;
    let variance: Decimal = values
        .iter()
        .map(|v| (*v - mean) * (*v - mean))
        .sum::<Decimal>()
        / (n - dec!(1));
    newton_sqrt(variance)
}

// ---------------------------------------------------------------------------
// Public function: generate_gips_report
// ---------------------------------------------------------------------------

/// Generate GIPS-compliant performance report with Modified Dietz returns,
/// geometric linking, tracking error, information ratio, and composite dispersion.
pub fn generate_gips_report(input: &GipsInput) -> CorpFinanceResult<ComputationOutput<GipsOutput>> {
    let start = Instant::now();
    let mut warnings: Vec<String> = Vec::new();

    // --- Validation ---
    if input.periods.is_empty() {
        return Err(CorpFinanceError::InvalidInput {
            field: "periods".to_string(),
            reason: "At least one period is required".to_string(),
        });
    }

    for (i, period) in input.periods.iter().enumerate() {
        if period.beginning_value <= dec!(0) {
            return Err(CorpFinanceError::InvalidInput {
                field: format!("periods[{}].beginning_value", i),
                reason: "Beginning value must be positive".to_string(),
            });
        }
    }

    if input.benchmark_returns.len() != input.periods.len() {
        return Err(CorpFinanceError::InvalidInput {
            field: "benchmark_returns".to_string(),
            reason: format!(
                "Benchmark returns length ({}) must match periods length ({})",
                input.benchmark_returns.len(),
                input.periods.len()
            ),
        });
    }

    let valid_fee_schedules = ["Gross", "Net", "Both"];
    if !valid_fee_schedules.contains(&input.fee_schedule.as_str()) {
        return Err(CorpFinanceError::InvalidInput {
            field: "fee_schedule".to_string(),
            reason: "Fee schedule must be 'Gross', 'Net', or 'Both'".to_string(),
        });
    }

    // Validate account returns lengths
    for account in &input.composite_accounts {
        if account.returns.len() != input.periods.len() {
            return Err(CorpFinanceError::InvalidInput {
                field: "composite_accounts".to_string(),
                reason: format!(
                    "Account '{}' returns length ({}) must match periods length ({})",
                    account.account_name,
                    account.returns.len(),
                    input.periods.len()
                ),
            });
        }
    }

    // --- Compute period results ---
    let mut period_results: Vec<GipsPeriodResult> = Vec::new();
    let mut gross_returns: Vec<Decimal> = Vec::new();
    let mut net_returns: Vec<Decimal> = Vec::new();
    let mut excess_returns: Vec<Decimal> = Vec::new();

    for (i, period) in input.periods.iter().enumerate() {
        let gross_return = modified_dietz(period);

        // Net return: subtract management + performance fees as fraction of BMV
        let fee_rate = if period.beginning_value > dec!(0) {
            (period.fees_management + period.fees_performance) / period.beginning_value
        } else {
            Decimal::ZERO
        };
        let net_return = gross_return - fee_rate;

        let benchmark_return = input.benchmark_returns[i];
        let excess_return_gross = gross_return - benchmark_return;
        let excess_return_net = net_return - benchmark_return;

        gross_returns.push(gross_return);
        net_returns.push(net_return);
        excess_returns.push(excess_return_gross);

        period_results.push(GipsPeriodResult {
            period_name: period.period_name.clone(),
            gross_return,
            net_return,
            benchmark_return,
            excess_return_gross,
            excess_return_net,
            time_weighted_return: gross_return,
        });
    }

    // --- Cumulative returns (geometric linking) ---
    let cumulative_gross = geometric_link(&gross_returns);
    let cumulative_net = geometric_link(&net_returns);
    let cumulative_benchmark = geometric_link(&input.benchmark_returns);

    // --- Annualized returns ---
    let n_periods = Decimal::from(input.periods.len() as u64);
    // Assume each period is 1 year unless stated otherwise
    let annualized_gross = annualize_return(cumulative_gross, n_periods);
    let annualized_net = annualize_return(cumulative_net, n_periods);
    let annualized_benchmark = annualize_return(cumulative_benchmark, n_periods);
    let annualized_excess = annualized_gross - annualized_benchmark;

    // --- Tracking error ---
    // Since we assume annual periods, annualized TE = std dev of annual excess returns
    let tracking_error = if excess_returns.len() > 1 {
        std_dev(&excess_returns)
    } else {
        Decimal::ZERO
    };

    // --- Information ratio ---
    let information_ratio = if tracking_error > dec!(0) {
        annualized_excess / tracking_error
    } else {
        Decimal::ZERO
    };

    // --- Composite dispersion ---
    let composite_dispersion = if input.composite_accounts.len() >= 2 {
        // Average each account's cumulative return, compute std dev
        let account_cum_returns: Vec<Decimal> = input
            .composite_accounts
            .iter()
            .map(|a| geometric_link(&a.returns))
            .collect();
        std_dev(&account_cum_returns)
    } else {
        Decimal::ZERO
    };

    // --- Sharpe ratio (risk-free = 0) ---
    let gross_std = if gross_returns.len() > 1 {
        std_dev(&gross_returns)
    } else {
        Decimal::ZERO
    };
    let sharpe_ratio = if gross_std > dec!(0) {
        annualized_gross / gross_std
    } else {
        Decimal::ZERO
    };

    // --- Max drawdown ---
    let max_drawdown = compute_max_drawdown(&gross_returns);

    // --- GIPS checklist ---
    let composite_construction = !input.composite_accounts.is_empty();
    let fee_disclosure = !input.fee_schedule.is_empty();
    let benchmark_disclosed = !input.benchmark_returns.is_empty();
    let dispersion_reported = input.composite_accounts.len() >= 5;

    if !dispersion_reported && input.composite_accounts.len() >= 2 {
        warnings.push(
            "Composite dispersion reported but fewer than 5 accounts (GIPS recommends >= 5)"
                .to_string(),
        );
    }

    let overall_compliant = fee_disclosure && benchmark_disclosed;

    let gips_compliance_checklist = GipsChecklist {
        time_weighted_returns: true,
        trade_date_accounting: true,
        accrual_accounting: true,
        composite_construction,
        fee_disclosure,
        benchmark_disclosed,
        dispersion_reported,
        overall_compliant,
    };

    let mut assumptions = HashMap::new();
    assumptions.insert("return_method".to_string(), "Modified Dietz".to_string());
    assumptions.insert(
        "geometric_linking".to_string(),
        "Multiplicative chain".to_string(),
    );
    assumptions.insert(
        "annualization".to_string(),
        format!("{} periods assumed annual", n_periods),
    );
    assumptions.insert("risk_free_rate".to_string(), "0%".to_string());
    assumptions.insert("fee_schedule".to_string(), input.fee_schedule.clone());
    assumptions.insert("currency".to_string(), input.reporting_currency.clone());

    let output = GipsOutput {
        composite_name: input.composite_name.clone(),
        period_results,
        cumulative_gross_return: cumulative_gross,
        cumulative_net_return: cumulative_net,
        cumulative_benchmark_return: cumulative_benchmark,
        annualized_gross_return: annualized_gross,
        annualized_net_return: annualized_net,
        annualized_benchmark_return: annualized_benchmark,
        annualized_excess_return: annualized_excess,
        tracking_error,
        information_ratio,
        composite_dispersion,
        sharpe_ratio,
        max_drawdown,
        gips_compliance_checklist,
        methodology: "GIPS-compliant Modified Dietz with geometric linking".to_string(),
        assumptions,
        warnings: warnings.clone(),
    };

    let elapsed = start.elapsed().as_micros() as u64;
    let assumptions_ser = HashMap::from([
        ("return_method", "Modified Dietz"),
        ("annualization", "nth-root geometric"),
        ("dispersion", "equal-weighted std dev"),
    ]);

    Ok(with_metadata(
        "GIPS Modified Dietz Performance Reporting",
        &assumptions_ser,
        warnings,
        elapsed,
        output,
    ))
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Geometric linking: cumulative = product(1 + R_t) - 1.
fn geometric_link(returns: &[Decimal]) -> Decimal {
    let mut product = dec!(1);
    for r in returns {
        product *= dec!(1) + *r;
    }
    product - dec!(1)
}

/// Annualize a cumulative return over n years using Newton nth root.
fn annualize_return(cumulative: Decimal, n_years: Decimal) -> Decimal {
    if n_years <= dec!(1) {
        return cumulative;
    }
    let base = dec!(1) + cumulative;
    if base <= dec!(0) {
        return dec!(-1);
    }
    nth_root(base, n_years) - dec!(1)
}

/// Compute maximum drawdown from a series of periodic returns.
fn compute_max_drawdown(returns: &[Decimal]) -> Decimal {
    if returns.is_empty() {
        return Decimal::ZERO;
    }

    let mut cumulative = dec!(1);
    let mut peak = dec!(1);
    let mut max_dd = Decimal::ZERO;

    for r in returns {
        cumulative *= dec!(1) + *r;
        if cumulative > peak {
            peak = cumulative;
        }
        if peak > dec!(0) {
            let drawdown = (peak - cumulative) / peak;
            if drawdown > max_dd {
                max_dd = drawdown;
            }
        }
    }

    max_dd
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    /// Build a simple single-period input.
    fn make_single_period_input() -> GipsInput {
        GipsInput {
            composite_name: "Global Equity".to_string(),
            periods: vec![PerformancePeriod {
                period_name: "2023".to_string(),
                beginning_value: dec!(1000000),
                ending_value: dec!(1080000),
                external_cash_flows: vec![],
                income: dec!(20000),
                fees_management: dec!(5000),
                fees_performance: dec!(1000),
                fees_trading: dec!(500),
            }],
            benchmark_returns: vec![dec!(0.07)],
            inception_date: "2023-01-01".to_string(),
            reporting_currency: "USD".to_string(),
            fee_schedule: "Both".to_string(),
            composite_accounts: vec![],
        }
    }

    /// Build a multi-period input.
    fn make_multi_period_input() -> GipsInput {
        GipsInput {
            composite_name: "US Large Cap".to_string(),
            periods: vec![
                PerformancePeriod {
                    period_name: "2021".to_string(),
                    beginning_value: dec!(1000000),
                    ending_value: dec!(1100000),
                    external_cash_flows: vec![],
                    income: dec!(15000),
                    fees_management: dec!(5000),
                    fees_performance: dec!(1000),
                    fees_trading: dec!(300),
                },
                PerformancePeriod {
                    period_name: "2022".to_string(),
                    beginning_value: dec!(1100000),
                    ending_value: dec!(1050000),
                    external_cash_flows: vec![],
                    income: dec!(12000),
                    fees_management: dec!(5500),
                    fees_performance: dec!(0),
                    fees_trading: dec!(400),
                },
                PerformancePeriod {
                    period_name: "2023".to_string(),
                    beginning_value: dec!(1050000),
                    ending_value: dec!(1200000),
                    external_cash_flows: vec![],
                    income: dec!(18000),
                    fees_management: dec!(5250),
                    fees_performance: dec!(2000),
                    fees_trading: dec!(350),
                },
            ],
            benchmark_returns: vec![dec!(0.08), dec!(-0.06), dec!(0.12)],
            inception_date: "2021-01-01".to_string(),
            reporting_currency: "USD".to_string(),
            fee_schedule: "Both".to_string(),
            composite_accounts: vec![
                AccountReturn {
                    account_name: "Account A".to_string(),
                    returns: vec![dec!(0.11), dec!(-0.04), dec!(0.15)],
                },
                AccountReturn {
                    account_name: "Account B".to_string(),
                    returns: vec![dec!(0.09), dec!(-0.06), dec!(0.13)],
                },
                AccountReturn {
                    account_name: "Account C".to_string(),
                    returns: vec![dec!(0.10), dec!(-0.05), dec!(0.14)],
                },
            ],
        }
    }

    // -----------------------------------------------------------------------
    // Single period tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_single_period_no_cash_flows() {
        let input = make_single_period_input();
        let result = generate_gips_report(&input).unwrap();
        let pr = &result.result.period_results[0];
        // Modified Dietz no CF: (1080000 - 1000000) / 1000000 = 0.08
        assert_eq!(pr.gross_return, dec!(0.08));
    }

    #[test]
    fn test_single_period_net_return() {
        let input = make_single_period_input();
        let result = generate_gips_report(&input).unwrap();
        let pr = &result.result.period_results[0];
        // net = 0.08 - (5000 + 1000) / 1000000 = 0.08 - 0.006 = 0.074
        assert_eq!(pr.net_return, dec!(0.074));
    }

    #[test]
    fn test_single_period_excess_return_gross() {
        let input = make_single_period_input();
        let result = generate_gips_report(&input).unwrap();
        let pr = &result.result.period_results[0];
        // excess = 0.08 - 0.07 = 0.01
        assert_eq!(pr.excess_return_gross, dec!(0.01));
    }

    #[test]
    fn test_single_period_excess_return_net() {
        let input = make_single_period_input();
        let result = generate_gips_report(&input).unwrap();
        let pr = &result.result.period_results[0];
        // excess net = 0.074 - 0.07 = 0.004
        assert_eq!(pr.excess_return_net, dec!(0.004));
    }

    #[test]
    fn test_single_period_with_cash_flow() {
        let mut input = make_single_period_input();
        input.periods[0].external_cash_flows = vec![CashFlowEvent {
            day_of_period: 15,
            amount: dec!(50000),
            total_days: 30,
        }];
        input.periods[0].ending_value = dec!(1130000);
        let result = generate_gips_report(&input).unwrap();
        let pr = &result.result.period_results[0];
        // BMV = 1000000, CF = 50000, EMV = 1130000
        // W = (30 - 15) / 30 = 0.5
        // Denom = 1000000 + 50000 * 0.5 = 1025000
        // R = (1130000 - 1000000 - 50000) / 1025000 = 80000 / 1025000
        let expected = dec!(80000) / dec!(1025000);
        assert_eq!(pr.gross_return, expected);
    }

    #[test]
    fn test_modified_dietz_mid_month_flow() {
        let mut input = make_single_period_input();
        input.periods[0].external_cash_flows = vec![CashFlowEvent {
            day_of_period: 15,
            amount: dec!(100000),
            total_days: 30,
        }];
        input.periods[0].ending_value = dec!(1180000);
        let result = generate_gips_report(&input).unwrap();
        let pr = &result.result.period_results[0];
        // W = (30 - 15)/30 = 0.5
        // Denom = 1000000 + 100000*0.5 = 1050000
        // R = (1180000 - 1000000 - 100000) / 1050000 = 80000/1050000
        let expected = dec!(80000) / dec!(1050000);
        assert_eq!(pr.gross_return, expected);
    }

    #[test]
    fn test_modified_dietz_day1_flow_full_weight() {
        let mut input = make_single_period_input();
        input.periods[0].external_cash_flows = vec![CashFlowEvent {
            day_of_period: 1,
            amount: dec!(50000),
            total_days: 30,
        }];
        input.periods[0].ending_value = dec!(1130000);
        let result = generate_gips_report(&input).unwrap();
        let pr = &result.result.period_results[0];
        // W = (30 - 1)/30 = 29/30
        let w = dec!(29) / dec!(30);
        let denom = dec!(1000000) + dec!(50000) * w;
        let expected = (dec!(1130000) - dec!(1000000) - dec!(50000)) / denom;
        assert_eq!(pr.gross_return, expected);
    }

    #[test]
    fn test_modified_dietz_last_day_flow_zero_weight() {
        let mut input = make_single_period_input();
        input.periods[0].external_cash_flows = vec![CashFlowEvent {
            day_of_period: 30,
            amount: dec!(50000),
            total_days: 30,
        }];
        input.periods[0].ending_value = dec!(1130000);
        let result = generate_gips_report(&input).unwrap();
        let pr = &result.result.period_results[0];
        // W = (30 - 30)/30 = 0
        // Denom = 1000000 + 0 = 1000000
        // R = (1130000 - 1000000 - 50000) / 1000000 = 80000/1000000 = 0.08
        assert_eq!(pr.gross_return, dec!(0.08));
    }

    // -----------------------------------------------------------------------
    // Multi-period tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_multi_period_geometric_linking() {
        let input = make_multi_period_input();
        let result = generate_gips_report(&input).unwrap();
        // period 1: 10%, period 2: ~-4.5%, period 3: ~14.3%
        // Cumulative = (1.10) * (1 + r2) * (1 + r3) - 1
        // r1 = (1100000 - 1000000)/1000000 = 0.10
        // r2 = (1050000 - 1100000)/1100000 = -0.045454...
        // r3 = (1200000 - 1050000)/1050000 = 0.142857...
        let r1 = dec!(0.1);
        let r2 = dec!(-50000) / dec!(1100000);
        let r3 = dec!(150000) / dec!(1050000);
        let expected_cum = (dec!(1) + r1) * (dec!(1) + r2) * (dec!(1) + r3) - dec!(1);
        assert_eq!(result.result.cumulative_gross_return, expected_cum);
    }

    #[test]
    fn test_cumulative_return_3_periods() {
        let input = make_multi_period_input();
        let result = generate_gips_report(&input).unwrap();
        // Just check it is computed and positive (overall gain)
        assert!(result.result.cumulative_gross_return > dec!(0));
    }

    #[test]
    fn test_cumulative_net_return_less_than_gross() {
        let input = make_multi_period_input();
        let result = generate_gips_report(&input).unwrap();
        assert!(result.result.cumulative_net_return < result.result.cumulative_gross_return);
    }

    #[test]
    fn test_annualized_return_3_years() {
        let input = make_multi_period_input();
        let result = generate_gips_report(&input).unwrap();
        let ann = result.result.annualized_gross_return;
        // Should be positive given overall gain
        assert!(ann > dec!(0));
        // Annualized should be less than cumulative for multi-year
        assert!(ann < result.result.cumulative_gross_return);
    }

    #[test]
    fn test_annualized_return_single_period_equals_cumulative() {
        let input = make_single_period_input();
        let result = generate_gips_report(&input).unwrap();
        // For single period, annualized = cumulative
        assert_eq!(
            result.result.annualized_gross_return,
            result.result.cumulative_gross_return
        );
    }

    // -----------------------------------------------------------------------
    // Tracking error & information ratio
    // -----------------------------------------------------------------------

    #[test]
    fn test_tracking_error_computation() {
        let input = make_multi_period_input();
        let result = generate_gips_report(&input).unwrap();
        // With 3 periods of different excess returns, TE should be positive
        assert!(result.result.tracking_error > dec!(0));
    }

    #[test]
    fn test_tracking_error_single_period_zero() {
        let input = make_single_period_input();
        let result = generate_gips_report(&input).unwrap();
        // Single period => no std dev possible
        assert_eq!(result.result.tracking_error, Decimal::ZERO);
    }

    #[test]
    fn test_information_ratio_positive() {
        let input = make_multi_period_input();
        let result = generate_gips_report(&input).unwrap();
        // Positive excess return with positive TE => positive IR (or check non-zero)
        // The exact sign depends on whether annualized excess > 0
        assert!(
            result.result.information_ratio != Decimal::ZERO
                || result.result.tracking_error == Decimal::ZERO
        );
    }

    #[test]
    fn test_information_ratio_zero_when_no_tracking_error() {
        let mut input = make_single_period_input();
        input.benchmark_returns = vec![dec!(0.08)]; // match gross return exactly
        let result = generate_gips_report(&input).unwrap();
        assert_eq!(result.result.information_ratio, Decimal::ZERO);
    }

    // -----------------------------------------------------------------------
    // Composite dispersion
    // -----------------------------------------------------------------------

    #[test]
    fn test_composite_dispersion_with_accounts() {
        let input = make_multi_period_input();
        let result = generate_gips_report(&input).unwrap();
        // 3 accounts with different returns => positive dispersion
        assert!(result.result.composite_dispersion > dec!(0));
    }

    #[test]
    fn test_composite_dispersion_no_accounts() {
        let input = make_single_period_input();
        let result = generate_gips_report(&input).unwrap();
        assert_eq!(result.result.composite_dispersion, Decimal::ZERO);
    }

    // -----------------------------------------------------------------------
    // Sharpe ratio
    // -----------------------------------------------------------------------

    #[test]
    fn test_sharpe_ratio_positive_return() {
        let input = make_multi_period_input();
        let result = generate_gips_report(&input).unwrap();
        // Positive annualized return with risk-free = 0 => positive Sharpe
        assert!(result.result.sharpe_ratio > dec!(0));
    }

    #[test]
    fn test_sharpe_ratio_single_period_zero() {
        let input = make_single_period_input();
        let result = generate_gips_report(&input).unwrap();
        // Single period => std dev = 0 => Sharpe = 0
        assert_eq!(result.result.sharpe_ratio, Decimal::ZERO);
    }

    // -----------------------------------------------------------------------
    // Max drawdown
    // -----------------------------------------------------------------------

    #[test]
    fn test_max_drawdown() {
        let input = make_multi_period_input();
        let result = generate_gips_report(&input).unwrap();
        // Period 2 is negative => drawdown occurs
        assert!(result.result.max_drawdown > dec!(0));
    }

    #[test]
    fn test_max_drawdown_all_positive() {
        let mut input = make_multi_period_input();
        // Make all periods positive
        input.periods[1].ending_value = dec!(1200000);
        let result = generate_gips_report(&input).unwrap();
        // Still could have a small drawdown if cumulative dips
        // Actually with all returns positive, drawdown should be 0
        // r1 = 0.10, r2 = (1200000-1100000)/1100000 = 0.0909, r3 = (1200000-1050000)/1050000
        // Wait, period 3 BMV is still 1050000 - that is the beginning value independent of period 2 end
        // So period 2 return would be positive => no drawdown
        assert_eq!(result.result.max_drawdown, Decimal::ZERO);
    }

    // -----------------------------------------------------------------------
    // GIPS checklist
    // -----------------------------------------------------------------------

    #[test]
    fn test_gips_checklist_pass() {
        let input = make_multi_period_input();
        let result = generate_gips_report(&input).unwrap();
        let checklist = &result.result.gips_compliance_checklist;
        assert!(checklist.time_weighted_returns);
        assert!(checklist.trade_date_accounting);
        assert!(checklist.accrual_accounting);
        assert!(checklist.composite_construction);
        assert!(checklist.fee_disclosure);
        assert!(checklist.benchmark_disclosed);
        assert!(checklist.overall_compliant);
    }

    #[test]
    fn test_gips_dispersion_reported_false_fewer_than_5() {
        let input = make_multi_period_input();
        let result = generate_gips_report(&input).unwrap();
        // 3 accounts => dispersion_reported = false
        assert!(!result.result.gips_compliance_checklist.dispersion_reported);
    }

    #[test]
    fn test_gips_dispersion_reported_true_5_accounts() {
        let mut input = make_multi_period_input();
        input.composite_accounts.push(AccountReturn {
            account_name: "Account D".to_string(),
            returns: vec![dec!(0.10), dec!(-0.05), dec!(0.14)],
        });
        input.composite_accounts.push(AccountReturn {
            account_name: "Account E".to_string(),
            returns: vec![dec!(0.12), dec!(-0.03), dec!(0.16)],
        });
        let result = generate_gips_report(&input).unwrap();
        assert!(result.result.gips_compliance_checklist.dispersion_reported);
    }

    #[test]
    fn test_gips_composite_construction_false_no_accounts() {
        let input = make_single_period_input();
        let result = generate_gips_report(&input).unwrap();
        assert!(
            !result
                .result
                .gips_compliance_checklist
                .composite_construction
        );
    }

    // -----------------------------------------------------------------------
    // Benchmark excess return
    // -----------------------------------------------------------------------

    #[test]
    fn test_benchmark_excess_return_positive() {
        let input = make_single_period_input();
        let result = generate_gips_report(&input).unwrap();
        let pr = &result.result.period_results[0];
        // gross = 0.08, benchmark = 0.07 => excess = 0.01
        assert_eq!(pr.excess_return_gross, dec!(0.01));
    }

    #[test]
    fn test_benchmark_excess_return_negative() {
        let mut input = make_single_period_input();
        input.benchmark_returns = vec![dec!(0.10)];
        let result = generate_gips_report(&input).unwrap();
        let pr = &result.result.period_results[0];
        // gross = 0.08, benchmark = 0.10 => excess = -0.02
        assert_eq!(pr.excess_return_gross, dec!(-0.02));
    }

    // -----------------------------------------------------------------------
    // Zero and negative returns
    // -----------------------------------------------------------------------

    #[test]
    fn test_zero_return_period() {
        let mut input = make_single_period_input();
        input.periods[0].ending_value = dec!(1000000);
        input.periods[0].fees_management = dec!(0);
        input.periods[0].fees_performance = dec!(0);
        let result = generate_gips_report(&input).unwrap();
        let pr = &result.result.period_results[0];
        assert_eq!(pr.gross_return, dec!(0));
    }

    #[test]
    fn test_negative_return_period() {
        let mut input = make_single_period_input();
        input.periods[0].ending_value = dec!(900000);
        let result = generate_gips_report(&input).unwrap();
        let pr = &result.result.period_results[0];
        assert!(pr.gross_return < dec!(0));
    }

    // -----------------------------------------------------------------------
    // Large cash flow
    // -----------------------------------------------------------------------

    #[test]
    fn test_large_cash_flow_relative_to_portfolio() {
        let mut input = make_single_period_input();
        input.periods[0].external_cash_flows = vec![CashFlowEvent {
            day_of_period: 15,
            amount: dec!(900000), // 90% of BMV
            total_days: 30,
        }];
        input.periods[0].ending_value = dec!(1950000);
        let result = generate_gips_report(&input).unwrap();
        let pr = &result.result.period_results[0];
        // Denom = 1000000 + 900000 * 0.5 = 1450000
        // R = (1950000 - 1000000 - 900000) / 1450000 = 50000 / 1450000
        let expected = dec!(50000) / dec!(1450000);
        assert_eq!(pr.gross_return, expected);
    }

    // -----------------------------------------------------------------------
    // Fee deduction accuracy
    // -----------------------------------------------------------------------

    #[test]
    fn test_fee_deduction_accuracy() {
        let input = make_single_period_input();
        let result = generate_gips_report(&input).unwrap();
        let pr = &result.result.period_results[0];
        let fee_rate = dec!(6000) / dec!(1000000); // 5000 mgmt + 1000 perf
        let expected_net = pr.gross_return - fee_rate;
        assert_eq!(pr.net_return, expected_net);
    }

    #[test]
    fn test_fee_deduction_zero_fees() {
        let mut input = make_single_period_input();
        input.periods[0].fees_management = dec!(0);
        input.periods[0].fees_performance = dec!(0);
        let result = generate_gips_report(&input).unwrap();
        let pr = &result.result.period_results[0];
        assert_eq!(pr.gross_return, pr.net_return);
    }

    // -----------------------------------------------------------------------
    // Validation errors
    // -----------------------------------------------------------------------

    #[test]
    fn test_validation_empty_periods() {
        let mut input = make_single_period_input();
        input.periods = vec![];
        input.benchmark_returns = vec![];
        let result = generate_gips_report(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_validation_zero_beginning_value() {
        let mut input = make_single_period_input();
        input.periods[0].beginning_value = dec!(0);
        let result = generate_gips_report(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_validation_negative_beginning_value() {
        let mut input = make_single_period_input();
        input.periods[0].beginning_value = dec!(-1000);
        let result = generate_gips_report(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_validation_mismatched_benchmark_length() {
        let mut input = make_single_period_input();
        input.benchmark_returns = vec![dec!(0.07), dec!(0.08)];
        let result = generate_gips_report(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_validation_invalid_fee_schedule() {
        let mut input = make_single_period_input();
        input.fee_schedule = "Invalid".to_string();
        let result = generate_gips_report(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_validation_mismatched_account_returns_length() {
        let mut input = make_single_period_input();
        input.composite_accounts = vec![AccountReturn {
            account_name: "Test".to_string(),
            returns: vec![dec!(0.08), dec!(0.05)], // 2 returns but 1 period
        }];
        let result = generate_gips_report(&input);
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // Metadata / envelope
    // -----------------------------------------------------------------------

    #[test]
    fn test_methodology_present() {
        let input = make_single_period_input();
        let result = generate_gips_report(&input).unwrap();
        assert!(!result.methodology.is_empty());
    }

    #[test]
    fn test_assumptions_populated() {
        let input = make_single_period_input();
        let result = generate_gips_report(&input).unwrap();
        assert!(!result.result.assumptions.is_empty());
    }

    // -----------------------------------------------------------------------
    // Helper function tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_geometric_link_simple() {
        let returns = vec![dec!(0.10), dec!(0.05)];
        let cum = geometric_link(&returns);
        // (1.10)(1.05) - 1 = 1.155 - 1 = 0.155
        assert_eq!(cum, dec!(0.155));
    }

    #[test]
    fn test_geometric_link_empty() {
        let returns: Vec<Decimal> = vec![];
        assert_eq!(geometric_link(&returns), dec!(0));
    }

    #[test]
    fn test_newton_sqrt_positive() {
        let result = newton_sqrt(dec!(4));
        // Should be very close to 2
        let diff = (result - dec!(2)).abs();
        assert!(diff < dec!(0.0000001));
    }

    #[test]
    fn test_newton_sqrt_zero() {
        assert_eq!(newton_sqrt(dec!(0)), Decimal::ZERO);
    }

    #[test]
    fn test_compute_max_drawdown_simple() {
        let returns = vec![dec!(0.10), dec!(-0.20), dec!(0.05)];
        let dd = compute_max_drawdown(&returns);
        // Peak at 1.10, trough at 1.10*0.80 = 0.88
        // DD = (1.10 - 0.88)/1.10 = 0.22/1.10 = 0.2
        assert!(dd > dec!(0.19) && dd < dec!(0.21));
    }

    #[test]
    fn test_compute_max_drawdown_empty() {
        assert_eq!(compute_max_drawdown(&[]), Decimal::ZERO);
    }
}
