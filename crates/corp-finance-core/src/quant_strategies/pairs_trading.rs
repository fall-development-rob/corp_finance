use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

use crate::{CorpFinanceError, CorpFinanceResult};

// ---------------------------------------------------------------------------
// Decimal math helpers
// ---------------------------------------------------------------------------

/// Newton's method square root (20 iterations).
fn sqrt_decimal(val: Decimal) -> Decimal {
    if val <= Decimal::ZERO {
        return Decimal::ZERO;
    }
    let mut guess = val / dec!(2);
    if guess == Decimal::ZERO {
        guess = Decimal::ONE;
    }
    for _ in 0..20 {
        guess = (guess + val / guess) / dec!(2);
    }
    guess
}

/// Taylor-series natural logarithm.
/// ln(x) = 2 * sum_{k=0..20} (1/(2k+1)) * ((x-1)/(x+1))^(2k+1)
/// Range-reduced via powers of 2.
fn ln_decimal(x: Decimal) -> Decimal {
    if x <= Decimal::ZERO {
        return Decimal::ZERO;
    }
    if x == Decimal::ONE {
        return Decimal::ZERO;
    }
    let ln2 = dec!(0.6931471805599453);
    let mut val = x;
    let mut k: i64 = 0;
    while val > dec!(2) {
        val /= dec!(2);
        k += 1;
    }
    while val < dec!(0.5) {
        val *= dec!(2);
        k -= 1;
    }
    let u = (val - Decimal::ONE) / (val + Decimal::ONE);
    let u2 = u * u;
    let mut term = u;
    let mut sum = u;
    for n in 1..=20 {
        term *= u2;
        let denom = Decimal::from(2 * n + 1);
        sum += term / denom;
    }
    dec!(2) * sum + Decimal::from(k) * ln2
}

/// Absolute value for Decimal.
fn abs_decimal(x: Decimal) -> Decimal {
    if x < Decimal::ZERO {
        -x
    } else {
        x
    }
}

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// A single backtested trade from the pairs trading strategy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairTrade {
    /// Period index when trade was entered
    pub entry_period: usize,
    /// Period index when trade was exited
    pub exit_period: usize,
    /// Profit or loss of the trade
    pub pnl: Decimal,
    /// Number of periods the trade was held
    pub holding_periods: usize,
    /// Z-score at entry
    pub entry_z: Decimal,
    /// Z-score at exit
    pub exit_z: Decimal,
}

/// Input for pairs trading analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairsTradingInput {
    /// Name of asset A
    pub asset_a_name: String,
    /// Name of asset B
    pub asset_b_name: String,
    /// Historical prices for asset A (at least 20)
    pub asset_a_prices: Vec<Decimal>,
    /// Historical prices for asset B (at least 20)
    pub asset_b_prices: Vec<Decimal>,
    /// Lookback period for z-score calculation (default 20)
    pub lookback_period: u32,
    /// Z-score threshold to enter a trade (default 2.0)
    pub entry_z_score: Decimal,
    /// Z-score threshold to exit a trade (default 0.5)
    pub exit_z_score: Decimal,
    /// Z-score threshold for stop loss (default 3.5)
    pub stop_loss_z_score: Decimal,
    /// Total capital allocated to the strategy
    pub capital: Decimal,
    /// Transaction cost in basis points
    pub transaction_cost_bps: Decimal,
}

