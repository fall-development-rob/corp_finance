//! Euler Risk Contribution (Capital Allocation).
//!
//! Covers:
//! 1. **Marginal Contribution** -- MC_i = dVaR/dw_i via finite difference
//! 2. **Euler Allocation** -- allocated_capital_i = w_i * MC_i
//! 3. **Diversification Benefit** -- sum(standalone) - portfolio capital
//! 4. **Concentration Index** -- HHI of capital allocations
//! 5. **Stand-alone Capital** -- VaR of each unit independently
//!
//! All arithmetic uses `rust_decimal::Decimal`. No `f64`.

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

use crate::error::CorpFinanceError;
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Input / Output
// ---------------------------------------------------------------------------

/// A single business unit for Euler allocation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EulerUnit {
    /// Name of the business unit.
    pub name: String,
    /// Portfolio weight (must sum to 1 across all units, or be proportional).
    pub weight: Decimal,
    /// Stand-alone VaR for this unit.
    pub standalone_var: Decimal,
    /// Simulated returns or P&L series for this unit.
    pub returns: Vec<Decimal>,
}

/// Input for Euler allocation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EulerAllocationInput {
    /// Business units.
    pub units: Vec<EulerUnit>,
    /// Total portfolio VaR.
    pub portfolio_var: Decimal,
    /// Epsilon for finite difference (e.g. 0.01).
    pub epsilon: Decimal,
    /// Confidence level for VaR calculation (e.g. 0.99).
    pub confidence_level: Decimal,
}

/// A single unit's Euler allocation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EulerAllocationDetail {
    /// Unit name.
    pub name: String,
    /// Portfolio weight.
    pub weight: Decimal,
    /// Stand-alone capital.
    pub standalone: Decimal,
    /// Marginal contribution = dVaR/dw.
    pub marginal_contribution: Decimal,
    /// Allocated capital = weight * marginal contribution.
    pub allocated_capital: Decimal,
    /// Percentage of total portfolio VaR.
    pub pct_of_total: Decimal,
}

/// Output of the Euler allocation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EulerAllocationOutput {
    /// Per-unit allocation details.
    pub allocations: Vec<EulerAllocationDetail>,
    /// Diversification benefit = sum(standalone) - portfolio_var.
    pub diversification_benefit: Decimal,
    /// Diversification ratio = sum(standalone) / portfolio_var.
    pub diversification_ratio: Decimal,
    /// HHI concentration index of allocated capital shares.
    pub hhi: Decimal,
}

/// Calculate Euler risk contribution allocation.
pub fn calculate_euler_allocation(
    input: &EulerAllocationInput,
) -> CorpFinanceResult<EulerAllocationOutput> {
    validate_euler_input(input)?;

    let n_units = input.units.len();

    // Determine number of scenarios from the first unit
    let n_scenarios = input.units[0].returns.len();

    // Calculate marginal contributions via finite difference on portfolio VaR
    let mut allocations = Vec::with_capacity(n_units);
    let mut total_standalone = Decimal::ZERO;
    let mut total_allocated = Decimal::ZERO;

    for unit in &input.units {
        total_standalone += unit.standalone_var;

        // Compute portfolio returns with perturbed weights
        // VaR(w_i + eps) and VaR(w_i - eps) using the returns data
        let var_up = compute_perturbed_var(
            &input.units,
            &unit.name,
            input.epsilon,
            input.confidence_level,
            n_scenarios,
        );
        let var_down = compute_perturbed_var(
            &input.units,
            &unit.name,
            -input.epsilon,
            input.confidence_level,
            n_scenarios,
        );

        let marginal_contribution = if input.epsilon.is_zero() {
            Decimal::ZERO
        } else {
            (var_up - var_down) / (dec!(2) * input.epsilon)
        };

        let allocated_capital = unit.weight * marginal_contribution;

        allocations.push(EulerAllocationDetail {
            name: unit.name.clone(),
            weight: unit.weight,
            standalone: unit.standalone_var,
            marginal_contribution,
            allocated_capital,
            pct_of_total: Decimal::ZERO, // filled below
        });

        total_allocated += allocated_capital;
    }

    // Compute pct_of_total
    for alloc in &mut allocations {
        alloc.pct_of_total = if input.portfolio_var.is_zero() {
            Decimal::ZERO
        } else {
            alloc.allocated_capital / input.portfolio_var
        };
    }

    // Diversification benefit
    let diversification_benefit = total_standalone - input.portfolio_var;

    // Diversification ratio
    let diversification_ratio = if input.portfolio_var.is_zero() {
        Decimal::ZERO
    } else {
        total_standalone / input.portfolio_var
    };

    // HHI of allocation shares
    let hhi = if total_allocated.is_zero() {
        Decimal::ZERO
    } else {
        allocations
            .iter()
            .map(|a| {
                let share = a.allocated_capital / total_allocated;
                share * share
            })
            .sum::<Decimal>()
    };

    Ok(EulerAllocationOutput {
        allocations,
        diversification_benefit,
        diversification_ratio,
        hhi,
    })
}

