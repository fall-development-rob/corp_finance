use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Instant;

use crate::error::CorpFinanceError;
use crate::types::{with_metadata, ComputationOutput};
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Types -- Best Execution & Transaction Cost Analysis
// ---------------------------------------------------------------------------

/// A single executed trade with price benchmarks and cost information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeExecution {
    pub trade_id: String,
    pub security: String,
    /// "Buy" or "Sell"
    pub side: String,
    pub quantity: Decimal,
    /// Price when the investment decision was made.
    pub decision_price: Decimal,
    /// Price when the order arrived at the market.
    pub arrival_price: Decimal,
    /// Actual fill price.
    pub execution_price: Decimal,
    /// Volume-weighted average price for the day.
    pub vwap_price: Decimal,
    /// Time-weighted average price for the day.
    pub twap_price: Decimal,
    /// Closing price.
    pub close_price: Decimal,
    /// Explicit costs (commissions + fees).
    pub commission: Decimal,
    /// Estimated market impact.
    pub market_impact_estimate: Decimal,
    /// Total order size.
    pub order_size: Decimal,
    /// Order as percentage of average daily volume.
    pub adv_pct: Decimal,
}

/// Input for best execution analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BestExecutionInput {
    pub trades: Vec<TradeExecution>,
    /// Benchmark type: "VWAP", "TWAP", "ArrivalPrice", or "Close".
    pub benchmark: String,
    pub reporting_currency: String,
}

/// Transaction cost analysis result for a single trade.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TcaResult {
    pub trade_id: String,
    pub security: String,
    /// Total cost vs decision price (Perold implementation shortfall).
    pub implementation_shortfall: Decimal,
    pub implementation_shortfall_bps: Decimal,
    /// Commissions and fees.
    pub explicit_costs: Decimal,
    pub explicit_costs_bps: Decimal,
    /// Spread + impact (implicit).
    pub implicit_costs: Decimal,
    pub implicit_costs_bps: Decimal,
    /// Arrival vs decision price.
    pub delay_cost: Decimal,
    pub delay_cost_bps: Decimal,
    /// Execution vs arrival price.
    pub market_impact: Decimal,
    pub market_impact_bps: Decimal,
    /// Close vs execution price (opportunity cost).
    pub timing_cost: Decimal,
    pub timing_cost_bps: Decimal,
    /// Execution vs chosen benchmark.
    pub benchmark_deviation: Decimal,
    pub benchmark_deviation_bps: Decimal,
    /// "Excellent", "Good", "Average", or "Poor".
    pub execution_quality: String,
}

/// Portfolio-level TCA summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioTcaSummary {
    pub total_notional: Decimal,
    pub total_explicit_costs: Decimal,
    pub total_implicit_costs: Decimal,
    pub total_implementation_shortfall: Decimal,
    pub avg_shortfall_bps: Decimal,
    pub avg_market_impact_bps: Decimal,
    /// Percentage of trades beating VWAP.
    pub pct_improved_vs_vwap: Decimal,
    /// Percentage of trades beating arrival price.
    pub pct_improved_vs_arrival: Decimal,
}

/// MiFID II compliance assessment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MifidCompliance {
    pub best_execution_achieved: bool,
    /// Execution within acceptable range of benchmark.
    pub price_compliance: bool,
    /// Assessed via delay cost.
    pub speed_compliance: bool,
    /// Total cost within thresholds.
    pub cost_compliance: bool,
    /// Fill rate assessment (assumed 100% for completed trades).
    pub likelihood_of_execution: bool,
    /// Weighted score 0-100.
    pub overall_score: Decimal,
    pub deficiencies: Vec<String>,
}

/// Full best execution analysis output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BestExecutionOutput {
    pub trade_results: Vec<TcaResult>,
    pub portfolio_summary: PortfolioTcaSummary,
    /// 0-100 overall execution score.
    pub execution_score: Decimal,
    pub mifid_compliance: MifidCompliance,
    pub methodology: String,
    pub assumptions: HashMap<String, String>,
    pub warnings: Vec<String>,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Convert an absolute amount to basis points relative to notional.
fn to_bps(amount: Decimal, notional: Decimal) -> Decimal {
    if notional == dec!(0) {
        Decimal::ZERO
    } else {
        amount / notional * dec!(10000)
    }
}

/// Determine benchmark price for a trade.
fn benchmark_price(trade: &TradeExecution, benchmark: &str) -> Decimal {
    match benchmark {
        "VWAP" => trade.vwap_price,
        "TWAP" => trade.twap_price,
        "ArrivalPrice" => trade.arrival_price,
        "Close" => trade.close_price,
        _ => trade.vwap_price,
    }
}

/// Classify execution quality based on benchmark deviation in bps.
fn execution_quality_rating(deviation_bps: Decimal) -> String {
    if deviation_bps < dec!(-5) {
        "Excellent".to_string()
    } else if deviation_bps <= dec!(5) {
        "Good".to_string()
    } else if deviation_bps <= dec!(20) {
        "Average".to_string()
    } else {
        "Poor".to_string()
    }
}

