//! CLO OC/IC Coverage Tests.
//!
//! Implements over-collateralisation (OC) and interest coverage (IC) tests
//! for CLO structures:
//! - OC Ratio = (collateral_par - defaulted) / cumulative_tranche_notional
//! - IC Ratio = (interest_income - senior_fees) / cumulative_interest_due
//! - Trigger breaches and cure mechanics (excess interest diversion)
//!
//! All arithmetic uses `rust_decimal::Decimal`. No `f64`.

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::error::CorpFinanceError;
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Input / Output types
// ---------------------------------------------------------------------------

/// A tranche for coverage test purposes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageTestTranche {
    /// Tranche name.
    pub name: String,
    /// Outstanding notional balance.
    pub notional: Decimal,
    /// Spread over reference rate (decimal).
    pub spread: Decimal,
    /// OC trigger level (ratio, e.g. 1.20 = 120%).
    pub oc_trigger: Decimal,
    /// IC trigger level (ratio, e.g. 1.50 = 150%).
    pub ic_trigger: Decimal,
}

/// Input for OC/IC coverage tests.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageTestInput {
    /// Tranches ordered from most senior to most junior (excluding equity).
    pub tranches: Vec<CoverageTestTranche>,
    /// Current collateral pool par value.
    pub pool_par: Decimal,
    /// Par value of defaulted assets (already removed from performing pool).
    pub defaulted_par: Decimal,
    /// Periodic interest income from the collateral pool.
    pub interest_income: Decimal,
    /// Senior fees (management, trustee, etc.) for the period.
    pub senior_fees: Decimal,
    /// Reference rate (SOFR/LIBOR, decimal).
    pub reference_rate: Decimal,
}

/// Result for a single tranche.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageTestTrancheResult {
    /// Tranche name.
    pub name: String,
    /// OC ratio at this tranche level.
    pub oc_ratio: Decimal,
    /// OC trigger.
    pub oc_trigger: Decimal,
    /// Whether OC test passes.
    pub oc_pass: bool,
    /// IC ratio at this tranche level.
    pub ic_ratio: Decimal,
    /// IC trigger.
    pub ic_trigger: Decimal,
    /// Whether IC test passes.
    pub ic_pass: bool,
    /// Amount that must be diverted to cure the OC breach (0 if passing).
    pub diversion_amount: Decimal,
}

/// Output of coverage tests.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageTestOutput {
    /// Per-tranche results.
    pub tranche_results: Vec<CoverageTestTrancheResult>,
    /// Whether any OC test is breached.
    pub any_oc_breach: bool,
    /// Whether any IC test is breached.
    pub any_ic_breach: bool,
    /// Total amount to be diverted to cure breaches.
    pub total_diversion: Decimal,
}

// ---------------------------------------------------------------------------
// Engine
// ---------------------------------------------------------------------------

