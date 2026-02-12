//! Multi-Stage Dividend Discount Model.
//!
//! Supports an arbitrary number of growth stages, each with a fixed growth
//! rate and duration, followed by a Gordon Growth terminal value.
//!
//! Key features:
//! 1. Year-by-year dividend projection with iterative discounting.
//! 2. Per-stage PV decomposition.
//! 3. Terminal value via Gordon Growth Model at the end of the last stage.
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

/// A single growth stage in the multi-stage DDM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrowthStage {
    /// Duration of this stage in years.
    pub years: u32,
    /// Dividend growth rate during this stage.
    pub growth_rate: Decimal,
}

/// Input for multi-stage DDM calculation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultistageDdmInput {
    /// Current dividend per share (D₀).
    pub d0: Decimal,
    /// Required rate of return.
    pub r: Decimal,
    /// Growth stages (in chronological order).
    pub stages: Vec<GrowthStage>,
    /// Perpetual growth rate after all explicit stages.
    pub terminal_growth: Decimal,
}

/// PV detail for a single growth stage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageValueDetail {
    /// Stage number (1-indexed).
    pub stage_num: u32,
    /// Present value of all dividends from this stage.
    pub pv: Decimal,
    /// Total undiscounted dividends paid during this stage.
    pub dividends_paid: Decimal,
}

/// Year-by-year dividend detail.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YearDetail {
    /// Year number (1-indexed).
    pub year: u32,
    /// Projected dividend for this year.
    pub dividend: Decimal,
    /// Present value of this year's dividend.
    pub pv: Decimal,
}

/// Output of the multi-stage DDM calculation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultistageDdmOutput {
    /// Intrinsic value per share.
    pub intrinsic_value: Decimal,
    /// PV decomposition by growth stage.
    pub stage_values: Vec<StageValueDetail>,
    /// Present value of the Gordon Growth terminal value.
    pub terminal_value: Decimal,
    /// Terminal value as a percentage of total intrinsic value.
    pub terminal_pct: Decimal,
    /// Year-by-year dividend and PV detail.
    pub year_by_year: Vec<YearDetail>,
}

// ---------------------------------------------------------------------------
// Core function
// ---------------------------------------------------------------------------

