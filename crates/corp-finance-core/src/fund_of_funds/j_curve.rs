//! J-Curve Model for private equity fund lifecycle analysis.
//!
//! Models the typical fund cash flow pattern:
//! - **Draw period** (years 1-4): capital calls / drawdowns
//! - **Investment period** (years 2-6): deployment into portfolio companies
//! - **Harvest period** (years 5-12): realizations and distributions
//!
//! Computes TVPI, DPI, RVPI, PME (Kaplan-Schoar), net/gross IRR,
//! and identifies the J-curve trough.
//!
//! All arithmetic uses `rust_decimal::Decimal`. No `f64`.

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

use crate::error::CorpFinanceError;
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Input / Output
// ---------------------------------------------------------------------------

/// Input for J-Curve fund lifecycle model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JCurveInput {
    /// Total fund commitment amount.
    pub total_commitment: Decimal,
    /// Drawdown schedule: fraction of commitment drawn each year (e.g. [0.25, 0.25, 0.25, 0.25]).
    pub drawdown_schedule: Vec<Decimal>,
    /// Distribution schedule: fraction of NAV distributed each year.
    pub distribution_schedule: Vec<Decimal>,
    /// Fund life in years.
    pub fund_life_years: u32,
    /// Annual growth rate of the portfolio (decimal, e.g. 0.12 = 12%).
    pub growth_rate: Decimal,
    /// Management fee as percentage of commitment (decimal, e.g. 0.02 = 2%).
    pub management_fee_pct: Decimal,
    /// Carried interest percentage (decimal, e.g. 0.20 = 20%).
    pub carry_pct: Decimal,
    /// Preferred return / hurdle rate (decimal, e.g. 0.08 = 8%).
    pub preferred_return: Decimal,
    /// Public market index returns per year for PME calculation (one per year).
    pub public_index_returns: Vec<Decimal>,
}

/// A single period in the J-Curve cash flow schedule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JCurvePeriod {
    /// Year number (0 = inception).
    pub year: u32,
    /// Capital contributions (drawdowns + management fees) in this period.
    pub contributions: Decimal,
    /// Distributions received in this period.
    pub distributions: Decimal,
    /// Net asset value at end of period.
    pub nav: Decimal,
    /// Net cash flow = distributions - contributions.
    pub net_cash_flow: Decimal,
    /// TVPI as of this period.
    pub tvpi: Decimal,
    /// DPI as of this period.
    pub dpi: Decimal,
    /// RVPI as of this period.
    pub rvpi: Decimal,
    /// Cumulative IRR through this period.
    pub cumulative_irr: Decimal,
}

/// Output of the J-Curve model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JCurveOutput {
    /// Period-by-period cash flow schedule.
    pub periods: Vec<JCurvePeriod>,
    /// Final TVPI at fund termination.
    pub final_tvpi: Decimal,
    /// Final DPI at fund termination.
    pub final_dpi: Decimal,
    /// Kaplan-Schoar PME ratio.
    pub pme_kaplan_schoar: Decimal,
    /// Net IRR to LPs (after fees and carry).
    pub net_irr: Decimal,
    /// Gross IRR (before fees and carry).
    pub gross_irr: Decimal,
    /// Year at which the J-curve trough occurs (lowest cumulative net cash flow).
    pub j_curve_trough_year: u32,
    /// Value at the J-curve trough (most negative cumulative net cash flow).
    pub j_curve_trough_value: Decimal,
}

// ---------------------------------------------------------------------------
// Core computation
// ---------------------------------------------------------------------------

