use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::error::CorpFinanceError;
use crate::types::*;
use crate::CorpFinanceResult;

/// Input for 2-way sensitivity analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensitivityInput {
    /// Base case input values (model-specific JSON)
    pub base_inputs: serde_json::Value,
    /// First variable to sweep
    pub variable_1: SensitivityVariable,
    /// Second variable to sweep
    pub variable_2: SensitivityVariable,
    /// Name of the output metric being measured
    pub output_metric: String,
    /// Model function identifier (e.g. "dcf", "lbo")
    pub compute_fn: String,
}

/// Output of 2-way sensitivity analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensitivityOutput {
    pub variable_1_name: String,
    pub variable_2_name: String,
    pub variable_1_values: Vec<Decimal>,
    pub variable_2_values: Vec<Decimal>,
    pub output_metric: String,
    /// Matrix[i][j] = output when variable_1 = variable_1_values[i], variable_2 = variable_2_values[j]
    pub matrix: Vec<Vec<Decimal>>,
    /// Base case output value
    pub base_case_value: Decimal,
    /// Position of the base case in the matrix (row, col)
    pub base_case_position: (usize, usize),
}

/// Generate the sweep values for a sensitivity variable from min to max with step.
fn generate_sweep_values(var: &SensitivityVariable) -> CorpFinanceResult<Vec<Decimal>> {
    if var.step <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: format!("variable:{}", var.name),
            reason: "Step must be positive".into(),
        });
    }
    if var.min > var.max {
        return Err(CorpFinanceError::InvalidInput {
            field: format!("variable:{}", var.name),
            reason: "Min must be <= max".into(),
        });
    }

    let mut values = Vec::new();
    let mut current = var.min;
    while current <= var.max {
        values.push(current);
        current += var.step;
    }
    // Ensure max is included if step doesn't land exactly on it
    if let Some(&last) = values.last() {
        if last < var.max {
            values.push(var.max);
        }
    }

    if values.is_empty() {
        values.push(var.min);
    }

    Ok(values)
}

/// Find the closest index to a target value in a sorted list.
fn closest_index(values: &[Decimal], target: Decimal) -> usize {
    values
        .iter()
        .enumerate()
        .min_by_key(|(_, v)| (**v - target).abs())
        .map(|(i, _)| i)
        .unwrap_or(0)
}

/// Build a 2-way sensitivity grid structure.
///
/// This function creates the grid framework with variable sweep values
/// and an empty matrix. The actual model evaluation function will be
/// plugged in by the caller using `evaluate_sensitivity`.
pub fn build_sensitivity_grid(
    input: &SensitivityInput,
) -> CorpFinanceResult<ComputationOutput<SensitivityOutput>> {
    let start = Instant::now();
    let warnings: Vec<String> = Vec::new();

    let v1_values = generate_sweep_values(&input.variable_1)?;
    let v2_values = generate_sweep_values(&input.variable_2)?;

    // Initialise matrix with zeros (to be filled by model evaluation)
    let matrix = vec![vec![Decimal::ZERO; v2_values.len()]; v1_values.len()];

    // Find base case position: midpoint of each range
    let mid1 = (input.variable_1.min + input.variable_1.max) / dec!(2);
    let mid2 = (input.variable_2.min + input.variable_2.max) / dec!(2);
    let base_row = closest_index(&v1_values, mid1);
    let base_col = closest_index(&v2_values, mid2);

    let output = SensitivityOutput {
        variable_1_name: input.variable_1.name.clone(),
        variable_2_name: input.variable_2.name.clone(),
        variable_1_values: v1_values,
        variable_2_values: v2_values,
        output_metric: input.output_metric.clone(),
        matrix,
        base_case_value: Decimal::ZERO,
        base_case_position: (base_row, base_col),
    };

    let elapsed = start.elapsed().as_micros() as u64;
    Ok(with_metadata(
        "2-Way Sensitivity Analysis Grid",
        &serde_json::json!({
            "variable_1": input.variable_1.name,
            "variable_2": input.variable_2.name,
            "output_metric": input.output_metric,
            "compute_fn": input.compute_fn,
        }),
        warnings,
        elapsed,
        output,
    ))
}

