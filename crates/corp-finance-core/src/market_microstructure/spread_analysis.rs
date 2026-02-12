use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::error::CorpFinanceError;
use crate::types::{with_metadata, ComputationOutput};
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Decimal math helpers
// ---------------------------------------------------------------------------

fn sqrt_decimal(x: Decimal) -> Decimal {
    if x <= Decimal::ZERO {
        return Decimal::ZERO;
    }
    if x == Decimal::ONE {
        return Decimal::ONE;
    }
    let two = dec!(2);
    let mut guess = x / two;
    if x > dec!(100) {
        guess = dec!(10);
    } else if x < dec!(0.01) {
        guess = dec!(0.1);
    }
    for _ in 0..20 {
        guess = (guess + x / guess) / two;
    }
    guess
}

fn exp_decimal(x: Decimal) -> Decimal {
    let two = dec!(2);
    if x > two || x < -two {
        let half = exp_decimal(x / two);
        return half * half;
    }
    let mut sum = Decimal::ONE;
    let mut term = Decimal::ONE;
    for n in 1u32..=40 {
        term = term * x / Decimal::from(n);
        sum += term;
    }
    sum
}

fn ln_decimal(x: Decimal) -> Decimal {
    if x <= Decimal::ZERO {
        return dec!(-999);
    }
    if x == Decimal::ONE {
        return Decimal::ZERO;
    }
    let mut y = if x > dec!(0.5) && x < dec!(2) {
        x - Decimal::ONE
    } else {
        let mut approx = Decimal::ZERO;
        let mut v = x;
        let e_approx = dec!(2.718281828459045);
        if x > Decimal::ONE {
            while v > e_approx {
                v /= e_approx;
                approx += Decimal::ONE;
            }
            approx + (v - Decimal::ONE)
        } else {
            while v < Decimal::ONE / e_approx {
                v *= e_approx;
                approx -= Decimal::ONE;
            }
            approx + (v - Decimal::ONE)
        }
    };
    for _ in 0..40 {
        let ey = exp_decimal(y);
        if ey == Decimal::ZERO {
            break;
        }
        y = y - Decimal::ONE + x / ey;
    }
    y
}

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

/// Direction of a trade.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TradeSide {
    Buy,
    Sell,
    Unknown,
}

/// Method for spread decomposition.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SpreadMethod {
    /// Simple quoted spread (ask - bid)
    Quoted,
    /// Effective spread: 2 * |trade_price - midpoint| * direction
    Effective,
    /// Effective spread adjusted for price impact
    Realized,
    /// Roll (1984) implied spread from autocovariance
    RollModel,
    /// Kyle lambda (price impact coefficient)
    KyleModel,
}

/// A single trade observation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeRecord {
    /// Epoch milliseconds
    pub timestamp: u64,
    pub price: Decimal,
    pub volume: Decimal,
    pub side: TradeSide,
}

/// A single quote (NBBO) observation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuoteRecord {
    /// Epoch milliseconds
    pub timestamp: u64,
    pub bid_price: Decimal,
    pub ask_price: Decimal,
    pub bid_size: Decimal,
    pub ask_size: Decimal,
}

/// Input for spread analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpreadAnalysisInput {
    pub security_name: String,
    /// Time-sequenced trade data
    pub trade_data: Vec<TradeRecord>,
    /// Time-sequenced quote data (NBBO)
    pub quote_data: Vec<QuoteRecord>,
    /// Decomposition method
    pub analysis_method: SpreadMethod,
    /// Benchmark spread for comparison (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub benchmark_spread: Option<Decimal>,
    /// Average daily volume
    pub daily_volume: Decimal,
    /// Market capitalisation (for liquidity scoring)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub market_cap: Option<Decimal>,
}

/// A spread metric with descriptive statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpreadMetric {
    /// Spread in price units
    pub absolute: Decimal,
    /// Spread in basis points relative to midpoint
    pub relative: Decimal,
    /// Median spread
    pub median: Decimal,
    /// Standard deviation
    pub std_dev: Decimal,
    /// 95th percentile
    pub percentile_95: Decimal,
}

/// Three-way spread decomposition (Huang-Stoll 1997 simplified).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpreadDecomposition {
    /// Informed trading component (bps)
    pub adverse_selection: Decimal,
    /// Market maker cost component (bps)
    pub order_processing: Decimal,
    /// Inventory holding cost component (bps)
    pub inventory: Decimal,
    /// Sum of components
    pub total: Decimal,
    /// Adverse selection as percentage of total
    pub adverse_selection_pct: Decimal,
    /// Description of method used
    pub method: String,
}

