//! Index Rebalancing Analysis.
//!
//! Covers:
//! 1. **Trade Generation** -- compute buy/sell orders to move from current to target weights
//! 2. **Turnover** -- total and one-way turnover measurement
//! 3. **Transaction Costs** -- cost estimation from bps-based model
//! 4. **Market Impact** -- simplified square-root impact model
//! 5. **Execution Days** -- estimated days to execute based on ADV
//! 6. **Optimal Frequency** -- heuristic based on turnover/cost tradeoff
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

/// Newton's method square root for Decimal (20 iterations).
fn decimal_sqrt(x: Decimal) -> Decimal {
    if x <= Decimal::ZERO {
        return Decimal::ZERO;
    }
    let mut guess = x / dec!(2);
    if guess.is_zero() {
        guess = Decimal::ONE;
    }
    for _ in 0..20 {
        guess = (guess + x / guess) / dec!(2);
    }
    guess
}

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// A single position with current vs target weight.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionWeight {
    pub ticker: String,
    pub current_weight: Decimal,
    pub target_weight: Decimal,
    pub price: Decimal,
    pub avg_daily_volume: Decimal,
}

// ---------------------------------------------------------------------------
// Input / Output
// ---------------------------------------------------------------------------

/// Input for rebalancing analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RebalancingInput {
    pub current_weights: Vec<PositionWeight>,
    pub portfolio_value: Decimal,
    /// Round-trip transaction cost in basis points.
    pub transaction_cost_bps: Decimal,
    /// Minimum absolute deviation to trigger a rebalance.
    pub rebalance_threshold: Decimal,
    /// "daily", "monthly", "quarterly", "annually"
    pub rebalance_frequency: String,
}

/// A generated trade order.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeOrder {
    pub ticker: String,
    /// "buy" or "sell"
    pub action: String,
    pub weight_change: Decimal,
    pub notional: Decimal,
    pub days_to_execute: Decimal,
}

/// Output of the rebalancing analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RebalancingOutput {
    pub trades: Vec<TradeOrder>,
    pub total_turnover: Decimal,
    pub one_way_turnover: Decimal,
    pub total_transaction_cost: Decimal,
    pub cost_drag: Decimal,
    pub positions_rebalanced: u32,
    pub positions_unchanged: u32,
    pub market_impact_estimate: Decimal,
    pub optimal_frequency_estimate: String,
}

// ---------------------------------------------------------------------------
// Calculation
// ---------------------------------------------------------------------------

