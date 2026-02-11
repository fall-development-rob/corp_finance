use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::error::CorpFinanceError;
use crate::types::{with_metadata, ComputationOutput};
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Types — Target Price Analysis
// ---------------------------------------------------------------------------

/// Comparable company multiples for a single peer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerMultiple {
    /// Peer company name
    pub company: String,
    /// Price / Earnings ratio
    pub pe_ratio: Decimal,
    /// Price / Book ratio
    pub pb_ratio: Decimal,
    /// Price / Sales ratio
    pub ps_ratio: Decimal,
    /// Enterprise Value / EBITDA (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ev_ebitda: Option<Decimal>,
    /// Price/Earnings-to-Growth ratio (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub peg_ratio: Option<Decimal>,
}

/// Input for target price calculation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetPriceInput {
    /// Current market price per share
    pub current_price: Decimal,
    /// Diluted shares outstanding
    pub shares_outstanding: Decimal,
    /// Current or forward earnings per share
    pub earnings_per_share: Decimal,
    /// Expected EPS growth rate (decimal, e.g. 0.15 = 15%)
    pub earnings_growth_rate: Decimal,
    /// Book value per share
    pub book_value_per_share: Decimal,
    /// Revenue per share
    pub revenue_per_share: Decimal,
    /// Dividend per share
    pub dividend_per_share: Decimal,
    /// Comparable company multiples
    pub peer_multiples: Vec<PeerMultiple>,
    /// Analyst consensus target prices (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub analyst_targets: Option<Vec<Decimal>>,
    /// Cost of equity (decimal, e.g. 0.10 = 10%)
    pub cost_of_equity: Decimal,
    /// Terminal / long-term growth rate for DDM
    pub terminal_growth: Decimal,
    /// Projection horizon in years (typically 1-3)
    pub projection_years: u32,
}

// ---------------------------------------------------------------------------
// Output types
// ---------------------------------------------------------------------------

/// Target price derived from one method.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MethodTarget {
    pub method: String,
    pub target_price: Decimal,
    pub weight: Decimal,
    pub rationale: String,
}

/// Relative valuation metrics vs peer group.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelativeValuation {
    /// Premium (+) or discount (-) vs median peer PE
    pub pe_vs_peers: Decimal,
    /// Premium (+) or discount (-) vs median peer PB
    pub pb_vs_peers: Decimal,
    /// Premium (+) or discount (-) vs median peer PS
    pub ps_vs_peers: Decimal,
    /// Percentile rank among peers (0-100)
    pub percentile_rank: Decimal,
}

/// Football-field summary for target price range.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FootballFieldSummary {
    pub lowest_target: Decimal,
    pub highest_target: Decimal,
    pub median_target: Decimal,
    /// (method_name, low, high)
    pub methods: Vec<(String, Decimal, Decimal)>,
}

/// Full target price output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetPriceOutput {
    /// Weighted average target price across methods
    pub composite_target: Decimal,
    /// (target - current) / current as decimal
    pub upside_downside_pct: Decimal,
    /// Recommendation string
    pub recommendation: String,
    /// Individual method targets
    pub method_targets: Vec<MethodTarget>,
    /// Relative valuation vs peers
    pub relative_valuation: RelativeValuation,
    /// Football field summary
    pub football_field_summary: FootballFieldSummary,
    /// (target - current) / target
    pub margin_of_safety: Decimal,
}

// ---------------------------------------------------------------------------
// Core calculation
// ---------------------------------------------------------------------------

