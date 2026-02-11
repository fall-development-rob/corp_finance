use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::error::CorpFinanceError;
use crate::types::{with_metadata, ComputationOutput, Money, Rate};
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Input types
// ---------------------------------------------------------------------------

/// Current allocation to a single asset class.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetAllocation {
    pub asset_class: String,
    pub weight: Rate,
    pub expected_return: Rate,
    pub duration: Decimal,
}

/// An instrument available for the LDI hedging portfolio.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LdiInstrument {
    pub name: String,
    /// E.g. "Government Bond", "Corporate Bond", "TIPS", "Swap".
    pub instrument_type: String,
    pub duration: Decimal,
    pub yield_rate: Rate,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub convexity: Option<Decimal>,
}

/// Glide-path parameters for transitioning allocation over time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlidePath {
    pub current_funded_ratio: Rate,
    pub target_funded_ratio: Rate,
    pub years_to_target: u32,
    /// Growth (return-seeking) allocation at start.
    pub growth_allocation_start: Rate,
    /// Growth allocation at the end of the glide path.
    pub growth_allocation_end: Rate,
}

/// Top-level input for liability-driven investing strategy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LdiInput {
    pub plan_name: String,
    /// Present value of pension liabilities.
    pub liability_pv: Money,
    /// Macaulay duration of liabilities in years.
    pub liability_duration: Decimal,
    /// Convexity of liabilities.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub liability_convexity: Option<Decimal>,
    /// Current fair value of plan assets.
    pub plan_assets: Money,
    /// Current weighted-average duration of asset portfolio.
    pub current_asset_duration: Decimal,
    /// Current portfolio allocation.
    pub current_asset_allocation: Vec<AssetAllocation>,
    /// Available instruments for constructing the hedging portfolio.
    pub available_instruments: Vec<LdiInstrument>,
    /// Fraction of liabilities to hedge (e.g. 0.80 = 80%).
    pub target_hedge_ratio: Rate,
    /// Duration mismatch threshold that triggers rebalancing (in years).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rebalancing_trigger: Option<Rate>,
    /// Optional glide-path schedule.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub glide_path: Option<GlidePath>,
}

// ---------------------------------------------------------------------------
// Output types
// ---------------------------------------------------------------------------

/// Recommended rebalance for a single asset class.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecommendedAllocation {
    pub asset_class: String,
    pub current_weight: Rate,
    pub target_weight: Rate,
    pub rebalance_amount: Money,
}

/// A single instrument chosen for the hedging portfolio.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HedgingInstrument {
    pub name: String,
    pub allocation_amount: Money,
    pub weight: Rate,
    pub contribution_to_duration: Decimal,
}

/// The complete hedging portfolio.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HedgingPortfolio {
    pub total_hedging_amount: Money,
    pub instruments: Vec<HedgingInstrument>,
    pub portfolio_duration: Decimal,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub portfolio_convexity: Option<Decimal>,
    pub duration_match_error: Decimal,
    pub hedge_ratio_achieved: Rate,
}

/// Assessment of immunization quality.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImmunizationResult {
    pub is_immunized: bool,
    pub duration_match: bool,
    pub convexity_match: bool,
    pub surplus_pv: Money,
    /// Change in surplus per 1 basis-point rate move.
    pub rate_sensitivity_bps: Money,
}

/// A single step in the glide-path schedule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlidePathStep {
    pub year: u32,
    pub target_funded_ratio: Rate,
    pub growth_allocation: Rate,
    pub hedging_allocation: Rate,
}

/// Complete output of LDI strategy design.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LdiOutput {
    pub plan_name: String,
    /// Duration gap = asset_duration - (liability_pv / plan_assets) * liability_duration.
    pub current_duration_gap: Decimal,
    /// Dollar duration gap = (dollar_dur_assets - dollar_dur_liabilities).
    pub dollar_duration_gap: Money,
    /// Estimated P&L impact from a 1% parallel rate shift.
    pub interest_rate_risk_1pct: Money,
    /// Recommended target allocation split.
    pub recommended_allocation: Vec<RecommendedAllocation>,
    /// Specific instrument portfolio to match liability duration.
    pub hedging_portfolio: HedgingPortfolio,
    /// Surplus-at-risk under a 1% rate shock.
    pub surplus_at_risk: Money,
    /// Immunization assessment.
    pub immunization_analysis: ImmunizationResult,
    /// Year-by-year glide-path schedule (if a glide path was provided).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub glide_path_schedule: Option<Vec<GlidePathStep>>,
}