/// Compute OC and IC coverage tests for all tranches.
pub fn calculate_coverage_tests(
    input: &CoverageTestInput,
) -> CorpFinanceResult<CoverageTestOutput> {
    validate_coverage_input(input)?;

    let adjusted_par = input.pool_par - input.defaulted_par;
    let net_interest = input.interest_income - input.senior_fees;

    let mut tranche_results: Vec<CoverageTestTrancheResult> =
        Vec::with_capacity(input.tranches.len());
    let mut any_oc_breach = false;
    let mut any_ic_breach = false;
    let mut total_diversion = Decimal::ZERO;

    // Compute cumulative notional and cumulative interest due from senior down
    let mut cumulative_notional = Decimal::ZERO;
    let mut cumulative_interest_due = Decimal::ZERO;

    for tranche in &input.tranches {
        cumulative_notional += tranche.notional;
        // Interest due on this tranche (periodic)
        let tranche_interest = tranche.notional * (tranche.spread + input.reference_rate);
        cumulative_interest_due += tranche_interest;

        // OC ratio = adjusted_par / cumulative_notional
        let oc_ratio = if cumulative_notional.is_zero() {
            Decimal::ZERO
        } else {
            adjusted_par / cumulative_notional
        };

        // IC ratio = net_interest / cumulative_interest_due
        let ic_ratio = if cumulative_interest_due.is_zero() {
            Decimal::ZERO
        } else {
            net_interest / cumulative_interest_due
        };

        let oc_pass = oc_ratio >= tranche.oc_trigger;
        let ic_pass = ic_ratio >= tranche.ic_trigger;

        if !oc_pass {
            any_oc_breach = true;
        }
        if !ic_pass {
            any_ic_breach = true;
        }

        // Diversion amount: how much principal must be paid down to cure OC
        // To cure: (adjusted_par) / (cumulative_notional - diversion) >= trigger
        // => cumulative_notional - diversion <= adjusted_par / trigger
        // => diversion >= cumulative_notional - adjusted_par / trigger
        let diversion_amount = if !oc_pass && tranche.oc_trigger > Decimal::ZERO {
            let target_notional = adjusted_par / tranche.oc_trigger;
            let needed = cumulative_notional - target_notional;
            if needed > Decimal::ZERO {
                needed
            } else {
                Decimal::ZERO
            }
        } else {
            Decimal::ZERO
        };

        total_diversion += diversion_amount;

        tranche_results.push(CoverageTestTrancheResult {
            name: tranche.name.clone(),
            oc_ratio,
            oc_trigger: tranche.oc_trigger,
            oc_pass,
            ic_ratio,
            ic_trigger: tranche.ic_trigger,
            ic_pass,
            diversion_amount,
        });
    }

    Ok(CoverageTestOutput {
        tranche_results,
        any_oc_breach,
        any_ic_breach,
        total_diversion,
    })
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

fn validate_coverage_input(input: &CoverageTestInput) -> CorpFinanceResult<()> {
    if input.tranches.is_empty() {
        return Err(CorpFinanceError::InsufficientData(
            "At least one tranche is required for coverage tests.".into(),
        ));
    }
    if input.pool_par < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "pool_par".into(),
            reason: "Pool par cannot be negative.".into(),
        });
    }
    if input.defaulted_par < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "defaulted_par".into(),
            reason: "Defaulted par cannot be negative.".into(),
        });
    }
    if input.interest_income < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "interest_income".into(),
            reason: "Interest income cannot be negative.".into(),
        });
    }
    if input.senior_fees < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "senior_fees".into(),
            reason: "Senior fees cannot be negative.".into(),
        });
    }
    for t in &input.tranches {
        if t.notional < Decimal::ZERO {
            return Err(CorpFinanceError::InvalidInput {
                field: format!("tranche.{}.notional", t.name),
                reason: "Tranche notional cannot be negative.".into(),
            });
        }
        if t.oc_trigger <= Decimal::ZERO {
            return Err(CorpFinanceError::InvalidInput {
                field: format!("tranche.{}.oc_trigger", t.name),
                reason: "OC trigger must be positive.".into(),
            });
        }
        if t.ic_trigger <= Decimal::ZERO {
            return Err(CorpFinanceError::InvalidInput {
                field: format!("tranche.{}.ic_trigger", t.name),
                reason: "IC trigger must be positive.".into(),
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

    fn sample_tranches() -> Vec<CoverageTestTranche> {
        vec![
            CoverageTestTranche {
                name: "AAA".into(),
                notional: dec!(300_000_000),
                spread: dec!(0.0130),
                oc_trigger: dec!(1.20),
                ic_trigger: dec!(1.50),
            },
            CoverageTestTranche {
                name: "AA".into(),
                notional: dec!(50_000_000),
                spread: dec!(0.0180),
                oc_trigger: dec!(1.15),
                ic_trigger: dec!(1.40),
            },
            CoverageTestTranche {
                name: "A".into(),
                notional: dec!(40_000_000),
                spread: dec!(0.0250),
                oc_trigger: dec!(1.10),
                ic_trigger: dec!(1.30),
            },
            CoverageTestTranche {
                name: "BBB".into(),
                notional: dec!(30_000_000),
                spread: dec!(0.0400),
                oc_trigger: dec!(1.05),
                ic_trigger: dec!(1.20),
            },
        ]
    }

    fn sample_input() -> CoverageTestInput {
        CoverageTestInput {
            tranches: sample_tranches(),
            pool_par: dec!(500_000_000),
            defaulted_par: dec!(10_000_000),
            interest_income: dec!(10_000_000),
            senior_fees: dec!(500_000),
            reference_rate: dec!(0.05),
        }
    }

    #[test]
    fn test_coverage_produces_results_for_all_tranches() {
        let input = sample_input();
        let out = calculate_coverage_tests(&input).unwrap();
        assert_eq!(out.tranche_results.len(), 4);
    }

    #[test]
    fn test_oc_ratio_aaa_correct() {
        let input = sample_input();
        let out = calculate_coverage_tests(&input).unwrap();
        // adjusted_par = 500M - 10M = 490M
        // cumulative at AAA = 300M
        // OC = 490/300 = 1.6333...
        let expected = dec!(490_000_000) / dec!(300_000_000);
        assert!(
            approx_eq(out.tranche_results[0].oc_ratio, expected, dec!(0.0001)),
            "AAA OC {} should be ~{}",
            out.tranche_results[0].oc_ratio,
            expected
        );
    }

    #[test]
    fn test_oc_ratio_decreases_down_stack() {
        let input = sample_input();
        let out = calculate_coverage_tests(&input).unwrap();
        for i in 1..out.tranche_results.len() {
            assert!(
                out.tranche_results[i].oc_ratio <= out.tranche_results[i - 1].oc_ratio,
                "OC should decrease down the capital structure"
            );
        }
    }

    #[test]
    fn test_ic_ratio_aaa_correct() {
        let input = sample_input();
        let out = calculate_coverage_tests(&input).unwrap();
        // net_interest = 10M - 0.5M = 9.5M
        // AAA interest due = 300M * (0.013 + 0.05) = 300M * 0.063 = 18.9M
        // IC = 9.5M / 18.9M
        let net_interest = dec!(9_500_000);
        let aaa_int = dec!(300_000_000) * dec!(0.063);
        let expected = net_interest / aaa_int;
        assert!(
            approx_eq(out.tranche_results[0].ic_ratio, expected, dec!(0.001)),
            "AAA IC {} should be ~{}",
            out.tranche_results[0].ic_ratio,
            expected
        );
    }

    #[test]
    fn test_all_tests_pass_healthy_deal() {
        // High par, low defaults => all should pass
        let input = CoverageTestInput {
            tranches: vec![CoverageTestTranche {
                name: "AAA".into(),
                notional: dec!(100_000_000),
                spread: dec!(0.0130),
                oc_trigger: dec!(1.10),
                ic_trigger: dec!(1.00),
            }],
            pool_par: dec!(500_000_000),
            defaulted_par: Decimal::ZERO,
            interest_income: dec!(50_000_000),
            senior_fees: Decimal::ZERO,
            reference_rate: dec!(0.05),
        };
        let out = calculate_coverage_tests(&input).unwrap();
        assert!(out.tranche_results[0].oc_pass);
        assert!(out.tranche_results[0].ic_pass);
        assert!(!out.any_oc_breach);
        assert!(!out.any_ic_breach);
    }

    #[test]
    fn test_oc_breach_triggers_diversion() {
        // Low par => OC breach
        let input = CoverageTestInput {
            tranches: vec![CoverageTestTranche {
                name: "AAA".into(),
                notional: dec!(100_000_000),
                spread: dec!(0.0130),
                oc_trigger: dec!(1.50),
                ic_trigger: dec!(1.00),
            }],
            pool_par: dec!(120_000_000),
            defaulted_par: dec!(10_000_000),
            interest_income: dec!(10_000_000),
            senior_fees: Decimal::ZERO,
            reference_rate: dec!(0.05),
        };
        let out = calculate_coverage_tests(&input).unwrap();
        // adjusted = 110M, OC = 110/100 = 1.10 < 1.50
        assert!(!out.tranche_results[0].oc_pass);
        assert!(out.any_oc_breach);
        assert!(out.tranche_results[0].diversion_amount > Decimal::ZERO);
    }

    #[test]
    fn test_diversion_amount_cures_oc() {
        let input = CoverageTestInput {
            tranches: vec![CoverageTestTranche {
                name: "AAA".into(),
                notional: dec!(100_000_000),
                spread: dec!(0.0130),
                oc_trigger: dec!(1.20),
                ic_trigger: dec!(1.00),
            }],
            pool_par: dec!(110_000_000),
            defaulted_par: Decimal::ZERO,
            interest_income: dec!(10_000_000),
            senior_fees: Decimal::ZERO,
            reference_rate: dec!(0.05),
        };
        let out = calculate_coverage_tests(&input).unwrap();
        // adjusted = 110M, OC = 110/100 = 1.10 < 1.20
        // target_notional = 110/1.20 = 91.666...M
        // diversion = 100 - 91.666... = 8.333...M
        let diversion = out.tranche_results[0].diversion_amount;
        let new_notional = dec!(100_000_000) - diversion;
        let new_oc = dec!(110_000_000) / new_notional;
        assert!(
            approx_eq(new_oc, dec!(1.20), dec!(0.01)),
            "After cure, OC {} should be ~1.20",
            new_oc
        );
    }

    #[test]
    fn test_ic_breach_detected() {
        let input = CoverageTestInput {
            tranches: vec![CoverageTestTranche {
                name: "AAA".into(),
                notional: dec!(100_000_000),
                spread: dec!(0.0130),
                oc_trigger: dec!(1.00),
                ic_trigger: dec!(2.00),
            }],
            pool_par: dec!(200_000_000),
            defaulted_par: Decimal::ZERO,
            interest_income: dec!(5_000_000),
            senior_fees: dec!(1_000_000),
            reference_rate: dec!(0.05),
        };
        let out = calculate_coverage_tests(&input).unwrap();
        // net_interest = 4M, interest_due = 100M * 0.063 = 6.3M
        // IC = 4/6.3 = 0.634... < 2.0
        assert!(!out.tranche_results[0].ic_pass);
        assert!(out.any_ic_breach);
    }

    #[test]
    fn test_no_diversion_when_passing() {
        let input = sample_input();
        let out = calculate_coverage_tests(&input).unwrap();
        // With 490M par and 300M AAA notional, OC = 1.63 > 1.20 trigger
        assert_eq!(out.tranche_results[0].diversion_amount, Decimal::ZERO);
    }

    #[test]
    fn test_cumulative_notional_across_tranches() {
        let input = sample_input();
        let out = calculate_coverage_tests(&input).unwrap();
        // At BBB level: cumulative = 300+50+40+30 = 420M
        // OC at BBB = 490/420 = 1.1666...
        let expected = dec!(490_000_000) / dec!(420_000_000);
        assert!(
            approx_eq(out.tranche_results[3].oc_ratio, expected, dec!(0.001)),
            "BBB OC {} should be ~{}",
            out.tranche_results[3].oc_ratio,
            expected
        );
    }

    #[test]
    fn test_zero_defaults_maximum_oc() {
        let mut input = sample_input();
        input.defaulted_par = Decimal::ZERO;
        let out = calculate_coverage_tests(&input).unwrap();
        // OC at AAA = 500/300 = 1.666...
        let expected = dec!(500_000_000) / dec!(300_000_000);
        assert!(
            approx_eq(out.tranche_results[0].oc_ratio, expected, dec!(0.001)),
            "AAA OC {} should be ~{}",
            out.tranche_results[0].oc_ratio,
            expected
        );
    }

    #[test]
    fn test_total_diversion_sum() {
        let input = CoverageTestInput {
            tranches: vec![
                CoverageTestTranche {
                    name: "AAA".into(),
                    notional: dec!(100_000_000),
                    spread: dec!(0.0130),
                    oc_trigger: dec!(1.50),
                    ic_trigger: dec!(1.00),
                },
                CoverageTestTranche {
                    name: "AA".into(),
                    notional: dec!(50_000_000),
                    spread: dec!(0.0180),
                    oc_trigger: dec!(1.40),
                    ic_trigger: dec!(1.00),
                },
            ],
            pool_par: dec!(130_000_000),
            defaulted_par: dec!(10_000_000),
            interest_income: dec!(50_000_000),
            senior_fees: Decimal::ZERO,
            reference_rate: dec!(0.05),
        };
        let out = calculate_coverage_tests(&input).unwrap();
        let sum: Decimal = out.tranche_results.iter().map(|r| r.diversion_amount).sum();
        assert_eq!(out.total_diversion, sum);
    }

    #[test]
    fn test_reject_empty_tranches() {
        let mut input = sample_input();
        input.tranches = vec![];
        assert!(calculate_coverage_tests(&input).is_err());
    }

    #[test]
    fn test_reject_negative_pool_par() {
        let mut input = sample_input();
        input.pool_par = dec!(-1);
        assert!(calculate_coverage_tests(&input).is_err());
    }

    #[test]
    fn test_reject_negative_defaulted_par() {
        let mut input = sample_input();
        input.defaulted_par = dec!(-1);
        assert!(calculate_coverage_tests(&input).is_err());
    }

    #[test]
    fn test_reject_negative_interest_income() {
        let mut input = sample_input();
        input.interest_income = dec!(-1);
        assert!(calculate_coverage_tests(&input).is_err());
    }

    #[test]
    fn test_reject_negative_senior_fees() {
        let mut input = sample_input();
        input.senior_fees = dec!(-1);
        assert!(calculate_coverage_tests(&input).is_err());
    }

    #[test]
    fn test_reject_zero_oc_trigger() {
        let mut input = sample_input();
        input.tranches[0].oc_trigger = Decimal::ZERO;
        assert!(calculate_coverage_tests(&input).is_err());
    }

    #[test]
    fn test_reject_zero_ic_trigger() {
        let mut input = sample_input();
        input.tranches[0].ic_trigger = Decimal::ZERO;
        assert!(calculate_coverage_tests(&input).is_err());
    }

    #[test]
    fn test_serialization_roundtrip() {
        let input = sample_input();
        let out = calculate_coverage_tests(&input).unwrap();
        let json = serde_json::to_string(&out).unwrap();
        let _: CoverageTestOutput = serde_json::from_str(&json).unwrap();
    }

    #[test]
    fn test_single_tranche_coverage() {
        let input = CoverageTestInput {
            tranches: vec![CoverageTestTranche {
                name: "Senior".into(),
                notional: dec!(80_000_000),
                spread: dec!(0.0200),
                oc_trigger: dec!(1.25),
                ic_trigger: dec!(1.50),
            }],
            pool_par: dec!(100_000_000),
            defaulted_par: Decimal::ZERO,
            interest_income: dec!(10_000_000),
            senior_fees: Decimal::ZERO,
            reference_rate: dec!(0.05),
        };
        let out = calculate_coverage_tests(&input).unwrap();
        // OC = 100/80 = 1.25 >= 1.25 => pass
        assert!(out.tranche_results[0].oc_pass);
        assert_eq!(out.tranche_results[0].diversion_amount, Decimal::ZERO);
    }
}
