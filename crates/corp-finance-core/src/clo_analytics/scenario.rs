//! CLO Scenario Analysis.
//!
//! Runs the CLO waterfall under multiple stress scenarios to compute:
//! - Per-tranche losses under each scenario
//! - Probability-weighted expected loss by tranche
//! - Attachment and detachment points
//!
//! All arithmetic uses `rust_decimal::Decimal`. No `f64`.

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

use crate::error::CorpFinanceError;
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Input / Output types
// ---------------------------------------------------------------------------

/// A tranche in the CLO capital structure (for scenario analysis).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioTranche {
    /// Tranche name.
    pub name: String,
    /// Rating label.
    pub rating: String,
    /// Initial notional balance.
    pub notional: Decimal,
    /// Spread over reference rate (decimal).
    pub spread: Decimal,
    /// Whether this is the equity tranche.
    pub is_equity: bool,
}

/// A stress scenario definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioDefinition {
    /// Scenario name (e.g. "Base", "Stress", "Severe").
    pub name: String,
    /// Annual conditional default rate (decimal).
    pub cdr: Decimal,
    /// Annual conditional prepayment rate (decimal).
    pub cpr: Decimal,
    /// Recovery rate (decimal).
    pub recovery: Decimal,
    /// Probability weight (decimal, e.g. 0.50 = 50%).
    pub probability: Decimal,
}

/// Input for CLO scenario analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloScenarioInput {
    /// Tranches ordered from most senior to equity.
    pub tranches: Vec<ScenarioTranche>,
    /// Initial collateral pool balance.
    pub pool_balance: Decimal,
    /// Weighted average spread of collateral.
    pub weighted_avg_spread: Decimal,
    /// Reference rate.
    pub reference_rate: Decimal,
    /// Scenarios to run.
    pub scenarios: Vec<ScenarioDefinition>,
    /// Number of projection periods.
    pub num_periods: u32,
}

/// Loss result for a single tranche in a scenario.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrancheScenarioLoss {
    /// Tranche name.
    pub name: String,
    /// Absolute loss amount.
    pub loss_amount: Decimal,
    /// Loss as percentage of tranche notional.
    pub loss_pct: Decimal,
}

/// Results for a single scenario.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioResult {
    /// Scenario name.
    pub scenario_name: String,
    /// Per-tranche losses.
    pub tranche_losses: Vec<TrancheScenarioLoss>,
}

/// Output of scenario analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloScenarioOutput {
    /// Per-scenario results.
    pub scenario_results: Vec<ScenarioResult>,
    /// Probability-weighted expected loss by tranche.
    pub expected_loss_by_tranche: Vec<(String, Decimal)>,
    /// Attachment points by tranche (percentage of pool).
    pub attachment_points: Vec<(String, Decimal)>,
    /// Detachment points by tranche (percentage of pool).
    pub detachment_points: Vec<(String, Decimal)>,
}

// ---------------------------------------------------------------------------
// Engine
// ---------------------------------------------------------------------------