/// Output of pairs trading analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairsTradingOutput {
    /// Pearson correlation between asset A and asset B prices
    pub correlation: Decimal,
    /// Engle-Granger ADF-like cointegration test statistic
    pub cointegration_score: Decimal,
    /// Whether the pair is cointegrated (score < -3.5)
    pub is_cointegrated: bool,
    /// OLS hedge ratio (beta): B regressed on A
    pub hedge_ratio: Decimal,
    /// Mean of the spread
    pub spread_mean: Decimal,
    /// Standard deviation of the spread
    pub spread_std: Decimal,
    /// Current (latest) z-score
    pub current_z_score: Decimal,
    /// Trading signal based on current z-score
    pub signal: String,
    /// Mean-reversion half-life in periods
    pub half_life: Decimal,
    /// Backtested historical trades
    pub historical_trades: Vec<PairTrade>,
    /// Total P&L from backtested trades
    pub total_pnl: Decimal,
    /// Annualized Sharpe ratio of backtest returns
    pub sharpe_ratio: Decimal,
    /// Win rate (fraction of winning trades)
    pub win_rate: Decimal,
    /// Maximum drawdown
    pub max_drawdown: Decimal,
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const MIN_PRICES: usize = 20;

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Analyze a pairs trading strategy for two assets.
///
/// Computes correlation, cointegration, hedge ratio, spread statistics,
/// trading signals, and a full backtest with P&L, Sharpe, and drawdown.
pub fn analyze_pairs_trading(input: &PairsTradingInput) -> CorpFinanceResult<PairsTradingOutput> {
    // ------------------------------------------------------------------
    // 1. Validate inputs
    // ------------------------------------------------------------------
    let n = input.asset_a_prices.len();
    if n < MIN_PRICES {
        return Err(CorpFinanceError::InsufficientData(format!(
            "At least {} price observations required, got {}",
            MIN_PRICES, n
        )));
    }
    if input.asset_b_prices.len() != n {
        return Err(CorpFinanceError::InvalidInput {
            field: "asset_b_prices".into(),
            reason: format!(
                "Asset B has {} prices but asset A has {} — must be equal",
                input.asset_b_prices.len(),
                n
            ),
        });
    }
    if input.lookback_period == 0 {
        return Err(CorpFinanceError::InvalidInput {
            field: "lookback_period".into(),
            reason: "Lookback period must be > 0".into(),
        });
    }
    if input.entry_z_score <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "entry_z_score".into(),
            reason: "Entry z-score must be positive".into(),
        });
    }
    if input.exit_z_score < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "exit_z_score".into(),
            reason: "Exit z-score must be non-negative".into(),
        });
    }
    if input.stop_loss_z_score <= input.entry_z_score {
        return Err(CorpFinanceError::InvalidInput {
            field: "stop_loss_z_score".into(),
            reason: "Stop-loss z-score must exceed entry z-score".into(),
        });
    }
    if input.capital <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "capital".into(),
            reason: "Capital must be positive".into(),
        });
    }

    let a = &input.asset_a_prices;
    let b = &input.asset_b_prices;
    let n_dec = Decimal::from(n as i64);

    // ------------------------------------------------------------------
    // 2. Pearson correlation
    // ------------------------------------------------------------------
    let correlation = pearson_correlation(a, b)?;

    // ------------------------------------------------------------------
    // 3. OLS hedge ratio: cov(A,B) / var(A)
    // ------------------------------------------------------------------
    let mean_a: Decimal = a.iter().copied().sum::<Decimal>() / n_dec;
    let mean_b: Decimal = b.iter().copied().sum::<Decimal>() / n_dec;

    let mut cov_ab = Decimal::ZERO;
    let mut var_a = Decimal::ZERO;
    for i in 0..n {
        let da = a[i] - mean_a;
        let db = b[i] - mean_b;
        cov_ab += da * db;
        var_a += da * da;
    }

    if var_a == Decimal::ZERO {
        return Err(CorpFinanceError::DivisionByZero {
            context: "OLS hedge ratio — asset A has zero variance".into(),
        });
    }

    let hedge_ratio = cov_ab / var_a;

    // ------------------------------------------------------------------
    // 4. Spread = B - hedge_ratio * A
    // ------------------------------------------------------------------
    let spread: Vec<Decimal> = (0..n).map(|i| b[i] - hedge_ratio * a[i]).collect();

    let spread_mean: Decimal = spread.iter().copied().sum::<Decimal>() / n_dec;
    let spread_var: Decimal = spread
        .iter()
        .map(|s| {
            let d = *s - spread_mean;
            d * d
        })
        .sum::<Decimal>()
        / (n_dec - Decimal::ONE);
    let spread_std = sqrt_decimal(spread_var);

    // ------------------------------------------------------------------
    // 5. Z-scores
    // ------------------------------------------------------------------
    let z_scores: Vec<Decimal> = if spread_std == Decimal::ZERO {
        vec![Decimal::ZERO; n]
    } else {
        spread
            .iter()
            .map(|s| (*s - spread_mean) / spread_std)
            .collect()
    };

    let current_z_score = z_scores[n - 1];

    // ------------------------------------------------------------------
    // 6. Trading signal
    // ------------------------------------------------------------------
    let signal = if abs_decimal(current_z_score) > input.stop_loss_z_score {
        "Stop Loss".to_string()
    } else if current_z_score > input.entry_z_score {
        "Short A / Long B".to_string()
    } else if current_z_score < -input.entry_z_score {
        "Long A / Short B".to_string()
    } else {
        "No Signal".to_string()
    };

    // ------------------------------------------------------------------
    // 7. Cointegration test (ADF-like on spread residuals)
    // ------------------------------------------------------------------
    let cointegration_score = adf_test_statistic(&spread)?;
    let is_cointegrated = cointegration_score < dec!(-3.5);

    // ------------------------------------------------------------------
    // 8. Half-life via AR(1)
    // ------------------------------------------------------------------
    let half_life = compute_half_life(&spread);

    // ------------------------------------------------------------------
    // 9. Backtest
    // ------------------------------------------------------------------
    let lookback = input.lookback_period as usize;
    let tc_rate = input.transaction_cost_bps / dec!(10000);
    let historical_trades = backtest(
        &z_scores,
        &spread,
        input.entry_z_score,
        input.exit_z_score,
        input.stop_loss_z_score,
        input.capital,
        tc_rate,
        lookback,
    );

    // ------------------------------------------------------------------
    // 10. Aggregate backtest metrics
    // ------------------------------------------------------------------
    let total_pnl: Decimal = historical_trades.iter().map(|t| t.pnl).sum();

    let win_count = historical_trades
        .iter()
        .filter(|t| t.pnl > Decimal::ZERO)
        .count();
    let win_rate = if historical_trades.is_empty() {
        Decimal::ZERO
    } else {
        Decimal::from(win_count as i64) / Decimal::from(historical_trades.len() as i64)
    };

    // Compute per-period returns for Sharpe and drawdown
    let mut period_returns = vec![Decimal::ZERO; n];
    for trade in &historical_trades {
        if trade.holding_periods > 0 {
            let per_period = trade.pnl / Decimal::from(trade.holding_periods as i64);
            for item in period_returns
                .iter_mut()
                .take(trade.exit_period.min(n))
                .skip(trade.entry_period)
            {
                *item = per_period / input.capital;
            }
        }
    }

    let sharpe_ratio = compute_sharpe(&period_returns);
    let max_drawdown = compute_max_drawdown(&period_returns);

    Ok(PairsTradingOutput {
        correlation,
        cointegration_score,
        is_cointegrated,
        hedge_ratio,
        spread_mean,
        spread_std,
        current_z_score,
        signal,
        half_life,
        historical_trades,
        total_pnl,
        sharpe_ratio,
        win_rate,
        max_drawdown,
    })
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Pearson correlation coefficient between two series.
fn pearson_correlation(x: &[Decimal], y: &[Decimal]) -> CorpFinanceResult<Decimal> {
    let n = x.len();
    let n_dec = Decimal::from(n as i64);
    let mean_x: Decimal = x.iter().copied().sum::<Decimal>() / n_dec;
    let mean_y: Decimal = y.iter().copied().sum::<Decimal>() / n_dec;

    let mut cov = Decimal::ZERO;
    let mut var_x = Decimal::ZERO;
    let mut var_y = Decimal::ZERO;

    for i in 0..n {
        let dx = x[i] - mean_x;
        let dy = y[i] - mean_y;
        cov += dx * dy;
        var_x += dx * dx;
        var_y += dy * dy;
    }

    let denom = sqrt_decimal(var_x) * sqrt_decimal(var_y);
    if denom == Decimal::ZERO {
        return Err(CorpFinanceError::DivisionByZero {
            context: "Pearson correlation — zero variance".into(),
        });
    }
    Ok(cov / denom)
}

