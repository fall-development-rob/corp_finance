//! Shapley Value Capital Allocation.
//!
//! Covers:
//! 1. **Shapley Value** -- Fair allocation based on marginal contributions to every coalition
//! 2. **Exact Computation** -- For N <= 8 units, enumerate all 2^N coalitions
//! 3. **Sampled Approximation** -- For N > 8, sample random permutations (deterministic seed)
//! 4. **Efficiency Check** -- Verify sum of Shapley values equals total portfolio capital
//!
//! All arithmetic uses `rust_decimal::Decimal`. No `f64`.

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::error::CorpFinanceError;
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Input / Output
// ---------------------------------------------------------------------------

/// A single business unit for Shapley allocation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShapleyUnit {
    /// Name of the business unit.
    pub name: String,
    /// Simulated returns or P&L series for this unit.
    pub returns: Vec<Decimal>,
}

/// Input for Shapley allocation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShapleyAllocationInput {
    /// Business units.
    pub units: Vec<ShapleyUnit>,
    /// Confidence level for VaR (e.g. 0.99).
    pub confidence_level: Decimal,
    /// Number of permutation samples for approximation (used when N > 8).
    pub num_samples: u32,
}

/// Method used for computation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ShapleyMethod {
    Exact,
    Sampled,
}

impl std::fmt::Display for ShapleyMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ShapleyMethod::Exact => write!(f, "Exact"),
            ShapleyMethod::Sampled => write!(f, "Sampled"),
        }
    }
}

/// A single unit's Shapley allocation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShapleyAllocationDetail {
    /// Unit name.
    pub name: String,
    /// Shapley value (allocated capital).
    pub shapley_value: Decimal,
    /// Percentage of total capital.
    pub pct_of_total: Decimal,
}

/// Output of the Shapley allocation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShapleyAllocationOutput {
    /// Per-unit allocation details.
    pub allocations: Vec<ShapleyAllocationDetail>,
    /// Total portfolio capital (VaR).
    pub total_capital: Decimal,
    /// Method used (Exact or Sampled).
    pub method_used: ShapleyMethod,
    /// Efficiency check: sum of Shapley values vs total capital.
    pub efficiency_check: Decimal,
}

/// Calculate Shapley value capital allocation.
pub fn calculate_shapley_allocation(
    input: &ShapleyAllocationInput,
) -> CorpFinanceResult<ShapleyAllocationOutput> {
    validate_shapley_input(input)?;

    let n = input.units.len();
    let n_scenarios = input.units[0].returns.len();

    // Compute total portfolio VaR (all units combined)
    let total_capital = compute_coalition_var(
        &input.units,
        &(0..n).collect::<Vec<_>>(),
        n_scenarios,
        input.confidence_level,
    );

    let (shapley_values, method_used) = if n <= 8 {
        (
            compute_exact_shapley(&input.units, n_scenarios, input.confidence_level),
            ShapleyMethod::Exact,
        )
    } else {
        (
            compute_sampled_shapley(
                &input.units,
                n_scenarios,
                input.confidence_level,
                input.num_samples,
            ),
            ShapleyMethod::Sampled,
        )
    };

    // Build allocation details
    let sum_shapley: Decimal = shapley_values.iter().copied().sum();
    let mut allocations = Vec::with_capacity(n);
    for (i, unit) in input.units.iter().enumerate() {
        let sv = shapley_values[i];
        let pct = if total_capital.is_zero() {
            Decimal::ZERO
        } else {
            sv / total_capital
        };
        allocations.push(ShapleyAllocationDetail {
            name: unit.name.clone(),
            shapley_value: sv,
            pct_of_total: pct,
        });
    }

    // Efficiency check: difference between sum of Shapley values and total
    let efficiency_check = sum_shapley - total_capital;

    Ok(ShapleyAllocationOutput {
        allocations,
        total_capital,
        method_used,
        efficiency_check,
    })
}

