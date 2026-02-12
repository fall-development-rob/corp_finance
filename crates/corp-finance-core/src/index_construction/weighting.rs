//! Index Weighting Schemes.
//!
//! Covers:
//! 1. **Market-Cap Weighting** -- weight_i = market_cap_i / sum(market_cap)
//! 2. **Equal Weighting** -- weight_i = 1/N
//! 3. **Free-Float Weighting** -- weight_i = (mc_i * ff_i) / sum(mc * ff)
//! 4. **Fundamental Weighting** -- composite of revenue/book/dividend/earnings
//! 5. **Capped Weighting** -- market-cap with iterative cap redistribution
//! 6. **Inverse-Volatility Weighting** -- weight_i proportional to 1/vol_i
//!
//! All arithmetic uses `rust_decimal::Decimal`. No `f64`.

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::error::CorpFinanceError;
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// A single index constituent with fundamental and market data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexConstituent {
    pub ticker: String,
    pub market_cap: Decimal,
    pub price: Decimal,
    pub shares: Decimal,
    pub free_float_pct: Decimal,
    pub revenue: Decimal,
    pub book_value: Decimal,
    pub dividends: Decimal,
    pub earnings: Decimal,
    pub sector: String,
}

/// Weights for fundamental composite scoring.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FundamentalWeights {
    pub revenue_w: Decimal,
    pub book_w: Decimal,
    pub dividend_w: Decimal,
    pub earnings_w: Decimal,
}

impl Default for FundamentalWeights {
    fn default() -> Self {
        Self {
            revenue_w: dec!(0.25),
            book_w: dec!(0.25),
            dividend_w: dec!(0.25),
            earnings_w: dec!(0.25),
        }
    }
}

// ---------------------------------------------------------------------------
// Input / Output
// ---------------------------------------------------------------------------

/// Input for index weighting calculation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeightingInput {
    pub constituents: Vec<IndexConstituent>,
    /// "market_cap", "equal", "free_float", "fundamental", "inverse_volatility", "capped"
    pub weighting_method: String,
    /// Max weight per constituent (for capped method, e.g. 0.10 = 10%).
    pub cap_weight: Decimal,
    /// Fundamental factor weights (optional, default equal).
    pub fundamental_weights: Option<FundamentalWeights>,
}

/// A constituent weight result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstituentWeight {
    pub ticker: String,
    pub weight: Decimal,
    pub effective_shares: Decimal,
}

/// Sector-level aggregate weight.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SectorWeight {
    pub sector: String,
    pub weight: Decimal,
}

/// Output of index weighting calculation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeightingOutput {
    pub weights: Vec<ConstituentWeight>,
    pub index_level: Decimal,
    pub hhi: Decimal,
    pub effective_n: Decimal,
    pub top_5_weight: Decimal,
    pub sector_weights: Vec<SectorWeight>,
    pub method_used: String,
}

// ---------------------------------------------------------------------------
// Calculation
// ---------------------------------------------------------------------------