/// Calculate target price using multiple valuation methods.
pub fn calculate_target_price(
    input: &TargetPriceInput,
) -> CorpFinanceResult<ComputationOutput<TargetPriceOutput>> {
    let start = Instant::now();
    let mut warnings: Vec<String> = Vec::new();

    // --- Validate ---
    if input.peer_multiples.is_empty() {
        return Err(CorpFinanceError::InsufficientData(
            "At least one peer company is required".into(),
        ));
    }
    if input.current_price <= dec!(0) {
        return Err(CorpFinanceError::InvalidInput {
            field: "current_price".into(),
            reason: "must be positive".into(),
        });
    }
    if input.shares_outstanding <= dec!(0) {
        return Err(CorpFinanceError::InvalidInput {
            field: "shares_outstanding".into(),
            reason: "must be positive".into(),
        });
    }

    // --- Compute peer medians ---
    let median_pe = median_of(
        &input
            .peer_multiples
            .iter()
            .map(|p| p.pe_ratio)
            .collect::<Vec<_>>(),
    );
    let median_pb = median_of(
        &input
            .peer_multiples
            .iter()
            .map(|p| p.pb_ratio)
            .collect::<Vec<_>>(),
    );
    let median_ps = median_of(
        &input
            .peer_multiples
            .iter()
            .map(|p| p.ps_ratio)
            .collect::<Vec<_>>(),
    );
    let peg_values: Vec<Decimal> = input
        .peer_multiples
        .iter()
        .filter_map(|p| p.peg_ratio)
        .collect();
    let has_peg = !peg_values.is_empty();
    let median_peg = if has_peg {
        median_of(&peg_values)
    } else {
        dec!(0)
    };

    // --- Individual method targets ---
    let mut method_targets: Vec<MethodTarget> = Vec::new();

    // 1. PE-based
    let pe_feasible = input.earnings_per_share > dec!(0);
    let pe_target = if pe_feasible {
        // Forward EPS = current EPS * (1 + g) ^ years
        let forward_eps = compound(
            input.earnings_per_share,
            input.earnings_growth_rate,
            input.projection_years,
        );
        let target = forward_eps * median_pe;
        method_targets.push(MethodTarget {
            method: "PE".into(),
            target_price: target,
            weight: dec!(0), // assigned later
            rationale: format!(
                "Forward EPS {:.2} x median peer PE {:.1}x",
                forward_eps, median_pe
            ),
        });
        Some(target)
    } else {
        warnings.push("EPS <= 0; PE method skipped".into());
        None
    };

    // 2. PEG-based
    let peg_feasible = has_peg && input.earnings_growth_rate > dec!(0) && pe_feasible;
    let peg_target = if peg_feasible {
        // Implied PE = median_peg * growth_rate_pct
        let growth_pct = input.earnings_growth_rate * dec!(100);
        let implied_pe = median_peg * growth_pct;
        let forward_eps = compound(
            input.earnings_per_share,
            input.earnings_growth_rate,
            input.projection_years,
        );
        let target = forward_eps * implied_pe;
        method_targets.push(MethodTarget {
            method: "PEG".into(),
            target_price: target,
            weight: dec!(0),
            rationale: format!(
                "Median PEG {:.2} x growth {:.0}% => implied PE {:.1}x",
                median_peg, growth_pct, implied_pe
            ),
        });
        Some(target)
    } else {
        if !has_peg {
            warnings.push("No PEG data available; PEG method skipped".into());
        }
        None
    };

    // 3. PB-based
    let pb_target = if input.book_value_per_share > dec!(0) {
        let target = input.book_value_per_share * median_pb;
        method_targets.push(MethodTarget {
            method: "PB".into(),
            target_price: target,
            weight: dec!(0),
            rationale: format!(
                "BVPS {:.2} x median peer PB {:.1}x",
                input.book_value_per_share, median_pb
            ),
        });
        Some(target)
    } else {
        warnings.push("Book value <= 0; PB method skipped".into());
        None
    };

    // 4. PS-based
    let ps_target = if input.revenue_per_share > dec!(0) {
        let target = input.revenue_per_share * median_ps;
        method_targets.push(MethodTarget {
            method: "PS".into(),
            target_price: target,
            weight: dec!(0),
            rationale: format!(
                "RPS {:.2} x median peer PS {:.1}x",
                input.revenue_per_share, median_ps
            ),
        });
        Some(target)
    } else {
        warnings.push("Revenue per share <= 0; PS method skipped".into());
        None
    };

    // 5. DDM (Gordon Growth Model)
    let ddm_feasible = input.dividend_per_share > dec!(0)
        && input.cost_of_equity > input.terminal_growth
        && input.terminal_growth >= dec!(0);
    let ddm_target = if ddm_feasible {
        let next_div = input.dividend_per_share * (dec!(1) + input.terminal_growth);
        let denom = input.cost_of_equity - input.terminal_growth;
        let target = next_div / denom;
        method_targets.push(MethodTarget {
            method: "DDM".into(),
            target_price: target,
            weight: dec!(0),
            rationale: format!(
                "DPS {:.2} grown at {:.1}%, discounted at ke={:.1}%",
                input.dividend_per_share,
                input.terminal_growth * dec!(100),
                input.cost_of_equity * dec!(100),
            ),
        });
        Some(target)
    } else {
        if input.dividend_per_share <= dec!(0) {
            warnings.push("No dividends; DDM method skipped".into());
        } else {
            warnings.push("Cost of equity <= terminal growth; DDM method skipped".into());
        }
        None
    };

    // 6. Analyst consensus
    let consensus_target = if let Some(ref targets) = input.analyst_targets {
        if !targets.is_empty() {
            let med = median_of(targets);
            method_targets.push(MethodTarget {
                method: "Consensus".into(),
                target_price: med,
                weight: dec!(0),
                rationale: format!("Median of {} analyst targets", targets.len()),
            });
            Some(med)
        } else {
            None
        }
    } else {
        None
    };

    if method_targets.is_empty() {
        return Err(CorpFinanceError::InsufficientData(
            "No valuation method produced a valid target price".into(),
        ));
    }

    // --- Assign weights ---
    // Base weights: PE=30, PB=15, PS=15, DDM=20, PEG=10, Consensus=10
    let mut weight_pe = if pe_target.is_some() {
        dec!(30)
    } else {
        dec!(0)
    };
    let mut weight_peg = if peg_target.is_some() {
        dec!(10)
    } else {
        dec!(0)
    };
    let mut weight_pb = if pb_target.is_some() {
        dec!(15)
    } else {
        dec!(0)
    };
    let mut weight_ps = if ps_target.is_some() {
        dec!(15)
    } else {
        dec!(0)
    };
    let mut weight_ddm = if ddm_target.is_some() {
        dec!(20)
    } else {
        dec!(0)
    };
    let mut weight_consensus = if consensus_target.is_some() {
        dec!(10)
    } else {
        dec!(0)
    };

    let total_weight =
        weight_pe + weight_peg + weight_pb + weight_ps + weight_ddm + weight_consensus;

    // Normalise to sum to 1.0
    if total_weight > dec!(0) {
        weight_pe /= total_weight;
        weight_peg /= total_weight;
        weight_pb /= total_weight;
        weight_ps /= total_weight;
        weight_ddm /= total_weight;
        weight_consensus /= total_weight;
    }

    // Apply weights to method_targets
    for mt in &mut method_targets {
        mt.weight = match mt.method.as_str() {
            "PE" => weight_pe,
            "PEG" => weight_peg,
            "PB" => weight_pb,
            "PS" => weight_ps,
            "DDM" => weight_ddm,
            "Consensus" => weight_consensus,
            _ => dec!(0),
        };
    }

    // --- Composite target ---
    let composite_target: Decimal = method_targets
        .iter()
        .map(|mt| mt.target_price * mt.weight)
        .sum();

    // --- Upside / downside ---
    let upside_downside_pct = (composite_target - input.current_price) / input.current_price;

    // --- Recommendation ---
    let recommendation = if upside_downside_pct > dec!(0.20) {
        "Strong Buy"
    } else if upside_downside_pct > dec!(0.10) {
        "Buy"
    } else if upside_downside_pct >= dec!(-0.10) {
        "Hold"
    } else if upside_downside_pct >= dec!(-0.20) {
        "Sell"
    } else {
        "Strong Sell"
    }
    .to_string();

    // --- Relative valuation ---
    let current_pe = if input.earnings_per_share > dec!(0) {
        input.current_price / input.earnings_per_share
    } else {
        dec!(0)
    };
    let current_pb = if input.book_value_per_share > dec!(0) {
        input.current_price / input.book_value_per_share
    } else {
        dec!(0)
    };
    let current_ps = if input.revenue_per_share > dec!(0) {
        input.current_price / input.revenue_per_share
    } else {
        dec!(0)
    };

    let pe_vs_peers = if median_pe > dec!(0) {
        (current_pe - median_pe) / median_pe
    } else {
        dec!(0)
    };
    let pb_vs_peers = if median_pb > dec!(0) {
        (current_pb - median_pb) / median_pb
    } else {
        dec!(0)
    };
    let ps_vs_peers = if median_ps > dec!(0) {
        (current_ps - median_ps) / median_ps
    } else {
        dec!(0)
    };

    // Percentile rank: what % of peers have a lower PE?
    let percentile_rank = if input.earnings_per_share > dec!(0) {
        let peer_pes: Vec<Decimal> = input.peer_multiples.iter().map(|p| p.pe_ratio).collect();
        let count_below = peer_pes.iter().filter(|&&pe| pe < current_pe).count();
        let n = peer_pes.len();
        if n > 0 {
            Decimal::from(count_below as u64) / Decimal::from(n as u64) * dec!(100)
        } else {
            dec!(50)
        }
    } else {
        dec!(50) // default
    };

    let relative_valuation = RelativeValuation {
        pe_vs_peers,
        pb_vs_peers,
        ps_vs_peers,
        percentile_rank,
    };

    // --- Football field ---
    let all_targets: Vec<Decimal> = method_targets.iter().map(|mt| mt.target_price).collect();
    let sorted_targets = {
        let mut s = all_targets.clone();
        s.sort();
        s
    };

    let lowest_target = sorted_targets.first().copied().unwrap_or(dec!(0));
    let highest_target = sorted_targets.last().copied().unwrap_or(dec!(0));
    let median_target = median_of(&all_targets);

    let methods_range: Vec<(String, Decimal, Decimal)> = method_targets
        .iter()
        .map(|mt| {
            // +/-10% around each method target for range
            let low = mt.target_price * dec!(0.90);
            let high = mt.target_price * dec!(1.10);
            (mt.method.clone(), low, high)
        })
        .collect();

    let football_field_summary = FootballFieldSummary {
        lowest_target,
        highest_target,
        median_target,
        methods: methods_range,
    };

    // --- Margin of safety ---
    let margin_of_safety = if composite_target > dec!(0) {
        (composite_target - input.current_price) / composite_target
    } else {
        dec!(0)
    };

    let output = TargetPriceOutput {
        composite_target,
        upside_downside_pct,
        recommendation,
        method_targets,
        relative_valuation,
        football_field_summary,
        margin_of_safety,
    };

    let elapsed = start.elapsed().as_micros() as u64;
    Ok(with_metadata(
        "Target Price Analysis (Multi-Method)",
        &serde_json::json!({
            "peer_count": input.peer_multiples.len(),
            "projection_years": input.projection_years,
            "cost_of_equity": input.cost_of_equity.to_string(),
        }),
        warnings,
        elapsed,
        output,
    ))
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Compute compound value: base * (1 + rate)^periods using iterative multiplication.
fn compound(base: Decimal, rate: Decimal, periods: u32) -> Decimal {
    let mut result = base;
    let factor = dec!(1) + rate;
    for _ in 0..periods {
        result *= factor;
    }
    result
}

/// Compute the median of a slice of Decimals.
fn median_of(values: &[Decimal]) -> Decimal {
    if values.is_empty() {
        return dec!(0);
    }
    let mut sorted = values.to_vec();
    sorted.sort();
    let n = sorted.len();
    if n % 2 == 1 {
        sorted[n / 2]
    } else {
        (sorted[n / 2 - 1] + sorted[n / 2]) / dec!(2)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    /// Baseline growth stock input — all methods should produce targets.
    fn growth_stock_input() -> TargetPriceInput {
        TargetPriceInput {
            current_price: dec!(100),
            shares_outstanding: dec!(1000),
            earnings_per_share: dec!(5),
            earnings_growth_rate: dec!(0.15),
            book_value_per_share: dec!(30),
            revenue_per_share: dec!(50),
            dividend_per_share: dec!(1.50),
            peer_multiples: vec![
                PeerMultiple {
                    company: "PeerA".into(),
                    pe_ratio: dec!(25),
                    pb_ratio: dec!(4),
                    ps_ratio: dec!(3),
                    ev_ebitda: Some(dec!(15)),
                    peg_ratio: Some(dec!(1.5)),
                },
                PeerMultiple {
                    company: "PeerB".into(),
                    pe_ratio: dec!(30),
                    pb_ratio: dec!(5),
                    ps_ratio: dec!(4),
                    ev_ebitda: Some(dec!(18)),
                    peg_ratio: Some(dec!(1.8)),
                },
                PeerMultiple {
                    company: "PeerC".into(),
                    pe_ratio: dec!(20),
                    pb_ratio: dec!(3),
                    ps_ratio: dec!(2),
                    ev_ebitda: Some(dec!(12)),
                    peg_ratio: Some(dec!(1.2)),
                },
            ],
            analyst_targets: Some(vec![dec!(120), dec!(130), dec!(110)]),
            cost_of_equity: dec!(0.10),
            terminal_growth: dec!(0.03),
            projection_years: 1,
        }
    }

    fn value_stock_input() -> TargetPriceInput {
        TargetPriceInput {
            current_price: dec!(50),
            shares_outstanding: dec!(500),
            earnings_per_share: dec!(6),
            earnings_growth_rate: dec!(0.05),
            book_value_per_share: dec!(45),
            revenue_per_share: dec!(80),
            dividend_per_share: dec!(3),
            peer_multiples: vec![
                PeerMultiple {
                    company: "ValuePeerA".into(),
                    pe_ratio: dec!(10),
                    pb_ratio: dec!(1.2),
                    ps_ratio: dec!(0.8),
                    ev_ebitda: Some(dec!(7)),
                    peg_ratio: Some(dec!(1.0)),
                },
                PeerMultiple {
                    company: "ValuePeerB".into(),
                    pe_ratio: dec!(12),
                    pb_ratio: dec!(1.5),
                    ps_ratio: dec!(1.0),
                    ev_ebitda: Some(dec!(8)),
                    peg_ratio: Some(dec!(1.2)),
                },
            ],
            analyst_targets: Some(vec![dec!(55), dec!(60)]),
            cost_of_equity: dec!(0.09),
            terminal_growth: dec!(0.02),
            projection_years: 1,
        }
    }

    fn dividend_stock_input() -> TargetPriceInput {
        TargetPriceInput {
            current_price: dec!(40),
            shares_outstanding: dec!(200),
            earnings_per_share: dec!(4),
            earnings_growth_rate: dec!(0.03),
            book_value_per_share: dec!(35),
            revenue_per_share: dec!(60),
            dividend_per_share: dec!(3),
            peer_multiples: vec![PeerMultiple {
                company: "DivPeer".into(),
                pe_ratio: dec!(12),
                pb_ratio: dec!(1.2),
                ps_ratio: dec!(0.7),
                ev_ebitda: None,
                peg_ratio: None,
            }],
            analyst_targets: None,
            cost_of_equity: dec!(0.08),
            terminal_growth: dec!(0.02),
            projection_years: 1,
        }
    }

    // -----------------------------------------------------------------------
    // Basic functionality tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_target_price_growth_stock() {
        let input = growth_stock_input();
        let result = calculate_target_price(&input).unwrap();
        let out = &result.result;

        assert!(out.composite_target > dec!(0));
        assert!(out.method_targets.len() >= 4); // PE, PB, PS, DDM, PEG, Consensus
    }

    #[test]
    fn test_target_price_composite_is_weighted_average() {
        let input = growth_stock_input();
        let result = calculate_target_price(&input).unwrap();
        let out = &result.result;

        let manual_composite: Decimal = out
            .method_targets
            .iter()
            .map(|mt| mt.target_price * mt.weight)
            .sum();
        assert!((out.composite_target - manual_composite).abs() < dec!(0.01));
    }

    #[test]
    fn test_target_price_weights_sum_to_one() {
        let input = growth_stock_input();
        let result = calculate_target_price(&input).unwrap();
        let total_w: Decimal = result.result.method_targets.iter().map(|m| m.weight).sum();
        assert!((total_w - dec!(1)).abs() < dec!(0.001));
    }

    #[test]
    fn test_target_price_pe_method() {
        let input = growth_stock_input();
        let result = calculate_target_price(&input).unwrap();
        let pe_mt = result
            .result
            .method_targets
            .iter()
            .find(|m| m.method == "PE")
            .unwrap();

        // Forward EPS = 5 * 1.15 = 5.75, median PE = 25 (middle of 20,25,30)
        let expected = dec!(5) * dec!(1.15) * dec!(25);
        assert_eq!(pe_mt.target_price, expected);
    }

    #[test]
    fn test_target_price_pb_method() {
        let input = growth_stock_input();
        let result = calculate_target_price(&input).unwrap();
        let pb_mt = result
            .result
            .method_targets
            .iter()
            .find(|m| m.method == "PB")
            .unwrap();

        // BVPS=30, median PB = 4 (middle of 3,4,5)
        assert_eq!(pb_mt.target_price, dec!(30) * dec!(4));
    }

    #[test]
    fn test_target_price_ps_method() {
        let input = growth_stock_input();
        let result = calculate_target_price(&input).unwrap();
        let ps_mt = result
            .result
            .method_targets
            .iter()
            .find(|m| m.method == "PS")
            .unwrap();

        // RPS=50, median PS = 3 (middle of 2,3,4)
        assert_eq!(ps_mt.target_price, dec!(50) * dec!(3));
    }

    #[test]
    fn test_target_price_ddm_method() {
        let input = growth_stock_input();
        let result = calculate_target_price(&input).unwrap();
        let ddm_mt = result
            .result
            .method_targets
            .iter()
            .find(|m| m.method == "DDM")
            .unwrap();

        // DPS=1.50 * (1+0.03) / (0.10 - 0.03) = 1.545 / 0.07 = 22.071...
        let expected = dec!(1.50) * dec!(1.03) / dec!(0.07);
        assert_eq!(ddm_mt.target_price, expected);
    }

    #[test]
    fn test_target_price_peg_method() {
        let input = growth_stock_input();
        let result = calculate_target_price(&input).unwrap();
        let peg_mt = result
            .result
            .method_targets
            .iter()
            .find(|m| m.method == "PEG")
            .unwrap();

        // Median PEG = 1.5 (middle of 1.2, 1.5, 1.8)
        // Implied PE = 1.5 * 15 = 22.5
        // Forward EPS = 5 * 1.15 = 5.75
        // Target = 5.75 * 22.5 = 129.375
        let growth_pct = dec!(0.15) * dec!(100); // 15
        let implied_pe = dec!(1.5) * growth_pct; // 22.5
        let forward_eps = dec!(5) * dec!(1.15); // 5.75
        let expected = forward_eps * implied_pe;
        assert_eq!(peg_mt.target_price, expected);
    }

    #[test]
    fn test_target_price_consensus_method() {
        let input = growth_stock_input();
        let result = calculate_target_price(&input).unwrap();
        let cons_mt = result
            .result
            .method_targets
            .iter()
            .find(|m| m.method == "Consensus")
            .unwrap();

        // Median of [110, 120, 130] = 120
        assert_eq!(cons_mt.target_price, dec!(120));
    }

    #[test]
    fn test_target_price_value_stock() {
        let input = value_stock_input();
        let result = calculate_target_price(&input).unwrap();
        let out = &result.result;

        assert!(out.composite_target > dec!(0));
        // Value stock at $50 — should be near peers
    }

    #[test]
    fn test_target_price_dividend_stock_ddm_dominant() {
        let input = dividend_stock_input();
        let result = calculate_target_price(&input).unwrap();
        let out = &result.result;

        // DDM should have a significant weight since no PEG, no consensus
        let ddm_mt = out
            .method_targets
            .iter()
            .find(|m| m.method == "DDM")
            .unwrap();
        // With no PEG and no consensus, DDM weight = 20/(30+15+15+20) = 20/80 = 0.25
        assert_eq!(ddm_mt.weight, dec!(20) / dec!(80));
    }

    // -----------------------------------------------------------------------
    // Upside / downside and recommendation tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_recommendation_strong_buy() {
        // Create input where composite >> current_price
        let mut input = growth_stock_input();
        input.current_price = dec!(50); // Low price vs high targets
        let result = calculate_target_price(&input).unwrap();
        let out = &result.result;

        assert!(out.upside_downside_pct > dec!(0.20));
        assert_eq!(out.recommendation, "Strong Buy");
    }

    #[test]
    fn test_recommendation_buy() {
        let mut input = growth_stock_input();
        // Adjust current_price so upside is 10-20%
        // We need composite to be ~110-120% of current price
        let result_base = calculate_target_price(&input).unwrap();
        let composite = result_base.result.composite_target;
        // Set price so upside is ~15%
        input.current_price = composite / dec!(1.15);
        let result = calculate_target_price(&input).unwrap();
        assert_eq!(result.result.recommendation, "Buy");
    }

    #[test]
    fn test_recommendation_hold() {
        let mut input = growth_stock_input();
        let result_base = calculate_target_price(&input).unwrap();
        let composite = result_base.result.composite_target;
        // Set price at composite so upside is ~0%
        input.current_price = composite;
        let result = calculate_target_price(&input).unwrap();
        assert_eq!(result.result.recommendation, "Hold");
    }

    #[test]
    fn test_recommendation_sell() {
        let mut input = growth_stock_input();
        let result_base = calculate_target_price(&input).unwrap();
        let composite = result_base.result.composite_target;
        // Set price so downside is ~15%
        input.current_price = composite / dec!(0.85);
        let result = calculate_target_price(&input).unwrap();
        assert_eq!(result.result.recommendation, "Sell");
    }

    #[test]
    fn test_recommendation_strong_sell() {
        let mut input = growth_stock_input();
        let result_base = calculate_target_price(&input).unwrap();
        let composite = result_base.result.composite_target;
        // Set price so downside is >20%
        input.current_price = composite / dec!(0.75);
        let result = calculate_target_price(&input).unwrap();
        assert_eq!(result.result.recommendation, "Strong Sell");
    }

    #[test]
    fn test_upside_downside_pct_calculation() {
        let input = growth_stock_input();
        let result = calculate_target_price(&input).unwrap();
        let out = &result.result;

        let expected = (out.composite_target - input.current_price) / input.current_price;
        assert_eq!(out.upside_downside_pct, expected);
    }

    // -----------------------------------------------------------------------
    // Relative valuation tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_relative_valuation_premium_vs_peers() {
        // Current PE = 100/5 = 20, median_peer PE = 25 => discount
        let input = growth_stock_input();
        let result = calculate_target_price(&input).unwrap();
        let rv = &result.result.relative_valuation;

        // PE: (20-25)/25 = -0.2 => trading at 20% discount
        assert_eq!(rv.pe_vs_peers, (dec!(20) - dec!(25)) / dec!(25));
        assert!(rv.pe_vs_peers < dec!(0)); // discount
    }

    #[test]
    fn test_relative_valuation_pb() {
        let input = growth_stock_input();
        let result = calculate_target_price(&input).unwrap();
        let rv = &result.result.relative_valuation;

        // Current PB = 100/30 = 3.333..., median PB = 4
        let current_pb = dec!(100) / dec!(30);
        let expected = (current_pb - dec!(4)) / dec!(4);
        assert!((rv.pb_vs_peers - expected).abs() < dec!(0.001));
    }

    #[test]
    fn test_relative_valuation_ps() {
        let input = growth_stock_input();
        let result = calculate_target_price(&input).unwrap();
        let rv = &result.result.relative_valuation;

        // Current PS = 100/50 = 2, median PS = 3 => (2-3)/3 = -0.333...
        let expected = (dec!(2) - dec!(3)) / dec!(3);
        assert!((rv.ps_vs_peers - expected).abs() < dec!(0.001));
    }

    #[test]
    fn test_percentile_rank() {
        let input = growth_stock_input();
        let result = calculate_target_price(&input).unwrap();
        let rv = &result.result.relative_valuation;

        // Current PE = 20, peers = [25, 30, 20] => 0 have lower PE
        // Actually peer PEs are [25, 30, 20], current PE = 100/5 = 20
        // count below 20 = 0, n=3, percentile = 0/3*100 = 0
        assert_eq!(rv.percentile_rank, dec!(0));
    }

    #[test]
    fn test_percentile_rank_high() {
        let mut input = growth_stock_input();
        // Make current PE very high
        input.earnings_per_share = dec!(2); // PE = 100/2 = 50, peers are 20,25,30
        let result = calculate_target_price(&input).unwrap();
        let rv = &result.result.relative_valuation;
        // All 3 peers below => 3/3*100 = 100
        assert_eq!(rv.percentile_rank, dec!(100));
    }

    // -----------------------------------------------------------------------
    // Edge case tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_zero_earnings_pe_skipped() {
        let mut input = growth_stock_input();
        input.earnings_per_share = dec!(0);
        let result = calculate_target_price(&input).unwrap();
        let has_pe = result
            .result
            .method_targets
            .iter()
            .any(|m| m.method == "PE");
        assert!(!has_pe);
        assert!(result.warnings.iter().any(|w| w.contains("EPS <= 0")));
    }

    #[test]
    fn test_negative_growth_ddm_skipped_if_ke_le_g() {
        let mut input = growth_stock_input();
        input.terminal_growth = dec!(0.12); // > ke of 0.10
        let result = calculate_target_price(&input).unwrap();
        let has_ddm = result
            .result
            .method_targets
            .iter()
            .any(|m| m.method == "DDM");
        assert!(!has_ddm);
    }

    #[test]
    fn test_no_consensus_available() {
        let mut input = growth_stock_input();
        input.analyst_targets = None;
        let result = calculate_target_price(&input).unwrap();
        let has_consensus = result
            .result
            .method_targets
            .iter()
            .any(|m| m.method == "Consensus");
        assert!(!has_consensus);
    }

    #[test]
    fn test_no_peg_data() {
        let mut input = growth_stock_input();
        for peer in &mut input.peer_multiples {
            peer.peg_ratio = None;
        }
        let result = calculate_target_price(&input).unwrap();
        let has_peg = result
            .result
            .method_targets
            .iter()
            .any(|m| m.method == "PEG");
        assert!(!has_peg);
        assert!(result.warnings.iter().any(|w| w.contains("No PEG data")));
    }

    #[test]
    fn test_single_peer() {
        let mut input = growth_stock_input();
        input.peer_multiples = vec![input.peer_multiples[0].clone()];
        let result = calculate_target_price(&input).unwrap();
        assert!(result.result.composite_target > dec!(0));
    }

    #[test]
    fn test_no_peers_error() {
        let mut input = growth_stock_input();
        input.peer_multiples = vec![];
        let result = calculate_target_price(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_zero_current_price_error() {
        let mut input = growth_stock_input();
        input.current_price = dec!(0);
        let result = calculate_target_price(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_zero_shares_error() {
        let mut input = growth_stock_input();
        input.shares_outstanding = dec!(0);
        let result = calculate_target_price(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_no_dividends_ddm_skipped() {
        let mut input = growth_stock_input();
        input.dividend_per_share = dec!(0);
        let result = calculate_target_price(&input).unwrap();
        let has_ddm = result
            .result
            .method_targets
            .iter()
            .any(|m| m.method == "DDM");
        assert!(!has_ddm);
        assert!(result.warnings.iter().any(|w| w.contains("No dividends")));
    }

    #[test]
    fn test_football_field_range() {
        let input = growth_stock_input();
        let result = calculate_target_price(&input).unwrap();
        let ff = &result.result.football_field_summary;

        assert!(ff.lowest_target <= ff.median_target);
        assert!(ff.median_target <= ff.highest_target);
        assert!(!ff.methods.is_empty());
    }

    #[test]
    fn test_football_field_method_ranges() {
        let input = growth_stock_input();
        let result = calculate_target_price(&input).unwrap();
        let ff = &result.result.football_field_summary;

        for (_, low, high) in &ff.methods {
            assert!(low <= high);
        }
    }

    #[test]
    fn test_margin_of_safety() {
        let input = growth_stock_input();
        let result = calculate_target_price(&input).unwrap();
        let out = &result.result;

        let expected = (out.composite_target - input.current_price) / out.composite_target;
        assert_eq!(out.margin_of_safety, expected);
    }

    #[test]
    fn test_methodology_metadata() {
        let input = growth_stock_input();
        let result = calculate_target_price(&input).unwrap();
        assert_eq!(result.methodology, "Target Price Analysis (Multi-Method)");
    }

    #[test]
    fn test_projection_years_multi() {
        let mut input = growth_stock_input();
        input.projection_years = 3;
        let result = calculate_target_price(&input).unwrap();
        let pe_mt = result
            .result
            .method_targets
            .iter()
            .find(|m| m.method == "PE")
            .unwrap();

        // Forward EPS = 5 * 1.15^3 = 5 * 1.521... = 7.60...
        let forward_eps = dec!(5) * dec!(1.15) * dec!(1.15) * dec!(1.15);
        let expected = forward_eps * dec!(25); // median PE
        assert_eq!(pe_mt.target_price, expected);
    }

    #[test]
    fn test_weight_redistribution_no_ddm_no_consensus_no_peg() {
        let mut input = growth_stock_input();
        input.dividend_per_share = dec!(0); // No DDM
        input.analyst_targets = None; // No consensus
        for peer in &mut input.peer_multiples {
            peer.peg_ratio = None; // No PEG
        }
        let result = calculate_target_price(&input).unwrap();
        let out = &result.result;

        // Only PE(30), PB(15), PS(15) => total=60
        // Weights: PE=30/60=0.5, PB=15/60=0.25, PS=15/60=0.25
        let pe_w = out
            .method_targets
            .iter()
            .find(|m| m.method == "PE")
            .unwrap()
            .weight;
        let pb_w = out
            .method_targets
            .iter()
            .find(|m| m.method == "PB")
            .unwrap()
            .weight;
        let ps_w = out
            .method_targets
            .iter()
            .find(|m| m.method == "PS")
            .unwrap()
            .weight;

        assert_eq!(pe_w, dec!(30) / dec!(60));
        assert_eq!(pb_w, dec!(15) / dec!(60));
        assert_eq!(ps_w, dec!(15) / dec!(60));
    }

    #[test]
    fn test_all_methods_produce_targets() {
        let input = growth_stock_input();
        let result = calculate_target_price(&input).unwrap();
        let methods: Vec<&str> = result
            .result
            .method_targets
            .iter()
            .map(|m| m.method.as_str())
            .collect();

        assert!(methods.contains(&"PE"));
        assert!(methods.contains(&"PB"));
        assert!(methods.contains(&"PS"));
        assert!(methods.contains(&"DDM"));
        assert!(methods.contains(&"PEG"));
        assert!(methods.contains(&"Consensus"));
    }

    // -----------------------------------------------------------------------
    // Helper tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_median_odd() {
        assert_eq!(median_of(&[dec!(3), dec!(1), dec!(2)]), dec!(2));
    }

    #[test]
    fn test_median_even() {
        assert_eq!(median_of(&[dec!(1), dec!(2), dec!(3), dec!(4)]), dec!(2.5));
    }

    #[test]
    fn test_median_single() {
        assert_eq!(median_of(&[dec!(42)]), dec!(42));
    }

    #[test]
    fn test_median_empty() {
        assert_eq!(median_of(&[]), dec!(0));
    }

    #[test]
    fn test_compound_zero_periods() {
        assert_eq!(compound(dec!(100), dec!(0.10), 0), dec!(100));
    }

    #[test]
    fn test_compound_one_period() {
        assert_eq!(compound(dec!(100), dec!(0.10), 1), dec!(110));
    }

    #[test]
    fn test_compound_three_periods() {
        // 100 * 1.1 * 1.1 * 1.1 = 133.1
        let result = compound(dec!(100), dec!(0.10), 3);
        assert_eq!(result, dec!(100) * dec!(1.1) * dec!(1.1) * dec!(1.1));
    }
}