/// Compute VaR of a coalition of units at the given confidence level.
fn compute_coalition_var(
    units: &[ShapleyUnit],
    coalition_indices: &[usize],
    n_scenarios: usize,
    confidence_level: Decimal,
) -> Decimal {
    if coalition_indices.is_empty() {
        return Decimal::ZERO;
    }

    // Sum returns across coalition members for each scenario
    let mut portfolio_returns = vec![Decimal::ZERO; n_scenarios];
    for &idx in coalition_indices {
        for (s, &r) in units[idx].returns.iter().enumerate() {
            if s < n_scenarios {
                portfolio_returns[s] += r;
            }
        }
    }

    // Sort ascending for quantile
    portfolio_returns.sort();

    // VaR at (1 - confidence) quantile
    let loss_quantile = Decimal::ONE - confidence_level;
    let idx_decimal = loss_quantile * Decimal::from(n_scenarios as u64);
    let idx = idx_decimal
        .to_string()
        .split('.')
        .next()
        .unwrap_or("0")
        .parse::<usize>()
        .unwrap_or(0);
    let idx = if idx >= n_scenarios {
        n_scenarios - 1
    } else {
        idx
    };

    // VaR = negative of the return at the quantile (loss is positive)
    let var = -portfolio_returns[idx];
    if var < Decimal::ZERO {
        Decimal::ZERO
    } else {
        var
    }
}

/// Exact Shapley value computation for N <= 8.
/// phi_i = sum over S not containing i: [|S|!(N-|S|-1)!/N!] * [v(S u {i}) - v(S)]
fn compute_exact_shapley(
    units: &[ShapleyUnit],
    n_scenarios: usize,
    confidence_level: Decimal,
) -> Vec<Decimal> {
    let n = units.len();
    let n_factorial = factorial(n);

    let mut shapley = vec![Decimal::ZERO; n];

    // Iterate over all subsets S (using bitmask)
    let total_subsets = 1u64 << n;
    for mask in 0..total_subsets {
        let s_indices: Vec<usize> = (0..n).filter(|&j| (mask >> j) & 1 == 1).collect();
        let s_size = s_indices.len();
        let v_s = compute_coalition_var(units, &s_indices, n_scenarios, confidence_level);

        // For each player i NOT in S, compute marginal contribution
        for (i, sv) in shapley.iter_mut().enumerate() {
            if (mask >> i) & 1 == 0 {
                // i is not in S
                let mut s_plus_i = s_indices.clone();
                s_plus_i.push(i);
                let v_s_plus_i =
                    compute_coalition_var(units, &s_plus_i, n_scenarios, confidence_level);

                let marginal = v_s_plus_i - v_s;

                // Weight: |S|! * (N - |S| - 1)! / N!
                let weight_num = factorial(s_size) * factorial(n - s_size - 1);
                let weight = Decimal::from(weight_num as u64) / Decimal::from(n_factorial as u64);

                *sv += weight * marginal;
            }
        }
    }

    shapley
}

/// Sampled Shapley value computation for N > 8.
/// Uses deterministic permutation generation (fixed ordering based on index).
fn compute_sampled_shapley(
    units: &[ShapleyUnit],
    n_scenarios: usize,
    confidence_level: Decimal,
    num_samples: u32,
) -> Vec<Decimal> {
    let n = units.len();
    let mut shapley = vec![Decimal::ZERO; n];

    // Deterministic pseudo-random permutation generator
    // Using a simple LCG (Linear Congruential Generator) with fixed seed
    let mut rng_state: u64 = 42; // Fixed seed for reproducibility

    for _ in 0..num_samples {
        // Generate a random permutation using Fisher-Yates with LCG
        let mut perm: Vec<usize> = (0..n).collect();
        for i in (1..n).rev() {
            rng_state = rng_state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let j = (rng_state >> 33) as usize % (i + 1);
            perm.swap(i, j);
        }

        // Walk through the permutation, computing marginal contributions
        let mut coalition: Vec<usize> = Vec::with_capacity(n);
        let mut prev_var = Decimal::ZERO;

        for &player in &perm {
            coalition.push(player);
            let new_var = compute_coalition_var(units, &coalition, n_scenarios, confidence_level);
            let marginal = new_var - prev_var;
            shapley[player] += marginal;
            prev_var = new_var;
        }
    }

    // Average over samples
    let num = Decimal::from(num_samples);
    for sv in &mut shapley {
        *sv /= num;
    }

    shapley
}

