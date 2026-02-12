//! Direct indexing and tax-loss harvesting analytics.
//!
//! Evaluates a portfolio of individual holdings to identify tax-loss harvesting
//! opportunities, estimate tax alpha, and assess tracking error impact.
//!
//! All arithmetic uses `rust_decimal::Decimal`. No `f64`.

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

use crate::error::CorpFinanceError;
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Newton's method square root for Decimal.
fn decimal_sqrt(x: Decimal) -> Decimal {
    if x <= Decimal::ZERO {
        return Decimal::ZERO;
    }
    let mut guess = x / dec!(2);
    if guess == Decimal::ZERO {
        guess = Decimal::ONE;
    }
    for _ in 0..30 {
        let next = (guess + x / guess) / dec!(2);
        if (next - guess).abs() < dec!(0.000000001) {
            return next;
        }
        guess = next;
    }
    guess
}

// ---------------------------------------------------------------------------
// Input / Output
// ---------------------------------------------------------------------------

/// A single holding in the portfolio.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Holding {
    /// Ticker symbol.
    pub ticker: String,
    /// Portfolio weight (0-1).
    pub weight: Decimal,
    /// Original cost basis.
    pub cost_basis: Decimal,
    /// Current market value.
    pub current_value: Decimal,
    /// Holding period in days.
    pub holding_period_days: u32,
}

/// Input for direct indexing analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectIndexingInput {
    /// Total portfolio value.
    pub portfolio_value: Decimal,
    /// Individual holdings.
    pub holdings: Vec<Holding>,
    /// Benchmark return for the period.
    pub benchmark_return: Decimal,
    /// Long-term capital gains tax rate.
    pub tax_rate_ltcg: Decimal,
    /// Short-term capital gains tax rate.
    pub tax_rate_stcg: Decimal,
    /// Wash sale window in days (default 30).
    pub wash_sale_window: u32,
    /// Maximum acceptable tracking error vs benchmark.
    pub tracking_error_budget: Decimal,
}

/// A candidate position for tax-loss harvesting.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarvestCandidate {
    /// Ticker symbol.
    pub ticker: String,
    /// Unrealized loss amount (positive number).
    pub loss_amount: Decimal,
    /// Holding period in days.
    pub holding_period_days: u32,
    /// Estimated tax benefit from harvesting.
    pub tax_benefit: Decimal,
}

/// Output of direct indexing analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectIndexingOutput {
    /// Total unrealized gains across portfolio.
    pub total_unrealized_gains: Decimal,
    /// Total unrealized losses across portfolio.
    pub total_unrealized_losses: Decimal,
    /// Losses harvestable (outside wash sale window).
    pub harvestable_losses: Decimal,
    /// Tax alpha = estimated_tax_savings / portfolio_value.
    pub tax_alpha: Decimal,
    /// Estimated tax savings from harvesting.
    pub estimated_tax_savings: Decimal,
    /// Short-term losses (held < 366 days).
    pub short_term_losses: Decimal,
    /// Long-term losses (held >= 366 days).
    pub long_term_losses: Decimal,
    /// Positions recommended for harvesting.
    pub positions_to_harvest: Vec<HarvestCandidate>,
    /// Estimated tracking error from harvesting.
    pub tracking_error_impact: Decimal,
    /// Net tax alpha after estimated transaction costs.
    pub net_tax_alpha: Decimal,
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