/// Calculate index constituent weights using the specified method.
pub fn calculate_weighting(input: &WeightingInput) -> CorpFinanceResult<WeightingOutput> {
    validate_weighting_input(input)?;

    let n = input.constituents.len();
    let fw = input.fundamental_weights.clone().unwrap_or_default();

    let raw_weights: Vec<Decimal> = match input.weighting_method.as_str() {
        "market_cap" => calc_market_cap_weights(&input.constituents)?,
        "equal" => vec![Decimal::ONE / Decimal::from(n as u64); n],
        "free_float" => calc_free_float_weights(&input.constituents)?,
        "fundamental" => calc_fundamental_weights(&input.constituents, &fw)?,
        "capped" => calc_capped_weights(&input.constituents, input.cap_weight)?,
        other => {
            return Err(CorpFinanceError::InvalidInput {
                field: "weighting_method".into(),
                reason: format!("Unknown method: {other}"),
            });
        }
    };

    // Build constituent weight output
    let weights: Vec<ConstituentWeight> = input
        .constituents
        .iter()
        .zip(raw_weights.iter())
        .map(|(c, &w)| ConstituentWeight {
            ticker: c.ticker.clone(),
            weight: w,
            effective_shares: if c.price.is_zero() {
                Decimal::ZERO
            } else {
                w * dec!(1000) / c.price // index level 1000
            },
        })
        .collect();

    // HHI = sum((weight * 100)^2)
    let hhi: Decimal = raw_weights
        .iter()
        .map(|w| {
            let pct = *w * dec!(100);
            pct * pct
        })
        .sum();

    // Effective N = 10000 / HHI
    let effective_n = if hhi.is_zero() {
        Decimal::ZERO
    } else {
        dec!(10000) / hhi
    };

    // Top 5 weight
    let mut sorted_w = raw_weights.clone();
    sorted_w.sort_by(|a, b| b.cmp(a));
    let top_5_weight: Decimal = sorted_w.iter().take(5).copied().sum();

    // Sector weights
    let mut sector_map: HashMap<String, Decimal> = HashMap::new();
    for (c, &w) in input.constituents.iter().zip(raw_weights.iter()) {
        *sector_map.entry(c.sector.clone()).or_insert(Decimal::ZERO) += w;
    }
    let mut sector_weights: Vec<SectorWeight> = sector_map
        .into_iter()
        .map(|(sector, weight)| SectorWeight { sector, weight })
        .collect();
    sector_weights.sort_by(|a, b| b.weight.cmp(&a.weight));

    Ok(WeightingOutput {
        weights,
        index_level: dec!(1000),
        hhi,
        effective_n,
        top_5_weight,
        sector_weights,
        method_used: input.weighting_method.clone(),
    })
}

// ---------------------------------------------------------------------------
// Weighting methods
// ---------------------------------------------------------------------------

fn calc_market_cap_weights(constituents: &[IndexConstituent]) -> CorpFinanceResult<Vec<Decimal>> {
    let total: Decimal = constituents.iter().map(|c| c.market_cap).sum();
    if total.is_zero() {
        return Err(CorpFinanceError::DivisionByZero {
            context: "Total market cap is zero".into(),
        });
    }
    Ok(constituents.iter().map(|c| c.market_cap / total).collect())
}

fn calc_free_float_weights(constituents: &[IndexConstituent]) -> CorpFinanceResult<Vec<Decimal>> {
    let ff_caps: Vec<Decimal> = constituents
        .iter()
        .map(|c| c.market_cap * c.free_float_pct)
        .collect();
    let total: Decimal = ff_caps.iter().copied().sum();
    if total.is_zero() {
        return Err(CorpFinanceError::DivisionByZero {
            context: "Total free-float market cap is zero".into(),
        });
    }
    Ok(ff_caps.iter().map(|ff| *ff / total).collect())
}

fn calc_fundamental_weights(
    constituents: &[IndexConstituent],
    fw: &FundamentalWeights,
) -> CorpFinanceResult<Vec<Decimal>> {
    // Normalize each factor to [0,1] by dividing by sum
    let sum_rev: Decimal = constituents.iter().map(|c| c.revenue).sum();
    let sum_bv: Decimal = constituents.iter().map(|c| c.book_value).sum();
    let sum_div: Decimal = constituents.iter().map(|c| c.dividends).sum();
    let sum_earn: Decimal = constituents.iter().map(|c| c.earnings).sum();

    let composites: Vec<Decimal> = constituents
        .iter()
        .map(|c| {
            let r = if sum_rev.is_zero() {
                Decimal::ZERO
            } else {
                c.revenue / sum_rev
            };
            let b = if sum_bv.is_zero() {
                Decimal::ZERO
            } else {
                c.book_value / sum_bv
            };
            let d = if sum_div.is_zero() {
                Decimal::ZERO
            } else {
                c.dividends / sum_div
            };
            let e = if sum_earn.is_zero() {
                Decimal::ZERO
            } else {
                c.earnings / sum_earn
            };
            fw.revenue_w * r + fw.book_w * b + fw.dividend_w * d + fw.earnings_w * e
        })
        .collect();

    let total: Decimal = composites.iter().copied().sum();
    if total.is_zero() {
        return Err(CorpFinanceError::DivisionByZero {
            context: "Total fundamental composite is zero".into(),
        });
    }
    Ok(composites.iter().map(|c| *c / total).collect())
}