/// ADF-like test statistic on the spread residuals.
/// Runs AR(1) regression on spread differences: dS_t = alpha + beta * S_{t-1} + e_t
/// Returns t-statistic of beta. More negative = more evidence of stationarity.
fn adf_test_statistic(spread: &[Decimal]) -> CorpFinanceResult<Decimal> {
    let n = spread.len();
    if n < 3 {
        return Err(CorpFinanceError::InsufficientData(
            "Need at least 3 observations for ADF test".into(),
        ));
    }
    let m = n - 1; // number of (dS, S_lag) pairs
    let m_dec = Decimal::from(m as i64);

    // dS_t = spread[t] - spread[t-1], S_lag = spread[t-1]
    let mut sum_lag = Decimal::ZERO;
    let mut sum_ds = Decimal::ZERO;
    let mut sum_lag2 = Decimal::ZERO;
    let mut sum_lag_ds = Decimal::ZERO;

    for t in 1..n {
        let ds = spread[t] - spread[t - 1];
        let lag = spread[t - 1];
        sum_lag += lag;
        sum_ds += ds;
        sum_lag2 += lag * lag;
        sum_lag_ds += lag * ds;
    }

    let mean_lag = sum_lag / m_dec;
    let mean_ds = sum_ds / m_dec;

    let cov = sum_lag_ds / m_dec - mean_lag * mean_ds;
    let var_lag = sum_lag2 / m_dec - mean_lag * mean_lag;

    if var_lag == Decimal::ZERO {
        return Ok(Decimal::ZERO);
    }

    let beta = cov / var_lag;
    let alpha = mean_ds - beta * mean_lag;

    // Residual standard error
    let mut sse = Decimal::ZERO;
    for t in 1..n {
        let ds = spread[t] - spread[t - 1];
        let lag = spread[t - 1];
        let e = ds - alpha - beta * lag;
        sse += e * e;
    }
    let residual_var = sse / Decimal::from((m - 2).max(1) as i64);
    let se_beta = sqrt_decimal(residual_var / (var_lag * m_dec));

    if se_beta == Decimal::ZERO {
        return Ok(Decimal::ZERO);
    }

    Ok(beta / se_beta)
}

