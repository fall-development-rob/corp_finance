use chrono::{NaiveDate, NaiveTime};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::time::Instant;

use crate::{types::*, CorpFinanceError, CorpFinanceResult};

// ---------------------------------------------------------------------------
// Input / Output types
// ---------------------------------------------------------------------------

/// A single trade entry recorded during the trading day.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeEntry {
    pub trade_number: u32,
    /// Contract symbol, e.g. "ES", "NQ", "CL"
    pub contract: String,
    pub time_open: NaiveTime,
    /// None if trade is still open
    pub time_close: Option<NaiveTime>,
    pub open_price: Money,
    pub profit_target: Option<Money>,
    pub stop_loss: Option<Money>,
    /// None if trade is still open
    pub actual_exit: Option<Money>,
    /// Explicit P&L override; computed from prices if None
    pub profit_loss: Option<Money>,
    pub comment: Option<String>,
}

/// Input representing a full trading day diary page.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradingDayInput {
    pub date: NaiveDate,
    /// Trader self-assessed confidence, 0-10
    pub confidence_level: u8,
    pub support_levels: Vec<Money>,
    pub resistance_levels: Vec<Money>,
    pub trades: Vec<TradeEntry>,
    pub cancelled_trades: Vec<CancelledTrade>,
    pub trader_comments: Option<String>,
    pub currency: Option<Currency>,
}

/// A trade that was planned but not executed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelledTrade {
    pub description: String,
    pub reason: String,
}

/// Computed output summarising the trading day.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradingDayOutput {
    pub date: NaiveDate,
    pub confidence_level: u8,
    pub num_winning_trades: u32,
    pub num_losing_trades: u32,
    pub total_profit: Money,
    pub total_losses: Money,
    pub total_daily_pnl: Money,
    /// Best open trade equity P&L (maximum single-trade P&L)
    pub best_ote_pnl: Money,
    /// Largest peak-to-trough drawdown in running P&L
    pub max_drawdown: Money,
    pub total_round_trips: u32,
    /// average_win / average_loss; None if no losing trades
    pub risk_reward_ratio: Option<Rate>,
    /// num_winning / total_completed
    pub win_rate: Rate,
    pub average_win: Money,
    pub average_loss: Money,
    pub largest_win: Money,
    pub largest_loss: Money,
    /// total_profit / total_losses; None if total_losses == 0
    pub profit_factor: Option<Rate>,
    pub trade_details: Vec<TradeDetail>,
    pub cancelled_trade_count: u32,
}

/// Per-trade detail in the output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeDetail {
    pub trade_number: u32,
    pub contract: String,
    pub pnl: Money,
    pub is_winner: bool,
    /// pnl / |open_price - stop_loss|; None if stop_loss not set
    pub return_on_risk: Option<Rate>,
    /// Minutes the trade was held; None if still open
    pub held_duration_minutes: Option<i64>,
    /// Whether actual_exit >= profit_target (long assumption)
    pub hit_target: bool,
    /// Whether actual_exit <= stop_loss (long assumption)
    pub hit_stop: bool,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Analyse a full trading day diary, computing summary statistics.