/// Analyze a single trade and return the TCA result.
fn analyze_trade(trade: &TradeExecution, benchmark: &str) -> TcaResult {
    let is_buy = trade.side == "Buy";
    let qty = trade.quantity;
    let notional = trade.decision_price * qty;

    // Implementation Shortfall (Perold)
    let implementation_shortfall = if is_buy {
        (trade.execution_price - trade.decision_price) * qty + trade.commission
    } else {
        (trade.decision_price - trade.execution_price) * qty + trade.commission
    };
    let implementation_shortfall_bps = to_bps(implementation_shortfall, notional);

    // Explicit costs
    let explicit_costs = trade.commission;
    let explicit_costs_bps = to_bps(explicit_costs, notional);

    // Delay cost
    let delay_cost = if is_buy {
        (trade.arrival_price - trade.decision_price) * qty
    } else {
        (trade.decision_price - trade.arrival_price) * qty
    };
    let delay_cost_bps = to_bps(delay_cost, notional);

    // Market impact
    let market_impact = if is_buy {
        (trade.execution_price - trade.arrival_price) * qty
    } else {
        (trade.arrival_price - trade.execution_price) * qty
    };
    let market_impact_bps = to_bps(market_impact, notional);

    // Timing cost
    let timing_cost = if is_buy {
        (trade.close_price - trade.execution_price) * qty
    } else {
        (trade.execution_price - trade.close_price) * qty
    };
    let timing_cost_bps = to_bps(timing_cost, notional);

    // Implicit costs = delay + market impact
    let implicit_costs = delay_cost + market_impact;
    let implicit_costs_bps = to_bps(implicit_costs, notional);

    // Benchmark deviation
    let bench = benchmark_price(trade, benchmark);
    let benchmark_deviation_price = if is_buy {
        trade.execution_price - bench
    } else {
        bench - trade.execution_price
    };
    let benchmark_deviation = benchmark_deviation_price * qty;
    let benchmark_deviation_bps = to_bps(benchmark_deviation, notional);

    let execution_quality = execution_quality_rating(benchmark_deviation_bps);

    TcaResult {
        trade_id: trade.trade_id.clone(),
        security: trade.security.clone(),
        implementation_shortfall,
        implementation_shortfall_bps,
        explicit_costs,
        explicit_costs_bps,
        implicit_costs,
        implicit_costs_bps,
        delay_cost,
        delay_cost_bps,
        market_impact,
        market_impact_bps,
        timing_cost,
        timing_cost_bps,
        benchmark_deviation,
        benchmark_deviation_bps,
        execution_quality,
    }
}

// ---------------------------------------------------------------------------
// Public function: analyze_best_execution
// ---------------------------------------------------------------------------

