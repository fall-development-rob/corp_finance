use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::error::CorpFinanceError;
use crate::types::{with_metadata, ComputationOutput};
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Input types
// ---------------------------------------------------------------------------

/// Type of venture exit event.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ExitType {
    Ipo,
    Acquisition,
    Secondary,
    WriteOff,
    Unrealised,
}

/// A single portfolio company investment within the venture fund.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VentureInvestment {
    pub company_name: String,
    /// Capital deployed into this company
    pub investment_amount: Decimal,
    /// Year within fund life when capital was deployed (1-indexed)
    pub investment_year: u32,
    /// Year within fund life when exit occurs (None = total loss or unrealised)
    pub exit_year: Option<u32>,
    /// Return multiple on this investment (e.g. 10.0 = 10x)
    pub exit_multiple: Option<Decimal>,
    /// Classification of the exit event
    pub exit_type: ExitType,
}

/// Input for venture fund returns modelling.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VentureFundInput {
    /// Total LP commitments
    pub fund_size: Decimal,
    /// Annual management fee as decimal (0.02 = 2%)
    pub management_fee_rate: Decimal,
    /// Carried interest rate (0.20 = 20%)
    pub carry_rate: Decimal,
    /// Preferred return hurdle (0.08 = 8%)
    pub hurdle_rate: Decimal,
    /// Total fund life in years (typically 10)
    pub fund_life_years: u32,
    /// Investment period in years (typically 3-5)
    pub investment_period_years: u32,
    /// Portfolio companies
    pub investments: Vec<VentureInvestment>,
    /// Fraction of early returns that can be reinvested (0.10 = 10%)
    pub recycling_rate: Decimal,
}

// ---------------------------------------------------------------------------
// Output types
// ---------------------------------------------------------------------------

/// Per-year cashflow detail — traces the J-curve.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YearlyCashflow {
    pub year: u32,
    /// Capital called from LPs this year
    pub capital_called: Decimal,
    /// Management fees charged this year
    pub management_fees: Decimal,
    /// Capital actually deployed to companies this year
    pub invested: Decimal,
    /// Distributions returned to LPs this year
    pub distributions: Decimal,
    /// Net cashflow = distributions - capital_called
    pub net_cashflow: Decimal,
    /// Running sum of net cashflows (the J-curve line)
    pub cumulative_net: Decimal,
    /// Estimated fund NAV at year end
    pub nav: Decimal,
    /// Distributions to paid-in
    pub dpi: Decimal,
    /// Total value to paid-in (distributions + NAV) / paid-in
    pub tvpi: Decimal,
    /// Residual value to paid-in (NAV / paid-in)
    pub rvpi: Decimal,
}

/// Aggregate fund-level metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FundMetrics {
    pub total_called: Decimal,
    pub total_invested: Decimal,
    pub total_distributions: Decimal,
    pub total_management_fees: Decimal,
    pub net_irr: Decimal,
    pub gross_irr: Decimal,
    pub dpi: Decimal,
    pub tvpi: Decimal,
    pub rvpi: Decimal,
    pub gross_moic: Decimal,
    pub net_moic: Decimal,
    pub carried_interest: Decimal,
    pub j_curve_trough_year: u32,
}

/// Portfolio-level statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioStats {
    pub num_investments: u32,
    pub num_exits: u32,
    pub num_writeoffs: u32,
    pub num_unrealised: u32,
    pub loss_ratio: Decimal,
    pub top_performer_multiple: Decimal,
    pub median_exit_multiple: Decimal,
    pub portfolio_concentration: Decimal,
}

/// Per-investment result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvestmentResult {
    pub company_name: String,
    pub invested: Decimal,
    pub exit_value: Decimal,
    pub multiple: Decimal,
    pub profit: Decimal,
    pub pct_of_fund_returns: Decimal,
}

/// Complete output for the venture fund model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VentureFundOutput {
    pub yearly_cashflows: Vec<YearlyCashflow>,
    pub fund_metrics: FundMetrics,
    pub portfolio_stats: PortfolioStats,
    pub investment_results: Vec<InvestmentResult>,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Newton-Raphson IRR on yearly net cashflows (negative = calls, positive = distributions).
/// Uses iterative multiplication for discount factors to avoid precision drift from `powd`.
/// All arithmetic uses checked operations to avoid overflow panics with large Decimal values.
fn compute_irr(cashflows: &[Decimal], max_iter: u32) -> Option<Decimal> {
    if cashflows.len() < 2 {
        return None;
    }

    // Check if all cashflows are non-positive (total loss) => IRR = -1
    let has_positive = cashflows.iter().any(|cf| *cf > Decimal::ZERO);
    let has_negative = cashflows.iter().any(|cf| *cf < Decimal::ZERO);
    if !has_positive || !has_negative {
        // If no positive cashflows after initial investment, IRR = -100%
        if !has_positive && has_negative {
            return Some(dec!(-1));
        }
        return None;
    }

    let mut rate = dec!(0.10); // initial guess
    let threshold = dec!(0.0000001);

    for _ in 0..max_iter {
        let mut npv_val = Decimal::ZERO;
        let mut dnpv = Decimal::ZERO;
        let one_plus_r = Decimal::ONE + rate;

        if one_plus_r.is_zero() {
            rate += dec!(0.01);
            continue;
        }

        let mut discount = Decimal::ONE; // (1+r)^0 = 1
        let mut overflow = false;

        for (t, cf) in cashflows.iter().enumerate() {
            if discount.is_zero() || overflow {
                break;
            }

            match cf.checked_div(discount) {
                Some(term) => npv_val += term,
                None => {
                    overflow = true;
                    break;
                }
            }

            if t > 0 {
                let t_dec = Decimal::from(t as i64);
                let denom = match discount.checked_mul(one_plus_r) {
                    Some(d) => d,
                    None => {
                        overflow = true;
                        break;
                    }
                };
                if !denom.is_zero() {
                    match (t_dec.checked_mul(*cf)).and_then(|n| n.checked_div(denom)) {
                        Some(term) => dnpv -= term,
                        None => {
                            overflow = true;
                            break;
                        }
                    }
                }
            }

            discount = match discount.checked_mul(one_plus_r) {
                Some(d) => d,
                None => {
                    overflow = true;
                    break;
                }
            };
        }

        // If overflow occurred, adjust rate towards zero and retry
        if overflow {
            rate /= dec!(2);
            continue;
        }

        if npv_val.abs() < threshold {
            return Some(rate);
        }

        if dnpv.is_zero() {
            return None;
        }

        match npv_val.checked_div(dnpv) {
            Some(step) => rate -= step,
            None => return None,
        }

        // Guard against divergence
        if rate < dec!(-0.99) {
            rate = dec!(-0.99);
        }
        if rate > dec!(10.0) {
            rate = dec!(10.0);
        }
    }

    // Did not converge
    None
}

