use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::{CorpFinanceError, CorpFinanceResult, types::*};

// ---------------------------------------------------------------------------
// Input / Output types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebtCapacityInput {
    pub ebitda: Money,
    pub interest_rate: Rate,
    /// Maximum tolerable net-debt / EBITDA multiple.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_leverage: Option<Multiple>,
    /// Minimum acceptable interest-coverage ratio.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_interest_coverage: Option<Multiple>,
    /// Minimum acceptable debt-service coverage ratio.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_dscr: Option<Multiple>,
    /// Minimum acceptable FFO-to-debt ratio.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_ffo_to_debt: Option<Rate>,
    /// Debt already outstanding.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub existing_debt: Option<Money>,
    /// Annual scheduled debt amortisation (principal repayment).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annual_amortisation: Option<Money>,
    /// Funds from operations (if known).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ffo: Option<Money>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebtCapacityOutput {
    pub max_debt_by_leverage: Option<Money>,
    pub max_debt_by_coverage: Option<Money>,
    pub max_debt_by_dscr: Option<Money>,
    pub max_debt_by_ffo: Option<Money>,
    /// The constraint that produces the lowest capacity.
    pub binding_constraint: String,
    /// Binding max minus existing debt.
    pub max_incremental_debt: Money,
    pub implied_leverage_at_max: Multiple,
    pub implied_coverage_at_max: Multiple,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Size the maximum debt capacity under multiple constraints and report the
