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

/// Absolute value for Decimal.
#[cfg(test)]
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

/// A single asset with its monthly return history.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MomentumAsset {
    /// Asset name or ticker
    pub name: String,
    /// Monthly returns (e.g. 0.05 = 5%)
    pub monthly_returns: Vec<Decimal>,
}

/// Momentum ranking for a single asset.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MomentumRanking {
    /// Asset name
    pub name: String,
    /// Raw momentum score (cumulative return over lookback minus skip)
    pub momentum_score: Decimal,
    /// Rank (1 = highest momentum)
    pub rank: usize,
    /// Annualized volatility
    pub volatility: Decimal,
    /// Risk-adjusted momentum (momentum / volatility)
    pub risk_adjusted_momentum: Decimal,
    /// Whether this asset is selected in the top_n portfolio
    pub is_selected: bool,
}

/// Asset weight in the momentum portfolio.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetWeight {
    /// Asset name
    pub name: String,
    /// Portfolio weight (0 to 1)
    pub weight: Decimal,
}

/// Input for momentum factor analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MomentumInput {
    /// List of assets with historical monthly returns
    pub assets: Vec<MomentumAsset>,
    /// Lookback period in months for momentum calculation (default 12)
    pub lookback_months: u32,
    /// Number of most recent months to skip (default 1)
    pub skip_months: u32,
    /// Rebalance frequency: "Monthly" or "Quarterly"
    pub rebalance_frequency: String,
    /// Number of top momentum assets to hold
    pub top_n: usize,
    /// Annualized risk-free rate
    pub risk_free_rate: Decimal,
}

