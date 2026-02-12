//! Concentrated stock position management strategies.
//!
//! Evaluates and compares five strategies for managing concentrated equity positions:
//! 1. **Outright Sale** -- sell, pay tax, reinvest diversified.
//! 2. **Collar** -- protective put + covered call for downside floor / upside cap.
//! 3. **Exchange Fund** -- contribute stock for diversified basket, 7-year lockup.
//! 4. **Prepaid Variable Forward** -- receive upfront advance, deliver shares later.
//! 5. **Charitable Remainder Trust** -- donate to CRT, receive income stream.
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

/// Input for concentrated stock position analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConcentratedStockInput {
    /// Current value of the concentrated position.
    pub position_value: Decimal,
    /// Original cost basis.
    pub cost_basis: Decimal,
    /// Annual dividend yield on the stock.
    pub annual_dividend_yield: Decimal,
    /// Annualized volatility.
    pub stock_volatility: Decimal,
    /// Risk-free rate.
    pub risk_free_rate: Decimal,
    /// Investment horizon in years.
    pub investment_horizon: u32,
    /// Long-term capital gains tax rate.
    pub tax_rate_ltcg: Decimal,
    /// Short-term capital gains tax rate.
    pub tax_rate_stcg: Decimal,
    /// Put strike as fraction of current price (e.g. 0.90 = 90%).
    pub collar_put_strike_pct: Decimal,
    /// Call strike as fraction of current price (e.g. 1.10 = 110%).
    pub collar_call_strike_pct: Decimal,
    /// Fraction of portfolio diversified via exchange fund (e.g. 0.70).
    pub exchange_fund_diversification_pct: Decimal,
    /// Fraction of value received upfront in prepaid variable forward.
    pub prepaid_forward_advance_pct: Decimal,
}

/// A single strategy comparison result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyComparison {
    /// Strategy name.
    pub name: String,
    /// After-tax value or equivalent.
    pub after_tax_value: Decimal,
    /// Risk reduction as a percentage (0-1).
    pub risk_reduction_pct: Decimal,
    /// Liquidity percentage (0-1).
    pub liquidity_pct: Decimal,
    /// Estimated cost of the strategy.
    pub cost: Decimal,
}

/// Output of concentrated stock analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConcentratedStockOutput {
    /// Unrealized gain = position_value - cost_basis.
    pub unrealized_gain: Decimal,
    /// Embedded tax liability = unrealized_gain * tax_rate_ltcg.
    pub embedded_tax: Decimal,
    /// After-tax value = position_value - embedded_tax.
    pub after_tax_value: Decimal,
    /// Strategy comparisons.
    pub strategies: Vec<StrategyComparison>,
    /// Recommended strategy name.
    pub recommended_strategy: String,
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

fn validate(input: &ConcentratedStockInput) -> CorpFinanceResult<()> {
    if input.position_value <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "position_value".into(),
            reason: "must be positive".into(),
        });
    }
    if input.cost_basis < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "cost_basis".into(),
            reason: "cannot be negative".into(),
        });
    }
    if input.tax_rate_ltcg < Decimal::ZERO || input.tax_rate_ltcg > Decimal::ONE {
        return Err(CorpFinanceError::InvalidInput {
            field: "tax_rate_ltcg".into(),
            reason: "must be between 0 and 1".into(),
        });
    }
    if input.tax_rate_stcg < Decimal::ZERO || input.tax_rate_stcg > Decimal::ONE {
        return Err(CorpFinanceError::InvalidInput {
            field: "tax_rate_stcg".into(),
            reason: "must be between 0 and 1".into(),
        });
    }
    if input.collar_put_strike_pct <= Decimal::ZERO || input.collar_put_strike_pct > Decimal::ONE {
        return Err(CorpFinanceError::InvalidInput {
            field: "collar_put_strike_pct".into(),
            reason: "must be between 0 (exclusive) and 1 (inclusive)".into(),
        });
    }
    if input.collar_call_strike_pct <= Decimal::ONE {
        return Err(CorpFinanceError::InvalidInput {
            field: "collar_call_strike_pct".into(),
            reason: "must be greater than 1".into(),
        });
    }
    if input.exchange_fund_diversification_pct < Decimal::ZERO
        || input.exchange_fund_diversification_pct > Decimal::ONE
    {
        return Err(CorpFinanceError::InvalidInput {
            field: "exchange_fund_diversification_pct".into(),
            reason: "must be between 0 and 1".into(),
        });
    }
    if input.prepaid_forward_advance_pct < Decimal::ZERO
        || input.prepaid_forward_advance_pct > Decimal::ONE
    {
        return Err(CorpFinanceError::InvalidInput {
            field: "prepaid_forward_advance_pct".into(),
            reason: "must be between 0 and 1".into(),
        });
    }
    if input.investment_horizon == 0 {
        return Err(CorpFinanceError::InvalidInput {
            field: "investment_horizon".into(),
            reason: "must be at least 1 year".into(),
        });
    }
    if input.stock_volatility < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "stock_volatility".into(),
            reason: "cannot be negative".into(),
        });
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Core function
// ---------------------------------------------------------------------------

