//! CECL / IFRS 9 expected credit loss provisioning.
//!
//! Covers:
//! 1. **12-month ECL** -- PD(1yr) x LGD x EAD for Stage 1.
//! 2. **Lifetime ECL** -- sum of discounted PD x LGD x EAD over remaining life.
//! 3. **IFRS 9 staging** -- Stage 1 (12-month), Stage 2/3 (lifetime).
//! 4. **Scenario weighting** -- base/adverse/severe probability-weighted ECL.
//! 5. **Coverage ratio** -- total ECL / total exposure.
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

/// A single loan segment for CECL/IFRS 9 calculation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoanSegment {
    /// Segment name.
    pub name: String,
    /// Exposure at default (balance).
    pub balance: Decimal,
    /// Probability of default under base scenario (annual).
    pub pd_base: Decimal,
    /// Probability of default under adverse scenario.
    pub pd_adverse: Decimal,
    /// Probability of default under severe scenario.
    pub pd_severe: Decimal,
    /// Loss given default (0-1).
    pub lgd: Decimal,
    /// Remaining life in years.
    pub remaining_life: Decimal,
    /// IFRS 9 stage: 1, 2, or 3.
    pub stage: u8,
}

/// Scenario probability weights.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioWeights {
    pub base: Decimal,
    pub adverse: Decimal,
    pub severe: Decimal,
}

/// Input for CECL provisioning calculation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CeclProvisioningInput {
    /// Loan segments.
    pub segments: Vec<LoanSegment>,
    /// Scenario weights (must sum to 1).
    pub scenario_weights: ScenarioWeights,
    /// Discount rate for lifetime ECL.
    pub discount_rate: Decimal,
}

/// Per-segment ECL result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SegmentEclResult {
    /// Segment name.
    pub name: String,
    /// 12-month ECL (PD x LGD x EAD).
    pub ecl_12month: Decimal,
    /// Lifetime ECL (discounted sum).
    pub ecl_lifetime: Decimal,
    /// Applied ECL (12-month for stage 1, lifetime for stage 2/3).
    pub applied_ecl: Decimal,
    /// Scenario-weighted ECL.
    pub weighted_ecl: Decimal,
    /// Stage (1, 2, or 3).
    pub stage: u8,
}

/// Scenario ECL breakdown.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioBreakdown {
    pub base_ecl: Decimal,
    pub adverse_ecl: Decimal,
    pub severe_ecl: Decimal,
}

/// Output of CECL provisioning calculation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CeclProvisioningOutput {
    /// Total expected credit loss.
    pub total_ecl: Decimal,
    /// Total exposure across all segments.
    pub total_exposure: Decimal,
    /// ECL coverage ratio (total_ecl / total_exposure).
    pub ecl_coverage_ratio: Decimal,
    /// Per-segment results.
    pub segment_results: Vec<SegmentEclResult>,
    /// Breakdown by scenario.
    pub scenario_breakdown: ScenarioBreakdown,
}

// ---------------------------------------------------------------------------
// Core function
// ---------------------------------------------------------------------------