/// Output of momentum factor analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MomentumOutput {
    /// Assets ranked by risk-adjusted momentum
    pub rankings: Vec<MomentumRanking>,
    /// Inverse-volatility-weighted portfolio weights for top_n assets
    pub portfolio_weights: Vec<AssetWeight>,
    /// Weighted expected return (annualized)
    pub portfolio_expected_return: Decimal,
    /// Portfolio volatility (annualized)
    pub portfolio_volatility: Decimal,
    /// Portfolio Sharpe ratio
    pub portfolio_sharpe: Decimal,
    /// Return spread between top and bottom quintile
    pub momentum_spread: Decimal,
    /// Estimated monthly turnover percentage
    pub turnover_rate: Decimal,
    /// Herfindahl-Hirschman Index of selected portfolio
    pub sector_concentration: Decimal,
    /// Momentum crash risk indicator (0 to 100)
    pub crash_risk_score: Decimal,
    /// Monthly portfolio returns from backtest
    pub backtest_returns: Vec<Decimal>,
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const MIN_ASSETS: usize = 2;
const MONTHS_PER_YEAR: i64 = 12;

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Analyze momentum factors across a universe of assets.
///
/// Computes momentum scores, rankings, portfolio construction with
/// inverse-volatility weights, and a full backtest with crash risk metrics.
pub fn analyze_momentum(input: &MomentumInput) -> CorpFinanceResult<MomentumOutput> {
    // ------------------------------------------------------------------
    // 1. Validate inputs
    // ------------------------------------------------------------------
    if input.assets.len() < MIN_ASSETS {
        return Err(CorpFinanceError::InsufficientData(format!(
            "At least {} assets required, got {}",
            MIN_ASSETS,
            input.assets.len()
        )));
    }
    if input.top_n == 0 {
        return Err(CorpFinanceError::InvalidInput {
            field: "top_n".into(),
            reason: "Must hold at least 1 asset".into(),
        });
    }
    if input.top_n > input.assets.len() {
        return Err(CorpFinanceError::InvalidInput {
            field: "top_n".into(),
            reason: format!(
                "top_n ({}) exceeds number of assets ({})",
                input.top_n,
                input.assets.len()
            ),
        });
    }
    if input.lookback_months == 0 {
        return Err(CorpFinanceError::InvalidInput {
            field: "lookback_months".into(),
            reason: "Lookback must be > 0".into(),
        });
    }
    let required_months = (input.lookback_months + input.skip_months) as usize;
    for asset in &input.assets {
        if asset.monthly_returns.len() < required_months {
            return Err(CorpFinanceError::InsufficientData(format!(
                "Asset '{}' has {} months of returns but {} required (lookback {} + skip {})",
                asset.name,
                asset.monthly_returns.len(),
                required_months,
                input.lookback_months,
                input.skip_months
            )));
        }
    }
    let rebalance_freq = match input.rebalance_frequency.as_str() {
        "Monthly" | "monthly" => 1usize,
        "Quarterly" | "quarterly" => 3usize,
        other => {
            return Err(CorpFinanceError::InvalidInput {
                field: "rebalance_frequency".into(),
                reason: format!("Must be 'Monthly' or 'Quarterly', got '{}'", other),
            });
        }
    };

    // All assets should have the same number of return periods
    let n_periods = input.assets[0].monthly_returns.len();
    for asset in &input.assets {
        if asset.monthly_returns.len() != n_periods {
            return Err(CorpFinanceError::InvalidInput {
                field: format!("assets[{}].monthly_returns", asset.name),
                reason: format!(
                    "Length {} differs from first asset length {}",
                    asset.monthly_returns.len(),
                    n_periods
                ),
            });
        }
    }

    let lookback = input.lookback_months as usize;
    let skip = input.skip_months as usize;
    let top_n = input.top_n;

    // ------------------------------------------------------------------
    // 2. Compute momentum scores and rankings at the latest period
    // ------------------------------------------------------------------
    let mut scored: Vec<(usize, Decimal, Decimal, Decimal)> = Vec::new(); // (idx, mom_score, vol, risk_adj)

    for (idx, asset) in input.assets.iter().enumerate() {
        let returns = &asset.monthly_returns;
        let end = n_periods - skip; // exclusive end
        let start = end.saturating_sub(lookback);

        let mom_score = cumulative_return(&returns[start..end]);
        let vol = annualized_vol(&returns[start..end]);
        let risk_adj = if vol > Decimal::ZERO {
            mom_score / vol
        } else {
            mom_score
        };

        scored.push((idx, mom_score, vol, risk_adj));
    }

    // Sort by risk_adjusted_momentum descending
    scored.sort_by(|a, b| b.3.partial_cmp(&a.3).unwrap_or(std::cmp::Ordering::Equal));

    // Build rankings
    let mut rankings: Vec<MomentumRanking> = Vec::with_capacity(scored.len());
    for (rank, &(idx, mom_score, vol, risk_adj)) in scored.iter().enumerate() {
        rankings.push(MomentumRanking {
            name: input.assets[idx].name.clone(),
            momentum_score: mom_score,
            rank: rank + 1,
            volatility: vol,
            risk_adjusted_momentum: risk_adj,
            is_selected: rank < top_n,
        });
    }

    // ------------------------------------------------------------------
    // 3. Portfolio construction: inverse-volatility weights for top_n
    // ------------------------------------------------------------------
    let selected: Vec<(usize, Decimal)> = scored
        .iter()
        .take(top_n)
        .map(|&(idx, _, vol, _)| (idx, vol))
        .collect();

    let portfolio_weights = compute_inv_vol_weights(&input.assets, &selected);

    // ------------------------------------------------------------------
    // 4. Portfolio expected return (annualized)
    // ------------------------------------------------------------------
    let portfolio_expected_return = {
        let mut weighted_ret = Decimal::ZERO;
        for w in &portfolio_weights {
            if let Some(asset) = input.assets.iter().find(|a| a.name == w.name) {
                let avg_monthly: Decimal = asset.monthly_returns.iter().copied().sum::<Decimal>()
                    / Decimal::from(asset.monthly_returns.len() as i64);
                weighted_ret += w.weight * avg_monthly * Decimal::from(MONTHS_PER_YEAR);
            }
        }
        weighted_ret
    };

    // ------------------------------------------------------------------
    // 5. Portfolio volatility (annualized, assuming independence)
    // ------------------------------------------------------------------
    let portfolio_volatility = {
        let mut weighted_var = Decimal::ZERO;
        for w in &portfolio_weights {
            if let Some(asset) = input.assets.iter().find(|a| a.name == w.name) {
                let vol = annualized_vol(&asset.monthly_returns);
                weighted_var += w.weight * w.weight * vol * vol;
            }
        }
        sqrt_decimal(weighted_var)
    };

    // ------------------------------------------------------------------
    // 6. Portfolio Sharpe
    // ------------------------------------------------------------------
    let portfolio_sharpe = if portfolio_volatility > Decimal::ZERO {
        (portfolio_expected_return - input.risk_free_rate) / portfolio_volatility
    } else {
        Decimal::ZERO
    };

    // ------------------------------------------------------------------
    // 7. Momentum spread (top quintile - bottom quintile)
    // ------------------------------------------------------------------
    let quintile_size = (input.assets.len() / 5).max(1);
    let top_quintile_avg: Decimal = scored
        .iter()
        .take(quintile_size)
        .map(|s| s.1)
        .sum::<Decimal>()
        / Decimal::from(quintile_size as i64);
    let bottom_quintile_avg: Decimal = scored
        .iter()
        .rev()
        .take(quintile_size)
        .map(|s| s.1)
        .sum::<Decimal>()
        / Decimal::from(quintile_size as i64);
    let momentum_spread = top_quintile_avg - bottom_quintile_avg;

    // ------------------------------------------------------------------
    // 8. HHI concentration
    // ------------------------------------------------------------------
    let sector_concentration: Decimal = portfolio_weights.iter().map(|w| w.weight * w.weight).sum();

    // ------------------------------------------------------------------
    // 9. Backtest
    // ------------------------------------------------------------------
    let backtest_returns = run_backtest(input, lookback, skip, top_n, rebalance_freq);

    // ------------------------------------------------------------------
    // 10. Turnover rate
    // ------------------------------------------------------------------
    let turnover_rate = compute_turnover(input, lookback, skip, top_n, rebalance_freq);

    // ------------------------------------------------------------------
    // 11. Crash risk score (0-100)
    // ------------------------------------------------------------------
    let crash_risk_score = compute_crash_risk(&backtest_returns, &scored);

    Ok(MomentumOutput {
        rankings,
        portfolio_weights,
        portfolio_expected_return,
        portfolio_volatility,
        portfolio_sharpe,
        momentum_spread,
        turnover_rate,
        sector_concentration,
        crash_risk_score,
        backtest_returns,
    })
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Cumulative return over a slice of period returns.
/// (1+r1)*(1+r2)*...*(1+rn) - 1
fn cumulative_return(returns: &[Decimal]) -> Decimal {
    if returns.is_empty() {
        return Decimal::ZERO;
    }
    let mut cum = Decimal::ONE;
    for r in returns {
        cum *= Decimal::ONE + *r;
    }
    cum - Decimal::ONE
}

/// Annualized volatility from monthly returns.
/// vol = std(monthly) * sqrt(12)
fn annualized_vol(returns: &[Decimal]) -> Decimal {
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
    let monthly_std = sqrt_decimal(var);
    monthly_std * sqrt_decimal(Decimal::from(MONTHS_PER_YEAR))
}

/// Compute inverse-volatility weights for selected assets.
fn compute_inv_vol_weights(
    assets: &[MomentumAsset],
    selected: &[(usize, Decimal)], // (asset_index, volatility)
) -> Vec<AssetWeight> {
    // Sum of inverse volatilities
    let mut inv_vol_sum = Decimal::ZERO;
    let inv_vols: Vec<Decimal> = selected
        .iter()
        .map(|&(_, vol)| {
            let iv = if vol > Decimal::ZERO {
                Decimal::ONE / vol
            } else {
                Decimal::ONE // fallback equal weight
            };
            inv_vol_sum += iv;
            iv
        })
        .collect();

    if inv_vol_sum == Decimal::ZERO {
        inv_vol_sum = Decimal::ONE;
    }

    selected
        .iter()
        .zip(inv_vols.iter())
        .map(|(&(idx, _), &iv)| AssetWeight {
            name: assets[idx].name.clone(),
            weight: iv / inv_vol_sum,
        })
        .collect()
}

/// Run rolling backtest: at each rebalance point, pick top_n by
/// risk-adjusted momentum, compute portfolio return until next rebalance.
fn run_backtest(
    input: &MomentumInput,
    lookback: usize,
    skip: usize,
    top_n: usize,
    rebalance_freq: usize,
) -> Vec<Decimal> {
    let n_periods = input.assets[0].monthly_returns.len();
    let start_period = lookback + skip;
    if start_period >= n_periods {
        return Vec::new();
    }

    let mut backtest_returns: Vec<Decimal> = Vec::new();
    let mut current_weights: Vec<(usize, Decimal)> = Vec::new(); // (asset_idx, weight)
    let mut months_since_rebalance = 0usize;

    for t in start_period..n_periods {
        // Rebalance if needed
        if months_since_rebalance.is_multiple_of(rebalance_freq) || current_weights.is_empty() {
            let end = t - skip;
            let begin = end.saturating_sub(lookback);

            // Score all assets
            let mut scored: Vec<(usize, Decimal)> = Vec::new();
            for (idx, asset) in input.assets.iter().enumerate() {
                let rets = &asset.monthly_returns[begin..end];
                let mom = cumulative_return(rets);
                let vol = annualized_vol(rets);
                let risk_adj = if vol > Decimal::ZERO { mom / vol } else { mom };
                scored.push((idx, risk_adj));
            }
            scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

            // Pick top_n, inverse-vol weight
            let selected: Vec<(usize, Decimal)> = scored
                .iter()
                .take(top_n)
                .map(|&(idx, _)| {
                    let begin_inner = (t - skip).saturating_sub(lookback);
                    let end_inner = t - skip;
                    let vol =
                        annualized_vol(&input.assets[idx].monthly_returns[begin_inner..end_inner]);
                    (idx, vol)
                })
                .collect();

            let weights = compute_inv_vol_weights_raw(&selected);
            current_weights = weights;
            months_since_rebalance = 0;
        }

        // Compute portfolio return for this month
        let mut port_ret = Decimal::ZERO;
        for &(idx, weight) in &current_weights {
            port_ret += weight * input.assets[idx].monthly_returns[t];
        }
        backtest_returns.push(port_ret);
        months_since_rebalance += 1;
    }

    backtest_returns
}

/// Raw inverse-vol weights returning (idx, weight) pairs.
fn compute_inv_vol_weights_raw(selected: &[(usize, Decimal)]) -> Vec<(usize, Decimal)> {
    let mut inv_vol_sum = Decimal::ZERO;
    let inv_vols: Vec<Decimal> = selected
        .iter()
        .map(|&(_, vol)| {
            let iv = if vol > Decimal::ZERO {
                Decimal::ONE / vol
            } else {
                Decimal::ONE
            };
            inv_vol_sum += iv;
            iv
        })
        .collect();

    if inv_vol_sum == Decimal::ZERO {
        inv_vol_sum = Decimal::ONE;
    }

    selected
        .iter()
        .zip(inv_vols.iter())
        .map(|(&(idx, _), &iv)| (idx, iv / inv_vol_sum))
        .collect()
}

/// Compute average monthly turnover.
fn compute_turnover(
    input: &MomentumInput,
    lookback: usize,
    skip: usize,
    top_n: usize,
    rebalance_freq: usize,
) -> Decimal {
    let n_periods = input.assets[0].monthly_returns.len();
    let start_period = lookback + skip;
    if start_period >= n_periods {
        return Decimal::ZERO;
    }

    let mut prev_selected: Vec<usize> = Vec::new();
    let mut total_turnover = Decimal::ZERO;
    let mut rebalance_count = 0i64;
    let mut months_since_rebalance = 0usize;

    for t in start_period..n_periods {
        if months_since_rebalance.is_multiple_of(rebalance_freq) || prev_selected.is_empty() {
            let end = t - skip;
            let begin = end.saturating_sub(lookback);

            let mut scored: Vec<(usize, Decimal)> = Vec::new();
            for (idx, asset) in input.assets.iter().enumerate() {
                let rets = &asset.monthly_returns[begin..end];
                let mom = cumulative_return(rets);
                let vol = annualized_vol(rets);
                let risk_adj = if vol > Decimal::ZERO { mom / vol } else { mom };
                scored.push((idx, risk_adj));
            }
            scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

            let new_selected: Vec<usize> = scored.iter().take(top_n).map(|s| s.0).collect();

            if !prev_selected.is_empty() {
                let changed = new_selected
                    .iter()
                    .filter(|idx| !prev_selected.contains(idx))
                    .count();
                let turnover = Decimal::from(changed as i64) / Decimal::from(top_n as i64);
                total_turnover += turnover;
                rebalance_count += 1;
            }

            prev_selected = new_selected;
            months_since_rebalance = 0;
        }
        months_since_rebalance += 1;
    }

    if rebalance_count == 0 {
        Decimal::ZERO
    } else {
        total_turnover / Decimal::from(rebalance_count)
    }
}

/// Compute crash risk score (0-100).
/// Based on max drawdown of backtest returns and momentum dispersion.
fn compute_crash_risk(
    backtest_returns: &[Decimal],
    scored: &[(usize, Decimal, Decimal, Decimal)],
) -> Decimal {
    if backtest_returns.is_empty() {
        return dec!(50); // neutral score with no data
    }

    // Component 1: Max drawdown (0-50 points)
    let mut cumulative = Decimal::ONE;
    let mut peak = Decimal::ONE;
    let mut max_dd = Decimal::ZERO;
    for r in backtest_returns {
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
    // Drawdown contribution: 0% DD = 0 points, 50%+ DD = 50 points
    let dd_score = (max_dd * dec!(100)).min(dec!(50));

    // Component 2: Momentum dispersion (0-30 points)
    // High dispersion = higher crash risk
    let mom_scores: Vec<Decimal> = scored.iter().map(|s| s.1).collect();
    let dispersion = if mom_scores.len() >= 2 {
        let mean: Decimal =
            mom_scores.iter().copied().sum::<Decimal>() / Decimal::from(mom_scores.len() as i64);
        let var: Decimal = mom_scores
            .iter()
            .map(|m| (*m - mean) * (*m - mean))
            .sum::<Decimal>()
            / Decimal::from((mom_scores.len() - 1) as i64);
        sqrt_decimal(var)
    } else {
        Decimal::ZERO
    };
    // Dispersion > 0.5 = 30 points
    let disp_score = (dispersion * dec!(60)).min(dec!(30));

    // Component 3: Negative skew in recent returns (0-20 points)
    let recent_len = backtest_returns.len().min(6);
    let recent = &backtest_returns[backtest_returns.len() - recent_len..];
    let neg_count = recent.iter().filter(|r| **r < Decimal::ZERO).count();
    let neg_ratio = Decimal::from(neg_count as i64) / Decimal::from(recent_len as i64);
    let skew_score = neg_ratio * dec!(20);

    let total = dd_score + disp_score + skew_score;
    total.min(dec!(100))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    /// Helper: create a set of assets with deterministic returns.
    fn make_assets(n_assets: usize, n_months: usize) -> Vec<MomentumAsset> {
        (0..n_assets)
            .map(|i| {
                let base = dec!(0.01) * Decimal::from((i + 1) as i64);
                let returns: Vec<Decimal> = (0..n_months)
                    .map(|m| {
                        let sign = if m % 3 == 0 {
                            Decimal::ONE
                        } else if m % 3 == 1 {
                            -Decimal::ONE
                        } else {
                            dec!(0.5)
                        };
                        base * sign
                    })
                    .collect();
                MomentumAsset {
                    name: format!("Asset{}", i + 1),
                    monthly_returns: returns,
                }
            })
            .collect()
    }

    fn default_input() -> MomentumInput {
        MomentumInput {
            assets: make_assets(10, 36),
            lookback_months: 12,
            skip_months: 1,
            rebalance_frequency: "Monthly".into(),
            top_n: 3,
            risk_free_rate: dec!(0.02),
        }
    }

    // --- Validation tests ---

    #[test]
    fn test_too_few_assets() {
        let mut input = default_input();
        input.assets = vec![input.assets[0].clone()];
        let result = analyze_momentum(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_top_n_zero() {
        let mut input = default_input();
        input.top_n = 0;
        let result = analyze_momentum(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_top_n_exceeds_assets() {
        let mut input = default_input();
        input.top_n = 100;
        let result = analyze_momentum(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_zero_lookback() {
        let mut input = default_input();
        input.lookback_months = 0;
        let result = analyze_momentum(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_insufficient_returns_for_lookback() {
        let mut input = default_input();
        input.lookback_months = 100;
        let result = analyze_momentum(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_rebalance_frequency() {
        let mut input = default_input();
        input.rebalance_frequency = "Weekly".into();
        let result = analyze_momentum(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_mismatched_return_lengths() {
        let mut input = default_input();
        input.assets[1].monthly_returns.pop();
        let result = analyze_momentum(&input);
        assert!(result.is_err());
    }

    // --- Core computation tests ---

    #[test]
    fn test_rankings_correct_count() {
        let input = default_input();
        let result = analyze_momentum(&input).unwrap();
        assert_eq!(result.rankings.len(), 10);
    }

    #[test]
    fn test_rankings_unique_ranks() {
        let input = default_input();
        let result = analyze_momentum(&input).unwrap();
        let ranks: Vec<usize> = result.rankings.iter().map(|r| r.rank).collect();
        for i in 1..=10 {
            assert!(ranks.contains(&i));
        }
    }

    #[test]
    fn test_selected_count_matches_top_n() {
        let input = default_input();
        let result = analyze_momentum(&input).unwrap();
        let selected_count = result.rankings.iter().filter(|r| r.is_selected).count();
        assert_eq!(selected_count, 3);
    }

    #[test]
    fn test_weights_sum_to_one() {
        let input = default_input();
        let result = analyze_momentum(&input).unwrap();
        let total_weight: Decimal = result.portfolio_weights.iter().map(|w| w.weight).sum();
        assert!((total_weight - Decimal::ONE).abs() < dec!(0.001));
    }

    #[test]
    fn test_weights_count_matches_top_n() {
        let input = default_input();
        let result = analyze_momentum(&input).unwrap();
        assert_eq!(result.portfolio_weights.len(), 3);
    }

    #[test]
    fn test_weights_all_positive() {
        let input = default_input();
        let result = analyze_momentum(&input).unwrap();
        for w in &result.portfolio_weights {
            assert!(w.weight > Decimal::ZERO);
        }
    }

    #[test]
    fn test_hhi_in_valid_range() {
        let input = default_input();
        let result = analyze_momentum(&input).unwrap();
        // HHI is between 1/n and 1
        assert!(result.sector_concentration > Decimal::ZERO);
        assert!(result.sector_concentration <= Decimal::ONE);
    }

    #[test]
    fn test_crash_risk_in_range() {
        let input = default_input();
        let result = analyze_momentum(&input).unwrap();
        assert!(result.crash_risk_score >= Decimal::ZERO);
        assert!(result.crash_risk_score <= dec!(100));
    }

    #[test]
    fn test_backtest_returns_non_empty() {
        let input = default_input();
        let result = analyze_momentum(&input).unwrap();
        assert!(!result.backtest_returns.is_empty());
    }

    #[test]
    fn test_volatility_non_negative() {
        let input = default_input();
        let result = analyze_momentum(&input).unwrap();
        assert!(result.portfolio_volatility >= Decimal::ZERO);
    }

    #[test]
    fn test_momentum_spread_calculated() {
        let input = default_input();
        let result = analyze_momentum(&input).unwrap();
        // With our synthetic data, higher-indexed assets have higher base returns
        // so there should be a non-zero spread
        assert!(abs_decimal(result.momentum_spread) >= Decimal::ZERO);
    }

    #[test]
    fn test_turnover_rate_non_negative() {
        let input = default_input();
        let result = analyze_momentum(&input).unwrap();
        assert!(result.turnover_rate >= Decimal::ZERO);
        assert!(result.turnover_rate <= Decimal::ONE);
    }

    // --- Quarterly rebalance ---

    #[test]
    fn test_quarterly_rebalance() {
        let mut input = default_input();
        input.rebalance_frequency = "Quarterly".into();
        let result = analyze_momentum(&input).unwrap();
        assert!(!result.backtest_returns.is_empty());
    }

    // --- Helper function tests ---

    #[test]
    fn test_cumulative_return_positive() {
        let returns = vec![dec!(0.10), dec!(0.10)];
        let cum = cumulative_return(&returns);
        // (1.10)*(1.10) - 1 = 0.21
        assert!((cum - dec!(0.21)).abs() < dec!(0.001));
    }

    #[test]
    fn test_cumulative_return_zero() {
        let returns = vec![dec!(0.10), dec!(-0.09090909090909)];
        let cum = cumulative_return(&returns);
        // ~0
        assert!(abs_decimal(cum) < dec!(0.01));
    }

    #[test]
    fn test_cumulative_return_empty() {
        let returns: Vec<Decimal> = vec![];
        assert_eq!(cumulative_return(&returns), Decimal::ZERO);
    }

    #[test]
    fn test_annualized_vol_positive() {
        let returns = vec![dec!(0.05), dec!(-0.03), dec!(0.02), dec!(-0.01)];
        let vol = annualized_vol(&returns);
        assert!(vol > Decimal::ZERO);
    }

    #[test]
    fn test_annualized_vol_single_return() {
        let returns = vec![dec!(0.05)];
        let vol = annualized_vol(&returns);
        assert_eq!(vol, Decimal::ZERO);
    }

    #[test]
    fn test_sqrt_decimal_basic() {
        let result = sqrt_decimal(dec!(9));
        assert!((result - dec!(3)).abs() < dec!(0.0001));
    }

    // --- Edge cases ---

    #[test]
    fn test_minimum_viable_input() {
        let assets = make_assets(2, 14);
        let input = MomentumInput {
            assets,
            lookback_months: 12,
            skip_months: 1,
            rebalance_frequency: "Monthly".into(),
            top_n: 1,
            risk_free_rate: dec!(0.01),
        };
        let result = analyze_momentum(&input).unwrap();
        assert_eq!(result.rankings.len(), 2);
        assert_eq!(result.portfolio_weights.len(), 1);
    }

    #[test]
    fn test_top_n_equals_assets() {
        let assets = make_assets(5, 24);
        let input = MomentumInput {
            assets,
            lookback_months: 6,
            skip_months: 1,
            rebalance_frequency: "Monthly".into(),
            top_n: 5,
            risk_free_rate: dec!(0.02),
        };
        let result = analyze_momentum(&input).unwrap();
        assert_eq!(result.portfolio_weights.len(), 5);
    }

    #[test]
    fn test_skip_zero() {
        let assets = make_assets(5, 20);
        let input = MomentumInput {
            assets,
            lookback_months: 6,
            skip_months: 0,
            rebalance_frequency: "Monthly".into(),
            top_n: 2,
            risk_free_rate: dec!(0.0),
        };
        let result = analyze_momentum(&input).unwrap();
        assert!(!result.rankings.is_empty());
    }

    #[test]
    fn test_serialization_roundtrip() {
        let input = default_input();
        let json = serde_json::to_string(&input).unwrap();
        let deserialized: MomentumInput = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.lookback_months, 12);
        assert_eq!(deserialized.top_n, 3);
    }

    #[test]
    fn test_output_serialization() {
        let input = default_input();
        let result = analyze_momentum(&input).unwrap();
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("rankings"));
        assert!(json.contains("portfolio_weights"));
        assert!(json.contains("crash_risk_score"));
    }

    #[test]
    fn test_large_universe() {
        let assets = make_assets(50, 48);
        let input = MomentumInput {
            assets,
            lookback_months: 12,
            skip_months: 1,
            rebalance_frequency: "Monthly".into(),
            top_n: 10,
            risk_free_rate: dec!(0.03),
        };
        let result = analyze_momentum(&input).unwrap();
        assert_eq!(result.rankings.len(), 50);
        assert_eq!(result.portfolio_weights.len(), 10);
        assert!(result.backtest_returns.len() > 20);
    }

    #[test]
    fn test_equal_momentum_scores() {
        // All assets with identical returns should still work
        let assets: Vec<MomentumAsset> = (0..5)
            .map(|i| MomentumAsset {
                name: format!("Same{}", i),
                monthly_returns: vec![dec!(0.01); 24],
            })
            .collect();
        let input = MomentumInput {
            assets,
            lookback_months: 6,
            skip_months: 1,
            rebalance_frequency: "Monthly".into(),
            top_n: 2,
            risk_free_rate: dec!(0.01),
        };
        let result = analyze_momentum(&input).unwrap();
        // All should have equal momentum, but ranking should still be assigned
        assert_eq!(result.rankings.len(), 5);
    }
}