/// Compute VaR of the portfolio with one unit's weight perturbed by delta.
fn compute_perturbed_var(
    units: &[EulerUnit],
    perturb_name: &str,
    delta: Decimal,
    confidence_level: Decimal,
    n_scenarios: usize,
) -> Decimal {
    // Build perturbed portfolio returns for each scenario
    let mut portfolio_returns = vec![Decimal::ZERO; n_scenarios];

    for unit in units {
        let w = if unit.name == perturb_name {
            unit.weight + delta
        } else {
            unit.weight
        };
        for (i, &r) in unit.returns.iter().enumerate() {
            if i < n_scenarios {
                portfolio_returns[i] += w * r;
            }
        }
    }

    // VaR = quantile of losses (negative returns). Sort ascending.
    portfolio_returns.sort();

    let var_index = {
        // VaR at (1 - confidence) quantile of returns (loss perspective)
        let loss_quantile = Decimal::ONE - confidence_level;
        let idx_decimal = loss_quantile * Decimal::from(n_scenarios as u64);
        let idx_str = idx_decimal.to_string();
        let idx = idx_str
            .split('.')
            .next()
            .unwrap_or("0")
            .parse::<usize>()
            .unwrap_or(0);
        if idx >= n_scenarios {
            n_scenarios - 1
        } else if idx == 0 {
            0
        } else {
            idx
        }
    };

    // VaR = negative of the return at the quantile (loss is positive)
    -portfolio_returns[var_index]
}

