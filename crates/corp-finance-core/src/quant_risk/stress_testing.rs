use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::error::CorpFinanceError;
use crate::types::{with_metadata, ComputationOutput};
use crate::CorpFinanceResult;

// Re-use AssetAllocation from sibling module
use super::risk_parity::AssetAllocation;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Classification of asset class for shock sensitivity mapping.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AssetClass {
    Equity,
    FixedIncome,
    Credit,
    Commodity,
    Currency,
    RealEstate,
    Alternative,
}

/// A single position in the portfolio being stress-tested.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioPosition {
    pub name: String,
    pub weight: Decimal,
    pub asset_class: AssetClass,
    /// Equity beta (default 1.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub beta: Option<Decimal>,
    /// Fixed income / credit duration (default 5.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<Decimal>,
    /// Currency code of FX exposure (e.g. "EUR", "JPY")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fx_exposure: Option<String>,
}

/// Whether the scenario is based on a real historical event or hypothetical.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ScenarioType {
    Historical,
    Hypothetical,
}

/// A single market risk factor shock.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketShock {
    /// Factor name: "equity_market", "interest_rates", "credit_spreads",
    /// "fx_usd", "commodities", "volatility"
    pub factor: String,
    /// Shock magnitude as a decimal (e.g. -0.40 for a 40% equity decline)
    pub shock_pct: Decimal,
}

/// A complete stress scenario consisting of one or more market shocks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StressScenario {
    pub name: String,
    pub scenario_type: ScenarioType,
    pub shocks: Vec<MarketShock>,
}

/// Input for the stress-testing engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StressTestInput {
    /// Current portfolio positions
    pub portfolio: Vec<PortfolioPosition>,
    /// Scenarios to evaluate
    pub scenarios: Vec<StressScenario>,
    /// Multiply historical-scenario impacts by 1.2 to account for
    /// crisis correlation spikes (default true)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub correlation_adjustments: Option<bool>,
}

/// Impact on a single position under one scenario.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionImpact {
    pub name: String,
    pub weight: Decimal,
    /// Percentage impact on the position
    pub impact_pct: Decimal,
    /// Contribution to portfolio P&L (weight * impact_pct)
    pub pnl_contribution: Decimal,
}

/// Result for a single stress scenario.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioResult {
    pub scenario_name: String,
    /// Portfolio-level percentage change
    pub portfolio_impact: Decimal,
    /// Per-position breakdown
    pub position_impacts: Vec<PositionImpact>,
    /// Whether the scenario loss exceeds a simple 10% VaR threshold
    pub var_breach: bool,
}

/// High-level portfolio risk summary across all scenarios.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioRiskSummary {
    pub current_weights: Vec<AssetAllocation>,
    pub max_drawdown_scenario: String,
    pub avg_scenario_loss: Decimal,
}

/// Output of the stress-testing engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StressTestOutput {
    pub scenario_results: Vec<ScenarioResult>,
    pub worst_case: ScenarioResult,
    pub portfolio_summary: PortfolioRiskSummary,
}

// ---------------------------------------------------------------------------
// Built-in historical scenarios
// ---------------------------------------------------------------------------

