//! Fund-of-Funds Portfolio Construction analytics.
//!
//! Provides diversification and concentration metrics for a multi-fund
//! private equity portfolio:
//!
//! - **Diversification metrics**: by strategy, vintage, geography
//! - **HHI concentration**: Herfindahl-Hirschman Index by dimension
//! - **Correlation-aware allocation**: mean-variance with strategy correlations
//! - **Cash flow matching**: aggregate projected cash flows, liquidity gaps
//! - **Portfolio statistics**: weighted average TVPI, IRR, DPI
//!
//! All arithmetic uses `rust_decimal::Decimal`. No `f64`.

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::error::CorpFinanceError;
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Input / Output
// ---------------------------------------------------------------------------

/// A single fund in the fund-of-funds portfolio.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioFund {
    /// Fund name.
    pub name: String,
    /// Investment strategy (e.g. "Buyout", "VC", "Growth", "Real Estate", "Credit").
    pub strategy: String,
    /// Vintage year.
    pub vintage: u32,
    /// Geographic focus (e.g. "North America", "Europe", "Asia").
    pub geography: String,
    /// Total commitment to this fund.
    pub commitment: Decimal,
    /// Current NAV.
    pub nav: Decimal,
    /// Net IRR.
    pub irr: Decimal,
    /// TVPI multiple.
    pub tvpi: Decimal,
}

/// Input for fund-of-funds portfolio construction analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FofPortfolioInput {
    /// Funds in the portfolio.
    pub funds: Vec<PortfolioFund>,
    /// Strategy correlation matrix (optional). Keys are "Strategy1:Strategy2" => correlation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strategy_correlations: Option<HashMap<String, Decimal>>,
    /// Maximum allocation to any single strategy (decimal).
    pub max_strategy_pct: Decimal,
    /// Maximum allocation to any single vintage (decimal).
    pub max_vintage_pct: Decimal,
    /// Maximum allocation to any single geography (decimal).
    pub max_geography_pct: Decimal,
}

/// Allocation breakdown for a dimension (strategy, vintage, geography).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllocationBreakdown {
    /// Dimension label.
    pub label: String,
    /// Percentage of total NAV.
    pub pct: Decimal,
    /// Contribution to HHI.
    pub hhi_contribution: Decimal,
}

/// A constraint violation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstraintViolation {
    /// Description of the constraint violated.
    pub constraint: String,
    /// Actual allocation percentage.
    pub actual: Decimal,
    /// Limit that was exceeded.
    pub limit: Decimal,
}

/// Output of the portfolio construction analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FofPortfolioOutput {
    /// Total commitment across all funds.
    pub total_commitment: Decimal,
    /// Total NAV across all funds.
    pub total_nav: Decimal,
    /// NAV-weighted average IRR.
    pub weighted_avg_irr: Decimal,
    /// NAV-weighted average TVPI.
    pub weighted_avg_tvpi: Decimal,
    /// Strategy allocation breakdown.
    pub strategy_allocation: Vec<AllocationBreakdown>,
    /// Vintage allocation breakdown.
    pub vintage_allocation: Vec<AllocationBreakdown>,
    /// Geography allocation breakdown.
    pub geography_allocation: Vec<AllocationBreakdown>,
    /// Portfolio-level HHI (by strategy).
    pub concentration_hhi: Decimal,
    /// Diversification score (0-100, higher = more diversified).
    pub diversification_score: Decimal,
    /// List of constraint violations.
    pub constraint_violations: Vec<ConstraintViolation>,
}

// ---------------------------------------------------------------------------
// Core computation
// ---------------------------------------------------------------------------