/// Analyse a concentrated stock position and compare diversification strategies.
pub fn analyze_concentrated_stock(
    input: &ConcentratedStockInput,
) -> CorpFinanceResult<ConcentratedStockOutput> {
    validate(input)?;

    let pv = input.position_value;
    let cb = input.cost_basis;
    let unrealized_gain = pv - cb;
    // Tax only applies when there is a gain
    let taxable_gain = if unrealized_gain > Decimal::ZERO {
        unrealized_gain
    } else {
        Decimal::ZERO
    };
    let embedded_tax = taxable_gain * input.tax_rate_ltcg;
    let after_tax_value = pv - embedded_tax;

    let mut strategies: Vec<StrategyComparison> = Vec::with_capacity(5);

    // 1. Outright Sale
    let sale_proceeds = pv - taxable_gain * input.tax_rate_ltcg;
    // Diversified portfolio expected return over horizon
    let diversified_return = input.risk_free_rate + dec!(0.04); // equity risk premium ~4%
    let mut sale_fv = sale_proceeds;
    let growth = Decimal::ONE + diversified_return;
    for _ in 0..input.investment_horizon {
        sale_fv *= growth;
    }
    strategies.push(StrategyComparison {
        name: "Outright Sale".into(),
        after_tax_value: sale_proceeds,
        risk_reduction_pct: Decimal::ONE, // fully diversified
        liquidity_pct: Decimal::ONE,
        cost: embedded_tax,
    });

    // 2. Collar
    let collar_floor = input.collar_put_strike_pct * pv;
    let collar_cap = input.collar_call_strike_pct * pv;
    let collar_cost = pv * dec!(0.02); // simplified net premium ~2%
                                       // Risk reduction: bounded by floor/cap band
    let band = collar_cap - collar_floor;
    let risk_reduction = if pv > Decimal::ZERO {
        Decimal::ONE - band / pv
    } else {
        Decimal::ZERO
    };
    let risk_reduction = if risk_reduction < Decimal::ZERO {
        Decimal::ZERO
    } else {
        risk_reduction
    };
    // After-tax: midpoint of band - collar cost (no immediate tax)
    let collar_atv = (collar_floor + collar_cap) / dec!(2) - collar_cost;
    strategies.push(StrategyComparison {
        name: "Collar".into(),
        after_tax_value: collar_atv,
        risk_reduction_pct: risk_reduction,
        liquidity_pct: dec!(0.0), // locked in collar
        cost: collar_cost,
    });

    // 3. Exchange Fund
    strategies.push(StrategyComparison {
        name: "Exchange Fund".into(),
        after_tax_value: pv, // no immediate tax
        risk_reduction_pct: input.exchange_fund_diversification_pct,
        liquidity_pct: dec!(0.0), // 7-year lockup
        cost: pv * dec!(0.01),    // ~1% management cost
    });

    // 4. Prepaid Variable Forward
    let _advance = input.prepaid_forward_advance_pct * pv;
    strategies.push(StrategyComparison {
        name: "Prepaid Variable Forward".into(),
        after_tax_value: pv,           // tax deferred
        risk_reduction_pct: dec!(0.5), // partial -- still own upside/downside beyond forward terms
        liquidity_pct: input.prepaid_forward_advance_pct,
        cost: pv * dec!(0.015), // ~1.5% forward pricing cost
    });

    // 5. Charitable Remainder Trust
    let crt_annual_income = pv * dec!(0.05);
    let crt_income_pv = crt_annual_income * Decimal::from(input.investment_horizon);
    let crt_tax_deduction = pv * dec!(0.30); // ~30% charitable deduction
    let crt_tax_savings = crt_tax_deduction * input.tax_rate_ltcg;
    strategies.push(StrategyComparison {
        name: "Charitable Remainder Trust".into(),
        after_tax_value: crt_income_pv + crt_tax_savings,
        risk_reduction_pct: Decimal::ONE, // fully diversified inside CRT
        liquidity_pct: dec!(0.0),         // income stream, not liquid
        cost: Decimal::ZERO,              // no cost beyond irrevocable donation
    });

    // Recommend: best risk-adjusted after-tax value
    // Score = after_tax_value * (0.6 + 0.2*risk_reduction + 0.2*liquidity)
    let recommended = strategies
        .iter()
        .max_by_key(|s| {
            s.after_tax_value
                * (dec!(0.6) + dec!(0.2) * s.risk_reduction_pct + dec!(0.2) * s.liquidity_pct)
        })
        .map(|s| s.name.clone())
        .unwrap_or_else(|| "Outright Sale".into());

    Ok(ConcentratedStockOutput {
        unrealized_gain,
        embedded_tax,
        after_tax_value,
        strategies,
        recommended_strategy: recommended,
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn base_input() -> ConcentratedStockInput {
        ConcentratedStockInput {
            position_value: dec!(1_000_000),
            cost_basis: dec!(200_000),
            annual_dividend_yield: dec!(0.02),
            stock_volatility: dec!(0.35),
            risk_free_rate: dec!(0.05),
            investment_horizon: 5,
            tax_rate_ltcg: dec!(0.20),
            tax_rate_stcg: dec!(0.37),
            collar_put_strike_pct: dec!(0.90),
            collar_call_strike_pct: dec!(1.10),
            exchange_fund_diversification_pct: dec!(0.70),
            prepaid_forward_advance_pct: dec!(0.80),
        }
    }

    #[test]
    fn test_unrealized_gain() {
        let out = analyze_concentrated_stock(&base_input()).unwrap();
        assert_eq!(out.unrealized_gain, dec!(800_000));
    }

    #[test]
    fn test_embedded_tax() {
        let out = analyze_concentrated_stock(&base_input()).unwrap();
        assert_eq!(out.embedded_tax, dec!(160_000)); // 800k * 0.20
    }

    #[test]
    fn test_after_tax_value() {
        let out = analyze_concentrated_stock(&base_input()).unwrap();
        assert_eq!(out.after_tax_value, dec!(840_000));
    }

    #[test]
    fn test_five_strategies_returned() {
        let out = analyze_concentrated_stock(&base_input()).unwrap();
        assert_eq!(out.strategies.len(), 5);
    }

    #[test]
    fn test_outright_sale_after_tax() {
        let out = analyze_concentrated_stock(&base_input()).unwrap();
        let sale = out
            .strategies
            .iter()
            .find(|s| s.name == "Outright Sale")
            .unwrap();
        // 1M - 800k * 0.20 = 840k
        assert_eq!(sale.after_tax_value, dec!(840_000));
    }

    #[test]
    fn test_outright_sale_full_liquidity() {
        let out = analyze_concentrated_stock(&base_input()).unwrap();
        let sale = out
            .strategies
            .iter()
            .find(|s| s.name == "Outright Sale")
            .unwrap();
        assert_eq!(sale.liquidity_pct, Decimal::ONE);
    }

    #[test]
    fn test_collar_bounded_value() {
        let out = analyze_concentrated_stock(&base_input()).unwrap();
        let collar = out.strategies.iter().find(|s| s.name == "Collar").unwrap();
        // midpoint of (900k, 1100k) - 2% cost = 1M - 20k = 980k
        assert_eq!(collar.after_tax_value, dec!(980_000));
    }

    #[test]
    fn test_collar_no_liquidity() {
        let out = analyze_concentrated_stock(&base_input()).unwrap();
        let collar = out.strategies.iter().find(|s| s.name == "Collar").unwrap();
        assert_eq!(collar.liquidity_pct, dec!(0.0));
    }

    #[test]
    fn test_exchange_fund_no_tax() {
        let out = analyze_concentrated_stock(&base_input()).unwrap();
        let ef = out
            .strategies
            .iter()
            .find(|s| s.name == "Exchange Fund")
            .unwrap();
        assert_eq!(ef.after_tax_value, dec!(1_000_000)); // no immediate tax
    }

    #[test]
    fn test_exchange_fund_lockup() {
        let out = analyze_concentrated_stock(&base_input()).unwrap();
        let ef = out
            .strategies
            .iter()
            .find(|s| s.name == "Exchange Fund")
            .unwrap();
        assert_eq!(ef.liquidity_pct, dec!(0.0));
    }

    #[test]
    fn test_prepaid_forward_liquidity() {
        let out = analyze_concentrated_stock(&base_input()).unwrap();
        let pvf = out
            .strategies
            .iter()
            .find(|s| s.name == "Prepaid Variable Forward")
            .unwrap();
        assert_eq!(pvf.liquidity_pct, dec!(0.80));
    }

    #[test]
    fn test_crt_income_stream() {
        let out = analyze_concentrated_stock(&base_input()).unwrap();
        let crt = out
            .strategies
            .iter()
            .find(|s| s.name == "Charitable Remainder Trust")
            .unwrap();
        // income = 1M * 0.05 * 5 = 250k, deduction savings = 1M*0.30*0.20 = 60k
        assert_eq!(crt.after_tax_value, dec!(310_000));
    }

    #[test]
    fn test_crt_full_risk_reduction() {
        let out = analyze_concentrated_stock(&base_input()).unwrap();
        let crt = out
            .strategies
            .iter()
            .find(|s| s.name == "Charitable Remainder Trust")
            .unwrap();
        assert_eq!(crt.risk_reduction_pct, Decimal::ONE);
    }

    #[test]
    fn test_zero_gain() {
        let mut inp = base_input();
        inp.cost_basis = dec!(1_000_000);
        let out = analyze_concentrated_stock(&inp).unwrap();
        assert_eq!(out.unrealized_gain, Decimal::ZERO);
        assert_eq!(out.embedded_tax, Decimal::ZERO);
    }

    #[test]
    fn test_loss_position() {
        let mut inp = base_input();
        inp.cost_basis = dec!(1_500_000);
        let out = analyze_concentrated_stock(&inp).unwrap();
        assert_eq!(out.unrealized_gain, dec!(-500_000));
        assert_eq!(out.embedded_tax, Decimal::ZERO); // no tax on unrealized loss
    }

    #[test]
    fn test_recommended_strategy_is_valid() {
        let out = analyze_concentrated_stock(&base_input()).unwrap();
        let names: Vec<&str> = out.strategies.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&out.recommended_strategy.as_str()));
    }

    #[test]
    fn test_invalid_position_value() {
        let mut inp = base_input();
        inp.position_value = dec!(-100);
        assert!(analyze_concentrated_stock(&inp).is_err());
    }

    #[test]
    fn test_invalid_tax_rate() {
        let mut inp = base_input();
        inp.tax_rate_ltcg = dec!(1.5);
        assert!(analyze_concentrated_stock(&inp).is_err());
    }

    #[test]
    fn test_invalid_collar_put() {
        let mut inp = base_input();
        inp.collar_put_strike_pct = dec!(1.5);
        assert!(analyze_concentrated_stock(&inp).is_err());
    }

    #[test]
    fn test_invalid_collar_call() {
        let mut inp = base_input();
        inp.collar_call_strike_pct = dec!(0.5);
        assert!(analyze_concentrated_stock(&inp).is_err());
    }

    #[test]
    fn test_invalid_horizon() {
        let mut inp = base_input();
        inp.investment_horizon = 0;
        assert!(analyze_concentrated_stock(&inp).is_err());
    }

    #[test]
    fn test_high_volatility_position() {
        let mut inp = base_input();
        inp.stock_volatility = dec!(0.80);
        let out = analyze_concentrated_stock(&inp).unwrap();
        assert_eq!(out.strategies.len(), 5);
    }

    #[test]
    fn test_collar_risk_reduction_positive() {
        let out = analyze_concentrated_stock(&base_input()).unwrap();
        let collar = out.strategies.iter().find(|s| s.name == "Collar").unwrap();
        assert!(collar.risk_reduction_pct >= Decimal::ZERO);
        assert!(collar.risk_reduction_pct <= Decimal::ONE);
    }

    #[test]
    fn test_outright_sale_cost_equals_embedded_tax() {
        let out = analyze_concentrated_stock(&base_input()).unwrap();
        let sale = out
            .strategies
            .iter()
            .find(|s| s.name == "Outright Sale")
            .unwrap();
        assert_eq!(sale.cost, out.embedded_tax);
    }
}