/// Perform rebalancing analysis on the portfolio.
pub fn calculate_rebalancing(input: &RebalancingInput) -> CorpFinanceResult<RebalancingOutput> {
    validate_rebalancing_input(input)?;

    let mut trades: Vec<TradeOrder> = Vec::new();
    let mut total_abs_change = Decimal::ZERO;
    let mut buy_total = Decimal::ZERO;
    let mut positions_rebalanced: u32 = 0;
    let mut positions_unchanged: u32 = 0;
    let mut total_impact = Decimal::ZERO;

    for pw in &input.current_weights {
        let diff = pw.target_weight - pw.current_weight;
        let abs_diff = diff.abs();

        if abs_diff <= input.rebalance_threshold {
            positions_unchanged += 1;
            continue;
        }

        positions_rebalanced += 1;
        total_abs_change += abs_diff;

        let notional = abs_diff * input.portfolio_value;
        let action = if diff > Decimal::ZERO {
            buy_total += abs_diff;
            "buy".to_string()
        } else {
            "sell".to_string()
        };

        // Days to execute = notional / (ADV * price * 0.20)
        let adv_value = pw.avg_daily_volume * pw.price;
        let participation = dec!(0.20);
        let days = if adv_value.is_zero() {
            dec!(999)
        } else {
            notional / (adv_value * participation)
        };

        // Market impact = sqrt(notional / ADV_value) * 10 bps
        let impact = if adv_value.is_zero() {
            Decimal::ZERO
        } else {
            decimal_sqrt(notional / adv_value) * dec!(0.0010)
        };
        total_impact += impact * notional;

        trades.push(TradeOrder {
            ticker: pw.ticker.clone(),
            action,
            weight_change: abs_diff,
            notional,
            days_to_execute: days,
        });
    }

    // Total turnover = sum(|delta|) / 2
    let total_turnover = total_abs_change / dec!(2);
    let one_way_turnover = buy_total;

    // Transaction cost = turnover * portfolio_value * cost_bps / 10000
    let total_transaction_cost =
        total_turnover * input.portfolio_value * input.transaction_cost_bps / dec!(10000);

    // Annualized cost drag based on frequency
    let rebal_per_year = match input.rebalance_frequency.as_str() {
        "daily" => dec!(252),
        "monthly" => dec!(12),
        "quarterly" => dec!(4),
        "annually" => dec!(1),
        _ => dec!(4),
    };
    let cost_drag = if input.portfolio_value.is_zero() {
        Decimal::ZERO
    } else {
        total_transaction_cost * rebal_per_year / input.portfolio_value
    };

    // Market impact as weighted average bps
    let market_impact_estimate = if input.portfolio_value.is_zero() {
        Decimal::ZERO
    } else {
        total_impact / input.portfolio_value
    };

    // Optimal frequency heuristic
    let optimal_frequency_estimate = if total_turnover < dec!(0.05) {
        "annually".to_string()
    } else if total_turnover < dec!(0.10) {
        "quarterly".to_string()
    } else {
        "monthly".to_string()
    };

    Ok(RebalancingOutput {
        trades,
        total_turnover,
        one_way_turnover,
        total_transaction_cost,
        cost_drag,
        positions_rebalanced,
        positions_unchanged,
        market_impact_estimate,
        optimal_frequency_estimate,
    })
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

fn validate_rebalancing_input(input: &RebalancingInput) -> CorpFinanceResult<()> {
    if input.current_weights.is_empty() {
        return Err(CorpFinanceError::InsufficientData(
            "At least one position is required".into(),
        ));
    }
    if input.portfolio_value <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "portfolio_value".into(),
            reason: "Portfolio value must be positive".into(),
        });
    }
    if input.transaction_cost_bps < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "transaction_cost_bps".into(),
            reason: "Transaction cost must be non-negative".into(),
        });
    }
    if input.rebalance_threshold < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "rebalance_threshold".into(),
            reason: "Threshold must be non-negative".into(),
        });
    }
    for pw in &input.current_weights {
        if pw.current_weight < Decimal::ZERO || pw.target_weight < Decimal::ZERO {
            return Err(CorpFinanceError::InvalidInput {
                field: "weight".into(),
                reason: format!("Negative weight for {}", pw.ticker),
            });
        }
        if pw.price < Decimal::ZERO {
            return Err(CorpFinanceError::InvalidInput {
                field: "price".into(),
                reason: format!("Negative price for {}", pw.ticker),
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

    fn make_position(ticker: &str, current: Decimal, target: Decimal) -> PositionWeight {
        PositionWeight {
            ticker: ticker.into(),
            current_weight: current,
            target_weight: target,
            price: dec!(50),
            avg_daily_volume: dec!(1_000_000),
        }
    }

    fn make_base_input() -> RebalancingInput {
        RebalancingInput {
            current_weights: vec![
                make_position("AAPL", dec!(0.30), dec!(0.25)),
                make_position("MSFT", dec!(0.25), dec!(0.25)),
                make_position("GOOG", dec!(0.20), dec!(0.25)),
                make_position("AMZN", dec!(0.15), dec!(0.15)),
                make_position("META", dec!(0.10), dec!(0.10)),
            ],
            portfolio_value: dec!(10_000_000),
            transaction_cost_bps: dec!(10),
            rebalance_threshold: dec!(0.01),
            rebalance_frequency: "quarterly".into(),
        }
    }

    // --- Basic rebalancing ---
    #[test]
    fn test_small_drift_rebalance() {
        let input = make_base_input();
        let out = calculate_rebalancing(&input).unwrap();
        // AAPL: sell 5%, GOOG: buy 5%, MSFT/AMZN/META unchanged
        assert_eq!(out.positions_rebalanced, 2);
        assert_eq!(out.positions_unchanged, 3);
    }

    #[test]
    fn test_trade_directions() {
        let input = make_base_input();
        let out = calculate_rebalancing(&input).unwrap();
        let aapl = out.trades.iter().find(|t| t.ticker == "AAPL").unwrap();
        let goog = out.trades.iter().find(|t| t.ticker == "GOOG").unwrap();
        assert_eq!(aapl.action, "sell");
        assert_eq!(goog.action, "buy");
    }

    #[test]
    fn test_weight_change_amounts() {
        let input = make_base_input();
        let out = calculate_rebalancing(&input).unwrap();
        let aapl = out.trades.iter().find(|t| t.ticker == "AAPL").unwrap();
        assert_eq!(aapl.weight_change, dec!(0.05));
    }

    #[test]
    fn test_notional_values() {
        let input = make_base_input();
        let out = calculate_rebalancing(&input).unwrap();
        let aapl = out.trades.iter().find(|t| t.ticker == "AAPL").unwrap();
        // 0.05 * 10M = 500,000
        assert_eq!(aapl.notional, dec!(500_000));
    }

    // --- Turnover ---
    #[test]
    fn test_total_turnover() {
        let input = make_base_input();
        let out = calculate_rebalancing(&input).unwrap();
        // |0.05| + |0.05| = 0.10, turnover = 0.10/2 = 0.05
        assert_eq!(out.total_turnover, dec!(0.05));
    }

    #[test]
    fn test_one_way_turnover() {
        let input = make_base_input();
        let out = calculate_rebalancing(&input).unwrap();
        // buy side: GOOG 0.05
        assert_eq!(out.one_way_turnover, dec!(0.05));
    }

    // --- Threshold filtering ---
    #[test]
    fn test_threshold_filters_small_changes() {
        let mut input = make_base_input();
        input.rebalance_threshold = dec!(0.06); // 6% threshold
        let out = calculate_rebalancing(&input).unwrap();
        // All changes are <= 5%, so nothing rebalanced
        assert_eq!(out.positions_rebalanced, 0);
        assert_eq!(out.trades.len(), 0);
    }

    #[test]
    fn test_zero_threshold_rebalances_all_nonzero() {
        let mut input = make_base_input();
        input.rebalance_threshold = Decimal::ZERO;
        let out = calculate_rebalancing(&input).unwrap();
        // AAPL(-5%) and GOOG(+5%) have non-zero diffs; MSFT/AMZN/META are unchanged
        assert_eq!(out.positions_rebalanced, 2);
        assert_eq!(out.positions_unchanged, 3);
    }

    // --- Transaction costs ---
    #[test]
    fn test_transaction_costs() {
        let input = make_base_input();
        let out = calculate_rebalancing(&input).unwrap();
        // turnover=0.05, cost = 0.05 * 10M * 10 / 10000 = 500
        assert_eq!(out.total_transaction_cost, dec!(500));
    }

    #[test]
    fn test_zero_cost_bps() {
        let mut input = make_base_input();
        input.transaction_cost_bps = Decimal::ZERO;
        let out = calculate_rebalancing(&input).unwrap();
        assert_eq!(out.total_transaction_cost, Decimal::ZERO);
    }

    // --- Cost drag ---
    #[test]
    fn test_cost_drag_quarterly() {
        let input = make_base_input();
        let out = calculate_rebalancing(&input).unwrap();
        // cost_drag = 500 * 4 / 10M = 0.0002 (2 bps annual)
        assert!(approx_eq(out.cost_drag, dec!(0.0002), dec!(0.00001)));
    }

    // --- Large rebalance ---
    #[test]
    fn test_large_rebalance() {
        let input = RebalancingInput {
            current_weights: vec![
                make_position("A", dec!(0.80), dec!(0.20)),
                make_position("B", dec!(0.20), dec!(0.80)),
            ],
            portfolio_value: dec!(10_000_000),
            transaction_cost_bps: dec!(10),
            rebalance_threshold: dec!(0.01),
            rebalance_frequency: "quarterly".into(),
        };
        let out = calculate_rebalancing(&input).unwrap();
        // turnover = (0.60 + 0.60)/2 = 0.60
        assert_eq!(out.total_turnover, dec!(0.60));
        assert_eq!(out.positions_rebalanced, 2);
    }

    // --- Illiquid names ---
    #[test]
    fn test_illiquid_name_high_days() {
        let mut input = make_base_input();
        // Make AAPL very illiquid
        input.current_weights[0].avg_daily_volume = dec!(100);
        let out = calculate_rebalancing(&input).unwrap();
        let aapl = out.trades.iter().find(|t| t.ticker == "AAPL").unwrap();
        // notional=500k, ADV_value=100*50=5000, days=500000/(5000*0.20)=500
        assert!(aapl.days_to_execute > dec!(100));
    }

    #[test]
    fn test_market_impact_positive() {
        let input = make_base_input();
        let out = calculate_rebalancing(&input).unwrap();
        assert!(out.market_impact_estimate > Decimal::ZERO);
    }

    // --- Optimal frequency ---
    #[test]
    fn test_optimal_frequency_low_turnover() {
        let mut input = make_base_input();
        // Make very small changes (turnover < 5%)
        input.current_weights = vec![
            make_position("A", dec!(0.51), dec!(0.50)),
            make_position("B", dec!(0.49), dec!(0.50)),
        ];
        input.rebalance_threshold = dec!(0.005);
        let out = calculate_rebalancing(&input).unwrap();
        assert_eq!(out.optimal_frequency_estimate, "annually");
    }

    #[test]
    fn test_optimal_frequency_high_turnover() {
        let input = RebalancingInput {
            current_weights: vec![
                make_position("A", dec!(0.80), dec!(0.20)),
                make_position("B", dec!(0.20), dec!(0.80)),
            ],
            portfolio_value: dec!(10_000_000),
            transaction_cost_bps: dec!(10),
            rebalance_threshold: dec!(0.01),
            rebalance_frequency: "quarterly".into(),
        };
        let out = calculate_rebalancing(&input).unwrap();
        assert_eq!(out.optimal_frequency_estimate, "monthly");
    }

    // --- Days to execute ---
    #[test]
    fn test_days_to_execute_liquid() {
        let input = make_base_input();
        let out = calculate_rebalancing(&input).unwrap();
        let aapl = out.trades.iter().find(|t| t.ticker == "AAPL").unwrap();
        // notional=500k, ADV_value=1M*50=50M, days=500000/(50M*0.20)=0.05
        assert!(aapl.days_to_execute < dec!(1));
    }

    // --- Validation ---
    #[test]
    fn test_reject_empty_positions() {
        let mut input = make_base_input();
        input.current_weights = vec![];
        assert!(calculate_rebalancing(&input).is_err());
    }

    #[test]
    fn test_reject_negative_portfolio_value() {
        let mut input = make_base_input();
        input.portfolio_value = dec!(-1);
        assert!(calculate_rebalancing(&input).is_err());
    }

    #[test]
    fn test_reject_negative_weight() {
        let mut input = make_base_input();
        input.current_weights[0].current_weight = dec!(-0.10);
        assert!(calculate_rebalancing(&input).is_err());
    }

    #[test]
    fn test_reject_negative_cost_bps() {
        let mut input = make_base_input();
        input.transaction_cost_bps = dec!(-5);
        assert!(calculate_rebalancing(&input).is_err());
    }

    #[test]
    fn test_serialization_roundtrip() {
        let input = make_base_input();
        let out = calculate_rebalancing(&input).unwrap();
        let json = serde_json::to_string(&out).unwrap();
        let _: RebalancingOutput = serde_json::from_str(&json).unwrap();
    }
}