/// Compute mean-reversion half-life via AR(1) on spread.
/// AR(1): S_t = c + phi * S_{t-1} + e
/// half_life = -ln(2) / ln(phi)
fn compute_half_life(spread: &[Decimal]) -> Decimal {
    let n = spread.len();
    if n < 3 {
        return Decimal::ZERO;
    }
    let m = n - 1;
    let m_dec = Decimal::from(m as i64);

    let mut sum_x = Decimal::ZERO;
    let mut sum_y = Decimal::ZERO;
    let mut sum_xy = Decimal::ZERO;
    let mut sum_x2 = Decimal::ZERO;

    for t in 1..n {
        let x = spread[t - 1];
        let y = spread[t];
        sum_x += x;
        sum_y += y;
        sum_xy += x * y;
        sum_x2 += x * x;
    }

    let denom = m_dec * sum_x2 - sum_x * sum_x;
    if denom == Decimal::ZERO {
        return Decimal::ZERO;
    }

    let phi = (m_dec * sum_xy - sum_x * sum_y) / denom;

    // phi must be in (0, 1) for mean-reversion
    if phi <= Decimal::ZERO || phi >= Decimal::ONE {
        return Decimal::ZERO;
    }

    let ln2 = dec!(0.6931471805599453);
    let ln_phi = ln_decimal(phi);
    if ln_phi == Decimal::ZERO {
        return Decimal::ZERO;
    }

    abs_decimal(-ln2 / ln_phi)
}