/// Full output of the spread analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpreadAnalysisOutput {
    /// Average quoted spread
    pub quoted_spread: SpreadMetric,
    /// Average effective spread
    pub effective_spread: SpreadMetric,
    /// Realized spread (only if method = Realized)
    pub realized_spread: Option<SpreadMetric>,
    /// Permanent price impact component
    pub price_impact: SpreadMetric,
    /// Roll (1984) implied spread
    pub roll_spread: Option<Decimal>,
    /// Kyle price impact coefficient
    pub kyle_lambda: Option<Decimal>,
    /// Huang-Stoll decomposition
    pub spread_decomposition: SpreadDecomposition,
    /// Composite liquidity score 0-100
    pub liquidity_score: Decimal,
    /// Amihud (2002) illiquidity ratio
    pub amihud_illiquidity: Decimal,
    /// Volume-weighted average spread
    pub volume_weighted_spread: Decimal,
    /// (ask_size - bid_size) / (ask_size + bid_size)
    pub depth_imbalance: Decimal,
    /// Number of trades
    pub trade_count: u32,
    /// Average trade size
    pub avg_trade_size: Decimal,
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Build a SpreadMetric from a vector of values and corresponding midpoints.
fn build_metric(values: &[Decimal], midpoints: &[Decimal]) -> SpreadMetric {
    if values.is_empty() {
        return SpreadMetric {
            absolute: Decimal::ZERO,
            relative: Decimal::ZERO,
            median: Decimal::ZERO,
            std_dev: Decimal::ZERO,
            percentile_95: Decimal::ZERO,
        };
    }
    let n = Decimal::from(values.len() as u32);
    let sum: Decimal = values.iter().copied().sum();
    let mean_abs = sum / n;

    // Relative: average of value/mid * 10000 bps
    let mean_rel = if midpoints.is_empty() {
        Decimal::ZERO
    } else {
        let rel_sum: Decimal = values
            .iter()
            .zip(midpoints.iter())
            .map(|(v, m)| {
                if *m != Decimal::ZERO {
                    *v / *m * dec!(10000)
                } else {
                    Decimal::ZERO
                }
            })
            .sum();
        rel_sum / n
    };

    // Median
    let mut sorted = values.to_vec();
    sorted.sort();
    let median = if sorted.len().is_multiple_of(2) {
        let mid = sorted.len() / 2;
        (sorted[mid - 1] + sorted[mid]) / dec!(2)
    } else {
        sorted[sorted.len() / 2]
    };

    // Std dev
    let var_sum: Decimal = values
        .iter()
        .map(|v| (*v - mean_abs) * (*v - mean_abs))
        .sum();
    let std_dev = if values.len() > 1 {
        sqrt_decimal(var_sum / Decimal::from(values.len() as u32 - 1))
    } else {
        Decimal::ZERO
    };

    // 95th percentile
    let p95_idx = ((sorted.len() as f64) * 0.95).ceil() as usize;
    let p95_idx = if p95_idx == 0 {
        0
    } else {
        p95_idx.min(sorted.len()) - 1
    };
    let percentile_95 = sorted[p95_idx];

    SpreadMetric {
        absolute: mean_abs,
        relative: mean_rel,
        median,
        std_dev,
        percentile_95,
    }
}