/// Compute the J-Curve fund lifecycle model.
pub fn calculate_j_curve(input: &JCurveInput) -> CorpFinanceResult<JCurveOutput> {
    validate_j_curve_input(input)?;

    let n = input.fund_life_years as usize;
    let one = Decimal::ONE;

    // Pre-compute drawdown and distribution rates per year, padding with zero.
    let drawdown_rate = |yr: usize| -> Decimal {
        if yr < input.drawdown_schedule.len() {
            input.drawdown_schedule[yr]
        } else {
            Decimal::ZERO
        }
    };
    let dist_rate = |yr: usize| -> Decimal {
        if yr < input.distribution_schedule.len() {
            input.distribution_schedule[yr]
        } else {
            Decimal::ZERO
        }
    };

    let mut periods: Vec<JCurvePeriod> = Vec::with_capacity(n + 1);
    let mut cumulative_contributions = Decimal::ZERO;
    let mut cumulative_distributions = Decimal::ZERO;
    let mut nav = Decimal::ZERO;
    let mut cumulative_net_cf = Decimal::ZERO;

    // Gross-level tracking (no fees/carry) for gross IRR.
    let mut gross_cfs: Vec<Decimal> = Vec::with_capacity(n + 1);
    // Net-level tracking for net IRR.
    let mut net_cfs: Vec<Decimal> = Vec::with_capacity(n + 1);

    // Track cumulative contributions for preferred return / carry calc.
    let mut total_contributed = Decimal::ZERO;

    let mut trough_year: u32 = 0;
    let mut trough_value = Decimal::ZERO;

    for yr in 0..=n {
        // Drawdown (capital call)
        let drawdown = input.total_commitment * drawdown_rate(yr);
        // Management fee on commitment
        let mgmt_fee = if yr == 0 {
            Decimal::ZERO
        } else {
            input.total_commitment * input.management_fee_pct
        };
        let contribution = drawdown + mgmt_fee;
        total_contributed += drawdown; // only the actual capital, not fees

        // Grow existing NAV
        if yr > 0 {
            nav *= one + input.growth_rate;
        }
        // Add new drawdown to NAV (actual capital, not fee)
        nav += drawdown;

        // Distribution = fraction of NAV
        let gross_dist = nav * dist_rate(yr);
        nav -= gross_dist;

        // Carry calculation on distributions: apply carry only on gains above preferred return.
        // Simple model: carry applies on cumulative profit above hurdle.
        let cumulative_profit = cumulative_distributions + gross_dist + nav - total_contributed;
        let hurdle_amount = total_contributed * input.preferred_return * Decimal::from(yr as u32);
        let carry = if cumulative_profit > hurdle_amount && gross_dist > Decimal::ZERO {
            let excess = cumulative_profit - hurdle_amount;
            let carry_amount = excess * input.carry_pct;
            // Carry cannot exceed gross distribution for this period
            if carry_amount > gross_dist {
                gross_dist
            } else {
                carry_amount
            }
        } else {
            Decimal::ZERO
        };

        let net_dist = gross_dist - carry;

        cumulative_contributions += contribution;
        cumulative_distributions += net_dist;
        let net_cash_flow = net_dist - contribution;
        cumulative_net_cf += net_cash_flow;

        // Multiples
        let tvpi = if cumulative_contributions.is_zero() {
            Decimal::ZERO
        } else {
            (cumulative_distributions + nav) / cumulative_contributions
        };
        let dpi = if cumulative_contributions.is_zero() {
            Decimal::ZERO
        } else {
            cumulative_distributions / cumulative_contributions
        };
        let rvpi = if cumulative_contributions.is_zero() {
            Decimal::ZERO
        } else {
            nav / cumulative_contributions
        };

        // Build cash flow vectors for IRR: negative = outflow, positive = inflow.
        // Net: outflow = contribution, inflow = net_dist.
        // At final year, add residual NAV to inflow.
        let net_cf_for_irr = if yr == n {
            net_dist + nav - contribution
        } else {
            net_dist - contribution
        };
        net_cfs.push(net_cf_for_irr);

        // Gross: no fees or carry.
        let gross_cf_for_irr = if yr == n {
            gross_dist + nav - drawdown
        } else {
            gross_dist - drawdown
        };
        gross_cfs.push(gross_cf_for_irr);

        // Cumulative IRR through this period.
        let mut irr_cfs: Vec<Decimal> = net_cfs.clone();
        // If not final year, add terminal NAV to current period for interim IRR.
        if yr < n && yr > 0 {
            let last_idx = irr_cfs.len() - 1;
            irr_cfs[last_idx] += nav;
        }
        let cumulative_irr = if yr == 0 {
            Decimal::ZERO
        } else {
            newton_irr(&irr_cfs)
        };

        // Track J-curve trough
        if cumulative_net_cf < trough_value {
            trough_value = cumulative_net_cf;
            trough_year = yr as u32;
        }

        periods.push(JCurvePeriod {
            year: yr as u32,
            contributions: contribution,
            distributions: net_dist,
            nav,
            net_cash_flow,
            tvpi,
            dpi,
            rvpi,
            cumulative_irr,
        });
    }

    let final_tvpi = if let Some(last) = periods.last() {
        last.tvpi
    } else {
        Decimal::ZERO
    };
    let final_dpi = if let Some(last) = periods.last() {
        last.dpi
    } else {
        Decimal::ZERO
    };

    let net_irr = newton_irr(&net_cfs);
    let gross_irr = newton_irr(&gross_cfs);

    // PME (Kaplan-Schoar)
    let pme_kaplan_schoar = calculate_pme(input, &periods)?;

    Ok(JCurveOutput {
        periods,
        final_tvpi,
        final_dpi,
        pme_kaplan_schoar,
        net_irr,
        gross_irr,
        j_curve_trough_year: trough_year,
        j_curve_trough_value: trough_value,
    })
}