/// Calculate the multi-stage DDM intrinsic value.
pub fn calculate_multistage_ddm(
    input: &MultistageDdmInput,
) -> CorpFinanceResult<MultistageDdmOutput> {
    validate_input(input)?;

    let r = input.r;
    let df_multiplier = Decimal::ONE / (Decimal::ONE + r);

    let mut current_dividend = input.d0;
    let mut discount_factor = Decimal::ONE; // df at t=0
    let mut year_counter: u32 = 0;

    let mut stage_values = Vec::with_capacity(input.stages.len());
    let mut year_by_year = Vec::new();
    let mut total_pv_dividends = Decimal::ZERO;

    for (idx, stage) in input.stages.iter().enumerate() {
        let mut stage_pv = Decimal::ZERO;
        let mut stage_dividends = Decimal::ZERO;

        for _y in 0..stage.years {
            year_counter += 1;
            // Grow dividend
            current_dividend *= Decimal::ONE + stage.growth_rate;
            // Advance discount factor
            discount_factor *= df_multiplier;

            let pv = current_dividend * discount_factor;
            stage_pv += pv;
            stage_dividends += current_dividend;
            total_pv_dividends += pv;

            year_by_year.push(YearDetail {
                year: year_counter,
                dividend: current_dividend,
                pv,
            });
        }

        stage_values.push(StageValueDetail {
            stage_num: (idx + 1) as u32,
            pv: stage_pv,
            dividends_paid: stage_dividends,
        });
    }

    // Terminal value at the end of the last stage
    // TV = D_T * (1 + g_terminal) / (r - g_terminal)
    let terminal_dividend = current_dividend * (Decimal::ONE + input.terminal_growth);
    let tv_undiscounted = terminal_dividend / (r - input.terminal_growth);
    let terminal_value = tv_undiscounted * discount_factor;

    let intrinsic_value = total_pv_dividends + terminal_value;

    let terminal_pct = if intrinsic_value == Decimal::ZERO {
        Decimal::ZERO
    } else {
        terminal_value / intrinsic_value * dec!(100)
    };

    Ok(MultistageDdmOutput {
        intrinsic_value,
        stage_values,
        terminal_value,
        terminal_pct,
        year_by_year,
    })
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

fn validate_input(input: &MultistageDdmInput) -> CorpFinanceResult<()> {
    if input.d0 < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "d0".into(),
            reason: "Current dividend must be non-negative.".into(),
        });
    }
    if input.r <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "r".into(),
            reason: "Required rate of return must be positive.".into(),
        });
    }
    if input.stages.is_empty() {
        return Err(CorpFinanceError::InsufficientData(
            "At least one growth stage is required.".into(),
        ));
    }
    if input.r <= input.terminal_growth {
        return Err(CorpFinanceError::FinancialImpossibility(
            "Required return (r) must exceed terminal growth rate for convergent valuation.".into(),
        ));
    }
    // Validate total years don't exceed reasonable limit
    let total_years: u32 = input.stages.iter().map(|s| s.years).sum();
    if total_years == 0 {
        return Err(CorpFinanceError::InvalidInput {
            field: "stages".into(),
            reason: "Total explicit years across all stages must be at least 1.".into(),
        });
    }
    if total_years > 200 {
        return Err(CorpFinanceError::InvalidInput {
            field: "stages".into(),
            reason: "Total years exceeds 200 — likely an error.".into(),
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

    fn single_stage_input() -> MultistageDdmInput {
        // Single stage: effectively a Gordon Growth after 5 years at 8%
        MultistageDdmInput {
            d0: dec!(2.00),
            r: dec!(0.10),
            stages: vec![GrowthStage {
                years: 5,
                growth_rate: dec!(0.08),
            }],
            terminal_growth: dec!(0.03),
        }
    }

    fn two_stage_input() -> MultistageDdmInput {
        MultistageDdmInput {
            d0: dec!(1.50),
            r: dec!(0.12),
            stages: vec![
                GrowthStage {
                    years: 3,
                    growth_rate: dec!(0.20),
                },
                GrowthStage {
                    years: 4,
                    growth_rate: dec!(0.10),
                },
            ],
            terminal_growth: dec!(0.04),
        }
    }

    fn three_stage_input() -> MultistageDdmInput {
        MultistageDdmInput {
            d0: dec!(1.00),
            r: dec!(0.11),
            stages: vec![
                GrowthStage {
                    years: 3,
                    growth_rate: dec!(0.25),
                },
                GrowthStage {
                    years: 3,
                    growth_rate: dec!(0.15),
                },
                GrowthStage {
                    years: 4,
                    growth_rate: dec!(0.08),
                },
            ],
            terminal_growth: dec!(0.03),
        }
    }

    #[test]
    fn test_single_stage_basic() {
        let input = single_stage_input();
        let out = calculate_multistage_ddm(&input).unwrap();
        assert!(out.intrinsic_value > Decimal::ZERO);
        assert_eq!(out.stage_values.len(), 1);
        assert_eq!(out.year_by_year.len(), 5);
    }

    #[test]
    fn test_single_stage_year_count() {
        let input = single_stage_input();
        let out = calculate_multistage_ddm(&input).unwrap();
        for (i, yd) in out.year_by_year.iter().enumerate() {
            assert_eq!(yd.year, (i + 1) as u32);
        }
    }

    #[test]
    fn test_dividends_grow_each_year() {
        let input = single_stage_input();
        let out = calculate_multistage_ddm(&input).unwrap();
        for i in 1..out.year_by_year.len() {
            assert!(out.year_by_year[i].dividend > out.year_by_year[i - 1].dividend);
        }
    }

    #[test]
    fn test_pv_decreases_over_time() {
        // With moderate growth and discounting, PV should generally decrease
        let input = MultistageDdmInput {
            d0: dec!(2.00),
            r: dec!(0.15),
            stages: vec![GrowthStage {
                years: 10,
                growth_rate: dec!(0.03),
            }],
            terminal_growth: dec!(0.02),
        };
        let out = calculate_multistage_ddm(&input).unwrap();
        // PV of last year should be less than PV of first year when r >> g
        assert!(out.year_by_year.last().unwrap().pv < out.year_by_year[0].pv);
    }

    #[test]
    fn test_two_stage_basic() {
        let input = two_stage_input();
        let out = calculate_multistage_ddm(&input).unwrap();
        assert_eq!(out.stage_values.len(), 2);
        assert_eq!(out.year_by_year.len(), 7);
        assert!(out.intrinsic_value > Decimal::ZERO);
    }

    #[test]
    fn test_three_stage_basic() {
        let input = three_stage_input();
        let out = calculate_multistage_ddm(&input).unwrap();
        assert_eq!(out.stage_values.len(), 3);
        assert_eq!(out.year_by_year.len(), 10);
    }

    #[test]
    fn test_terminal_dominates() {
        // Short explicit period, terminal should dominate
        let input = MultistageDdmInput {
            d0: dec!(1.00),
            r: dec!(0.10),
            stages: vec![GrowthStage {
                years: 1,
                growth_rate: dec!(0.05),
            }],
            terminal_growth: dec!(0.04),
        };
        let out = calculate_multistage_ddm(&input).unwrap();
        assert!(out.terminal_pct > dec!(90));
    }

    #[test]
    fn test_terminal_pct_sum_to_100() {
        let input = two_stage_input();
        let out = calculate_multistage_ddm(&input).unwrap();
        let stage_pv_sum: Decimal = out.stage_values.iter().map(|s| s.pv).sum();
        let stage_pct = stage_pv_sum / out.intrinsic_value * dec!(100);
        let total_pct = stage_pct + out.terminal_pct;
        assert!(approx_eq(total_pct, dec!(100), dec!(0.01)));
    }

    #[test]
    fn test_sum_of_year_pv_equals_stage_pv() {
        let input = two_stage_input();
        let out = calculate_multistage_ddm(&input).unwrap();

        // Stage 1: years 1-3, Stage 2: years 4-7
        let stage1_pv: Decimal = out.year_by_year[0..3].iter().map(|y| y.pv).sum();
        let stage2_pv: Decimal = out.year_by_year[3..7].iter().map(|y| y.pv).sum();

        assert!(approx_eq(stage1_pv, out.stage_values[0].pv, dec!(0.0001)));
        assert!(approx_eq(stage2_pv, out.stage_values[1].pv, dec!(0.0001)));
    }

    #[test]
    fn test_intrinsic_equals_sum_stages_plus_terminal() {
        let input = three_stage_input();
        let out = calculate_multistage_ddm(&input).unwrap();
        let stage_sum: Decimal = out.stage_values.iter().map(|s| s.pv).sum();
        let total = stage_sum + out.terminal_value;
        assert!(approx_eq(out.intrinsic_value, total, dec!(0.0001)));
    }

    #[test]
    fn test_first_year_dividend() {
        let input = single_stage_input();
        let out = calculate_multistage_ddm(&input).unwrap();
        // D1 = D0 * (1 + g) = 2.00 * 1.08 = 2.16
        assert!(approx_eq(
            out.year_by_year[0].dividend,
            dec!(2.16),
            dec!(0.0001)
        ));
    }

    #[test]
    fn test_first_year_pv() {
        let input = single_stage_input();
        let out = calculate_multistage_ddm(&input).unwrap();
        // PV1 = 2.16 / 1.10 = 1.96363636
        let expected = dec!(2.16) / dec!(1.10);
        assert!(approx_eq(out.year_by_year[0].pv, expected, dec!(0.001)));
    }

    #[test]
    fn test_reject_empty_stages() {
        let input = MultistageDdmInput {
            d0: dec!(1),
            r: dec!(0.10),
            stages: vec![],
            terminal_growth: dec!(0.03),
        };
        assert!(calculate_multistage_ddm(&input).is_err());
    }

    #[test]
    fn test_reject_r_leq_terminal_growth() {
        let input = MultistageDdmInput {
            d0: dec!(1),
            r: dec!(0.03),
            stages: vec![GrowthStage {
                years: 5,
                growth_rate: dec!(0.10),
            }],
            terminal_growth: dec!(0.03),
        };
        assert!(calculate_multistage_ddm(&input).is_err());
    }

    #[test]
    fn test_reject_negative_dividend() {
        let input = MultistageDdmInput {
            d0: dec!(-1),
            ..single_stage_input()
        };
        assert!(calculate_multistage_ddm(&input).is_err());
    }

    #[test]
    fn test_reject_zero_r() {
        let input = MultistageDdmInput {
            d0: dec!(1),
            r: Decimal::ZERO,
            stages: vec![GrowthStage {
                years: 5,
                growth_rate: dec!(0.10),
            }],
            terminal_growth: dec!(-0.01),
        };
        assert!(calculate_multistage_ddm(&input).is_err());
    }

    #[test]
    fn test_reject_excessive_years() {
        let input = MultistageDdmInput {
            d0: dec!(1),
            r: dec!(0.10),
            stages: vec![GrowthStage {
                years: 201,
                growth_rate: dec!(0.05),
            }],
            terminal_growth: dec!(0.03),
        };
        assert!(calculate_multistage_ddm(&input).is_err());
    }

    #[test]
    fn test_reject_zero_total_years() {
        let input = MultistageDdmInput {
            d0: dec!(1),
            r: dec!(0.10),
            stages: vec![GrowthStage {
                years: 0,
                growth_rate: dec!(0.05),
            }],
            terminal_growth: dec!(0.03),
        };
        assert!(calculate_multistage_ddm(&input).is_err());
    }

    #[test]
    fn test_zero_growth_all_stages() {
        let input = MultistageDdmInput {
            d0: dec!(4.00),
            r: dec!(0.08),
            stages: vec![GrowthStage {
                years: 5,
                growth_rate: Decimal::ZERO,
            }],
            terminal_growth: Decimal::ZERO,
        };
        // r > terminal_growth check: 0.08 > 0 => OK
        let out = calculate_multistage_ddm(&input).unwrap();
        // All dividends = D0 = 4.00
        for yd in &out.year_by_year {
            assert!(approx_eq(yd.dividend, dec!(4.00), dec!(0.0001)));
        }
    }

    #[test]
    fn test_negative_terminal_growth() {
        let input = MultistageDdmInput {
            d0: dec!(2.00),
            r: dec!(0.10),
            stages: vec![GrowthStage {
                years: 3,
                growth_rate: dec!(0.05),
            }],
            terminal_growth: dec!(-0.02),
        };
        let out = calculate_multistage_ddm(&input).unwrap();
        assert!(out.intrinsic_value > Decimal::ZERO);
        // Terminal value should be smaller with negative growth
        assert!(out.terminal_value > Decimal::ZERO);
    }

    #[test]
    fn test_stage_dividends_paid() {
        let input = single_stage_input();
        let out = calculate_multistage_ddm(&input).unwrap();
        let manual_sum: Decimal = out.year_by_year.iter().map(|y| y.dividend).sum();
        assert!(approx_eq(
            out.stage_values[0].dividends_paid,
            manual_sum,
            dec!(0.0001)
        ));
    }

    #[test]
    fn test_serialization_roundtrip() {
        let input = two_stage_input();
        let out = calculate_multistage_ddm(&input).unwrap();
        let json = serde_json::to_string(&out).unwrap();
        let _: MultistageDdmOutput = serde_json::from_str(&json).unwrap();
    }
}
