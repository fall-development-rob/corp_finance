//! Emerging-market equity risk premium (ERP) decomposition.
//!
//! Implements:
//! 1. **Historical ERP** -- local_return - risk_free
//! 2. **Relative ERP** -- local_return - us_return
//! 3. **Implied ERP** -- Gordon growth model: div_yield + earnings_growth - rf
//! 4. **Damodaran ERP** -- US ERP + CRP (sovereign_spread x lambda)
//! 5. **Supply-side ERP** -- dividend_yield + real_earnings_growth + repricing
//! 6. **Consensus ERP** -- average of all methods
//! 7. **Valuation indicator** -- Buffett indicator and PE-based
//! 8. **Decomposition** -- DM premium, country risk, currency risk, liquidity
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

/// Input for EM equity premium decomposition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmEquityPremiumInput {
    /// Historical annualised local market return (e.g. 0.12 = 12%).
    pub local_market_return: Decimal,
    /// US market annualised return.
    pub us_market_return: Decimal,
    /// Risk-free rate.
    pub risk_free_rate: Decimal,
    /// Sovereign spread over US Treasuries.
    pub sovereign_spread: Decimal,
    /// Local equity market volatility.
    pub equity_vol_local: Decimal,
    /// US equity market volatility.
    pub equity_vol_us: Decimal,
    /// Local bond market volatility.
    pub bond_vol_local: Decimal,
    /// Market cap to GDP ratio (Buffett indicator).
    pub market_cap_to_gdp: Decimal,
    /// Local market P/E ratio.
    pub pe_ratio: Decimal,
    /// Local market dividend yield.
    pub dividend_yield: Decimal,
    /// Real GDP growth rate.
    pub gdp_growth: Decimal,
    /// Nominal earnings growth rate.
    pub earnings_growth: Decimal,
    /// FX volatility vs USD (for currency risk component).
    pub fx_volatility: Decimal,
}

/// Decomposition of the equity risk premium into components.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErpDecomposition {
    /// Developed-market ERP component (US ERP).
    pub dm_premium: Decimal,
    /// Country risk premium component.
    pub country_risk: Decimal,
    /// Currency risk component.
    pub currency_risk: Decimal,
    /// Liquidity / size premium.
    pub liquidity_premium: Decimal,
}

/// Output from EM equity premium analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmEquityPremiumOutput {
    /// Historical ERP = local_return - risk_free.
    pub historical_erp: Decimal,
    /// Relative ERP = local_return - us_return.
    pub relative_erp: Decimal,
    /// Implied ERP (Gordon model) = div_yield + earnings_growth - rf.
    pub implied_erp: Decimal,
    /// Damodaran ERP = US ERP + CRP.
    pub damodaran_erp: Decimal,
    /// Lambda = equity_vol / bond_vol.
    pub lambda: Decimal,
    /// Supply-side ERP = div_yield + real_earnings_growth + repricing.
    pub supply_side_erp: Decimal,
    /// Average of all ERP methods.
    pub consensus_erp: Decimal,
    /// Valuation indicator based on Buffett ratio and PE.
    pub valuation_indicator: String,
    /// Decomposition of total ERP.
    pub decomposition: ErpDecomposition,
}

// ---------------------------------------------------------------------------
// Main function
// ---------------------------------------------------------------------------

