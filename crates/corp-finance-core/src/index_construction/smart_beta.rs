//! Smart Beta / Factor Tilt Construction.
//!
//! Covers:
//! 1. **Factor Z-Scores** -- normalize each factor to z-score
//! 2. **Composite Scoring** -- weighted sum of tilts * z-scores
//! 3. **Weight Construction** -- from composite scores with cap/floor
//! 4. **Factor Exposures** -- weighted average factor scores
//! 5. **Active Share vs MCW** -- comparison to market-cap weights
//! 6. **Factor Purity** -- correlation of tilts to actual exposures
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

/// Compute mean of a slice.
fn mean(vals: &[Decimal]) -> Decimal {
    if vals.is_empty() {
        return Decimal::ZERO;
    }
    let sum: Decimal = vals.iter().copied().sum();
    sum / Decimal::from(vals.len() as u64)
}

/// Compute std dev (population) of a slice.
fn std_dev(vals: &[Decimal]) -> Decimal {
    if vals.len() < 2 {
        return Decimal::ZERO;
    }
    let m = mean(vals);
    let var: Decimal = vals.iter().map(|v| (*v - m) * (*v - m)).sum::<Decimal>()
        / Decimal::from(vals.len() as u64);
    decimal_sqrt(var)
}

/// Z-score normalization: (x - mean) / std.
fn z_scores(vals: &[Decimal]) -> Vec<Decimal> {
    let m = mean(vals);
    let s = std_dev(vals);
    if s.is_zero() {
        return vec![Decimal::ZERO; vals.len()];
    }
    vals.iter().map(|v| (*v - m) / s).collect()
}

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// A constituent with factor scores.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmartBetaConstituent {
    pub ticker: String,
    pub market_cap: Decimal,
    pub price: Decimal,
    pub beta: Decimal,
    pub momentum_score: Decimal,
    pub value_score: Decimal,
    pub quality_score: Decimal,
    pub volatility: Decimal,
    pub dividend_yield: Decimal,
}

/// Factor tilt configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactorTilts {
    pub value_tilt: Decimal,
    pub momentum_tilt: Decimal,
    pub quality_tilt: Decimal,
    pub low_vol_tilt: Decimal,
    pub dividend_tilt: Decimal,
}

/// Factor exposure output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactorExposures {
    pub value_exposure: Decimal,
    pub momentum_exposure: Decimal,
    pub quality_exposure: Decimal,
    pub low_vol_exposure: Decimal,
    pub dividend_exposure: Decimal,
}

/// Constituent weight result (reused from weighting module).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstituentWeight {
    pub ticker: String,
    pub weight: Decimal,
}

// ---------------------------------------------------------------------------
// Input / Output
// ---------------------------------------------------------------------------

/// Input for smart beta construction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmartBetaInput {
    pub constituents: Vec<SmartBetaConstituent>,
    pub factor_tilts: FactorTilts,
    /// Maximum weight per position.
    pub max_weight: Decimal,
    /// Minimum weight per position (floor, e.g. 0).
    pub min_weight: Decimal,
}

/// Output of smart beta construction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmartBetaOutput {
    pub weights: Vec<ConstituentWeight>,
    pub portfolio_beta: Decimal,
    pub portfolio_dividend_yield: Decimal,
    pub factor_exposures: FactorExposures,
    pub active_share_vs_mcw: Decimal,
    pub concentration_hhi: Decimal,
    pub num_holdings: u32,
    pub factor_purity: Decimal,
}

// ---------------------------------------------------------------------------
// Calculation
// ---------------------------------------------------------------------------