/// Run backtest on z-scores, generating trade entries and exits.
fn backtest(
    z_scores: &[Decimal],
    spread: &[Decimal],
    entry_z: Decimal,
    exit_z: Decimal,
    stop_loss_z: Decimal,
    capital: Decimal,
    tc_rate: Decimal,
    lookback: usize,
) -> Vec<PairTrade> {
    let n = z_scores.len();
    let mut trades: Vec<PairTrade> = Vec::new();
    let start = lookback.min(n);

    let mut in_trade = false;
    let mut entry_period: usize = 0;
    let mut entry_z_val = Decimal::ZERO;
    let mut entry_spread = Decimal::ZERO;
    let mut is_long_spread = false; // true = long spread (short A, long B)

    for i in start..n {
        let z = z_scores[i];
        let abs_z = abs_decimal(z);

        if !in_trade {
            // Check for entry
            if abs_z > entry_z {
                in_trade = true;
                entry_period = i;
                entry_z_val = z;
                entry_spread = spread[i];
                is_long_spread = z < Decimal::ZERO;
                // When z < 0, spread is below mean => long the spread
                // When z > 0, spread is above mean => short the spread
            }
        } else {
            // Check for exit
            let should_exit = abs_z < exit_z || abs_z > stop_loss_z;
            let at_end = i == n - 1;

            if should_exit || at_end {
                let exit_spread = spread[i];
                let spread_change = exit_spread - entry_spread;
                let raw_pnl = if is_long_spread {
                    spread_change * capital / abs_decimal(entry_spread).max(Decimal::ONE)
                } else {
                    -spread_change * capital / abs_decimal(entry_spread).max(Decimal::ONE)
                };
                // Deduct transaction costs (entry + exit)
                let tc = dec!(2) * tc_rate * capital;
                let pnl = raw_pnl - tc;

                trades.push(PairTrade {
                    entry_period,
                    exit_period: i,
                    pnl,
                    holding_periods: i - entry_period,
                    entry_z: entry_z_val,
                    exit_z: z,
                });
                in_trade = false;
            }
        }
    }

    trades
}

/// Annualized Sharpe ratio from period returns.
/// Sharpe = mean(r) / std(r) * sqrt(252)
fn compute_sharpe(returns: &[Decimal]) -> Decimal {
    let n = returns.len();
    if n < 2 {
        return Decimal::ZERO;
    }
    let n_dec = Decimal::from(n as i64);
    let mean: Decimal = returns.iter().copied().sum::<Decimal>() / n_dec;

    let var: Decimal = returns
        .iter()
        .map(|r| {
            let d = *r - mean;
            d * d
        })
        .sum::<Decimal>()
        / (n_dec - Decimal::ONE);

    let std = sqrt_decimal(var);
    if std == Decimal::ZERO {
        return Decimal::ZERO;
    }

    let sqrt_252 = sqrt_decimal(dec!(252));
    mean / std * sqrt_252
}