pub fn analyze_trading_day(
    input: &TradingDayInput,
) -> CorpFinanceResult<ComputationOutput<TradingDayOutput>> {
    let start = Instant::now();
    let warnings: Vec<String> = Vec::new();

    // -- Validation ----------------------------------------------------------
    validate_input(input)?;

    // -- Compute per-trade P&L -----------------------------------------------
    let mut trade_details: Vec<TradeDetail> = Vec::new();
    let mut pnls: Vec<Money> = Vec::new();

    for trade in &input.trades {
        let pnl = resolve_pnl(trade);
        match pnl {
            Some(p) => {
                let return_on_risk = compute_return_on_risk(trade, p);
                let held_duration = compute_duration(trade);
                let hit_target = match (trade.actual_exit, trade.profit_target) {
                    (Some(exit), Some(target)) => exit >= target,
                    _ => false,
                };
                let hit_stop = match (trade.actual_exit, trade.stop_loss) {
                    (Some(exit), Some(sl)) => exit <= sl,
                    _ => false,
                };

                trade_details.push(TradeDetail {
                    trade_number: trade.trade_number,
                    contract: trade.contract.clone(),
                    pnl: p,
                    is_winner: p > Decimal::ZERO,
                    return_on_risk,
                    held_duration_minutes: held_duration,
                    hit_target,
                    hit_stop,
                });
                pnls.push(p);
            }
            None => {
                // Trade still open — include in details with zero P&L
                trade_details.push(TradeDetail {
                    trade_number: trade.trade_number,
                    contract: trade.contract.clone(),
                    pnl: Decimal::ZERO,
                    is_winner: false,
                    return_on_risk: None,
                    held_duration_minutes: compute_duration(trade),
                    hit_target: false,
                    hit_stop: false,
                });
            }
        }
    }

    // -- Separate winners and losers -----------------------------------------
    let winners: Vec<Money> = pnls
        .iter()
        .copied()
        .filter(|&p| p > Decimal::ZERO)
        .collect();
    let losers: Vec<Money> = pnls
        .iter()
        .copied()
        .filter(|&p| p < Decimal::ZERO)
        .collect();
    let breakeven_count = pnls.iter().filter(|&&p| p == Decimal::ZERO).count() as u32;

    let num_winning = winners.len() as u32;
    let num_losing = losers.len() as u32;
    let total_completed = pnls.len() as u32;

    let total_profit: Money = winners.iter().copied().sum();
    // total_losses is stored as a positive number (absolute value of sum of losers)
    let total_losses: Money = losers.iter().copied().sum::<Decimal>().abs();
    let total_daily_pnl: Money = total_profit - total_losses;

    // -- Averages ------------------------------------------------------------
    let average_win = if num_winning > 0 {
        total_profit / Decimal::from(num_winning)
    } else {
        Decimal::ZERO
    };
    let average_loss = if num_losing > 0 {
        total_losses / Decimal::from(num_losing)
    } else {
        Decimal::ZERO
    };

    // -- Largest win/loss ----------------------------------------------------
    let largest_win = winners.iter().copied().max().unwrap_or(Decimal::ZERO);
    let largest_loss = losers
        .iter()
        .copied()
        .map(|d| d.abs())
        .max()
        .unwrap_or(Decimal::ZERO);

    // -- Risk-reward ratio ---------------------------------------------------
    let risk_reward_ratio = if num_losing > 0 && average_loss > Decimal::ZERO {
        Some(average_win / average_loss)
    } else {
        None
    };

    // -- Win rate ------------------------------------------------------------
    let win_rate = if total_completed > 0 {
        Decimal::from(num_winning) / Decimal::from(total_completed)
    } else {
        Decimal::ZERO
    };

    // -- Best OTE (max single-trade P&L) -------------------------------------
    let best_ote_pnl = pnls.iter().copied().max().unwrap_or(Decimal::ZERO);

    // -- Max drawdown (peak-to-trough in running P&L) ------------------------
    let max_drawdown = compute_max_drawdown(&pnls);

    // -- Profit factor -------------------------------------------------------
    let profit_factor = if total_losses > Decimal::ZERO {
        Some(total_profit / total_losses)
    } else {
        None
    };

    // -- Round trips (completed trades) --------------------------------------
    let total_round_trips = num_winning + num_losing + breakeven_count;

    // -- Build output --------------------------------------------------------
    let output = TradingDayOutput {
        date: input.date,
        confidence_level: input.confidence_level,
        num_winning_trades: num_winning,
        num_losing_trades: num_losing,
        total_profit,
        total_losses,
        total_daily_pnl,
        best_ote_pnl,
        max_drawdown,
        total_round_trips,
        risk_reward_ratio,
        win_rate,
        average_win,
        average_loss,
        largest_win,
        largest_loss,
        profit_factor,
        trade_details,
        cancelled_trade_count: input.cancelled_trades.len() as u32,
    };

    let elapsed = start.elapsed().as_micros() as u64;
    let assumptions = serde_json::json!({
        "methodology": "Trading diary day analysis",
        "pnl_direction": "long-biased (exit - open)",
        "drawdown": "peak-to-trough on cumulative running P&L",
        "risk_reward": "average_win / average_loss",
    });

    Ok(with_metadata(
        "Trading Day Diary Analysis",
        &assumptions,
        warnings,
        elapsed,
        output,
    ))
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn validate_input(input: &TradingDayInput) -> CorpFinanceResult<()> {
    if input.confidence_level > 10 {
        return Err(CorpFinanceError::InvalidInput {
            field: "confidence_level".into(),
            reason: "Confidence level must be between 0 and 10.".into(),
        });
    }
    if input.trades.is_empty() {
        return Err(CorpFinanceError::InvalidInput {
            field: "trades".into(),
            reason: "At least one trade is required.".into(),
        });
    }
    // Check trade_number uniqueness
    let mut seen = HashSet::new();
    for trade in &input.trades {
        if !seen.insert(trade.trade_number) {
            return Err(CorpFinanceError::InvalidInput {
                field: "trade_number".into(),
                reason: format!(
                    "Duplicate trade_number: {}. Trade numbers must be unique.",
                    trade.trade_number
                ),
            });
        }
    }
    Ok(())
}

/// Resolve the P&L for a trade.
/// Uses explicit profit_loss if provided, otherwise computes exit - open.
/// Returns None if the trade has no exit (still open).
fn resolve_pnl(trade: &TradeEntry) -> Option<Money> {
    if let Some(pl) = trade.profit_loss {
        return Some(pl);
    }
    trade.actual_exit.map(|exit| exit - trade.open_price)
}

/// Compute return on risk: pnl / |open_price - stop_loss|
fn compute_return_on_risk(trade: &TradeEntry, pnl: Money) -> Option<Rate> {
    trade.stop_loss.and_then(|sl| {
        let risk = (trade.open_price - sl).abs();
        if risk.is_zero() {
            None
        } else {
            Some(pnl / risk)
        }
    })
}

/// Compute held duration in minutes between open and close times.
fn compute_duration(trade: &TradeEntry) -> Option<i64> {
    trade.time_close.map(|close| {
        let duration = close.signed_duration_since(trade.time_open);
        duration.num_minutes()
    })
}

/// Compute maximum drawdown as the largest peak-to-trough decline
/// in the cumulative running P&L series.
fn compute_max_drawdown(pnls: &[Money]) -> Money {
    if pnls.is_empty() {
        return Decimal::ZERO;
    }
    let mut cumulative = Decimal::ZERO;
    let mut peak = Decimal::ZERO;
    let mut max_dd = Decimal::ZERO;

    for &pnl in pnls {
        cumulative += pnl;
        if cumulative > peak {
            peak = cumulative;
        }
        let drawdown = peak - cumulative;
        if drawdown > max_dd {
            max_dd = drawdown;
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
    use chrono::{NaiveDate, NaiveTime};
    use rust_decimal_macros::dec;

    fn make_time(h: u32, m: u32) -> NaiveTime {
        NaiveTime::from_hms_opt(h, m, 0).unwrap()
    }

    fn make_date() -> NaiveDate {
        NaiveDate::from_ymd_opt(2025, 6, 15).unwrap()
    }

    fn winning_trade(num: u32, pnl: Decimal) -> TradeEntry {
        TradeEntry {
            trade_number: num,
            contract: "ES".to_string(),
            time_open: make_time(9, 30),
            time_close: Some(make_time(10, 0)),
            open_price: dec!(4500),
            profit_target: Some(dec!(4500) + pnl),
            stop_loss: Some(dec!(4490)),
            actual_exit: Some(dec!(4500) + pnl),
            profit_loss: None,
            comment: None,
        }
    }

    fn losing_trade(num: u32, loss: Decimal) -> TradeEntry {
        // loss is a positive amount representing the size of the loss
        TradeEntry {
            trade_number: num,
            contract: "NQ".to_string(),
            time_open: make_time(10, 0),
            time_close: Some(make_time(10, 30)),
            open_price: dec!(15000),
            profit_target: Some(dec!(15050)),
            stop_loss: Some(dec!(15000) - loss),
            actual_exit: Some(dec!(15000) - loss),
            profit_loss: None,
            comment: None,
        }
    }

    // -----------------------------------------------------------------------
    // Test 1: Basic day with 3 winners, 2 losers
    // -----------------------------------------------------------------------
    #[test]
    fn test_basic_day_mixed() {
        let input = TradingDayInput {
            date: make_date(),
            confidence_level: 7,
            support_levels: vec![dec!(4480)],
            resistance_levels: vec![dec!(4520)],
            trades: vec![
                winning_trade(1, dec!(20)), // +20
                winning_trade(2, dec!(15)), // +15
                losing_trade(3, dec!(10)),  // -10
                winning_trade(4, dec!(25)), // +25
                losing_trade(5, dec!(12)),  // -12
            ],
            cancelled_trades: vec![],
            trader_comments: Some("Solid day".into()),
            currency: None,
        };

        let result = analyze_trading_day(&input).unwrap();
        let out = &result.result;

        assert_eq!(out.num_winning_trades, 3);
        assert_eq!(out.num_losing_trades, 2);
        assert_eq!(out.total_profit, dec!(60)); // 20 + 15 + 25
        assert_eq!(out.total_losses, dec!(22)); // 10 + 12
        assert_eq!(out.total_daily_pnl, dec!(38)); // 60 - 22
        assert_eq!(out.total_round_trips, 5);
    }

    // -----------------------------------------------------------------------
    // Test 2: All winning trades
    // -----------------------------------------------------------------------
    #[test]
    fn test_all_winners() {
        let input = TradingDayInput {
            date: make_date(),
            confidence_level: 9,
            support_levels: vec![],
            resistance_levels: vec![],
            trades: vec![
                winning_trade(1, dec!(10)),
                winning_trade(2, dec!(20)),
                winning_trade(3, dec!(30)),
            ],
            cancelled_trades: vec![],
            trader_comments: None,
            currency: None,
        };

        let result = analyze_trading_day(&input).unwrap();
        let out = &result.result;

        assert_eq!(out.num_winning_trades, 3);
        assert_eq!(out.num_losing_trades, 0);
        assert_eq!(out.total_profit, dec!(60));
        assert_eq!(out.total_losses, Decimal::ZERO);
        assert_eq!(out.total_daily_pnl, dec!(60));
        assert!(out.risk_reward_ratio.is_none());
        assert!(out.profit_factor.is_none());
        assert_eq!(out.win_rate, Decimal::ONE);
    }

    // -----------------------------------------------------------------------
    // Test 3: All losing trades
    // -----------------------------------------------------------------------
    #[test]
    fn test_all_losers() {
        let input = TradingDayInput {
            date: make_date(),
            confidence_level: 3,
            support_levels: vec![],
            resistance_levels: vec![],
            trades: vec![losing_trade(1, dec!(15)), losing_trade(2, dec!(25))],
            cancelled_trades: vec![],
            trader_comments: None,
            currency: None,
        };

        let result = analyze_trading_day(&input).unwrap();
        let out = &result.result;

        assert_eq!(out.num_winning_trades, 0);
        assert_eq!(out.num_losing_trades, 2);
        assert_eq!(out.total_profit, Decimal::ZERO);
        assert_eq!(out.total_losses, dec!(40)); // 15 + 25
        assert_eq!(out.total_daily_pnl, dec!(-40));
        assert_eq!(out.win_rate, Decimal::ZERO);
        assert_eq!(out.average_win, Decimal::ZERO);
        assert_eq!(out.largest_win, Decimal::ZERO);
    }

    // -----------------------------------------------------------------------
    // Test 4: Single trade
    // -----------------------------------------------------------------------
    #[test]
    fn test_single_trade() {
        let input = TradingDayInput {
            date: make_date(),
            confidence_level: 5,
            support_levels: vec![],
            resistance_levels: vec![],
            trades: vec![winning_trade(1, dec!(50))],
            cancelled_trades: vec![],
            trader_comments: None,
            currency: None,
        };

        let result = analyze_trading_day(&input).unwrap();
        let out = &result.result;

        assert_eq!(out.num_winning_trades, 1);
        assert_eq!(out.total_round_trips, 1);
        assert_eq!(out.win_rate, Decimal::ONE);
        assert_eq!(out.total_daily_pnl, dec!(50));
        assert_eq!(out.best_ote_pnl, dec!(50));
    }

    // -----------------------------------------------------------------------
    // Test 5: Risk-reward ratio calculation
    // -----------------------------------------------------------------------
    #[test]
    fn test_risk_reward_ratio() {
        // 2 winners at +20 each (avg_win = 20), 1 loser at -10 (avg_loss = 10)
        // risk_reward = 20 / 10 = 2.0
        let input = TradingDayInput {
            date: make_date(),
            confidence_level: 6,
            support_levels: vec![],
            resistance_levels: vec![],
            trades: vec![
                winning_trade(1, dec!(20)),
                winning_trade(2, dec!(20)),
                losing_trade(3, dec!(10)),
            ],
            cancelled_trades: vec![],
            trader_comments: None,
            currency: None,
        };

        let result = analyze_trading_day(&input).unwrap();
        let out = &result.result;

        assert_eq!(out.average_win, dec!(20));
        assert_eq!(out.average_loss, dec!(10));
        assert_eq!(out.risk_reward_ratio, Some(dec!(2)));
    }

    // -----------------------------------------------------------------------
    // Test 6: Win rate calculation
    // -----------------------------------------------------------------------
    #[test]
    fn test_win_rate() {
        // 1 winner, 3 losers => win_rate = 0.25
        let input = TradingDayInput {
            date: make_date(),
            confidence_level: 4,
            support_levels: vec![],
            resistance_levels: vec![],
            trades: vec![
                winning_trade(1, dec!(30)),
                losing_trade(2, dec!(10)),
                losing_trade(3, dec!(10)),
                losing_trade(4, dec!(10)),
            ],
            cancelled_trades: vec![],
            trader_comments: None,
            currency: None,
        };

        let result = analyze_trading_day(&input).unwrap();
        let out = &result.result;

        assert_eq!(out.win_rate, dec!(0.25));
        assert_eq!(out.num_winning_trades, 1);
        assert_eq!(out.num_losing_trades, 3);
    }

    // -----------------------------------------------------------------------
    // Test 7: Max drawdown calculation
    // -----------------------------------------------------------------------
    #[test]
    fn test_max_drawdown() {
        // P&L sequence: +20, -30, +10, -5
        // Cumulative:   20, -10, 0, -5
        // Peak track:   20,  20, 20, 20
        // Drawdown:      0,  30, 20, 25
        // Max drawdown = 30
        let input = TradingDayInput {
            date: make_date(),
            confidence_level: 5,
            support_levels: vec![],
            resistance_levels: vec![],
            trades: vec![
                TradeEntry {
                    trade_number: 1,
                    contract: "ES".into(),
                    time_open: make_time(9, 30),
                    time_close: Some(make_time(9, 45)),
                    open_price: dec!(4500),
                    profit_target: None,
                    stop_loss: None,
                    actual_exit: Some(dec!(4520)),
                    profit_loss: None,
                    comment: None,
                },
                TradeEntry {
                    trade_number: 2,
                    contract: "ES".into(),
                    time_open: make_time(10, 0),
                    time_close: Some(make_time(10, 15)),
                    open_price: dec!(4520),
                    profit_target: None,
                    stop_loss: None,
                    actual_exit: Some(dec!(4490)),
                    profit_loss: None,
                    comment: None,
                },
                TradeEntry {
                    trade_number: 3,
                    contract: "ES".into(),
                    time_open: make_time(10, 30),
                    time_close: Some(make_time(10, 45)),
                    open_price: dec!(4490),
                    profit_target: None,
                    stop_loss: None,
                    actual_exit: Some(dec!(4500)),
                    profit_loss: None,
                    comment: None,
                },
                TradeEntry {
                    trade_number: 4,
                    contract: "ES".into(),
                    time_open: make_time(11, 0),
                    time_close: Some(make_time(11, 15)),
                    open_price: dec!(4500),
                    profit_target: None,
                    stop_loss: None,
                    actual_exit: Some(dec!(4495)),
                    profit_loss: None,
                    comment: None,
                },
            ],
            cancelled_trades: vec![],
            trader_comments: None,
            currency: None,
        };

        let result = analyze_trading_day(&input).unwrap();
        let out = &result.result;

        assert_eq!(out.max_drawdown, dec!(30));
    }

    // -----------------------------------------------------------------------
    // Test 8: Profit factor calculation
    // -----------------------------------------------------------------------
    #[test]
    fn test_profit_factor() {
        // Winners: +30, +20 = 50 total profit
        // Losers: -10, -15 = 25 total losses
        // Profit factor = 50 / 25 = 2.0
        let input = TradingDayInput {
            date: make_date(),
            confidence_level: 7,
            support_levels: vec![],
            resistance_levels: vec![],
            trades: vec![
                winning_trade(1, dec!(30)),
                winning_trade(2, dec!(20)),
                losing_trade(3, dec!(10)),
                losing_trade(4, dec!(15)),
            ],
            cancelled_trades: vec![],
            trader_comments: None,
            currency: None,
        };

        let result = analyze_trading_day(&input).unwrap();
        let out = &result.result;

        assert_eq!(out.total_profit, dec!(50));
        assert_eq!(out.total_losses, dec!(25));
        assert_eq!(out.profit_factor, Some(dec!(2)));
    }

    // -----------------------------------------------------------------------
    // Test 9: Trade with no stop loss (return_on_risk = None)
    // -----------------------------------------------------------------------
    #[test]
    fn test_no_stop_loss_return_on_risk() {
        let input = TradingDayInput {
            date: make_date(),
            confidence_level: 5,
            support_levels: vec![],
            resistance_levels: vec![],
            trades: vec![TradeEntry {
                trade_number: 1,
                contract: "CL".into(),
                time_open: make_time(9, 0),
                time_close: Some(make_time(9, 30)),
                open_price: dec!(75),
                profit_target: Some(dec!(76)),
                stop_loss: None,
                actual_exit: Some(dec!(76)),
                profit_loss: None,
                comment: None,
            }],
            cancelled_trades: vec![],
            trader_comments: None,
            currency: None,
        };

        let result = analyze_trading_day(&input).unwrap();
        let detail = &result.result.trade_details[0];
        assert!(detail.return_on_risk.is_none());
        assert_eq!(detail.pnl, dec!(1));
        assert!(detail.is_winner);
    }

    // -----------------------------------------------------------------------
    // Test 10: Confidence validation (>10 = error)
    // -----------------------------------------------------------------------
    #[test]
    fn test_confidence_validation() {
        let input = TradingDayInput {
            date: make_date(),
            confidence_level: 11,
            support_levels: vec![],
            resistance_levels: vec![],
            trades: vec![winning_trade(1, dec!(10))],
            cancelled_trades: vec![],
            trader_comments: None,
            currency: None,
        };

        let err = analyze_trading_day(&input).unwrap_err();
        match err {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "confidence_level");
            }
            other => panic!("Expected InvalidInput for confidence_level, got {other:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // Test 11: Empty trades error
    // -----------------------------------------------------------------------
    #[test]
    fn test_empty_trades_error() {
        let input = TradingDayInput {
            date: make_date(),
            confidence_level: 5,
            support_levels: vec![],
            resistance_levels: vec![],
            trades: vec![],
            cancelled_trades: vec![],
            trader_comments: None,
            currency: None,
        };

        let err = analyze_trading_day(&input).unwrap_err();
        match err {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "trades");
            }
            other => panic!("Expected InvalidInput for trades, got {other:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // Test 12: Metadata populated
    // -----------------------------------------------------------------------
    #[test]
    fn test_metadata_populated() {
        let input = TradingDayInput {
            date: make_date(),
            confidence_level: 5,
            support_levels: vec![],
            resistance_levels: vec![],
            trades: vec![winning_trade(1, dec!(10))],
            cancelled_trades: vec![],
            trader_comments: None,
            currency: None,
        };

        let result = analyze_trading_day(&input).unwrap();
        assert!(!result.methodology.is_empty());
        assert!(result.methodology.contains("Trading"));
        assert_eq!(result.metadata.precision, "rust_decimal_128bit");
        assert!(result.metadata.computation_time_us < 1_000_000);
    }

    // -----------------------------------------------------------------------
    // Test 13: Duplicate trade number error
    // -----------------------------------------------------------------------
    #[test]
    fn test_duplicate_trade_number() {
        let input = TradingDayInput {
            date: make_date(),
            confidence_level: 5,
            support_levels: vec![],
            resistance_levels: vec![],
            trades: vec![
                winning_trade(1, dec!(10)),
                winning_trade(1, dec!(20)), // duplicate
            ],
            cancelled_trades: vec![],
            trader_comments: None,
            currency: None,
        };

        let err = analyze_trading_day(&input).unwrap_err();
        match err {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "trade_number");
            }
            other => panic!("Expected InvalidInput for trade_number, got {other:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // Test 14: Return on risk with stop loss
    // -----------------------------------------------------------------------
    #[test]
    fn test_return_on_risk_with_stop() {
        // open = 4500, stop = 4490, risk = 10
        // exit = 4520, pnl = 20
        // return_on_risk = 20 / 10 = 2.0
        let input = TradingDayInput {
            date: make_date(),
            confidence_level: 7,
            support_levels: vec![],
            resistance_levels: vec![],
            trades: vec![TradeEntry {
                trade_number: 1,
                contract: "ES".into(),
                time_open: make_time(9, 30),
                time_close: Some(make_time(10, 0)),
                open_price: dec!(4500),
                profit_target: Some(dec!(4520)),
                stop_loss: Some(dec!(4490)),
                actual_exit: Some(dec!(4520)),
                profit_loss: None,
                comment: None,
            }],
            cancelled_trades: vec![],
            trader_comments: None,
            currency: None,
        };

        let result = analyze_trading_day(&input).unwrap();
        let detail = &result.result.trade_details[0];
        assert_eq!(detail.return_on_risk, Some(dec!(2)));
        assert!(detail.hit_target);
        assert!(!detail.hit_stop);
    }

    // -----------------------------------------------------------------------
    // Test 15: Held duration in minutes
    // -----------------------------------------------------------------------
    #[test]
    fn test_held_duration() {
        let input = TradingDayInput {
            date: make_date(),
            confidence_level: 5,
            support_levels: vec![],
            resistance_levels: vec![],
            trades: vec![TradeEntry {
                trade_number: 1,
                contract: "ES".into(),
                time_open: make_time(9, 30),
                time_close: Some(make_time(10, 15)),
                open_price: dec!(4500),
                profit_target: None,
                stop_loss: None,
                actual_exit: Some(dec!(4510)),
                profit_loss: None,
                comment: None,
            }],
            cancelled_trades: vec![],
            trader_comments: None,
            currency: None,
        };

        let result = analyze_trading_day(&input).unwrap();
        let detail = &result.result.trade_details[0];
        assert_eq!(detail.held_duration_minutes, Some(45));
    }

    // -----------------------------------------------------------------------
    // Test 16: Cancelled trades counted
    // -----------------------------------------------------------------------
    #[test]
    fn test_cancelled_trades_counted() {
        let input = TradingDayInput {
            date: make_date(),
            confidence_level: 5,
            support_levels: vec![],
            resistance_levels: vec![],
            trades: vec![winning_trade(1, dec!(10))],
            cancelled_trades: vec![
                CancelledTrade {
                    description: "Long ES at open".into(),
                    reason: "News risk".into(),
                },
                CancelledTrade {
                    description: "Short NQ at VWAP".into(),
                    reason: "Missed entry".into(),
                },
            ],
            trader_comments: None,
            currency: None,
        };

        let result = analyze_trading_day(&input).unwrap();
        assert_eq!(result.result.cancelled_trade_count, 2);
    }

    // -----------------------------------------------------------------------
    // Test 17: Explicit profit_loss override
    // -----------------------------------------------------------------------
    #[test]
    fn test_explicit_pnl_override() {
        // open=4500, exit=4510 would give pnl=10 from prices,
        // but explicit profit_loss=25 should override
        let input = TradingDayInput {
            date: make_date(),
            confidence_level: 5,
            support_levels: vec![],
            resistance_levels: vec![],
            trades: vec![TradeEntry {
                trade_number: 1,
                contract: "ES".into(),
                time_open: make_time(9, 30),
                time_close: Some(make_time(10, 0)),
                open_price: dec!(4500),
                profit_target: None,
                stop_loss: None,
                actual_exit: Some(dec!(4510)),
                profit_loss: Some(dec!(25)),
                comment: None,
            }],
            cancelled_trades: vec![],
            trader_comments: None,
            currency: None,
        };

        let result = analyze_trading_day(&input).unwrap();
        let detail = &result.result.trade_details[0];
        assert_eq!(detail.pnl, dec!(25));
    }
}
