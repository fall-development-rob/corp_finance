//! Commitment Pacing Model for fund-of-funds portfolio management.
//!
//! Projects future drawdowns, distributions, and NAV from existing and new
//! fund commitments to maintain a target private equity allocation.
//!
//! Key outputs:
//! - **Vintage year planning**: allocate new commitments across years
//! - **Drawdown modeling**: project future capital calls from unfunded commitments
//! - **NAV projection**: project future NAV from existing + new commitments
//! - **Over-commitment ratio**: total_commitments / target_nav_allocation
//! - **Cash flow projection**: aggregate contributions - distributions
//! - **Target allocation tracking**: current_nav / total_portfolio vs target_pct
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

/// Description of an existing fund in the portfolio.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExistingFund {
    /// Vintage year of the fund.
    pub vintage: u32,
    /// Total commitment to the fund.
    pub commitment: Decimal,
    /// Remaining unfunded commitment.
    pub unfunded: Decimal,
    /// Current net asset value.
    pub nav: Decimal,
    /// Expected annual drawdown rate on unfunded (decimal).
    pub drawdown_rate: Decimal,
    /// Expected annual distribution rate on NAV (decimal).
    pub distribution_rate: Decimal,
}

/// Input for commitment pacing model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitmentPacingInput {
    /// Existing funds in the portfolio.
    pub existing_funds: Vec<ExistingFund>,
    /// Target allocation to private equity (decimal, e.g. 0.15 = 15%).
    pub target_allocation_pct: Decimal,
    /// Total portfolio value (all asset classes).
    pub total_portfolio_value: Decimal,
    /// Number of years to project forward.
    pub planning_years: u32,
    /// Planned new commitment per year.
    pub new_commitment_per_year: Decimal,
    /// Drawdown curve for new commitments: fraction drawn per year of fund life.
    pub drawdown_curve: Vec<Decimal>,
    /// Distribution curve for new commitments: fraction of NAV distributed per year.
    pub distribution_curve: Vec<Decimal>,
}

/// A single year projection in the pacing model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PacingYearProjection {
    /// Projection year (1-based).
    pub year: u32,
    /// New commitments made in this year.
    pub new_commitments: Decimal,
    /// Total projected drawdowns across all funds.
    pub projected_drawdowns: Decimal,
    /// Total projected distributions across all funds.
    pub projected_distributions: Decimal,
    /// Projected NAV at year end.
    pub projected_nav: Decimal,
    /// Allocation percentage: projected_nav / total_portfolio_value.
    pub allocation_pct: Decimal,
    /// Over-commitment ratio: total unfunded / target_nav.
    pub over_commitment_ratio: Decimal,
    /// Net cash flow: distributions - drawdowns.
    pub net_cash_flow: Decimal,
}

/// Output of the commitment pacing model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitmentPacingOutput {
    /// Year-by-year projections.
    pub yearly_projections: Vec<PacingYearProjection>,
    /// Recommended annual commitment pace to reach target.
    pub recommended_pace: Decimal,
    /// Estimated years to reach target allocation.
    pub years_to_target: u32,
    /// Peak over-commitment ratio during the projection period.
    pub peak_over_commitment: Decimal,
}

// ---------------------------------------------------------------------------
// Core computation
// ---------------------------------------------------------------------------

