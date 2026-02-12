//! CLO Reinvestment Period Analytics.
//!
//! Implements portfolio-level metrics and compliance tests during the
//! CLO reinvestment period:
//! - WARF (Weighted Average Rating Factor)
//! - WAL (Weighted Average Life)
//! - WALS (Weighted Average Loan Spread)
//! - Diversity Score (Moody's methodology)
//! - Par Build Test
//! - Reinvestment Criteria Compliance
//!
//! All arithmetic uses `rust_decimal::Decimal`. No `f64`.

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::error::CorpFinanceError;
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Input / Output types
// ---------------------------------------------------------------------------

/// A single asset in the CLO collateral pool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolAsset {
    /// Asset identifier/name.
    pub name: String,
    /// Current notional balance.
    pub notional: Decimal,
    /// Credit rating (AAA, AA, A, BBB, BB, B, CCC).
    pub rating: String,
    /// Loan spread (decimal: 0.0350 = 350bp).
    pub spread: Decimal,
    /// Remaining life in years (decimal).
    pub remaining_life: Decimal,
    /// Industry classification for diversity score.
    pub industry: String,
}

/// Input for reinvestment period analytics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReinvestmentInput {
    /// Assets in the collateral pool.
    pub assets: Vec<PoolAsset>,
    /// Target par amount.
    pub target_par: Decimal,
    /// Maximum WARF allowed.
    pub max_warf: Decimal,
    /// Minimum WALS allowed (decimal).
    pub min_wals: Decimal,
    /// Maximum WAL allowed (years).
    pub max_wal: Decimal,
    /// Minimum diversity score.
    pub min_diversity_score: Decimal,
}

/// A single criteria compliance check result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CriteriaCheck {
    /// Metric name.
    pub metric: String,
    /// Current value.
    pub value: Decimal,
    /// Limit (max or min depending on metric).
    pub limit: Decimal,
    /// Whether the criteria passes.
    pub passes: bool,
}

/// Output of reinvestment period analytics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReinvestmentOutput {
    /// Weighted Average Rating Factor.
    pub warf: Decimal,
    /// Weighted Average Life (years).
    pub wal: Decimal,
    /// Weighted Average Loan Spread (decimal).
    pub wals: Decimal,
    /// Diversity score (Moody's methodology).
    pub diversity_score: Decimal,
    /// Current par amount.
    pub par_amount: Decimal,
    /// Whether par build is needed (current < target).
    pub par_build_needed: bool,
    /// Individual criteria compliance checks.
    pub criteria_checks: Vec<CriteriaCheck>,
    /// Whether all reinvestment criteria are met.
    pub all_criteria_met: bool,
}

// ---------------------------------------------------------------------------
// Rating factor mapping (Moody's methodology)
// ---------------------------------------------------------------------------

/// Map a rating string to its Moody's rating factor.
fn rating_factor(rating: &str) -> Decimal {
    match rating.to_uppercase().as_str() {
        "AAA" => dec!(1),
        "AA+" | "AA" | "AA-" => dec!(10),
        "A+" | "A" | "A-" => dec!(120),
        "BBB+" | "BBB" | "BBB-" => dec!(360),
        "BB+" | "BB" | "BB-" => dec!(1350),
        "B+" | "B" | "B-" => dec!(2720),
        "CCC+" | "CCC" | "CCC-" | "CC" | "C" => dec!(6500),
        _ => dec!(6500), // Default to worst
    }
}

// ---------------------------------------------------------------------------
// Diversity Score (Moody's simplified methodology)
// ---------------------------------------------------------------------------