// ---------------------------------------------------------------------------
// PME (Kaplan-Schoar)
// ---------------------------------------------------------------------------

/// Kaplan-Schoar PME = sum(dist_t * index_0/index_t) / sum(contrib_t * index_0/index_t).
/// We compute the index level at each period from the returns series.
fn calculate_pme(input: &JCurveInput, periods: &[JCurvePeriod]) -> CorpFinanceResult<Decimal> {
    if input.public_index_returns.is_empty() {
        return Ok(Decimal::ZERO);
    }

    let one = Decimal::ONE;
    // Build cumulative index level: index_0 = 1, index_t = index_{t-1} * (1 + r_t).
    let mut index_levels: Vec<Decimal> = Vec::with_capacity(periods.len());
    let mut level = one;
    index_levels.push(level);
    for r in &input.public_index_returns {
        level *= one + *r;
        index_levels.push(level);
    }
    // Pad if needed
    while index_levels.len() < periods.len() {
        index_levels.push(level);
    }

    let index_end = if let Some(&last) = index_levels.last() {
        if last.is_zero() {
            one
        } else {
            last
        }
    } else {
        one
    };

    let mut pv_distributions = Decimal::ZERO;
    let mut pv_contributions = Decimal::ZERO;

    for (i, p) in periods.iter().enumerate() {
        let idx = if i < index_levels.len() {
            index_levels[i]
        } else {
            index_end
        };
        if idx.is_zero() {
            continue;
        }
        // Future-value factor: index_end / index_t
        let fv_factor = index_end / idx;
        pv_distributions += p.distributions * fv_factor;
        pv_contributions += p.contributions * fv_factor;
        // At terminal period, add NAV
        if i == periods.len() - 1 {
            pv_distributions += p.nav * fv_factor;
        }
    }

    if pv_contributions.is_zero() {
        return Ok(Decimal::ZERO);
    }

    Ok(pv_distributions / pv_contributions)
}

// ---------------------------------------------------------------------------
// Newton-Raphson IRR
// ---------------------------------------------------------------------------