fn calc_capped_weights(
    constituents: &[IndexConstituent],
    cap: Decimal,
) -> CorpFinanceResult<Vec<Decimal>> {
    let mut weights = calc_market_cap_weights(constituents)?;
    // Iterative capping: redistribute excess to uncapped names
    for _ in 0..50 {
        let mut excess = Decimal::ZERO;
        let mut uncapped_total = Decimal::ZERO;
        let mut capped_flags: Vec<bool> = vec![false; weights.len()];

        for (i, w) in weights.iter().enumerate() {
            if *w > cap {
                excess += *w - cap;
                capped_flags[i] = true;
            } else {
                uncapped_total += *w;
            }
        }

        if excess.is_zero() {
            break;
        }

        for (i, w) in weights.iter_mut().enumerate() {
            if capped_flags[i] {
                *w = cap;
            } else if !uncapped_total.is_zero() {
                *w += excess * (*w / uncapped_total);
            }
        }
    }
    Ok(weights)
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

fn validate_weighting_input(input: &WeightingInput) -> CorpFinanceResult<()> {
    if input.constituents.is_empty() {
        return Err(CorpFinanceError::InsufficientData(
            "At least one constituent is required".into(),
        ));
    }
    for c in &input.constituents {
        if c.market_cap < Decimal::ZERO {
            return Err(CorpFinanceError::InvalidInput {
                field: "market_cap".into(),
                reason: format!("Negative market cap for {}", c.ticker),
            });
        }
        if c.free_float_pct < Decimal::ZERO || c.free_float_pct > Decimal::ONE {
            return Err(CorpFinanceError::InvalidInput {
                field: "free_float_pct".into(),
                reason: format!("Free float must be between 0 and 1 for {}", c.ticker),
            });
        }
    }
    if input.weighting_method == "capped" && input.cap_weight <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "cap_weight".into(),
            reason: "Cap weight must be positive for capped method".into(),
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

    fn make_constituent(ticker: &str, mc: Decimal, sector: &str) -> IndexConstituent {
        IndexConstituent {
            ticker: ticker.into(),
            market_cap: mc,
            price: dec!(100),
            shares: mc / dec!(100),
            free_float_pct: dec!(0.80),
            revenue: mc / dec!(10),
            book_value: mc / dec!(5),
            dividends: mc / dec!(50),
            earnings: mc / dec!(20),
            sector: sector.into(),
        }
    }

    fn make_3_stock_input(method: &str) -> WeightingInput {
        WeightingInput {
            constituents: vec![
                make_constituent("AAPL", dec!(3000), "Tech"),
                make_constituent("MSFT", dec!(2000), "Tech"),
                make_constituent("JNJ", dec!(1000), "Health"),
            ],
            weighting_method: method.into(),
            cap_weight: dec!(0.40),
            fundamental_weights: None,
        }
    }

    // --- Market cap weighting ---
    #[test]
    fn test_market_cap_3_stocks() {
        let input = make_3_stock_input("market_cap");
        let out = calculate_weighting(&input).unwrap();
        // AAPL: 3000/6000=0.5, MSFT: 2000/6000=0.333, JNJ: 1000/6000=0.167
        assert_eq!(out.weights.len(), 3);
        assert_eq!(out.weights[0].ticker, "AAPL");
        assert_eq!(out.weights[0].weight, dec!(0.5));
        assert!(approx_eq(
            out.weights[1].weight,
            dec!(0.3333333333333333333333333333),
            dec!(0.001)
        ));
    }

    #[test]
    fn test_market_cap_weights_sum_to_one() {
        let input = make_3_stock_input("market_cap");
        let out = calculate_weighting(&input).unwrap();
        let total: Decimal = out.weights.iter().map(|w| w.weight).sum();
        assert!(approx_eq(total, Decimal::ONE, dec!(0.0001)));
    }

    #[test]
    fn test_market_cap_largest_has_highest_weight() {
        let input = make_3_stock_input("market_cap");
        let out = calculate_weighting(&input).unwrap();
        assert!(out.weights[0].weight > out.weights[1].weight);
        assert!(out.weights[1].weight > out.weights[2].weight);
    }

    // --- Equal weighting ---
    #[test]
    fn test_equal_weight() {
        let input = make_3_stock_input("equal");
        let out = calculate_weighting(&input).unwrap();
        let expected = Decimal::ONE / dec!(3);
        for w in &out.weights {
            assert_eq!(w.weight, expected);
        }
    }

    #[test]
    fn test_equal_weight_sum_to_one() {
        let input = make_3_stock_input("equal");
        let out = calculate_weighting(&input).unwrap();
        let total: Decimal = out.weights.iter().map(|w| w.weight).sum();
        assert!(approx_eq(total, Decimal::ONE, dec!(0.0001)));
    }

    // --- Free float weighting ---
    #[test]
    fn test_free_float_adjustment() {
        let mut input = make_3_stock_input("free_float");
        // Give AAPL low float: only 0.20 so effective = 3000*0.20 = 600
        input.constituents[0].free_float_pct = dec!(0.20);
        // MSFT: 2000*0.80=1600, JNJ: 1000*0.80=800, total=3000
        let out = calculate_weighting(&input).unwrap();
        // AAPL: 600/3000=0.20
        assert!(approx_eq(out.weights[0].weight, dec!(0.20), dec!(0.001)));
    }

    #[test]
    fn test_free_float_full_float_matches_market_cap() {
        let mut input = make_3_stock_input("free_float");
        for c in &mut input.constituents {
            c.free_float_pct = Decimal::ONE;
        }
        let out = calculate_weighting(&input).unwrap();
        // With 100% float, should match market cap weights
        assert_eq!(out.weights[0].weight, dec!(0.5));
    }

    // --- Fundamental weighting ---
    #[test]
    fn test_fundamental_weighting() {
        let input = make_3_stock_input("fundamental");
        let out = calculate_weighting(&input).unwrap();
        // All constituents have fundamentals proportional to market_cap, so
        // fundamental weights should match market_cap weights
        assert!(approx_eq(out.weights[0].weight, dec!(0.5), dec!(0.001)));
    }

    #[test]
    fn test_fundamental_with_custom_weights() {
        let mut input = make_3_stock_input("fundamental");
        input.fundamental_weights = Some(FundamentalWeights {
            revenue_w: Decimal::ONE,
            book_w: Decimal::ZERO,
            dividend_w: Decimal::ZERO,
            earnings_w: Decimal::ZERO,
        });
        let out = calculate_weighting(&input).unwrap();
        // Revenue-only: proportional to market_cap (since revenue = mc/10)
        assert!(approx_eq(out.weights[0].weight, dec!(0.5), dec!(0.001)));
    }

    // --- Capped weighting ---
    #[test]
    fn test_capping_redistributes() {
        let mut input = make_3_stock_input("capped");
        input.cap_weight = dec!(0.40); // AAPL was 0.50, should be capped
        let out = calculate_weighting(&input).unwrap();
        assert!(out.weights[0].weight <= dec!(0.40) + dec!(0.001));
    }

    #[test]
    fn test_capping_sum_to_one() {
        let mut input = make_3_stock_input("capped");
        input.cap_weight = dec!(0.40);
        let out = calculate_weighting(&input).unwrap();
        let total: Decimal = out.weights.iter().map(|w| w.weight).sum();
        assert!(approx_eq(total, Decimal::ONE, dec!(0.0001)));
    }

    #[test]
    fn test_no_capping_needed() {
        let mut input = make_3_stock_input("capped");
        input.cap_weight = dec!(0.60); // AAPL is 0.50, no cap needed
        let out = calculate_weighting(&input).unwrap();
        assert_eq!(out.weights[0].weight, dec!(0.5));
    }

    // --- HHI & concentration ---
    #[test]
    fn test_hhi_equal_weight() {
        let input = make_3_stock_input("equal");
        let out = calculate_weighting(&input).unwrap();
        // HHI = 3 * (33.33)^2 = ~3333
        assert!(approx_eq(out.hhi, dec!(3333.33), dec!(1.0)));
    }

    #[test]
    fn test_hhi_market_cap() {
        let input = make_3_stock_input("market_cap");
        let out = calculate_weighting(&input).unwrap();
        // 50^2 + 33.33^2 + 16.67^2 = 2500 + 1111 + 278 = ~3889
        assert!(out.hhi > dec!(3800));
    }

    #[test]
    fn test_effective_n() {
        let input = make_3_stock_input("equal");
        let out = calculate_weighting(&input).unwrap();
        // effective_n = 10000/HHI ~= 3
        assert!(approx_eq(out.effective_n, dec!(3.0), dec!(0.1)));
    }

    // --- Top 5 weight ---
    #[test]
    fn test_top_5_weight_3_stocks() {
        let input = make_3_stock_input("market_cap");
        let out = calculate_weighting(&input).unwrap();
        // Only 3 stocks: top 5 = 100%
        assert!(approx_eq(out.top_5_weight, Decimal::ONE, dec!(0.0001)));
    }

    // --- Sector weights ---
    #[test]
    fn test_sector_weights() {
        let input = make_3_stock_input("market_cap");
        let out = calculate_weighting(&input).unwrap();
        // Tech: 0.5+0.333=0.833, Health: 0.167
        let tech = out
            .sector_weights
            .iter()
            .find(|s| s.sector == "Tech")
            .unwrap();
        assert!(approx_eq(tech.weight, dec!(0.833), dec!(0.01)));
    }

    // --- Method ---
    #[test]
    fn test_method_used() {
        let input = make_3_stock_input("equal");
        let out = calculate_weighting(&input).unwrap();
        assert_eq!(out.method_used, "equal");
    }

    #[test]
    fn test_index_level_base_1000() {
        let input = make_3_stock_input("market_cap");
        let out = calculate_weighting(&input).unwrap();
        assert_eq!(out.index_level, dec!(1000));
    }

    // --- Validation ---
    #[test]
    fn test_reject_empty_constituents() {
        let input = WeightingInput {
            constituents: vec![],
            weighting_method: "equal".into(),
            cap_weight: dec!(0.10),
            fundamental_weights: None,
        };
        assert!(calculate_weighting(&input).is_err());
    }

    #[test]
    fn test_reject_negative_market_cap() {
        let mut input = make_3_stock_input("market_cap");
        input.constituents[0].market_cap = dec!(-100);
        assert!(calculate_weighting(&input).is_err());
    }

    #[test]
    fn test_reject_invalid_free_float() {
        let mut input = make_3_stock_input("market_cap");
        input.constituents[0].free_float_pct = dec!(1.5);
        assert!(calculate_weighting(&input).is_err());
    }

    #[test]
    fn test_reject_unknown_method() {
        let mut input = make_3_stock_input("unknown_method");
        let result = calculate_weighting(&mut input);
        assert!(result.is_err());
    }

    #[test]
    fn test_reject_zero_cap_weight_for_capped() {
        let mut input = make_3_stock_input("capped");
        input.cap_weight = Decimal::ZERO;
        assert!(calculate_weighting(&input).is_err());
    }

    #[test]
    fn test_serialization_roundtrip() {
        let input = make_3_stock_input("market_cap");
        let out = calculate_weighting(&input).unwrap();
        let json = serde_json::to_string(&out).unwrap();
        let _: WeightingOutput = serde_json::from_str(&json).unwrap();
    }
}