/// Compute the commitment pacing model.
pub fn calculate_commitment_pacing(
    input: &CommitmentPacingInput,
) -> CorpFinanceResult<CommitmentPacingOutput> {
    validate_pacing_input(input)?;

    let target_nav = input.total_portfolio_value * input.target_allocation_pct;
    let n = input.planning_years as usize;

    // Track each fund's state: (unfunded, nav)
    let mut fund_states: Vec<(Decimal, Decimal)> = input
        .existing_funds
        .iter()
        .map(|f| (f.unfunded, f.nav))
        .collect();

    // Track new commitments by year for drawdown curve application.
    // new_commitments_by_year[i] = commitment made at projection year i.
    let mut new_commitments_by_year: Vec<Decimal> = Vec::with_capacity(n);

    let mut projections: Vec<PacingYearProjection> = Vec::with_capacity(n);
    let mut peak_oc = Decimal::ZERO;
    let mut years_to_target: u32 = 0;
    let mut reached_target = false;

    for yr in 0..n {
        let year_num = (yr + 1) as u32;

        // New commitment this year
        let new_commit = input.new_commitment_per_year;
        new_commitments_by_year.push(new_commit);

        // Project drawdowns and distributions from existing funds.
        let mut total_drawdowns = Decimal::ZERO;
        let mut total_distributions = Decimal::ZERO;

        for (unfunded, nav) in fund_states.iter_mut() {
            // Drawdown: fraction of remaining unfunded
            let dd = *unfunded
                * input
                    .existing_funds
                    .first()
                    .map_or(dec!(0.25), |f| f.drawdown_rate);
            let dd = dd.min(*unfunded);
            *unfunded -= dd;
            *nav += dd;
            total_drawdowns += dd;

            // Distribution: fraction of NAV
            let dist = *nav
                * input
                    .existing_funds
                    .first()
                    .map_or(dec!(0.10), |f| f.distribution_rate);
            *nav -= dist;
            total_distributions += dist;
        }

        // Project drawdowns from new commitments using the drawdown curve.
        for (commit_yr, commit_amt) in new_commitments_by_year.iter().enumerate() {
            let age = yr - commit_yr; // age of this commitment in years
            if age < input.drawdown_curve.len() {
                let dd_rate = input.drawdown_curve[age];
                let dd = *commit_amt * dd_rate;
                total_drawdowns += dd;
            }
            // Distributions from new commitments using distribution curve.
            if age < input.distribution_curve.len() {
                let dist_rate = input.distribution_curve[age];
                // Approximate NAV from new commitment as commitment * avg growth
                // Simple model: NAV ~ cumulative drawdowns not yet distributed
                let cumulative_dd: Decimal = (0..=age)
                    .filter(|a| *a < input.drawdown_curve.len())
                    .map(|a| input.drawdown_curve[a])
                    .sum::<Decimal>()
                    * *commit_amt;
                let dist = cumulative_dd * dist_rate;
                total_distributions += dist;
            }
        }

        // Compute aggregate NAV
        let existing_nav: Decimal = fund_states.iter().map(|(_, nav)| *nav).sum();
        // New commitments NAV: cumulative drawdowns - cumulative distributions.
        let new_nav: Decimal = new_commitments_by_year
            .iter()
            .enumerate()
            .map(|(cy, ca)| {
                let age = yr - cy;
                let cum_dd: Decimal = (0..=age)
                    .filter(|a| *a < input.drawdown_curve.len())
                    .map(|a| input.drawdown_curve[a])
                    .sum::<Decimal>()
                    * *ca;
                let cum_dist: Decimal = (0..=age)
                    .filter(|a| *a < input.distribution_curve.len())
                    .map(|a| {
                        let dd_sum: Decimal = (0..=a
                            .min(input.drawdown_curve.len().saturating_sub(1)))
                            .filter(|b| *b < input.drawdown_curve.len())
                            .map(|b| input.drawdown_curve[b])
                            .sum::<Decimal>()
                            * *ca;
                        dd_sum * input.distribution_curve[a]
                    })
                    .sum();
                if cum_dd > cum_dist {
                    cum_dd - cum_dist
                } else {
                    Decimal::ZERO
                }
            })
            .sum();

        let projected_nav = existing_nav + new_nav;

        let allocation_pct = if input.total_portfolio_value.is_zero() {
            Decimal::ZERO
        } else {
            projected_nav / input.total_portfolio_value
        };

        // Over-commitment ratio: total unfunded / target_nav
        let total_unfunded: Decimal = fund_states.iter().map(|(u, _)| *u).sum::<Decimal>()
            + new_commitments_by_year
                .iter()
                .enumerate()
                .map(|(cy, ca)| {
                    let age = yr - cy;
                    let drawn: Decimal = (0..=age)
                        .filter(|a| *a < input.drawdown_curve.len())
                        .map(|a| input.drawdown_curve[a])
                        .sum::<Decimal>();
                    let remaining = Decimal::ONE - drawn;
                    if remaining > Decimal::ZERO {
                        *ca * remaining
                    } else {
                        Decimal::ZERO
                    }
                })
                .sum::<Decimal>();

        let total_commitments = projected_nav + total_unfunded;
        let over_commitment_ratio = if target_nav.is_zero() {
            Decimal::ZERO
        } else {
            total_commitments / target_nav
        };

        if over_commitment_ratio > peak_oc {
            peak_oc = over_commitment_ratio;
        }

        if !reached_target && allocation_pct >= input.target_allocation_pct {
            years_to_target = year_num;
            reached_target = true;
        }

        let net_cash_flow = total_distributions - total_drawdowns;

        projections.push(PacingYearProjection {
            year: year_num,
            new_commitments: new_commit,
            projected_drawdowns: total_drawdowns,
            projected_distributions: total_distributions,
            projected_nav,
            allocation_pct,
            over_commitment_ratio,
            net_cash_flow,
        });
    }

    if !reached_target {
        years_to_target = input.planning_years;
    }

    // Recommended pace: target_nav - current_nav, spread over planning years,
    // adjusted for over-commitment (multiply by typical OC ratio ~1.4).
    let current_nav: Decimal = input.existing_funds.iter().map(|f| f.nav).sum();
    let nav_gap = target_nav - current_nav;
    let recommended_pace = if input.planning_years == 0 || nav_gap <= Decimal::ZERO {
        Decimal::ZERO
    } else {
        // Typical over-commitment multiplier
        let oc_multiplier = dec!(1.4);
        (nav_gap * oc_multiplier) / Decimal::from(input.planning_years)
    };

    Ok(CommitmentPacingOutput {
        yearly_projections: projections,
        recommended_pace,
        years_to_target,
        peak_over_commitment: peak_oc,
    })
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

fn validate_pacing_input(input: &CommitmentPacingInput) -> CorpFinanceResult<()> {
    if input.total_portfolio_value <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "total_portfolio_value".into(),
            reason: "Total portfolio value must be positive.".into(),
        });
    }
    if input.target_allocation_pct <= Decimal::ZERO || input.target_allocation_pct > Decimal::ONE {
        return Err(CorpFinanceError::InvalidInput {
            field: "target_allocation_pct".into(),
            reason: "Target allocation must be in (0, 1].".into(),
        });
    }
    if input.planning_years == 0 {
        return Err(CorpFinanceError::InvalidInput {
            field: "planning_years".into(),
            reason: "Planning years must be at least 1.".into(),
        });
    }
    if input.new_commitment_per_year < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "new_commitment_per_year".into(),
            reason: "New commitment per year cannot be negative.".into(),
        });
    }
    if input.drawdown_curve.is_empty() {
        return Err(CorpFinanceError::InsufficientData(
            "Drawdown curve must have at least one entry.".into(),
        ));
    }
    if input.distribution_curve.is_empty() {
        return Err(CorpFinanceError::InsufficientData(
            "Distribution curve must have at least one entry.".into(),
        ));
    }
    for (i, d) in input.drawdown_curve.iter().enumerate() {
        if *d < Decimal::ZERO || *d > Decimal::ONE {
            return Err(CorpFinanceError::InvalidInput {
                field: format!("drawdown_curve[{}]", i),
                reason: "Drawdown curve values must be in [0, 1].".into(),
            });
        }
    }
    for (i, d) in input.distribution_curve.iter().enumerate() {
        if *d < Decimal::ZERO || *d > Decimal::ONE {
            return Err(CorpFinanceError::InvalidInput {
                field: format!("distribution_curve[{}]", i),
                reason: "Distribution curve values must be in [0, 1].".into(),
            });
        }
    }
    for fund in &input.existing_funds {
        if fund.commitment < Decimal::ZERO {
            return Err(CorpFinanceError::InvalidInput {
                field: "existing_funds.commitment".into(),
                reason: "Fund commitment cannot be negative.".into(),
            });
        }
        if fund.unfunded < Decimal::ZERO {
            return Err(CorpFinanceError::InvalidInput {
                field: "existing_funds.unfunded".into(),
                reason: "Unfunded commitment cannot be negative.".into(),
            });
        }
        if fund.nav < Decimal::ZERO {
            return Err(CorpFinanceError::InvalidInput {
                field: "existing_funds.nav".into(),
                reason: "NAV cannot be negative.".into(),
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

    fn approx_eq(a: Decimal, b: Decimal, eps: Decimal) -> bool {
        (a - b).abs() < eps
    }

    fn default_input() -> CommitmentPacingInput {
        CommitmentPacingInput {
            existing_funds: vec![
                ExistingFund {
                    vintage: 2020,
                    commitment: dec!(50_000_000),
                    unfunded: dec!(15_000_000),
                    nav: dec!(40_000_000),
                    drawdown_rate: dec!(0.25),
                    distribution_rate: dec!(0.10),
                },
                ExistingFund {
                    vintage: 2022,
                    commitment: dec!(30_000_000),
                    unfunded: dec!(20_000_000),
                    nav: dec!(12_000_000),
                    drawdown_rate: dec!(0.30),
                    distribution_rate: dec!(0.05),
                },
            ],
            target_allocation_pct: dec!(0.15),
            total_portfolio_value: dec!(500_000_000),
            planning_years: 5,
            new_commitment_per_year: dec!(25_000_000),
            drawdown_curve: vec![dec!(0.25), dec!(0.30), dec!(0.25), dec!(0.15), dec!(0.05)],
            distribution_curve: vec![dec!(0.0), dec!(0.0), dec!(0.05), dec!(0.10), dec!(0.15)],
        }
    }

    #[test]
    fn test_pacing_basic_output_structure() {
        let input = default_input();
        let out = calculate_commitment_pacing(&input).unwrap();
        assert_eq!(out.yearly_projections.len(), 5);
    }

    #[test]
    fn test_pacing_year_numbers_sequential() {
        let input = default_input();
        let out = calculate_commitment_pacing(&input).unwrap();
        for (i, p) in out.yearly_projections.iter().enumerate() {
            assert_eq!(p.year, (i + 1) as u32);
        }
    }

    #[test]
    fn test_pacing_new_commitments_per_year() {
        let input = default_input();
        let out = calculate_commitment_pacing(&input).unwrap();
        for p in &out.yearly_projections {
            assert_eq!(p.new_commitments, dec!(25_000_000));
        }
    }

    #[test]
    fn test_pacing_projected_drawdowns_positive() {
        let input = default_input();
        let out = calculate_commitment_pacing(&input).unwrap();
        for p in &out.yearly_projections {
            assert!(
                p.projected_drawdowns > Decimal::ZERO,
                "Year {}: drawdowns should be positive",
                p.year
            );
        }
    }

    #[test]
    fn test_pacing_projected_nav_positive() {
        let input = default_input();
        let out = calculate_commitment_pacing(&input).unwrap();
        for p in &out.yearly_projections {
            assert!(
                p.projected_nav >= Decimal::ZERO,
                "Year {}: NAV should be non-negative",
                p.year
            );
        }
    }

    #[test]
    fn test_pacing_allocation_pct_in_range() {
        let input = default_input();
        let out = calculate_commitment_pacing(&input).unwrap();
        for p in &out.yearly_projections {
            assert!(
                p.allocation_pct >= Decimal::ZERO && p.allocation_pct <= Decimal::ONE,
                "Year {}: allocation {} out of range",
                p.year,
                p.allocation_pct
            );
        }
    }

    #[test]
    fn test_pacing_over_commitment_ratio_positive() {
        let input = default_input();
        let out = calculate_commitment_pacing(&input).unwrap();
        assert!(
            out.peak_over_commitment > Decimal::ZERO,
            "Peak OC should be positive"
        );
    }

    #[test]
    fn test_pacing_net_cash_flow_equals_dist_minus_dd() {
        let input = default_input();
        let out = calculate_commitment_pacing(&input).unwrap();
        for p in &out.yearly_projections {
            let expected = p.projected_distributions - p.projected_drawdowns;
            assert!(
                approx_eq(p.net_cash_flow, expected, dec!(0.01)),
                "Year {}: NCF mismatch",
                p.year
            );
        }
    }

    #[test]
    fn test_pacing_recommended_pace_positive() {
        let input = default_input();
        let out = calculate_commitment_pacing(&input).unwrap();
        // Target NAV = 500M * 0.15 = 75M, current nav = 52M, gap exists
        assert!(
            out.recommended_pace > Decimal::ZERO,
            "Recommended pace should be positive when below target"
        );
    }

    #[test]
    fn test_pacing_years_to_target_bounded() {
        let input = default_input();
        let out = calculate_commitment_pacing(&input).unwrap();
        assert!(out.years_to_target <= input.planning_years);
    }

    #[test]
    fn test_pacing_no_existing_funds() {
        let mut input = default_input();
        input.existing_funds = vec![];
        let out = calculate_commitment_pacing(&input).unwrap();
        assert_eq!(out.yearly_projections.len(), 5);
    }

    #[test]
    fn test_pacing_zero_new_commitment() {
        let mut input = default_input();
        input.new_commitment_per_year = Decimal::ZERO;
        let out = calculate_commitment_pacing(&input).unwrap();
        for p in &out.yearly_projections {
            assert_eq!(p.new_commitments, Decimal::ZERO);
        }
    }

    // -- Validation tests --

    #[test]
    fn test_reject_zero_portfolio_value() {
        let mut input = default_input();
        input.total_portfolio_value = Decimal::ZERO;
        assert!(calculate_commitment_pacing(&input).is_err());
    }

    #[test]
    fn test_reject_zero_target_allocation() {
        let mut input = default_input();
        input.target_allocation_pct = Decimal::ZERO;
        assert!(calculate_commitment_pacing(&input).is_err());
    }

    #[test]
    fn test_reject_target_allocation_over_one() {
        let mut input = default_input();
        input.target_allocation_pct = dec!(1.5);
        assert!(calculate_commitment_pacing(&input).is_err());
    }

    #[test]
    fn test_reject_zero_planning_years() {
        let mut input = default_input();
        input.planning_years = 0;
        assert!(calculate_commitment_pacing(&input).is_err());
    }

    #[test]
    fn test_reject_negative_new_commitment() {
        let mut input = default_input();
        input.new_commitment_per_year = dec!(-1);
        assert!(calculate_commitment_pacing(&input).is_err());
    }

    #[test]
    fn test_reject_empty_drawdown_curve() {
        let mut input = default_input();
        input.drawdown_curve = vec![];
        assert!(calculate_commitment_pacing(&input).is_err());
    }

    #[test]
    fn test_reject_empty_distribution_curve() {
        let mut input = default_input();
        input.distribution_curve = vec![];
        assert!(calculate_commitment_pacing(&input).is_err());
    }

    #[test]
    fn test_reject_negative_fund_nav() {
        let mut input = default_input();
        input.existing_funds[0].nav = dec!(-1);
        assert!(calculate_commitment_pacing(&input).is_err());
    }

    #[test]
    fn test_serialization_roundtrip() {
        let input = default_input();
        let out = calculate_commitment_pacing(&input).unwrap();
        let json = serde_json::to_string(&out).unwrap();
        let _: CommitmentPacingOutput = serde_json::from_str(&json).unwrap();
    }

    #[test]
    fn test_single_year_projection() {
        let mut input = default_input();
        input.planning_years = 1;
        let out = calculate_commitment_pacing(&input).unwrap();
        assert_eq!(out.yearly_projections.len(), 1);
    }
}