/// Given trade data, find the prevailing quote at or before a trade's timestamp.
fn find_quote_at(quotes: &[QuoteRecord], ts: u64) -> Option<&QuoteRecord> {
    // Binary search for the latest quote <= ts
    let mut best: Option<&QuoteRecord> = None;
    let mut lo = 0usize;
    let mut hi = quotes.len();
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        if quotes[mid].timestamp <= ts {
            best = Some(&quotes[mid]);
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    best
}

/// Lee-Ready tick test: classify trade direction from price changes.
/// Returns +1 for buy, -1 for sell.
fn lee_ready_classify(
    trade_price: Decimal,
    prev_trade_price: Option<Decimal>,
    quote: Option<&QuoteRecord>,
) -> Decimal {
    // First: tick test
    if let Some(prev) = prev_trade_price {
        if trade_price > prev {
            return Decimal::ONE;
        }
        if trade_price < prev {
            return -Decimal::ONE;
        }
    }
    // If same price or no previous, use quote midpoint
    if let Some(q) = quote {
        let mid = (q.bid_price + q.ask_price) / dec!(2);
        if trade_price > mid {
            return Decimal::ONE;
        }
        if trade_price < mid {
            return -Decimal::ONE;
        }
    }
    // Default: assume buy
    Decimal::ONE
}

/// Compute trade direction sign: +1 for buy, -1 for sell.
fn direction_sign(
    trade: &TradeRecord,
    prev_price: Option<Decimal>,
    quote: Option<&QuoteRecord>,
) -> Decimal {
    match trade.side {
        TradeSide::Buy => Decimal::ONE,
        TradeSide::Sell => -Decimal::ONE,
        TradeSide::Unknown => lee_ready_classify(trade.price, prev_price, quote),
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Analyse bid-ask spreads and market quality metrics.
pub fn analyze_spreads(
    input: &SpreadAnalysisInput,
) -> CorpFinanceResult<ComputationOutput<SpreadAnalysisOutput>> {
    let start = Instant::now();
    let mut warnings: Vec<String> = Vec::new();

    // -- Validation --
    if input.trade_data.is_empty() {
        return Err(CorpFinanceError::InsufficientData(
            "trade_data must contain at least one record".into(),
        ));
    }
    if input.quote_data.is_empty() {
        return Err(CorpFinanceError::InsufficientData(
            "quote_data must contain at least one record".into(),
        ));
    }
    if input.daily_volume <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "daily_volume".into(),
            reason: "must be positive".into(),
        });
    }

    let trades = &input.trade_data;
    let quotes = &input.quote_data;

    // -----------------------------------------------------------------------
    // 1. Quoted spread
    // -----------------------------------------------------------------------
    let mut quoted_values: Vec<Decimal> = Vec::with_capacity(quotes.len());
    let mut quoted_midpoints: Vec<Decimal> = Vec::with_capacity(quotes.len());
    for q in quotes {
        let spread = q.ask_price - q.bid_price;
        let mid = (q.bid_price + q.ask_price) / dec!(2);
        quoted_values.push(spread);
        quoted_midpoints.push(mid);
    }
    let quoted_spread = build_metric(&quoted_values, &quoted_midpoints);

    // -----------------------------------------------------------------------
    // 2. Effective spread, price impact, realized spread
    // -----------------------------------------------------------------------
    let mut eff_values: Vec<Decimal> = Vec::new();
    let mut eff_midpoints: Vec<Decimal> = Vec::new();
    let mut impact_values: Vec<Decimal> = Vec::new();
    let mut impact_midpoints: Vec<Decimal> = Vec::new();
    let mut realized_values: Vec<Decimal> = Vec::new();
    let mut realized_midpoints: Vec<Decimal> = Vec::new();

    let mut prev_price: Option<Decimal> = None;
    let trade_count = trades.len() as u32;

    for (i, t) in trades.iter().enumerate() {
        let q = find_quote_at(quotes, t.timestamp);
        let mid = q
            .map(|qq| (qq.bid_price + qq.ask_price) / dec!(2))
            .unwrap_or(t.price);

        let dir = direction_sign(t, prev_price, q);
        let eff = dec!(2) * abs_decimal(t.price - mid);
        eff_values.push(eff);
        eff_midpoints.push(mid);

        // Price impact: direction * (mid_{t+delta} - mid_t)
        // Use next trade's prevailing midpoint as mid_{t+delta}
        let next_mid = if i + 1 < trades.len() {
            find_quote_at(quotes, trades[i + 1].timestamp)
                .map(|qq| (qq.bid_price + qq.ask_price) / dec!(2))
                .unwrap_or(mid)
        } else {
            mid
        };

        let pi = dir * (next_mid - mid);
        impact_values.push(abs_decimal(pi));
        impact_midpoints.push(mid);

        // Realized spread = effective - 2*price_impact
        let rs = eff - dec!(2) * abs_decimal(pi);
        realized_values.push(rs);
        realized_midpoints.push(mid);

        prev_price = Some(t.price);
    }

    let effective_spread = build_metric(&eff_values, &eff_midpoints);
    let price_impact = build_metric(&impact_values, &impact_midpoints);

    let realized_spread = if input.analysis_method == SpreadMethod::Realized {
        Some(build_metric(&realized_values, &realized_midpoints))
    } else {
        None
    };

    // -----------------------------------------------------------------------
    // 3. Roll (1984) implied spread
    // -----------------------------------------------------------------------
    let roll_spread = if trades.len() >= 3 {
        // Compute price changes
        let mut delta_p: Vec<Decimal> = Vec::with_capacity(trades.len() - 1);
        for i in 1..trades.len() {
            delta_p.push(trades[i].price - trades[i - 1].price);
        }
        // Autocovariance of consecutive price changes
        if delta_p.len() >= 2 {
            let n_dp = delta_p.len();
            let mean_dp: Decimal =
                delta_p.iter().copied().sum::<Decimal>() / Decimal::from(n_dp as u32);
            let mut cov_sum = Decimal::ZERO;
            for i in 1..n_dp {
                cov_sum += (delta_p[i] - mean_dp) * (delta_p[i - 1] - mean_dp);
            }
            let cov = cov_sum / Decimal::from((n_dp - 1) as u32);
            if cov < Decimal::ZERO {
                Some(dec!(2) * sqrt_decimal(-cov))
            } else {
                warnings.push(
                    "Positive autocovariance in price changes; Roll spread set to zero".into(),
                );
                Some(Decimal::ZERO)
            }
        } else {
            None
        }
    } else {
        None
    };

    // -----------------------------------------------------------------------
    // 4. Kyle lambda
    // -----------------------------------------------------------------------
    let kyle_lambda = if trades.len() >= 3 {
        // delta_p and signed_volume
        let mut dp: Vec<Decimal> = Vec::new();
        let mut sv: Vec<Decimal> = Vec::new();
        let mut prev_p: Option<Decimal> = None;
        for t in trades {
            if let Some(pp) = prev_p {
                let q = find_quote_at(quotes, t.timestamp);
                let dir = direction_sign(t, Some(pp), q);
                dp.push(t.price - pp);
                sv.push(dir * t.volume);
            }
            prev_p = Some(t.price);
        }
        if dp.len() >= 2 {
            let n = Decimal::from(dp.len() as u32);
            let mean_dp: Decimal = dp.iter().copied().sum::<Decimal>() / n;
            let mean_sv: Decimal = sv.iter().copied().sum::<Decimal>() / n;
            let mut cov_dp_sv = Decimal::ZERO;
            let mut var_sv = Decimal::ZERO;
            for i in 0..dp.len() {
                cov_dp_sv += (dp[i] - mean_dp) * (sv[i] - mean_sv);
                var_sv += (sv[i] - mean_sv) * (sv[i] - mean_sv);
            }
            if var_sv != Decimal::ZERO {
                Some(cov_dp_sv / var_sv)
            } else {
                warnings.push("Zero variance in signed volume; Kyle lambda undefined".into());
                None
            }
        } else {
            None
        }
    } else {
        None
    };

    // -----------------------------------------------------------------------
    // 5. Spread decomposition (Huang-Stoll 1997 simplified)
    // -----------------------------------------------------------------------
    let adverse_selection_abs = price_impact.relative; // permanent component in bps
    let total_decomp = effective_spread.relative;
    let order_processing_abs = if !realized_values.is_empty() {
        let rs_metric = build_metric(&realized_values, &realized_midpoints);
        rs_metric.relative
    } else {
        Decimal::ZERO
    };
    let inventory_abs = total_decomp - adverse_selection_abs - order_processing_abs;
    let inventory_abs = if inventory_abs < Decimal::ZERO {
        Decimal::ZERO
    } else {
        inventory_abs
    };
    let adverse_selection_pct = if total_decomp != Decimal::ZERO {
        adverse_selection_abs / total_decomp * dec!(100)
    } else {
        Decimal::ZERO
    };

    let spread_decomposition = SpreadDecomposition {
        adverse_selection: adverse_selection_abs,
        order_processing: order_processing_abs,
        inventory: inventory_abs,
        total: adverse_selection_abs + order_processing_abs + inventory_abs,
        adverse_selection_pct,
        method: "Huang-Stoll 1997 (simplified)".into(),
    };

    // -----------------------------------------------------------------------
    // 6. Amihud illiquidity ratio: avg(|r_t| / volume_t)
    // -----------------------------------------------------------------------
    let amihud_illiquidity = if trades.len() >= 2 {
        let mut amihud_sum = Decimal::ZERO;
        let mut amihud_count = 0u32;
        for i in 1..trades.len() {
            let prev = trades[i - 1].price;
            if prev > Decimal::ZERO && trades[i].volume > Decimal::ZERO {
                let log_ret = abs_decimal(ln_decimal(trades[i].price / prev));
                amihud_sum += log_ret / trades[i].volume;
                amihud_count += 1;
            }
        }
        if amihud_count > 0 {
            amihud_sum / Decimal::from(amihud_count)
        } else {
            Decimal::ZERO
        }
    } else {
        Decimal::ZERO
    };

    // -----------------------------------------------------------------------
    // 7. Volume-weighted spread
    // -----------------------------------------------------------------------
    let volume_weighted_spread = {
        let mut vw_sum = Decimal::ZERO;
        let mut vol_sum = Decimal::ZERO;
        for t in trades {
            if let Some(q) = find_quote_at(quotes, t.timestamp) {
                let spread = q.ask_price - q.bid_price;
                vw_sum += spread * t.volume;
                vol_sum += t.volume;
            }
        }
        if vol_sum > Decimal::ZERO {
            vw_sum / vol_sum
        } else {
            Decimal::ZERO
        }
    };

    // -----------------------------------------------------------------------
    // 8. Depth imbalance
    // -----------------------------------------------------------------------
    let depth_imbalance = {
        let mut total_ask_size = Decimal::ZERO;
        let mut total_bid_size = Decimal::ZERO;
        for q in quotes {
            total_ask_size += q.ask_size;
            total_bid_size += q.bid_size;
        }
        let denom = total_ask_size + total_bid_size;
        if denom > Decimal::ZERO {
            (total_ask_size - total_bid_size) / denom
        } else {
            Decimal::ZERO
        }
    };

    // -----------------------------------------------------------------------
    // 9. Average trade size
    // -----------------------------------------------------------------------
    let total_volume: Decimal = trades.iter().map(|t| t.volume).sum();
    let avg_trade_size = if trade_count > 0 {
        total_volume / Decimal::from(trade_count)
    } else {
        Decimal::ZERO
    };

    // -----------------------------------------------------------------------
    // 10. Liquidity score: composite 0-100
    //     spread(30%), depth(25%), volume(25%), impact(20%)
    // -----------------------------------------------------------------------
    let liquidity_score = {
        // Spread component: lower is better. Score = max(0, 100 - relative_spread_bps)
        let spread_score = {
            let s = dec!(100) - effective_spread.relative;
            if s < Decimal::ZERO {
                Decimal::ZERO
            } else if s > dec!(100) {
                dec!(100)
            } else {
                s
            }
        };

        // Depth component: ratio of total quote size to daily volume
        let depth_score = {
            let total_quote_size: Decimal = quotes
                .iter()
                .map(|q| q.bid_size + q.ask_size)
                .sum::<Decimal>()
                / Decimal::from(quotes.len().max(1) as u32);
            let ratio = if input.daily_volume > Decimal::ZERO {
                total_quote_size / input.daily_volume * dec!(10000)
            } else {
                Decimal::ZERO
            };
            if ratio > dec!(100) {
                dec!(100)
            } else {
                ratio
            }
        };

        // Volume component: trade_count / expected (use 1000 as normalizer)
        let volume_score = {
            let ratio = Decimal::from(trade_count) / dec!(1000) * dec!(100);
            if ratio > dec!(100) {
                dec!(100)
            } else {
                ratio
            }
        };

        // Impact component: lower price impact is better
        let impact_score = {
            let s = dec!(100) - price_impact.relative * dec!(2);
            if s < Decimal::ZERO {
                Decimal::ZERO
            } else if s > dec!(100) {
                dec!(100)
            } else {
                s
            }
        };

        let composite = spread_score * dec!(0.30)
            + depth_score * dec!(0.25)
            + volume_score * dec!(0.25)
            + impact_score * dec!(0.20);
        if composite > dec!(100) {
            dec!(100)
        } else if composite < Decimal::ZERO {
            Decimal::ZERO
        } else {
            composite
        }
    };

    // -----------------------------------------------------------------------
    // Build output
    // -----------------------------------------------------------------------
    let output = SpreadAnalysisOutput {
        quoted_spread,
        effective_spread,
        realized_spread,
        price_impact,
        roll_spread,
        kyle_lambda,
        spread_decomposition,
        liquidity_score,
        amihud_illiquidity,
        volume_weighted_spread,
        depth_imbalance,
        trade_count,
        avg_trade_size,
    };

    let elapsed = start.elapsed().as_micros() as u64;
    Ok(with_metadata(
        "Spread Analysis (Quoted/Effective/Roll/Kyle/Huang-Stoll)",
        &serde_json::json!({
            "method": format!("{:?}", input.analysis_method),
            "trade_count": trade_count,
            "quote_count": quotes.len(),
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

    // Helper to build a simple set of quotes
    fn sample_quotes() -> Vec<QuoteRecord> {
        vec![
            QuoteRecord {
                timestamp: 1000,
                bid_price: dec!(99.90),
                ask_price: dec!(100.10),
                bid_size: dec!(500),
                ask_size: dec!(500),
            },
            QuoteRecord {
                timestamp: 2000,
                bid_price: dec!(99.95),
                ask_price: dec!(100.05),
                bid_size: dec!(600),
                ask_size: dec!(400),
            },
            QuoteRecord {
                timestamp: 3000,
                bid_price: dec!(100.00),
                ask_price: dec!(100.20),
                bid_size: dec!(400),
                ask_size: dec!(600),
            },
            QuoteRecord {
                timestamp: 4000,
                bid_price: dec!(99.85),
                ask_price: dec!(100.15),
                bid_size: dec!(500),
                ask_size: dec!(500),
            },
            QuoteRecord {
                timestamp: 5000,
                bid_price: dec!(99.90),
                ask_price: dec!(100.10),
                bid_size: dec!(700),
                ask_size: dec!(300),
            },
        ]
    }

    fn sample_trades() -> Vec<TradeRecord> {
        vec![
            TradeRecord {
                timestamp: 1100,
                price: dec!(100.05),
                volume: dec!(100),
                side: TradeSide::Buy,
            },
            TradeRecord {
                timestamp: 2100,
                price: dec!(99.97),
                volume: dec!(200),
                side: TradeSide::Sell,
            },
            TradeRecord {
                timestamp: 3100,
                price: dec!(100.15),
                volume: dec!(150),
                side: TradeSide::Buy,
            },
            TradeRecord {
                timestamp: 4100,
                price: dec!(99.90),
                volume: dec!(300),
                side: TradeSide::Sell,
            },
            TradeRecord {
                timestamp: 5100,
                price: dec!(100.02),
                volume: dec!(100),
                side: TradeSide::Buy,
            },
        ]
    }

    fn basic_input() -> SpreadAnalysisInput {
        SpreadAnalysisInput {
            security_name: "TEST".into(),
            trade_data: sample_trades(),
            quote_data: sample_quotes(),
            analysis_method: SpreadMethod::Quoted,
            benchmark_spread: None,
            daily_volume: dec!(100000),
            market_cap: Some(dec!(1000000000)),
        }
    }

    // --- Validation tests ---

    #[test]
    fn test_empty_trade_data() {
        let mut input = basic_input();
        input.trade_data = vec![];
        let result = analyze_spreads(&input);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("trade_data"));
    }

    #[test]
    fn test_empty_quote_data() {
        let mut input = basic_input();
        input.quote_data = vec![];
        let result = analyze_spreads(&input);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("quote_data"));
    }

    #[test]
    fn test_zero_daily_volume() {
        let mut input = basic_input();
        input.daily_volume = Decimal::ZERO;
        let result = analyze_spreads(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_negative_daily_volume() {
        let mut input = basic_input();
        input.daily_volume = dec!(-100);
        let result = analyze_spreads(&input);
        assert!(result.is_err());
    }

    // --- Quoted spread tests ---

    #[test]
    fn test_quoted_spread_positive() {
        let input = basic_input();
        let result = analyze_spreads(&input).unwrap();
        assert!(result.result.quoted_spread.absolute > Decimal::ZERO);
    }

    #[test]
    fn test_quoted_spread_relative_in_bps() {
        let input = basic_input();
        let result = analyze_spreads(&input).unwrap();
        // Relative should be in basis points
        assert!(result.result.quoted_spread.relative > Decimal::ZERO);
        assert!(result.result.quoted_spread.relative < dec!(10000));
    }

    #[test]
    fn test_quoted_spread_single_quote() {
        let mut input = basic_input();
        input.quote_data = vec![QuoteRecord {
            timestamp: 1000,
            bid_price: dec!(99.00),
            ask_price: dec!(101.00),
            bid_size: dec!(100),
            ask_size: dec!(100),
        }];
        let result = analyze_spreads(&input).unwrap();
        assert_eq!(result.result.quoted_spread.absolute, dec!(2.00));
    }

    #[test]
    fn test_quoted_spread_median() {
        let input = basic_input();
        let result = analyze_spreads(&input).unwrap();
        // Median should be >= 0
        assert!(result.result.quoted_spread.median >= Decimal::ZERO);
    }

    #[test]
    fn test_quoted_spread_percentile_95() {
        let input = basic_input();
        let result = analyze_spreads(&input).unwrap();
        // 95th percentile should be >= median
        assert!(result.result.quoted_spread.percentile_95 >= result.result.quoted_spread.median);
    }

    // --- Effective spread tests ---

    #[test]
    fn test_effective_spread_positive() {
        let input = basic_input();
        let result = analyze_spreads(&input).unwrap();
        assert!(result.result.effective_spread.absolute > Decimal::ZERO);
    }

    #[test]
    fn test_effective_spread_with_known_sides() {
        let input = basic_input();
        let result = analyze_spreads(&input).unwrap();
        // Effective spread should be roughly 2 * |price - mid|
        assert!(result.result.effective_spread.absolute > Decimal::ZERO);
    }

    #[test]
    fn test_effective_spread_unknown_side_uses_tick_test() {
        let mut input = basic_input();
        // Set all trades to Unknown side
        for t in input.trade_data.iter_mut() {
            t.side = TradeSide::Unknown;
        }
        let result = analyze_spreads(&input).unwrap();
        assert!(result.result.effective_spread.absolute > Decimal::ZERO);
    }

    // --- Price impact tests ---

    #[test]
    fn test_price_impact_computed() {
        let input = basic_input();
        let result = analyze_spreads(&input).unwrap();
        // Price impact should be non-negative on average
        assert!(result.result.price_impact.absolute >= Decimal::ZERO);
    }

    #[test]
    fn test_price_impact_relative_in_bps() {
        let input = basic_input();
        let result = analyze_spreads(&input).unwrap();
        assert!(result.result.price_impact.relative >= Decimal::ZERO);
    }

    // --- Realized spread tests ---

    #[test]
    fn test_realized_spread_none_for_quoted_method() {
        let input = basic_input();
        let result = analyze_spreads(&input).unwrap();
        assert!(result.result.realized_spread.is_none());
    }

    #[test]
    fn test_realized_spread_some_for_realized_method() {
        let mut input = basic_input();
        input.analysis_method = SpreadMethod::Realized;
        let result = analyze_spreads(&input).unwrap();
        assert!(result.result.realized_spread.is_some());
    }

    #[test]
    fn test_realized_spread_values() {
        let mut input = basic_input();
        input.analysis_method = SpreadMethod::Realized;
        let result = analyze_spreads(&input).unwrap();
        let rs = result.result.realized_spread.unwrap();
        // Realized spread can be negative (when impact > spread)
        assert!(rs.std_dev >= Decimal::ZERO);
    }

    // --- Roll model tests ---

    #[test]
    fn test_roll_spread_computed() {
        let input = basic_input();
        let result = analyze_spreads(&input).unwrap();
        assert!(result.result.roll_spread.is_some());
    }

    #[test]
    fn test_roll_spread_non_negative() {
        let input = basic_input();
        let result = analyze_spreads(&input).unwrap();
        if let Some(rs) = result.result.roll_spread {
            assert!(rs >= Decimal::ZERO);
        }
    }

    #[test]
    fn test_roll_spread_two_trades_returns_none() {
        let mut input = basic_input();
        input.trade_data = input.trade_data[..2].to_vec();
        let result = analyze_spreads(&input).unwrap();
        // With only 2 trades we get 1 delta, which is < 2 so None
        assert!(result.result.roll_spread.is_none());
    }

    #[test]
    fn test_roll_positive_autocovariance_zero() {
        // Create trades with strictly increasing prices (positive autocov)
        let mut input = basic_input();
        input.trade_data = vec![
            TradeRecord {
                timestamp: 1100,
                price: dec!(100.00),
                volume: dec!(100),
                side: TradeSide::Buy,
            },
            TradeRecord {
                timestamp: 2100,
                price: dec!(100.10),
                volume: dec!(100),
                side: TradeSide::Buy,
            },
            TradeRecord {
                timestamp: 3100,
                price: dec!(100.20),
                volume: dec!(100),
                side: TradeSide::Buy,
            },
            TradeRecord {
                timestamp: 4100,
                price: dec!(100.30),
                volume: dec!(100),
                side: TradeSide::Buy,
            },
            TradeRecord {
                timestamp: 5100,
                price: dec!(100.40),
                volume: dec!(100),
                side: TradeSide::Buy,
            },
        ];
        let result = analyze_spreads(&input).unwrap();
        if let Some(rs) = result.result.roll_spread {
            assert_eq!(rs, Decimal::ZERO);
        }
    }

    // --- Kyle lambda tests ---

    #[test]
    fn test_kyle_lambda_computed() {
        let input = basic_input();
        let result = analyze_spreads(&input).unwrap();
        assert!(result.result.kyle_lambda.is_some());
    }

    #[test]
    fn test_kyle_lambda_with_two_trades() {
        let mut input = basic_input();
        input.trade_data = input.trade_data[..2].to_vec();
        let result = analyze_spreads(&input).unwrap();
        // 2 trades => 1 pair, which is < 2
        assert!(result.result.kyle_lambda.is_none());
    }

    // --- Spread decomposition tests ---

    #[test]
    fn test_decomposition_sums_to_total() {
        let input = basic_input();
        let result = analyze_spreads(&input).unwrap();
        let d = &result.result.spread_decomposition;
        let sum = d.adverse_selection + d.order_processing + d.inventory;
        let diff = abs_decimal(sum - d.total);
        assert!(
            diff < dec!(0.01),
            "decomposition sum mismatch: {} vs {}",
            sum,
            d.total
        );
    }

    #[test]
    fn test_decomposition_method_label() {
        let input = basic_input();
        let result = analyze_spreads(&input).unwrap();
        assert!(result
            .result
            .spread_decomposition
            .method
            .contains("Huang-Stoll"));
    }

    #[test]
    fn test_decomposition_adverse_selection_pct_range() {
        let input = basic_input();
        let result = analyze_spreads(&input).unwrap();
        let pct = result.result.spread_decomposition.adverse_selection_pct;
        assert!(pct >= Decimal::ZERO && pct <= dec!(100));
    }

    #[test]
    fn test_decomposition_non_negative_inventory() {
        let input = basic_input();
        let result = analyze_spreads(&input).unwrap();
        assert!(result.result.spread_decomposition.inventory >= Decimal::ZERO);
    }

    // --- Amihud illiquidity tests ---

    #[test]
    fn test_amihud_positive() {
        let input = basic_input();
        let result = analyze_spreads(&input).unwrap();
        assert!(result.result.amihud_illiquidity >= Decimal::ZERO);
    }

    #[test]
    fn test_amihud_single_trade() {
        let mut input = basic_input();
        input.trade_data = vec![input.trade_data[0].clone()];
        let result = analyze_spreads(&input).unwrap();
        assert_eq!(result.result.amihud_illiquidity, Decimal::ZERO);
    }

    // --- Volume-weighted spread tests ---

    #[test]
    fn test_vw_spread_positive() {
        let input = basic_input();
        let result = analyze_spreads(&input).unwrap();
        assert!(result.result.volume_weighted_spread > Decimal::ZERO);
    }

    // --- Depth imbalance tests ---

    #[test]
    fn test_depth_imbalance_range() {
        let input = basic_input();
        let result = analyze_spreads(&input).unwrap();
        let di = result.result.depth_imbalance;
        assert!(di >= dec!(-1) && di <= dec!(1));
    }

    #[test]
    fn test_depth_imbalance_zero_when_symmetric() {
        let mut input = basic_input();
        input.quote_data = vec![QuoteRecord {
            timestamp: 1000,
            bid_price: dec!(99.90),
            ask_price: dec!(100.10),
            bid_size: dec!(500),
            ask_size: dec!(500),
        }];
        let result = analyze_spreads(&input).unwrap();
        assert_eq!(result.result.depth_imbalance, Decimal::ZERO);
    }

    #[test]
    fn test_depth_imbalance_positive_when_ask_heavy() {
        let mut input = basic_input();
        input.quote_data = vec![QuoteRecord {
            timestamp: 1000,
            bid_price: dec!(99.90),
            ask_price: dec!(100.10),
            bid_size: dec!(100),
            ask_size: dec!(900),
        }];
        let result = analyze_spreads(&input).unwrap();
        assert!(result.result.depth_imbalance > Decimal::ZERO);
    }

    // --- Trade count and avg trade size ---

    #[test]
    fn test_trade_count() {
        let input = basic_input();
        let result = analyze_spreads(&input).unwrap();
        assert_eq!(result.result.trade_count, 5);
    }

    #[test]
    fn test_avg_trade_size() {
        let input = basic_input();
        let result = analyze_spreads(&input).unwrap();
        // (100+200+150+300+100)/5 = 170
        assert_eq!(result.result.avg_trade_size, dec!(170));
    }

    // --- Liquidity score tests ---

    #[test]
    fn test_liquidity_score_range() {
        let input = basic_input();
        let result = analyze_spreads(&input).unwrap();
        let ls = result.result.liquidity_score;
        assert!(ls >= Decimal::ZERO && ls <= dec!(100));
    }

    #[test]
    fn test_liquidity_score_better_with_tighter_spread() {
        // Create two inputs: one with tight spread, one with wide
        let mut tight = basic_input();
        tight.quote_data = vec![QuoteRecord {
            timestamp: 1000,
            bid_price: dec!(99.99),
            ask_price: dec!(100.01),
            bid_size: dec!(1000),
            ask_size: dec!(1000),
        }];

        let mut wide = basic_input();
        wide.quote_data = vec![QuoteRecord {
            timestamp: 1000,
            bid_price: dec!(99.00),
            ask_price: dec!(101.00),
            bid_size: dec!(1000),
            ask_size: dec!(1000),
        }];

        let tight_result = analyze_spreads(&tight).unwrap();
        let wide_result = analyze_spreads(&wide).unwrap();
        assert!(tight_result.result.liquidity_score >= wide_result.result.liquidity_score);
    }

    // --- Metadata tests ---

    #[test]
    fn test_methodology_string() {
        let input = basic_input();
        let result = analyze_spreads(&input).unwrap();
        assert!(result.methodology.contains("Spread Analysis"));
    }

    #[test]
    fn test_computation_time_recorded() {
        let input = basic_input();
        let result = analyze_spreads(&input).unwrap();
        // Computation time should be recorded (>= 0)
        assert!(result.metadata.computation_time_us < 10_000_000);
    }

    // --- Math helper tests ---

    #[test]
    fn test_sqrt_decimal_perfect_square() {
        let s = sqrt_decimal(dec!(4));
        let diff = abs_decimal(s - dec!(2));
        assert!(diff < dec!(0.0001));
    }

    #[test]
    fn test_sqrt_decimal_zero() {
        assert_eq!(sqrt_decimal(Decimal::ZERO), Decimal::ZERO);
    }

    #[test]
    fn test_sqrt_decimal_one() {
        assert_eq!(sqrt_decimal(Decimal::ONE), Decimal::ONE);
    }

    #[test]
    fn test_sqrt_decimal_small() {
        let s = sqrt_decimal(dec!(0.0001));
        let diff = abs_decimal(s - dec!(0.01));
        assert!(diff < dec!(0.0001));
    }

    #[test]
    fn test_ln_decimal_one() {
        assert_eq!(ln_decimal(Decimal::ONE), Decimal::ZERO);
    }

    #[test]
    fn test_ln_decimal_e() {
        let e = dec!(2.718281828459045);
        let result = ln_decimal(e);
        let diff = abs_decimal(result - Decimal::ONE);
        assert!(diff < dec!(0.001));
    }

    #[test]
    fn test_exp_decimal_zero() {
        assert_eq!(exp_decimal(Decimal::ZERO), Decimal::ONE);
    }

    #[test]
    fn test_exp_ln_roundtrip() {
        let x = dec!(3.5);
        let result = ln_decimal(exp_decimal(x));
        let diff = abs_decimal(result - x);
        assert!(diff < dec!(0.01));
    }

    // --- Serialization ---

    #[test]
    fn test_output_serializable() {
        let input = basic_input();
        let result = analyze_spreads(&input).unwrap();
        let json = serde_json::to_string(&result).unwrap();
        assert!(!json.is_empty());
    }

    // --- Edge cases ---

    #[test]
    fn test_single_trade_single_quote() {
        let input = SpreadAnalysisInput {
            security_name: "EDGE".into(),
            trade_data: vec![TradeRecord {
                timestamp: 1100,
                price: dec!(100.05),
                volume: dec!(100),
                side: TradeSide::Buy,
            }],
            quote_data: vec![QuoteRecord {
                timestamp: 1000,
                bid_price: dec!(99.90),
                ask_price: dec!(100.10),
                bid_size: dec!(500),
                ask_size: dec!(500),
            }],
            analysis_method: SpreadMethod::Quoted,
            benchmark_spread: Some(dec!(0.20)),
            daily_volume: dec!(50000),
            market_cap: None,
        };
        let result = analyze_spreads(&input).unwrap();
        assert_eq!(result.result.trade_count, 1);
        assert_eq!(result.result.avg_trade_size, dec!(100));
    }

    #[test]
    fn test_all_unknown_sides() {
        let mut input = basic_input();
        for t in input.trade_data.iter_mut() {
            t.side = TradeSide::Unknown;
        }
        let result = analyze_spreads(&input).unwrap();
        assert!(result.result.effective_spread.absolute > Decimal::ZERO);
    }

    #[test]
    fn test_effective_method() {
        let mut input = basic_input();
        input.analysis_method = SpreadMethod::Effective;
        let result = analyze_spreads(&input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_roll_method() {
        let mut input = basic_input();
        input.analysis_method = SpreadMethod::RollModel;
        let result = analyze_spreads(&input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_kyle_method() {
        let mut input = basic_input();
        input.analysis_method = SpreadMethod::KyleModel;
        let result = analyze_spreads(&input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_many_trades_stress() {
        let mut input = basic_input();
        // Generate 100 trades alternating buy/sell
        let mut trades = Vec::new();
        for i in 0..100 {
            trades.push(TradeRecord {
                timestamp: 1000 + i * 100,
                price: dec!(100) + Decimal::from(i % 10) * dec!(0.01),
                volume: dec!(50) + Decimal::from(i % 5) * dec!(10),
                side: if i % 2 == 0 {
                    TradeSide::Buy
                } else {
                    TradeSide::Sell
                },
            });
        }
        input.trade_data = trades;
        let result = analyze_spreads(&input).unwrap();
        assert_eq!(result.result.trade_count, 100);
    }

    #[test]
    fn test_wide_spread_low_liquidity() {
        let input = SpreadAnalysisInput {
            security_name: "WIDE".into(),
            trade_data: vec![
                TradeRecord {
                    timestamp: 1100,
                    price: dec!(105.00),
                    volume: dec!(10),
                    side: TradeSide::Buy,
                },
                TradeRecord {
                    timestamp: 2100,
                    price: dec!(95.00),
                    volume: dec!(10),
                    side: TradeSide::Sell,
                },
                TradeRecord {
                    timestamp: 3100,
                    price: dec!(104.00),
                    volume: dec!(10),
                    side: TradeSide::Buy,
                },
            ],
            quote_data: vec![QuoteRecord {
                timestamp: 1000,
                bid_price: dec!(90.00),
                ask_price: dec!(110.00),
                bid_size: dec!(10),
                ask_size: dec!(10),
            }],
            analysis_method: SpreadMethod::Quoted,
            benchmark_spread: None,
            daily_volume: dec!(100),
            market_cap: None,
        };
        let result = analyze_spreads(&input).unwrap();
        // Wide spread should result in lower liquidity score
        assert!(result.result.liquidity_score < dec!(80));
    }

    #[test]
    fn test_abs_decimal_positive() {
        assert_eq!(abs_decimal(dec!(5)), dec!(5));
    }

    #[test]
    fn test_abs_decimal_negative() {
        assert_eq!(abs_decimal(dec!(-5)), dec!(5));
    }

    #[test]
    fn test_abs_decimal_zero() {
        assert_eq!(abs_decimal(Decimal::ZERO), Decimal::ZERO);
    }
}