/// Return the five canonical historical stress scenarios.
pub fn get_historical_scenarios() -> Vec<StressScenario> {
    vec![
        // GFC 2008
        StressScenario {
            name: "GFC 2008".into(),
            scenario_type: ScenarioType::Historical,
            shocks: vec![
                MarketShock {
                    factor: "equity_market".into(),
                    shock_pct: dec!(-0.38),
                },
                MarketShock {
                    factor: "interest_rates".into(),
                    shock_pct: dec!(-0.02), // -200 bps
                },
                MarketShock {
                    factor: "credit_spreads".into(),
                    shock_pct: dec!(0.04), // +400 bps
                },
                MarketShock {
                    factor: "commodities".into(),
                    shock_pct: dec!(-0.35),
                },
            ],
        },
        // COVID March 2020
        StressScenario {
            name: "COVID March 2020".into(),
            scenario_type: ScenarioType::Historical,
            shocks: vec![
                MarketShock {
                    factor: "equity_market".into(),
                    shock_pct: dec!(-0.34),
                },
                MarketShock {
                    factor: "interest_rates".into(),
                    shock_pct: dec!(-0.01), // -100 bps
                },
                MarketShock {
                    factor: "credit_spreads".into(),
                    shock_pct: dec!(0.03), // +300 bps
                },
            ],
        },
        // Taper Tantrum 2013
        StressScenario {
            name: "Taper Tantrum 2013".into(),
            scenario_type: ScenarioType::Historical,
            shocks: vec![
                MarketShock {
                    factor: "equity_market".into(),
                    shock_pct: dec!(-0.06),
                },
                MarketShock {
                    factor: "interest_rates".into(),
                    shock_pct: dec!(0.01), // +100 bps
                },
                MarketShock {
                    factor: "credit_spreads".into(),
                    shock_pct: dec!(0.005), // +50 bps
                },
            ],
        },
        // Dot-Com 2000
        StressScenario {
            name: "Dot-Com 2000".into(),
            scenario_type: ScenarioType::Historical,
            shocks: vec![
                MarketShock {
                    factor: "equity_market".into(),
                    shock_pct: dec!(-0.49),
                },
                MarketShock {
                    factor: "interest_rates".into(),
                    shock_pct: dec!(-0.03), // -300 bps
                },
            ],
        },
        // Euro Crisis 2011
        StressScenario {
            name: "Euro Crisis 2011".into(),
            scenario_type: ScenarioType::Historical,
            shocks: vec![
                MarketShock {
                    factor: "equity_market".into(),
                    shock_pct: dec!(-0.22),
                },
                MarketShock {
                    factor: "interest_rates".into(),
                    shock_pct: dec!(0.01), // +100 bps
                },
                MarketShock {
                    factor: "credit_spreads".into(),
                    shock_pct: dec!(0.02), // +200 bps
                },
            ],
        },
    ]
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Run portfolio stress tests across one or more scenarios.
pub fn run_stress_test(
    input: &StressTestInput,
) -> CorpFinanceResult<ComputationOutput<StressTestOutput>> {
    let start = Instant::now();
    let warnings: Vec<String> = Vec::new();

    // -- Validation --
    if input.portfolio.is_empty() {
        return Err(CorpFinanceError::InsufficientData(
            "Portfolio must contain at least one position".into(),
        ));
    }
    if input.scenarios.is_empty() {
        return Err(CorpFinanceError::InsufficientData(
            "At least one stress scenario required".into(),
        ));
    }
    for pos in &input.portfolio {
        if pos.weight < Decimal::ZERO || pos.weight > Decimal::ONE {
            return Err(CorpFinanceError::InvalidInput {
                field: format!("portfolio.{}.weight", pos.name),
                reason: "Weight must be between 0 and 1".into(),
            });
        }
    }

    let use_corr_adj = input.correlation_adjustments.unwrap_or(true);

    // -- Evaluate each scenario --
    let mut scenario_results: Vec<ScenarioResult> = Vec::with_capacity(input.scenarios.len());

    for scenario in &input.scenarios {
        let result = evaluate_scenario(&input.portfolio, scenario, use_corr_adj);
        scenario_results.push(result);
    }

    // -- Find worst case (most negative portfolio_impact) --
    let worst_idx = scenario_results
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| {
            a.portfolio_impact
                .partial_cmp(&b.portfolio_impact)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(i, _)| i)
        .unwrap_or(0);
    let worst_case = scenario_results[worst_idx].clone();

    // -- Portfolio summary --
    let current_weights: Vec<AssetAllocation> = input
        .portfolio
        .iter()
        .map(|p| AssetAllocation {
            name: p.name.clone(),
            weight: p.weight,
        })
        .collect();

    let total_loss: Decimal = scenario_results.iter().map(|r| r.portfolio_impact).sum();
    let avg_scenario_loss = total_loss / Decimal::from(scenario_results.len() as i64);

    let portfolio_summary = PortfolioRiskSummary {
        current_weights,
        max_drawdown_scenario: worst_case.scenario_name.clone(),
        avg_scenario_loss,
    };

    let output = StressTestOutput {
        scenario_results,
        worst_case,
        portfolio_summary,
    };

    let elapsed = start.elapsed().as_micros() as u64;
    Ok(with_metadata(
        "Portfolio Stress Testing",
        &serde_json::json!({
            "num_positions": input.portfolio.len(),
            "num_scenarios": input.scenarios.len(),
            "correlation_adjustments": use_corr_adj,
        }),
        warnings,
        elapsed,
        output,
    ))
}

// ---------------------------------------------------------------------------
// Internal logic
// ---------------------------------------------------------------------------

/// Evaluate a single scenario against the portfolio.
fn evaluate_scenario(
    portfolio: &[PortfolioPosition],
    scenario: &StressScenario,
    use_corr_adj: bool,
) -> ScenarioResult {
    let mut position_impacts: Vec<PositionImpact> = Vec::with_capacity(portfolio.len());
    let mut portfolio_impact = Decimal::ZERO;

    for pos in portfolio {
        let impact_pct = compute_position_impact(pos, &scenario.shocks);
        let pnl_contribution = pos.weight * impact_pct;
        portfolio_impact += pnl_contribution;

        position_impacts.push(PositionImpact {
            name: pos.name.clone(),
            weight: pos.weight,
            impact_pct,
            pnl_contribution,
        });
    }

    // Crisis correlation adjustment for historical scenarios
    if use_corr_adj && scenario.scenario_type == ScenarioType::Historical {
        portfolio_impact *= dec!(1.2);
    }

    // VaR breach: simple 10% threshold
    let var_breach = portfolio_impact < dec!(-0.10);

    ScenarioResult {
        scenario_name: scenario.name.clone(),
        portfolio_impact,
        position_impacts,
        var_breach,
    }
}

/// Compute the impact on a single position from the given shocks.
fn compute_position_impact(pos: &PortfolioPosition, shocks: &[MarketShock]) -> Decimal {
    let equity_shock = find_shock(shocks, "equity_market");
    let rate_shock = find_shock(shocks, "interest_rates");
    let credit_shock = find_shock(shocks, "credit_spreads");
    let commodity_shock = find_shock(shocks, "commodities");
    let fx_shock = find_shock(shocks, "fx_usd");
    let _vol_shock = find_shock(shocks, "volatility");

    match pos.asset_class {
        AssetClass::Equity => {
            let beta = pos.beta.unwrap_or(Decimal::ONE);
            equity_shock * beta
        }
        AssetClass::FixedIncome => {
            let duration = pos.duration.unwrap_or(dec!(5));
            -duration * rate_shock
        }
        AssetClass::Credit => {
            let duration = pos.duration.unwrap_or(dec!(5));
            -credit_shock * duration * dec!(0.5)
        }
        AssetClass::Commodity => commodity_shock,
        AssetClass::Currency => {
            if pos.fx_exposure.is_some() {
                fx_shock
            } else {
                Decimal::ZERO
            }
        }
        AssetClass::RealEstate => {
            let rate_duration = dec!(3);
            equity_shock * dec!(0.6) + rate_shock * (-rate_duration)
        }
        AssetClass::Alternative => equity_shock * dec!(0.4),
    }
}

/// Look up a shock factor by name, returning 0 if not present.
fn find_shock(shocks: &[MarketShock], factor: &str) -> Decimal {
    shocks
        .iter()
        .find(|s| s.factor == factor)
        .map(|s| s.shock_pct)
        .unwrap_or(Decimal::ZERO)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn single_equity_portfolio() -> Vec<PortfolioPosition> {
        vec![PortfolioPosition {
            name: "US Equities".into(),
            weight: Decimal::ONE,
            asset_class: AssetClass::Equity,
            beta: Some(dec!(1.0)),
            duration: None,
            fx_exposure: None,
        }]
    }

    fn diversified_portfolio() -> Vec<PortfolioPosition> {
        vec![
            PortfolioPosition {
                name: "US Equities".into(),
                weight: dec!(0.40),
                asset_class: AssetClass::Equity,
                beta: Some(dec!(1.1)),
                duration: None,
                fx_exposure: None,
            },
            PortfolioPosition {
                name: "US Treasuries".into(),
                weight: dec!(0.30),
                asset_class: AssetClass::FixedIncome,
                beta: None,
                duration: Some(dec!(7)),
                fx_exposure: None,
            },
            PortfolioPosition {
                name: "IG Credit".into(),
                weight: dec!(0.15),
                asset_class: AssetClass::Credit,
                beta: None,
                duration: Some(dec!(5)),
                fx_exposure: None,
            },
            PortfolioPosition {
                name: "Commodities".into(),
                weight: dec!(0.10),
                asset_class: AssetClass::Commodity,
                beta: None,
                duration: None,
                fx_exposure: None,
            },
            PortfolioPosition {
                name: "EUR FX".into(),
                weight: dec!(0.05),
                asset_class: AssetClass::Currency,
                beta: None,
                duration: None,
                fx_exposure: Some("EUR".into()),
            },
        ]
    }

    fn simple_equity_crash() -> StressScenario {
        StressScenario {
            name: "Equity Crash".into(),
            scenario_type: ScenarioType::Hypothetical,
            shocks: vec![MarketShock {
                factor: "equity_market".into(),
                shock_pct: dec!(-0.30),
            }],
        }
    }

    // -- Single equity position --

    #[test]
    fn test_single_equity_crash() {
        let input = StressTestInput {
            portfolio: single_equity_portfolio(),
            scenarios: vec![simple_equity_crash()],
            correlation_adjustments: Some(false),
        };
        let result = run_stress_test(&input).unwrap();
        let sr = &result.result.scenario_results[0];
        // 100% equity * beta 1.0 * -30% = -30%
        assert_eq!(sr.portfolio_impact, dec!(-0.30));
        assert!(sr.var_breach);
    }

    #[test]
    fn test_single_equity_with_beta() {
        let portfolio = vec![PortfolioPosition {
            name: "High Beta".into(),
            weight: Decimal::ONE,
            asset_class: AssetClass::Equity,
            beta: Some(dec!(1.5)),
            duration: None,
            fx_exposure: None,
        }];
        let input = StressTestInput {
            portfolio,
            scenarios: vec![simple_equity_crash()],
            correlation_adjustments: Some(false),
        };
        let result = run_stress_test(&input).unwrap();
        // beta 1.5 * -30% = -45%
        assert_eq!(
            result.result.scenario_results[0].portfolio_impact,
            dec!(-0.45)
        );
    }

    // -- Diversified portfolio --

    #[test]
    fn test_diversified_portfolio_gfc() {
        let scenarios = get_historical_scenarios();
        let gfc = scenarios
            .into_iter()
            .find(|s| s.name == "GFC 2008")
            .unwrap();
        let input = StressTestInput {
            portfolio: diversified_portfolio(),
            scenarios: vec![gfc],
            correlation_adjustments: Some(false),
        };
        let result = run_stress_test(&input).unwrap();
        let sr = &result.result.scenario_results[0];
        assert!(sr.portfolio_impact < Decimal::ZERO);
        // Diversified should suffer less than pure equity
        assert!(sr.portfolio_impact > dec!(-0.38));
    }

    #[test]
    fn test_diversified_portfolio_all_historical() {
        let input = StressTestInput {
            portfolio: diversified_portfolio(),
            scenarios: get_historical_scenarios(),
            correlation_adjustments: Some(true),
        };
        let result = run_stress_test(&input).unwrap();
        assert_eq!(result.result.scenario_results.len(), 5);
        let worst = &result.result.worst_case;
        for sr in &result.result.scenario_results {
            assert!(worst.portfolio_impact <= sr.portfolio_impact);
        }
    }

    // -- Correlation adjustment --

    #[test]
    fn test_correlation_adjustment_multiplier() {
        let hist_scenario = StressScenario {
            name: "Historical Test".into(),
            scenario_type: ScenarioType::Historical,
            shocks: vec![MarketShock {
                factor: "equity_market".into(),
                shock_pct: dec!(-0.20),
            }],
        };
        let input_adj = StressTestInput {
            portfolio: single_equity_portfolio(),
            scenarios: vec![hist_scenario.clone()],
            correlation_adjustments: Some(true),
        };
        let input_no_adj = StressTestInput {
            portfolio: single_equity_portfolio(),
            scenarios: vec![hist_scenario],
            correlation_adjustments: Some(false),
        };
        let adj = run_stress_test(&input_adj).unwrap();
        let no_adj = run_stress_test(&input_no_adj).unwrap();
        let adj_impact = adj.result.scenario_results[0].portfolio_impact;
        let no_adj_impact = no_adj.result.scenario_results[0].portfolio_impact;
        let tolerance = dec!(0.0001);
        assert!(
            (adj_impact - no_adj_impact * dec!(1.2)).abs() < tolerance,
            "Adjusted {} should be 1.2x of unadjusted {}",
            adj_impact,
            no_adj_impact
        );
    }

    #[test]
    fn test_hypothetical_no_correlation_adjustment() {
        let input = StressTestInput {
            portfolio: single_equity_portfolio(),
            scenarios: vec![simple_equity_crash()],
            correlation_adjustments: Some(true),
        };
        let result = run_stress_test(&input).unwrap();
        // Hypothetical scenarios should not get the 1.2x multiplier
        assert_eq!(
            result.result.scenario_results[0].portfolio_impact,
            dec!(-0.30)
        );
    }

    // -- Fixed income & duration --

    #[test]
    fn test_fixed_income_duration_sensitivity() {
        let short_dur = vec![PortfolioPosition {
            name: "Short Duration".into(),
            weight: Decimal::ONE,
            asset_class: AssetClass::FixedIncome,
            beta: None,
            duration: Some(dec!(2)),
            fx_exposure: None,
        }];
        let long_dur = vec![PortfolioPosition {
            name: "Long Duration".into(),
            weight: Decimal::ONE,
            asset_class: AssetClass::FixedIncome,
            beta: None,
            duration: Some(dec!(15)),
            fx_exposure: None,
        }];
        let rate_hike = StressScenario {
            name: "Rate Hike".into(),
            scenario_type: ScenarioType::Hypothetical,
            shocks: vec![MarketShock {
                factor: "interest_rates".into(),
                shock_pct: dec!(0.02), // +200bps
            }],
        };
        let short_input = StressTestInput {
            portfolio: short_dur,
            scenarios: vec![rate_hike.clone()],
            correlation_adjustments: Some(false),
        };
        let long_input = StressTestInput {
            portfolio: long_dur,
            scenarios: vec![rate_hike],
            correlation_adjustments: Some(false),
        };
        let short_result = run_stress_test(&short_input).unwrap();
        let long_result = run_stress_test(&long_input).unwrap();
        // Short duration: impact = -2 * 0.02 = -0.04
        assert_eq!(
            short_result.result.scenario_results[0].portfolio_impact,
            dec!(-0.04)
        );
        // Long duration: impact = -15 * 0.02 = -0.30
        assert_eq!(
            long_result.result.scenario_results[0].portfolio_impact,
            dec!(-0.30)
        );
        assert!(
            long_result.result.scenario_results[0].portfolio_impact
                < short_result.result.scenario_results[0].portfolio_impact
        );
    }

    // -- Credit --

    #[test]
    fn test_credit_spread_widening() {
        let portfolio = vec![PortfolioPosition {
            name: "IG Credit".into(),
            weight: Decimal::ONE,
            asset_class: AssetClass::Credit,
            beta: None,
            duration: Some(dec!(5)),
            fx_exposure: None,
        }];
        let scenario = StressScenario {
            name: "Spread Widening".into(),
            scenario_type: ScenarioType::Hypothetical,
            shocks: vec![MarketShock {
                factor: "credit_spreads".into(),
                shock_pct: dec!(0.03), // +300bps
            }],
        };
        let input = StressTestInput {
            portfolio,
            scenarios: vec![scenario],
            correlation_adjustments: Some(false),
        };
        let result = run_stress_test(&input).unwrap();
        // Impact = -0.03 * 5 * 0.5 = -0.075
        assert_eq!(
            result.result.scenario_results[0].portfolio_impact,
            dec!(-0.075)
        );
    }

    // -- FX exposure --

    #[test]
    fn test_fx_exposure_impact() {
        let portfolio = vec![PortfolioPosition {
            name: "EUR Position".into(),
            weight: Decimal::ONE,
            asset_class: AssetClass::Currency,
            beta: None,
            duration: None,
            fx_exposure: Some("EUR".into()),
        }];
        let scenario = StressScenario {
            name: "USD Strengthening".into(),
            scenario_type: ScenarioType::Hypothetical,
            shocks: vec![MarketShock {
                factor: "fx_usd".into(),
                shock_pct: dec!(-0.15),
            }],
        };
        let input = StressTestInput {
            portfolio,
            scenarios: vec![scenario],
            correlation_adjustments: Some(false),
        };
        let result = run_stress_test(&input).unwrap();
        assert_eq!(
            result.result.scenario_results[0].portfolio_impact,
            dec!(-0.15)
        );
    }

    #[test]
    fn test_no_fx_exposure_no_impact() {
        let portfolio = vec![PortfolioPosition {
            name: "Domestic".into(),
            weight: Decimal::ONE,
            asset_class: AssetClass::Currency,
            beta: None,
            duration: None,
            fx_exposure: None,
        }];
        let scenario = StressScenario {
            name: "FX Shock".into(),
            scenario_type: ScenarioType::Hypothetical,
            shocks: vec![MarketShock {
                factor: "fx_usd".into(),
                shock_pct: dec!(-0.20),
            }],
        };
        let input = StressTestInput {
            portfolio,
            scenarios: vec![scenario],
            correlation_adjustments: Some(false),
        };
        let result = run_stress_test(&input).unwrap();
        assert_eq!(
            result.result.scenario_results[0].portfolio_impact,
            Decimal::ZERO
        );
    }

    // -- Real estate and alternatives --

    #[test]
    fn test_real_estate_dual_sensitivity() {
        let portfolio = vec![PortfolioPosition {
            name: "REITs".into(),
            weight: Decimal::ONE,
            asset_class: AssetClass::RealEstate,
            beta: None,
            duration: None,
            fx_exposure: None,
        }];
        let scenario = StressScenario {
            name: "Combined Shock".into(),
            scenario_type: ScenarioType::Hypothetical,
            shocks: vec![
                MarketShock {
                    factor: "equity_market".into(),
                    shock_pct: dec!(-0.20),
                },
                MarketShock {
                    factor: "interest_rates".into(),
                    shock_pct: dec!(0.01),
                },
            ],
        };
        let input = StressTestInput {
            portfolio,
            scenarios: vec![scenario],
            correlation_adjustments: Some(false),
        };
        let result = run_stress_test(&input).unwrap();
        // RealEstate: equity*0.6 + rates*(-3) = -0.20*0.6 + 0.01*(-3) = -0.15
        let tolerance = dec!(0.0001);
        assert!(
            (result.result.scenario_results[0].portfolio_impact - dec!(-0.15)).abs() < tolerance
        );
    }

    #[test]
    fn test_alternative_partial_equity_sensitivity() {
        let portfolio = vec![PortfolioPosition {
            name: "Hedge Funds".into(),
            weight: Decimal::ONE,
            asset_class: AssetClass::Alternative,
            beta: None,
            duration: None,
            fx_exposure: None,
        }];
        let input = StressTestInput {
            portfolio,
            scenarios: vec![simple_equity_crash()],
            correlation_adjustments: Some(false),
        };
        let result = run_stress_test(&input).unwrap();
        // Alternative: equity * 0.4 = -0.30 * 0.4 = -0.12
        assert_eq!(
            result.result.scenario_results[0].portfolio_impact,
            dec!(-0.12)
        );
    }

    // -- Custom hypothetical scenario --

    #[test]
    fn test_custom_hypothetical_scenario() {
        let scenario = StressScenario {
            name: "Stagflation".into(),
            scenario_type: ScenarioType::Hypothetical,
            shocks: vec![
                MarketShock {
                    factor: "equity_market".into(),
                    shock_pct: dec!(-0.25),
                },
                MarketShock {
                    factor: "interest_rates".into(),
                    shock_pct: dec!(0.03),
                },
                MarketShock {
                    factor: "commodities".into(),
                    shock_pct: dec!(0.20),
                },
            ],
        };
        let input = StressTestInput {
            portfolio: diversified_portfolio(),
            scenarios: vec![scenario],
            correlation_adjustments: Some(false),
        };
        let result = run_stress_test(&input).unwrap();
        assert_eq!(result.result.scenario_results.len(), 1);
        let impacts = &result.result.scenario_results[0].position_impacts;
        assert_eq!(impacts.len(), 5);
    }

    // -- Built-in historical scenarios --

    #[test]
    fn test_get_historical_scenarios_returns_five() {
        let scenarios = get_historical_scenarios();
        assert_eq!(scenarios.len(), 5);
        assert_eq!(scenarios[0].name, "GFC 2008");
        assert_eq!(scenarios[1].name, "COVID March 2020");
        assert_eq!(scenarios[2].name, "Taper Tantrum 2013");
        assert_eq!(scenarios[3].name, "Dot-Com 2000");
        assert_eq!(scenarios[4].name, "Euro Crisis 2011");
    }

    #[test]
    fn test_all_historical_scenarios_are_historical_type() {
        for s in get_historical_scenarios() {
            assert_eq!(s.scenario_type, ScenarioType::Historical);
        }
    }

    // -- VaR breach --

    #[test]
    fn test_var_breach_flag() {
        let input = StressTestInput {
            portfolio: single_equity_portfolio(),
            scenarios: vec![
                StressScenario {
                    name: "Small Dip".into(),
                    scenario_type: ScenarioType::Hypothetical,
                    shocks: vec![MarketShock {
                        factor: "equity_market".into(),
                        shock_pct: dec!(-0.05),
                    }],
                },
                simple_equity_crash(), // -30%
            ],
            correlation_adjustments: Some(false),
        };
        let result = run_stress_test(&input).unwrap();
        assert!(!result.result.scenario_results[0].var_breach);
        assert!(result.result.scenario_results[1].var_breach);
    }

    // -- Portfolio summary --

    #[test]
    fn test_portfolio_summary_avg_loss() {
        let input = StressTestInput {
            portfolio: single_equity_portfolio(),
            scenarios: vec![
                StressScenario {
                    name: "Mild".into(),
                    scenario_type: ScenarioType::Hypothetical,
                    shocks: vec![MarketShock {
                        factor: "equity_market".into(),
                        shock_pct: dec!(-0.10),
                    }],
                },
                StressScenario {
                    name: "Severe".into(),
                    scenario_type: ScenarioType::Hypothetical,
                    shocks: vec![MarketShock {
                        factor: "equity_market".into(),
                        shock_pct: dec!(-0.30),
                    }],
                },
            ],
            correlation_adjustments: Some(false),
        };
        let result = run_stress_test(&input).unwrap();
        // Average = (-0.10 + -0.30) / 2 = -0.20
        assert_eq!(
            result.result.portfolio_summary.avg_scenario_loss,
            dec!(-0.20)
        );
        assert_eq!(
            result.result.portfolio_summary.max_drawdown_scenario,
            "Severe"
        );
    }

    // -- Validation errors --

    #[test]
    fn test_empty_portfolio_error() {
        let input = StressTestInput {
            portfolio: vec![],
            scenarios: vec![simple_equity_crash()],
            correlation_adjustments: None,
        };
        assert!(run_stress_test(&input).is_err());
    }

    #[test]
    fn test_empty_scenarios_error() {
        let input = StressTestInput {
            portfolio: single_equity_portfolio(),
            scenarios: vec![],
            correlation_adjustments: None,
        };
        assert!(run_stress_test(&input).is_err());
    }

    #[test]
    fn test_invalid_weight_error() {
        let portfolio = vec![PortfolioPosition {
            name: "Bad Weight".into(),
            weight: dec!(1.5),
            asset_class: AssetClass::Equity,
            beta: None,
            duration: None,
            fx_exposure: None,
        }];
        let input = StressTestInput {
            portfolio,
            scenarios: vec![simple_equity_crash()],
            correlation_adjustments: None,
        };
        assert!(run_stress_test(&input).is_err());
    }

    // -- Commodity position --

    #[test]
    fn test_commodity_direct_shock() {
        let portfolio = vec![PortfolioPosition {
            name: "Gold".into(),
            weight: Decimal::ONE,
            asset_class: AssetClass::Commodity,
            beta: None,
            duration: None,
            fx_exposure: None,
        }];
        let scenario = StressScenario {
            name: "Commodity Crash".into(),
            scenario_type: ScenarioType::Hypothetical,
            shocks: vec![MarketShock {
                factor: "commodities".into(),
                shock_pct: dec!(-0.25),
            }],
        };
        let input = StressTestInput {
            portfolio,
            scenarios: vec![scenario],
            correlation_adjustments: Some(false),
        };
        let result = run_stress_test(&input).unwrap();
        assert_eq!(
            result.result.scenario_results[0].portfolio_impact,
            dec!(-0.25)
        );
    }

    // -- Default beta and duration --

    #[test]
    fn test_default_equity_beta() {
        let portfolio = vec![PortfolioPosition {
            name: "No Beta".into(),
            weight: Decimal::ONE,
            asset_class: AssetClass::Equity,
            beta: None, // Should default to 1.0
            duration: None,
            fx_exposure: None,
        }];
        let input = StressTestInput {
            portfolio,
            scenarios: vec![simple_equity_crash()],
            correlation_adjustments: Some(false),
        };
        let result = run_stress_test(&input).unwrap();
        // Default beta=1.0, so impact = -0.30
        assert_eq!(
            result.result.scenario_results[0].portfolio_impact,
            dec!(-0.30)
        );
    }

    #[test]
    fn test_default_fi_duration() {
        let portfolio = vec![PortfolioPosition {
            name: "No Duration".into(),
            weight: Decimal::ONE,
            asset_class: AssetClass::FixedIncome,
            beta: None,
            duration: None, // Should default to 5
            fx_exposure: None,
        }];
        let scenario = StressScenario {
            name: "Rate Hike".into(),
            scenario_type: ScenarioType::Hypothetical,
            shocks: vec![MarketShock {
                factor: "interest_rates".into(),
                shock_pct: dec!(0.01),
            }],
        };
        let input = StressTestInput {
            portfolio,
            scenarios: vec![scenario],
            correlation_adjustments: Some(false),
        };
        let result = run_stress_test(&input).unwrap();
        // Default duration=5, impact = -5 * 0.01 = -0.05
        assert_eq!(
            result.result.scenario_results[0].portfolio_impact,
            dec!(-0.05)
        );
    }
}