/// Evaluate a 2-way sensitivity grid using a provided computation function.
///
/// The `eval_fn` receives (variable_1_value, variable_2_value) and returns
/// the output metric value.
pub fn evaluate_sensitivity<F>(
    input: &SensitivityInput,
    eval_fn: F,
) -> CorpFinanceResult<ComputationOutput<SensitivityOutput>>
where
    F: Fn(Decimal, Decimal) -> CorpFinanceResult<Decimal>,
{
    let start = Instant::now();
    let mut warnings: Vec<String> = Vec::new();

    let v1_values = generate_sweep_values(&input.variable_1)?;
    let v2_values = generate_sweep_values(&input.variable_2)?;

    let mut matrix = Vec::with_capacity(v1_values.len());

    for v1 in &v1_values {
        let mut row = Vec::with_capacity(v2_values.len());
        for v2 in &v2_values {
            match eval_fn(*v1, *v2) {
                Ok(val) => row.push(val),
                Err(e) => {
                    warnings.push(format!(
                        "Evaluation failed at ({v1}, {v2}): {e}"
                    ));
                    row.push(Decimal::ZERO);
                }
            }
        }
        matrix.push(row);
    }

    let mid1 = (input.variable_1.min + input.variable_1.max) / dec!(2);
    let mid2 = (input.variable_2.min + input.variable_2.max) / dec!(2);
    let base_row = closest_index(&v1_values, mid1);
    let base_col = closest_index(&v2_values, mid2);
    let base_case_value = matrix[base_row][base_col];

    let output = SensitivityOutput {
        variable_1_name: input.variable_1.name.clone(),
        variable_2_name: input.variable_2.name.clone(),
        variable_1_values: v1_values,
        variable_2_values: v2_values,
        output_metric: input.output_metric.clone(),
        matrix,
        base_case_value,
        base_case_position: (base_row, base_col),
    };

    let elapsed = start.elapsed().as_micros() as u64;
    Ok(with_metadata(
        "2-Way Sensitivity Analysis (Evaluated)",
        &serde_json::json!({
            "variable_1": input.variable_1.name,
            "variable_2": input.variable_2.name,
            "output_metric": input.output_metric,
            "compute_fn": input.compute_fn,
        }),
        warnings,
        elapsed,
        output,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn sample_input() -> SensitivityInput {
        SensitivityInput {
            base_inputs: serde_json::json!({}),
            variable_1: SensitivityVariable {
                name: "WACC".into(),
                min: dec!(0.08),
                max: dec!(0.12),
                step: dec!(0.01),
            },
            variable_2: SensitivityVariable {
                name: "Growth Rate".into(),
                min: dec!(0.01),
                max: dec!(0.05),
                step: dec!(0.01),
            },
            output_metric: "Enterprise Value".into(),
            compute_fn: "dcf".into(),
        }
    }

    #[test]
    fn test_build_grid() {
        let result = build_sensitivity_grid(&sample_input()).unwrap();
        let out = &result.result;

        assert_eq!(out.variable_1_name, "WACC");
        assert_eq!(out.variable_2_name, "Growth Rate");
        // WACC: 0.08, 0.09, 0.10, 0.11, 0.12 => 5 values
        assert_eq!(out.variable_1_values.len(), 5);
        // Growth: 0.01, 0.02, 0.03, 0.04, 0.05 => 5 values
        assert_eq!(out.variable_2_values.len(), 5);
        // Matrix dimensions
        assert_eq!(out.matrix.len(), 5);
        assert_eq!(out.matrix[0].len(), 5);
    }

    #[test]
    fn test_evaluate_sensitivity() {
        let input = sample_input();
        // Simple model: output = 1000 / (v1 - v2)
        let result = evaluate_sensitivity(&input, |wacc, growth| {
            let spread = wacc - growth;
            if spread.is_zero() {
                return Err(CorpFinanceError::DivisionByZero {
                    context: "test model".into(),
                });
            }
            Ok(dec!(1000) / spread)
        })
        .unwrap();

        let out = &result.result;
        // Check that values decrease as WACC increases (higher discount => lower value)
        let col = 0; // growth = 0.01
        for i in 0..out.matrix.len() - 1 {
            assert!(out.matrix[i][col] > out.matrix[i + 1][col]);
        }

        // Check that values increase as growth increases (for fixed WACC)
        let row = 0; // WACC = 0.08
        for j in 0..out.matrix[0].len() - 1 {
            assert!(out.matrix[row][j] < out.matrix[row][j + 1]);
        }
    }

    #[test]
    fn test_sweep_values() {
        let var = SensitivityVariable {
            name: "test".into(),
            min: dec!(1),
            max: dec!(5),
            step: dec!(1),
        };
        let vals = generate_sweep_values(&var).unwrap();
        assert_eq!(vals, vec![dec!(1), dec!(2), dec!(3), dec!(4), dec!(5)]);
    }

    #[test]
    fn test_sweep_with_non_exact_step() {
        let var = SensitivityVariable {
            name: "test".into(),
            min: dec!(0),
            max: dec!(1),
            step: dec!(0.3),
        };
        let vals = generate_sweep_values(&var).unwrap();
        // 0, 0.3, 0.6, 0.9, 1.0 (max appended)
        assert_eq!(vals.len(), 5);
        assert_eq!(*vals.last().unwrap(), dec!(1));
    }

    #[test]
    fn test_invalid_step() {
        let input = SensitivityInput {
            base_inputs: serde_json::json!({}),
            variable_1: SensitivityVariable {
                name: "bad".into(),
                min: dec!(0),
                max: dec!(1),
                step: dec!(0),
            },
            variable_2: SensitivityVariable {
                name: "ok".into(),
                min: dec!(0),
                max: dec!(1),
                step: dec!(0.5),
            },
            output_metric: "test".into(),
            compute_fn: "test".into(),
        };
        assert!(build_sensitivity_grid(&input).is_err());
    }

    #[test]
    fn test_base_case_position() {
        let result = build_sensitivity_grid(&sample_input()).unwrap();
        let out = &result.result;
        // Midpoint of WACC 0.08-0.12 = 0.10 => index 2
        // Midpoint of Growth 0.01-0.05 = 0.03 => index 2
        assert_eq!(out.base_case_position, (2, 2));
    }
}
