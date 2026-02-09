use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::error::CorpFinanceError;
use crate::types::*;
use crate::CorpFinanceResult;

/// Input for scenario analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioInput {
    /// List of scenarios (from types::Scenario)
    pub scenarios: Vec<Scenario>,
    /// Base case input values (model-specific JSON)
    pub base_inputs: serde_json::Value,
}

/// Result for a single scenario
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioResult {
    pub name: String,
    pub probability: Rate,
    pub output_value: Decimal,
    pub deviation_from_base: Decimal,
    pub deviation_pct: Rate,
}

/// Output of scenario analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioOutput {
    pub results: Vec<ScenarioResult>,
    pub probability_weighted_value: Decimal,
}

/// Run scenario analysis with pre-computed output values.
///
/// Each scenario must include a probability and an output value.
/// This function validates probabilities sum to ~1.0, calculates
/// probability-weighted value, and deviations from base case.
pub fn analyze_scenarios(
    input: &ScenarioInput,
    output_values: &[Decimal],
    base_case_value: Decimal,
) -> CorpFinanceResult<ComputationOutput<ScenarioOutput>> {
    let start = Instant::now();
    let mut warnings: Vec<String> = Vec::new();

    if input.scenarios.is_empty() {
        return Err(CorpFinanceError::InsufficientData(
            "At least one scenario required".into(),
        ));
    }

    if input.scenarios.len() != output_values.len() {
        return Err(CorpFinanceError::InvalidInput {
            field: "output_values".into(),
            reason: "Must have one output value per scenario".into(),
        });
    }

    // Validate probabilities
    let total_prob: Decimal = input.scenarios.iter().map(|s| s.probability).sum();
    let prob_tolerance = dec!(0.001);
    if (total_prob - Decimal::ONE).abs() > prob_tolerance {
        return Err(CorpFinanceError::InvalidInput {
            field: "probabilities".into(),
            reason: format!("Probabilities must sum to 1.0 (got {total_prob})"),
        });
    }

    for s in &input.scenarios {
        if s.probability < Decimal::ZERO || s.probability > Decimal::ONE {
            return Err(CorpFinanceError::InvalidInput {
                field: format!("scenario:{} probability", s.name),
                reason: "Probability must be between 0 and 1".into(),
            });
        }
    }

    // Normalise probabilities if they're very close but not exact
    let prob_sum: Decimal = input.scenarios.iter().map(|s| s.probability).sum();
    if (prob_sum - Decimal::ONE).abs() > Decimal::ZERO
        && (prob_sum - Decimal::ONE).abs() <= prob_tolerance
    {
        warnings.push(format!(
            "Probabilities sum to {prob_sum}; treated as approximately 1.0"
        ));
    }

    let mut results = Vec::with_capacity(input.scenarios.len());
    let mut probability_weighted_value = Decimal::ZERO;

    for (scenario, output_value) in input.scenarios.iter().zip(output_values.iter()) {
        let deviation = *output_value - base_case_value;
        let deviation_pct = if base_case_value.is_zero() {
            if deviation.is_zero() {
                Decimal::ZERO
            } else {
                warnings.push(format!(
                    "Base case is zero; cannot compute deviation_pct for scenario '{}'",
                    scenario.name
                ));
                Decimal::ZERO
            }
        } else {
            deviation / base_case_value
        };

        probability_weighted_value += scenario.probability * *output_value;

        results.push(ScenarioResult {
            name: scenario.name.clone(),
            probability: scenario.probability,
            output_value: *output_value,
            deviation_from_base: deviation,
            deviation_pct,
        });
    }

    let output = ScenarioOutput {
        results,
        probability_weighted_value,
    };

    let elapsed = start.elapsed().as_micros() as u64;
    Ok(with_metadata(
        "Bear/Base/Bull Scenario Analysis",
        &serde_json::json!({
            "num_scenarios": input.scenarios.len(),
            "base_case_value": base_case_value.to_string(),
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

    fn bear_base_bull() -> ScenarioInput {
        ScenarioInput {
            scenarios: vec![
                Scenario {
                    name: "Bear".into(),
                    probability: dec!(0.25),
                    overrides: serde_json::json!({"growth": -0.05}),
                },
                Scenario {
                    name: "Base".into(),
                    probability: dec!(0.50),
                    overrides: serde_json::json!({"growth": 0.03}),
                },
                Scenario {
                    name: "Bull".into(),
                    probability: dec!(0.25),
                    overrides: serde_json::json!({"growth": 0.08}),
                },
            ],
            base_inputs: serde_json::json!({}),
        }
    }

    #[test]
    fn test_basic_scenario_analysis() {
        let input = bear_base_bull();
        let values = vec![dec!(800), dec!(1000), dec!(1200)];
        let result = analyze_scenarios(&input, &values, dec!(1000)).unwrap();
        let out = &result.result;

        assert_eq!(out.results.len(), 3);

        // Probability-weighted = 0.25*800 + 0.50*1000 + 0.25*1200 = 200 + 500 + 300 = 1000
        assert_eq!(out.probability_weighted_value, dec!(1000));
    }

    #[test]
    fn test_deviations() {
        let input = bear_base_bull();
        let values = vec![dec!(800), dec!(1000), dec!(1200)];
        let result = analyze_scenarios(&input, &values, dec!(1000)).unwrap();
        let out = &result.result;

        assert_eq!(out.results[0].deviation_from_base, dec!(-200));
        assert_eq!(out.results[0].deviation_pct, dec!(-0.2));
        assert_eq!(out.results[1].deviation_from_base, Decimal::ZERO);
        assert_eq!(out.results[2].deviation_from_base, dec!(200));
        assert_eq!(out.results[2].deviation_pct, dec!(0.2));
    }

    #[test]
    fn test_probabilities_must_sum_to_one() {
        let input = ScenarioInput {
            scenarios: vec![
                Scenario {
                    name: "A".into(),
                    probability: dec!(0.30),
                    overrides: serde_json::json!({}),
                },
                Scenario {
                    name: "B".into(),
                    probability: dec!(0.30),
                    overrides: serde_json::json!({}),
                },
            ],
            base_inputs: serde_json::json!({}),
        };
        let values = vec![dec!(100), dec!(200)];
        assert!(analyze_scenarios(&input, &values, dec!(150)).is_err());
    }

    #[test]
    fn test_mismatched_values_length() {
        let input = bear_base_bull();
        let values = vec![dec!(800), dec!(1000)]; // Only 2 for 3 scenarios
        assert!(analyze_scenarios(&input, &values, dec!(1000)).is_err());
    }

    #[test]
    fn test_empty_scenarios() {
        let input = ScenarioInput {
            scenarios: vec![],
            base_inputs: serde_json::json!({}),
        };
        assert!(analyze_scenarios(&input, &[], dec!(0)).is_err());
    }

    #[test]
    fn test_asymmetric_probabilities() {
        let input = ScenarioInput {
            scenarios: vec![
                Scenario {
                    name: "Downside".into(),
                    probability: dec!(0.10),
                    overrides: serde_json::json!({}),
                },
                Scenario {
                    name: "Base".into(),
                    probability: dec!(0.70),
                    overrides: serde_json::json!({}),
                },
                Scenario {
                    name: "Upside".into(),
                    probability: dec!(0.20),
                    overrides: serde_json::json!({}),
                },
            ],
            base_inputs: serde_json::json!({}),
        };
        let values = vec![dec!(500), dec!(1000), dec!(1500)];
        let result = analyze_scenarios(&input, &values, dec!(1000)).unwrap();

        // PW = 0.10*500 + 0.70*1000 + 0.20*1500 = 50 + 700 + 300 = 1050
        assert_eq!(result.result.probability_weighted_value, dec!(1050));
    }

    #[test]
    fn test_negative_probability_error() {
        let input = ScenarioInput {
            scenarios: vec![
                Scenario {
                    name: "Bad".into(),
                    probability: dec!(-0.5),
                    overrides: serde_json::json!({}),
                },
                Scenario {
                    name: "Good".into(),
                    probability: dec!(1.5),
                    overrides: serde_json::json!({}),
                },
            ],
            base_inputs: serde_json::json!({}),
        };
        let values = vec![dec!(100), dec!(200)];
        assert!(analyze_scenarios(&input, &values, dec!(150)).is_err());
    }
}