/// Newton-Raphson IRR with 30 iterations, initial guess 0.10.
fn newton_irr(cash_flows: &[Decimal]) -> Decimal {
    if cash_flows.is_empty() || cash_flows.len() < 2 {
        return Decimal::ZERO;
    }

    let one = Decimal::ONE;
    let mut rate = dec!(0.10);
    let max_iter = 30;
    let eps = dec!(0.0000001);

    for _ in 0..max_iter {
        let mut npv = Decimal::ZERO;
        let mut dnpv = Decimal::ZERO;
        let mut df = one; // discount factor at t
        let denom = one + rate;

        if denom.is_zero() {
            rate = dec!(0.05);
            continue;
        }

        for (t, cf) in cash_flows.iter().enumerate() {
            if t == 0 {
                npv += *cf;
            } else {
                // df = 1 / (1+rate)^t, computed iteratively
                df /= denom;
                npv += *cf * df;
                // d(NPV)/d(rate) = sum(-t * cf / (1+rate)^(t+1))
                let t_dec = Decimal::from(t as u32);
                dnpv += -t_dec * *cf * df / denom;
            }
        }

        if dnpv.is_zero() {
            break;
        }

        let delta = npv / dnpv;
        rate -= delta;

        // Clamp to [-0.99, 10.0]
        if rate < dec!(-0.99) {
            rate = dec!(-0.99);
        }
        if rate > dec!(10.0) {
            rate = dec!(10.0);
        }

        if delta.abs() < eps {
            break;
        }
    }

    rate
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

fn validate_j_curve_input(input: &JCurveInput) -> CorpFinanceResult<()> {
    if input.total_commitment <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "total_commitment".into(),
            reason: "Total commitment must be positive.".into(),
        });
    }
    if input.fund_life_years == 0 {
        return Err(CorpFinanceError::InvalidInput {
            field: "fund_life_years".into(),
            reason: "Fund life must be at least 1 year.".into(),
        });
    }
    if input.drawdown_schedule.is_empty() {
        return Err(CorpFinanceError::InsufficientData(
            "Drawdown schedule must have at least one entry.".into(),
        ));
    }
    if input.distribution_schedule.is_empty() {
        return Err(CorpFinanceError::InsufficientData(
            "Distribution schedule must have at least one entry.".into(),
        ));
    }
    for (i, d) in input.drawdown_schedule.iter().enumerate() {
        if *d < Decimal::ZERO {
            return Err(CorpFinanceError::InvalidInput {
                field: format!("drawdown_schedule[{}]", i),
                reason: "Drawdown percentages cannot be negative.".into(),
            });
        }
    }
    for (i, d) in input.distribution_schedule.iter().enumerate() {
        if *d < Decimal::ZERO {
            return Err(CorpFinanceError::InvalidInput {
                field: format!("distribution_schedule[{}]", i),
                reason: "Distribution percentages cannot be negative.".into(),
            });
        }
    }
    if input.management_fee_pct < Decimal::ZERO || input.management_fee_pct > dec!(0.10) {
        return Err(CorpFinanceError::InvalidInput {
            field: "management_fee_pct".into(),
            reason: "Management fee must be in [0, 0.10].".into(),
        });
    }
    if input.carry_pct < Decimal::ZERO || input.carry_pct > Decimal::ONE {
        return Err(CorpFinanceError::InvalidInput {
            field: "carry_pct".into(),
            reason: "Carry percentage must be in [0, 1].".into(),
        });
    }
    if input.preferred_return < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "preferred_return".into(),
            reason: "Preferred return cannot be negative.".into(),
        });
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

    fn approx_eq(a: Decimal, b: Decimal, eps: Decimal) -> bool {
        (a - b).abs() < eps
    }

    fn default_input() -> JCurveInput {
        JCurveInput {
            total_commitment: dec!(100_000_000),
            drawdown_schedule: vec![dec!(0.30), dec!(0.30), dec!(0.25), dec!(0.15)],
            distribution_schedule: vec![
                dec!(0.0),
                dec!(0.0),
                dec!(0.0),
                dec!(0.05),
                dec!(0.10),
                dec!(0.15),
                dec!(0.20),
                dec!(0.25),
                dec!(0.30),
                dec!(0.40),
                dec!(1.0),
            ],
            fund_life_years: 10,
            growth_rate: dec!(0.12),
            management_fee_pct: dec!(0.02),
            carry_pct: dec!(0.20),
            preferred_return: dec!(0.08),
            public_index_returns: vec![
                dec!(0.10),
                dec!(0.08),
                dec!(0.12),
                dec!(0.05),
                dec!(0.15),
                dec!(0.07),
                dec!(0.09),
                dec!(0.11),
                dec!(0.06),
                dec!(0.10),
            ],
        }
    }

    #[test]
    fn test_j_curve_basic_output_structure() {
        let input = default_input();
        let out = calculate_j_curve(&input).unwrap();
        // fund_life_years = 10, so periods 0..10 = 11 entries
        assert_eq!(out.periods.len(), 11);
    }

    #[test]
    fn test_j_curve_first_period_zero_distributions() {
        let input = default_input();
        let out = calculate_j_curve(&input).unwrap();
        assert_eq!(out.periods[0].distributions, Decimal::ZERO);
    }

    #[test]
    fn test_j_curve_trough_in_early_years() {
        let input = default_input();
        let out = calculate_j_curve(&input).unwrap();
        // Trough should be in years 1-4 when drawdowns dominate
        assert!(
            out.j_curve_trough_year <= 5,
            "Trough at year {}",
            out.j_curve_trough_year
        );
    }

    #[test]
    fn test_j_curve_trough_value_negative() {
        let input = default_input();
        let out = calculate_j_curve(&input).unwrap();
        assert!(
            out.j_curve_trough_value < Decimal::ZERO,
            "Trough value {} should be negative",
            out.j_curve_trough_value
        );
    }

    #[test]
    fn test_j_curve_final_tvpi_positive() {
        let input = default_input();
        let out = calculate_j_curve(&input).unwrap();
        assert!(out.final_tvpi > Decimal::ZERO, "TVPI should be positive");
    }

    #[test]
    fn test_j_curve_final_dpi_positive() {
        let input = default_input();
        let out = calculate_j_curve(&input).unwrap();
        assert!(out.final_dpi > Decimal::ZERO, "DPI should be positive");
    }

    #[test]
    fn test_j_curve_tvpi_equals_dpi_plus_rvpi() {
        let input = default_input();
        let out = calculate_j_curve(&input).unwrap();
        for p in &out.periods {
            let sum = p.dpi + p.rvpi;
            assert!(
                approx_eq(p.tvpi, sum, dec!(0.01)),
                "Year {}: TVPI {} != DPI {} + RVPI {}",
                p.year,
                p.tvpi,
                p.dpi,
                p.rvpi
            );
        }
    }

    #[test]
    fn test_j_curve_net_cash_flow_is_dist_minus_contrib() {
        let input = default_input();
        let out = calculate_j_curve(&input).unwrap();
        for p in &out.periods {
            let expected = p.distributions - p.contributions;
            assert_eq!(p.net_cash_flow, expected, "Year {}: NCF mismatch", p.year);
        }
    }

    #[test]
    fn test_j_curve_gross_irr_gt_net_irr() {
        let input = default_input();
        let out = calculate_j_curve(&input).unwrap();
        assert!(
            out.gross_irr >= out.net_irr,
            "Gross IRR {} should be >= Net IRR {}",
            out.gross_irr,
            out.net_irr
        );
    }

    #[test]
    fn test_j_curve_pme_nonzero_with_index() {
        let input = default_input();
        let out = calculate_j_curve(&input).unwrap();
        assert!(
            out.pme_kaplan_schoar > Decimal::ZERO,
            "PME should be positive when index returns provided"
        );
    }

    #[test]
    fn test_j_curve_pme_zero_without_index() {
        let mut input = default_input();
        input.public_index_returns = vec![];
        let out = calculate_j_curve(&input).unwrap();
        assert_eq!(out.pme_kaplan_schoar, Decimal::ZERO);
    }

    #[test]
    fn test_j_curve_no_fees_higher_net_irr() {
        let with_fees = default_input();
        let mut no_fees = default_input();
        no_fees.management_fee_pct = Decimal::ZERO;
        no_fees.carry_pct = Decimal::ZERO;
        let out_fees = calculate_j_curve(&with_fees).unwrap();
        let out_nofees = calculate_j_curve(&no_fees).unwrap();
        assert!(
            out_nofees.net_irr >= out_fees.net_irr,
            "No-fee IRR {} should >= fee IRR {}",
            out_nofees.net_irr,
            out_fees.net_irr
        );
    }

    #[test]
    fn test_j_curve_contributions_positive() {
        let input = default_input();
        let out = calculate_j_curve(&input).unwrap();
        for p in &out.periods {
            assert!(
                p.contributions >= Decimal::ZERO,
                "Year {}: contributions should be non-negative",
                p.year
            );
        }
    }

    #[test]
    fn test_j_curve_nav_non_negative() {
        let input = default_input();
        let out = calculate_j_curve(&input).unwrap();
        for p in &out.periods {
            assert!(
                p.nav >= Decimal::ZERO,
                "Year {}: NAV {} should be non-negative",
                p.year,
                p.nav
            );
        }
    }

    // -- Validation tests --

    #[test]
    fn test_reject_zero_commitment() {
        let mut input = default_input();
        input.total_commitment = Decimal::ZERO;
        assert!(calculate_j_curve(&input).is_err());
    }

    #[test]
    fn test_reject_negative_commitment() {
        let mut input = default_input();
        input.total_commitment = dec!(-1);
        assert!(calculate_j_curve(&input).is_err());
    }

    #[test]
    fn test_reject_zero_fund_life() {
        let mut input = default_input();
        input.fund_life_years = 0;
        assert!(calculate_j_curve(&input).is_err());
    }

    #[test]
    fn test_reject_empty_drawdown_schedule() {
        let mut input = default_input();
        input.drawdown_schedule = vec![];
        assert!(calculate_j_curve(&input).is_err());
    }

    #[test]
    fn test_reject_empty_distribution_schedule() {
        let mut input = default_input();
        input.distribution_schedule = vec![];
        assert!(calculate_j_curve(&input).is_err());
    }

    #[test]
    fn test_reject_negative_drawdown() {
        let mut input = default_input();
        input.drawdown_schedule = vec![dec!(-0.1), dec!(0.5)];
        assert!(calculate_j_curve(&input).is_err());
    }

    #[test]
    fn test_reject_excessive_management_fee() {
        let mut input = default_input();
        input.management_fee_pct = dec!(0.15);
        assert!(calculate_j_curve(&input).is_err());
    }

    #[test]
    fn test_reject_negative_carry() {
        let mut input = default_input();
        input.carry_pct = dec!(-0.01);
        assert!(calculate_j_curve(&input).is_err());
    }

    #[test]
    fn test_reject_negative_preferred_return() {
        let mut input = default_input();
        input.preferred_return = dec!(-0.05);
        assert!(calculate_j_curve(&input).is_err());
    }

    #[test]
    fn test_serialization_roundtrip() {
        let input = default_input();
        let out = calculate_j_curve(&input).unwrap();
        let json = serde_json::to_string(&out).unwrap();
        let _: JCurveOutput = serde_json::from_str(&json).unwrap();
    }

    #[test]
    fn test_short_fund_life() {
        let mut input = default_input();
        input.fund_life_years = 3;
        let out = calculate_j_curve(&input).unwrap();
        assert_eq!(out.periods.len(), 4); // 0,1,2,3
    }
}