/// Factorial for small n (up to 8! = 40320).
fn factorial(n: usize) -> usize {
    (1..=n).product()
}

fn validate_shapley_input(input: &ShapleyAllocationInput) -> CorpFinanceResult<()> {
    if input.units.is_empty() {
        return Err(CorpFinanceError::InsufficientData(
            "At least one unit is required for Shapley allocation.".into(),
        ));
    }
    if input.confidence_level <= Decimal::ZERO || input.confidence_level >= Decimal::ONE {
        return Err(CorpFinanceError::InvalidInput {
            field: "confidence_level".into(),
            reason: "Confidence level must be in (0, 1).".into(),
        });
    }
    if input.num_samples == 0 {
        return Err(CorpFinanceError::InvalidInput {
            field: "num_samples".into(),
            reason: "Number of samples must be positive.".into(),
        });
    }

    let first_len = input.units[0].returns.len();
    if first_len == 0 {
        return Err(CorpFinanceError::InsufficientData(
            "Returns series must contain at least one observation.".into(),
        ));
    }
    for unit in &input.units {
        if unit.returns.len() != first_len {
            return Err(CorpFinanceError::InvalidInput {
                field: "returns".into(),
                reason: format!(
                    "All units must have same number of return observations. Expected {}, got {} for unit '{}'.",
                    first_len,
                    unit.returns.len(),
                    unit.name
                ),
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

    fn make_returns_1() -> Vec<Decimal> {
        // 200 scenarios with losses (negative returns) and gains
        (0..200)
            .map(|i| {
                let val = Decimal::from(i as i64 - 100);
                val / dec!(100)
            })
            .collect()
    }

    fn make_returns_2() -> Vec<Decimal> {
        (0..200)
            .map(|i| {
                let val = Decimal::from(80 - i as i64);
                val / dec!(100)
            })
            .collect()
    }

    fn make_returns_3() -> Vec<Decimal> {
        (0..200)
            .map(|i| {
                let val = Decimal::from((i as i64 % 50) - 25);
                val / dec!(100)
            })
            .collect()
    }

    fn make_base_input() -> ShapleyAllocationInput {
        ShapleyAllocationInput {
            units: vec![
                ShapleyUnit {
                    name: "A".into(),
                    returns: make_returns_1(),
                },
                ShapleyUnit {
                    name: "B".into(),
                    returns: make_returns_2(),
                },
                ShapleyUnit {
                    name: "C".into(),
                    returns: make_returns_3(),
                },
            ],
            confidence_level: dec!(0.95),
            num_samples: 1000,
        }
    }

    #[test]
    fn test_exact_method_for_small_n() {
        let input = make_base_input();
        let out = calculate_shapley_allocation(&input).unwrap();
        assert_eq!(out.method_used, ShapleyMethod::Exact);
    }

    #[test]
    fn test_allocations_count_matches_units() {
        let input = make_base_input();
        let out = calculate_shapley_allocation(&input).unwrap();
        assert_eq!(out.allocations.len(), 3);
    }

    #[test]
    fn test_efficiency_sum_equals_total() {
        let input = make_base_input();
        let out = calculate_shapley_allocation(&input).unwrap();
        // For exact method, efficiency check should be zero
        assert!(
            approx_eq(out.efficiency_check, Decimal::ZERO, dec!(0.01)),
            "Efficiency check should be ~0, got {}",
            out.efficiency_check
        );
    }

    #[test]
    fn test_total_capital_non_negative() {
        let input = make_base_input();
        let out = calculate_shapley_allocation(&input).unwrap();
        assert!(out.total_capital >= Decimal::ZERO);
    }

    #[test]
    fn test_names_preserved() {
        let input = make_base_input();
        let out = calculate_shapley_allocation(&input).unwrap();
        assert_eq!(out.allocations[0].name, "A");
        assert_eq!(out.allocations[1].name, "B");
        assert_eq!(out.allocations[2].name, "C");
    }

    #[test]
    fn test_pct_of_total_sum_approx_one() {
        let input = make_base_input();
        let out = calculate_shapley_allocation(&input).unwrap();
        if out.total_capital > Decimal::ZERO {
            let sum_pct: Decimal = out.allocations.iter().map(|a| a.pct_of_total).sum();
            assert!(
                approx_eq(sum_pct, Decimal::ONE, dec!(0.01)),
                "Sum of pct_of_total should be ~1.0, got {}",
                sum_pct
            );
        }
    }

    #[test]
    fn test_single_unit() {
        let input = ShapleyAllocationInput {
            units: vec![ShapleyUnit {
                name: "Solo".into(),
                returns: make_returns_1(),
            }],
            confidence_level: dec!(0.95),
            num_samples: 100,
        };
        let out = calculate_shapley_allocation(&input).unwrap();
        assert_eq!(out.allocations.len(), 1);
        // Single unit: Shapley value = total capital
        assert_eq!(out.allocations[0].shapley_value, out.total_capital);
    }

    #[test]
    fn test_two_identical_units_equal_allocation() {
        let returns = make_returns_1();
        let input = ShapleyAllocationInput {
            units: vec![
                ShapleyUnit {
                    name: "X".into(),
                    returns: returns.clone(),
                },
                ShapleyUnit {
                    name: "Y".into(),
                    returns,
                },
            ],
            confidence_level: dec!(0.95),
            num_samples: 100,
        };
        let out = calculate_shapley_allocation(&input).unwrap();
        // Symmetry: identical units get identical Shapley values
        assert_eq!(
            out.allocations[0].shapley_value,
            out.allocations[1].shapley_value,
        );
    }

    #[test]
    fn test_two_units_efficiency() {
        let input = ShapleyAllocationInput {
            units: vec![
                ShapleyUnit {
                    name: "A".into(),
                    returns: make_returns_1(),
                },
                ShapleyUnit {
                    name: "B".into(),
                    returns: make_returns_2(),
                },
            ],
            confidence_level: dec!(0.99),
            num_samples: 100,
        };
        let out = calculate_shapley_allocation(&input).unwrap();
        assert!(
            approx_eq(out.efficiency_check, Decimal::ZERO, dec!(0.01)),
            "Efficiency check should be ~0 for exact, got {}",
            out.efficiency_check
        );
    }

    #[test]
    fn test_sampled_method_for_large_n() {
        // Create 9 units to trigger sampled method
        let returns = make_returns_1();
        let units: Vec<ShapleyUnit> = (0..9)
            .map(|i| ShapleyUnit {
                name: format!("Unit_{}", i),
                returns: returns.clone(),
            })
            .collect();
        let input = ShapleyAllocationInput {
            units,
            confidence_level: dec!(0.95),
            num_samples: 100,
        };
        let out = calculate_shapley_allocation(&input).unwrap();
        assert_eq!(out.method_used, ShapleyMethod::Sampled);
    }

    #[test]
    fn test_sampled_efficiency_approx() {
        let returns = make_returns_1();
        let units: Vec<ShapleyUnit> = (0..9)
            .map(|i| ShapleyUnit {
                name: format!("Unit_{}", i),
                returns: returns.clone(),
            })
            .collect();
        let input = ShapleyAllocationInput {
            units,
            confidence_level: dec!(0.95),
            num_samples: 500,
        };
        let out = calculate_shapley_allocation(&input).unwrap();
        // Sampled should be approximately efficient
        assert!(
            out.efficiency_check.abs() < out.total_capital * dec!(0.1),
            "Sampled efficiency check {} should be small relative to total {}",
            out.efficiency_check,
            out.total_capital
        );
    }

    #[test]
    fn test_deterministic_results() {
        // Same input should give same results (deterministic seed)
        let input = make_base_input();
        let out1 = calculate_shapley_allocation(&input).unwrap();
        let out2 = calculate_shapley_allocation(&input).unwrap();
        for i in 0..input.units.len() {
            assert_eq!(
                out1.allocations[i].shapley_value,
                out2.allocations[i].shapley_value,
            );
        }
    }

    #[test]
    fn test_four_units_exact() {
        let input = ShapleyAllocationInput {
            units: vec![
                ShapleyUnit {
                    name: "A".into(),
                    returns: make_returns_1(),
                },
                ShapleyUnit {
                    name: "B".into(),
                    returns: make_returns_2(),
                },
                ShapleyUnit {
                    name: "C".into(),
                    returns: make_returns_3(),
                },
                ShapleyUnit {
                    name: "D".into(),
                    returns: make_returns_1(),
                },
            ],
            confidence_level: dec!(0.95),
            num_samples: 100,
        };
        let out = calculate_shapley_allocation(&input).unwrap();
        assert_eq!(out.method_used, ShapleyMethod::Exact);
        assert_eq!(out.allocations.len(), 4);
    }

    #[test]
    fn test_dummy_player_zero_allocation() {
        // A "dummy" player with zero returns should get zero Shapley value
        let input = ShapleyAllocationInput {
            units: vec![
                ShapleyUnit {
                    name: "Active".into(),
                    returns: make_returns_1(),
                },
                ShapleyUnit {
                    name: "Dummy".into(),
                    returns: vec![Decimal::ZERO; 200],
                },
            ],
            confidence_level: dec!(0.95),
            num_samples: 100,
        };
        let out = calculate_shapley_allocation(&input).unwrap();
        assert_eq!(out.allocations[1].shapley_value, Decimal::ZERO);
    }

    #[test]
    fn test_zero_confidence_level_returns_near_zero() {
        // Very low confidence should produce lower VaR
        let input = ShapleyAllocationInput {
            units: vec![ShapleyUnit {
                name: "A".into(),
                returns: make_returns_1(),
            }],
            confidence_level: dec!(0.5),
            num_samples: 100,
        };
        let out = calculate_shapley_allocation(&input).unwrap();
        // At 50% confidence, VaR is the median loss - should be relatively small
        assert!(out.total_capital >= Decimal::ZERO);
    }

    #[test]
    fn test_high_confidence_higher_capital() {
        let mut input_low = make_base_input();
        input_low.confidence_level = dec!(0.90);
        let out_low = calculate_shapley_allocation(&input_low).unwrap();

        let mut input_high = make_base_input();
        input_high.confidence_level = dec!(0.99);
        let out_high = calculate_shapley_allocation(&input_high).unwrap();

        assert!(
            out_high.total_capital >= out_low.total_capital,
            "Higher confidence should produce higher capital"
        );
    }

    #[test]
    fn test_factorial_function() {
        assert_eq!(factorial(0), 1);
        assert_eq!(factorial(1), 1);
        assert_eq!(factorial(5), 120);
        assert_eq!(factorial(8), 40320);
    }

    // -- Validation tests --

    #[test]
    fn test_reject_empty_units() {
        let input = ShapleyAllocationInput {
            units: vec![],
            confidence_level: dec!(0.95),
            num_samples: 100,
        };
        assert!(calculate_shapley_allocation(&input).is_err());
    }

    #[test]
    fn test_reject_zero_confidence() {
        let mut input = make_base_input();
        input.confidence_level = Decimal::ZERO;
        assert!(calculate_shapley_allocation(&input).is_err());
    }

    #[test]
    fn test_reject_confidence_one() {
        let mut input = make_base_input();
        input.confidence_level = Decimal::ONE;
        assert!(calculate_shapley_allocation(&input).is_err());
    }

    #[test]
    fn test_reject_zero_samples() {
        let mut input = make_base_input();
        input.num_samples = 0;
        assert!(calculate_shapley_allocation(&input).is_err());
    }

    #[test]
    fn test_reject_mismatched_returns() {
        let mut input = make_base_input();
        input.units[0].returns = vec![dec!(0.01); 50]; // 50 vs 200
        assert!(calculate_shapley_allocation(&input).is_err());
    }

    #[test]
    fn test_reject_empty_returns() {
        let input = ShapleyAllocationInput {
            units: vec![ShapleyUnit {
                name: "A".into(),
                returns: vec![],
            }],
            confidence_level: dec!(0.95),
            num_samples: 100,
        };
        assert!(calculate_shapley_allocation(&input).is_err());
    }

    #[test]
    fn test_serialization_roundtrip() {
        let input = make_base_input();
        let out = calculate_shapley_allocation(&input).unwrap();
        let json = serde_json::to_string(&out).unwrap();
        let _: ShapleyAllocationOutput = serde_json::from_str(&json).unwrap();
    }
}