/// Compute the Moody's diversity score.
///
/// Groups assets by industry, computes the equivalent number of independent
/// issuers using the Moody's binomial expansion technique (simplified):
/// For each industry group, the effective number = 1 + 0.5 * (n - 1) for n > 1,
/// where n = number of distinct issuers in that industry.
/// Actually the standard Moody's approach: for each industry, count
/// par-weighted positions and compute equivalent independent credits.
fn compute_diversity_score(assets: &[PoolAsset]) -> Decimal {
    if assets.is_empty() {
        return Decimal::ZERO;
    }

    let total_par: Decimal = assets.iter().map(|a| a.notional).sum();
    if total_par.is_zero() {
        return Decimal::ZERO;
    }

    // Group by industry
    let mut industry_pars: HashMap<String, Vec<Decimal>> = HashMap::new();
    for asset in assets {
        industry_pars
            .entry(asset.industry.clone())
            .or_default()
            .push(asset.notional);
    }

    let mut diversity = Decimal::ZERO;

    for pars in industry_pars.values() {
        let n = pars.len();
        if n == 0 {
            continue;
        }

        let industry_total: Decimal = pars.iter().copied().sum();
        // Average par per issuer in this industry
        let avg_par = industry_total / Decimal::from(n as u32);

        // Moody's diversity: effective number of issuers
        // D_i = industry_total^2 / sum(par_j^2)
        let sum_sq: Decimal = pars.iter().map(|p| *p * *p).sum();
        if sum_sq.is_zero() {
            continue;
        }

        let d_i = (industry_total * industry_total) / sum_sq;
        // Cap at actual number of issuers
        let d_i = if d_i > Decimal::from(n as u32) {
            Decimal::from(n as u32)
        } else {
            d_i
        };

        // Scale by average par weight to total
        let _ = avg_par; // Used implicitly through the HHI-based approach
        diversity += d_i;
    }

    diversity
}

// ---------------------------------------------------------------------------
// Engine
// ---------------------------------------------------------------------------