/// Construct smart-beta / factor-tilt weights.
pub fn calculate_smart_beta(input: &SmartBetaInput) -> CorpFinanceResult<SmartBetaOutput> {
    validate_smart_beta_input(input)?;

    let n = input.constituents.len();

    // Extract raw factor scores
    let values: Vec<Decimal> = input.constituents.iter().map(|c| c.value_score).collect();
    let momentums: Vec<Decimal> = input
        .constituents
        .iter()
        .map(|c| c.momentum_score)
        .collect();
    let qualities: Vec<Decimal> = input.constituents.iter().map(|c| c.quality_score).collect();
    // Low vol: invert volatility so lower vol gets higher z-score
    let inv_vols: Vec<Decimal> = input
        .constituents
        .iter()
        .map(|c| {
            if c.volatility.is_zero() {
                Decimal::ZERO
            } else {
                Decimal::ONE / c.volatility
            }
        })
        .collect();
    let dividends: Vec<Decimal> = input
        .constituents
        .iter()
        .map(|c| c.dividend_yield)
        .collect();

    // Z-score normalize
    let z_val = z_scores(&values);
    let z_mom = z_scores(&momentums);
    let z_qual = z_scores(&qualities);
    let z_lvol = z_scores(&inv_vols);
    let z_div = z_scores(&dividends);

    let tilts = &input.factor_tilts;

    // Composite score for each constituent
    let composites: Vec<Decimal> = (0..n)
        .map(|i| {
            tilts.value_tilt * z_val[i]
                + tilts.momentum_tilt * z_mom[i]
                + tilts.quality_tilt * z_qual[i]
                + tilts.low_vol_tilt * z_lvol[i]
                + tilts.dividend_tilt * z_div[i]
        })
        .collect();

    // Shift composites so minimum is 0, then normalize
    let min_composite = composites.iter().copied().min().unwrap_or(Decimal::ZERO);
    let shifted: Vec<Decimal> = composites.iter().map(|c| *c - min_composite).collect();

    let shifted_sum: Decimal = shifted.iter().copied().sum();

    // If all composites are identical (shifted all zero), fall back to equal weight
    let mut weights: Vec<Decimal> = if shifted_sum.is_zero() {
        vec![Decimal::ONE / Decimal::from(n as u64); n]
    } else {
        shifted.iter().map(|w| *w / shifted_sum).collect()
    };

    // Apply cap iteratively: cap excess weights, redistribute proportionally to uncapped
    for _ in 0..50 {
        let mut excess = Decimal::ZERO;
        let mut uncapped_total = Decimal::ZERO;
        let mut capped_flags: Vec<bool> = vec![false; weights.len()];

        for (i, w) in weights.iter().enumerate() {
            if *w > input.max_weight {
                excess += *w - input.max_weight;
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
                *w = input.max_weight;
            } else if !uncapped_total.is_zero() {
                *w += excess * (*w / uncapped_total);
            }
        }
    }

    // Apply floor: set minimum weights for non-zero positions
    if input.min_weight > Decimal::ZERO {
        for _ in 0..50 {
            let mut deficit = Decimal::ZERO;
            let mut above_floor_total = Decimal::ZERO;
            let mut floored_flags: Vec<bool> = vec![false; weights.len()];

            for (i, w) in weights.iter().enumerate() {
                if *w > Decimal::ZERO && *w < input.min_weight {
                    deficit += input.min_weight - *w;
                    floored_flags[i] = true;
                } else if *w >= input.min_weight {
                    above_floor_total += *w;
                }
            }

            if deficit.is_zero() {
                break;
            }

            for (i, w) in weights.iter_mut().enumerate() {
                if floored_flags[i] {
                    *w = input.min_weight;
                } else if *w >= input.min_weight && !above_floor_total.is_zero() {
                    *w -= deficit * (*w / above_floor_total);
                }
            }
        }
    }

    // Count holdings (weight > 0)
    let num_holdings = weights.iter().filter(|w| **w > Decimal::ZERO).count() as u32;

    // Portfolio beta
    let portfolio_beta: Decimal = input
        .constituents
        .iter()
        .zip(weights.iter())
        .map(|(c, w)| c.beta * *w)
        .sum();

    // Portfolio dividend yield
    let portfolio_dividend_yield: Decimal = input
        .constituents
        .iter()
        .zip(weights.iter())
        .map(|(c, w)| c.dividend_yield * *w)
        .sum();

    // Factor exposures = weighted average of raw factor scores
    let factor_exposures = FactorExposures {
        value_exposure: weighted_avg(&values, &weights),
        momentum_exposure: weighted_avg(&momentums, &weights),
        quality_exposure: weighted_avg(&qualities, &weights),
        low_vol_exposure: weighted_avg(&inv_vols, &weights),
        dividend_exposure: weighted_avg(&dividends, &weights),
    };

    // Active share vs MCW
    let total_mc: Decimal = input.constituents.iter().map(|c| c.market_cap).sum();
    let mcw_weights: Vec<Decimal> = if total_mc.is_zero() {
        vec![Decimal::ONE / Decimal::from(n as u64); n]
    } else {
        input
            .constituents
            .iter()
            .map(|c| c.market_cap / total_mc)
            .collect()
    };
    let active_share_vs_mcw: Decimal = weights
        .iter()
        .zip(mcw_weights.iter())
        .map(|(w, m)| (*w - *m).abs())
        .sum::<Decimal>()
        / dec!(2);

    // HHI
    let concentration_hhi: Decimal = weights
        .iter()
        .map(|w| {
            let pct = *w * dec!(100);
            pct * pct
        })
        .sum();

    // Factor purity: correlation of tilt vector with actual exposure deviations
    let factor_purity = calc_factor_purity(
        tilts,
        &factor_exposures,
        &values,
        &momentums,
        &qualities,
        &inv_vols,
        &dividends,
    );

    let weight_out: Vec<ConstituentWeight> = input
        .constituents
        .iter()
        .zip(weights.iter())
        .map(|(c, w)| ConstituentWeight {
            ticker: c.ticker.clone(),
            weight: *w,
        })
        .collect();

    Ok(SmartBetaOutput {
        weights: weight_out,
        portfolio_beta,
        portfolio_dividend_yield,
        factor_exposures,
        active_share_vs_mcw,
        concentration_hhi,
        num_holdings,
        factor_purity,
    })
}