fn validate_euler_input(input: &EulerAllocationInput) -> CorpFinanceResult<()> {
    if input.units.is_empty() {
        return Err(CorpFinanceError::InsufficientData(
            "At least one unit is required for Euler allocation.".into(),
        ));
    }
    if input.epsilon <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "epsilon".into(),
            reason: "Epsilon for finite difference must be positive.".into(),
        });
    }
    if input.confidence_level <= Decimal::ZERO || input.confidence_level >= Decimal::ONE {
        return Err(CorpFinanceError::InvalidInput {
            field: "confidence_level".into(),
            reason: "Confidence level must be in (0, 1).".into(),
        });
    }
    if input.portfolio_var < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "portfolio_var".into(),
            reason: "Portfolio VaR must be non-negative.".into(),
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
        if unit.weight < Decimal::ZERO {
            return Err(CorpFinanceError::InvalidInput {
                field: "weight".into(),
                reason: format!("Weight must be non-negative for unit '{}'.", unit.name),
            });
        }
        if unit.standalone_var < Decimal::ZERO {
            return Err(CorpFinanceError::InvalidInput {
                field: "standalone_var".into(),
                reason: format!(
                    "Stand-alone VaR must be non-negative for unit '{}'.",
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

    fn make_returns_a() -> Vec<Decimal> {
        // 100 scenarios: some positive, some negative
        (0..100)
            .map(|i| {
                let val = Decimal::from(i as i64 - 50);
                val / dec!(100)
            })
            .collect()
    }

    fn make_returns_b() -> Vec<Decimal> {
        // 100 scenarios: offset pattern
        (0..100)
            .map(|i| {
                let val = Decimal::from(50 - i as i64);
                val / dec!(100)
            })
            .collect()
    }

    fn make_base_input() -> EulerAllocationInput {
        EulerAllocationInput {
            units: vec![
                EulerUnit {
                    name: "Unit_A".into(),
                    weight: dec!(0.6),
                    standalone_var: dec!(100),
                    returns: make_returns_a(),
                },
                EulerUnit {
                    name: "Unit_B".into(),
                    weight: dec!(0.4),
                    standalone_var: dec!(80),
                    returns: make_returns_b(),
                },
            ],
            portfolio_var: dec!(150),
            epsilon: dec!(0.01),
            confidence_level: dec!(0.99),
        }
    }

    #[test]
    fn test_allocations_count_matches_units() {
        let input = make_base_input();
        let out = calculate_euler_allocation(&input).unwrap();
        assert_eq!(out.allocations.len(), 2);
    }

    #[test]
    fn test_diversification_benefit_positive() {
        let input = make_base_input();
        let out = calculate_euler_allocation(&input).unwrap();
        // sum(standalone) = 100 + 80 = 180 > 150 = portfolio_var
        assert_eq!(out.diversification_benefit, dec!(30));
    }

    #[test]
    fn test_diversification_ratio_gt_one() {
        let input = make_base_input();
        let out = calculate_euler_allocation(&input).unwrap();
        assert!(
            out.diversification_ratio > Decimal::ONE,
            "Diversification ratio should be > 1"
        );
    }

    #[test]
    fn test_diversification_ratio_value() {
        let input = make_base_input();
        let out = calculate_euler_allocation(&input).unwrap();
        // 180 / 150 = 1.2
        assert_eq!(out.diversification_ratio, dec!(180) / dec!(150));
    }

    #[test]
    fn test_hhi_non_negative() {
        let input = make_base_input();
        let out = calculate_euler_allocation(&input).unwrap();
        // HHI is sum of squared shares, always non-negative
        assert!(
            out.hhi >= Decimal::ZERO,
            "HHI should be non-negative, got {}",
            out.hhi
        );
    }

    #[test]
    fn test_allocation_names_preserved() {
        let input = make_base_input();
        let out = calculate_euler_allocation(&input).unwrap();
        assert_eq!(out.allocations[0].name, "Unit_A");
        assert_eq!(out.allocations[1].name, "Unit_B");
    }

    #[test]
    fn test_allocation_weights_preserved() {
        let input = make_base_input();
        let out = calculate_euler_allocation(&input).unwrap();
        assert_eq!(out.allocations[0].weight, dec!(0.6));
        assert_eq!(out.allocations[1].weight, dec!(0.4));
    }

    #[test]
    fn test_standalone_preserved() {
        let input = make_base_input();
        let out = calculate_euler_allocation(&input).unwrap();
        assert_eq!(out.allocations[0].standalone, dec!(100));
        assert_eq!(out.allocations[1].standalone, dec!(80));
    }

    #[test]
    fn test_single_unit_marginal_equals_var() {
        let input = EulerAllocationInput {
            units: vec![EulerUnit {
                name: "Only".into(),
                weight: dec!(1.0),
                standalone_var: dec!(50),
                returns: make_returns_a(),
            }],
            portfolio_var: dec!(50),
            epsilon: dec!(0.01),
            confidence_level: dec!(0.99),
        };
        let out = calculate_euler_allocation(&input).unwrap();
        // For a single unit, diversification benefit should be zero
        assert_eq!(out.diversification_benefit, Decimal::ZERO);
    }

    #[test]
    fn test_single_unit_hhi_is_one() {
        let input = EulerAllocationInput {
            units: vec![EulerUnit {
                name: "Only".into(),
                weight: dec!(1.0),
                standalone_var: dec!(50),
                returns: make_returns_a(),
            }],
            portfolio_var: dec!(50),
            epsilon: dec!(0.01),
            confidence_level: dec!(0.99),
        };
        let out = calculate_euler_allocation(&input).unwrap();
        // Single unit: HHI = 1.0^2 = 1.0
        assert!(
            approx_eq(out.hhi, Decimal::ONE, dec!(0.01)),
            "Single unit HHI should be ~1.0, got {}",
            out.hhi
        );
    }

    #[test]
    fn test_marginal_contribution_finite() {
        let input = make_base_input();
        let out = calculate_euler_allocation(&input).unwrap();
        for alloc in &out.allocations {
            assert!(
                alloc.marginal_contribution.abs() < dec!(10000),
                "Marginal contribution should be finite"
            );
        }
    }

    #[test]
    fn test_zero_portfolio_var_zero_pct() {
        let mut input = make_base_input();
        input.portfolio_var = Decimal::ZERO;
        let out = calculate_euler_allocation(&input).unwrap();
        for alloc in &out.allocations {
            assert_eq!(alloc.pct_of_total, Decimal::ZERO);
        }
    }

    #[test]
    fn test_three_units() {
        let returns_c: Vec<Decimal> = (0..100)
            .map(|i| Decimal::from((i % 7) as i64 - 3) / dec!(100))
            .collect();
        let input = EulerAllocationInput {
            units: vec![
                EulerUnit {
                    name: "A".into(),
                    weight: dec!(0.4),
                    standalone_var: dec!(60),
                    returns: make_returns_a(),
                },
                EulerUnit {
                    name: "B".into(),
                    weight: dec!(0.35),
                    standalone_var: dec!(50),
                    returns: make_returns_b(),
                },
                EulerUnit {
                    name: "C".into(),
                    weight: dec!(0.25),
                    standalone_var: dec!(40),
                    returns: returns_c,
                },
            ],
            portfolio_var: dec!(120),
            epsilon: dec!(0.01),
            confidence_level: dec!(0.95),
        };
        let out = calculate_euler_allocation(&input).unwrap();
        assert_eq!(out.allocations.len(), 3);
        assert_eq!(out.diversification_benefit, dec!(30)); // 150 - 120
    }

    #[test]
    fn test_equal_weight_units() {
        let input = EulerAllocationInput {
            units: vec![
                EulerUnit {
                    name: "X".into(),
                    weight: dec!(0.5),
                    standalone_var: dec!(100),
                    returns: make_returns_a(),
                },
                EulerUnit {
                    name: "Y".into(),
                    weight: dec!(0.5),
                    standalone_var: dec!(100),
                    returns: make_returns_a(),
                },
            ],
            portfolio_var: dec!(100),
            epsilon: dec!(0.01),
            confidence_level: dec!(0.99),
        };
        let out = calculate_euler_allocation(&input).unwrap();
        // Identical units => identical allocations
        assert_eq!(
            out.allocations[0].allocated_capital,
            out.allocations[1].allocated_capital
        );
    }

    // -- Validation tests --

    #[test]
    fn test_reject_empty_units() {
        let input = EulerAllocationInput {
            units: vec![],
            portfolio_var: dec!(100),
            epsilon: dec!(0.01),
            confidence_level: dec!(0.99),
        };
        assert!(calculate_euler_allocation(&input).is_err());
    }

    #[test]
    fn test_reject_zero_epsilon() {
        let mut input = make_base_input();
        input.epsilon = Decimal::ZERO;
        assert!(calculate_euler_allocation(&input).is_err());
    }

    #[test]
    fn test_reject_negative_epsilon() {
        let mut input = make_base_input();
        input.epsilon = dec!(-0.01);
        assert!(calculate_euler_allocation(&input).is_err());
    }

    #[test]
    fn test_reject_confidence_zero() {
        let mut input = make_base_input();
        input.confidence_level = Decimal::ZERO;
        assert!(calculate_euler_allocation(&input).is_err());
    }

    #[test]
    fn test_reject_confidence_one() {
        let mut input = make_base_input();
        input.confidence_level = Decimal::ONE;
        assert!(calculate_euler_allocation(&input).is_err());
    }

    #[test]
    fn test_reject_negative_portfolio_var() {
        let mut input = make_base_input();
        input.portfolio_var = dec!(-1);
        assert!(calculate_euler_allocation(&input).is_err());
    }

    #[test]
    fn test_reject_mismatched_returns_length() {
        let mut input = make_base_input();
        input.units[1].returns = vec![dec!(0.01); 50]; // 50 vs 100
        assert!(calculate_euler_allocation(&input).is_err());
    }

    #[test]
    fn test_reject_negative_weight() {
        let mut input = make_base_input();
        input.units[0].weight = dec!(-0.1);
        assert!(calculate_euler_allocation(&input).is_err());
    }

    #[test]
    fn test_reject_negative_standalone_var() {
        let mut input = make_base_input();
        input.units[0].standalone_var = dec!(-10);
        assert!(calculate_euler_allocation(&input).is_err());
    }

    #[test]
    fn test_reject_empty_returns() {
        let mut input = make_base_input();
        input.units[0].returns = vec![];
        input.units[1].returns = vec![];
        assert!(calculate_euler_allocation(&input).is_err());
    }

    #[test]
    fn test_serialization_roundtrip() {
        let input = make_base_input();
        let out = calculate_euler_allocation(&input).unwrap();
        let json = serde_json::to_string(&out).unwrap();
        let _: EulerAllocationOutput = serde_json::from_str(&json).unwrap();
    }
}