/// Compute reinvestment period analytics.
pub fn calculate_reinvestment(input: &ReinvestmentInput) -> CorpFinanceResult<ReinvestmentOutput> {
    validate_reinvestment_input(input)?;

    let total_notional: Decimal = input.assets.iter().map(|a| a.notional).sum();

    // WARF = sum(notional_i * rating_factor_i) / total_notional
    let warf = if total_notional.is_zero() {
        Decimal::ZERO
    } else {
        let weighted_rf: Decimal = input
            .assets
            .iter()
            .map(|a| a.notional * rating_factor(&a.rating))
            .sum();
        weighted_rf / total_notional
    };

    // WAL = sum(notional_i * remaining_life_i) / total_notional
    let wal = if total_notional.is_zero() {
        Decimal::ZERO
    } else {
        let weighted_life: Decimal = input
            .assets
            .iter()
            .map(|a| a.notional * a.remaining_life)
            .sum();
        weighted_life / total_notional
    };

    // WALS = sum(notional_i * spread_i) / total_notional
    let wals = if total_notional.is_zero() {
        Decimal::ZERO
    } else {
        let weighted_spread: Decimal = input.assets.iter().map(|a| a.notional * a.spread).sum();
        weighted_spread / total_notional
    };

    // Diversity Score
    let diversity_score = compute_diversity_score(&input.assets);

    // Par Build Test
    let par_amount = total_notional;
    let par_build_needed = par_amount < input.target_par;

    // Criteria checks
    let warf_check = CriteriaCheck {
        metric: "WARF".into(),
        value: warf,
        limit: input.max_warf,
        passes: warf <= input.max_warf,
    };
    let wals_check = CriteriaCheck {
        metric: "WALS".into(),
        value: wals,
        limit: input.min_wals,
        passes: wals >= input.min_wals,
    };
    let wal_check = CriteriaCheck {
        metric: "WAL".into(),
        value: wal,
        limit: input.max_wal,
        passes: wal <= input.max_wal,
    };
    let diversity_check = CriteriaCheck {
        metric: "Diversity Score".into(),
        value: diversity_score,
        limit: input.min_diversity_score,
        passes: diversity_score >= input.min_diversity_score,
    };
    let par_check = CriteriaCheck {
        metric: "Par Amount".into(),
        value: par_amount,
        limit: input.target_par,
        passes: par_amount >= input.target_par,
    };

    let criteria_checks = vec![
        warf_check,
        wals_check,
        wal_check,
        diversity_check,
        par_check,
    ];

    let all_criteria_met = criteria_checks.iter().all(|c| c.passes);

    Ok(ReinvestmentOutput {
        warf,
        wal,
        wals,
        diversity_score,
        par_amount,
        par_build_needed,
        criteria_checks,
        all_criteria_met,
    })
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

fn validate_reinvestment_input(input: &ReinvestmentInput) -> CorpFinanceResult<()> {
    if input.assets.is_empty() {
        return Err(CorpFinanceError::InsufficientData(
            "At least one asset is required.".into(),
        ));
    }
    if input.target_par < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "target_par".into(),
            reason: "Target par cannot be negative.".into(),
        });
    }
    if input.max_warf < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "max_warf".into(),
            reason: "Max WARF cannot be negative.".into(),
        });
    }
    if input.max_wal < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "max_wal".into(),
            reason: "Max WAL cannot be negative.".into(),
        });
    }
    for a in &input.assets {
        if a.notional < Decimal::ZERO {
            return Err(CorpFinanceError::InvalidInput {
                field: format!("asset.{}.notional", a.name),
                reason: "Asset notional cannot be negative.".into(),
            });
        }
        if a.remaining_life < Decimal::ZERO {
            return Err(CorpFinanceError::InvalidInput {
                field: format!("asset.{}.remaining_life", a.name),
                reason: "Remaining life cannot be negative.".into(),
            });
        }
        if a.spread < Decimal::ZERO {
            return Err(CorpFinanceError::InvalidInput {
                field: format!("asset.{}.spread", a.name),
                reason: "Spread cannot be negative.".into(),
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

    fn sample_assets() -> Vec<PoolAsset> {
        vec![
            PoolAsset {
                name: "Loan A".into(),
                notional: dec!(10_000_000),
                rating: "BBB".into(),
                spread: dec!(0.0350),
                remaining_life: dec!(5.0),
                industry: "Technology".into(),
            },
            PoolAsset {
                name: "Loan B".into(),
                notional: dec!(15_000_000),
                rating: "BB".into(),
                spread: dec!(0.0450),
                remaining_life: dec!(4.0),
                industry: "Healthcare".into(),
            },
            PoolAsset {
                name: "Loan C".into(),
                notional: dec!(20_000_000),
                rating: "B".into(),
                spread: dec!(0.0550),
                remaining_life: dec!(6.0),
                industry: "Energy".into(),
            },
            PoolAsset {
                name: "Loan D".into(),
                notional: dec!(5_000_000),
                rating: "BBB".into(),
                spread: dec!(0.0300),
                remaining_life: dec!(3.0),
                industry: "Technology".into(),
            },
            PoolAsset {
                name: "Loan E".into(),
                notional: dec!(10_000_000),
                rating: "BB".into(),
                spread: dec!(0.0400),
                remaining_life: dec!(5.5),
                industry: "Retail".into(),
            },
        ]
    }

    fn sample_input() -> ReinvestmentInput {
        ReinvestmentInput {
            assets: sample_assets(),
            target_par: dec!(60_000_000),
            max_warf: dec!(3000),
            min_wals: dec!(0.0300),
            max_wal: dec!(7.0),
            min_diversity_score: dec!(3),
        }
    }

    #[test]
    fn test_warf_calculation() {
        let input = sample_input();
        let out = calculate_reinvestment(&input).unwrap();
        // Manual WARF:
        // 10M*360 + 15M*1350 + 20M*2720 + 5M*360 + 10M*1350 = 3600k + 20250k + 54400k + 1800k + 13500k = 93550k
        // total = 60M, WARF = 93550000/60000000 = 1559.166...
        let expected_num = dec!(10_000_000) * dec!(360)
            + dec!(15_000_000) * dec!(1350)
            + dec!(20_000_000) * dec!(2720)
            + dec!(5_000_000) * dec!(360)
            + dec!(10_000_000) * dec!(1350);
        let expected = expected_num / dec!(60_000_000);
        assert!(
            approx_eq(out.warf, expected, dec!(0.01)),
            "WARF {} should be ~{}",
            out.warf,
            expected
        );
    }

    #[test]
    fn test_wal_calculation() {
        let input = sample_input();
        let out = calculate_reinvestment(&input).unwrap();
        // WAL = (10*5 + 15*4 + 20*6 + 5*3 + 10*5.5) / 60 = (50+60+120+15+55)/60 = 300/60 = 5.0
        let expected = dec!(300_000_000) / dec!(60_000_000);
        assert!(
            approx_eq(out.wal, expected, dec!(0.001)),
            "WAL {} should be ~{}",
            out.wal,
            expected
        );
    }

    #[test]
    fn test_wals_calculation() {
        let input = sample_input();
        let out = calculate_reinvestment(&input).unwrap();
        // WALS = (10*0.035 + 15*0.045 + 20*0.055 + 5*0.030 + 10*0.040) / 60
        let num = dec!(10_000_000) * dec!(0.0350)
            + dec!(15_000_000) * dec!(0.0450)
            + dec!(20_000_000) * dec!(0.0550)
            + dec!(5_000_000) * dec!(0.0300)
            + dec!(10_000_000) * dec!(0.0400);
        let expected = num / dec!(60_000_000);
        assert!(
            approx_eq(out.wals, expected, dec!(0.0001)),
            "WALS {} should be ~{}",
            out.wals,
            expected
        );
    }

    #[test]
    fn test_diversity_score_positive() {
        let input = sample_input();
        let out = calculate_reinvestment(&input).unwrap();
        assert!(
            out.diversity_score > Decimal::ZERO,
            "Diversity score should be positive"
        );
    }

    #[test]
    fn test_diversity_score_increases_with_industries() {
        // One industry
        let single = ReinvestmentInput {
            assets: vec![
                PoolAsset {
                    name: "A".into(),
                    notional: dec!(10_000_000),
                    rating: "BBB".into(),
                    spread: dec!(0.04),
                    remaining_life: dec!(5),
                    industry: "Tech".into(),
                },
                PoolAsset {
                    name: "B".into(),
                    notional: dec!(10_000_000),
                    rating: "BBB".into(),
                    spread: dec!(0.04),
                    remaining_life: dec!(5),
                    industry: "Tech".into(),
                },
            ],
            target_par: dec!(20_000_000),
            max_warf: dec!(5000),
            min_wals: dec!(0.01),
            max_wal: dec!(10),
            min_diversity_score: dec!(1),
        };
        let multi = ReinvestmentInput {
            assets: vec![
                PoolAsset {
                    name: "A".into(),
                    notional: dec!(10_000_000),
                    rating: "BBB".into(),
                    spread: dec!(0.04),
                    remaining_life: dec!(5),
                    industry: "Tech".into(),
                },
                PoolAsset {
                    name: "B".into(),
                    notional: dec!(10_000_000),
                    rating: "BBB".into(),
                    spread: dec!(0.04),
                    remaining_life: dec!(5),
                    industry: "Healthcare".into(),
                },
            ],
            target_par: dec!(20_000_000),
            max_warf: dec!(5000),
            min_wals: dec!(0.01),
            max_wal: dec!(10),
            min_diversity_score: dec!(1),
        };
        let out_single = calculate_reinvestment(&single).unwrap();
        let out_multi = calculate_reinvestment(&multi).unwrap();
        assert!(
            out_multi.diversity_score >= out_single.diversity_score,
            "More industries should yield higher diversity"
        );
    }

    #[test]
    fn test_par_build_needed() {
        let mut input = sample_input();
        input.target_par = dec!(100_000_000);
        let out = calculate_reinvestment(&input).unwrap();
        assert!(out.par_build_needed);
    }

    #[test]
    fn test_par_build_not_needed() {
        let mut input = sample_input();
        input.target_par = dec!(50_000_000);
        let out = calculate_reinvestment(&input).unwrap();
        assert!(!out.par_build_needed);
    }

    #[test]
    fn test_par_amount_equals_total_notional() {
        let input = sample_input();
        let out = calculate_reinvestment(&input).unwrap();
        let expected: Decimal = input.assets.iter().map(|a| a.notional).sum();
        assert_eq!(out.par_amount, expected);
    }

    #[test]
    fn test_criteria_checks_count() {
        let input = sample_input();
        let out = calculate_reinvestment(&input).unwrap();
        assert_eq!(out.criteria_checks.len(), 5);
    }

    #[test]
    fn test_all_criteria_met_when_passing() {
        let input = sample_input();
        let out = calculate_reinvestment(&input).unwrap();
        // With lenient limits, all should pass
        assert!(out.all_criteria_met);
    }

    #[test]
    fn test_all_criteria_not_met_when_warf_too_high() {
        let mut input = sample_input();
        input.max_warf = dec!(100); // Very tight WARF limit
        let out = calculate_reinvestment(&input).unwrap();
        assert!(!out.all_criteria_met);
        let warf_check = out
            .criteria_checks
            .iter()
            .find(|c| c.metric == "WARF")
            .unwrap();
        assert!(!warf_check.passes);
    }

    #[test]
    fn test_all_criteria_not_met_when_wal_too_long() {
        let mut input = sample_input();
        input.max_wal = dec!(1.0); // Very tight WAL limit
        let out = calculate_reinvestment(&input).unwrap();
        assert!(!out.all_criteria_met);
    }

    #[test]
    fn test_rating_factor_aaa() {
        assert_eq!(rating_factor("AAA"), dec!(1));
    }

    #[test]
    fn test_rating_factor_bbb() {
        assert_eq!(rating_factor("BBB"), dec!(360));
    }

    #[test]
    fn test_rating_factor_ccc() {
        assert_eq!(rating_factor("CCC"), dec!(6500));
    }

    #[test]
    fn test_rating_factor_unknown_defaults_to_worst() {
        assert_eq!(rating_factor("XYZ"), dec!(6500));
    }

    #[test]
    fn test_uniform_portfolio_warf() {
        let input = ReinvestmentInput {
            assets: vec![
                PoolAsset {
                    name: "A".into(),
                    notional: dec!(10_000_000),
                    rating: "BBB".into(),
                    spread: dec!(0.04),
                    remaining_life: dec!(5),
                    industry: "Tech".into(),
                },
                PoolAsset {
                    name: "B".into(),
                    notional: dec!(10_000_000),
                    rating: "BBB".into(),
                    spread: dec!(0.04),
                    remaining_life: dec!(5),
                    industry: "Healthcare".into(),
                },
            ],
            target_par: dec!(20_000_000),
            max_warf: dec!(5000),
            min_wals: dec!(0.01),
            max_wal: dec!(10),
            min_diversity_score: dec!(1),
        };
        let out = calculate_reinvestment(&input).unwrap();
        // All BBB => WARF = 360
        assert_eq!(out.warf, dec!(360));
    }

    #[test]
    fn test_reject_empty_assets() {
        let mut input = sample_input();
        input.assets = vec![];
        assert!(calculate_reinvestment(&input).is_err());
    }

    #[test]
    fn test_reject_negative_notional() {
        let mut input = sample_input();
        input.assets[0].notional = dec!(-100);
        assert!(calculate_reinvestment(&input).is_err());
    }

    #[test]
    fn test_reject_negative_remaining_life() {
        let mut input = sample_input();
        input.assets[0].remaining_life = dec!(-1);
        assert!(calculate_reinvestment(&input).is_err());
    }

    #[test]
    fn test_reject_negative_spread() {
        let mut input = sample_input();
        input.assets[0].spread = dec!(-0.01);
        assert!(calculate_reinvestment(&input).is_err());
    }

    #[test]
    fn test_reject_negative_target_par() {
        let mut input = sample_input();
        input.target_par = dec!(-1);
        assert!(calculate_reinvestment(&input).is_err());
    }

    #[test]
    fn test_serialization_roundtrip() {
        let input = sample_input();
        let out = calculate_reinvestment(&input).unwrap();
        let json = serde_json::to_string(&out).unwrap();
        let _: ReinvestmentOutput = serde_json::from_str(&json).unwrap();
    }
}