/// Perform MiFID II best execution analysis and transaction cost analysis (TCA).
///
/// Computes implementation shortfall (Perold decomposition), benchmark deviation,
/// MiFID II compliance scoring, and portfolio-level summary statistics.
pub fn analyze_best_execution(
    input: &BestExecutionInput,
) -> CorpFinanceResult<ComputationOutput<BestExecutionOutput>> {
    let start = Instant::now();
    let mut warnings: Vec<String> = Vec::new();

    // --- Validation ---
    if input.trades.is_empty() {
        return Err(CorpFinanceError::InvalidInput {
            field: "trades".to_string(),
            reason: "At least one trade is required".to_string(),
        });
    }

    let valid_benchmarks = ["VWAP", "TWAP", "ArrivalPrice", "Close"];
    if !valid_benchmarks.contains(&input.benchmark.as_str()) {
        return Err(CorpFinanceError::InvalidInput {
            field: "benchmark".to_string(),
            reason: format!(
                "Benchmark must be one of: VWAP, TWAP, ArrivalPrice, Close. Got '{}'",
                input.benchmark
            ),
        });
    }

    for (i, trade) in input.trades.iter().enumerate() {
        if trade.quantity <= dec!(0) {
            return Err(CorpFinanceError::InvalidInput {
                field: format!("trades[{}].quantity", i),
                reason: "Quantity must be positive".to_string(),
            });
        }
        if trade.decision_price <= dec!(0) {
            return Err(CorpFinanceError::InvalidInput {
                field: format!("trades[{}].decision_price", i),
                reason: "Decision price must be positive".to_string(),
            });
        }
        if trade.arrival_price <= dec!(0) {
            return Err(CorpFinanceError::InvalidInput {
                field: format!("trades[{}].arrival_price", i),
                reason: "Arrival price must be positive".to_string(),
            });
        }
        if trade.execution_price <= dec!(0) {
            return Err(CorpFinanceError::InvalidInput {
                field: format!("trades[{}].execution_price", i),
                reason: "Execution price must be positive".to_string(),
            });
        }
        if trade.vwap_price <= dec!(0) {
            return Err(CorpFinanceError::InvalidInput {
                field: format!("trades[{}].vwap_price", i),
                reason: "VWAP price must be positive".to_string(),
            });
        }
        if trade.twap_price <= dec!(0) {
            return Err(CorpFinanceError::InvalidInput {
                field: format!("trades[{}].twap_price", i),
                reason: "TWAP price must be positive".to_string(),
            });
        }
        if trade.close_price <= dec!(0) {
            return Err(CorpFinanceError::InvalidInput {
                field: format!("trades[{}].close_price", i),
                reason: "Close price must be positive".to_string(),
            });
        }
        if trade.side != "Buy" && trade.side != "Sell" {
            return Err(CorpFinanceError::InvalidInput {
                field: format!("trades[{}].side", i),
                reason: "Side must be 'Buy' or 'Sell'".to_string(),
            });
        }
    }

    // --- Analyze each trade ---
    let trade_results: Vec<TcaResult> = input
        .trades
        .iter()
        .map(|t| analyze_trade(t, &input.benchmark))
        .collect();

    // --- Portfolio summary ---
    let total_notional: Decimal = input
        .trades
        .iter()
        .map(|t| t.decision_price * t.quantity)
        .sum();

    let total_explicit_costs: Decimal = trade_results.iter().map(|r| r.explicit_costs).sum();
    let total_implicit_costs: Decimal = trade_results.iter().map(|r| r.implicit_costs).sum();
    let total_implementation_shortfall: Decimal = trade_results
        .iter()
        .map(|r| r.implementation_shortfall)
        .sum();

    let avg_shortfall_bps = if total_notional == dec!(0) {
        Decimal::ZERO
    } else {
        total_implementation_shortfall / total_notional * dec!(10000)
    };

    let total_market_impact: Decimal = trade_results.iter().map(|r| r.market_impact).sum();
    let avg_market_impact_bps = if total_notional == dec!(0) {
        Decimal::ZERO
    } else {
        total_market_impact / total_notional * dec!(10000)
    };

    // Count trades beating VWAP
    let n_trades = Decimal::from(input.trades.len() as u64);
    let beats_vwap = input
        .trades
        .iter()
        .filter(|t| {
            if t.side == "Buy" {
                t.execution_price <= t.vwap_price
            } else {
                t.execution_price >= t.vwap_price
            }
        })
        .count();
    let pct_improved_vs_vwap = if n_trades == dec!(0) {
        Decimal::ZERO
    } else {
        Decimal::from(beats_vwap as u64) / n_trades * dec!(100)
    };

    // Count trades beating arrival
    let beats_arrival = input
        .trades
        .iter()
        .filter(|t| {
            if t.side == "Buy" {
                t.execution_price <= t.arrival_price
            } else {
                t.execution_price >= t.arrival_price
            }
        })
        .count();
    let pct_improved_vs_arrival = if n_trades == dec!(0) {
        Decimal::ZERO
    } else {
        Decimal::from(beats_arrival as u64) / n_trades * dec!(100)
    };

    let portfolio_summary = PortfolioTcaSummary {
        total_notional,
        total_explicit_costs,
        total_implicit_costs,
        total_implementation_shortfall,
        avg_shortfall_bps,
        avg_market_impact_bps,
        pct_improved_vs_vwap,
        pct_improved_vs_arrival,
    };

    // --- MiFID II Compliance ---
    let mut deficiencies: Vec<String> = Vec::new();

    // Price compliance: avg benchmark deviation within +/-20 bps
    let avg_bench_dev_bps = if trade_results.is_empty() {
        Decimal::ZERO
    } else {
        let sum_dev: Decimal = trade_results
            .iter()
            .map(|r| r.benchmark_deviation_bps)
            .sum();
        sum_dev / Decimal::from(trade_results.len() as u64)
    };
    let price_compliance = avg_bench_dev_bps.abs() <= dec!(20);
    if !price_compliance {
        deficiencies.push(format!(
            "Price: avg benchmark deviation {:.1} bps exceeds +/-20 bps threshold",
            avg_bench_dev_bps
        ));
    }

    // Speed compliance: avg delay cost < 10 bps
    let avg_delay_bps = if trade_results.is_empty() {
        Decimal::ZERO
    } else {
        let sum_delay: Decimal = trade_results.iter().map(|r| r.delay_cost_bps).sum();
        sum_delay / Decimal::from(trade_results.len() as u64)
    };
    let speed_compliance = avg_delay_bps.abs() < dec!(10);
    if !speed_compliance {
        deficiencies.push(format!(
            "Speed: avg delay cost {:.1} bps exceeds 10 bps threshold",
            avg_delay_bps
        ));
    }

    // Cost compliance: avg IS < 30 bps
    let cost_compliance = avg_shortfall_bps.abs() < dec!(30);
    if !cost_compliance {
        deficiencies.push(format!(
            "Cost: avg implementation shortfall {:.1} bps exceeds 30 bps threshold",
            avg_shortfall_bps
        ));
    }

    // Likelihood of execution: assumed 100% for completed trades
    let likelihood_of_execution = true;

    // Overall score: weighted average (price 40%, cost 30%, speed 20%, likelihood 10%)
    let price_score = if price_compliance {
        dec!(100)
    } else {
        let excess = (avg_bench_dev_bps.abs() - dec!(20)).max(dec!(0));
        (dec!(100) - excess * dec!(2)).max(dec!(0))
    };
    let cost_score = if cost_compliance {
        dec!(100)
    } else {
        let excess = (avg_shortfall_bps.abs() - dec!(30)).max(dec!(0));
        (dec!(100) - excess * dec!(2)).max(dec!(0))
    };
    let speed_score = if speed_compliance {
        dec!(100)
    } else {
        let excess = (avg_delay_bps.abs() - dec!(10)).max(dec!(0));
        (dec!(100) - excess * dec!(2)).max(dec!(0))
    };
    let likelihood_score = dec!(100);

    let overall_score = price_score * dec!(0.4)
        + cost_score * dec!(0.3)
        + speed_score * dec!(0.2)
        + likelihood_score * dec!(0.1);

    let best_execution_achieved = price_compliance && cost_compliance && speed_compliance;

    let mifid_compliance = MifidCompliance {
        best_execution_achieved,
        price_compliance,
        speed_compliance,
        cost_compliance,
        likelihood_of_execution,
        overall_score,
        deficiencies,
    };

    // --- Execution score ---
    let execution_score = (dec!(100) - avg_shortfall_bps.abs().min(dec!(100))).max(dec!(0));

    // --- Warnings ---
    for trade in &input.trades {
        if trade.adv_pct > dec!(20) {
            warnings.push(format!(
                "Trade {} has high ADV% ({:.1}%), expect elevated market impact",
                trade.trade_id, trade.adv_pct
            ));
        }
    }

    let mut assumptions = HashMap::new();
    assumptions.insert("benchmark".to_string(), input.benchmark.clone());
    assumptions.insert("currency".to_string(), input.reporting_currency.clone());
    assumptions.insert(
        "is_decomposition".to_string(),
        "Perold (1988) implementation shortfall".to_string(),
    );
    assumptions.insert("mifid_price_threshold_bps".to_string(), "20".to_string());
    assumptions.insert("mifid_cost_threshold_bps".to_string(), "30".to_string());
    assumptions.insert("mifid_speed_threshold_bps".to_string(), "10".to_string());

    let output = BestExecutionOutput {
        trade_results,
        portfolio_summary,
        execution_score,
        mifid_compliance,
        methodology: "Perold Implementation Shortfall with MiFID II best execution assessment"
            .to_string(),
        assumptions,
        warnings: warnings.clone(),
    };

    let elapsed = start.elapsed().as_micros() as u64;
    let assumptions_ser = HashMap::from([
        ("benchmark", input.benchmark.as_str()),
        ("is_method", "Perold (1988)"),
        ("mifid_ii", "RTS 27/28 framework"),
    ]);

    Ok(with_metadata(
        "Perold Implementation Shortfall + MiFID II Best Execution",
        &assumptions_ser,
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

    /// Build a default buy trade for testing.
    fn make_buy_trade() -> TradeExecution {
        TradeExecution {
            trade_id: "T001".to_string(),
            security: "AAPL".to_string(),
            side: "Buy".to_string(),
            quantity: dec!(1000),
            decision_price: dec!(100),
            arrival_price: dec!(100.10),
            execution_price: dec!(100.20),
            vwap_price: dec!(100.15),
            twap_price: dec!(100.12),
            close_price: dec!(100.50),
            commission: dec!(10),
            market_impact_estimate: dec!(5),
            order_size: dec!(1000),
            adv_pct: dec!(2),
        }
    }

    /// Build a default sell trade for testing.
    fn make_sell_trade() -> TradeExecution {
        TradeExecution {
            trade_id: "T002".to_string(),
            security: "MSFT".to_string(),
            side: "Sell".to_string(),
            quantity: dec!(500),
            decision_price: dec!(200),
            arrival_price: dec!(199.90),
            execution_price: dec!(199.80),
            vwap_price: dec!(199.85),
            twap_price: dec!(199.88),
            close_price: dec!(199.50),
            commission: dec!(8),
            market_impact_estimate: dec!(3),
            order_size: dec!(500),
            adv_pct: dec!(1),
        }
    }

    fn make_input(trades: Vec<TradeExecution>, benchmark: &str) -> BestExecutionInput {
        BestExecutionInput {
            trades,
            benchmark: benchmark.to_string(),
            reporting_currency: "USD".to_string(),
        }
    }

    // -----------------------------------------------------------------------
    // Single trade tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_single_buy_trade_is() {
        let input = make_input(vec![make_buy_trade()], "VWAP");
        let result = analyze_best_execution(&input).unwrap();
        let tca = &result.result.trade_results[0];
        // IS = (100.20 - 100) * 1000 + 10 = 200 + 10 = 210
        assert_eq!(tca.implementation_shortfall, dec!(210));
    }

    #[test]
    fn test_single_buy_trade_is_bps() {
        let input = make_input(vec![make_buy_trade()], "VWAP");
        let result = analyze_best_execution(&input).unwrap();
        let tca = &result.result.trade_results[0];
        // notional = 100 * 1000 = 100000
        // IS bps = 210 / 100000 * 10000 = 21
        assert_eq!(tca.implementation_shortfall_bps, dec!(21));
    }

    #[test]
    fn test_single_buy_trade_explicit_costs() {
        let input = make_input(vec![make_buy_trade()], "VWAP");
        let result = analyze_best_execution(&input).unwrap();
        let tca = &result.result.trade_results[0];
        assert_eq!(tca.explicit_costs, dec!(10));
        assert_eq!(tca.explicit_costs_bps, dec!(1));
    }

    #[test]
    fn test_single_buy_trade_delay_cost() {
        let input = make_input(vec![make_buy_trade()], "VWAP");
        let result = analyze_best_execution(&input).unwrap();
        let tca = &result.result.trade_results[0];
        // delay = (100.10 - 100) * 1000 = 100
        assert_eq!(tca.delay_cost, dec!(100));
        assert_eq!(tca.delay_cost_bps, dec!(10));
    }

    #[test]
    fn test_single_buy_trade_market_impact() {
        let input = make_input(vec![make_buy_trade()], "VWAP");
        let result = analyze_best_execution(&input).unwrap();
        let tca = &result.result.trade_results[0];
        // market impact = (100.20 - 100.10) * 1000 = 100
        assert_eq!(tca.market_impact, dec!(100));
        assert_eq!(tca.market_impact_bps, dec!(10));
    }

    #[test]
    fn test_single_buy_trade_timing_cost() {
        let input = make_input(vec![make_buy_trade()], "VWAP");
        let result = analyze_best_execution(&input).unwrap();
        let tca = &result.result.trade_results[0];
        // timing = (100.50 - 100.20) * 1000 = 300
        assert_eq!(tca.timing_cost, dec!(300));
        assert_eq!(tca.timing_cost_bps, dec!(30));
    }

    #[test]
    fn test_single_buy_trade_implicit_costs() {
        let input = make_input(vec![make_buy_trade()], "VWAP");
        let result = analyze_best_execution(&input).unwrap();
        let tca = &result.result.trade_results[0];
        // implicit = delay + market_impact = 100 + 100 = 200
        assert_eq!(tca.implicit_costs, dec!(200));
        assert_eq!(tca.implicit_costs_bps, dec!(20));
    }

    #[test]
    fn test_single_sell_trade_is() {
        let input = make_input(vec![make_sell_trade()], "VWAP");
        let result = analyze_best_execution(&input).unwrap();
        let tca = &result.result.trade_results[0];
        // IS sell = (200 - 199.80) * 500 + 8 = 100 + 8 = 108
        assert_eq!(tca.implementation_shortfall, dec!(108));
    }

    #[test]
    fn test_single_sell_trade_delay_cost() {
        let input = make_input(vec![make_sell_trade()], "VWAP");
        let result = analyze_best_execution(&input).unwrap();
        let tca = &result.result.trade_results[0];
        // delay sell = (200 - 199.90) * 500 = 50
        assert_eq!(tca.delay_cost, dec!(50));
    }

    #[test]
    fn test_single_sell_trade_market_impact() {
        let input = make_input(vec![make_sell_trade()], "VWAP");
        let result = analyze_best_execution(&input).unwrap();
        let tca = &result.result.trade_results[0];
        // market impact sell = (199.90 - 199.80) * 500 = 50
        assert_eq!(tca.market_impact, dec!(50));
    }

    #[test]
    fn test_single_sell_trade_timing_cost() {
        let input = make_input(vec![make_sell_trade()], "VWAP");
        let result = analyze_best_execution(&input).unwrap();
        let tca = &result.result.trade_results[0];
        // timing sell = (199.80 - 199.50) * 500 = 150
        assert_eq!(tca.timing_cost, dec!(150));
    }

    #[test]
    fn test_vwap_benchmark_deviation_buy() {
        let input = make_input(vec![make_buy_trade()], "VWAP");
        let result = analyze_best_execution(&input).unwrap();
        let tca = &result.result.trade_results[0];
        // buy deviation = (100.20 - 100.15) * 1000 = 50
        assert_eq!(tca.benchmark_deviation, dec!(50));
        assert_eq!(tca.benchmark_deviation_bps, dec!(5));
    }

    #[test]
    fn test_arrival_price_benchmark_deviation_buy() {
        let input = make_input(vec![make_buy_trade()], "ArrivalPrice");
        let result = analyze_best_execution(&input).unwrap();
        let tca = &result.result.trade_results[0];
        // buy deviation vs arrival = (100.20 - 100.10) * 1000 = 100
        assert_eq!(tca.benchmark_deviation, dec!(100));
        assert_eq!(tca.benchmark_deviation_bps, dec!(10));
    }

    #[test]
    fn test_twap_benchmark_deviation_buy() {
        let input = make_input(vec![make_buy_trade()], "TWAP");
        let result = analyze_best_execution(&input).unwrap();
        let tca = &result.result.trade_results[0];
        // buy deviation vs twap = (100.20 - 100.12) * 1000 = 80
        assert_eq!(tca.benchmark_deviation, dec!(80));
        assert_eq!(tca.benchmark_deviation_bps, dec!(8));
    }

    #[test]
    fn test_close_benchmark_deviation_buy() {
        let input = make_input(vec![make_buy_trade()], "Close");
        let result = analyze_best_execution(&input).unwrap();
        let tca = &result.result.trade_results[0];
        // buy deviation vs close = (100.20 - 100.50) * 1000 = -300
        assert_eq!(tca.benchmark_deviation, dec!(-300));
        assert_eq!(tca.benchmark_deviation_bps, dec!(-30));
    }

    #[test]
    fn test_zero_commission_trade() {
        let mut trade = make_buy_trade();
        trade.commission = dec!(0);
        let input = make_input(vec![trade], "VWAP");
        let result = analyze_best_execution(&input).unwrap();
        let tca = &result.result.trade_results[0];
        // IS = (100.20 - 100) * 1000 + 0 = 200
        assert_eq!(tca.implementation_shortfall, dec!(200));
        assert_eq!(tca.explicit_costs, dec!(0));
        assert_eq!(tca.explicit_costs_bps, dec!(0));
    }

    #[test]
    fn test_negative_implementation_shortfall_outperformance() {
        let mut trade = make_buy_trade();
        trade.execution_price = dec!(99.90);
        trade.arrival_price = dec!(99.95);
        trade.commission = dec!(5);
        let input = make_input(vec![trade], "VWAP");
        let result = analyze_best_execution(&input).unwrap();
        let tca = &result.result.trade_results[0];
        // IS = (99.90 - 100) * 1000 + 5 = -100 + 5 = -95
        assert_eq!(tca.implementation_shortfall, dec!(-95));
    }

    #[test]
    fn test_delay_cost_positive_price_moved_against_buy() {
        // Arrival price higher than decision price => positive delay cost for buy
        let trade = make_buy_trade(); // arrival=100.10, decision=100
        let input = make_input(vec![trade], "VWAP");
        let result = analyze_best_execution(&input).unwrap();
        let tca = &result.result.trade_results[0];
        assert!(tca.delay_cost > dec!(0));
    }

    #[test]
    fn test_execution_quality_excellent() {
        let mut trade = make_buy_trade();
        trade.execution_price = dec!(100.10);
        trade.vwap_price = dec!(100.20);
        let input = make_input(vec![trade], "VWAP");
        let result = analyze_best_execution(&input).unwrap();
        let tca = &result.result.trade_results[0];
        // deviation = (100.10 - 100.20) * 1000 = -100 => -10 bps => Excellent
        assert_eq!(tca.execution_quality, "Excellent");
    }

    #[test]
    fn test_execution_quality_good() {
        let mut trade = make_buy_trade();
        trade.execution_price = dec!(100.15);
        trade.vwap_price = dec!(100.15);
        let input = make_input(vec![trade], "VWAP");
        let result = analyze_best_execution(&input).unwrap();
        let tca = &result.result.trade_results[0];
        // deviation = 0 bps => Good
        assert_eq!(tca.execution_quality, "Good");
    }

    #[test]
    fn test_execution_quality_average() {
        let mut trade = make_buy_trade();
        trade.execution_price = dec!(100.25);
        trade.vwap_price = dec!(100.15);
        let input = make_input(vec![trade], "VWAP");
        let result = analyze_best_execution(&input).unwrap();
        let tca = &result.result.trade_results[0];
        // deviation = (100.25 - 100.15) * 1000 = 100 => 10 bps => Average
        assert_eq!(tca.execution_quality, "Average");
    }

    #[test]
    fn test_execution_quality_poor() {
        let mut trade = make_buy_trade();
        trade.execution_price = dec!(100.50);
        trade.vwap_price = dec!(100.15);
        let input = make_input(vec![trade], "VWAP");
        let result = analyze_best_execution(&input).unwrap();
        let tca = &result.result.trade_results[0];
        // deviation = (100.50 - 100.15) * 1000 = 350 => 35 bps => Poor
        assert_eq!(tca.execution_quality, "Poor");
    }

    // -----------------------------------------------------------------------
    // Multi-trade / portfolio tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_multi_trade_portfolio_summary() {
        let input = make_input(vec![make_buy_trade(), make_sell_trade()], "VWAP");
        let result = analyze_best_execution(&input).unwrap();
        let summary = &result.result.portfolio_summary;
        // buy notional = 100 * 1000 = 100000; sell notional = 200 * 500 = 100000
        assert_eq!(summary.total_notional, dec!(200000));
        assert_eq!(result.result.trade_results.len(), 2);
    }

    #[test]
    fn test_portfolio_total_explicit_costs() {
        let input = make_input(vec![make_buy_trade(), make_sell_trade()], "VWAP");
        let result = analyze_best_execution(&input).unwrap();
        let summary = &result.result.portfolio_summary;
        // 10 + 8 = 18
        assert_eq!(summary.total_explicit_costs, dec!(18));
    }

    #[test]
    fn test_portfolio_total_implicit_costs() {
        let input = make_input(vec![make_buy_trade(), make_sell_trade()], "VWAP");
        let result = analyze_best_execution(&input).unwrap();
        let summary = &result.result.portfolio_summary;
        // buy implicit = 200, sell implicit = 100
        assert_eq!(summary.total_implicit_costs, dec!(300));
    }

    #[test]
    fn test_portfolio_total_is() {
        let input = make_input(vec![make_buy_trade(), make_sell_trade()], "VWAP");
        let result = analyze_best_execution(&input).unwrap();
        let summary = &result.result.portfolio_summary;
        // buy IS = 210, sell IS = 108
        assert_eq!(summary.total_implementation_shortfall, dec!(318));
    }

    #[test]
    fn test_portfolio_avg_shortfall_bps() {
        let input = make_input(vec![make_buy_trade(), make_sell_trade()], "VWAP");
        let result = analyze_best_execution(&input).unwrap();
        let summary = &result.result.portfolio_summary;
        // 318 / 200000 * 10000 = 15.9
        assert_eq!(summary.avg_shortfall_bps, dec!(15.9));
    }

    #[test]
    fn test_portfolio_pct_improved_vs_vwap() {
        // buy: exec 100.20 > vwap 100.15 => does not beat
        // sell: exec 199.80 < vwap 199.85 => does not beat
        let input = make_input(vec![make_buy_trade(), make_sell_trade()], "VWAP");
        let result = analyze_best_execution(&input).unwrap();
        let summary = &result.result.portfolio_summary;
        assert_eq!(summary.pct_improved_vs_vwap, dec!(0));
    }

    #[test]
    fn test_portfolio_pct_improved_vs_arrival() {
        // buy: exec 100.20 > arrival 100.10 => does not beat
        // sell: exec 199.80 < arrival 199.90 => does not beat
        let input = make_input(vec![make_buy_trade(), make_sell_trade()], "VWAP");
        let result = analyze_best_execution(&input).unwrap();
        let summary = &result.result.portfolio_summary;
        assert_eq!(summary.pct_improved_vs_arrival, dec!(0));
    }

    #[test]
    fn test_portfolio_with_trade_beating_vwap() {
        let mut trade = make_buy_trade();
        trade.execution_price = dec!(100.10); // below vwap of 100.15
        let input = make_input(vec![trade], "VWAP");
        let result = analyze_best_execution(&input).unwrap();
        assert_eq!(
            result.result.portfolio_summary.pct_improved_vs_vwap,
            dec!(100)
        );
    }

    #[test]
    fn test_mixed_buy_sell_portfolio() {
        let buy = make_buy_trade();
        let sell = make_sell_trade();
        let input = make_input(vec![buy, sell], "VWAP");
        let result = analyze_best_execution(&input).unwrap();
        assert_eq!(result.result.trade_results[0].security, "AAPL");
        assert_eq!(result.result.trade_results[1].security, "MSFT");
    }

    // -----------------------------------------------------------------------
    // IS decomposition
    // -----------------------------------------------------------------------

    #[test]
    fn test_is_decomposition_sums_correctly_buy() {
        let input = make_input(vec![make_buy_trade()], "VWAP");
        let result = analyze_best_execution(&input).unwrap();
        let tca = &result.result.trade_results[0];
        // IS ~ delay + market_impact + explicit_costs
        let decomposed = tca.delay_cost + tca.market_impact + tca.explicit_costs;
        assert_eq!(tca.implementation_shortfall, decomposed);
    }

    #[test]
    fn test_is_decomposition_sums_correctly_sell() {
        let input = make_input(vec![make_sell_trade()], "VWAP");
        let result = analyze_best_execution(&input).unwrap();
        let tca = &result.result.trade_results[0];
        let decomposed = tca.delay_cost + tca.market_impact + tca.explicit_costs;
        assert_eq!(tca.implementation_shortfall, decomposed);
    }

    // -----------------------------------------------------------------------
    // MiFID compliance tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_mifid_compliance_pass() {
        // Small costs => all pass
        let mut trade = make_buy_trade();
        trade.execution_price = dec!(100.05);
        trade.arrival_price = dec!(100.02);
        trade.commission = dec!(5);
        let input = make_input(vec![trade], "VWAP");
        let result = analyze_best_execution(&input).unwrap();
        let mifid = &result.result.mifid_compliance;
        assert!(mifid.price_compliance);
        assert!(mifid.speed_compliance);
        assert!(mifid.cost_compliance);
        assert!(mifid.best_execution_achieved);
    }

    #[test]
    fn test_mifid_compliance_price_fail() {
        let mut trade = make_buy_trade();
        // benchmark deviation > 20 bps
        trade.execution_price = dec!(100.50);
        trade.vwap_price = dec!(100.10);
        let input = make_input(vec![trade], "VWAP");
        let result = analyze_best_execution(&input).unwrap();
        let mifid = &result.result.mifid_compliance;
        // deviation = (100.50 - 100.10) * 1000 = 400 => 40 bps => fail
        assert!(!mifid.price_compliance);
        assert!(!mifid.best_execution_achieved);
    }

    #[test]
    fn test_mifid_compliance_speed_fail() {
        let mut trade = make_buy_trade();
        trade.arrival_price = dec!(101.50);
        let input = make_input(vec![trade], "VWAP");
        let result = analyze_best_execution(&input).unwrap();
        let mifid = &result.result.mifid_compliance;
        // delay = (101.50 - 100) * 1000 = 1500 => 150 bps => fail
        assert!(!mifid.speed_compliance);
    }

    #[test]
    fn test_mifid_compliance_cost_fail() {
        let mut trade = make_buy_trade();
        trade.execution_price = dec!(100.40);
        trade.commission = dec!(50);
        let input = make_input(vec![trade], "VWAP");
        let result = analyze_best_execution(&input).unwrap();
        let mifid = &result.result.mifid_compliance;
        // IS = (100.40 - 100) * 1000 + 50 = 450 => 45 bps => fail
        assert!(!mifid.cost_compliance);
    }

    #[test]
    fn test_mifid_likelihood_always_true() {
        let input = make_input(vec![make_buy_trade()], "VWAP");
        let result = analyze_best_execution(&input).unwrap();
        assert!(result.result.mifid_compliance.likelihood_of_execution);
    }

    #[test]
    fn test_mifid_overall_score_perfect() {
        let mut trade = make_buy_trade();
        trade.execution_price = dec!(100.01);
        trade.arrival_price = dec!(100.005);
        trade.commission = dec!(1);
        trade.vwap_price = dec!(100.01);
        let input = make_input(vec![trade], "VWAP");
        let result = analyze_best_execution(&input).unwrap();
        assert_eq!(result.result.mifid_compliance.overall_score, dec!(100));
    }

    #[test]
    fn test_mifid_deficiencies_populated() {
        let mut trade = make_buy_trade();
        trade.execution_price = dec!(100.50);
        trade.arrival_price = dec!(101.00);
        trade.commission = dec!(50);
        let input = make_input(vec![trade], "VWAP");
        let result = analyze_best_execution(&input).unwrap();
        assert!(!result.result.mifid_compliance.deficiencies.is_empty());
    }

    // -----------------------------------------------------------------------
    // Execution score
    // -----------------------------------------------------------------------

    #[test]
    fn test_execution_score_low_cost() {
        let mut trade = make_buy_trade();
        trade.execution_price = dec!(100.01);
        trade.commission = dec!(1);
        let input = make_input(vec![trade], "VWAP");
        let result = analyze_best_execution(&input).unwrap();
        // avg_shortfall_bps ~ 2 => score ~ 98
        assert!(result.result.execution_score > dec!(90));
    }

    #[test]
    fn test_execution_score_capped_at_zero() {
        let mut trade = make_buy_trade();
        trade.execution_price = dec!(112);
        trade.commission = dec!(100);
        let input = make_input(vec![trade], "VWAP");
        let result = analyze_best_execution(&input).unwrap();
        assert!(result.result.execution_score >= dec!(0));
    }

    // -----------------------------------------------------------------------
    // Edge cases
    // -----------------------------------------------------------------------

    #[test]
    fn test_equal_prices_zero_cost() {
        let mut trade = make_buy_trade();
        trade.decision_price = dec!(100);
        trade.arrival_price = dec!(100);
        trade.execution_price = dec!(100);
        trade.vwap_price = dec!(100);
        trade.twap_price = dec!(100);
        trade.close_price = dec!(100);
        trade.commission = dec!(0);
        let input = make_input(vec![trade], "VWAP");
        let result = analyze_best_execution(&input).unwrap();
        let tca = &result.result.trade_results[0];
        assert_eq!(tca.implementation_shortfall, dec!(0));
        assert_eq!(tca.delay_cost, dec!(0));
        assert_eq!(tca.market_impact, dec!(0));
        assert_eq!(tca.timing_cost, dec!(0));
        assert_eq!(tca.benchmark_deviation, dec!(0));
    }

    #[test]
    fn test_high_adv_pct_warning() {
        let mut trade = make_buy_trade();
        trade.adv_pct = dec!(25);
        let input = make_input(vec![trade], "VWAP");
        let result = analyze_best_execution(&input).unwrap();
        assert!(result
            .result
            .warnings
            .iter()
            .any(|w| w.contains("high ADV%")));
    }

    #[test]
    fn test_large_order() {
        let mut trade = make_buy_trade();
        trade.quantity = dec!(1000000);
        trade.order_size = dec!(1000000);
        trade.adv_pct = dec!(50);
        let input = make_input(vec![trade], "VWAP");
        let result = analyze_best_execution(&input).unwrap();
        // Just confirm it computes without overflow
        assert!(!result.result.trade_results.is_empty());
    }

    #[test]
    fn test_small_order() {
        let mut trade = make_buy_trade();
        trade.quantity = dec!(1);
        let input = make_input(vec![trade], "VWAP");
        let result = analyze_best_execution(&input).unwrap();
        assert!(!result.result.trade_results.is_empty());
    }

    #[test]
    fn test_extreme_market_impact() {
        let mut trade = make_buy_trade();
        trade.execution_price = dec!(110);
        let input = make_input(vec![trade], "VWAP");
        let result = analyze_best_execution(&input).unwrap();
        let tca = &result.result.trade_results[0];
        // IS = (110 - 100) * 1000 + 10 = 10010
        assert_eq!(tca.implementation_shortfall, dec!(10010));
    }

    // -----------------------------------------------------------------------
    // Validation error tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_validation_empty_trades() {
        let input = make_input(vec![], "VWAP");
        let result = analyze_best_execution(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_validation_invalid_benchmark() {
        let input = make_input(vec![make_buy_trade()], "INVALID");
        let result = analyze_best_execution(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_validation_zero_quantity() {
        let mut trade = make_buy_trade();
        trade.quantity = dec!(0);
        let input = make_input(vec![trade], "VWAP");
        let result = analyze_best_execution(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_validation_negative_price() {
        let mut trade = make_buy_trade();
        trade.decision_price = dec!(-10);
        let input = make_input(vec![trade], "VWAP");
        let result = analyze_best_execution(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_validation_invalid_side() {
        let mut trade = make_buy_trade();
        trade.side = "Short".to_string();
        let input = make_input(vec![trade], "VWAP");
        let result = analyze_best_execution(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_validation_zero_arrival_price() {
        let mut trade = make_buy_trade();
        trade.arrival_price = dec!(0);
        let input = make_input(vec![trade], "VWAP");
        let result = analyze_best_execution(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_validation_zero_vwap_price() {
        let mut trade = make_buy_trade();
        trade.vwap_price = dec!(0);
        let input = make_input(vec![trade], "VWAP");
        let result = analyze_best_execution(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_validation_zero_close_price() {
        let mut trade = make_buy_trade();
        trade.close_price = dec!(0);
        let input = make_input(vec![trade], "VWAP");
        let result = analyze_best_execution(&input);
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // Sell-specific scenarios
    // -----------------------------------------------------------------------

    #[test]
    fn test_sell_benchmark_deviation_vwap() {
        let input = make_input(vec![make_sell_trade()], "VWAP");
        let result = analyze_best_execution(&input).unwrap();
        let tca = &result.result.trade_results[0];
        // sell deviation = (199.85 - 199.80) * 500 = 25 => 2.5 bps
        assert_eq!(tca.benchmark_deviation, dec!(25));
        assert_eq!(tca.benchmark_deviation_bps, dec!(2.5));
    }

    #[test]
    fn test_sell_implicit_costs() {
        let input = make_input(vec![make_sell_trade()], "VWAP");
        let result = analyze_best_execution(&input).unwrap();
        let tca = &result.result.trade_results[0];
        // delay = 50, impact = 50 => 100
        assert_eq!(tca.implicit_costs, dec!(100));
    }

    // -----------------------------------------------------------------------
    // Metadata / envelope
    // -----------------------------------------------------------------------

    #[test]
    fn test_methodology_present() {
        let input = make_input(vec![make_buy_trade()], "VWAP");
        let result = analyze_best_execution(&input).unwrap();
        assert!(!result.methodology.is_empty());
    }

    #[test]
    fn test_assumptions_hashmap_populated() {
        let input = make_input(vec![make_buy_trade()], "VWAP");
        let result = analyze_best_execution(&input).unwrap();
        assert!(!result.result.assumptions.is_empty());
    }
}