/// Compute the median of a sorted slice of Decimals.
fn median_sorted(sorted: &[Decimal]) -> Decimal {
    if sorted.is_empty() {
        return Decimal::ZERO;
    }
    let n = sorted.len();
    if n % 2 == 1 {
        sorted[n / 2]
    } else {
        (sorted[n / 2 - 1] + sorted[n / 2]) / dec!(2)
    }
}

// ---------------------------------------------------------------------------
// Main function
// ---------------------------------------------------------------------------

/// Model a venture capital fund's returns including the J-curve, DPI/TVPI/RVPI
/// metrics, carried interest, and power-law portfolio statistics.
pub fn model_venture_fund(
    input: &VentureFundInput,
) -> CorpFinanceResult<ComputationOutput<VentureFundOutput>> {
    let start = Instant::now();
    let mut warnings: Vec<String> = Vec::new();

    // --- Validation ---
    if input.fund_size <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "fund_size".into(),
            reason: "Fund size must be positive".into(),
        });
    }
    if input.fund_life_years == 0 {
        return Err(CorpFinanceError::InvalidInput {
            field: "fund_life_years".into(),
            reason: "Fund life must be at least 1 year".into(),
        });
    }
    if input.investment_period_years == 0 || input.investment_period_years > input.fund_life_years {
        return Err(CorpFinanceError::InvalidInput {
            field: "investment_period_years".into(),
            reason: "Investment period must be 1..=fund_life_years".into(),
        });
    }
    if input.carry_rate < Decimal::ZERO || input.carry_rate > Decimal::ONE {
        return Err(CorpFinanceError::InvalidInput {
            field: "carry_rate".into(),
            reason: "Carry rate must be between 0 and 1".into(),
        });
    }
    if input.hurdle_rate < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "hurdle_rate".into(),
            reason: "Hurdle rate must be non-negative".into(),
        });
    }
    if input.management_fee_rate < Decimal::ZERO || input.management_fee_rate > Decimal::ONE {
        return Err(CorpFinanceError::InvalidInput {
            field: "management_fee_rate".into(),
            reason: "Management fee rate must be between 0 and 1".into(),
        });
    }
    if input.recycling_rate < Decimal::ZERO || input.recycling_rate > Decimal::ONE {
        return Err(CorpFinanceError::InvalidInput {
            field: "recycling_rate".into(),
            reason: "Recycling rate must be between 0 and 1".into(),
        });
    }

    for inv in &input.investments {
        if inv.investment_year == 0 || inv.investment_year > input.fund_life_years {
            return Err(CorpFinanceError::InvalidInput {
                field: "investment_year".into(),
                reason: format!(
                    "Investment year for {} must be 1..={}",
                    inv.company_name, input.fund_life_years
                ),
            });
        }
        if let Some(ey) = inv.exit_year {
            if ey < inv.investment_year || ey > input.fund_life_years {
                return Err(CorpFinanceError::InvalidInput {
                    field: "exit_year".into(),
                    reason: format!(
                        "Exit year for {} must be >= investment_year and <= fund_life_years",
                        inv.company_name
                    ),
                });
            }
        }
        if inv.investment_amount <= Decimal::ZERO {
            return Err(CorpFinanceError::InvalidInput {
                field: "investment_amount".into(),
                reason: format!(
                    "Investment amount for {} must be positive",
                    inv.company_name
                ),
            });
        }
    }

    // --- Build per-investment results ---
    let mut investment_results: Vec<InvestmentResult> = Vec::new();
    let mut total_exit_value = Decimal::ZERO;
    let mut total_invested_in_companies = Decimal::ZERO;

    for inv in &input.investments {
        let exit_value = match inv.exit_type {
            ExitType::WriteOff => Decimal::ZERO,
            ExitType::Unrealised => {
                // Unrealised: use exit_multiple if provided as mark-to-market,
                // otherwise assume at cost (1.0x)
                let mult = inv.exit_multiple.unwrap_or(Decimal::ONE);
                inv.investment_amount * mult
            }
            _ => {
                let mult = inv.exit_multiple.unwrap_or(Decimal::ZERO);
                inv.investment_amount * mult
            }
        };
        let profit = exit_value - inv.investment_amount;
        total_exit_value += exit_value;
        total_invested_in_companies += inv.investment_amount;

        investment_results.push(InvestmentResult {
            company_name: inv.company_name.clone(),
            invested: inv.investment_amount,
            exit_value,
            multiple: if inv.investment_amount.is_zero() {
                Decimal::ZERO
            } else {
                exit_value / inv.investment_amount
            },
            profit,
            pct_of_fund_returns: Decimal::ZERO, // filled in later
        });
    }

    // Calculate pct_of_fund_returns
    let total_fund_profit: Decimal = investment_results
        .iter()
        .filter(|r| r.profit > Decimal::ZERO)
        .map(|r| r.profit)
        .sum();

    if total_fund_profit > Decimal::ZERO {
        for result in &mut investment_results {
            if result.profit > Decimal::ZERO {
                result.pct_of_fund_returns = result.profit / total_fund_profit;
            }
        }
    }

    // --- Build yearly cashflows (J-curve) ---
    // Pre-compute per-year investment deployments and exit proceeds
    let n_years = input.fund_life_years as usize;
    let mut yearly_deployments = vec![Decimal::ZERO; n_years + 1]; // index 0 unused
    let mut yearly_exit_proceeds = vec![Decimal::ZERO; n_years + 1];

    for inv in &input.investments {
        yearly_deployments[inv.investment_year as usize] += inv.investment_amount;

        if inv.exit_type != ExitType::WriteOff && inv.exit_type != ExitType::Unrealised {
            if let (Some(ey), Some(mult)) = (inv.exit_year, inv.exit_multiple) {
                yearly_exit_proceeds[ey as usize] += inv.investment_amount * mult;
            }
        }
    }

    let mut yearly_cashflows: Vec<YearlyCashflow> = Vec::with_capacity(n_years);
    let mut cumulative_called = Decimal::ZERO;
    let mut cumulative_invested = Decimal::ZERO;
    let mut cumulative_distributions = Decimal::ZERO;
    let mut cumulative_fees = Decimal::ZERO;
    let mut cumulative_net = Decimal::ZERO;
    let mut recycled_capital = Decimal::ZERO;

    // Track NAV: sum of unrealised + not-yet-exited investments at cost (or mark)
    // We track active investments: deployed but not yet exited/written-off
    // Build a map of (investment_year, exit_year|None) -> amount at cost
    // For NAV, we value unrealised at their mark, active at cost
    let active_investments: Vec<(u32, Option<u32>, Decimal, &VentureInvestment)> = input
        .investments
        .iter()
        .map(|inv| {
            (
                inv.investment_year,
                inv.exit_year,
                inv.investment_amount,
                inv,
            )
        })
        .collect();

    // Net cashflows for LP IRR (year 0..=fund_life_years)
    // Year 0: no activity typically, but we include for IRR array alignment
    let mut lp_net_cashflows: Vec<Decimal> = vec![Decimal::ZERO; n_years + 1];
    // Gross cashflows: investment out vs exit proceeds in (no fees/carry)
    let mut gross_cashflows: Vec<Decimal> = vec![Decimal::ZERO; n_years + 1];

    for year in 1..=input.fund_life_years {
        let y = year as usize;
        let in_investment_period = year <= input.investment_period_years;

        // --- Management fees ---
        // During investment period: fee on total commitments
        // After investment period: fee on invested capital (net invested)
        let fee_base = if in_investment_period {
            input.fund_size
        } else {
            cumulative_invested
        };
        let mgmt_fee = fee_base * input.management_fee_rate;

        // --- Capital deployment ---
        let base_deployment = yearly_deployments[y];
        // Add recycled capital to deployment (only during investment period)
        let recycled_deploy = if in_investment_period {
            let available = recycled_capital;
            recycled_capital = Decimal::ZERO;
            available
        } else {
            Decimal::ZERO
        };
        let actual_deployed = base_deployment + recycled_deploy;

        // --- Capital called = deployment + fees (minus any recycled which doesn't need calling) ---
        // LPs provide: fees + new deployments (not recycled portion)
        let capital_called = mgmt_fee + base_deployment;

        cumulative_called += capital_called;
        cumulative_invested += actual_deployed;
        cumulative_fees += mgmt_fee;

        // --- Exit proceeds ---
        let gross_proceeds = yearly_exit_proceeds[y];

        // Recycling: a fraction of early exit proceeds can be reinvested
        let recycling_amount = if in_investment_period {
            gross_proceeds * input.recycling_rate
        } else {
            Decimal::ZERO
        };
        recycled_capital += recycling_amount;

        // Distributions to LPs = gross proceeds - recycled amount
        let distributions = gross_proceeds - recycling_amount;
        cumulative_distributions += distributions;

        // --- Net cashflow from LP perspective ---
        let net_cf = distributions - capital_called;
        cumulative_net += net_cf;

        // --- NAV calculation ---
        // NAV = sum of active investments valued at cost or mark
        let mut nav = Decimal::ZERO;
        for (inv_year, exit_year, amount, inv) in &active_investments {
            if year >= *inv_year {
                let has_exited = match exit_year {
                    Some(ey) => year >= *ey,
                    None => inv.exit_type == ExitType::WriteOff,
                };
                if !has_exited {
                    // Value at cost for active, at mark for unrealised with multiple
                    if inv.exit_type == ExitType::Unrealised {
                        let mult = inv.exit_multiple.unwrap_or(Decimal::ONE);
                        nav += *amount * mult;
                    } else {
                        // Pre-exit: value at cost
                        nav += *amount;
                    }
                }
            }
        }

        // Compute period metrics
        let dpi = if cumulative_called.is_zero() {
            Decimal::ZERO
        } else {
            cumulative_distributions / cumulative_called
        };

        let tvpi = if cumulative_called.is_zero() {
            Decimal::ZERO
        } else {
            (cumulative_distributions + nav) / cumulative_called
        };

        let rvpi = if cumulative_called.is_zero() {
            Decimal::ZERO
        } else {
            nav / cumulative_called
        };

        yearly_cashflows.push(YearlyCashflow {
            year,
            capital_called,
            management_fees: mgmt_fee,
            invested: actual_deployed,
            distributions,
            net_cashflow: net_cf,
            cumulative_net,
            nav,
            dpi,
            tvpi,
            rvpi,
        });

        // LP cashflow for IRR: negative when calling, positive when distributing
        lp_net_cashflows[y] = net_cf;

        // Gross cashflow: deployments out (negative), exit proceeds in (positive)
        gross_cashflows[y] = gross_proceeds - actual_deployed;
    }

    // --- Fund metrics ---
    let total_called = cumulative_called;
    let total_invested = cumulative_invested;
    let total_distributions = cumulative_distributions;
    let total_management_fees = cumulative_fees;

    // Gross MOIC: total exit value / total invested in companies
    let gross_moic = if total_invested_in_companies.is_zero() {
        Decimal::ZERO
    } else {
        total_exit_value / total_invested_in_companies
    };

    // Final NAV (from last year)
    let final_nav = yearly_cashflows
        .last()
        .map(|yc| yc.nav)
        .unwrap_or(Decimal::ZERO);

    // Final DPI, TVPI, RVPI
    let final_dpi = if total_called.is_zero() {
        Decimal::ZERO
    } else {
        total_distributions / total_called
    };

    let final_tvpi = if total_called.is_zero() {
        Decimal::ZERO
    } else {
        (total_distributions + final_nav) / total_called
    };

    let final_rvpi = if total_called.is_zero() {
        Decimal::ZERO
    } else {
        final_nav / total_called
    };

    // --- Carried interest ---
    // Carry is computed on total profit above the hurdle
    // LP total value = distributions + NAV
    // LP profit = total value - total called
    // Hurdle amount = total_called * hurdle_rate * fund_life_years (simple)
    // Carry = carry_rate * max(0, LP_profit - hurdle_amount)
    let lp_total_value = total_distributions + final_nav;
    let lp_profit = lp_total_value - total_called;
    let hurdle_amount = total_called * input.hurdle_rate * Decimal::from(input.fund_life_years);
    let carry_eligible = (lp_profit - hurdle_amount).max(Decimal::ZERO);
    let carried_interest = carry_eligible * input.carry_rate;

    // Net distributions after carry
    let net_distributions = total_distributions - carried_interest;

    // Net MOIC: LP net / LP called
    let net_moic = if total_called.is_zero() {
        Decimal::ZERO
    } else {
        (net_distributions + final_nav) / total_called
    };

    // --- IRR calculations ---
    // LP net IRR: include carry deduction in final year
    let mut lp_cashflows_for_irr = lp_net_cashflows.clone();
    // Deduct carried interest from the last year's cashflow
    if !lp_cashflows_for_irr.is_empty() {
        let last_idx = lp_cashflows_for_irr.len() - 1;
        lp_cashflows_for_irr[last_idx] -= carried_interest;
        // Add terminal NAV to last year for IRR purposes
        lp_cashflows_for_irr[last_idx] += final_nav;
    }

    let net_irr = compute_irr(&lp_cashflows_for_irr, 50).unwrap_or_else(|| {
        warnings.push("Net IRR did not converge".into());
        Decimal::ZERO
    });

    // Gross IRR: fund-level cashflows (no fees, no carry)
    let mut gross_cfs_for_irr = gross_cashflows.clone();
    if !gross_cfs_for_irr.is_empty() {
        let last_idx = gross_cfs_for_irr.len() - 1;
        // Add unrealised NAV at final year for gross IRR
        let unrealised_nav: Decimal = input
            .investments
            .iter()
            .filter(|inv| inv.exit_type == ExitType::Unrealised)
            .map(|inv| inv.investment_amount * inv.exit_multiple.unwrap_or(Decimal::ONE))
            .sum();
        gross_cfs_for_irr[last_idx] += unrealised_nav;
    }

    let gross_irr = compute_irr(&gross_cfs_for_irr, 50).unwrap_or_else(|| {
        warnings.push("Gross IRR did not converge".into());
        Decimal::ZERO
    });

    // J-curve trough year
    let j_curve_trough_year = yearly_cashflows
        .iter()
        .min_by(|a, b| a.cumulative_net.cmp(&b.cumulative_net))
        .map(|yc| yc.year)
        .unwrap_or(1);

    let fund_metrics = FundMetrics {
        total_called,
        total_invested,
        total_distributions,
        total_management_fees,
        net_irr,
        gross_irr,
        dpi: final_dpi,
        tvpi: final_tvpi,
        rvpi: final_rvpi,
        gross_moic,
        net_moic,
        carried_interest,
        j_curve_trough_year,
    };

    // --- Portfolio stats ---
    let num_investments = input.investments.len() as u32;
    let num_exits = input
        .investments
        .iter()
        .filter(|inv| {
            inv.exit_type != ExitType::WriteOff
                && inv.exit_type != ExitType::Unrealised
                && inv.exit_year.is_some()
        })
        .count() as u32;
    let num_writeoffs = input
        .investments
        .iter()
        .filter(|inv| inv.exit_type == ExitType::WriteOff)
        .count() as u32;
    let num_unrealised = input
        .investments
        .iter()
        .filter(|inv| inv.exit_type == ExitType::Unrealised)
        .count() as u32;

    let loss_ratio = if num_investments == 0 {
        Decimal::ZERO
    } else {
        Decimal::from(num_writeoffs) / Decimal::from(num_investments)
    };

    // Exit multiples for exited investments (not writeoffs, not unrealised)
    let mut exit_multiples: Vec<Decimal> = input
        .investments
        .iter()
        .filter(|inv| {
            inv.exit_type != ExitType::WriteOff
                && inv.exit_type != ExitType::Unrealised
                && inv.exit_multiple.is_some()
        })
        .map(|inv| inv.exit_multiple.unwrap())
        .collect();
    exit_multiples.sort();

    let top_performer_multiple = exit_multiples.last().copied().unwrap_or(Decimal::ZERO);
    let median_exit_multiple = median_sorted(&exit_multiples);

    // Portfolio concentration: largest single exit proceeds as % of total exit proceeds
    let exited_proceeds: Vec<Decimal> = input
        .investments
        .iter()
        .filter(|inv| {
            inv.exit_type != ExitType::WriteOff
                && inv.exit_type != ExitType::Unrealised
                && inv.exit_year.is_some()
        })
        .map(|inv| inv.investment_amount * inv.exit_multiple.unwrap_or(Decimal::ZERO))
        .collect();
    let total_exited: Decimal = exited_proceeds.iter().sum();
    let max_exit = exited_proceeds
        .iter()
        .max()
        .copied()
        .unwrap_or(Decimal::ZERO);
    let portfolio_concentration = if total_exited.is_zero() {
        Decimal::ZERO
    } else {
        max_exit / total_exited
    };

    let portfolio_stats = PortfolioStats {
        num_investments,
        num_exits,
        num_writeoffs,
        num_unrealised,
        loss_ratio,
        top_performer_multiple,
        median_exit_multiple,
        portfolio_concentration,
    };

    let output = VentureFundOutput {
        yearly_cashflows,
        fund_metrics,
        portfolio_stats,
        investment_results,
    };

    let elapsed = start.elapsed().as_micros() as u64;
    Ok(with_metadata(
        "Venture Fund Returns: J-Curve, DPI/TVPI/RVPI, Power-Law Portfolio",
        &serde_json::json!({
            "fund_size": input.fund_size.to_string(),
            "fund_life_years": input.fund_life_years,
            "investment_period_years": input.investment_period_years,
            "num_investments": input.investments.len(),
            "management_fee_rate": input.management_fee_rate.to_string(),
            "carry_rate": input.carry_rate.to_string(),
            "hurdle_rate": input.hurdle_rate.to_string(),
            "recycling_rate": input.recycling_rate.to_string(),
        }),
        warnings,
        elapsed,
        output,
    ))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    /// Helper: create a standard 10-year fund with given investments.
    fn make_fund(investments: Vec<VentureInvestment>) -> VentureFundInput {
        VentureFundInput {
            fund_size: dec!(100_000_000),
            management_fee_rate: dec!(0.02),
            carry_rate: dec!(0.20),
            hurdle_rate: dec!(0.08),
            fund_life_years: 10,
            investment_period_years: 5,
            investments,
            recycling_rate: dec!(0.0),
        }
    }

    fn inv(
        name: &str,
        amount: Decimal,
        inv_year: u32,
        exit_year: Option<u32>,
        exit_multiple: Option<Decimal>,
        exit_type: ExitType,
    ) -> VentureInvestment {
        VentureInvestment {
            company_name: name.to_string(),
            investment_amount: amount,
            investment_year: inv_year,
            exit_year,
            exit_multiple,
            exit_type,
        }
    }

    // -----------------------------------------------------------------------
    // 1. Simple fund: 3 investments, 1 write-off, 1 3x exit, 1 10x exit
    // -----------------------------------------------------------------------
    #[test]
    fn test_simple_fund_three_investments() {
        let input = make_fund(vec![
            inv(
                "Alpha",
                dec!(10_000_000),
                1,
                Some(7),
                Some(dec!(10)),
                ExitType::Ipo,
            ),
            inv(
                "Beta",
                dec!(10_000_000),
                2,
                Some(6),
                Some(dec!(3)),
                ExitType::Acquisition,
            ),
            inv("Gamma", dec!(10_000_000), 1, None, None, ExitType::WriteOff),
        ]);
        let result = model_venture_fund(&input).unwrap();
        let out = &result.result;

        // 3 investments
        assert_eq!(out.portfolio_stats.num_investments, 3);
        // 2 exits (Alpha + Beta)
        assert_eq!(out.portfolio_stats.num_exits, 2);
        // 1 writeoff
        assert_eq!(out.portfolio_stats.num_writeoffs, 1);

        // Alpha: 10M * 10x = 100M exit, Beta: 10M * 3x = 30M exit
        let alpha = out
            .investment_results
            .iter()
            .find(|r| r.company_name == "Alpha")
            .unwrap();
        assert_eq!(alpha.exit_value, dec!(100_000_000));
        assert_eq!(alpha.multiple, dec!(10));

        let beta = out
            .investment_results
            .iter()
            .find(|r| r.company_name == "Beta")
            .unwrap();
        assert_eq!(beta.exit_value, dec!(30_000_000));
        assert_eq!(beta.multiple, dec!(3));

        let gamma = out
            .investment_results
            .iter()
            .find(|r| r.company_name == "Gamma")
            .unwrap();
        assert_eq!(gamma.exit_value, Decimal::ZERO);
        assert_eq!(gamma.multiple, Decimal::ZERO);

        // Gross MOIC should be > 1 (we made money overall)
        assert!(out.fund_metrics.gross_moic > Decimal::ONE);
    }

    // -----------------------------------------------------------------------
    // 2. J-curve: cumulative net negative in early years, positive later
    // -----------------------------------------------------------------------
    #[test]
    fn test_j_curve_shape() {
        let input = make_fund(vec![
            inv(
                "A",
                dec!(20_000_000),
                1,
                Some(8),
                Some(dec!(5)),
                ExitType::Ipo,
            ),
            inv(
                "B",
                dec!(15_000_000),
                2,
                Some(9),
                Some(dec!(3)),
                ExitType::Acquisition,
            ),
        ]);
        let result = model_venture_fund(&input).unwrap();
        let cashflows = &result.result.yearly_cashflows;

        // Early years should have negative cumulative net (fees + investments, no returns)
        assert!(
            cashflows[0].cumulative_net < Decimal::ZERO,
            "Year 1 cumulative should be negative"
        );
        assert!(
            cashflows[1].cumulative_net < Decimal::ZERO,
            "Year 2 cumulative should be negative"
        );

        // After exits, cumulative should turn positive
        let last = cashflows.last().unwrap();
        assert!(
            last.cumulative_net > Decimal::ZERO,
            "Final cumulative should be positive"
        );
    }

    // -----------------------------------------------------------------------
    // 3. DPI/TVPI/RVPI calculations
    // -----------------------------------------------------------------------
    #[test]
    fn test_dpi_tvpi_rvpi() {
        let input = make_fund(vec![inv(
            "A",
            dec!(20_000_000),
            1,
            Some(5),
            Some(dec!(4)),
            ExitType::Ipo,
        )]);
        let result = model_venture_fund(&input).unwrap();
        let metrics = &result.result.fund_metrics;

        // DPI = total distributions / total called, should be > 0
        assert!(
            metrics.dpi > Decimal::ZERO,
            "DPI should be positive after exits"
        );

        // TVPI >= DPI (TVPI includes NAV)
        assert!(metrics.tvpi >= metrics.dpi, "TVPI should be >= DPI");

        // DPI + RVPI should approximately equal TVPI
        let sum = metrics.dpi + metrics.rvpi;
        assert!(
            (sum - metrics.tvpi).abs() < dec!(0.001),
            "DPI + RVPI should equal TVPI, got {} + {} vs {}",
            metrics.dpi,
            metrics.rvpi,
            metrics.tvpi
        );
    }

    // -----------------------------------------------------------------------
    // 4. Management fee calculation (investment period vs post-investment)
    // -----------------------------------------------------------------------
    #[test]
    fn test_management_fees() {
        let input = make_fund(vec![inv(
            "A",
            dec!(30_000_000),
            1,
            Some(10),
            Some(dec!(3)),
            ExitType::Acquisition,
        )]);
        let result = model_venture_fund(&input).unwrap();
        let cashflows = &result.result.yearly_cashflows;

        // During investment period (years 1-5): fee = fund_size * 0.02 = 2M
        for y in 0..5 {
            assert_eq!(
                cashflows[y].management_fees,
                dec!(2_000_000),
                "Year {} fee should be 2M (investment period)",
                y + 1
            );
        }

        // After investment period (year 6+): fee = cumulative_invested * 0.02
        // By year 6, invested = 30M, so fee = 30M * 0.02 = 600K
        assert_eq!(
            cashflows[5].management_fees,
            dec!(600_000),
            "Year 6 fee should be based on invested capital"
        );
    }

    // -----------------------------------------------------------------------
    // 5. Carried interest above hurdle
    // -----------------------------------------------------------------------
    #[test]
    fn test_carried_interest_above_hurdle() {
        let input = make_fund(vec![
            inv(
                "A",
                dec!(20_000_000),
                1,
                Some(5),
                Some(dec!(10)),
                ExitType::Ipo,
            ),
            inv(
                "B",
                dec!(10_000_000),
                2,
                Some(6),
                Some(dec!(5)),
                ExitType::Acquisition,
            ),
        ]);
        let result = model_venture_fund(&input).unwrap();
        let metrics = &result.result.fund_metrics;

        // With big exits (10x and 5x), we should have carry above hurdle
        assert!(
            metrics.carried_interest > Decimal::ZERO,
            "Carry should be positive with high returns"
        );
    }

    // -----------------------------------------------------------------------
    // 6. No carry when below hurdle
    // -----------------------------------------------------------------------
    #[test]
    fn test_no_carry_below_hurdle() {
        // Small exit that barely returns capital — below 8% hurdle over 10 years
        let input = make_fund(vec![inv(
            "A",
            dec!(10_000_000),
            1,
            Some(10),
            Some(dec!(1.05)),
            ExitType::Acquisition,
        )]);
        let result = model_venture_fund(&input).unwrap();
        let metrics = &result.result.fund_metrics;

        // 1.05x on 10M = 10.5M exit, but total called includes fees
        // Hurdle amount = total_called * 0.08 * 10 = very large
        // Profit after fees is tiny, well below hurdle
        assert_eq!(
            metrics.carried_interest,
            Decimal::ZERO,
            "No carry when returns are below hurdle"
        );
    }

    // -----------------------------------------------------------------------
    // 7. Gross vs net IRR (net should be lower)
    // -----------------------------------------------------------------------
    #[test]
    fn test_gross_vs_net_irr() {
        let input = make_fund(vec![
            inv(
                "A",
                dec!(20_000_000),
                1,
                Some(6),
                Some(dec!(5)),
                ExitType::Ipo,
            ),
            inv(
                "B",
                dec!(15_000_000),
                2,
                Some(7),
                Some(dec!(3)),
                ExitType::Acquisition,
            ),
        ]);
        let result = model_venture_fund(&input).unwrap();
        let metrics = &result.result.fund_metrics;

        // Net IRR should be less than or equal to gross IRR (fees and carry reduce returns)
        assert!(
            metrics.net_irr <= metrics.gross_irr,
            "Net IRR ({}) should be <= gross IRR ({})",
            metrics.net_irr,
            metrics.gross_irr
        );
    }

    // -----------------------------------------------------------------------
    // 8. Loss ratio with writeoffs
    // -----------------------------------------------------------------------
    #[test]
    fn test_loss_ratio() {
        let input = make_fund(vec![
            inv(
                "A",
                dec!(10_000_000),
                1,
                Some(5),
                Some(dec!(5)),
                ExitType::Ipo,
            ),
            inv("B", dec!(10_000_000), 2, None, None, ExitType::WriteOff),
            inv("C", dec!(10_000_000), 3, None, None, ExitType::WriteOff),
            inv(
                "D",
                dec!(10_000_000),
                1,
                Some(6),
                Some(dec!(2)),
                ExitType::Acquisition,
            ),
        ]);
        let result = model_venture_fund(&input).unwrap();
        let stats = &result.result.portfolio_stats;

        // 2 out of 4 are writeoffs = 50%
        assert_eq!(stats.loss_ratio, dec!(0.5));
        assert_eq!(stats.num_writeoffs, 2);
    }

    // -----------------------------------------------------------------------
    // 9. Portfolio concentration (one big winner)
    // -----------------------------------------------------------------------
    #[test]
    fn test_portfolio_concentration() {
        let input = make_fund(vec![
            inv(
                "BigWinner",
                dec!(10_000_000),
                1,
                Some(5),
                Some(dec!(50)),
                ExitType::Ipo,
            ),
            inv(
                "SmallWin",
                dec!(10_000_000),
                2,
                Some(6),
                Some(dec!(2)),
                ExitType::Acquisition,
            ),
        ]);
        let result = model_venture_fund(&input).unwrap();
        let stats = &result.result.portfolio_stats;

        // BigWinner: 10M * 50 = 500M, SmallWin: 10M * 2 = 20M
        // Concentration = 500M / 520M ~= 0.9615
        let expected_conc = dec!(500_000_000) / dec!(520_000_000);
        assert!(
            (stats.portfolio_concentration - expected_conc).abs() < dec!(0.001),
            "Concentration should be ~96%, got {}",
            stats.portfolio_concentration
        );
    }

    // -----------------------------------------------------------------------
    // 10. Median exit multiple
    // -----------------------------------------------------------------------
    #[test]
    fn test_median_exit_multiple() {
        let input = make_fund(vec![
            inv(
                "A",
                dec!(5_000_000),
                1,
                Some(5),
                Some(dec!(2)),
                ExitType::Acquisition,
            ),
            inv(
                "B",
                dec!(5_000_000),
                2,
                Some(6),
                Some(dec!(5)),
                ExitType::Ipo,
            ),
            inv(
                "C",
                dec!(5_000_000),
                3,
                Some(7),
                Some(dec!(10)),
                ExitType::Ipo,
            ),
        ]);
        let result = model_venture_fund(&input).unwrap();
        let stats = &result.result.portfolio_stats;

        // Sorted multiples: [2, 5, 10], median = 5
        assert_eq!(stats.median_exit_multiple, dec!(5));
    }

    // -----------------------------------------------------------------------
    // 11. Fund with all writeoffs (IRR should reflect total loss)
    // -----------------------------------------------------------------------
    #[test]
    fn test_all_writeoffs() {
        let input = make_fund(vec![
            inv("A", dec!(10_000_000), 1, None, None, ExitType::WriteOff),
            inv("B", dec!(10_000_000), 2, None, None, ExitType::WriteOff),
            inv("C", dec!(10_000_000), 3, None, None, ExitType::WriteOff),
        ]);
        let result = model_venture_fund(&input).unwrap();
        let metrics = &result.result.fund_metrics;

        // All writeoffs: DPI = 0
        assert_eq!(metrics.dpi, Decimal::ZERO);
        assert_eq!(metrics.total_distributions, Decimal::ZERO);
        assert_eq!(metrics.carried_interest, Decimal::ZERO);

        // Gross MOIC = 0 (all exits worth 0)
        assert_eq!(metrics.gross_moic, Decimal::ZERO);

        // Loss ratio = 100%
        assert_eq!(result.result.portfolio_stats.loss_ratio, Decimal::ONE);

        // Net IRR should be -100% (or very close)
        assert!(
            metrics.net_irr <= dec!(-0.90),
            "Net IRR should reflect near-total loss, got {}",
            metrics.net_irr
        );
    }

    // -----------------------------------------------------------------------
    // 12. Fund with no exits yet (RVPI > 0, DPI = 0)
    // -----------------------------------------------------------------------
    #[test]
    fn test_no_exits_yet() {
        let input = make_fund(vec![
            inv(
                "A",
                dec!(15_000_000),
                1,
                None,
                Some(dec!(2)),
                ExitType::Unrealised,
            ),
            inv(
                "B",
                dec!(10_000_000),
                2,
                None,
                Some(dec!(3)),
                ExitType::Unrealised,
            ),
        ]);
        let result = model_venture_fund(&input).unwrap();
        let metrics = &result.result.fund_metrics;

        // No exits => DPI = 0
        assert_eq!(metrics.dpi, Decimal::ZERO);
        assert_eq!(metrics.total_distributions, Decimal::ZERO);

        // But we have unrealised value => RVPI > 0
        assert!(
            metrics.rvpi > Decimal::ZERO,
            "RVPI should be positive with unrealised"
        );

        // TVPI should equal RVPI when DPI = 0
        assert_eq!(metrics.tvpi, metrics.rvpi);

        // No carry since no distributions
        assert_eq!(metrics.carried_interest, Decimal::ZERO);
    }

    // -----------------------------------------------------------------------
    // 13. Recycling: early exits reinvested
    // -----------------------------------------------------------------------
    #[test]
    fn test_recycling() {
        let mut input = make_fund(vec![
            inv(
                "Early",
                dec!(10_000_000),
                1,
                Some(3),
                Some(dec!(3)),
                ExitType::Acquisition,
            ),
            inv(
                "Later",
                dec!(10_000_000),
                2,
                Some(8),
                Some(dec!(5)),
                ExitType::Ipo,
            ),
        ]);
        input.recycling_rate = dec!(0.10); // 10% recycling

        let result = model_venture_fund(&input).unwrap();
        let cashflows = &result.result.yearly_cashflows;

        // Year 3: Early exits at 3x = 30M proceeds
        // Recycling = 30M * 0.10 = 3M withheld from distribution
        // Distributions = 30M - 3M = 27M
        let year3 = &cashflows[2];
        assert_eq!(year3.distributions, dec!(27_000_000));

        // The recycled 3M should be deployed later
        // Total invested should be > 20M (original) because of recycling
        assert!(
            result.result.fund_metrics.total_invested > dec!(20_000_000),
            "Total invested should exceed original deployments due to recycling"
        );
    }

    // -----------------------------------------------------------------------
    // 14. Multiple exits in same year
    // -----------------------------------------------------------------------
    #[test]
    fn test_multiple_exits_same_year() {
        let input = make_fund(vec![
            inv(
                "A",
                dec!(10_000_000),
                1,
                Some(5),
                Some(dec!(4)),
                ExitType::Ipo,
            ),
            inv(
                "B",
                dec!(10_000_000),
                2,
                Some(5),
                Some(dec!(2)),
                ExitType::Acquisition,
            ),
            inv(
                "C",
                dec!(10_000_000),
                3,
                Some(5),
                Some(dec!(6)),
                ExitType::Ipo,
            ),
        ]);
        let result = model_venture_fund(&input).unwrap();
        let cashflows = &result.result.yearly_cashflows;

        // Year 5: A exits at 40M, B at 20M, C at 60M = 120M total
        let year5 = &cashflows[4];
        assert_eq!(year5.distributions, dec!(120_000_000));
    }

    // -----------------------------------------------------------------------
    // 15. Validation: zero fund size
    // -----------------------------------------------------------------------
    #[test]
    fn test_validation_zero_fund_size() {
        let mut input = make_fund(vec![]);
        input.fund_size = Decimal::ZERO;
        let result = model_venture_fund(&input);
        assert!(result.is_err());
        match result.unwrap_err() {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "fund_size");
            }
            other => panic!("Expected InvalidInput for fund_size, got: {other:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // 16. Validation: investment year out of range
    // -----------------------------------------------------------------------
    #[test]
    fn test_validation_investment_year_out_of_range() {
        let input = make_fund(vec![inv(
            "Bad",
            dec!(10_000_000),
            15,
            Some(16),
            Some(dec!(2)),
            ExitType::Acquisition,
        )]);
        let result = model_venture_fund(&input);
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // 17. Validation: exit year before investment year
    // -----------------------------------------------------------------------
    #[test]
    fn test_validation_exit_before_investment() {
        let input = make_fund(vec![inv(
            "Bad",
            dec!(10_000_000),
            5,
            Some(3),
            Some(dec!(2)),
            ExitType::Acquisition,
        )]);
        let result = model_venture_fund(&input);
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // 18. Pct of fund returns (power law)
    // -----------------------------------------------------------------------
    #[test]
    fn test_pct_of_fund_returns() {
        let input = make_fund(vec![
            inv(
                "Big",
                dec!(10_000_000),
                1,
                Some(5),
                Some(dec!(20)),
                ExitType::Ipo,
            ),
            inv(
                "Small",
                dec!(10_000_000),
                2,
                Some(6),
                Some(dec!(2)),
                ExitType::Acquisition,
            ),
            inv("Loss", dec!(10_000_000), 1, None, None, ExitType::WriteOff),
        ]);
        let result = model_venture_fund(&input).unwrap();
        let results = &result.result.investment_results;

        // Big: profit = 10M*20 - 10M = 190M
        // Small: profit = 10M*2 - 10M = 10M
        // Loss: profit = 0 - 10M = -10M (excluded from positive profit sum)
        // Total positive profit = 200M
        let big = results.iter().find(|r| r.company_name == "Big").unwrap();
        assert_eq!(big.profit, dec!(190_000_000));
        // Big pct = 190M / 200M = 0.95
        assert_eq!(
            big.pct_of_fund_returns,
            dec!(190_000_000) / dec!(200_000_000)
        );

        let small = results.iter().find(|r| r.company_name == "Small").unwrap();
        assert_eq!(
            small.pct_of_fund_returns,
            dec!(10_000_000) / dec!(200_000_000)
        );

        let loss = results.iter().find(|r| r.company_name == "Loss").unwrap();
        assert_eq!(loss.pct_of_fund_returns, Decimal::ZERO);
    }

    // -----------------------------------------------------------------------
    // 19. Empty fund (no investments, just fees)
    // -----------------------------------------------------------------------
    #[test]
    fn test_empty_fund_no_investments() {
        let input = make_fund(vec![]);
        let result = model_venture_fund(&input).unwrap();
        let metrics = &result.result.fund_metrics;

        // No investments => no distributions
        assert_eq!(metrics.total_distributions, Decimal::ZERO);
        assert_eq!(metrics.total_invested, Decimal::ZERO);
        assert_eq!(metrics.dpi, Decimal::ZERO);

        // But there should be management fees called
        assert!(metrics.total_management_fees > Decimal::ZERO);
        assert!(metrics.total_called > Decimal::ZERO);

        assert_eq!(metrics.carried_interest, Decimal::ZERO);
    }

    // -----------------------------------------------------------------------
    // 20. Median with even number of exits
    // -----------------------------------------------------------------------
    #[test]
    fn test_median_even_exits() {
        let input = make_fund(vec![
            inv(
                "A",
                dec!(5_000_000),
                1,
                Some(5),
                Some(dec!(2)),
                ExitType::Acquisition,
            ),
            inv(
                "B",
                dec!(5_000_000),
                2,
                Some(6),
                Some(dec!(4)),
                ExitType::Ipo,
            ),
            inv(
                "C",
                dec!(5_000_000),
                3,
                Some(7),
                Some(dec!(6)),
                ExitType::Ipo,
            ),
            inv(
                "D",
                dec!(5_000_000),
                4,
                Some(8),
                Some(dec!(8)),
                ExitType::Ipo,
            ),
        ]);
        let result = model_venture_fund(&input).unwrap();
        let stats = &result.result.portfolio_stats;

        // Sorted: [2, 4, 6, 8], median = (4 + 6) / 2 = 5
        assert_eq!(stats.median_exit_multiple, dec!(5));
    }

    // -----------------------------------------------------------------------
    // 21. Top performer multiple
    // -----------------------------------------------------------------------
    #[test]
    fn test_top_performer_multiple() {
        let input = make_fund(vec![
            inv(
                "A",
                dec!(5_000_000),
                1,
                Some(5),
                Some(dec!(100)),
                ExitType::Ipo,
            ),
            inv(
                "B",
                dec!(5_000_000),
                2,
                Some(6),
                Some(dec!(3)),
                ExitType::Acquisition,
            ),
        ]);
        let result = model_venture_fund(&input).unwrap();
        assert_eq!(
            result.result.portfolio_stats.top_performer_multiple,
            dec!(100)
        );
    }

    // -----------------------------------------------------------------------
    // 22. Validation: negative carry rate
    // -----------------------------------------------------------------------
    #[test]
    fn test_validation_negative_carry_rate() {
        let mut input = make_fund(vec![]);
        input.carry_rate = dec!(-0.10);
        assert!(model_venture_fund(&input).is_err());
    }

    // -----------------------------------------------------------------------
    // 23. Validation: investment period > fund life
    // -----------------------------------------------------------------------
    #[test]
    fn test_validation_investment_period_exceeds_fund_life() {
        let mut input = make_fund(vec![]);
        input.investment_period_years = 15;
        assert!(model_venture_fund(&input).is_err());
    }

    // -----------------------------------------------------------------------
    // 24. Year 1 cashflow structure
    // -----------------------------------------------------------------------
    #[test]
    fn test_year_one_cashflow() {
        let input = make_fund(vec![inv(
            "A",
            dec!(20_000_000),
            1,
            Some(8),
            Some(dec!(5)),
            ExitType::Ipo,
        )]);
        let result = model_venture_fund(&input).unwrap();
        let y1 = &result.result.yearly_cashflows[0];

        // Year 1: fee = 100M * 0.02 = 2M, deployed = 20M
        // Capital called = 2M + 20M = 22M
        assert_eq!(y1.management_fees, dec!(2_000_000));
        assert_eq!(y1.invested, dec!(20_000_000));
        assert_eq!(y1.capital_called, dec!(22_000_000));

        // No exits in year 1
        assert_eq!(y1.distributions, Decimal::ZERO);

        // Net cashflow = 0 - 22M = -22M
        assert_eq!(y1.net_cashflow, dec!(-22_000_000));
    }

    // -----------------------------------------------------------------------
    // 25. J-curve trough year identification
    // -----------------------------------------------------------------------
    #[test]
    fn test_j_curve_trough_year() {
        let input = make_fund(vec![
            inv(
                "A",
                dec!(20_000_000),
                1,
                Some(8),
                Some(dec!(5)),
                ExitType::Ipo,
            ),
            inv(
                "B",
                dec!(15_000_000),
                3,
                Some(9),
                Some(dec!(3)),
                ExitType::Acquisition,
            ),
        ]);
        let result = model_venture_fund(&input).unwrap();

        // Trough should be during investment period but before exits
        let trough = result.result.fund_metrics.j_curve_trough_year;
        assert!(
            trough >= 1 && trough <= 7,
            "Trough year should be before major exits, got {}",
            trough
        );
    }

    // -----------------------------------------------------------------------
    // 26. Secondary exit type handled correctly
    // -----------------------------------------------------------------------
    #[test]
    fn test_secondary_exit() {
        let input = make_fund(vec![inv(
            "A",
            dec!(10_000_000),
            1,
            Some(4),
            Some(dec!(2.5)),
            ExitType::Secondary,
        )]);
        let result = model_venture_fund(&input).unwrap();
        let a = result
            .result
            .investment_results
            .iter()
            .find(|r| r.company_name == "A")
            .unwrap();

        assert_eq!(a.exit_value, dec!(25_000_000));
        assert_eq!(a.multiple, dec!(2.5));
        assert_eq!(result.result.portfolio_stats.num_exits, 1);
    }
}
