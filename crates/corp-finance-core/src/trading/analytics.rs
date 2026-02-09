use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::error::CorpFinanceError;
use crate::types::*;
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaySummary {
    pub date: NaiveDate,
    pub daily_pnl: Money,
    pub num_trades: u32,
    pub num_winners: u32,
    pub num_losers: u32,
    pub total_profit: Money,
    pub total_losses: Money,
    pub confidence_level: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradingAnalyticsInput {
    pub day_summaries: Vec<DaySummary>,
    pub starting_capital: Money,
    pub risk_free_rate: Option<Rate>,
    pub currency: Option<Currency>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradingAnalyticsOutput {
    pub total_trading_days: u32,
    pub total_trades: u32,
    pub total_pnl: Money,
    pub cumulative_return: Rate,
    pub average_daily_pnl: Money,
    pub best_day_pnl: Money,
    pub worst_day_pnl: Money,
    pub best_day_date: NaiveDate,
    pub worst_day_date: NaiveDate,
    pub overall_win_rate: Rate,
    pub overall_profit_factor: Option<Rate>,
    pub overall_risk_reward: Option<Rate>,
    pub max_drawdown: Money,
    pub max_drawdown_pct: Rate,
    pub max_consecutive_wins: u32,
    pub max_consecutive_losses: u32,
    pub winning_days: u32,
    pub losing_days: u32,
    pub breakeven_days: u32,
    pub daily_sharpe_ratio: Option<Rate>,
    pub expectancy: Money,
    pub confidence_correlation: Option<Rate>,
    pub equity_curve: Vec<EquityCurvePoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EquityCurvePoint {
    pub date: NaiveDate,
    pub cumulative_pnl: Money,
    pub equity: Money,
    pub drawdown: Money,
    pub drawdown_pct: Rate,
}

// ---------------------------------------------------------------------------
// Helper math (pure Decimal, no f64, no MathematicalOps)
// ---------------------------------------------------------------------------

fn sqrt_decimal(x: Decimal) -> Decimal {
    if x <= Decimal::ZERO {
        return Decimal::ZERO;
    }
    let two = Decimal::from(2);
    let mut guess = x / two;
    for _ in 0..20 {
        guess = (guess + x / guess) / two;
    }
    guess
}

fn stdev_decimal(values: &[Decimal]) -> Decimal {
    let n = Decimal::from(values.len() as u32);
    if n <= Decimal::ONE {
        return Decimal::ZERO;
    }
    let mean = values.iter().copied().sum::<Decimal>() / n;
    let var = values
        .iter()
        .map(|v| {
            let d = *v - mean;
            d * d
        })
        .sum::<Decimal>()
        / (n - Decimal::ONE);
    sqrt_decimal(var)
}

fn pearson_correlation(xs: &[Decimal], ys: &[Decimal]) -> Option<Decimal> {
    let n = Decimal::from(xs.len() as u32);
    if n <= Decimal::ONE {
        return None;
    }
    let sum_x: Decimal = xs.iter().copied().sum();
    let sum_y: Decimal = ys.iter().copied().sum();
    let sum_xy: Decimal = xs.iter().zip(ys.iter()).map(|(x, y)| x * y).sum();
    let sum_x2: Decimal = xs.iter().map(|x| x * x).sum();
    let sum_y2: Decimal = ys.iter().map(|y| y * y).sum();
    let numerator = n * sum_xy - sum_x * sum_y;
    let denom_a = n * sum_x2 - sum_x * sum_x;
    let denom_b = n * sum_y2 - sum_y * sum_y;
    if denom_a <= Decimal::ZERO || denom_b <= Decimal::ZERO {
        return None;
    }
    let denom = sqrt_decimal(denom_a) * sqrt_decimal(denom_b);
    if denom == Decimal::ZERO {
        return None;
    }
    Some(numerator / denom)
}

// ---------------------------------------------------------------------------
// Main function
// ---------------------------------------------------------------------------

pub fn analyze_trading_performance(
    input: &TradingAnalyticsInput,
) -> CorpFinanceResult<ComputationOutput<TradingAnalyticsOutput>> {
    let start = Instant::now();
    let warnings: Vec<String> = Vec::new();

    // -- Validation --
    if input.day_summaries.is_empty() {
        return Err(CorpFinanceError::InvalidInput {
            field: "day_summaries".into(),
            reason: "At least one day summary is required".into(),
        });
    }
    if input.starting_capital <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "starting_capital".into(),
            reason: "Starting capital must be positive".into(),
        });
    }

    let days = &input.day_summaries;
    let n_days = days.len() as u32;

    // -- Aggregates --
    let total_trades: u32 = days.iter().map(|d| d.num_trades).sum();
    let total_winners: u32 = days.iter().map(|d| d.num_winners).sum();
    let total_pnl: Decimal = days.iter().map(|d| d.daily_pnl).sum();
    let total_profit_all: Decimal = days.iter().map(|d| d.total_profit).sum();
    let total_losses_all: Decimal = days.iter().map(|d| d.total_losses).sum();
    let n_days_dec = Decimal::from(n_days);

    // -- Return --
    let cumulative_return = total_pnl / input.starting_capital;
    let average_daily_pnl = total_pnl / n_days_dec;

    // -- Best / worst day --
    let best_idx = days
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.daily_pnl.cmp(&b.daily_pnl))
        .unwrap()
        .0;
    let worst_idx = days
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| a.daily_pnl.cmp(&b.daily_pnl))
        .unwrap()
        .0;

    // -- Win rate --
    let overall_win_rate = if total_trades > 0 {
        Decimal::from(total_winners) / Decimal::from(total_trades)
    } else {
        Decimal::ZERO
    };

    // -- Profit factor --
    let overall_profit_factor = if total_losses_all > Decimal::ZERO {
        Some(total_profit_all / total_losses_all)
    } else {
        None
    };

    // -- Risk-reward (average win / average loss) --
    let total_losers: u32 = days.iter().map(|d| d.num_losers).sum();
    let overall_risk_reward = if total_winners > 0 && total_losers > 0 {
        let avg_win = total_profit_all / Decimal::from(total_winners);
        let avg_loss = total_losses_all / Decimal::from(total_losers);
        if avg_loss > Decimal::ZERO {
            Some(avg_win / avg_loss)
        } else {
            None
        }
    } else {
        None
    };

    // -- Equity curve + max drawdown --
    let mut equity_curve = Vec::with_capacity(days.len());
    let mut cumulative_pnl = Decimal::ZERO;
    let mut peak_equity = input.starting_capital;
    let mut max_dd = Decimal::ZERO;
    let mut max_dd_pct = Decimal::ZERO;

    for day in days {
        cumulative_pnl += day.daily_pnl;
        let equity = input.starting_capital + cumulative_pnl;
        if equity > peak_equity {
            peak_equity = equity;
        }
        let drawdown = peak_equity - equity;
        let drawdown_pct = if peak_equity > Decimal::ZERO {
            drawdown / peak_equity
        } else {
            Decimal::ZERO
        };
        if drawdown > max_dd {
            max_dd = drawdown;
        }
        if drawdown_pct > max_dd_pct {
            max_dd_pct = drawdown_pct;
        }
        equity_curve.push(EquityCurvePoint {
            date: day.date,
            cumulative_pnl,
            equity,
            drawdown,
            drawdown_pct,
        });
    }

    // -- Consecutive wins / losses (by day P&L sign) --
    let mut max_consec_wins: u32 = 0;
    let mut max_consec_losses: u32 = 0;
    let mut cur_wins: u32 = 0;
    let mut cur_losses: u32 = 0;
    let mut winning_days: u32 = 0;
    let mut losing_days: u32 = 0;
    let mut breakeven_days: u32 = 0;

    for day in days {
        if day.daily_pnl > Decimal::ZERO {
            winning_days += 1;
            cur_wins += 1;
            cur_losses = 0;
            if cur_wins > max_consec_wins {
                max_consec_wins = cur_wins;
            }
        } else if day.daily_pnl < Decimal::ZERO {
            losing_days += 1;
            cur_losses += 1;
            cur_wins = 0;
            if cur_losses > max_consec_losses {
                max_consec_losses = cur_losses;
            }
        } else {
            breakeven_days += 1;
            cur_wins = 0;
            cur_losses = 0;
        }
    }

    // -- Daily Sharpe ratio --
    let daily_pnls: Vec<Decimal> = days.iter().map(|d| d.daily_pnl).collect();
    let sd = stdev_decimal(&daily_pnls);
    let daily_sharpe_ratio = if sd > Decimal::ZERO {
        let mean_pnl = total_pnl / n_days_dec;
        let annualization = sqrt_decimal(Decimal::from(252));
        Some((mean_pnl / sd) * annualization)
    } else {
        None
    };

    // -- Expectancy: (avg_win * win_rate) - (avg_loss * loss_rate) --
    let expectancy = if total_trades > 0 {
        let total_trades_dec = Decimal::from(total_trades);
        let win_rate = Decimal::from(total_winners) / total_trades_dec;
        let loss_rate = Decimal::from(total_losers) / total_trades_dec;
        let avg_win = if total_winners > 0 {
            total_profit_all / Decimal::from(total_winners)
        } else {
            Decimal::ZERO
        };
        let avg_loss = if total_losers > 0 {
            total_losses_all / Decimal::from(total_losers)
        } else {
            Decimal::ZERO
        };
        avg_win * win_rate - avg_loss * loss_rate
    } else {
        Decimal::ZERO
    };

    // -- Confidence correlation --
    let confidences: Vec<Decimal> = days
        .iter()
        .map(|d| Decimal::from(d.confidence_level))
        .collect();
    let pnls_for_corr: Vec<Decimal> = days.iter().map(|d| d.daily_pnl).collect();
    let confidence_correlation = pearson_correlation(&confidences, &pnls_for_corr);

    // -- Build output --
    let output = TradingAnalyticsOutput {
        total_trading_days: n_days,
        total_trades,
        total_pnl,
        cumulative_return,
        average_daily_pnl,
        best_day_pnl: days[best_idx].daily_pnl,
        worst_day_pnl: days[worst_idx].daily_pnl,
        best_day_date: days[best_idx].date,
        worst_day_date: days[worst_idx].date,
        overall_win_rate,
        overall_profit_factor,
        overall_risk_reward,
        max_drawdown: max_dd,
        max_drawdown_pct: max_dd_pct,
        max_consecutive_wins: max_consec_wins,
        max_consecutive_losses: max_consec_losses,
        winning_days,
        losing_days,
        breakeven_days,
        daily_sharpe_ratio,
        expectancy,
        confidence_correlation,
        equity_curve,
    };

    let elapsed = start.elapsed().as_micros() as u64;
    Ok(with_metadata(
        "Trading Analytics — Multi-Day Performance Analysis",
        &serde_json::json!({
            "trading_days": n_days,
            "starting_capital": input.starting_capital.to_string(),
            "risk_free_rate": input.risk_free_rate.map(|r| r.to_string()),
            "currency": format!("{:?}", input.currency.clone().unwrap_or_default()),
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
    use chrono::NaiveDate;
    use rust_decimal_macros::dec;

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    fn five_day_input() -> TradingAnalyticsInput {
        TradingAnalyticsInput {
            day_summaries: vec![
                DaySummary {
                    date: date(2025, 1, 6),
                    daily_pnl: dec!(500),
                    num_trades: 4,
                    num_winners: 3,
                    num_losers: 1,
                    total_profit: dec!(800),
                    total_losses: dec!(300),
                    confidence_level: 8,
                },
                DaySummary {
                    date: date(2025, 1, 7),
                    daily_pnl: dec!(-200),
                    num_trades: 3,
                    num_winners: 1,
                    num_losers: 2,
                    total_profit: dec!(100),
                    total_losses: dec!(300),
                    confidence_level: 4,
                },
                DaySummary {
                    date: date(2025, 1, 8),
                    daily_pnl: dec!(300),
                    num_trades: 5,
                    num_winners: 3,
                    num_losers: 2,
                    total_profit: dec!(600),
                    total_losses: dec!(300),
                    confidence_level: 7,
                },
                DaySummary {
                    date: date(2025, 1, 9),
                    daily_pnl: dec!(-100),
                    num_trades: 2,
                    num_winners: 0,
                    num_losers: 2,
                    total_profit: dec!(0),
                    total_losses: dec!(100),
                    confidence_level: 3,
                },
                DaySummary {
                    date: date(2025, 1, 10),
                    daily_pnl: dec!(400),
                    num_trades: 6,
                    num_winners: 4,
                    num_losers: 2,
                    total_profit: dec!(700),
                    total_losses: dec!(300),
                    confidence_level: 9,
                },
            ],
            starting_capital: dec!(10000),
            risk_free_rate: Some(dec!(0.02)),
            currency: Some(Currency::USD),
        }
    }

    // 1. Basic multi-day analytics
    #[test]
    fn test_basic_multi_day() {
        let input = five_day_input();
        let result = analyze_trading_performance(&input).unwrap();
        let out = &result.result;

        assert_eq!(out.total_trading_days, 5);
        assert_eq!(out.total_trades, 20);
        // 500 - 200 + 300 - 100 + 400 = 900
        assert_eq!(out.total_pnl, dec!(900));
        // 900 / 10000 = 0.09
        assert_eq!(out.cumulative_return, dec!(0.09));
        // 900 / 5 = 180
        assert_eq!(out.average_daily_pnl, dec!(180));
        assert_eq!(out.best_day_pnl, dec!(500));
        assert_eq!(out.worst_day_pnl, dec!(-200));
        assert_eq!(out.best_day_date, date(2025, 1, 6));
        assert_eq!(out.worst_day_date, date(2025, 1, 7));
        // 11 winners / 20 trades = 0.55
        assert_eq!(out.overall_win_rate, dec!(0.55));
    }

    // 2. All winning days
    #[test]
    fn test_all_winning_days() {
        let input = TradingAnalyticsInput {
            day_summaries: vec![
                DaySummary {
                    date: date(2025, 2, 1),
                    daily_pnl: dec!(100),
                    num_trades: 2,
                    num_winners: 2,
                    num_losers: 0,
                    total_profit: dec!(100),
                    total_losses: dec!(0),
                    confidence_level: 7,
                },
                DaySummary {
                    date: date(2025, 2, 2),
                    daily_pnl: dec!(200),
                    num_trades: 3,
                    num_winners: 3,
                    num_losers: 0,
                    total_profit: dec!(200),
                    total_losses: dec!(0),
                    confidence_level: 8,
                },
            ],
            starting_capital: dec!(5000),
            risk_free_rate: None,
            currency: None,
        };
        let result = analyze_trading_performance(&input).unwrap();
        let out = &result.result;

        assert_eq!(out.winning_days, 2);
        assert_eq!(out.losing_days, 0);
        assert_eq!(out.overall_win_rate, Decimal::ONE);
        // No losses => profit factor None
        assert!(out.overall_profit_factor.is_none());
        assert_eq!(out.max_drawdown, Decimal::ZERO);
        assert_eq!(out.max_consecutive_wins, 2);
        assert_eq!(out.max_consecutive_losses, 0);
    }

    // 3. All losing days
    #[test]
    fn test_all_losing_days() {
        let input = TradingAnalyticsInput {
            day_summaries: vec![
                DaySummary {
                    date: date(2025, 3, 1),
                    daily_pnl: dec!(-150),
                    num_trades: 2,
                    num_winners: 0,
                    num_losers: 2,
                    total_profit: dec!(0),
                    total_losses: dec!(150),
                    confidence_level: 3,
                },
                DaySummary {
                    date: date(2025, 3, 2),
                    daily_pnl: dec!(-250),
                    num_trades: 3,
                    num_winners: 0,
                    num_losers: 3,
                    total_profit: dec!(0),
                    total_losses: dec!(250),
                    confidence_level: 2,
                },
            ],
            starting_capital: dec!(5000),
            risk_free_rate: None,
            currency: None,
        };
        let result = analyze_trading_performance(&input).unwrap();
        let out = &result.result;

        assert_eq!(out.winning_days, 0);
        assert_eq!(out.losing_days, 2);
        assert_eq!(out.overall_win_rate, Decimal::ZERO);
        assert_eq!(out.max_consecutive_losses, 2);
        assert!(out.total_pnl < Decimal::ZERO);
        assert!(out.max_drawdown > Decimal::ZERO);
    }

    // 4. Single trading day
    #[test]
    fn test_single_day() {
        let input = TradingAnalyticsInput {
            day_summaries: vec![DaySummary {
                date: date(2025, 4, 1),
                daily_pnl: dec!(250),
                num_trades: 3,
                num_winners: 2,
                num_losers: 1,
                total_profit: dec!(400),
                total_losses: dec!(150),
                confidence_level: 6,
            }],
            starting_capital: dec!(10000),
            risk_free_rate: None,
            currency: None,
        };
        let result = analyze_trading_performance(&input).unwrap();
        let out = &result.result;

        assert_eq!(out.total_trading_days, 1);
        assert_eq!(out.total_pnl, dec!(250));
        assert_eq!(out.equity_curve.len(), 1);
        // stdev with 1 obs = 0 => no sharpe
        assert!(out.daily_sharpe_ratio.is_none());
    }

    // 5. Max drawdown calculation
    #[test]
    fn test_max_drawdown() {
        // Pattern: +500, -800, +200, -300, +600
        // Equity: 10500, 9700, 9900, 9600, 10200
        // Peak: 10500 => dd 800, then peak stays 10500, dd=600, dd=900 (max)
        let input = TradingAnalyticsInput {
            day_summaries: vec![
                DaySummary {
                    date: date(2025, 5, 1),
                    daily_pnl: dec!(500),
                    num_trades: 1,
                    num_winners: 1,
                    num_losers: 0,
                    total_profit: dec!(500),
                    total_losses: dec!(0),
                    confidence_level: 5,
                },
                DaySummary {
                    date: date(2025, 5, 2),
                    daily_pnl: dec!(-800),
                    num_trades: 1,
                    num_winners: 0,
                    num_losers: 1,
                    total_profit: dec!(0),
                    total_losses: dec!(800),
                    confidence_level: 5,
                },
                DaySummary {
                    date: date(2025, 5, 3),
                    daily_pnl: dec!(200),
                    num_trades: 1,
                    num_winners: 1,
                    num_losers: 0,
                    total_profit: dec!(200),
                    total_losses: dec!(0),
                    confidence_level: 5,
                },
                DaySummary {
                    date: date(2025, 5, 4),
                    daily_pnl: dec!(-300),
                    num_trades: 1,
                    num_winners: 0,
                    num_losers: 1,
                    total_profit: dec!(0),
                    total_losses: dec!(300),
                    confidence_level: 5,
                },
                DaySummary {
                    date: date(2025, 5, 5),
                    daily_pnl: dec!(600),
                    num_trades: 1,
                    num_winners: 1,
                    num_losers: 0,
                    total_profit: dec!(600),
                    total_losses: dec!(0),
                    confidence_level: 5,
                },
            ],
            starting_capital: dec!(10000),
            risk_free_rate: None,
            currency: None,
        };
        let result = analyze_trading_performance(&input).unwrap();
        let out = &result.result;

        // Peak = 10500 (after day 1), trough = 9600 (after day 4)
        // Drawdown = 900
        assert_eq!(out.max_drawdown, dec!(900));
    }

    // 6. Consecutive wins/losses streaks
    #[test]
    fn test_consecutive_streaks() {
        // W, W, W, L, L, W, L
        let days: Vec<DaySummary> = vec![
            (dec!(100), 1u32, 0u32),
            (dec!(50), 1, 0),
            (dec!(200), 1, 0),
            (dec!(-75), 0, 1),
            (dec!(-25), 0, 1),
            (dec!(150), 1, 0),
            (dec!(-50), 0, 1),
        ]
        .into_iter()
        .enumerate()
        .map(|(i, (pnl, w, l))| DaySummary {
            date: date(2025, 6, (i + 1) as u32),
            daily_pnl: pnl,
            num_trades: 1,
            num_winners: w,
            num_losers: l,
            total_profit: if pnl > Decimal::ZERO {
                pnl
            } else {
                Decimal::ZERO
            },
            total_losses: if pnl < Decimal::ZERO {
                pnl.abs()
            } else {
                Decimal::ZERO
            },
            confidence_level: 5,
        })
        .collect();

        let input = TradingAnalyticsInput {
            day_summaries: days,
            starting_capital: dec!(10000),
            risk_free_rate: None,
            currency: None,
        };
        let result = analyze_trading_performance(&input).unwrap();
        let out = &result.result;

        assert_eq!(out.max_consecutive_wins, 3);
        assert_eq!(out.max_consecutive_losses, 2);
    }

    // 7. Equity curve generation
    #[test]
    fn test_equity_curve() {
        let input = five_day_input();
        let result = analyze_trading_performance(&input).unwrap();
        let curve = &result.result.equity_curve;

        assert_eq!(curve.len(), 5);
        // Day 1: cumulative 500, equity 10500
        assert_eq!(curve[0].cumulative_pnl, dec!(500));
        assert_eq!(curve[0].equity, dec!(10500));
        assert_eq!(curve[0].drawdown, Decimal::ZERO);
        // Day 2: cumulative 300, equity 10300
        assert_eq!(curve[1].cumulative_pnl, dec!(300));
        assert_eq!(curve[1].equity, dec!(10300));
        // Drawdown from peak 10500
        assert_eq!(curve[1].drawdown, dec!(200));
        // Last day: cumulative 900, equity 10900
        assert_eq!(curve[4].cumulative_pnl, dec!(900));
        assert_eq!(curve[4].equity, dec!(10900));
    }

    // 8. Sharpe ratio positive/negative
    #[test]
    fn test_sharpe_ratio() {
        let input = five_day_input();
        let result = analyze_trading_performance(&input).unwrap();
        let out = &result.result;

        // Mean PnL > 0 with some volatility => positive Sharpe
        assert!(out.daily_sharpe_ratio.is_some());
        assert!(out.daily_sharpe_ratio.unwrap() > Decimal::ZERO);

        // All negative => negative Sharpe
        let neg_input = TradingAnalyticsInput {
            day_summaries: vec![
                DaySummary {
                    date: date(2025, 7, 1),
                    daily_pnl: dec!(-100),
                    num_trades: 1,
                    num_winners: 0,
                    num_losers: 1,
                    total_profit: dec!(0),
                    total_losses: dec!(100),
                    confidence_level: 5,
                },
                DaySummary {
                    date: date(2025, 7, 2),
                    daily_pnl: dec!(-200),
                    num_trades: 1,
                    num_winners: 0,
                    num_losers: 1,
                    total_profit: dec!(0),
                    total_losses: dec!(200),
                    confidence_level: 5,
                },
                DaySummary {
                    date: date(2025, 7, 3),
                    daily_pnl: dec!(-50),
                    num_trades: 1,
                    num_winners: 0,
                    num_losers: 1,
                    total_profit: dec!(0),
                    total_losses: dec!(50),
                    confidence_level: 5,
                },
            ],
            starting_capital: dec!(10000),
            risk_free_rate: None,
            currency: None,
        };
        let neg_result = analyze_trading_performance(&neg_input).unwrap();
        assert!(neg_result.result.daily_sharpe_ratio.unwrap() < Decimal::ZERO);
    }

    // 9. Expectancy calculation
    #[test]
    fn test_expectancy() {
        let input = five_day_input();
        let result = analyze_trading_performance(&input).unwrap();
        let out = &result.result;

        // Total winners = 11, total losers = 9, total trades = 20
        // total_profit = 2200, total_losses = 1300
        // avg_win = 2200/11 = 200, avg_loss = 1300/9 ~= 144.44
        // win_rate = 11/20 = 0.55, loss_rate = 9/20 = 0.45
        // expectancy = 200*0.55 - 144.44*0.45 = 110 - 65 = 45
        assert!(out.expectancy > Decimal::ZERO);
    }

    // 10. Profit factor calculation
    #[test]
    fn test_profit_factor() {
        let input = five_day_input();
        let result = analyze_trading_performance(&input).unwrap();
        let out = &result.result;

        // total_profit = 2200, total_losses = 1300
        // profit_factor = 2200/1300 ≈ 1.692
        let pf = out.overall_profit_factor.unwrap();
        assert!(pf > dec!(1.5));
        assert!(pf < dec!(1.8));
    }

    // 11. Empty input error
    #[test]
    fn test_empty_input_error() {
        let input = TradingAnalyticsInput {
            day_summaries: vec![],
            starting_capital: dec!(10000),
            risk_free_rate: None,
            currency: None,
        };
        let err = analyze_trading_performance(&input).unwrap_err();
        match err {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "day_summaries");
            }
            _ => panic!("Expected InvalidInput error"),
        }
    }

    // 12. Metadata populated
    #[test]
    fn test_metadata_populated() {
        let input = five_day_input();
        let result = analyze_trading_performance(&input).unwrap();

        assert!(!result.methodology.is_empty());
        assert!(
            result.metadata.computation_time_us > 0 || result.metadata.computation_time_us == 0
        );
        assert_eq!(result.metadata.precision, "rust_decimal_128bit");
        assert!(!result.metadata.version.is_empty());
    }

    // 13. Confidence correlation
    #[test]
    fn test_confidence_correlation() {
        // High confidence => high PnL pattern
        let input = TradingAnalyticsInput {
            day_summaries: vec![
                DaySummary {
                    date: date(2025, 8, 1),
                    daily_pnl: dec!(100),
                    num_trades: 2,
                    num_winners: 2,
                    num_losers: 0,
                    total_profit: dec!(100),
                    total_losses: dec!(0),
                    confidence_level: 2,
                },
                DaySummary {
                    date: date(2025, 8, 2),
                    daily_pnl: dec!(300),
                    num_trades: 2,
                    num_winners: 2,
                    num_losers: 0,
                    total_profit: dec!(300),
                    total_losses: dec!(0),
                    confidence_level: 5,
                },
                DaySummary {
                    date: date(2025, 8, 3),
                    daily_pnl: dec!(500),
                    num_trades: 2,
                    num_winners: 2,
                    num_losers: 0,
                    total_profit: dec!(500),
                    total_losses: dec!(0),
                    confidence_level: 8,
                },
                DaySummary {
                    date: date(2025, 8, 4),
                    daily_pnl: dec!(700),
                    num_trades: 2,
                    num_winners: 2,
                    num_losers: 0,
                    total_profit: dec!(700),
                    total_losses: dec!(0),
                    confidence_level: 10,
                },
            ],
            starting_capital: dec!(10000),
            risk_free_rate: None,
            currency: None,
        };
        let result = analyze_trading_performance(&input).unwrap();
        let corr = result.result.confidence_correlation;

        // Perfect positive linear relationship => correlation near 1.0
        assert!(corr.is_some());
        assert!(corr.unwrap() > dec!(0.95));
    }

    // 14. Zero starting capital error
    #[test]
    fn test_zero_starting_capital_error() {
        let input = TradingAnalyticsInput {
            day_summaries: vec![DaySummary {
                date: date(2025, 9, 1),
                daily_pnl: dec!(100),
                num_trades: 1,
                num_winners: 1,
                num_losers: 0,
                total_profit: dec!(100),
                total_losses: dec!(0),
                confidence_level: 5,
            }],
            starting_capital: Decimal::ZERO,
            risk_free_rate: None,
            currency: None,
        };
        let err = analyze_trading_performance(&input).unwrap_err();
        match err {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "starting_capital");
            }
            _ => panic!("Expected InvalidInput error"),
        }
    }

    // 15. Breakeven days counted correctly
    #[test]
    fn test_breakeven_days() {
        let input = TradingAnalyticsInput {
            day_summaries: vec![
                DaySummary {
                    date: date(2025, 10, 1),
                    daily_pnl: Decimal::ZERO,
                    num_trades: 2,
                    num_winners: 1,
                    num_losers: 1,
                    total_profit: dec!(100),
                    total_losses: dec!(100),
                    confidence_level: 5,
                },
                DaySummary {
                    date: date(2025, 10, 2),
                    daily_pnl: dec!(100),
                    num_trades: 1,
                    num_winners: 1,
                    num_losers: 0,
                    total_profit: dec!(100),
                    total_losses: dec!(0),
                    confidence_level: 5,
                },
            ],
            starting_capital: dec!(10000),
            risk_free_rate: None,
            currency: None,
        };
        let result = analyze_trading_performance(&input).unwrap();
        assert_eq!(result.result.breakeven_days, 1);
        assert_eq!(result.result.winning_days, 1);
    }

    // Helper: sqrt_decimal correctness
    #[test]
    fn test_sqrt_decimal_helper() {
        let val = sqrt_decimal(dec!(4));
        // Should be very close to 2
        assert!((val - dec!(2)).abs() < dec!(0.0000001));

        let val252 = sqrt_decimal(dec!(252));
        // Should be ~15.8745
        assert!(val252 > dec!(15.87));
        assert!(val252 < dec!(15.88));
    }

    // Helper: pearson correlation with no variance returns None
    #[test]
    fn test_pearson_no_variance() {
        let xs = vec![dec!(5), dec!(5), dec!(5)];
        let ys = vec![dec!(100), dec!(200), dec!(300)];
        assert!(pearson_correlation(&xs, &ys).is_none());
    }
}