fn weighted_avg(scores: &[Decimal], weights: &[Decimal]) -> Decimal {
    scores
        .iter()
        .zip(weights.iter())
        .map(|(s, w)| *s * *w)
        .sum()
}

/// Factor purity: how well tilts correlate with actual exposures.
/// Simplified: cosine similarity between tilt vector and z-scored exposure vector.
fn calc_factor_purity(
    tilts: &FactorTilts,
    exposures: &FactorExposures,
    _values: &[Decimal],
    _momentums: &[Decimal],
    _qualities: &[Decimal],
    _inv_vols: &[Decimal],
    _dividends: &[Decimal],
) -> Decimal {
    // Normalize tilts and exposures to unit vectors and compute dot product
    let tilt_vec = [
        tilts.value_tilt,
        tilts.momentum_tilt,
        tilts.quality_tilt,
        tilts.low_vol_tilt,
        tilts.dividend_tilt,
    ];
    let exp_vec = [
        exposures.value_exposure,
        exposures.momentum_exposure,
        exposures.quality_exposure,
        exposures.low_vol_exposure,
        exposures.dividend_exposure,
    ];

    let dot: Decimal = tilt_vec
        .iter()
        .zip(exp_vec.iter())
        .map(|(t, e)| *t * *e)
        .sum();
    let norm_t = decimal_sqrt(tilt_vec.iter().map(|t| *t * *t).sum());
    let norm_e = decimal_sqrt(exp_vec.iter().map(|e| *e * *e).sum());

    if norm_t.is_zero() || norm_e.is_zero() {
        return Decimal::ZERO;
    }
    dot / (norm_t * norm_e)
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

fn validate_smart_beta_input(input: &SmartBetaInput) -> CorpFinanceResult<()> {
    if input.constituents.is_empty() {
        return Err(CorpFinanceError::InsufficientData(
            "At least one constituent is required".into(),
        ));
    }
    if input.max_weight <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "max_weight".into(),
            reason: "Max weight must be positive".into(),
        });
    }
    if input.min_weight < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "min_weight".into(),
            reason: "Min weight must be non-negative".into(),
        });
    }
    if input.min_weight > input.max_weight {
        return Err(CorpFinanceError::InvalidInput {
            field: "min_weight".into(),
            reason: "Min weight must not exceed max weight".into(),
        });
    }
    for c in &input.constituents {
        if c.market_cap < Decimal::ZERO {
            return Err(CorpFinanceError::InvalidInput {
                field: "market_cap".into(),
                reason: format!("Negative market cap for {}", c.ticker),
            });
        }
        if c.volatility < Decimal::ZERO {
            return Err(CorpFinanceError::InvalidInput {
                field: "volatility".into(),
                reason: format!("Negative volatility for {}", c.ticker),
            });
        }
    }
    // Validate tilt range
    let tilts = &input.factor_tilts;
    for (name, val) in [
        ("value_tilt", tilts.value_tilt),
        ("momentum_tilt", tilts.momentum_tilt),
        ("quality_tilt", tilts.quality_tilt),
        ("low_vol_tilt", tilts.low_vol_tilt),
        ("dividend_tilt", tilts.dividend_tilt),
    ] {
        if val < Decimal::ZERO || val > Decimal::ONE {
            return Err(CorpFinanceError::InvalidInput {
                field: name.into(),
                reason: "Tilt must be between 0 and 1".into(),
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

    fn make_constituent(
        ticker: &str,
        mc: Decimal,
        val: Decimal,
        mom: Decimal,
        qual: Decimal,
        vol: Decimal,
        div: Decimal,
    ) -> SmartBetaConstituent {
        SmartBetaConstituent {
            ticker: ticker.into(),
            market_cap: mc,
            price: dec!(100),
            beta: dec!(1.0),
            momentum_score: mom,
            value_score: val,
            quality_score: qual,
            volatility: vol,
            dividend_yield: div,
        }
    }

    fn make_base_input() -> SmartBetaInput {
        SmartBetaInput {
            constituents: vec![
                make_constituent(
                    "A",
                    dec!(5000),
                    dec!(0.8),
                    dec!(0.10),
                    dec!(0.15),
                    dec!(0.20),
                    dec!(0.03),
                ),
                make_constituent(
                    "B",
                    dec!(3000),
                    dec!(0.5),
                    dec!(0.20),
                    dec!(0.12),
                    dec!(0.25),
                    dec!(0.02),
                ),
                make_constituent(
                    "C",
                    dec!(2000),
                    dec!(0.3),
                    dec!(0.05),
                    dec!(0.20),
                    dec!(0.15),
                    dec!(0.04),
                ),
                make_constituent(
                    "D",
                    dec!(4000),
                    dec!(0.6),
                    dec!(0.15),
                    dec!(0.18),
                    dec!(0.30),
                    dec!(0.01),
                ),
                make_constituent(
                    "E",
                    dec!(1000),
                    dec!(0.9),
                    dec!(-0.05),
                    dec!(0.10),
                    dec!(0.35),
                    dec!(0.05),
                ),
            ],
            factor_tilts: FactorTilts {
                value_tilt: dec!(0.20),
                momentum_tilt: dec!(0.20),
                quality_tilt: dec!(0.20),
                low_vol_tilt: dec!(0.20),
                dividend_tilt: dec!(0.20),
            },
            max_weight: dec!(0.40),
            min_weight: Decimal::ZERO,
        }
    }

    // --- Pure value tilt ---
    #[test]
    fn test_pure_value_tilt() {
        let mut input = make_base_input();
        input.factor_tilts = FactorTilts {
            value_tilt: Decimal::ONE,
            momentum_tilt: Decimal::ZERO,
            quality_tilt: Decimal::ZERO,
            low_vol_tilt: Decimal::ZERO,
            dividend_tilt: Decimal::ZERO,
        };
        let out = calculate_smart_beta(&input).unwrap();
        // E has highest value (0.9), should have highest weight
        let e_weight = out.weights.iter().find(|w| w.ticker == "E").unwrap().weight;
        let b_weight = out.weights.iter().find(|w| w.ticker == "B").unwrap().weight;
        assert!(e_weight > b_weight);
    }

    // --- Pure momentum tilt ---
    #[test]
    fn test_pure_momentum_tilt() {
        let mut input = make_base_input();
        input.factor_tilts = FactorTilts {
            value_tilt: Decimal::ZERO,
            momentum_tilt: Decimal::ONE,
            quality_tilt: Decimal::ZERO,
            low_vol_tilt: Decimal::ZERO,
            dividend_tilt: Decimal::ZERO,
        };
        let out = calculate_smart_beta(&input).unwrap();
        // B has highest momentum (0.20)
        let b_weight = out.weights.iter().find(|w| w.ticker == "B").unwrap().weight;
        let e_weight = out.weights.iter().find(|w| w.ticker == "E").unwrap().weight;
        assert!(b_weight > e_weight);
    }

    // --- Multi-factor ---
    #[test]
    fn test_multi_factor_weights_sum_to_one() {
        let input = make_base_input();
        let out = calculate_smart_beta(&input).unwrap();
        let total: Decimal = out.weights.iter().map(|w| w.weight).sum();
        assert!(approx_eq(total, Decimal::ONE, dec!(0.001)));
    }

    #[test]
    fn test_multi_factor_all_weights_non_negative() {
        let input = make_base_input();
        let out = calculate_smart_beta(&input).unwrap();
        for w in &out.weights {
            assert!(w.weight >= Decimal::ZERO);
        }
    }

    // --- Capping ---
    #[test]
    fn test_capping_enforced() {
        let mut input = make_base_input();
        input.max_weight = dec!(0.25);
        let out = calculate_smart_beta(&input).unwrap();
        for w in &out.weights {
            assert!(
                w.weight <= dec!(0.25) + dec!(0.01),
                "Weight {} for {} exceeds cap",
                w.weight,
                w.ticker
            );
        }
    }

    #[test]
    fn test_capping_still_sums_to_one() {
        let mut input = make_base_input();
        input.max_weight = dec!(0.25);
        let out = calculate_smart_beta(&input).unwrap();
        let total: Decimal = out.weights.iter().map(|w| w.weight).sum();
        assert!(approx_eq(total, Decimal::ONE, dec!(0.01)));
    }

    // --- Low vol strategy ---
    #[test]
    fn test_low_vol_tilt() {
        let mut input = make_base_input();
        input.factor_tilts = FactorTilts {
            value_tilt: Decimal::ZERO,
            momentum_tilt: Decimal::ZERO,
            quality_tilt: Decimal::ZERO,
            low_vol_tilt: Decimal::ONE,
            dividend_tilt: Decimal::ZERO,
        };
        let out = calculate_smart_beta(&input).unwrap();
        // C has lowest vol (0.15), should get highest weight
        let c_weight = out.weights.iter().find(|w| w.ticker == "C").unwrap().weight;
        let e_weight = out.weights.iter().find(|w| w.ticker == "E").unwrap().weight;
        assert!(c_weight > e_weight);
    }

    // --- Equal tilts ---
    #[test]
    fn test_equal_tilts_diversified() {
        let input = make_base_input();
        let out = calculate_smart_beta(&input).unwrap();
        // With equal tilts, at least some holdings remain and concentration is moderate
        assert!(out.num_holdings >= 2);
        // No single holding should have > 60%
        for w in &out.weights {
            assert!(w.weight <= dec!(0.60));
        }
    }

    // --- Portfolio beta ---
    #[test]
    fn test_portfolio_beta() {
        let input = make_base_input();
        let out = calculate_smart_beta(&input).unwrap();
        // All betas are 1.0, so portfolio beta = sum of weights * 1.0 = sum of weights
        // With normalization, sum = 1.0 so beta should be 1.0
        let total_w: Decimal = out.weights.iter().map(|w| w.weight).sum();
        assert!(approx_eq(out.portfolio_beta, total_w, dec!(0.001)));
    }

    // --- Dividend yield ---
    #[test]
    fn test_portfolio_dividend_yield() {
        let input = make_base_input();
        let out = calculate_smart_beta(&input).unwrap();
        assert!(out.portfolio_dividend_yield > Decimal::ZERO);
    }

    // --- Factor exposures ---
    #[test]
    fn test_factor_exposures_positive() {
        let input = make_base_input();
        let out = calculate_smart_beta(&input).unwrap();
        assert!(out.factor_exposures.value_exposure > Decimal::ZERO);
        assert!(out.factor_exposures.quality_exposure > Decimal::ZERO);
    }

    // --- Active share vs MCW ---
    #[test]
    fn test_active_share_vs_mcw() {
        let input = make_base_input();
        let out = calculate_smart_beta(&input).unwrap();
        assert!(out.active_share_vs_mcw > Decimal::ZERO);
        assert!(out.active_share_vs_mcw <= Decimal::ONE);
    }

    // --- HHI ---
    #[test]
    fn test_concentration_hhi() {
        let input = make_base_input();
        let out = calculate_smart_beta(&input).unwrap();
        assert!(out.concentration_hhi > Decimal::ZERO);
        assert!(out.concentration_hhi <= dec!(10000));
    }

    // --- Factor purity ---
    #[test]
    fn test_factor_purity() {
        let input = make_base_input();
        let out = calculate_smart_beta(&input).unwrap();
        // Factor purity should be between -1 and 1 (cosine similarity)
        assert!(out.factor_purity >= dec!(-1));
        assert!(out.factor_purity <= Decimal::ONE + dec!(0.001));
    }

    // --- Empty after filtering ---
    #[test]
    fn test_all_negative_composite_fallback_equal() {
        let mut input = make_base_input();
        // Set all factor scores to zero -> z_scores all zero -> composites zero -> fallback equal
        for c in &mut input.constituents {
            c.value_score = dec!(0.50);
            c.momentum_score = dec!(0.50);
            c.quality_score = dec!(0.50);
            c.volatility = dec!(0.20);
            c.dividend_yield = dec!(0.03);
        }
        let out = calculate_smart_beta(&input).unwrap();
        // Should fallback to equal weight since all z-scores are 0
        let expected = Decimal::ONE / dec!(5);
        for w in &out.weights {
            assert!(approx_eq(w.weight, expected, dec!(0.01)));
        }
    }

    // --- Num holdings ---
    #[test]
    fn test_num_holdings() {
        let input = make_base_input();
        let out = calculate_smart_beta(&input).unwrap();
        assert!(out.num_holdings <= input.constituents.len() as u32);
        assert!(out.num_holdings > 0);
    }

    // --- Validation ---
    #[test]
    fn test_reject_empty_constituents() {
        let mut input = make_base_input();
        input.constituents = vec![];
        assert!(calculate_smart_beta(&input).is_err());
    }

    #[test]
    fn test_reject_zero_max_weight() {
        let mut input = make_base_input();
        input.max_weight = Decimal::ZERO;
        assert!(calculate_smart_beta(&input).is_err());
    }

    #[test]
    fn test_reject_negative_min_weight() {
        let mut input = make_base_input();
        input.min_weight = dec!(-0.01);
        assert!(calculate_smart_beta(&input).is_err());
    }

    #[test]
    fn test_reject_min_exceeds_max() {
        let mut input = make_base_input();
        input.min_weight = dec!(0.50);
        input.max_weight = dec!(0.10);
        assert!(calculate_smart_beta(&input).is_err());
    }

    #[test]
    fn test_reject_negative_volatility() {
        let mut input = make_base_input();
        input.constituents[0].volatility = dec!(-0.10);
        assert!(calculate_smart_beta(&input).is_err());
    }

    #[test]
    fn test_reject_tilt_out_of_range() {
        let mut input = make_base_input();
        input.factor_tilts.value_tilt = dec!(1.5);
        assert!(calculate_smart_beta(&input).is_err());
    }

    #[test]
    fn test_serialization_roundtrip() {
        let input = make_base_input();
        let out = calculate_smart_beta(&input).unwrap();
        let json = serde_json::to_string(&out).unwrap();
        let _: SmartBetaOutput = serde_json::from_str(&json).unwrap();
    }
}