/// Calculate EM equity risk premium using multiple methods with decomposition.
pub fn calculate_em_equity_premium(
    input: &EmEquityPremiumInput,
) -> CorpFinanceResult<EmEquityPremiumOutput> {
    // Validation
    if input.bond_vol_local <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "bond_vol_local".to_string(),
            reason: "Bond volatility must be positive".to_string(),
        });
    }
    if input.equity_vol_local < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "equity_vol_local".to_string(),
            reason: "Equity volatility cannot be negative".to_string(),
        });
    }
    if input.pe_ratio <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "pe_ratio".to_string(),
            reason: "P/E ratio must be positive".to_string(),
        });
    }
    if input.market_cap_to_gdp < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "market_cap_to_gdp".to_string(),
            reason: "Market cap to GDP ratio cannot be negative".to_string(),
        });
    }
    if input.dividend_yield < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "dividend_yield".to_string(),
            reason: "Dividend yield cannot be negative".to_string(),
        });
    }
    if input.fx_volatility < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "fx_volatility".to_string(),
            reason: "FX volatility cannot be negative".to_string(),
        });
    }
    if input.equity_vol_us < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "equity_vol_us".to_string(),
            reason: "US equity volatility cannot be negative".to_string(),
        });
    }

    // 1. Historical ERP
    let historical_erp = input.local_market_return - input.risk_free_rate;

    // 2. Relative ERP
    let relative_erp = input.local_market_return - input.us_market_return;

    // 3. Implied ERP (Gordon growth model)
    let implied_erp = input.dividend_yield + input.earnings_growth - input.risk_free_rate;

    // 4. Damodaran ERP
    let lambda = input.equity_vol_local / input.bond_vol_local;
    let us_erp = input.us_market_return - input.risk_free_rate;
    let crp = input.sovereign_spread * lambda;
    let damodaran_erp = us_erp + crp;

    // 5. Supply-side ERP
    // Real earnings growth ~ earnings_growth - inflation (approximate with rf as proxy)
    let real_earnings_growth = input.earnings_growth - input.risk_free_rate;
    // Repricing component: based on PE being above/below historical average (~15)
    let repricing = if input.pe_ratio > dec!(15) {
        dec!(-0.005) // PE compression expected
    } else if input.pe_ratio < dec!(10) {
        dec!(0.010) // PE expansion expected
    } else {
        Decimal::ZERO
    };
    let supply_side_erp = input.dividend_yield + real_earnings_growth + repricing;

    // 6. Consensus ERP (average of 5 methods)
    let five = dec!(5);
    let consensus_erp =
        (historical_erp + implied_erp + damodaran_erp + supply_side_erp + relative_erp) / five;

    // 7. Valuation indicator
    let buffett_indicator = if input.market_cap_to_gdp < dec!(0.8) {
        "Undervalued"
    } else if input.market_cap_to_gdp <= dec!(1.2) {
        "Fair"
    } else {
        "Overvalued"
    };
    let pe_indicator = if input.pe_ratio < dec!(10) {
        "Cheap"
    } else if input.pe_ratio <= dec!(20) {
        "Fair"
    } else {
        "Expensive"
    };
    let valuation_indicator = format!("Buffett: {}, PE: {}", buffett_indicator, pe_indicator);

    // 8. Decomposition
    let dm_premium = us_erp;
    let country_risk = crp;
    let currency_risk = input.fx_volatility * dec!(0.5);
    let liquidity_premium = if input.market_cap_to_gdp < dec!(0.5) {
        dec!(0.02)
    } else if input.market_cap_to_gdp < dec!(1.0) {
        dec!(0.01)
    } else {
        Decimal::ZERO
    };

    let decomposition = ErpDecomposition {
        dm_premium,
        country_risk,
        currency_risk,
        liquidity_premium,
    };

    Ok(EmEquityPremiumOutput {
        historical_erp,
        relative_erp,
        implied_erp,
        damodaran_erp,
        lambda,
        supply_side_erp,
        consensus_erp,
        valuation_indicator,
        decomposition,
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn base_input() -> EmEquityPremiumInput {
        EmEquityPremiumInput {
            local_market_return: dec!(0.12),
            us_market_return: dec!(0.10),
            risk_free_rate: dec!(0.04),
            sovereign_spread: dec!(0.03),
            equity_vol_local: dec!(0.25),
            equity_vol_us: dec!(0.16),
            bond_vol_local: dec!(0.10),
            market_cap_to_gdp: dec!(0.70),
            pe_ratio: dec!(12),
            dividend_yield: dec!(0.03),
            gdp_growth: dec!(0.05),
            earnings_growth: dec!(0.08),
            fx_volatility: dec!(0.12),
        }
    }

    #[test]
    fn test_historical_erp() {
        let input = base_input();
        let out = calculate_em_equity_premium(&input).unwrap();
        // 0.12 - 0.04 = 0.08
        assert_eq!(out.historical_erp, dec!(0.08));
    }

    #[test]
    fn test_relative_erp() {
        let input = base_input();
        let out = calculate_em_equity_premium(&input).unwrap();
        // 0.12 - 0.10 = 0.02
        assert_eq!(out.relative_erp, dec!(0.02));
    }

    #[test]
    fn test_implied_erp_gordon() {
        let input = base_input();
        let out = calculate_em_equity_premium(&input).unwrap();
        // 0.03 + 0.08 - 0.04 = 0.07
        assert_eq!(out.implied_erp, dec!(0.07));
    }

    #[test]
    fn test_damodaran_erp() {
        let input = base_input();
        let out = calculate_em_equity_premium(&input).unwrap();
        // lambda = 0.25/0.10 = 2.5
        // us_erp = 0.10 - 0.04 = 0.06
        // crp = 0.03 * 2.5 = 0.075
        // damodaran = 0.06 + 0.075 = 0.135
        assert_eq!(out.lambda, dec!(2.5));
        assert_eq!(out.damodaran_erp, dec!(0.135));
    }

    #[test]
    fn test_lambda() {
        let input = base_input();
        let out = calculate_em_equity_premium(&input).unwrap();
        assert_eq!(out.lambda, dec!(2.5));
    }

    #[test]
    fn test_supply_side_erp() {
        let input = base_input();
        let out = calculate_em_equity_premium(&input).unwrap();
        // real_earnings = 0.08 - 0.04 = 0.04
        // PE=12 (between 10 and 15) -> repricing = 0
        // supply = 0.03 + 0.04 + 0 = 0.07
        assert_eq!(out.supply_side_erp, dec!(0.07));
    }

    #[test]
    fn test_consensus_erp_is_average() {
        let input = base_input();
        let out = calculate_em_equity_premium(&input).unwrap();
        let sum = out.historical_erp
            + out.implied_erp
            + out.damodaran_erp
            + out.supply_side_erp
            + out.relative_erp;
        assert_eq!(out.consensus_erp, sum / dec!(5));
    }

    #[test]
    fn test_valuation_undervalued() {
        let mut input = base_input();
        input.market_cap_to_gdp = dec!(0.50);
        input.pe_ratio = dec!(8);
        let out = calculate_em_equity_premium(&input).unwrap();
        assert!(out.valuation_indicator.contains("Undervalued"));
        assert!(out.valuation_indicator.contains("Cheap"));
    }

    #[test]
    fn test_valuation_overvalued() {
        let mut input = base_input();
        input.market_cap_to_gdp = dec!(1.5);
        input.pe_ratio = dec!(25);
        let out = calculate_em_equity_premium(&input).unwrap();
        assert!(out.valuation_indicator.contains("Overvalued"));
        assert!(out.valuation_indicator.contains("Expensive"));
    }

    #[test]
    fn test_valuation_fair() {
        let mut input = base_input();
        input.market_cap_to_gdp = dec!(1.0);
        input.pe_ratio = dec!(15);
        let out = calculate_em_equity_premium(&input).unwrap();
        assert!(out.valuation_indicator.contains("Buffett: Fair"));
        assert!(out.valuation_indicator.contains("PE: Fair"));
    }

    #[test]
    fn test_decomposition_dm_premium() {
        let input = base_input();
        let out = calculate_em_equity_premium(&input).unwrap();
        // us_erp = 0.10 - 0.04 = 0.06
        assert_eq!(out.decomposition.dm_premium, dec!(0.06));
    }

    #[test]
    fn test_decomposition_country_risk() {
        let input = base_input();
        let out = calculate_em_equity_premium(&input).unwrap();
        // crp = 0.03 * 2.5 = 0.075
        assert_eq!(out.decomposition.country_risk, dec!(0.075));
    }

    #[test]
    fn test_decomposition_currency_risk() {
        let input = base_input();
        let out = calculate_em_equity_premium(&input).unwrap();
        // 0.12 * 0.5 = 0.06
        assert_eq!(out.decomposition.currency_risk, dec!(0.06));
    }

    #[test]
    fn test_liquidity_premium_small_market() {
        let mut input = base_input();
        input.market_cap_to_gdp = dec!(0.30);
        let out = calculate_em_equity_premium(&input).unwrap();
        assert_eq!(out.decomposition.liquidity_premium, dec!(0.02));
    }

    #[test]
    fn test_liquidity_premium_medium_market() {
        let mut input = base_input();
        input.market_cap_to_gdp = dec!(0.70);
        let out = calculate_em_equity_premium(&input).unwrap();
        assert_eq!(out.decomposition.liquidity_premium, dec!(0.01));
    }

    #[test]
    fn test_liquidity_premium_large_market() {
        let mut input = base_input();
        input.market_cap_to_gdp = dec!(1.5);
        let out = calculate_em_equity_premium(&input).unwrap();
        assert_eq!(out.decomposition.liquidity_premium, Decimal::ZERO);
    }

    #[test]
    fn test_high_growth_em() {
        let mut input = base_input();
        input.local_market_return = dec!(0.18);
        input.earnings_growth = dec!(0.15);
        input.gdp_growth = dec!(0.08);
        let out = calculate_em_equity_premium(&input).unwrap();
        assert!(out.historical_erp > dec!(0.10));
        assert!(out.implied_erp > dec!(0.10));
    }

    #[test]
    fn test_frontier_small_lambda() {
        let mut input = base_input();
        input.equity_vol_local = dec!(0.10);
        input.bond_vol_local = dec!(0.10);
        let out = calculate_em_equity_premium(&input).unwrap();
        assert_eq!(out.lambda, Decimal::ONE);
        // crp = spread * 1 = 0.03
        assert_eq!(out.decomposition.country_risk, dec!(0.03));
    }

    #[test]
    fn test_large_lambda() {
        let mut input = base_input();
        input.equity_vol_local = dec!(0.40);
        input.bond_vol_local = dec!(0.08);
        let out = calculate_em_equity_premium(&input).unwrap();
        assert_eq!(out.lambda, dec!(5));
    }

    #[test]
    fn test_invalid_zero_bond_vol() {
        let mut input = base_input();
        input.bond_vol_local = Decimal::ZERO;
        let err = calculate_em_equity_premium(&input).unwrap_err();
        assert!(matches!(err, CorpFinanceError::InvalidInput { .. }));
    }

    #[test]
    fn test_invalid_negative_pe() {
        let mut input = base_input();
        input.pe_ratio = dec!(-5);
        let err = calculate_em_equity_premium(&input).unwrap_err();
        assert!(matches!(err, CorpFinanceError::InvalidInput { .. }));
    }

    #[test]
    fn test_invalid_negative_dividend_yield() {
        let mut input = base_input();
        input.dividend_yield = dec!(-0.01);
        let err = calculate_em_equity_premium(&input).unwrap_err();
        assert!(matches!(err, CorpFinanceError::InvalidInput { .. }));
    }

    #[test]
    fn test_pe_compression_repricing() {
        let mut input = base_input();
        input.pe_ratio = dec!(20); // above 15
        let out = calculate_em_equity_premium(&input).unwrap();
        // supply_side should include -0.005 repricing
        let real_eg = input.earnings_growth - input.risk_free_rate;
        let expected = input.dividend_yield + real_eg + dec!(-0.005);
        assert_eq!(out.supply_side_erp, expected);
    }

    #[test]
    fn test_pe_expansion_repricing() {
        let mut input = base_input();
        input.pe_ratio = dec!(8); // below 10
        let out = calculate_em_equity_premium(&input).unwrap();
        let real_eg = input.earnings_growth - input.risk_free_rate;
        let expected = input.dividend_yield + real_eg + dec!(0.010);
        assert_eq!(out.supply_side_erp, expected);
    }

    #[test]
    fn test_invalid_negative_market_cap_gdp() {
        let mut input = base_input();
        input.market_cap_to_gdp = dec!(-0.5);
        let err = calculate_em_equity_premium(&input).unwrap_err();
        assert!(matches!(err, CorpFinanceError::InvalidInput { .. }));
    }
}