fn validate(input: &DirectIndexingInput) -> CorpFinanceResult<()> {
    if input.portfolio_value <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "portfolio_value".into(),
            reason: "must be positive".into(),
        });
    }
    if input.holdings.is_empty() {
        return Err(CorpFinanceError::InsufficientData(
            "holdings cannot be empty".into(),
        ));
    }
    if input.tax_rate_ltcg < Decimal::ZERO || input.tax_rate_ltcg > Decimal::ONE {
        return Err(CorpFinanceError::InvalidInput {
            field: "tax_rate_ltcg".into(),
            reason: "must be between 0 and 1".into(),
        });
    }
    if input.tax_rate_stcg < Decimal::ZERO || input.tax_rate_stcg > Decimal::ONE {
        return Err(CorpFinanceError::InvalidInput {
            field: "tax_rate_stcg".into(),
            reason: "must be between 0 and 1".into(),
        });
    }
    if input.tracking_error_budget < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "tracking_error_budget".into(),
            reason: "cannot be negative".into(),
        });
    }
    for h in &input.holdings {
        if h.current_value < Decimal::ZERO {
            return Err(CorpFinanceError::InvalidInput {
                field: format!("holdings[{}].current_value", h.ticker),
                reason: "cannot be negative".into(),
            });
        }
        if h.weight < Decimal::ZERO || h.weight > Decimal::ONE {
            return Err(CorpFinanceError::InvalidInput {
                field: format!("holdings[{}].weight", h.ticker),
                reason: "must be between 0 and 1".into(),
            });
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Core function
// ---------------------------------------------------------------------------

/// Analyse a direct indexing portfolio for tax-loss harvesting opportunities.
pub fn analyze_direct_indexing(
    input: &DirectIndexingInput,
) -> CorpFinanceResult<DirectIndexingOutput> {
    validate(input)?;

    let lt_threshold = 366u32; // days for long-term treatment

    let mut total_gains = Decimal::ZERO;
    let mut total_losses = Decimal::ZERO;
    let mut harvestable = Decimal::ZERO;
    let mut st_losses = Decimal::ZERO;
    let mut lt_losses = Decimal::ZERO;
    let mut candidates: Vec<HarvestCandidate> = Vec::new();

    for h in &input.holdings {
        let pnl = h.current_value - h.cost_basis;
        if pnl > Decimal::ZERO {
            total_gains += pnl;
        } else if pnl < Decimal::ZERO {
            let loss = pnl.abs();
            total_losses += loss;

            let is_long_term = h.holding_period_days >= lt_threshold;

            if is_long_term {
                lt_losses += loss;
            } else {
                st_losses += loss;
            }

            // Only harvestable if outside wash sale window
            // (We check holding period > wash_sale_window as a proxy: if position
            // was recently acquired within wash_sale_window days, the loss may not
            // be harvestable. For simplicity, all current losses are harvestable
            // unless holding_period <= wash_sale_window, indicating a recent repurchase.)
            let outside_wash = h.holding_period_days > input.wash_sale_window;
            if outside_wash {
                harvestable += loss;
                let tax_rate = if is_long_term {
                    input.tax_rate_ltcg
                } else {
                    input.tax_rate_stcg
                };
                let benefit = loss * tax_rate;
                candidates.push(HarvestCandidate {
                    ticker: h.ticker.clone(),
                    loss_amount: loss,
                    holding_period_days: h.holding_period_days,
                    tax_benefit: benefit,
                });
            }
        }
    }

    // Sort candidates by tax benefit descending
    candidates.sort_by(|a, b| b.tax_benefit.cmp(&a.tax_benefit));

    // Estimated tax savings
    let estimated_tax_savings: Decimal = candidates.iter().map(|c| c.tax_benefit).sum();

    // Tax alpha
    let tax_alpha = if input.portfolio_value > Decimal::ZERO {
        estimated_tax_savings / input.portfolio_value
    } else {
        Decimal::ZERO
    };

    // Tracking error impact from harvesting
    // Simplified: TE ~ sqrt(sum of weight^2 of harvested positions) * position volatility proxy
    let harvested_weight_sq: Decimal = candidates
        .iter()
        .filter_map(|c| {
            input
                .holdings
                .iter()
                .find(|h| h.ticker == c.ticker)
                .map(|h| h.weight * h.weight)
        })
        .sum();
    // Assume ~20% annualized vol for individual stocks
    let stock_vol = dec!(0.20);
    let tracking_error_impact = decimal_sqrt(harvested_weight_sq) * stock_vol;

    // Transaction cost estimate: ~5bps per harvested position
    let transaction_costs =
        Decimal::from(candidates.len() as u32) * dec!(0.0005) * input.portfolio_value
            / Decimal::from(input.holdings.len().max(1) as u32);

    let net_tax_alpha = if input.portfolio_value > Decimal::ZERO {
        (estimated_tax_savings - transaction_costs) / input.portfolio_value
    } else {
        Decimal::ZERO
    };

    Ok(DirectIndexingOutput {
        total_unrealized_gains: total_gains,
        total_unrealized_losses: total_losses,
        harvestable_losses: harvestable,
        tax_alpha,
        estimated_tax_savings,
        short_term_losses: st_losses,
        long_term_losses: lt_losses,
        positions_to_harvest: candidates,
        tracking_error_impact,
        net_tax_alpha,
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn make_holding(
        ticker: &str,
        weight: Decimal,
        basis: Decimal,
        current: Decimal,
        days: u32,
    ) -> Holding {
        Holding {
            ticker: ticker.into(),
            weight,
            cost_basis: basis,
            current_value: current,
            holding_period_days: days,
        }
    }

    fn base_input() -> DirectIndexingInput {
        DirectIndexingInput {
            portfolio_value: dec!(1_000_000),
            holdings: vec![
                make_holding("AAPL", dec!(0.10), dec!(120_000), dec!(100_000), 400),
                make_holding("MSFT", dec!(0.10), dec!(80_000), dec!(110_000), 500),
                make_holding("GOOG", dec!(0.08), dec!(90_000), dec!(70_000), 200),
                make_holding("AMZN", dec!(0.07), dec!(60_000), dec!(75_000), 100),
                make_holding("META", dec!(0.05), dec!(55_000), dec!(40_000), 60),
                make_holding("NVDA", dec!(0.10), dec!(50_000), dec!(130_000), 300),
                make_holding("TSLA", dec!(0.06), dec!(70_000), dec!(50_000), 20),
            ],
            benchmark_return: dec!(0.10),
            tax_rate_ltcg: dec!(0.20),
            tax_rate_stcg: dec!(0.37),
            wash_sale_window: 30,
            tracking_error_budget: dec!(0.02),
        }
    }

    #[test]
    fn test_total_unrealized_gains() {
        let out = analyze_direct_indexing(&base_input()).unwrap();
        // MSFT +30k, AMZN +15k, NVDA +80k = 125k
        assert_eq!(out.total_unrealized_gains, dec!(125_000));
    }

    #[test]
    fn test_total_unrealized_losses() {
        let out = analyze_direct_indexing(&base_input()).unwrap();
        // AAPL -20k, GOOG -20k, META -15k, TSLA -20k = 75k
        assert_eq!(out.total_unrealized_losses, dec!(75_000));
    }

    #[test]
    fn test_harvestable_excludes_wash_sale() {
        let out = analyze_direct_indexing(&base_input()).unwrap();
        // TSLA held 20 days <= 30 wash sale window -> not harvestable
        // META held 60 > 30 -> harvestable
        // Harvestable = AAPL 20k + GOOG 20k + META 15k = 55k
        assert_eq!(out.harvestable_losses, dec!(55_000));
    }

    #[test]
    fn test_short_term_losses() {
        let out = analyze_direct_indexing(&base_input()).unwrap();
        // GOOG 200 days (ST) = 20k, META 60 (ST) = 15k, TSLA 20 (ST) = 20k
        assert_eq!(out.short_term_losses, dec!(55_000));
    }

    #[test]
    fn test_long_term_losses() {
        let out = analyze_direct_indexing(&base_input()).unwrap();
        // AAPL 400 days (LT) = 20k
        assert_eq!(out.long_term_losses, dec!(20_000));
    }

    #[test]
    fn test_positions_to_harvest_count() {
        let out = analyze_direct_indexing(&base_input()).unwrap();
        // AAPL, GOOG, META (TSLA excluded by wash sale)
        assert_eq!(out.positions_to_harvest.len(), 3);
    }

    #[test]
    fn test_candidates_sorted_by_benefit() {
        let out = analyze_direct_indexing(&base_input()).unwrap();
        for i in 1..out.positions_to_harvest.len() {
            assert!(
                out.positions_to_harvest[i - 1].tax_benefit
                    >= out.positions_to_harvest[i].tax_benefit
            );
        }
    }

    #[test]
    fn test_tax_alpha_positive() {
        let out = analyze_direct_indexing(&base_input()).unwrap();
        assert!(out.tax_alpha > Decimal::ZERO);
    }

    #[test]
    fn test_tax_savings_calculated() {
        let out = analyze_direct_indexing(&base_input()).unwrap();
        // AAPL: 20k * 0.20(LT) = 4k, GOOG: 20k * 0.37(ST) = 7.4k, META: 15k * 0.37(ST) = 5.55k
        // Total = 16.95k
        let expected = dec!(4_000) + dec!(7_400) + dec!(5_550);
        assert_eq!(out.estimated_tax_savings, expected);
    }

    #[test]
    fn test_no_losses_portfolio() {
        let inp = DirectIndexingInput {
            portfolio_value: dec!(500_000),
            holdings: vec![
                make_holding("AAPL", dec!(0.50), dec!(100_000), dec!(250_000), 400),
                make_holding("MSFT", dec!(0.50), dec!(100_000), dec!(250_000), 500),
            ],
            benchmark_return: dec!(0.10),
            tax_rate_ltcg: dec!(0.20),
            tax_rate_stcg: dec!(0.37),
            wash_sale_window: 30,
            tracking_error_budget: dec!(0.02),
        };
        let out = analyze_direct_indexing(&inp).unwrap();
        assert_eq!(out.total_unrealized_losses, Decimal::ZERO);
        assert_eq!(out.harvestable_losses, Decimal::ZERO);
        assert_eq!(out.estimated_tax_savings, Decimal::ZERO);
        assert!(out.positions_to_harvest.is_empty());
    }

    #[test]
    fn test_all_within_wash_sale() {
        let inp = DirectIndexingInput {
            portfolio_value: dec!(500_000),
            holdings: vec![
                make_holding("AAPL", dec!(0.50), dec!(300_000), dec!(200_000), 10),
                make_holding("MSFT", dec!(0.50), dec!(300_000), dec!(200_000), 15),
            ],
            benchmark_return: dec!(0.10),
            tax_rate_ltcg: dec!(0.20),
            tax_rate_stcg: dec!(0.37),
            wash_sale_window: 30,
            tracking_error_budget: dec!(0.02),
        };
        let out = analyze_direct_indexing(&inp).unwrap();
        assert_eq!(out.total_unrealized_losses, dec!(200_000));
        assert_eq!(out.harvestable_losses, Decimal::ZERO);
        assert!(out.positions_to_harvest.is_empty());
    }

    #[test]
    fn test_tracking_error_non_negative() {
        let out = analyze_direct_indexing(&base_input()).unwrap();
        assert!(out.tracking_error_impact >= Decimal::ZERO);
    }

    #[test]
    fn test_net_tax_alpha_less_than_gross() {
        let out = analyze_direct_indexing(&base_input()).unwrap();
        assert!(out.net_tax_alpha <= out.tax_alpha);
    }

    #[test]
    fn test_invalid_portfolio_value() {
        let mut inp = base_input();
        inp.portfolio_value = Decimal::ZERO;
        assert!(analyze_direct_indexing(&inp).is_err());
    }

    #[test]
    fn test_empty_holdings() {
        let mut inp = base_input();
        inp.holdings = vec![];
        assert!(analyze_direct_indexing(&inp).is_err());
    }

    #[test]
    fn test_invalid_tax_rate() {
        let mut inp = base_input();
        inp.tax_rate_ltcg = dec!(1.5);
        assert!(analyze_direct_indexing(&inp).is_err());
    }

    #[test]
    fn test_invalid_weight() {
        let mut inp = base_input();
        inp.holdings[0].weight = dec!(1.5);
        assert!(analyze_direct_indexing(&inp).is_err());
    }

    #[test]
    fn test_single_holding_loss() {
        let inp = DirectIndexingInput {
            portfolio_value: dec!(100_000),
            holdings: vec![make_holding(
                "AAPL",
                dec!(1.0),
                dec!(120_000),
                dec!(100_000),
                400,
            )],
            benchmark_return: dec!(0.10),
            tax_rate_ltcg: dec!(0.20),
            tax_rate_stcg: dec!(0.37),
            wash_sale_window: 30,
            tracking_error_budget: dec!(0.02),
        };
        let out = analyze_direct_indexing(&inp).unwrap();
        assert_eq!(out.harvestable_losses, dec!(20_000));
        assert_eq!(out.positions_to_harvest.len(), 1);
        assert_eq!(out.positions_to_harvest[0].tax_benefit, dec!(4_000));
    }
}
