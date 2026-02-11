use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::error::CorpFinanceError;
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SectorWeight {
    pub sector: String,
    pub portfolio_weight: Decimal,
    pub benchmark_weight: Decimal,
    pub portfolio_return: Decimal,
    pub benchmark_return: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrinsonInput {
    pub portfolio_name: String,
    pub benchmark_name: String,
    pub sectors: Vec<SectorWeight>,
    pub risk_free_rate: Decimal,
    pub periods: Option<Vec<PeriodData>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeriodData {
    pub period_name: String,
    pub sectors: Vec<SectorWeight>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SectorAttribution {
    pub sector: String,
    pub allocation_effect: Decimal,
    pub selection_effect: Decimal,
    pub interaction_effect: Decimal,
    pub total_effect: Decimal,
    pub portfolio_contribution: Decimal,
    pub benchmark_contribution: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrinsonOutput {
    pub portfolio_return: Decimal,
    pub benchmark_return: Decimal,
    pub active_return: Decimal,
    pub total_allocation: Decimal,
    pub total_selection: Decimal,
    pub total_interaction: Decimal,
    pub sector_attribution: Vec<SectorAttribution>,
    pub information_ratio: Option<Decimal>,
    pub multi_period_linked: Option<Vec<PeriodAttribution>>,
    pub methodology: String,
    pub assumptions: HashMap<String, String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeriodAttribution {
    pub period_name: String,
    pub portfolio_return: Decimal,
    pub benchmark_return: Decimal,
    pub active_return: Decimal,
    pub allocation: Decimal,
    pub selection: Decimal,
    pub interaction: Decimal,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Natural logarithm via series ln(x) = 2 * sum_{k=0..N} u^(2k+1)/(2k+1)
/// where u = (x-1)/(x+1).
fn decimal_ln(x: Decimal) -> Decimal {
    if x <= Decimal::ZERO {
        return Decimal::ZERO;
    }
    let u = (x - Decimal::ONE) / (x + Decimal::ONE);
    let u_sq = u * u;
    let mut term = u;
    let mut sum = u;
    for k in 1..40u32 {
        term *= u_sq;
        let denom = Decimal::from(2 * k + 1);
        sum += term / denom;
    }
    sum * Decimal::from(2)
}

/// Square root via Newton's method (20 iterations).
fn decimal_sqrt(x: Decimal) -> Decimal {
    if x <= Decimal::ZERO {
        return Decimal::ZERO;
    }
    let mut guess = x / Decimal::from(2);
    if guess == Decimal::ZERO {
        guess = Decimal::ONE;
    }
    for _ in 0..20 {
        guess = (guess + x / guess) / Decimal::from(2);
    }
    guess
}

/// Validate a set of sector weights sum to ~1.0 within tolerance.
fn validate_weights(
    sectors: &[SectorWeight],
    which: &str,
    warnings: &mut Vec<String>,
) -> CorpFinanceResult<()> {
    let sum: Decimal = match which {
        "portfolio" => sectors.iter().map(|s| s.portfolio_weight).sum(),
        _ => sectors.iter().map(|s| s.benchmark_weight).sum(),
    };
    let diff = (sum - Decimal::ONE).abs();
    if diff > dec!(0.02) {
        return Err(CorpFinanceError::InvalidInput {
            field: format!("{}_weights", which),
            reason: format!(
                "{} weights sum to {} (must be within 0.02 of 1.0)",
                which, sum
            ),
        });
    }

    // Check for negative weights (short positions)
    for s in sectors {
        let w = if which == "portfolio" {
            s.portfolio_weight
        } else {
            s.benchmark_weight
        };
        if w < Decimal::ZERO {
            warnings.push(format!(
                "Negative {} weight in sector '{}': {} (short position)",
                which, s.sector, w
            ));
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Single-period Brinson-Fachler computation
// ---------------------------------------------------------------------------

struct SinglePeriodResult {
    portfolio_return: Decimal,
    benchmark_return: Decimal,
    active_return: Decimal,
    total_allocation: Decimal,
    total_selection: Decimal,
    total_interaction: Decimal,
    sector_attribution: Vec<SectorAttribution>,
}

fn compute_single_period(sectors: &[SectorWeight]) -> SinglePeriodResult {
    // Portfolio and benchmark total returns
    let portfolio_return: Decimal = sectors
        .iter()
        .map(|s| s.portfolio_weight * s.portfolio_return)
        .sum();
    let benchmark_return: Decimal = sectors
        .iter()
        .map(|s| s.benchmark_weight * s.benchmark_return)
        .sum();

    let mut total_allocation = Decimal::ZERO;
    let mut total_selection = Decimal::ZERO;
    let mut total_interaction = Decimal::ZERO;
    let mut sector_attribution = Vec::with_capacity(sectors.len());

    for s in sectors {
        let allocation_effect =
            (s.portfolio_weight - s.benchmark_weight) * (s.benchmark_return - benchmark_return);
        let selection_effect = s.benchmark_weight * (s.portfolio_return - s.benchmark_return);
        let interaction_effect =
            (s.portfolio_weight - s.benchmark_weight) * (s.portfolio_return - s.benchmark_return);
        let total_effect = allocation_effect + selection_effect + interaction_effect;
        let portfolio_contribution = s.portfolio_weight * s.portfolio_return;
        let benchmark_contribution = s.benchmark_weight * s.benchmark_return;

        total_allocation += allocation_effect;
        total_selection += selection_effect;
        total_interaction += interaction_effect;

        sector_attribution.push(SectorAttribution {
            sector: s.sector.clone(),
            allocation_effect,
            selection_effect,
            interaction_effect,
            total_effect,
            portfolio_contribution,
            benchmark_contribution,
        });
    }

    SinglePeriodResult {
        portfolio_return,
        benchmark_return,
        active_return: portfolio_return - benchmark_return,
        total_allocation,
        total_selection,
        total_interaction,
        sector_attribution,
    }
}

// ---------------------------------------------------------------------------
// Multi-period linking (Carino method)
// ---------------------------------------------------------------------------

fn link_multi_period(periods: &[PeriodData]) -> Vec<PeriodAttribution> {
    let mut period_results: Vec<PeriodAttribution> = Vec::with_capacity(periods.len());

    // Compute single-period results for each period
    let mut sp_results: Vec<SinglePeriodResult> = Vec::with_capacity(periods.len());
    for p in periods {
        let sp = compute_single_period(&p.sectors);
        period_results.push(PeriodAttribution {
            period_name: p.period_name.clone(),
            portfolio_return: sp.portfolio_return,
            benchmark_return: sp.benchmark_return,
            active_return: sp.active_return,
            allocation: sp.total_allocation,
            selection: sp.total_selection,
            interaction: sp.total_interaction,
        });
        sp_results.push(sp);
    }

    // Compute Carino linking coefficients
    let mut linking_coefficients: Vec<Decimal> = Vec::with_capacity(periods.len());
    for sp in &sp_results {
        let k = if sp.portfolio_return != Decimal::ZERO {
            decimal_ln(Decimal::ONE + sp.portfolio_return) / sp.portfolio_return
        } else {
            Decimal::ONE
        };
        linking_coefficients.push(k);
    }

    // Total compounded portfolio return
    let mut total_compound = Decimal::ONE;
    for sp in &sp_results {
        total_compound *= Decimal::ONE + sp.portfolio_return;
    }
    let total_return = total_compound - Decimal::ONE;

    // Sum of k_t * R_p_t
    let sum_k_rp: Decimal = linking_coefficients
        .iter()
        .zip(sp_results.iter())
        .map(|(k, sp)| *k * sp.portfolio_return)
        .sum();

    // Adjustment factor
    let adj = if sum_k_rp != Decimal::ZERO {
        total_return / sum_k_rp
    } else {
        Decimal::ONE
    };

    // Apply linking to each period's attribution
    for (i, pa) in period_results.iter_mut().enumerate() {
        let k = linking_coefficients[i];
        pa.allocation = k * pa.allocation * adj;
        pa.selection = k * pa.selection * adj;
        pa.interaction = k * pa.interaction * adj;
    }

    period_results
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Perform Brinson-Fachler performance attribution.
///
/// Single-period attribution decomposes the active return into allocation,
/// selection, and interaction effects. Multi-period linking uses the Carino
/// method when `periods` is provided.
pub fn brinson_attribution(input: &BrinsonInput) -> CorpFinanceResult<BrinsonOutput> {
    let mut warnings = Vec::new();

    // Validate
    if input.sectors.is_empty() {
        return Err(CorpFinanceError::InvalidInput {
            field: "sectors".into(),
            reason: "At least one sector is required".into(),
        });
    }
    validate_weights(&input.sectors, "portfolio", &mut warnings)?;
    validate_weights(&input.sectors, "benchmark", &mut warnings)?;

    // Compute single-period
    let sp = compute_single_period(&input.sectors);

    // Multi-period linking
    let (multi_period_linked, information_ratio) = if let Some(ref periods) = input.periods {
        if periods.is_empty() {
            (None, None)
        } else {
            // Validate each period's weights
            for p in periods {
                if p.sectors.is_empty() {
                    return Err(CorpFinanceError::InvalidInput {
                        field: "periods.sectors".into(),
                        reason: format!("Period '{}' must have at least one sector", p.period_name),
                    });
                }
                validate_weights(&p.sectors, "portfolio", &mut warnings)?;
                validate_weights(&p.sectors, "benchmark", &mut warnings)?;
            }

            let linked = link_multi_period(periods);

            // Information ratio = mean(active) / std(active)
            let n = Decimal::from(linked.len() as u32);
            let active_returns: Vec<Decimal> = linked.iter().map(|pa| pa.active_return).collect();
            let mean_active: Decimal = active_returns.iter().copied().sum::<Decimal>() / n;

            let variance: Decimal = active_returns
                .iter()
                .map(|r| {
                    let diff = *r - mean_active;
                    diff * diff
                })
                .sum::<Decimal>()
                / n;

            let std_active = decimal_sqrt(variance);
            let ir = if std_active > Decimal::ZERO {
                Some(mean_active / std_active)
            } else {
                None
            };

            (Some(linked), ir)
        }
    } else {
        (None, None)
    };

    let mut assumptions = HashMap::new();
    assumptions.insert("model".into(), "Brinson-Fachler".into());
    assumptions.insert("linking_method".into(), "Carino".into());
    assumptions.insert("weight_tolerance".into(), "0.02".into());

    Ok(BrinsonOutput {
        portfolio_return: sp.portfolio_return,
        benchmark_return: sp.benchmark_return,
        active_return: sp.active_return,
        total_allocation: sp.total_allocation,
        total_selection: sp.total_selection,
        total_interaction: sp.total_interaction,
        sector_attribution: sp.sector_attribution,
        information_ratio,
        multi_period_linked,
        methodology: "Brinson-Fachler single-period attribution with Carino multi-period linking"
            .into(),
        assumptions,
        warnings,
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn make_sector(name: &str, pw: Decimal, bw: Decimal, pr: Decimal, br: Decimal) -> SectorWeight {
        SectorWeight {
            sector: name.into(),
            portfolio_weight: pw,
            benchmark_weight: bw,
            portfolio_return: pr,
            benchmark_return: br,
        }
    }

    fn basic_3_sector_input() -> BrinsonInput {
        BrinsonInput {
            portfolio_name: "Test Portfolio".into(),
            benchmark_name: "Test Benchmark".into(),
            sectors: vec![
                make_sector("Equity", dec!(0.60), dec!(0.50), dec!(0.10), dec!(0.08)),
                make_sector("Bonds", dec!(0.30), dec!(0.40), dec!(0.04), dec!(0.05)),
                make_sector("Cash", dec!(0.10), dec!(0.10), dec!(0.02), dec!(0.02)),
            ],
            risk_free_rate: dec!(0.02),
            periods: None,
        }
    }

    // ---- Basic 3-sector attribution ----

    #[test]
    fn test_basic_3_sector_returns() {
        let out = brinson_attribution(&basic_3_sector_input()).unwrap();
        // R_p = 0.60*0.10 + 0.30*0.04 + 0.10*0.02 = 0.060 + 0.012 + 0.002 = 0.074
        assert_eq!(out.portfolio_return, dec!(0.074));
        // R_b = 0.50*0.08 + 0.40*0.05 + 0.10*0.02 = 0.040 + 0.020 + 0.002 = 0.062
        assert_eq!(out.benchmark_return, dec!(0.062));
        assert_eq!(out.active_return, dec!(0.012));
    }

    #[test]
    fn test_basic_3_sector_sum_identity() {
        let out = brinson_attribution(&basic_3_sector_input()).unwrap();
        let sum = out.total_allocation + out.total_selection + out.total_interaction;
        assert_eq!(sum, out.active_return);
    }

    #[test]
    fn test_sector_effects_sum_to_totals() {
        let out = brinson_attribution(&basic_3_sector_input()).unwrap();
        let alloc_sum: Decimal = out
            .sector_attribution
            .iter()
            .map(|s| s.allocation_effect)
            .sum();
        let sel_sum: Decimal = out
            .sector_attribution
            .iter()
            .map(|s| s.selection_effect)
            .sum();
        let inter_sum: Decimal = out
            .sector_attribution
            .iter()
            .map(|s| s.interaction_effect)
            .sum();
        assert_eq!(alloc_sum, out.total_allocation);
        assert_eq!(sel_sum, out.total_selection);
        assert_eq!(inter_sum, out.total_interaction);
    }

    #[test]
    fn test_sector_total_effect_equals_component_sum() {
        let out = brinson_attribution(&basic_3_sector_input()).unwrap();
        for sa in &out.sector_attribution {
            assert_eq!(
                sa.total_effect,
                sa.allocation_effect + sa.selection_effect + sa.interaction_effect
            );
        }
    }

    #[test]
    fn test_portfolio_contribution() {
        let out = brinson_attribution(&basic_3_sector_input()).unwrap();
        let eq = &out.sector_attribution[0];
        assert_eq!(eq.portfolio_contribution, dec!(0.060));
        let bo = &out.sector_attribution[1];
        assert_eq!(bo.portfolio_contribution, dec!(0.012));
    }

    #[test]
    fn test_benchmark_contribution() {
        let out = brinson_attribution(&basic_3_sector_input()).unwrap();
        let eq = &out.sector_attribution[0];
        assert_eq!(eq.benchmark_contribution, dec!(0.040));
    }

    // ---- Zero active return ----

    #[test]
    fn test_zero_active_return() {
        let input = BrinsonInput {
            portfolio_name: "Same".into(),
            benchmark_name: "Same".into(),
            sectors: vec![
                make_sector("A", dec!(0.50), dec!(0.50), dec!(0.10), dec!(0.10)),
                make_sector("B", dec!(0.50), dec!(0.50), dec!(0.05), dec!(0.05)),
            ],
            risk_free_rate: dec!(0.02),
            periods: None,
        };
        let out = brinson_attribution(&input).unwrap();
        assert_eq!(out.active_return, Decimal::ZERO);
        assert_eq!(out.total_allocation, Decimal::ZERO);
        assert_eq!(out.total_selection, Decimal::ZERO);
        assert_eq!(out.total_interaction, Decimal::ZERO);
    }

    // ---- Single sector ----

    #[test]
    fn test_single_sector() {
        let input = BrinsonInput {
            portfolio_name: "P".into(),
            benchmark_name: "B".into(),
            sectors: vec![make_sector(
                "All",
                dec!(1.0),
                dec!(1.0),
                dec!(0.12),
                dec!(0.10),
            )],
            risk_free_rate: dec!(0.02),
            periods: None,
        };
        let out = brinson_attribution(&input).unwrap();
        assert_eq!(out.portfolio_return, dec!(0.12));
        assert_eq!(out.benchmark_return, dec!(0.10));
        assert_eq!(out.active_return, dec!(0.02));
        // With same weights, allocation = 0, selection = 1.0*(0.12-0.10) = 0.02
        assert_eq!(out.total_allocation, Decimal::ZERO);
        assert_eq!(out.total_selection, dec!(0.02));
        assert_eq!(out.total_interaction, Decimal::ZERO);
    }

    // ---- Many sectors (10+) ----

    #[test]
    fn test_many_sectors() {
        let mut sectors = Vec::new();
        for i in 0..10 {
            sectors.push(make_sector(
                &format!("S{}", i),
                dec!(0.10),
                dec!(0.10),
                Decimal::from(i + 1) / dec!(100),
                Decimal::from(i + 2) / dec!(100),
            ));
        }
        let input = BrinsonInput {
            portfolio_name: "Wide".into(),
            benchmark_name: "Bench".into(),
            sectors,
            risk_free_rate: dec!(0.01),
            periods: None,
        };
        let out = brinson_attribution(&input).unwrap();
        let sum = out.total_allocation + out.total_selection + out.total_interaction;
        assert_eq!(sum, out.active_return);
    }

    // ---- Negative returns ----

    #[test]
    fn test_negative_returns() {
        let input = BrinsonInput {
            portfolio_name: "P".into(),
            benchmark_name: "B".into(),
            sectors: vec![
                make_sector("A", dec!(0.60), dec!(0.50), dec!(-0.05), dec!(-0.03)),
                make_sector("B", dec!(0.40), dec!(0.50), dec!(0.02), dec!(0.04)),
            ],
            risk_free_rate: dec!(0.01),
            periods: None,
        };
        let out = brinson_attribution(&input).unwrap();
        let sum = out.total_allocation + out.total_selection + out.total_interaction;
        assert_eq!(sum, out.active_return);
    }

    // ---- All-cash benchmark ----

    #[test]
    fn test_all_cash_benchmark() {
        let input = BrinsonInput {
            portfolio_name: "P".into(),
            benchmark_name: "Cash".into(),
            sectors: vec![
                make_sector("Eq", dec!(0.80), dec!(0.0), dec!(0.15), dec!(0.0)),
                make_sector("Cash", dec!(0.20), dec!(1.0), dec!(0.02), dec!(0.02)),
            ],
            risk_free_rate: dec!(0.02),
            periods: None,
        };
        let out = brinson_attribution(&input).unwrap();
        let sum = out.total_allocation + out.total_selection + out.total_interaction;
        assert_eq!(sum, out.active_return);
    }

    // ---- Weight validation ----

    #[test]
    fn test_portfolio_weights_not_summing_to_1() {
        let input = BrinsonInput {
            portfolio_name: "P".into(),
            benchmark_name: "B".into(),
            sectors: vec![
                make_sector("A", dec!(0.30), dec!(0.50), dec!(0.10), dec!(0.08)),
                make_sector("B", dec!(0.30), dec!(0.50), dec!(0.05), dec!(0.04)),
            ],
            risk_free_rate: dec!(0.02),
            periods: None,
        };
        let result = brinson_attribution(&input);
        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        assert!(err_msg.contains("portfolio"));
    }

    #[test]
    fn test_benchmark_weights_not_summing_to_1() {
        let input = BrinsonInput {
            portfolio_name: "P".into(),
            benchmark_name: "B".into(),
            sectors: vec![
                make_sector("A", dec!(0.50), dec!(0.30), dec!(0.10), dec!(0.08)),
                make_sector("B", dec!(0.50), dec!(0.30), dec!(0.05), dec!(0.04)),
            ],
            risk_free_rate: dec!(0.02),
            periods: None,
        };
        let result = brinson_attribution(&input);
        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        assert!(err_msg.contains("benchmark"));
    }

    #[test]
    fn test_weight_tolerance_pass() {
        // Weights sum to 1.01 -- within 0.02 tolerance
        let input = BrinsonInput {
            portfolio_name: "P".into(),
            benchmark_name: "B".into(),
            sectors: vec![
                make_sector("A", dec!(0.51), dec!(0.50), dec!(0.10), dec!(0.08)),
                make_sector("B", dec!(0.50), dec!(0.50), dec!(0.05), dec!(0.04)),
            ],
            risk_free_rate: dec!(0.02),
            periods: None,
        };
        assert!(brinson_attribution(&input).is_ok());
    }

    #[test]
    fn test_empty_sectors_error() {
        let input = BrinsonInput {
            portfolio_name: "P".into(),
            benchmark_name: "B".into(),
            sectors: vec![],
            risk_free_rate: dec!(0.02),
            periods: None,
        };
        assert!(brinson_attribution(&input).is_err());
    }

    // ---- Negative weight warning ----

    #[test]
    fn test_negative_weight_warns() {
        let input = BrinsonInput {
            portfolio_name: "P".into(),
            benchmark_name: "B".into(),
            sectors: vec![
                make_sector("Long", dec!(1.20), dec!(1.0), dec!(0.10), dec!(0.08)),
                make_sector("Short", dec!(-0.20), dec!(0.0), dec!(0.05), dec!(0.02)),
            ],
            risk_free_rate: dec!(0.02),
            periods: None,
        };
        let out = brinson_attribution(&input).unwrap();
        assert!(out.warnings.iter().any(|w| w.contains("short position")));
    }

    // ---- Identical portfolios ----

    #[test]
    fn test_identical_portfolios() {
        let input = BrinsonInput {
            portfolio_name: "P".into(),
            benchmark_name: "B".into(),
            sectors: vec![
                make_sector("A", dec!(0.60), dec!(0.60), dec!(0.08), dec!(0.08)),
                make_sector("B", dec!(0.40), dec!(0.40), dec!(0.05), dec!(0.05)),
            ],
            risk_free_rate: dec!(0.02),
            periods: None,
        };
        let out = brinson_attribution(&input).unwrap();
        assert_eq!(out.active_return, Decimal::ZERO);
        for sa in &out.sector_attribution {
            assert_eq!(sa.allocation_effect, Decimal::ZERO);
            assert_eq!(sa.selection_effect, Decimal::ZERO);
            assert_eq!(sa.interaction_effect, Decimal::ZERO);
        }
    }

    // ---- 100% allocation to one sector ----

    #[test]
    fn test_concentrated_allocation() {
        let input = BrinsonInput {
            portfolio_name: "P".into(),
            benchmark_name: "B".into(),
            sectors: vec![
                make_sector("Hot", dec!(1.0), dec!(0.50), dec!(0.15), dec!(0.12)),
                make_sector("Cold", dec!(0.0), dec!(0.50), dec!(0.03), dec!(0.04)),
            ],
            risk_free_rate: dec!(0.02),
            periods: None,
        };
        let out = brinson_attribution(&input).unwrap();
        let sum = out.total_allocation + out.total_selection + out.total_interaction;
        assert_eq!(sum, out.active_return);
    }

    // ---- Large return differentials ----

    #[test]
    fn test_large_return_differentials() {
        let input = BrinsonInput {
            portfolio_name: "P".into(),
            benchmark_name: "B".into(),
            sectors: vec![
                make_sector("Growth", dec!(0.70), dec!(0.50), dec!(0.50), dec!(0.05)),
                make_sector("Value", dec!(0.30), dec!(0.50), dec!(-0.20), dec!(0.03)),
            ],
            risk_free_rate: dec!(0.02),
            periods: None,
        };
        let out = brinson_attribution(&input).unwrap();
        let sum = out.total_allocation + out.total_selection + out.total_interaction;
        assert_eq!(sum, out.active_return);
    }

    // ---- Sign of allocation effect ----

    #[test]
    fn test_allocation_effect_overweight_outperformer_positive() {
        // Overweight sector that beats benchmark => positive allocation
        let input = BrinsonInput {
            portfolio_name: "P".into(),
            benchmark_name: "B".into(),
            sectors: vec![
                make_sector("Winner", dec!(0.80), dec!(0.50), dec!(0.12), dec!(0.12)),
                make_sector("Loser", dec!(0.20), dec!(0.50), dec!(0.02), dec!(0.02)),
            ],
            risk_free_rate: dec!(0.02),
            periods: None,
        };
        let out = brinson_attribution(&input).unwrap();
        // R_b = 0.50*0.12 + 0.50*0.02 = 0.07
        // Winner allocation = (0.80 - 0.50) * (0.12 - 0.07) = 0.30 * 0.05 = 0.015
        let winner = &out.sector_attribution[0];
        assert!(winner.allocation_effect > Decimal::ZERO);
    }

    #[test]
    fn test_allocation_effect_overweight_underperformer_negative() {
        // Overweight sector that trails benchmark => negative allocation
        let input = BrinsonInput {
            portfolio_name: "P".into(),
            benchmark_name: "B".into(),
            sectors: vec![
                make_sector("Laggard", dec!(0.80), dec!(0.50), dec!(0.02), dec!(0.02)),
                make_sector("Star", dec!(0.20), dec!(0.50), dec!(0.12), dec!(0.12)),
            ],
            risk_free_rate: dec!(0.02),
            periods: None,
        };
        let out = brinson_attribution(&input).unwrap();
        // R_b = 0.50*0.02 + 0.50*0.12 = 0.07
        // Laggard allocation = (0.80-0.50)*(0.02-0.07) = 0.30*(-0.05) = -0.015
        let laggard = &out.sector_attribution[0];
        assert!(laggard.allocation_effect < Decimal::ZERO);
    }

    // ---- Sign of selection effect ----

    #[test]
    fn test_selection_effect_outperform_positive() {
        let input = BrinsonInput {
            portfolio_name: "P".into(),
            benchmark_name: "B".into(),
            sectors: vec![
                make_sector("A", dec!(0.50), dec!(0.50), dec!(0.12), dec!(0.08)),
                make_sector("B", dec!(0.50), dec!(0.50), dec!(0.05), dec!(0.05)),
            ],
            risk_free_rate: dec!(0.02),
            periods: None,
        };
        let out = brinson_attribution(&input).unwrap();
        let a = &out.sector_attribution[0];
        // selection = 0.50 * (0.12 - 0.08) = 0.02 > 0
        assert!(a.selection_effect > Decimal::ZERO);
    }

    #[test]
    fn test_selection_effect_underperform_negative() {
        let input = BrinsonInput {
            portfolio_name: "P".into(),
            benchmark_name: "B".into(),
            sectors: vec![
                make_sector("A", dec!(0.50), dec!(0.50), dec!(0.04), dec!(0.08)),
                make_sector("B", dec!(0.50), dec!(0.50), dec!(0.05), dec!(0.05)),
            ],
            risk_free_rate: dec!(0.02),
            periods: None,
        };
        let out = brinson_attribution(&input).unwrap();
        let a = &out.sector_attribution[0];
        // selection = 0.50 * (0.04 - 0.08) = -0.02 < 0
        assert!(a.selection_effect < Decimal::ZERO);
    }

    // ---- Zero weights ----

    #[test]
    fn test_zero_portfolio_weight_sector() {
        let input = BrinsonInput {
            portfolio_name: "P".into(),
            benchmark_name: "B".into(),
            sectors: vec![
                make_sector("Held", dec!(1.0), dec!(0.50), dec!(0.10), dec!(0.08)),
                make_sector("NotHeld", dec!(0.0), dec!(0.50), dec!(0.05), dec!(0.06)),
            ],
            risk_free_rate: dec!(0.02),
            periods: None,
        };
        let out = brinson_attribution(&input).unwrap();
        let sum = out.total_allocation + out.total_selection + out.total_interaction;
        assert_eq!(sum, out.active_return);
    }

    #[test]
    fn test_zero_benchmark_weight_sector() {
        let input = BrinsonInput {
            portfolio_name: "P".into(),
            benchmark_name: "B".into(),
            sectors: vec![
                make_sector("InBench", dec!(0.50), dec!(1.0), dec!(0.10), dec!(0.08)),
                make_sector("NotInBench", dec!(0.50), dec!(0.0), dec!(0.05), dec!(0.03)),
            ],
            risk_free_rate: dec!(0.02),
            periods: None,
        };
        let out = brinson_attribution(&input).unwrap();
        let sum = out.total_allocation + out.total_selection + out.total_interaction;
        assert_eq!(sum, out.active_return);
    }

    // ---- Multi-period linking ----

    #[test]
    fn test_multi_period_two_periods() {
        let input = BrinsonInput {
            portfolio_name: "P".into(),
            benchmark_name: "B".into(),
            sectors: vec![
                make_sector("A", dec!(0.60), dec!(0.50), dec!(0.10), dec!(0.08)),
                make_sector("B", dec!(0.40), dec!(0.50), dec!(0.04), dec!(0.05)),
            ],
            risk_free_rate: dec!(0.02),
            periods: Some(vec![
                PeriodData {
                    period_name: "Q1".into(),
                    sectors: vec![
                        make_sector("A", dec!(0.60), dec!(0.50), dec!(0.05), dec!(0.04)),
                        make_sector("B", dec!(0.40), dec!(0.50), dec!(0.02), dec!(0.03)),
                    ],
                },
                PeriodData {
                    period_name: "Q2".into(),
                    sectors: vec![
                        make_sector("A", dec!(0.55), dec!(0.50), dec!(0.06), dec!(0.05)),
                        make_sector("B", dec!(0.45), dec!(0.50), dec!(0.03), dec!(0.02)),
                    ],
                },
            ]),
        };
        let out = brinson_attribution(&input).unwrap();
        let linked = out.multi_period_linked.as_ref().unwrap();
        assert_eq!(linked.len(), 2);
        assert_eq!(linked[0].period_name, "Q1");
        assert_eq!(linked[1].period_name, "Q2");
    }

    #[test]
    fn test_multi_period_three_periods() {
        let mk_period = |name: &str, pr: Decimal, br: Decimal| PeriodData {
            period_name: name.into(),
            sectors: vec![
                make_sector("A", dec!(0.60), dec!(0.50), pr, br),
                make_sector("B", dec!(0.40), dec!(0.50), pr / dec!(2), br / dec!(2)),
            ],
        };
        let input = BrinsonInput {
            portfolio_name: "P".into(),
            benchmark_name: "B".into(),
            sectors: vec![
                make_sector("A", dec!(0.60), dec!(0.50), dec!(0.10), dec!(0.08)),
                make_sector("B", dec!(0.40), dec!(0.50), dec!(0.05), dec!(0.04)),
            ],
            risk_free_rate: dec!(0.02),
            periods: Some(vec![
                mk_period("Q1", dec!(0.03), dec!(0.02)),
                mk_period("Q2", dec!(0.04), dec!(0.03)),
                mk_period("Q3", dec!(0.05), dec!(0.04)),
            ]),
        };
        let out = brinson_attribution(&input).unwrap();
        let linked = out.multi_period_linked.as_ref().unwrap();
        assert_eq!(linked.len(), 3);
    }

    #[test]
    fn test_multi_period_linked_effects_have_values() {
        let input = BrinsonInput {
            portfolio_name: "P".into(),
            benchmark_name: "B".into(),
            sectors: vec![
                make_sector("A", dec!(0.60), dec!(0.50), dec!(0.10), dec!(0.08)),
                make_sector("B", dec!(0.40), dec!(0.50), dec!(0.04), dec!(0.05)),
            ],
            risk_free_rate: dec!(0.02),
            periods: Some(vec![
                PeriodData {
                    period_name: "H1".into(),
                    sectors: vec![
                        make_sector("A", dec!(0.60), dec!(0.50), dec!(0.08), dec!(0.06)),
                        make_sector("B", dec!(0.40), dec!(0.50), dec!(0.03), dec!(0.04)),
                    ],
                },
                PeriodData {
                    period_name: "H2".into(),
                    sectors: vec![
                        make_sector("A", dec!(0.65), dec!(0.50), dec!(0.10), dec!(0.07)),
                        make_sector("B", dec!(0.35), dec!(0.50), dec!(0.02), dec!(0.03)),
                    ],
                },
            ]),
        };
        let out = brinson_attribution(&input).unwrap();
        let linked = out.multi_period_linked.unwrap();
        // Linked effects should be non-zero when there are active returns
        let total_linked_alloc: Decimal = linked.iter().map(|p| p.allocation).sum();
        let total_linked_sel: Decimal = linked.iter().map(|p| p.selection).sum();
        // At least one of allocation or selection should be non-zero
        assert!(
            total_linked_alloc != Decimal::ZERO || total_linked_sel != Decimal::ZERO,
            "Expected non-zero linked attribution effects"
        );
    }

    // ---- Information ratio ----

    #[test]
    fn test_information_ratio_computed() {
        let input = BrinsonInput {
            portfolio_name: "P".into(),
            benchmark_name: "B".into(),
            sectors: vec![
                make_sector("A", dec!(0.60), dec!(0.50), dec!(0.10), dec!(0.08)),
                make_sector("B", dec!(0.40), dec!(0.50), dec!(0.04), dec!(0.05)),
            ],
            risk_free_rate: dec!(0.02),
            periods: Some(vec![
                PeriodData {
                    period_name: "Q1".into(),
                    sectors: vec![
                        make_sector("A", dec!(0.60), dec!(0.50), dec!(0.05), dec!(0.04)),
                        make_sector("B", dec!(0.40), dec!(0.50), dec!(0.02), dec!(0.03)),
                    ],
                },
                PeriodData {
                    period_name: "Q2".into(),
                    sectors: vec![
                        make_sector("A", dec!(0.55), dec!(0.50), dec!(0.06), dec!(0.03)),
                        make_sector("B", dec!(0.45), dec!(0.50), dec!(0.03), dec!(0.02)),
                    ],
                },
            ]),
        };
        let out = brinson_attribution(&input).unwrap();
        assert!(out.information_ratio.is_some());
        // IR should be positive (portfolio outperforms in both periods)
        assert!(out.information_ratio.unwrap() > Decimal::ZERO);
    }

    #[test]
    fn test_information_ratio_none_without_periods() {
        let out = brinson_attribution(&basic_3_sector_input()).unwrap();
        assert!(out.information_ratio.is_none());
    }

    #[test]
    fn test_information_ratio_zero_std_returns_none() {
        // Same active return every period => std=0 => IR is None
        let input = BrinsonInput {
            portfolio_name: "P".into(),
            benchmark_name: "B".into(),
            sectors: vec![
                make_sector("A", dec!(0.50), dec!(0.50), dec!(0.10), dec!(0.08)),
                make_sector("B", dec!(0.50), dec!(0.50), dec!(0.06), dec!(0.04)),
            ],
            risk_free_rate: dec!(0.02),
            periods: Some(vec![
                PeriodData {
                    period_name: "Q1".into(),
                    sectors: vec![
                        make_sector("A", dec!(0.50), dec!(0.50), dec!(0.10), dec!(0.08)),
                        make_sector("B", dec!(0.50), dec!(0.50), dec!(0.06), dec!(0.04)),
                    ],
                },
                PeriodData {
                    period_name: "Q2".into(),
                    sectors: vec![
                        make_sector("A", dec!(0.50), dec!(0.50), dec!(0.10), dec!(0.08)),
                        make_sector("B", dec!(0.50), dec!(0.50), dec!(0.06), dec!(0.04)),
                    ],
                },
            ]),
        };
        let out = brinson_attribution(&input).unwrap();
        // All periods have the same active return => std = 0 => IR is None
        assert!(out.information_ratio.is_none());
    }

    // ---- Edge cases ----

    #[test]
    fn test_methodology_string() {
        let out = brinson_attribution(&basic_3_sector_input()).unwrap();
        assert!(out.methodology.contains("Brinson-Fachler"));
    }

    #[test]
    fn test_assumptions_present() {
        let out = brinson_attribution(&basic_3_sector_input()).unwrap();
        assert!(out.assumptions.contains_key("model"));
        assert!(out.assumptions.contains_key("linking_method"));
    }

    #[test]
    fn test_all_sectors_same_return() {
        let input = BrinsonInput {
            portfolio_name: "P".into(),
            benchmark_name: "B".into(),
            sectors: vec![
                make_sector("A", dec!(0.60), dec!(0.50), dec!(0.07), dec!(0.07)),
                make_sector("B", dec!(0.40), dec!(0.50), dec!(0.07), dec!(0.07)),
            ],
            risk_free_rate: dec!(0.02),
            periods: None,
        };
        let out = brinson_attribution(&input).unwrap();
        // All returns identical => all effects zero
        assert_eq!(out.active_return, Decimal::ZERO);
        assert_eq!(out.total_allocation, Decimal::ZERO);
        assert_eq!(out.total_selection, Decimal::ZERO);
        assert_eq!(out.total_interaction, Decimal::ZERO);
    }

    #[test]
    fn test_two_sectors_only_allocation() {
        // Same returns in both portfolio and benchmark per sector,
        // but different weights => only allocation effect
        let input = BrinsonInput {
            portfolio_name: "P".into(),
            benchmark_name: "B".into(),
            sectors: vec![
                make_sector("High", dec!(0.70), dec!(0.50), dec!(0.10), dec!(0.10)),
                make_sector("Low", dec!(0.30), dec!(0.50), dec!(0.04), dec!(0.04)),
            ],
            risk_free_rate: dec!(0.02),
            periods: None,
        };
        let out = brinson_attribution(&input).unwrap();
        assert_eq!(out.total_selection, Decimal::ZERO);
        assert_eq!(out.total_interaction, Decimal::ZERO);
        // Active return = allocation only
        assert_eq!(out.active_return, out.total_allocation);
    }

    #[test]
    fn test_two_sectors_only_selection() {
        // Same weights, different returns => only selection effect
        let input = BrinsonInput {
            portfolio_name: "P".into(),
            benchmark_name: "B".into(),
            sectors: vec![
                make_sector("A", dec!(0.50), dec!(0.50), dec!(0.12), dec!(0.08)),
                make_sector("B", dec!(0.50), dec!(0.50), dec!(0.06), dec!(0.04)),
            ],
            risk_free_rate: dec!(0.02),
            periods: None,
        };
        let out = brinson_attribution(&input).unwrap();
        assert_eq!(out.total_allocation, Decimal::ZERO);
        assert_eq!(out.total_interaction, Decimal::ZERO);
        assert_eq!(out.active_return, out.total_selection);
    }

    #[test]
    fn test_very_small_weights() {
        let input = BrinsonInput {
            portfolio_name: "P".into(),
            benchmark_name: "B".into(),
            sectors: vec![
                make_sector("Main", dec!(0.999), dec!(0.998), dec!(0.10), dec!(0.08)),
                make_sector("Tiny", dec!(0.001), dec!(0.002), dec!(0.05), dec!(0.03)),
            ],
            risk_free_rate: dec!(0.02),
            periods: None,
        };
        let out = brinson_attribution(&input).unwrap();
        let sum = out.total_allocation + out.total_selection + out.total_interaction;
        assert_eq!(sum, out.active_return);
    }

    #[test]
    fn test_portfolio_return_matches_sector_contributions() {
        let out = brinson_attribution(&basic_3_sector_input()).unwrap();
        let contrib_sum: Decimal = out
            .sector_attribution
            .iter()
            .map(|s| s.portfolio_contribution)
            .sum();
        assert_eq!(contrib_sum, out.portfolio_return);
    }

    #[test]
    fn test_benchmark_return_matches_sector_contributions() {
        let out = brinson_attribution(&basic_3_sector_input()).unwrap();
        let contrib_sum: Decimal = out
            .sector_attribution
            .iter()
            .map(|s| s.benchmark_contribution)
            .sum();
        assert_eq!(contrib_sum, out.benchmark_return);
    }

    #[test]
    fn test_no_warnings_basic_input() {
        let out = brinson_attribution(&basic_3_sector_input()).unwrap();
        assert!(out.warnings.is_empty());
    }

    #[test]
    fn test_multi_period_empty_periods_vec() {
        let mut input = basic_3_sector_input();
        input.periods = Some(vec![]);
        let out = brinson_attribution(&input).unwrap();
        assert!(out.multi_period_linked.is_none());
        assert!(out.information_ratio.is_none());
    }

    // ---- Decimal precision ----

    #[test]
    fn test_decimal_ln_basic() {
        // ln(1) = 0
        let result = decimal_ln(Decimal::ONE);
        assert_eq!(result, Decimal::ZERO);
    }

    #[test]
    fn test_decimal_sqrt_basic() {
        // sqrt(4) = 2
        let result = decimal_sqrt(Decimal::from(4));
        let diff = (result - Decimal::from(2)).abs();
        assert!(diff < dec!(0.0000001));
    }

    #[test]
    fn test_decimal_sqrt_zero() {
        assert_eq!(decimal_sqrt(Decimal::ZERO), Decimal::ZERO);
    }
}