/// binding (most restrictive) one.
pub fn calculate_debt_capacity(
    input: &DebtCapacityInput,
) -> CorpFinanceResult<ComputationOutput<DebtCapacityOutput>> {
    let start = Instant::now();
    let warnings: Vec<String> = Vec::new();

    validate_input(input)?;

    let existing = input.existing_debt.unwrap_or(Decimal::ZERO);

    // -- By leverage ----------------------------------------------------------
    let max_by_leverage = input.max_leverage.map(|ml| input.ebitda * ml);

    // -- By interest-coverage -------------------------------------------------
    // coverage = EBITDA / interest = EBITDA / (debt * rate)
    // => debt = EBITDA / (min_coverage * rate)
    let max_by_coverage = match input.min_interest_coverage {
        Some(min_cov) => {
            if input.interest_rate.is_zero() {
                None // infinite capacity if rate is zero
            } else {
                let denom = min_cov * input.interest_rate;
                if denom.is_zero() {
                    None
                } else {
                    Some(input.ebitda / denom)
                }
            }
        }
        None => None,
    };

    // -- By DSCR --------------------------------------------------------------
    // DSCR = EBITDA / (interest + amort) = EBITDA / (debt*rate + amort)
    // => debt = (EBITDA / min_dscr - amort) / rate
    let max_by_dscr = match input.min_dscr {
        Some(min_dscr) => {
            if input.interest_rate.is_zero() {
                None
            } else {
                let amort = input.annual_amortisation.unwrap_or(Decimal::ZERO);
                let service_per_unit_debt = input.interest_rate; // interest per $ of debt
                if service_per_unit_debt.is_zero() {
                    None
                } else {
                    // EBITDA >= min_dscr * (debt * rate + amort)
                    // debt <= (EBITDA / min_dscr - amort) / rate
                    let numerator = input.ebitda / min_dscr - amort;
                    if numerator <= Decimal::ZERO {
                        Some(Decimal::ZERO)
                    } else {
                        Some(numerator / service_per_unit_debt)
                    }
                }
            }
        }
        None => None,
    };

    // -- By FFO / debt --------------------------------------------------------
    // ffo_to_debt = FFO / debt >= min_ffo_to_debt
    // => debt <= FFO / min_ffo_to_debt
    let max_by_ffo = match (input.ffo, input.min_ffo_to_debt) {
        (Some(ffo), Some(min_ratio)) => {
            if min_ratio.is_zero() {
                None
            } else {
                Some(ffo / min_ratio)
            }
        }
        _ => None,
    };

    // -- Binding constraint ---------------------------------------------------
    let mut candidates: Vec<(&str, Money)> = Vec::new();
    if let Some(v) = max_by_leverage {
        candidates.push(("max_leverage", v));
    }
    if let Some(v) = max_by_coverage {
        candidates.push(("min_interest_coverage", v));
    }
    if let Some(v) = max_by_dscr {
        candidates.push(("min_dscr", v));
    }
    if let Some(v) = max_by_ffo {
        candidates.push(("min_ffo_to_debt", v));
    }

    if candidates.is_empty() {
        return Err(CorpFinanceError::InsufficientData(
            "At least one capacity constraint must be provided.".into(),
        ));
    }

    let (binding_name, binding_max) = candidates
        .iter()
        .min_by_key(|(_, v)| *v)
        .unwrap();

    let max_incremental = (*binding_max - existing).max(Decimal::ZERO);

    let implied_leverage = if input.ebitda.is_zero() {
        Decimal::ZERO
    } else {
        *binding_max / input.ebitda
    };

    let implied_interest = *binding_max * input.interest_rate;
    let implied_coverage = if implied_interest.is_zero() {
        dec!(999)
    } else {
        input.ebitda / implied_interest
    };

    let output = DebtCapacityOutput {
        max_debt_by_leverage: max_by_leverage,
        max_debt_by_coverage: max_by_coverage,
        max_debt_by_dscr: max_by_dscr,
        max_debt_by_ffo: max_by_ffo,
        binding_constraint: binding_name.to_string(),
        max_incremental_debt: max_incremental,
        implied_leverage_at_max: implied_leverage,
        implied_coverage_at_max: implied_coverage,
    };

    let elapsed = start.elapsed().as_micros() as u64;
    let assumptions = serde_json::json!({
        "dscr_amortisation": input.annual_amortisation.unwrap_or(Decimal::ZERO).to_string(),
        "existing_debt": existing.to_string(),
    });

    Ok(with_metadata(
        "Debt Capacity Sizing (multi-constraint)",
        &assumptions,
        warnings,
        elapsed,
        output,
    ))
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn validate_input(input: &DebtCapacityInput) -> CorpFinanceResult<()> {
    if input.ebitda <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "ebitda".into(),
            reason: "EBITDA must be positive for debt capacity sizing.".into(),
        });
    }
    if input.interest_rate < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "interest_rate".into(),
            reason: "Interest rate cannot be negative.".into(),
        });
    }
    if let Some(ml) = input.max_leverage {
        if ml <= Decimal::ZERO {
            return Err(CorpFinanceError::InvalidInput {
                field: "max_leverage".into(),
                reason: "Max leverage must be positive.".into(),
            });
        }
    }
    if let Some(mc) = input.min_interest_coverage {
        if mc <= Decimal::ZERO {
            return Err(CorpFinanceError::InvalidInput {
                field: "min_interest_coverage".into(),
                reason: "Min interest coverage must be positive.".into(),
            });
        }
    }
    if let Some(md) = input.min_dscr {
        if md <= Decimal::ZERO {
            return Err(CorpFinanceError::InvalidInput {
                field: "min_dscr".into(),
                reason: "Min DSCR must be positive.".into(),
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

    fn base_input() -> DebtCapacityInput {
        DebtCapacityInput {
            ebitda: dec!(100_000),
            interest_rate: dec!(0.05),
            max_leverage: Some(dec!(4.0)),
            min_interest_coverage: Some(dec!(3.0)),
            min_dscr: Some(dec!(1.5)),
            min_ffo_to_debt: Some(dec!(0.20)),
            existing_debt: Some(dec!(200_000)),
            annual_amortisation: Some(dec!(10_000)),
            ffo: Some(dec!(90_000)),
        }
    }

    #[test]
    fn test_leverage_capacity() {
        let input = base_input();
        let result = calculate_debt_capacity(&input).unwrap();
        // max_by_leverage = 100k * 4 = 400k
        assert_eq!(result.result.max_debt_by_leverage, Some(dec!(400_000)));
    }

    #[test]
    fn test_coverage_capacity() {
        let input = base_input();
        let result = calculate_debt_capacity(&input).unwrap();
        // max_by_coverage = 100k / (3 * 0.05) = 100k / 0.15 = 666_666.666...
        let expected = dec!(100_000) / dec!(0.15);
        assert_eq!(result.result.max_debt_by_coverage, Some(expected));
    }

    #[test]
    fn test_dscr_capacity() {
        let input = base_input();
        let result = calculate_debt_capacity(&input).unwrap();
        // debt <= (EBITDA / min_dscr - amort) / rate
        // = (100k / 1.5 - 10k) / 0.05
        let ebitda_over_dscr = dec!(100_000) / dec!(1.5);
        let numerator = ebitda_over_dscr - dec!(10_000);
        let expected = numerator / dec!(0.05);
        assert_eq!(result.result.max_debt_by_dscr, Some(expected));
    }

    #[test]
    fn test_ffo_capacity() {
        let input = base_input();
        let result = calculate_debt_capacity(&input).unwrap();
        // max_by_ffo = 90k / 0.20 = 450k
        assert_eq!(result.result.max_debt_by_ffo, Some(dec!(450_000)));
    }

    #[test]
    fn test_binding_constraint_is_minimum() {
        let input = base_input();
        let result = calculate_debt_capacity(&input).unwrap();
        let out = &result.result;

        let all_caps: Vec<Money> = [
            out.max_debt_by_leverage,
            out.max_debt_by_coverage,
            out.max_debt_by_dscr,
            out.max_debt_by_ffo,
        ]
        .iter()
        .filter_map(|x| *x)
        .collect();

        let min_cap = all_caps.iter().copied().min().unwrap();

        // incremental = binding - existing
        let expected_incremental = (min_cap - dec!(200_000)).max(Decimal::ZERO);
        assert_eq!(out.max_incremental_debt, expected_incremental);
    }

    #[test]
    fn test_leverage_only_constraint() {
        let input = DebtCapacityInput {
            ebitda: dec!(100_000),
            interest_rate: dec!(0.05),
            max_leverage: Some(dec!(3.0)),
            min_interest_coverage: None,
            min_dscr: None,
            min_ffo_to_debt: None,
            existing_debt: None,
            annual_amortisation: None,
            ffo: None,
        };
        let result = calculate_debt_capacity(&input).unwrap();
        assert_eq!(result.result.max_debt_by_leverage, Some(dec!(300_000)));
        assert_eq!(result.result.binding_constraint, "max_leverage");
        // No existing debt => incremental = 300k
        assert_eq!(result.result.max_incremental_debt, dec!(300_000));
    }

    #[test]
    fn test_no_constraints_fails() {
        let input = DebtCapacityInput {
            ebitda: dec!(100_000),
            interest_rate: dec!(0.05),
            max_leverage: None,
            min_interest_coverage: None,
            min_dscr: None,
            min_ffo_to_debt: None,
            existing_debt: None,
            annual_amortisation: None,
            ffo: None,
        };
        let err = calculate_debt_capacity(&input).unwrap_err();
        match err {
            CorpFinanceError::InsufficientData(_) => {} // expected
            other => panic!("Expected InsufficientData, got {other:?}"),
        }
    }

    #[test]
    fn test_negative_ebitda_rejected() {
        let mut input = base_input();
        input.ebitda = dec!(-50_000);
        let err = calculate_debt_capacity(&input).unwrap_err();
        match err {
            CorpFinanceError::InvalidInput { field, .. } => assert_eq!(field, "ebitda"),
            other => panic!("Expected InvalidInput, got {other:?}"),
        }
    }

    #[test]
    fn test_zero_rate_coverage_none() {
        let input = DebtCapacityInput {
            ebitda: dec!(100_000),
            interest_rate: Decimal::ZERO,
            max_leverage: Some(dec!(3.0)),
            min_interest_coverage: Some(dec!(2.0)),
            min_dscr: None,
            min_ffo_to_debt: None,
            existing_debt: None,
            annual_amortisation: None,
            ffo: None,
        };
        let result = calculate_debt_capacity(&input).unwrap();
        // With zero rate, coverage constraint is unconstrained => None
        assert_eq!(result.result.max_debt_by_coverage, None);
        // Leverage still binds
        assert_eq!(result.result.binding_constraint, "max_leverage");
    }

    #[test]
    fn test_implied_metrics() {
        let input = DebtCapacityInput {
            ebitda: dec!(100_000),
            interest_rate: dec!(0.05),
            max_leverage: Some(dec!(4.0)),
            min_interest_coverage: None,
            min_dscr: None,
            min_ffo_to_debt: None,
            existing_debt: None,
            annual_amortisation: None,
            ffo: None,
        };
        let result = calculate_debt_capacity(&input).unwrap();
        // implied leverage = 400k / 100k = 4.0
        assert_eq!(result.result.implied_leverage_at_max, dec!(4.0));
        // implied coverage = 100k / (400k * 0.05) = 100k / 20k = 5.0
        assert_eq!(result.result.implied_coverage_at_max, dec!(5));
    }

    #[test]
    fn test_metadata_populated() {
        let input = base_input();
        let result = calculate_debt_capacity(&input).unwrap();
        assert!(!result.methodology.is_empty());
        assert_eq!(result.metadata.precision, "rust_decimal_128bit");
    }
}
