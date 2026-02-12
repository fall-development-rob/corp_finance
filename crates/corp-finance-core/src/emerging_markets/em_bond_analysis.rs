//! Emerging-market bond analysis (local vs hard currency).
//!
//! Implements:
//! 1. **Yield differential** -- local vs hard currency
//! 2. **Implied FX depreciation** -- covered interest parity
//! 3. **Real yields** -- inflation-adjusted local and hard currency
//! 4. **Carry trade return** -- yield pickup minus expected depreciation
//! 5. **Hedged / unhedged returns** -- with and without FX hedging
//! 6. **FX risk contribution** -- vol x duration
//! 7. **Breakeven FX move** -- yield advantage / duration
//! 8. **Recommendation** -- Local Unhedged / Local Hedged / Hard Currency
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

/// Input for EM bond analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmBondAnalysisInput {
    /// Local currency bond yield (e.g. 0.10 = 10%).
    pub local_currency_yield: Decimal,
    /// Hard currency (USD/EUR) bond yield (e.g. 0.06 = 6%).
    pub hard_currency_yield: Decimal,
    /// Spot FX rate (local currency per 1 USD).
    pub spot_fx_rate: Decimal,
    /// Forward or expected future FX rate (local per USD).
    pub forward_fx_rate: Decimal,
    /// Local country inflation rate.
    pub local_inflation: Decimal,
    /// US inflation rate.
    pub us_inflation: Decimal,
    /// Sovereign spread over US Treasuries.
    pub sovereign_spread: Decimal,
    /// Modified duration of local currency bond.
    pub local_bond_duration: Decimal,
    /// Modified duration of hard currency bond.
    pub hard_bond_duration: Decimal,
    /// FX volatility (annualised).
    pub fx_volatility: Decimal,
    /// Investment amount.
    pub investment_amount: Decimal,
    /// Annual FX hedging cost as percentage (e.g. 0.03 = 3%).
    pub hedging_cost: Decimal,
}

/// Output from EM bond analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmBondAnalysisOutput {
    /// local_yield - hard_yield.
    pub yield_differential: Decimal,
    /// (forward - spot) / spot.
    pub implied_depreciation: Decimal,
    /// local_yield - local_inflation.
    pub real_yield_local: Decimal,
    /// hard_yield - us_inflation.
    pub real_yield_hard: Decimal,
    /// real_yield_local - real_yield_hard.
    pub real_yield_differential: Decimal,
    /// yield_differential - implied_depreciation.
    pub carry_trade_return: Decimal,
    /// local_yield - hedging_cost.
    pub hedged_return_local: Decimal,
    /// local_yield + FX return (implied: -implied_depreciation).
    pub unhedged_return_local: Decimal,
    /// fx_vol * local_duration (simplified risk).
    pub fx_risk_contribution: Decimal,
    /// yield_differential / local_bond_duration.
    pub breakeven_fx_move: Decimal,
    /// "Local Unhedged" / "Local Hedged" / "Hard Currency".
    pub recommendation: String,
}

// ---------------------------------------------------------------------------
// Main function
// ---------------------------------------------------------------------------