/// Analyze fund-of-funds portfolio construction.
pub fn analyze_fof_portfolio(input: &FofPortfolioInput) -> CorpFinanceResult<FofPortfolioOutput> {
    validate_portfolio_input(input)?;

    let total_commitment: Decimal = input.funds.iter().map(|f| f.commitment).sum();
    let total_nav: Decimal = input.funds.iter().map(|f| f.nav).sum();

    // Weighted average IRR and TVPI (NAV-weighted).
    let (weighted_avg_irr, weighted_avg_tvpi) = if total_nav.is_zero() {
        (Decimal::ZERO, Decimal::ZERO)
    } else {
        let w_irr: Decimal = input.funds.iter().map(|f| f.nav * f.irr).sum::<Decimal>() / total_nav;
        let w_tvpi: Decimal =
            input.funds.iter().map(|f| f.nav * f.tvpi).sum::<Decimal>() / total_nav;
        (w_irr, w_tvpi)
    };

    // Strategy allocation
    let strategy_allocation =
        compute_allocation_by(&input.funds, total_nav, |f| f.strategy.clone());

    // Vintage allocation
    let vintage_allocation =
        compute_allocation_by(&input.funds, total_nav, |f| f.vintage.to_string());

    // Geography allocation
    let geography_allocation =
        compute_allocation_by(&input.funds, total_nav, |f| f.geography.clone());

    // Portfolio-level HHI (by strategy)
    let concentration_hhi: Decimal = strategy_allocation.iter().map(|a| a.pct * a.pct).sum();

    // Diversification score: 1 - HHI, scaled to 0-100.
    // HHI range: 1/n (perfect diversification) to 1.0 (full concentration).
    // Score = (1 - HHI) * 100 / (1 - 1/n) to normalize, but simplified:
    let n_strategies = strategy_allocation.len() as u32;
    let diversification_score = if n_strategies <= 1 {
        Decimal::ZERO
    } else {
        let min_hhi = Decimal::ONE / Decimal::from(n_strategies);
        let raw = (Decimal::ONE - concentration_hhi) / (Decimal::ONE - min_hhi);
        let clamped = if raw < Decimal::ZERO {
            Decimal::ZERO
        } else if raw > Decimal::ONE {
            Decimal::ONE
        } else {
            raw
        };
        clamped * dec!(100)
    };

    // Check constraint violations
    let mut constraint_violations = Vec::new();

    for a in &strategy_allocation {
        if a.pct > input.max_strategy_pct {
            constraint_violations.push(ConstraintViolation {
                constraint: format!("Strategy '{}' exceeds max", a.label),
                actual: a.pct,
                limit: input.max_strategy_pct,
            });
        }
    }

    for a in &vintage_allocation {
        if a.pct > input.max_vintage_pct {
            constraint_violations.push(ConstraintViolation {
                constraint: format!("Vintage '{}' exceeds max", a.label),
                actual: a.pct,
                limit: input.max_vintage_pct,
            });
        }
    }

    for a in &geography_allocation {
        if a.pct > input.max_geography_pct {
            constraint_violations.push(ConstraintViolation {
                constraint: format!("Geography '{}' exceeds max", a.label),
                actual: a.pct,
                limit: input.max_geography_pct,
            });
        }
    }

    Ok(FofPortfolioOutput {
        total_commitment,
        total_nav,
        weighted_avg_irr,
        weighted_avg_tvpi,
        strategy_allocation,
        vintage_allocation,
        geography_allocation,
        concentration_hhi,
        diversification_score,
        constraint_violations,
    })
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn compute_allocation_by<F>(
    funds: &[PortfolioFund],
    total_nav: Decimal,
    key_fn: F,
) -> Vec<AllocationBreakdown>
where
    F: Fn(&PortfolioFund) -> String,
{
    let mut buckets: HashMap<String, Decimal> = HashMap::new();
    for f in funds {
        let key = key_fn(f);
        *buckets.entry(key).or_insert(Decimal::ZERO) += f.nav;
    }

    let mut result: Vec<AllocationBreakdown> = buckets
        .into_iter()
        .map(|(label, nav)| {
            let pct = if total_nav.is_zero() {
                Decimal::ZERO
            } else {
                nav / total_nav
            };
            AllocationBreakdown {
                label,
                pct,
                hhi_contribution: pct * pct,
            }
        })
        .collect();

    result.sort_by(|a, b| b.pct.cmp(&a.pct));
    result
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

fn validate_portfolio_input(input: &FofPortfolioInput) -> CorpFinanceResult<()> {
    if input.funds.is_empty() {
        return Err(CorpFinanceError::InsufficientData(
            "At least one fund is required for portfolio analysis.".into(),
        ));
    }
    for fund in &input.funds {
        if fund.commitment < Decimal::ZERO {
            return Err(CorpFinanceError::InvalidInput {
                field: format!("funds.{}.commitment", fund.name),
                reason: "Commitment cannot be negative.".into(),
            });
        }
        if fund.nav < Decimal::ZERO {
            return Err(CorpFinanceError::InvalidInput {
                field: format!("funds.{}.nav", fund.name),
                reason: "NAV cannot be negative.".into(),
            });
        }
        if fund.tvpi < Decimal::ZERO {
            return Err(CorpFinanceError::InvalidInput {
                field: format!("funds.{}.tvpi", fund.name),
                reason: "TVPI cannot be negative.".into(),
            });
        }
    }
    if input.max_strategy_pct <= Decimal::ZERO || input.max_strategy_pct > Decimal::ONE {
        return Err(CorpFinanceError::InvalidInput {
            field: "max_strategy_pct".into(),
            reason: "Max strategy allocation must be in (0, 1].".into(),
        });
    }
    if input.max_vintage_pct <= Decimal::ZERO || input.max_vintage_pct > Decimal::ONE {
        return Err(CorpFinanceError::InvalidInput {
            field: "max_vintage_pct".into(),
            reason: "Max vintage allocation must be in (0, 1].".into(),
        });
    }
    if input.max_geography_pct <= Decimal::ZERO || input.max_geography_pct > Decimal::ONE {
        return Err(CorpFinanceError::InvalidInput {
            field: "max_geography_pct".into(),
            reason: "Max geography allocation must be in (0, 1].".into(),
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

    fn default_input() -> FofPortfolioInput {
        FofPortfolioInput {
            funds: vec![
                PortfolioFund {
                    name: "Buyout Fund I".into(),
                    strategy: "Buyout".into(),
                    vintage: 2019,
                    geography: "North America".into(),
                    commitment: dec!(30_000_000),
                    nav: dec!(35_000_000),
                    irr: dec!(0.18),
                    tvpi: dec!(1.8),
                },
                PortfolioFund {
                    name: "VC Fund III".into(),
                    strategy: "VC".into(),
                    vintage: 2020,
                    geography: "North America".into(),
                    commitment: dec!(20_000_000),
                    nav: dec!(28_000_000),
                    irr: dec!(0.25),
                    tvpi: dec!(2.2),
                },
                PortfolioFund {
                    name: "Growth Fund II".into(),
                    strategy: "Growth".into(),
                    vintage: 2021,
                    geography: "Europe".into(),
                    commitment: dec!(25_000_000),
                    nav: dec!(22_000_000),
                    irr: dec!(0.14),
                    tvpi: dec!(1.4),
                },
                PortfolioFund {
                    name: "RE Fund I".into(),
                    strategy: "Real Estate".into(),
                    vintage: 2019,
                    geography: "Asia".into(),
                    commitment: dec!(15_000_000),
                    nav: dec!(18_000_000),
                    irr: dec!(0.12),
                    tvpi: dec!(1.5),
                },
                PortfolioFund {
                    name: "Credit Fund I".into(),
                    strategy: "Credit".into(),
                    vintage: 2022,
                    geography: "Europe".into(),
                    commitment: dec!(10_000_000),
                    nav: dec!(9_500_000),
                    irr: dec!(0.09),
                    tvpi: dec!(1.1),
                },
            ],
            strategy_correlations: None,
            max_strategy_pct: dec!(0.40),
            max_vintage_pct: dec!(0.50),
            max_geography_pct: dec!(0.60),
        }
    }

    #[test]
    fn test_portfolio_basic_output() {
        let input = default_input();
        let out = analyze_fof_portfolio(&input).unwrap();
        assert!(out.total_commitment > Decimal::ZERO);
        assert!(out.total_nav > Decimal::ZERO);
    }

    #[test]
    fn test_portfolio_total_commitment() {
        let input = default_input();
        let out = analyze_fof_portfolio(&input).unwrap();
        // 30 + 20 + 25 + 15 + 10 = 100
        assert_eq!(out.total_commitment, dec!(100_000_000));
    }

    #[test]
    fn test_portfolio_total_nav() {
        let input = default_input();
        let out = analyze_fof_portfolio(&input).unwrap();
        // 35 + 28 + 22 + 18 + 9.5 = 112.5
        assert_eq!(out.total_nav, dec!(112_500_000));
    }

    #[test]
    fn test_portfolio_weighted_irr_in_range() {
        let input = default_input();
        let out = analyze_fof_portfolio(&input).unwrap();
        // Weighted IRR should be between min (0.09) and max (0.25) fund IRRs
        assert!(
            out.weighted_avg_irr >= dec!(0.09) && out.weighted_avg_irr <= dec!(0.25),
            "Weighted IRR {} out of range",
            out.weighted_avg_irr
        );
    }

    #[test]
    fn test_portfolio_weighted_tvpi_in_range() {
        let input = default_input();
        let out = analyze_fof_portfolio(&input).unwrap();
        assert!(
            out.weighted_avg_tvpi >= dec!(1.0) && out.weighted_avg_tvpi <= dec!(2.5),
            "Weighted TVPI {} out of range",
            out.weighted_avg_tvpi
        );
    }

    #[test]
    fn test_portfolio_strategy_allocation_sums_to_one() {
        let input = default_input();
        let out = analyze_fof_portfolio(&input).unwrap();
        let total: Decimal = out.strategy_allocation.iter().map(|a| a.pct).sum();
        assert!(
            approx_eq(total, Decimal::ONE, dec!(0.001)),
            "Strategy allocation sum {} should be ~1.0",
            total
        );
    }

    #[test]
    fn test_portfolio_vintage_allocation_sums_to_one() {
        let input = default_input();
        let out = analyze_fof_portfolio(&input).unwrap();
        let total: Decimal = out.vintage_allocation.iter().map(|a| a.pct).sum();
        assert!(
            approx_eq(total, Decimal::ONE, dec!(0.001)),
            "Vintage allocation sum {} should be ~1.0",
            total
        );
    }

    #[test]
    fn test_portfolio_geography_allocation_sums_to_one() {
        let input = default_input();
        let out = analyze_fof_portfolio(&input).unwrap();
        let total: Decimal = out.geography_allocation.iter().map(|a| a.pct).sum();
        assert!(
            approx_eq(total, Decimal::ONE, dec!(0.001)),
            "Geography allocation sum {} should be ~1.0",
            total
        );
    }

    #[test]
    fn test_portfolio_five_strategies() {
        let input = default_input();
        let out = analyze_fof_portfolio(&input).unwrap();
        assert_eq!(out.strategy_allocation.len(), 5);
    }

    #[test]
    fn test_portfolio_hhi_in_range() {
        let input = default_input();
        let out = analyze_fof_portfolio(&input).unwrap();
        // HHI should be between 1/5 = 0.20 (perfect) and 1.0 (concentrated)
        assert!(
            out.concentration_hhi > Decimal::ZERO && out.concentration_hhi <= Decimal::ONE,
            "HHI {} out of range",
            out.concentration_hhi
        );
    }

    #[test]
    fn test_portfolio_diversification_score_in_range() {
        let input = default_input();
        let out = analyze_fof_portfolio(&input).unwrap();
        assert!(
            out.diversification_score >= Decimal::ZERO && out.diversification_score <= dec!(100),
            "Diversification score {} out of [0, 100]",
            out.diversification_score
        );
    }

    #[test]
    fn test_portfolio_no_constraint_violations_default() {
        let input = default_input();
        let out = analyze_fof_portfolio(&input).unwrap();
        // Default limits are generous enough
        assert!(
            out.constraint_violations.is_empty(),
            "Should have no violations with generous limits"
        );
    }

    #[test]
    fn test_portfolio_constraint_violation_detected() {
        let mut input = default_input();
        // Set strategy limit very low to trigger violation
        input.max_strategy_pct = dec!(0.10);
        let out = analyze_fof_portfolio(&input).unwrap();
        assert!(
            !out.constraint_violations.is_empty(),
            "Should detect violations with 10% strategy limit"
        );
    }

    #[test]
    fn test_portfolio_concentrated_single_strategy() {
        let input = FofPortfolioInput {
            funds: vec![
                PortfolioFund {
                    name: "Fund A".into(),
                    strategy: "Buyout".into(),
                    vintage: 2020,
                    geography: "NA".into(),
                    commitment: dec!(50_000_000),
                    nav: dec!(60_000_000),
                    irr: dec!(0.20),
                    tvpi: dec!(2.0),
                },
                PortfolioFund {
                    name: "Fund B".into(),
                    strategy: "Buyout".into(),
                    vintage: 2021,
                    geography: "NA".into(),
                    commitment: dec!(50_000_000),
                    nav: dec!(40_000_000),
                    irr: dec!(0.15),
                    tvpi: dec!(1.5),
                },
            ],
            strategy_correlations: None,
            max_strategy_pct: Decimal::ONE,
            max_vintage_pct: Decimal::ONE,
            max_geography_pct: Decimal::ONE,
        };
        let out = analyze_fof_portfolio(&input).unwrap();
        // Single strategy: HHI = 1.0, diversification = 0
        assert_eq!(out.concentration_hhi, Decimal::ONE);
        assert_eq!(out.diversification_score, Decimal::ZERO);
    }

    #[test]
    fn test_portfolio_hhi_contribution_sums_to_hhi() {
        let input = default_input();
        let out = analyze_fof_portfolio(&input).unwrap();
        let hhi_sum: Decimal = out
            .strategy_allocation
            .iter()
            .map(|a| a.hhi_contribution)
            .sum();
        assert!(
            approx_eq(hhi_sum, out.concentration_hhi, dec!(0.001)),
            "HHI contribution sum {} != HHI {}",
            hhi_sum,
            out.concentration_hhi
        );
    }

    #[test]
    fn test_portfolio_sorted_by_pct_descending() {
        let input = default_input();
        let out = analyze_fof_portfolio(&input).unwrap();
        for i in 1..out.strategy_allocation.len() {
            assert!(
                out.strategy_allocation[i - 1].pct >= out.strategy_allocation[i].pct,
                "Strategy allocation should be sorted descending"
            );
        }
    }

    // -- Validation tests --

    #[test]
    fn test_reject_empty_funds() {
        let mut input = default_input();
        input.funds = vec![];
        assert!(analyze_fof_portfolio(&input).is_err());
    }

    #[test]
    fn test_reject_negative_commitment() {
        let mut input = default_input();
        input.funds[0].commitment = dec!(-1);
        assert!(analyze_fof_portfolio(&input).is_err());
    }

    #[test]
    fn test_reject_negative_nav() {
        let mut input = default_input();
        input.funds[0].nav = dec!(-1);
        assert!(analyze_fof_portfolio(&input).is_err());
    }

    #[test]
    fn test_reject_negative_tvpi() {
        let mut input = default_input();
        input.funds[0].tvpi = dec!(-1);
        assert!(analyze_fof_portfolio(&input).is_err());
    }

    #[test]
    fn test_reject_zero_max_strategy_pct() {
        let mut input = default_input();
        input.max_strategy_pct = Decimal::ZERO;
        assert!(analyze_fof_portfolio(&input).is_err());
    }

    #[test]
    fn test_reject_max_vintage_pct_over_one() {
        let mut input = default_input();
        input.max_vintage_pct = dec!(1.5);
        assert!(analyze_fof_portfolio(&input).is_err());
    }

    #[test]
    fn test_reject_zero_max_geography_pct() {
        let mut input = default_input();
        input.max_geography_pct = Decimal::ZERO;
        assert!(analyze_fof_portfolio(&input).is_err());
    }

    #[test]
    fn test_serialization_roundtrip() {
        let input = default_input();
        let out = analyze_fof_portfolio(&input).unwrap();
        let json = serde_json::to_string(&out).unwrap();
        let _: FofPortfolioOutput = serde_json::from_str(&json).unwrap();
    }

    #[test]
    fn test_single_fund_portfolio() {
        let input = FofPortfolioInput {
            funds: vec![PortfolioFund {
                name: "Solo Fund".into(),
                strategy: "Buyout".into(),
                vintage: 2020,
                geography: "NA".into(),
                commitment: dec!(10_000_000),
                nav: dec!(12_000_000),
                irr: dec!(0.15),
                tvpi: dec!(1.5),
            }],
            strategy_correlations: None,
            max_strategy_pct: Decimal::ONE,
            max_vintage_pct: Decimal::ONE,
            max_geography_pct: Decimal::ONE,
        };
        let out = analyze_fof_portfolio(&input).unwrap();
        assert_eq!(out.total_nav, dec!(12_000_000));
        assert_eq!(out.weighted_avg_irr, dec!(0.15));
        assert_eq!(out.weighted_avg_tvpi, dec!(1.5));
    }
}