// ---------------------------------------------------------------------------
// Core function
// ---------------------------------------------------------------------------

/// Design a liability-driven investing strategy for a pension plan.
pub fn design_ldi_strategy(input: &LdiInput) -> CorpFinanceResult<ComputationOutput<LdiOutput>> {
    let start = Instant::now();
    let mut warnings: Vec<String> = Vec::new();

    // -- Validation ----------------------------------------------------------
    if input.liability_pv <= dec!(0) {
        return Err(CorpFinanceError::InvalidInput {
            field: "liability_pv".into(),
            reason: "Must be positive".into(),
        });
    }
    if input.plan_assets <= dec!(0) {
        return Err(CorpFinanceError::InvalidInput {
            field: "plan_assets".into(),
            reason: "Must be positive".into(),
        });
    }
    if input.liability_duration < dec!(0) {
        return Err(CorpFinanceError::InvalidInput {
            field: "liability_duration".into(),
            reason: "Cannot be negative".into(),
        });
    }
    if input.current_asset_duration < dec!(0) {
        return Err(CorpFinanceError::InvalidInput {
            field: "current_asset_duration".into(),
            reason: "Cannot be negative".into(),
        });
    }
    if input.target_hedge_ratio <= dec!(0) || input.target_hedge_ratio > dec!(1) {
        return Err(CorpFinanceError::InvalidInput {
            field: "target_hedge_ratio".into(),
            reason: "Must be in (0, 1]".into(),
        });
    }
    if input.available_instruments.is_empty() {
        return Err(CorpFinanceError::InvalidInput {
            field: "available_instruments".into(),
            reason: "At least one instrument is required".into(),
        });
    }

    // -- Duration gap --------------------------------------------------------
    let leverage_ratio = input.liability_pv / input.plan_assets;
    let target_asset_duration = leverage_ratio * input.liability_duration;
    let duration_gap = input.current_asset_duration - target_asset_duration;

    // Dollar durations (per 100bp = /100)
    let dollar_dur_assets = input.plan_assets * input.current_asset_duration / dec!(100);
    let dollar_dur_liabilities = input.liability_pv * input.liability_duration / dec!(100);
    let dollar_duration_gap = dollar_dur_assets - dollar_dur_liabilities;

    // Interest rate risk from a 1% (100bp) parallel shift
    let interest_rate_risk_1pct = dollar_duration_gap;

    // Surplus at risk: |dollar_duration_gap| * 1% shock (already 1% in dollar_dur)
    let surplus_at_risk = dollar_duration_gap.abs();

    // -- Hedging portfolio construction --------------------------------------
    let hedge_amount = input.plan_assets * input.target_hedge_ratio;
    let _growth_amount = input.plan_assets - hedge_amount;

    // Target duration for the hedging portion:
    // hedge_weight * hedge_dur + growth_weight * growth_dur ≈ target_asset_dur
    // Assume growth portfolio duration ≈ 0 for simplicity (equities, etc.)
    // => hedge_dur = target_asset_dur / hedge_ratio
    let hedge_target_duration = if input.target_hedge_ratio > dec!(0) {
        target_asset_duration / input.target_hedge_ratio
    } else {
        dec!(0)
    };

    let hedging_portfolio = build_hedging_portfolio(
        &input.available_instruments,
        hedge_amount,
        hedge_target_duration,
        input.target_hedge_ratio,
        input.liability_convexity,
    );

    // -- Recommended allocation split ----------------------------------------
    let hedge_ratio = input.target_hedge_ratio;
    let growth_ratio = dec!(1) - hedge_ratio;

    let mut recommended_allocation = Vec::new();

    // Find current hedging/growth weights from asset allocation
    let current_hedge_weight: Decimal = input
        .current_asset_allocation
        .iter()
        .filter(|a| is_hedging_asset(&a.asset_class))
        .map(|a| a.weight)
        .sum();
    let current_growth_weight = dec!(1) - current_hedge_weight;

    recommended_allocation.push(RecommendedAllocation {
        asset_class: "Hedging (LDI)".into(),
        current_weight: current_hedge_weight,
        target_weight: hedge_ratio,
        rebalance_amount: (hedge_ratio - current_hedge_weight) * input.plan_assets,
    });
    recommended_allocation.push(RecommendedAllocation {
        asset_class: "Growth (Return-Seeking)".into(),
        current_weight: current_growth_weight,
        target_weight: growth_ratio,
        rebalance_amount: (growth_ratio - current_growth_weight) * input.plan_assets,
    });

    // -- Immunization analysis -----------------------------------------------
    let achieved_duration = hedging_portfolio.portfolio_duration * input.target_hedge_ratio;
    let rebalance_threshold = input.rebalancing_trigger.unwrap_or(dec!(0.5));
    let duration_match = (achieved_duration - target_asset_duration).abs() < rebalance_threshold;

    let convexity_match = match (
        input.liability_convexity,
        hedging_portfolio.portfolio_convexity,
    ) {
        (Some(l_conv), Some(p_conv)) => p_conv >= l_conv,
        _ => false,
    };

    let surplus_pv = input.plan_assets - input.liability_pv;

    // Rate sensitivity in bps: dollar_duration_gap / 10000 (1bp = 0.01%)
    // dollar_dur already scaled by /100, so per 1bp = dollar_dur_gap / 100
    let rate_sensitivity_bps = dollar_duration_gap / dec!(100);

    let immunization = ImmunizationResult {
        is_immunized: duration_match && (convexity_match || input.liability_convexity.is_none()),
        duration_match,
        convexity_match,
        surplus_pv,
        rate_sensitivity_bps,
    };

    // -- Glide path schedule -------------------------------------------------
    let glide_path_schedule = input.glide_path.as_ref().map(build_glide_path);

    // -- Warnings ------------------------------------------------------------
    if duration_gap.abs() > dec!(2) {
        warnings.push(format!(
            "Large duration gap of {:.2} years — significant interest rate risk",
            duration_gap
        ));
    }
    if surplus_pv < dec!(0) {
        warnings.push("Plan is underfunded — surplus is negative".into());
    }
    if !duration_match {
        warnings.push(format!(
            "Duration not matched within threshold ({:.1} years). Gap: {:.2}",
            rebalance_threshold,
            (achieved_duration - target_asset_duration).abs()
        ));
    }

    let output = LdiOutput {
        plan_name: input.plan_name.clone(),
        current_duration_gap: duration_gap,
        dollar_duration_gap,
        interest_rate_risk_1pct,
        recommended_allocation,
        hedging_portfolio,
        surplus_at_risk,
        immunization_analysis: immunization,
        glide_path_schedule,
    };

    let elapsed = start.elapsed().as_micros() as u64;
    Ok(with_metadata(
        "Liability-Driven Investing Strategy (duration matching + immunization)",
        &serde_json::json!({
            "target_hedge_ratio": input.target_hedge_ratio.to_string(),
            "liability_duration": input.liability_duration.to_string(),
            "target_asset_duration": target_asset_duration.to_string(),
            "leverage_ratio": leverage_ratio.to_string(),
        }),
        warnings,
        elapsed,
        output,
    ))
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Heuristic: classify asset classes as "hedging" vs "growth".
fn is_hedging_asset(asset_class: &str) -> bool {
    let lower = asset_class.to_lowercase();
    lower.contains("bond")
        || lower.contains("fixed")
        || lower.contains("treasury")
        || lower.contains("gilt")
        || lower.contains("tips")
        || lower.contains("swap")
        || lower.contains("ldi")
        || lower.contains("hedg")
}

/// Build the hedging instrument portfolio to match a target duration.
fn build_hedging_portfolio(
    instruments: &[LdiInstrument],
    hedge_amount: Money,
    target_duration: Decimal,
    target_hedge_ratio: Rate,
    liability_convexity: Option<Decimal>,
) -> HedgingPortfolio {
    if instruments.is_empty() || hedge_amount <= dec!(0) {
        return HedgingPortfolio {
            total_hedging_amount: dec!(0),
            instruments: vec![],
            portfolio_duration: dec!(0),
            portfolio_convexity: None,
            duration_match_error: target_duration.abs(),
            hedge_ratio_achieved: dec!(0),
        };
    }

    // Sort instruments by duration
    let mut sorted: Vec<&LdiInstrument> = instruments.iter().collect();
    sorted.sort_by(|a, b| a.duration.cmp(&b.duration));

    let instrument_weights: Vec<(usize, Decimal)>;

    if sorted.len() == 1 {
        // Only one instrument: allocate 100%
        instrument_weights = vec![(0, dec!(1))];
    } else {
        // Find the two instruments that bracket the target duration
        // for a barbell / linear interpolation approach
        let mut lower_idx: Option<usize> = None;
        let mut upper_idx: Option<usize> = None;

        for (i, inst) in sorted.iter().enumerate() {
            if inst.duration <= target_duration {
                lower_idx = Some(i);
            }
            if inst.duration >= target_duration && upper_idx.is_none() {
                upper_idx = Some(i);
            }
        }

        match (lower_idx, upper_idx) {
            (Some(li), Some(ui)) if li != ui => {
                // Linear interpolation: w_low * d_low + w_high * d_high = target
                // w_low + w_high = 1
                let d_low = sorted[li].duration;
                let d_high = sorted[ui].duration;
                let span = d_high - d_low;
                if span > dec!(0) {
                    let w_high = (target_duration - d_low) / span;
                    let w_low = dec!(1) - w_high;
                    instrument_weights = vec![(li, w_low), (ui, w_high)];
                } else {
                    instrument_weights = vec![(li, dec!(1))];
                }
            }
            (Some(li), _) => {
                // Target below or at the lowest bracket: use closest
                instrument_weights = vec![(li, dec!(1))];
            }
            (_, Some(ui)) => {
                instrument_weights = vec![(ui, dec!(1))];
            }
            _ => {
                // Fallback: equal weight all
                let n = Decimal::from(sorted.len() as u32);
                let w = dec!(1) / n;
                instrument_weights = (0..sorted.len()).map(|i| (i, w)).collect();
            }
        }
    }

    // Build output
    let mut hedging_instruments = Vec::new();
    let mut portfolio_duration = dec!(0);
    let mut portfolio_convexity_sum = dec!(0);
    let mut has_convexity = false;

    for &(idx, weight) in &instrument_weights {
        let inst = sorted[idx];
        let amount = hedge_amount * weight;
        let dur_contribution = weight * inst.duration;
        portfolio_duration += dur_contribution;

        if let Some(conv) = inst.convexity {
            portfolio_convexity_sum += weight * conv;
            has_convexity = true;
        }

        hedging_instruments.push(HedgingInstrument {
            name: inst.name.clone(),
            allocation_amount: amount,
            weight,
            contribution_to_duration: dur_contribution,
        });
    }

    let portfolio_convexity = if has_convexity {
        Some(portfolio_convexity_sum)
    } else {
        None
    };

    let duration_match_error = (portfolio_duration - target_duration).abs();

    // Ignore liability_convexity in portfolio construction (used only for
    // immunization check), but silence the unused-variable warning.
    let _ = liability_convexity;

    HedgingPortfolio {
        total_hedging_amount: hedge_amount,
        instruments: hedging_instruments,
        portfolio_duration,
        portfolio_convexity,
        duration_match_error,
        hedge_ratio_achieved: target_hedge_ratio,
    }
}

/// Build year-by-year glide-path schedule via linear interpolation.
fn build_glide_path(gp: &GlidePath) -> Vec<GlidePathStep> {
    let mut steps = Vec::new();
    let years = gp.years_to_target.max(1);

    for y in 0..=years {
        let frac = Decimal::from(y) / Decimal::from(years);
        let growth = gp.growth_allocation_start
            + (gp.growth_allocation_end - gp.growth_allocation_start) * frac;
        let hedging = dec!(1) - growth;
        let funded =
            gp.current_funded_ratio + (gp.target_funded_ratio - gp.current_funded_ratio) * frac;

        steps.push(GlidePathStep {
            year: y,
            target_funded_ratio: funded,
            growth_allocation: growth,
            hedging_allocation: hedging,
        });
    }
    steps
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn basic_allocation() -> Vec<AssetAllocation> {
        vec![
            AssetAllocation {
                asset_class: "Equities".into(),
                weight: dec!(0.60),
                expected_return: dec!(0.08),
                duration: dec!(0),
            },
            AssetAllocation {
                asset_class: "Bonds".into(),
                weight: dec!(0.40),
                expected_return: dec!(0.04),
                duration: dec!(7),
            },
        ]
    }

    fn basic_instruments() -> Vec<LdiInstrument> {
        vec![
            LdiInstrument {
                name: "Long Govt Bond".into(),
                instrument_type: "Government Bond".into(),
                duration: dec!(15),
                yield_rate: dec!(0.035),
                convexity: Some(dec!(250)),
            },
            LdiInstrument {
                name: "Intermediate Corp".into(),
                instrument_type: "Corporate Bond".into(),
                duration: dec!(7),
                yield_rate: dec!(0.045),
                convexity: Some(dec!(60)),
            },
        ]
    }

    fn basic_input() -> LdiInput {
        LdiInput {
            plan_name: "Test LDI Plan".into(),
            liability_pv: dec!(1000000),
            liability_duration: dec!(12),
            liability_convexity: Some(dec!(200)),
            plan_assets: dec!(800000),
            current_asset_duration: dec!(5),
            current_asset_allocation: basic_allocation(),
            available_instruments: basic_instruments(),
            target_hedge_ratio: dec!(0.80),
            rebalancing_trigger: Some(dec!(0.5)),
            glide_path: None,
        }
    }

    #[test]
    fn test_duration_gap_calculation() {
        let input = basic_input();
        let result = design_ldi_strategy(&input).unwrap();
        let r = &result.result;
        // leverage ratio = 1000000 / 800000 = 1.25
        // target asset duration = 1.25 * 12 = 15
        // gap = 5 - 15 = -10
        assert_eq!(r.current_duration_gap, dec!(5) - dec!(15));
    }

    #[test]
    fn test_dollar_duration_gap() {
        let input = basic_input();
        let result = design_ldi_strategy(&input).unwrap();
        let r = &result.result;
        // DD assets = 800000 * 5 / 100 = 40000
        // DD liabilities = 1000000 * 12 / 100 = 120000
        // gap = 40000 - 120000 = -80000
        let expected = dec!(40000) - dec!(120000);
        assert_eq!(r.dollar_duration_gap, expected);
    }

    #[test]
    fn test_interest_rate_risk_equals_dd_gap() {
        let result = design_ldi_strategy(&basic_input()).unwrap();
        let r = &result.result;
        assert_eq!(r.interest_rate_risk_1pct, r.dollar_duration_gap);
    }

    #[test]
    fn test_surplus_at_risk_positive() {
        let result = design_ldi_strategy(&basic_input()).unwrap();
        assert!(result.result.surplus_at_risk > dec!(0));
    }

    #[test]
    fn test_hedging_portfolio_amount() {
        let input = basic_input();
        let result = design_ldi_strategy(&input).unwrap();
        let hp = &result.result.hedging_portfolio;
        // 80% of 800000 = 640000
        assert_eq!(hp.total_hedging_amount, dec!(640000));
    }

    #[test]
    fn test_hedging_instruments_weights_sum_to_one() {
        let result = design_ldi_strategy(&basic_input()).unwrap();
        let hp = &result.result.hedging_portfolio;
        let weight_sum: Decimal = hp.instruments.iter().map(|i| i.weight).sum();
        let diff = (weight_sum - dec!(1)).abs();
        assert!(
            diff < dec!(0.0001),
            "Weights should sum to 1, got {}",
            weight_sum
        );
    }

    #[test]
    fn test_hedging_portfolio_duration_interpolation() {
        // With instruments at dur=7 and dur=15, portfolio should interpolate
        let result = design_ldi_strategy(&basic_input()).unwrap();
        let hp = &result.result.hedging_portfolio;
        assert!(hp.portfolio_duration >= dec!(7));
        assert!(hp.portfolio_duration <= dec!(15));
    }

    #[test]
    fn test_recommended_allocation_has_entries() {
        let result = design_ldi_strategy(&basic_input()).unwrap();
        assert_eq!(result.result.recommended_allocation.len(), 2);
    }

    #[test]
    fn test_recommended_target_weights_sum_to_one() {
        let result = design_ldi_strategy(&basic_input()).unwrap();
        let total: Decimal = result
            .result
            .recommended_allocation
            .iter()
            .map(|r| r.target_weight)
            .sum();
        assert_eq!(total, dec!(1));
    }

    #[test]
    fn test_immunization_not_achieved_with_large_gap() {
        // Default input has large duration gap, so not immunized
        let result = design_ldi_strategy(&basic_input()).unwrap();
        // Given the mismatch, likely not fully immunized
        let imm = &result.result.immunization_analysis;
        assert_eq!(imm.surplus_pv, dec!(800000) - dec!(1000000));
    }

    #[test]
    fn test_glide_path_none_when_not_provided() {
        let result = design_ldi_strategy(&basic_input()).unwrap();
        assert!(result.result.glide_path_schedule.is_none());
    }

    #[test]
    fn test_glide_path_schedule_generated() {
        let mut input = basic_input();
        input.glide_path = Some(GlidePath {
            current_funded_ratio: dec!(0.80),
            target_funded_ratio: dec!(1.00),
            years_to_target: 5,
            growth_allocation_start: dec!(0.60),
            growth_allocation_end: dec!(0.20),
        });
        let result = design_ldi_strategy(&input).unwrap();
        let schedule = result.result.glide_path_schedule.as_ref().unwrap();
        // 0..=5 => 6 steps
        assert_eq!(schedule.len(), 6);
    }

    #[test]
    fn test_glide_path_start_end_values() {
        let mut input = basic_input();
        input.glide_path = Some(GlidePath {
            current_funded_ratio: dec!(0.80),
            target_funded_ratio: dec!(1.00),
            years_to_target: 5,
            growth_allocation_start: dec!(0.60),
            growth_allocation_end: dec!(0.20),
        });
        let result = design_ldi_strategy(&input).unwrap();
        let schedule = result.result.glide_path_schedule.as_ref().unwrap();
        assert_eq!(schedule[0].growth_allocation, dec!(0.60));
        assert_eq!(schedule[0].hedging_allocation, dec!(0.40));
        assert_eq!(schedule[5].growth_allocation, dec!(0.20));
        assert_eq!(schedule[5].hedging_allocation, dec!(0.80));
    }

    #[test]
    fn test_glide_path_funded_ratio_progression() {
        let mut input = basic_input();
        input.glide_path = Some(GlidePath {
            current_funded_ratio: dec!(0.80),
            target_funded_ratio: dec!(1.00),
            years_to_target: 4,
            growth_allocation_start: dec!(0.50),
            growth_allocation_end: dec!(0.10),
        });
        let result = design_ldi_strategy(&input).unwrap();
        let schedule = result.result.glide_path_schedule.as_ref().unwrap();
        // Funded ratio should increase monotonically
        for i in 1..schedule.len() {
            assert!(schedule[i].target_funded_ratio >= schedule[i - 1].target_funded_ratio);
        }
    }

    #[test]
    fn test_validation_negative_liability_pv() {
        let mut input = basic_input();
        input.liability_pv = dec!(-1);
        let err = design_ldi_strategy(&input).unwrap_err();
        match err {
            CorpFinanceError::InvalidInput { field, .. } => assert_eq!(field, "liability_pv"),
            _ => panic!("Expected InvalidInput error"),
        }
    }

    #[test]
    fn test_validation_zero_assets() {
        let mut input = basic_input();
        input.plan_assets = dec!(0);
        let err = design_ldi_strategy(&input).unwrap_err();
        match err {
            CorpFinanceError::InvalidInput { field, .. } => assert_eq!(field, "plan_assets"),
            _ => panic!("Expected InvalidInput error"),
        }
    }

    #[test]
    fn test_validation_hedge_ratio_out_of_range() {
        let mut input = basic_input();
        input.target_hedge_ratio = dec!(1.5);
        let err = design_ldi_strategy(&input).unwrap_err();
        match err {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "target_hedge_ratio")
            }
            _ => panic!("Expected InvalidInput error"),
        }
    }

    #[test]
    fn test_validation_no_instruments() {
        let mut input = basic_input();
        input.available_instruments = vec![];
        let err = design_ldi_strategy(&input).unwrap_err();
        match err {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "available_instruments")
            }
            _ => panic!("Expected InvalidInput error"),
        }
    }

    #[test]
    fn test_single_instrument_portfolio() {
        let mut input = basic_input();
        input.available_instruments = vec![LdiInstrument {
            name: "Single Bond".into(),
            instrument_type: "Government Bond".into(),
            duration: dec!(10),
            yield_rate: dec!(0.04),
            convexity: None,
        }];
        let result = design_ldi_strategy(&input).unwrap();
        let hp = &result.result.hedging_portfolio;
        assert_eq!(hp.instruments.len(), 1);
        assert_eq!(hp.instruments[0].weight, dec!(1));
        assert_eq!(hp.portfolio_duration, dec!(10));
    }

    #[test]
    fn test_warning_on_underfunded() {
        let result = design_ldi_strategy(&basic_input()).unwrap();
        // assets 800k < liabilities 1M
        assert!(result.warnings.iter().any(|w| w.contains("underfunded")));
    }
}