/// Calculate CECL/IFRS 9 expected credit losses.
pub fn calculate_cecl(input: &CeclProvisioningInput) -> CorpFinanceResult<CeclProvisioningOutput> {
    validate_cecl_input(input)?;

    let mut segment_results = Vec::with_capacity(input.segments.len());
    let mut total_ecl = Decimal::ZERO;
    let mut total_exposure = Decimal::ZERO;
    let mut total_base_ecl = Decimal::ZERO;
    let mut total_adverse_ecl = Decimal::ZERO;
    let mut total_severe_ecl = Decimal::ZERO;

    for seg in &input.segments {
        total_exposure += seg.balance;

        // Calculate ECL for each scenario
        let base_12m = seg.pd_base * seg.lgd * seg.balance;
        let adverse_12m = seg.pd_adverse * seg.lgd * seg.balance;
        let severe_12m = seg.pd_severe * seg.lgd * seg.balance;

        let base_lifetime = lifetime_ecl(
            seg.pd_base,
            seg.lgd,
            seg.balance,
            seg.remaining_life,
            input.discount_rate,
        );
        let adverse_lifetime = lifetime_ecl(
            seg.pd_adverse,
            seg.lgd,
            seg.balance,
            seg.remaining_life,
            input.discount_rate,
        );
        let severe_lifetime = lifetime_ecl(
            seg.pd_severe,
            seg.lgd,
            seg.balance,
            seg.remaining_life,
            input.discount_rate,
        );

        // Apply staging
        let (applied_base, applied_adverse, applied_severe) = match seg.stage {
            1 => (base_12m, adverse_12m, severe_12m),
            _ => (base_lifetime, adverse_lifetime, severe_lifetime),
        };

        // Scenario-weighted ECL
        let weighted = input.scenario_weights.base * applied_base
            + input.scenario_weights.adverse * applied_adverse
            + input.scenario_weights.severe * applied_severe;

        total_ecl += weighted;
        total_base_ecl += applied_base;
        total_adverse_ecl += applied_adverse;
        total_severe_ecl += applied_severe;

        // For display, use base scenario 12m and lifetime
        let ecl_12month = base_12m;
        let ecl_lifetime = base_lifetime;
        let applied_ecl = match seg.stage {
            1 => ecl_12month,
            _ => ecl_lifetime,
        };

        segment_results.push(SegmentEclResult {
            name: seg.name.clone(),
            ecl_12month,
            ecl_lifetime,
            applied_ecl,
            weighted_ecl: weighted,
            stage: seg.stage,
        });
    }

    let ecl_coverage_ratio = if total_exposure > Decimal::ZERO {
        total_ecl / total_exposure
    } else {
        Decimal::ZERO
    };

    Ok(CeclProvisioningOutput {
        total_ecl,
        total_exposure,
        ecl_coverage_ratio,
        segment_results,
        scenario_breakdown: ScenarioBreakdown {
            base_ecl: total_base_ecl,
            adverse_ecl: total_adverse_ecl,
            severe_ecl: total_severe_ecl,
        },
    })
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Lifetime ECL = sum_{t=1}^{remaining_life} PD * LGD * EAD / (1+r)^t
/// Uses iterative discount factor (not powd).
fn lifetime_ecl(
    pd: Decimal,
    lgd: Decimal,
    ead: Decimal,
    remaining_life: Decimal,
    discount_rate: Decimal,
) -> Decimal {
    let annual_loss = pd * lgd * ead;
    let periods = remaining_life
        .floor()
        .to_string()
        .parse::<u32>()
        .unwrap_or(0);
    let mut sum = Decimal::ZERO;
    let mut discount_factor = Decimal::ONE;
    let one_plus_r = Decimal::ONE + discount_rate;

    for _t in 0..periods {
        discount_factor *= one_plus_r;
        sum += annual_loss / discount_factor;
    }

    // Handle fractional year
    let frac = remaining_life - Decimal::from(periods);
    if frac > Decimal::ZERO {
        discount_factor *= one_plus_r;
        sum += annual_loss * frac / discount_factor;
    }

    sum
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

fn validate_cecl_input(input: &CeclProvisioningInput) -> CorpFinanceResult<()> {
    if input.segments.is_empty() {
        return Err(CorpFinanceError::InsufficientData(
            "At least one loan segment is required.".into(),
        ));
    }

    // Scenario weights must sum to 1
    let weight_sum = input.scenario_weights.base
        + input.scenario_weights.adverse
        + input.scenario_weights.severe;
    if (weight_sum - Decimal::ONE).abs() > dec!(0.001) {
        return Err(CorpFinanceError::InvalidInput {
            field: "scenario_weights".into(),
            reason: format!("Scenario weights must sum to 1.0, got {}.", weight_sum),
        });
    }

    for w in [
        input.scenario_weights.base,
        input.scenario_weights.adverse,
        input.scenario_weights.severe,
    ] {
        if w < Decimal::ZERO {
            return Err(CorpFinanceError::InvalidInput {
                field: "scenario_weights".into(),
                reason: "Scenario weights cannot be negative.".into(),
            });
        }
    }

    if input.discount_rate < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "discount_rate".into(),
            reason: "Discount rate cannot be negative.".into(),
        });
    }

    for seg in &input.segments {
        if seg.balance < Decimal::ZERO {
            return Err(CorpFinanceError::InvalidInput {
                field: "balance".into(),
                reason: format!("Segment '{}' has negative balance.", seg.name),
            });
        }
        if seg.pd_base < Decimal::ZERO || seg.pd_base > Decimal::ONE {
            return Err(CorpFinanceError::InvalidInput {
                field: "pd_base".into(),
                reason: format!("Segment '{}' PD base must be in [0, 1].", seg.name),
            });
        }
        if seg.pd_adverse < Decimal::ZERO || seg.pd_adverse > Decimal::ONE {
            return Err(CorpFinanceError::InvalidInput {
                field: "pd_adverse".into(),
                reason: format!("Segment '{}' PD adverse must be in [0, 1].", seg.name),
            });
        }
        if seg.pd_severe < Decimal::ZERO || seg.pd_severe > Decimal::ONE {
            return Err(CorpFinanceError::InvalidInput {
                field: "pd_severe".into(),
                reason: format!("Segment '{}' PD severe must be in [0, 1].", seg.name),
            });
        }
        if seg.lgd < Decimal::ZERO || seg.lgd > Decimal::ONE {
            return Err(CorpFinanceError::InvalidInput {
                field: "lgd".into(),
                reason: format!("Segment '{}' LGD must be in [0, 1].", seg.name),
            });
        }
        if seg.remaining_life <= Decimal::ZERO {
            return Err(CorpFinanceError::InvalidInput {
                field: "remaining_life".into(),
                reason: format!("Segment '{}' remaining life must be positive.", seg.name),
            });
        }
        if seg.stage < 1 || seg.stage > 3 {
            return Err(CorpFinanceError::InvalidInput {
                field: "stage".into(),
                reason: format!("Segment '{}' stage must be 1, 2, or 3.", seg.name),
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

    fn single_segment_input() -> CeclProvisioningInput {
        CeclProvisioningInput {
            segments: vec![LoanSegment {
                name: "Commercial".into(),
                balance: dec!(100_000_000),
                pd_base: dec!(0.02),
                pd_adverse: dec!(0.05),
                pd_severe: dec!(0.10),
                lgd: dec!(0.40),
                remaining_life: dec!(5),
                stage: 1,
            }],
            scenario_weights: ScenarioWeights {
                base: dec!(0.60),
                adverse: dec!(0.30),
                severe: dec!(0.10),
            },
            discount_rate: dec!(0.05),
        }
    }

    fn multi_segment_input() -> CeclProvisioningInput {
        CeclProvisioningInput {
            segments: vec![
                LoanSegment {
                    name: "Mortgage".into(),
                    balance: dec!(200_000_000),
                    pd_base: dec!(0.01),
                    pd_adverse: dec!(0.03),
                    pd_severe: dec!(0.06),
                    lgd: dec!(0.25),
                    remaining_life: dec!(10),
                    stage: 1,
                },
                LoanSegment {
                    name: "CRE Watchlist".into(),
                    balance: dec!(50_000_000),
                    pd_base: dec!(0.05),
                    pd_adverse: dec!(0.10),
                    pd_severe: dec!(0.20),
                    lgd: dec!(0.45),
                    remaining_life: dec!(3),
                    stage: 2,
                },
                LoanSegment {
                    name: "Impaired C&I".into(),
                    balance: dec!(10_000_000),
                    pd_base: dec!(0.30),
                    pd_adverse: dec!(0.50),
                    pd_severe: dec!(0.70),
                    lgd: dec!(0.60),
                    remaining_life: dec!(2),
                    stage: 3,
                },
            ],
            scenario_weights: ScenarioWeights {
                base: dec!(0.50),
                adverse: dec!(0.35),
                severe: dec!(0.15),
            },
            discount_rate: dec!(0.05),
        }
    }

    #[test]
    fn test_single_segment_12month_ecl() {
        let input = single_segment_input();
        let out = calculate_cecl(&input).unwrap();
        // 12m ECL = 0.02 * 0.40 * 100M = 800,000
        assert_eq!(out.segment_results[0].ecl_12month, dec!(800_000));
    }

    #[test]
    fn test_single_segment_stage_1_uses_12month() {
        let input = single_segment_input();
        let out = calculate_cecl(&input).unwrap();
        assert_eq!(out.segment_results[0].stage, 1);
        assert_eq!(
            out.segment_results[0].applied_ecl,
            out.segment_results[0].ecl_12month
        );
    }

    #[test]
    fn test_lifetime_ecl_greater_than_12month() {
        let input = single_segment_input();
        let out = calculate_cecl(&input).unwrap();
        assert!(
            out.segment_results[0].ecl_lifetime > out.segment_results[0].ecl_12month,
            "Lifetime ECL should exceed 12-month for multi-year loan"
        );
    }

    #[test]
    fn test_stage_2_uses_lifetime() {
        let mut input = single_segment_input();
        input.segments[0].stage = 2;
        let out = calculate_cecl(&input).unwrap();
        assert_eq!(
            out.segment_results[0].applied_ecl,
            out.segment_results[0].ecl_lifetime
        );
    }

    #[test]
    fn test_stage_3_uses_lifetime() {
        let mut input = single_segment_input();
        input.segments[0].stage = 3;
        let out = calculate_cecl(&input).unwrap();
        assert_eq!(
            out.segment_results[0].applied_ecl,
            out.segment_results[0].ecl_lifetime
        );
    }

    #[test]
    fn test_scenario_weighting() {
        let input = single_segment_input();
        let out = calculate_cecl(&input).unwrap();
        // Weighted ECL should be between base and severe ECL
        let _base_ecl = out.segment_results[0].ecl_12month;
        // Adverse 12m = 0.05 * 0.40 * 100M = 2,000,000
        // Severe 12m = 0.10 * 0.40 * 100M = 4,000,000
        // Weighted = 0.60*800k + 0.30*2M + 0.10*4M = 480k + 600k + 400k = 1,480,000
        let expected_weighted = dec!(1_480_000);
        assert!(
            approx_eq(
                out.segment_results[0].weighted_ecl,
                expected_weighted,
                dec!(1)
            ),
            "Expected weighted ECL ~{}, got {}",
            expected_weighted,
            out.segment_results[0].weighted_ecl
        );
    }

    #[test]
    fn test_100_pct_base_weight() {
        let mut input = single_segment_input();
        input.scenario_weights = ScenarioWeights {
            base: Decimal::ONE,
            adverse: Decimal::ZERO,
            severe: Decimal::ZERO,
        };
        let out = calculate_cecl(&input).unwrap();
        // Weighted ECL should equal base-scenario applied ECL
        assert_eq!(
            out.segment_results[0].weighted_ecl,
            out.segment_results[0].applied_ecl
        );
    }

    #[test]
    fn test_total_exposure() {
        let input = multi_segment_input();
        let out = calculate_cecl(&input).unwrap();
        assert_eq!(out.total_exposure, dec!(260_000_000));
    }

    #[test]
    fn test_total_ecl_is_sum_of_segments() {
        let input = multi_segment_input();
        let out = calculate_cecl(&input).unwrap();
        let sum: Decimal = out.segment_results.iter().map(|s| s.weighted_ecl).sum();
        assert!(approx_eq(out.total_ecl, sum, dec!(0.01)));
    }

    #[test]
    fn test_ecl_coverage_ratio() {
        let input = multi_segment_input();
        let out = calculate_cecl(&input).unwrap();
        let expected = out.total_ecl / out.total_exposure;
        assert!(approx_eq(out.ecl_coverage_ratio, expected, dec!(0.000001)));
    }

    #[test]
    fn test_scenario_breakdown_adverse_gt_base() {
        let input = multi_segment_input();
        let out = calculate_cecl(&input).unwrap();
        assert!(
            out.scenario_breakdown.adverse_ecl > out.scenario_breakdown.base_ecl,
            "Adverse ECL should exceed base ECL"
        );
    }

    #[test]
    fn test_scenario_breakdown_severe_gt_adverse() {
        let input = multi_segment_input();
        let out = calculate_cecl(&input).unwrap();
        assert!(
            out.scenario_breakdown.severe_ecl > out.scenario_breakdown.adverse_ecl,
            "Severe ECL should exceed adverse ECL"
        );
    }

    #[test]
    fn test_zero_pd_gives_zero_ecl() {
        let mut input = single_segment_input();
        input.segments[0].pd_base = Decimal::ZERO;
        input.segments[0].pd_adverse = Decimal::ZERO;
        input.segments[0].pd_severe = Decimal::ZERO;
        let out = calculate_cecl(&input).unwrap();
        assert_eq!(out.segment_results[0].ecl_12month, Decimal::ZERO);
        assert_eq!(out.segment_results[0].weighted_ecl, Decimal::ZERO);
    }

    #[test]
    fn test_high_pd_scenario() {
        let mut input = single_segment_input();
        input.segments[0].pd_base = dec!(0.50);
        input.segments[0].pd_adverse = dec!(0.70);
        input.segments[0].pd_severe = dec!(0.90);
        input.segments[0].stage = 3;
        let out = calculate_cecl(&input).unwrap();
        // High PD should produce significant ECL
        assert!(out.total_ecl > dec!(10_000_000));
    }

    #[test]
    fn test_discount_effect() {
        // Higher discount rate -> lower lifetime ECL
        let input1 = CeclProvisioningInput {
            discount_rate: dec!(0.01),
            ..single_segment_input()
        };
        let input2 = CeclProvisioningInput {
            discount_rate: dec!(0.10),
            ..single_segment_input()
        };
        let out1 = calculate_cecl(&input1).unwrap();
        let out2 = calculate_cecl(&input2).unwrap();
        assert!(
            out1.segment_results[0].ecl_lifetime > out2.segment_results[0].ecl_lifetime,
            "Higher discount rate should reduce lifetime ECL"
        );
    }

    #[test]
    fn test_fractional_remaining_life() {
        let mut input = single_segment_input();
        input.segments[0].remaining_life = dec!(2.5);
        input.segments[0].stage = 2;
        let out = calculate_cecl(&input).unwrap();
        // Should handle fractional year
        assert!(out.segment_results[0].ecl_lifetime > Decimal::ZERO);
    }

    #[test]
    fn test_reject_empty_segments() {
        let input = CeclProvisioningInput {
            segments: vec![],
            scenario_weights: ScenarioWeights {
                base: dec!(0.60),
                adverse: dec!(0.30),
                severe: dec!(0.10),
            },
            discount_rate: dec!(0.05),
        };
        assert!(calculate_cecl(&input).is_err());
    }

    #[test]
    fn test_reject_weights_not_summing_to_one() {
        let mut input = single_segment_input();
        input.scenario_weights.base = dec!(0.50);
        // 0.50 + 0.30 + 0.10 = 0.90 != 1.0
        assert!(calculate_cecl(&input).is_err());
    }

    #[test]
    fn test_reject_negative_balance() {
        let mut input = single_segment_input();
        input.segments[0].balance = dec!(-100);
        assert!(calculate_cecl(&input).is_err());
    }

    #[test]
    fn test_reject_pd_above_one() {
        let mut input = single_segment_input();
        input.segments[0].pd_base = dec!(1.5);
        assert!(calculate_cecl(&input).is_err());
    }

    #[test]
    fn test_reject_lgd_above_one() {
        let mut input = single_segment_input();
        input.segments[0].lgd = dec!(1.1);
        assert!(calculate_cecl(&input).is_err());
    }

    #[test]
    fn test_reject_invalid_stage() {
        let mut input = single_segment_input();
        input.segments[0].stage = 4;
        assert!(calculate_cecl(&input).is_err());
    }

    #[test]
    fn test_reject_zero_remaining_life() {
        let mut input = single_segment_input();
        input.segments[0].remaining_life = Decimal::ZERO;
        assert!(calculate_cecl(&input).is_err());
    }

    #[test]
    fn test_reject_negative_discount_rate() {
        let mut input = single_segment_input();
        input.discount_rate = dec!(-0.01);
        assert!(calculate_cecl(&input).is_err());
    }

    #[test]
    fn test_serialization_roundtrip() {
        let input = multi_segment_input();
        let out = calculate_cecl(&input).unwrap();
        let json = serde_json::to_string(&out).unwrap();
        let _: CeclProvisioningOutput = serde_json::from_str(&json).unwrap();
    }
}