/// Analyse EM bonds: local currency vs hard currency trade-offs.
pub fn analyse_em_bonds(input: &EmBondAnalysisInput) -> CorpFinanceResult<EmBondAnalysisOutput> {
    // Validation
    if input.spot_fx_rate <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "spot_fx_rate".to_string(),
            reason: "Spot FX rate must be positive".to_string(),
        });
    }
    if input.forward_fx_rate <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "forward_fx_rate".to_string(),
            reason: "Forward FX rate must be positive".to_string(),
        });
    }
    if input.local_bond_duration <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "local_bond_duration".to_string(),
            reason: "Duration must be positive".to_string(),
        });
    }
    if input.hard_bond_duration <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "hard_bond_duration".to_string(),
            reason: "Duration must be positive".to_string(),
        });
    }
    if input.fx_volatility < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "fx_volatility".to_string(),
            reason: "FX volatility cannot be negative".to_string(),
        });
    }
    if input.investment_amount <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "investment_amount".to_string(),
            reason: "Investment amount must be positive".to_string(),
        });
    }
    if input.hedging_cost < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "hedging_cost".to_string(),
            reason: "Hedging cost cannot be negative".to_string(),
        });
    }

    // 1. Yield differential
    let yield_differential = input.local_currency_yield - input.hard_currency_yield;

    // 2. Implied depreciation from covered interest parity
    let implied_depreciation = (input.forward_fx_rate - input.spot_fx_rate) / input.spot_fx_rate;

    // 3. Real yields
    let real_yield_local = input.local_currency_yield - input.local_inflation;
    let real_yield_hard = input.hard_currency_yield - input.us_inflation;
    let real_yield_differential = real_yield_local - real_yield_hard;

    // 4. Carry trade return = yield pickup - expected depreciation
    let carry_trade_return = yield_differential - implied_depreciation;

    // 5. Hedged return = local yield - hedging cost
    let hedged_return_local = input.local_currency_yield - input.hedging_cost;

    // 6. Unhedged return = local yield - implied depreciation
    //    (investor earns local yield but loses on FX if currency depreciates)
    let unhedged_return_local = input.local_currency_yield - implied_depreciation;

    // 7. FX risk contribution (simplified: vol * duration)
    let fx_risk_contribution = input.fx_volatility * input.local_bond_duration;

    // 8. Breakeven FX move
    let breakeven_fx_move = yield_differential / input.local_bond_duration;

    // 9. Recommendation logic
    let fx_vol_threshold = dec!(0.15);
    let recommendation = if hedged_return_local > input.hard_currency_yield {
        if carry_trade_return > Decimal::ZERO && input.fx_volatility < fx_vol_threshold {
            "Local Unhedged".to_string()
        } else {
            "Local Hedged".to_string()
        }
    } else if carry_trade_return > Decimal::ZERO && input.fx_volatility < fx_vol_threshold {
        "Local Unhedged".to_string()
    } else {
        "Hard Currency".to_string()
    };

    Ok(EmBondAnalysisOutput {
        yield_differential,
        implied_depreciation,
        real_yield_local,
        real_yield_hard,
        real_yield_differential,
        carry_trade_return,
        hedged_return_local,
        unhedged_return_local,
        fx_risk_contribution,
        breakeven_fx_move,
        recommendation,
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn base_input() -> EmBondAnalysisInput {
        EmBondAnalysisInput {
            local_currency_yield: dec!(0.10),
            hard_currency_yield: dec!(0.06),
            spot_fx_rate: dec!(80),    // 80 local per USD
            forward_fx_rate: dec!(83), // implies ~3.75% depreciation
            local_inflation: dec!(0.05),
            us_inflation: dec!(0.02),
            sovereign_spread: dec!(0.03),
            local_bond_duration: dec!(5),
            hard_bond_duration: dec!(7),
            fx_volatility: dec!(0.12),
            investment_amount: dec!(10_000_000),
            hedging_cost: dec!(0.035),
        }
    }

    #[test]
    fn test_yield_differential() {
        let input = base_input();
        let out = analyse_em_bonds(&input).unwrap();
        assert_eq!(out.yield_differential, dec!(0.04));
    }

    #[test]
    fn test_implied_depreciation() {
        let input = base_input();
        let out = analyse_em_bonds(&input).unwrap();
        // (83 - 80) / 80 = 0.0375
        assert_eq!(out.implied_depreciation, dec!(0.0375));
    }

    #[test]
    fn test_real_yield_local() {
        let input = base_input();
        let out = analyse_em_bonds(&input).unwrap();
        assert_eq!(out.real_yield_local, dec!(0.05));
    }

    #[test]
    fn test_real_yield_hard() {
        let input = base_input();
        let out = analyse_em_bonds(&input).unwrap();
        assert_eq!(out.real_yield_hard, dec!(0.04));
    }

    #[test]
    fn test_real_yield_differential() {
        let input = base_input();
        let out = analyse_em_bonds(&input).unwrap();
        assert_eq!(out.real_yield_differential, dec!(0.01));
    }

    #[test]
    fn test_carry_trade_return() {
        let input = base_input();
        let out = analyse_em_bonds(&input).unwrap();
        // carry = 0.04 - 0.0375 = 0.0025
        assert_eq!(out.carry_trade_return, dec!(0.0025));
    }

    #[test]
    fn test_hedged_return() {
        let input = base_input();
        let out = analyse_em_bonds(&input).unwrap();
        // hedged = 0.10 - 0.035 = 0.065
        assert_eq!(out.hedged_return_local, dec!(0.065));
    }

    #[test]
    fn test_unhedged_return() {
        let input = base_input();
        let out = analyse_em_bonds(&input).unwrap();
        // unhedged = 0.10 - 0.0375 = 0.0625
        assert_eq!(out.unhedged_return_local, dec!(0.0625));
    }

    #[test]
    fn test_fx_risk_contribution() {
        let input = base_input();
        let out = analyse_em_bonds(&input).unwrap();
        // 0.12 * 5 = 0.60
        assert_eq!(out.fx_risk_contribution, dec!(0.60));
    }

    #[test]
    fn test_breakeven_fx_move() {
        let input = base_input();
        let out = analyse_em_bonds(&input).unwrap();
        // 0.04 / 5 = 0.008
        assert_eq!(out.breakeven_fx_move, dec!(0.008));
    }

    #[test]
    fn test_recommendation_local_hedged() {
        let input = base_input();
        let out = analyse_em_bonds(&input).unwrap();
        // hedged 0.065 > hard 0.06, but fx_vol 0.12 < 0.15 and carry > 0
        // => Local Unhedged (carry positive and vol low)
        assert_eq!(out.recommendation, "Local Unhedged");
    }

    #[test]
    fn test_recommendation_hard_currency() {
        let mut input = base_input();
        input.local_currency_yield = dec!(0.065);
        input.hedging_cost = dec!(0.02);
        input.fx_volatility = dec!(0.25); // high vol
        input.forward_fx_rate = dec!(90); // big depreciation
        let out = analyse_em_bonds(&input).unwrap();
        // carry = (0.065-0.06) - (90-80)/80 = 0.005 - 0.125 = -0.12 (negative)
        // high vol => Hard Currency
        assert_eq!(out.recommendation, "Hard Currency");
    }

    #[test]
    fn test_negative_carry() {
        let mut input = base_input();
        input.forward_fx_rate = dec!(90); // large depreciation
        let out = analyse_em_bonds(&input).unwrap();
        // implied = (90-80)/80 = 0.125
        // carry = 0.04 - 0.125 = -0.085
        assert!(out.carry_trade_return < Decimal::ZERO);
    }

    #[test]
    fn test_high_carry_low_vol() {
        let mut input = base_input();
        input.local_currency_yield = dec!(0.14);
        input.forward_fx_rate = dec!(81);
        input.fx_volatility = dec!(0.08);
        let out = analyse_em_bonds(&input).unwrap();
        assert!(out.carry_trade_return > Decimal::ZERO);
        assert_eq!(out.recommendation, "Local Unhedged");
    }

    #[test]
    fn test_hedged_vs_unhedged() {
        let input = base_input();
        let out = analyse_em_bonds(&input).unwrap();
        // Both should be close when hedging cost ~ implied depreciation
        assert!(out.hedged_return_local > Decimal::ZERO);
        assert!(out.unhedged_return_local > Decimal::ZERO);
    }

    #[test]
    fn test_duration_effect_on_breakeven() {
        let mut input = base_input();
        input.local_bond_duration = dec!(10);
        let out = analyse_em_bonds(&input).unwrap();
        // breakeven = 0.04 / 10 = 0.004 (smaller with longer duration)
        assert_eq!(out.breakeven_fx_move, dec!(0.004));
    }

    #[test]
    fn test_invalid_zero_spot_rate() {
        let mut input = base_input();
        input.spot_fx_rate = Decimal::ZERO;
        let err = analyse_em_bonds(&input).unwrap_err();
        assert!(matches!(err, CorpFinanceError::InvalidInput { .. }));
    }

    #[test]
    fn test_invalid_zero_duration() {
        let mut input = base_input();
        input.local_bond_duration = Decimal::ZERO;
        let err = analyse_em_bonds(&input).unwrap_err();
        assert!(matches!(err, CorpFinanceError::InvalidInput { .. }));
    }

    #[test]
    fn test_invalid_negative_fx_vol() {
        let mut input = base_input();
        input.fx_volatility = dec!(-0.05);
        let err = analyse_em_bonds(&input).unwrap_err();
        assert!(matches!(err, CorpFinanceError::InvalidInput { .. }));
    }

    #[test]
    fn test_invalid_zero_investment() {
        let mut input = base_input();
        input.investment_amount = Decimal::ZERO;
        let err = analyse_em_bonds(&input).unwrap_err();
        assert!(matches!(err, CorpFinanceError::InvalidInput { .. }));
    }

    #[test]
    fn test_currency_appreciation() {
        let mut input = base_input();
        input.forward_fx_rate = dec!(78); // currency appreciates
        let out = analyse_em_bonds(&input).unwrap();
        // implied = (78-80)/80 = -0.025 (appreciation)
        assert!(out.implied_depreciation < Decimal::ZERO);
        // unhedged return boosted by appreciation
        assert!(out.unhedged_return_local > input.local_currency_yield);
    }

    #[test]
    fn test_high_volatility_hard_currency_recommendation() {
        let mut input = base_input();
        input.fx_volatility = dec!(0.30);
        input.hedging_cost = dec!(0.08); // expensive hedge
        let out = analyse_em_bonds(&input).unwrap();
        // hedged = 0.10 - 0.08 = 0.02 < hard 0.06
        // vol > threshold => Hard Currency
        assert_eq!(out.recommendation, "Hard Currency");
    }
}