/// Run CLO scenario analysis.
pub fn calculate_clo_scenario(input: &CloScenarioInput) -> CorpFinanceResult<CloScenarioOutput> {
    validate_scenario_input(input)?;

    let period_days: u32 = 90; // quarterly
    let basis = dec!(360);
    let period_frac = Decimal::from(period_days) / basis;

    // --- Compute attachment/detachment points ---
    let total_notional: Decimal = input.tranches.iter().map(|t| t.notional).sum();
    // Tranches are ordered senior to equity.
    // Equity attaches at 0%, detaches at equity_pct.
    // BBB attaches at equity_pct, detaches at equity_pct + bbb_pct. etc.
    // Senior tranche detaches at 100%.
    // We compute from bottom (equity) up.
    let mut cumulative_from_bottom = Decimal::ZERO;
    let reversed_indices: Vec<usize> = (0..input.tranches.len()).rev().collect();

    // Pre-allocate with placeholders
    let mut attach_vec: Vec<(String, Decimal)> = input
        .tranches
        .iter()
        .map(|t| (t.name.clone(), Decimal::ZERO))
        .collect();
    let mut detach_vec: Vec<(String, Decimal)> = input
        .tranches
        .iter()
        .map(|t| (t.name.clone(), Decimal::ZERO))
        .collect();

    for &i in &reversed_indices {
        let t = &input.tranches[i];
        let tranche_pct = if total_notional.is_zero() {
            Decimal::ZERO
        } else {
            t.notional / total_notional
        };
        attach_vec[i].1 = cumulative_from_bottom;
        cumulative_from_bottom += tranche_pct;
        detach_vec[i].1 = cumulative_from_bottom;
    }

    let attachment_points = attach_vec;
    let detachment_points = detach_vec;

    // --- Run scenarios ---
    let mut scenario_results: Vec<ScenarioResult> = Vec::with_capacity(input.scenarios.len());

    // For expected loss calculation
    let mut expected_loss_accum: Vec<Decimal> = vec![Decimal::ZERO; input.tranches.len()];

    for scenario in &input.scenarios {
        // Simplified waterfall for loss computation
        let cdr_periodic = scenario.cdr * period_frac;
        let cpr_periodic = scenario.cpr * period_frac;

        let mut pool = input.pool_balance;
        let mut tranche_bal: Vec<Decimal> = input.tranches.iter().map(|t| t.notional).collect();

        let mut cumulative_loss = Decimal::ZERO;

        for _period in 1..=input.num_periods {
            if pool <= Decimal::ZERO {
                break;
            }

            // Defaults
            let defaults = pool * cdr_periodic;
            let loss = defaults * (Decimal::ONE - scenario.recovery);

            // Prepayments
            let surviving = pool - defaults;
            let prepayments = if surviving > Decimal::ZERO {
                surviving * cpr_periodic
            } else {
                Decimal::ZERO
            };

            cumulative_loss += loss;

            // Collateral interest income (computed but not distributed in simplified scenario)
            let _interest_income =
                pool * (input.weighted_avg_spread + input.reference_rate) * period_frac;

            // Principal available (prepayments + recoveries received immediately for simplicity)
            let recoveries = defaults * scenario.recovery;
            let avail_principal = prepayments + recoveries;

            // Sequential principal distribution
            let mut remaining_principal = avail_principal;
            for (i, tranche) in input.tranches.iter().enumerate() {
                if tranche.is_equity {
                    continue;
                }
                let paid = tranche_bal[i].min(remaining_principal);
                remaining_principal -= paid;
                tranche_bal[i] -= paid;
            }
            // Equity gets remainder
            for (i, tranche) in input.tranches.iter().enumerate() {
                if tranche.is_equity {
                    let paid = tranche_bal[i].min(remaining_principal);
                    remaining_principal -= paid;
                    tranche_bal[i] -= paid;
                }
            }

            pool = pool - defaults - prepayments;
            if pool < Decimal::ZERO {
                pool = Decimal::ZERO;
            }
        }

        // Distribute cumulative losses bottom-up (equity first)
        let mut remaining_loss = cumulative_loss;
        let mut tranche_losses: Vec<TrancheScenarioLoss> = input
            .tranches
            .iter()
            .map(|t| TrancheScenarioLoss {
                name: t.name.clone(),
                loss_amount: Decimal::ZERO,
                loss_pct: Decimal::ZERO,
            })
            .collect();

        // Bottom-up loss allocation
        for &i in &reversed_indices {
            let t = &input.tranches[i];
            let loss_to_tranche = t.notional.min(remaining_loss);
            remaining_loss -= loss_to_tranche;
            tranche_losses[i].loss_amount = loss_to_tranche;
            tranche_losses[i].loss_pct = if t.notional.is_zero() {
                Decimal::ZERO
            } else {
                loss_to_tranche / t.notional
            };

            // Accumulate for expected loss
            expected_loss_accum[i] += loss_to_tranche * scenario.probability;
        }

        scenario_results.push(ScenarioResult {
            scenario_name: scenario.name.clone(),
            tranche_losses,
        });
    }

    let expected_loss_by_tranche: Vec<(String, Decimal)> = input
        .tranches
        .iter()
        .enumerate()
        .map(|(i, t)| (t.name.clone(), expected_loss_accum[i]))
        .collect();

    Ok(CloScenarioOutput {
        scenario_results,
        expected_loss_by_tranche,
        attachment_points,
        detachment_points,
    })
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

fn validate_scenario_input(input: &CloScenarioInput) -> CorpFinanceResult<()> {
    if input.tranches.is_empty() {
        return Err(CorpFinanceError::InsufficientData(
            "At least one tranche is required.".into(),
        ));
    }
    if input.scenarios.is_empty() {
        return Err(CorpFinanceError::InsufficientData(
            "At least one scenario is required.".into(),
        ));
    }
    if input.pool_balance <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "pool_balance".into(),
            reason: "Pool balance must be positive.".into(),
        });
    }
    if input.num_periods == 0 {
        return Err(CorpFinanceError::InvalidInput {
            field: "num_periods".into(),
            reason: "Must have at least one projection period.".into(),
        });
    }
    for s in &input.scenarios {
        if s.cdr < Decimal::ZERO || s.cdr > Decimal::ONE {
            return Err(CorpFinanceError::InvalidInput {
                field: format!("scenario.{}.cdr", s.name),
                reason: "CDR must be in [0, 1].".into(),
            });
        }
        if s.cpr < Decimal::ZERO || s.cpr > Decimal::ONE {
            return Err(CorpFinanceError::InvalidInput {
                field: format!("scenario.{}.cpr", s.name),
                reason: "CPR must be in [0, 1].".into(),
            });
        }
        if s.recovery < Decimal::ZERO || s.recovery > Decimal::ONE {
            return Err(CorpFinanceError::InvalidInput {
                field: format!("scenario.{}.recovery", s.name),
                reason: "Recovery must be in [0, 1].".into(),
            });
        }
        if s.probability < Decimal::ZERO || s.probability > Decimal::ONE {
            return Err(CorpFinanceError::InvalidInput {
                field: format!("scenario.{}.probability", s.name),
                reason: "Probability must be in [0, 1].".into(),
            });
        }
    }
    for t in &input.tranches {
        if t.notional < Decimal::ZERO {
            return Err(CorpFinanceError::InvalidInput {
                field: format!("tranche.{}.notional", t.name),
                reason: "Tranche notional cannot be negative.".into(),
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

    fn sample_tranches() -> Vec<ScenarioTranche> {
        vec![
            ScenarioTranche {
                name: "AAA".into(),
                rating: "AAA".into(),
                notional: dec!(300_000_000),
                spread: dec!(0.0130),
                is_equity: false,
            },
            ScenarioTranche {
                name: "AA".into(),
                rating: "AA".into(),
                notional: dec!(50_000_000),
                spread: dec!(0.0180),
                is_equity: false,
            },
            ScenarioTranche {
                name: "A".into(),
                rating: "A".into(),
                notional: dec!(40_000_000),
                spread: dec!(0.0250),
                is_equity: false,
            },
            ScenarioTranche {
                name: "BBB".into(),
                rating: "BBB".into(),
                notional: dec!(30_000_000),
                spread: dec!(0.0400),
                is_equity: false,
            },
            ScenarioTranche {
                name: "Equity".into(),
                rating: "NR".into(),
                notional: dec!(80_000_000),
                spread: Decimal::ZERO,
                is_equity: true,
            },
        ]
    }

    fn sample_scenarios() -> Vec<ScenarioDefinition> {
        vec![
            ScenarioDefinition {
                name: "Base".into(),
                cdr: dec!(0.02),
                cpr: dec!(0.15),
                recovery: dec!(0.40),
                probability: dec!(0.50),
            },
            ScenarioDefinition {
                name: "Stress".into(),
                cdr: dec!(0.05),
                cpr: dec!(0.10),
                recovery: dec!(0.30),
                probability: dec!(0.30),
            },
            ScenarioDefinition {
                name: "Severe".into(),
                cdr: dec!(0.10),
                cpr: dec!(0.05),
                recovery: dec!(0.20),
                probability: dec!(0.20),
            },
        ]
    }

    fn sample_input() -> CloScenarioInput {
        CloScenarioInput {
            tranches: sample_tranches(),
            pool_balance: dec!(500_000_000),
            weighted_avg_spread: dec!(0.0350),
            reference_rate: dec!(0.05),
            scenarios: sample_scenarios(),
            num_periods: 20,
        }
    }

    #[test]
    fn test_scenario_results_count() {
        let input = sample_input();
        let out = calculate_clo_scenario(&input).unwrap();
        assert_eq!(out.scenario_results.len(), 3);
    }

    #[test]
    fn test_each_scenario_has_all_tranche_losses() {
        let input = sample_input();
        let out = calculate_clo_scenario(&input).unwrap();
        for sr in &out.scenario_results {
            assert_eq!(sr.tranche_losses.len(), 5);
        }
    }

    #[test]
    fn test_base_scenario_lower_losses_than_severe() {
        let input = sample_input();
        let out = calculate_clo_scenario(&input).unwrap();
        let base_total: Decimal = out.scenario_results[0]
            .tranche_losses
            .iter()
            .map(|tl| tl.loss_amount)
            .sum();
        let severe_total: Decimal = out.scenario_results[2]
            .tranche_losses
            .iter()
            .map(|tl| tl.loss_amount)
            .sum();
        assert!(
            base_total <= severe_total,
            "Base losses {} should be <= Severe losses {}",
            base_total,
            severe_total
        );
    }

    #[test]
    fn test_equity_absorbs_first_loss() {
        let input = sample_input();
        let out = calculate_clo_scenario(&input).unwrap();
        // In base scenario, equity should absorb losses first
        let base = &out.scenario_results[0];
        let equity_loss = &base.tranche_losses[4]; // equity is last
                                                   // Total losses should go to equity first
        let total_loss: Decimal = base.tranche_losses.iter().map(|tl| tl.loss_amount).sum();
        if total_loss > Decimal::ZERO {
            assert!(
                equity_loss.loss_amount > Decimal::ZERO,
                "Equity should absorb first loss"
            );
        }
    }

    #[test]
    fn test_loss_pct_bounded_0_1() {
        let input = sample_input();
        let out = calculate_clo_scenario(&input).unwrap();
        for sr in &out.scenario_results {
            for tl in &sr.tranche_losses {
                assert!(tl.loss_pct >= Decimal::ZERO && tl.loss_pct <= Decimal::ONE);
            }
        }
    }

    #[test]
    fn test_expected_loss_weighted_by_probability() {
        let input = sample_input();
        let out = calculate_clo_scenario(&input).unwrap();
        // Expected loss = sum(loss_i * prob_i) across scenarios
        for (i, (name, el)) in out.expected_loss_by_tranche.iter().enumerate() {
            let manual: Decimal = out
                .scenario_results
                .iter()
                .zip(input.scenarios.iter())
                .map(|(sr, s)| sr.tranche_losses[i].loss_amount * s.probability)
                .sum();
            assert!(
                approx_eq(*el, manual, dec!(1)),
                "Expected loss for {} {} should be ~{}",
                name,
                el,
                manual
            );
        }
    }

    #[test]
    fn test_attachment_points_count() {
        let input = sample_input();
        let out = calculate_clo_scenario(&input).unwrap();
        assert_eq!(out.attachment_points.len(), 5);
    }

    #[test]
    fn test_detachment_points_count() {
        let input = sample_input();
        let out = calculate_clo_scenario(&input).unwrap();
        assert_eq!(out.detachment_points.len(), 5);
    }

    #[test]
    fn test_equity_attaches_at_zero() {
        let input = sample_input();
        let out = calculate_clo_scenario(&input).unwrap();
        // Equity is last tranche (index 4)
        let eq_attach = out.attachment_points[4].1;
        assert_eq!(eq_attach, Decimal::ZERO, "Equity should attach at 0%");
    }

    #[test]
    fn test_aaa_detaches_at_100() {
        let input = sample_input();
        let out = calculate_clo_scenario(&input).unwrap();
        // AAA is first tranche (index 0), detaches at 100%
        let aaa_detach = out.detachment_points[0].1;
        assert!(
            approx_eq(aaa_detach, Decimal::ONE, dec!(0.001)),
            "AAA should detach at ~100%, got {}",
            aaa_detach
        );
    }

    #[test]
    fn test_attachment_less_than_detachment() {
        let input = sample_input();
        let out = calculate_clo_scenario(&input).unwrap();
        for i in 0..input.tranches.len() {
            assert!(
                out.attachment_points[i].1 < out.detachment_points[i].1,
                "Attachment should be < detachment for {}",
                input.tranches[i].name
            );
        }
    }

    #[test]
    fn test_consecutive_attach_detach() {
        let input = sample_input();
        let out = calculate_clo_scenario(&input).unwrap();
        // Each tranche's detachment = next tranche's attachment (going bottom-up)
        // Equity detach = BBB attach, BBB detach = A attach, etc.
        let equity_idx = 4;
        let bbb_idx = 3;
        assert!(
            approx_eq(
                out.detachment_points[equity_idx].1,
                out.attachment_points[bbb_idx].1,
                dec!(0.001)
            ),
            "Equity detach {} should equal BBB attach {}",
            out.detachment_points[equity_idx].1,
            out.attachment_points[bbb_idx].1
        );
    }

    #[test]
    fn test_zero_cdr_no_losses() {
        let mut input = sample_input();
        input.scenarios = vec![ScenarioDefinition {
            name: "NoDefault".into(),
            cdr: Decimal::ZERO,
            cpr: dec!(0.10),
            recovery: dec!(0.40),
            probability: Decimal::ONE,
        }];
        let out = calculate_clo_scenario(&input).unwrap();
        for tl in &out.scenario_results[0].tranche_losses {
            assert_eq!(tl.loss_amount, Decimal::ZERO);
        }
    }

    #[test]
    fn test_expected_loss_count() {
        let input = sample_input();
        let out = calculate_clo_scenario(&input).unwrap();
        assert_eq!(out.expected_loss_by_tranche.len(), 5);
    }

    #[test]
    fn test_reject_empty_tranches() {
        let mut input = sample_input();
        input.tranches = vec![];
        assert!(calculate_clo_scenario(&input).is_err());
    }

    #[test]
    fn test_reject_empty_scenarios() {
        let mut input = sample_input();
        input.scenarios = vec![];
        assert!(calculate_clo_scenario(&input).is_err());
    }

    #[test]
    fn test_reject_negative_pool_balance() {
        let mut input = sample_input();
        input.pool_balance = dec!(-100);
        assert!(calculate_clo_scenario(&input).is_err());
    }

    #[test]
    fn test_reject_cdr_out_of_range() {
        let mut input = sample_input();
        input.scenarios[0].cdr = dec!(1.5);
        assert!(calculate_clo_scenario(&input).is_err());
    }

    #[test]
    fn test_reject_cpr_out_of_range() {
        let mut input = sample_input();
        input.scenarios[0].cpr = dec!(-0.01);
        assert!(calculate_clo_scenario(&input).is_err());
    }

    #[test]
    fn test_reject_recovery_out_of_range() {
        let mut input = sample_input();
        input.scenarios[0].recovery = dec!(1.1);
        assert!(calculate_clo_scenario(&input).is_err());
    }

    #[test]
    fn test_reject_probability_out_of_range() {
        let mut input = sample_input();
        input.scenarios[0].probability = dec!(1.5);
        assert!(calculate_clo_scenario(&input).is_err());
    }

    #[test]
    fn test_reject_zero_periods() {
        let mut input = sample_input();
        input.num_periods = 0;
        assert!(calculate_clo_scenario(&input).is_err());
    }

    #[test]
    fn test_single_scenario() {
        let mut input = sample_input();
        input.scenarios = vec![ScenarioDefinition {
            name: "Single".into(),
            cdr: dec!(0.03),
            cpr: dec!(0.10),
            recovery: dec!(0.35),
            probability: Decimal::ONE,
        }];
        let out = calculate_clo_scenario(&input).unwrap();
        assert_eq!(out.scenario_results.len(), 1);
    }

    #[test]
    fn test_serialization_roundtrip() {
        let input = sample_input();
        let out = calculate_clo_scenario(&input).unwrap();
        let json = serde_json::to_string(&out).unwrap();
        let _: CloScenarioOutput = serde_json::from_str(&json).unwrap();
    }
}