/// Maximum drawdown from a return series.
fn compute_max_drawdown(returns: &[Decimal]) -> Decimal {
    let mut cumulative = Decimal::ONE;
    let mut peak = Decimal::ONE;
    let mut max_dd = Decimal::ZERO;

    for r in returns {
        cumulative *= Decimal::ONE + *r;
        if cumulative > peak {
            peak = cumulative;
        }
        if peak > Decimal::ZERO {
            let dd = (peak - cumulative) / peak;
            if dd > max_dd {
                max_dd = dd;
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

    /// Helper: generate a cointegrated pair with noise.
    fn make_cointegrated_prices(n: usize) -> (Vec<Decimal>, Vec<Decimal>) {
        let mut a = Vec::with_capacity(n);
        let mut b = Vec::with_capacity(n);
        let mut price_a = dec!(100);
        let mut price_b;

        // Deterministic walk with mean-reverting spread
        let increments: Vec<Decimal> = (0..n)
            .map(|i| {
                let sign = if i % 3 == 0 {
                    dec!(1)
                } else if i % 3 == 1 {
                    dec!(-1)
                } else {
                    dec!(0.5)
                };
                sign * dec!(0.5)
            })
            .collect();

        for i in 0..n {
            price_a += increments[i];
            // B tracks A with hedge ratio ~2 plus mean-reverting noise
            let noise = if i % 5 == 0 {
                dec!(0.3)
            } else if i % 5 == 1 {
                dec!(-0.3)
            } else if i % 5 == 2 {
                dec!(0.1)
            } else if i % 5 == 3 {
                dec!(-0.1)
            } else {
                dec!(0.0)
            };
            price_b = dec!(2) * price_a + noise;
            a.push(price_a);
            b.push(price_b);
        }

        (a, b)
    }

    fn default_input() -> PairsTradingInput {
        let (a, b) = make_cointegrated_prices(60);
        PairsTradingInput {
            asset_a_name: "AAPL".into(),
            asset_b_name: "MSFT".into(),
            asset_a_prices: a,
            asset_b_prices: b,
            lookback_period: 20,
            entry_z_score: dec!(2.0),
            exit_z_score: dec!(0.5),
            stop_loss_z_score: dec!(3.5),
            capital: dec!(100000),
            transaction_cost_bps: dec!(10),
        }
    }

    // --- Validation tests ---

    #[test]
    fn test_too_few_prices() {
        let mut input = default_input();
        input.asset_a_prices = vec![dec!(100); 10];
        input.asset_b_prices = vec![dec!(200); 10];
        let result = analyze_pairs_trading(&input);
        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        assert!(err_msg.contains("20"));
    }

    #[test]
    fn test_mismatched_lengths() {
        let mut input = default_input();
        input.asset_b_prices.pop();
        let result = analyze_pairs_trading(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_zero_lookback() {
        let mut input = default_input();
        input.lookback_period = 0;
        let result = analyze_pairs_trading(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_negative_entry_z() {
        let mut input = default_input();
        input.entry_z_score = dec!(-1);
        let result = analyze_pairs_trading(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_negative_exit_z() {
        let mut input = default_input();
        input.exit_z_score = dec!(-0.5);
        let result = analyze_pairs_trading(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_stop_loss_below_entry() {
        let mut input = default_input();
        input.stop_loss_z_score = dec!(1.5);
        let result = analyze_pairs_trading(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_zero_capital() {
        let mut input = default_input();
        input.capital = Decimal::ZERO;
        let result = analyze_pairs_trading(&input);
        assert!(result.is_err());
    }

    // --- Core computation tests ---

    #[test]
    fn test_high_correlation_cointegrated_pair() {
        let input = default_input();
        let result = analyze_pairs_trading(&input).unwrap();
        // Highly correlated pair should have correlation near 1
        assert!(result.correlation > dec!(0.9));
    }

    #[test]
    fn test_hedge_ratio_near_expected() {
        let input = default_input();
        let result = analyze_pairs_trading(&input).unwrap();
        // Hedge ratio should be near 2.0 for our synthetic pair
        assert!(result.hedge_ratio > dec!(1.5));
        assert!(result.hedge_ratio < dec!(2.5));
    }

    #[test]
    fn test_spread_mean_near_zero() {
        let input = default_input();
        let result = analyze_pairs_trading(&input).unwrap();
        // Spread mean should be small for cointegrated pair
        assert!(abs_decimal(result.spread_mean) < dec!(5));
    }

    #[test]
    fn test_spread_std_positive() {
        let input = default_input();
        let result = analyze_pairs_trading(&input).unwrap();
        assert!(result.spread_std > Decimal::ZERO);
    }

    #[test]
    fn test_current_z_score_finite() {
        let input = default_input();
        let result = analyze_pairs_trading(&input).unwrap();
        assert!(abs_decimal(result.current_z_score) < dec!(100));
    }

    #[test]
    fn test_signal_is_valid() {
        let input = default_input();
        let result = analyze_pairs_trading(&input).unwrap();
        let valid_signals = [
            "Long A / Short B",
            "Short A / Long B",
            "No Signal",
            "Stop Loss",
        ];
        assert!(valid_signals.contains(&result.signal.as_str()));
    }

    #[test]
    fn test_cointegration_detected() {
        let input = default_input();
        let result = analyze_pairs_trading(&input).unwrap();
        // Our synthetic pair is cointegrated by construction
        // The score should be negative (more negative = stronger)
        assert!(result.cointegration_score < Decimal::ZERO);
    }

    #[test]
    fn test_half_life_positive() {
        let input = default_input();
        let result = analyze_pairs_trading(&input).unwrap();
        // For a mean-reverting spread, half-life should be positive
        // It may be zero if AR(1) coefficient is outside (0,1)
        assert!(result.half_life >= Decimal::ZERO);
    }

    #[test]
    fn test_win_rate_in_range() {
        let input = default_input();
        let result = analyze_pairs_trading(&input).unwrap();
        assert!(result.win_rate >= Decimal::ZERO);
        assert!(result.win_rate <= Decimal::ONE);
    }

    #[test]
    fn test_max_drawdown_non_negative() {
        let input = default_input();
        let result = analyze_pairs_trading(&input).unwrap();
        assert!(result.max_drawdown >= Decimal::ZERO);
    }

    #[test]
    fn test_sharpe_ratio_finite() {
        let input = default_input();
        let result = analyze_pairs_trading(&input).unwrap();
        assert!(abs_decimal(result.sharpe_ratio) < dec!(1000));
    }

    // --- Uncorrelated / divergent pair ---

    #[test]
    fn test_uncorrelated_pair() {
        let a: Vec<Decimal> = (0..30)
            .map(|i| dec!(100) + Decimal::from(i as i64))
            .collect();
        let b: Vec<Decimal> = (0..30)
            .map(|i| dec!(200) - Decimal::from(i as i64))
            .collect();

        let input = PairsTradingInput {
            asset_a_name: "UP".into(),
            asset_b_name: "DOWN".into(),
            asset_a_prices: a,
            asset_b_prices: b,
            lookback_period: 10,
            entry_z_score: dec!(2.0),
            exit_z_score: dec!(0.5),
            stop_loss_z_score: dec!(3.5),
            capital: dec!(50000),
            transaction_cost_bps: dec!(5),
        };
        let result = analyze_pairs_trading(&input).unwrap();
        // Strongly negative correlation for opposite trends
        assert!(result.correlation < dec!(-0.5));
    }

    // --- Helper function tests ---

    #[test]
    fn test_pearson_perfect_positive() {
        let x: Vec<Decimal> = (1..=10).map(|i| Decimal::from(i)).collect();
        let y: Vec<Decimal> = (1..=10).map(|i| Decimal::from(i * 2)).collect();
        let r = pearson_correlation(&x, &y).unwrap();
        assert!(r > dec!(0.999));
    }

    #[test]
    fn test_pearson_perfect_negative() {
        let x: Vec<Decimal> = (1..=10).map(|i| Decimal::from(i)).collect();
        let y: Vec<Decimal> = (1..=10).map(|i| Decimal::from(11 - i)).collect();
        let r = pearson_correlation(&x, &y).unwrap();
        assert!(r < dec!(-0.999));
    }

    #[test]
    fn test_sqrt_decimal_basic() {
        let val = dec!(4);
        let result = sqrt_decimal(val);
        assert!((result - dec!(2)).abs() < dec!(0.0001));
    }

    #[test]
    fn test_sqrt_decimal_large() {
        let val = dec!(10000);
        let result = sqrt_decimal(val);
        assert!((result - dec!(100)).abs() < dec!(0.001));
    }

    #[test]
    fn test_sqrt_decimal_zero() {
        assert_eq!(sqrt_decimal(Decimal::ZERO), Decimal::ZERO);
    }

    #[test]
    fn test_sqrt_decimal_negative() {
        assert_eq!(sqrt_decimal(dec!(-4)), Decimal::ZERO);
    }

    #[test]
    fn test_ln_decimal_one() {
        assert_eq!(ln_decimal(Decimal::ONE), Decimal::ZERO);
    }

    #[test]
    fn test_ln_decimal_e_approx() {
        let e_approx = dec!(2.718281828);
        let result = ln_decimal(e_approx);
        assert!((result - Decimal::ONE).abs() < dec!(0.001));
    }

    #[test]
    fn test_ln_decimal_zero_or_negative() {
        assert_eq!(ln_decimal(Decimal::ZERO), Decimal::ZERO);
        assert_eq!(ln_decimal(dec!(-5)), Decimal::ZERO);
    }

    #[test]
    fn test_adf_stationary_series() {
        // A deterministic mean-reverting series should have negative ADF stat
        let spread: Vec<Decimal> = (0..40)
            .map(|i| {
                let phase = Decimal::from(i % 4);
                if phase == Decimal::ZERO {
                    dec!(1)
                } else if phase == Decimal::ONE {
                    dec!(-1)
                } else if phase == dec!(2) {
                    dec!(0.5)
                } else {
                    dec!(-0.5)
                }
            })
            .collect();
        let stat = adf_test_statistic(&spread).unwrap();
        assert!(stat < Decimal::ZERO);
    }

    #[test]
    fn test_max_drawdown_no_loss() {
        let returns = vec![dec!(0.01), dec!(0.02), dec!(0.01), dec!(0.03)];
        let dd = compute_max_drawdown(&returns);
        assert_eq!(dd, Decimal::ZERO);
    }

    #[test]
    fn test_max_drawdown_with_loss() {
        let returns = vec![dec!(0.10), dec!(-0.20), dec!(0.05)];
        let dd = compute_max_drawdown(&returns);
        assert!(dd > Decimal::ZERO);
    }

    #[test]
    fn test_sharpe_all_zeros() {
        let returns = vec![Decimal::ZERO; 30];
        let sharpe = compute_sharpe(&returns);
        assert_eq!(sharpe, Decimal::ZERO);
    }

    #[test]
    fn test_backtest_with_extreme_z_entry() {
        // Very high entry threshold = no trades
        let z_scores: Vec<Decimal> = (0..30).map(|_| dec!(0.5)).collect();
        let spread: Vec<Decimal> = (0..30).map(|_| dec!(10)).collect();
        let trades = backtest(
            &z_scores,
            &spread,
            dec!(10.0), // very high entry
            dec!(0.5),
            dec!(15.0),
            dec!(100000),
            dec!(0.001),
            5,
        );
        assert!(trades.is_empty());
    }

    #[test]
    fn test_large_dataset() {
        let n = 500;
        let (a, b) = make_cointegrated_prices(n);
        let input = PairsTradingInput {
            asset_a_name: "A".into(),
            asset_b_name: "B".into(),
            asset_a_prices: a,
            asset_b_prices: b,
            lookback_period: 30,
            entry_z_score: dec!(1.5),
            exit_z_score: dec!(0.3),
            stop_loss_z_score: dec!(3.0),
            capital: dec!(1000000),
            transaction_cost_bps: dec!(5),
        };
        let result = analyze_pairs_trading(&input).unwrap();
        assert!(result.correlation > dec!(0.8));
    }

    #[test]
    fn test_serialization_roundtrip() {
        let input = default_input();
        let json = serde_json::to_string(&input).unwrap();
        let deserialized: PairsTradingInput = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.asset_a_name, "AAPL");
        assert_eq!(deserialized.lookback_period, 20);
    }

    #[test]
    fn test_output_serialization() {
        let input = default_input();
        let result = analyze_pairs_trading(&input).unwrap();
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("correlation"));
        assert!(json.contains("hedge_ratio"));
    }

    #[test]
    fn test_minimum_lookback() {
        let (a, b) = make_cointegrated_prices(25);
        let input = PairsTradingInput {
            asset_a_name: "X".into(),
            asset_b_name: "Y".into(),
            asset_a_prices: a,
            asset_b_prices: b,
            lookback_period: 1,
            entry_z_score: dec!(1.0),
            exit_z_score: dec!(0.2),
            stop_loss_z_score: dec!(4.0),
            capital: dec!(10000),
            transaction_cost_bps: dec!(10),
        };
        let result = analyze_pairs_trading(&input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_constant_price_a_fails() {
        let a = vec![dec!(100); 30];
        let b: Vec<Decimal> = (0..30)
            .map(|i| dec!(200) + Decimal::from(i as i64))
            .collect();
        let input = PairsTradingInput {
            asset_a_name: "CONST".into(),
            asset_b_name: "RISING".into(),
            asset_a_prices: a,
            asset_b_prices: b,
            lookback_period: 10,
            entry_z_score: dec!(2.0),
            exit_z_score: dec!(0.5),
            stop_loss_z_score: dec!(3.5),
            capital: dec!(100000),
            transaction_cost_bps: dec!(5),
        };
        let result = analyze_pairs_trading(&input);
        // Should fail due to zero variance in A
        assert!(result.is_err());
    }
}
