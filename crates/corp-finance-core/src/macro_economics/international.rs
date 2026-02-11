//! International economics analysis.
//!
//! Provides Purchasing Power Parity (PPP), Covered and Uncovered Interest Rate
//! Parity (CIP/UIP), real exchange rate analysis, balance of payments
//! assessment, and multi-year exchange rate projections.
//!
//! All calculations use `rust_decimal::Decimal` for precision. No `f64`.

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::{CorpFinanceError, CorpFinanceResult};

// ---------------------------------------------------------------------------
// Input / Output types
// ---------------------------------------------------------------------------

/// Input for international economics analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InternationalInput {
    /// Domestic country name.
    pub domestic_country: String,
    /// Foreign country name.
    pub foreign_country: String,
    /// Spot exchange rate: domestic per foreign (e.g., 1.25 USD/EUR).
    pub spot_exchange_rate: Decimal,
    /// Domestic inflation rate.
    pub domestic_inflation: Decimal,
    /// Foreign inflation rate.
    pub foreign_inflation: Decimal,
    /// Domestic nominal interest rate.
    pub domestic_interest_rate: Decimal,
    /// Foreign nominal interest rate.
    pub foreign_interest_rate: Decimal,
    /// Domestic real GDP growth rate.
    pub domestic_gdp_growth: Decimal,
    /// Foreign real GDP growth rate.
    pub foreign_gdp_growth: Decimal,
    /// Observed forward exchange rate (optional, for CIP arbitrage check).
    pub forward_exchange_rate: Option<Decimal>,
    /// PPP-implied "fair value" base rate (optional).
    pub ppp_base_rate: Option<Decimal>,
    /// Domestic current account balance as % of GDP.
    pub current_account_pct_gdp: Decimal,
    /// Projection horizon in years (>= 1).
    pub years_forward: u32,
}

/// Purchasing Power Parity result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PppResult {
    /// PPP-implied exchange rate.
    pub ppp_implied_rate: Decimal,
    /// Current spot rate.
    pub current_rate: Decimal,
    /// Misalignment: positive = domestic currency overvalued.
    pub misalignment_pct: Decimal,
    /// Narrative interpretation.
    pub big_mac_equivalent: String,
    /// Expected annual reversion toward PPP (default 15%).
    pub reversion_rate_annual: Decimal,
}

/// Interest Rate Parity result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterestParityResult {
    /// CIP-implied forward rate.
    pub covered_forward: Decimal,
    /// UIP-implied expected future spot rate.
    pub uncovered_expected: Decimal,
    /// Annualised forward premium/discount.
    pub forward_premium_pct: Decimal,
    /// Carry trade return (borrow low, invest high).
    pub carry_trade_return: Decimal,
    /// Carry trade risk assessment.
    pub carry_trade_risk: String,
}

/// Real exchange rate result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealExchangeResult {
    /// Nominal exchange rate.
    pub nominal_rate: Decimal,
    /// Real exchange rate adjusted for price levels.
    pub real_rate: Decimal,
    /// Real appreciation percentage.
    pub real_appreciation_pct: Decimal,
    /// Terms of trade proxy.
    pub terms_of_trade_proxy: Decimal,
}

/// Balance of payments result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceOfPaymentsResult {
    /// Current account as % of GDP.
    pub current_account_pct: Decimal,
    /// "Sustainable", "Watch", or "Unsustainable".
    pub sustainability_assessment: String,
    /// "Appreciation", "Neutral", or "Depreciation".
    pub implied_fx_pressure: String,
    /// Whether twin deficit risk is present.
    pub twin_deficit_risk: bool,
}

/// Full international economics output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InternationalOutput {
    /// PPP analysis.
    pub purchasing_power_parity: PppResult,
    /// Interest rate parity analysis.
    pub interest_rate_parity: InterestParityResult,
    /// Real exchange rate analysis.
    pub real_exchange_rate: RealExchangeResult,
    /// Balance of payments analysis.
    pub balance_of_payments: BalanceOfPaymentsResult,
    /// Projected exchange rates: (year, rate).
    pub projected_rates: Vec<(u32, Decimal)>,
    /// Methodology description.
    pub methodology: String,
    /// Key assumptions.
    pub assumptions: HashMap<String, String>,
    /// Warnings.
    pub warnings: Vec<String>,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Analyse international economics: PPP, interest rate parity, real exchange
/// rates, balance of payments, and multi-year rate projections.
pub fn analyze_international(input: &InternationalInput) -> CorpFinanceResult<InternationalOutput> {
    let warnings = validate_input(input)?;

    let ppp = compute_ppp(input);
    let irp = compute_interest_parity(input);
    let rer = compute_real_exchange_rate(input);
    let bop = compute_balance_of_payments(input);
    let projected_rates = compute_projections(input, &ppp, &irp);

    let mut assumptions = HashMap::new();
    assumptions.insert(
        "ppp_reversion".into(),
        "15% annual reversion toward PPP".into(),
    );
    assumptions.insert(
        "projection_weights".into(),
        "50% PPP path + 50% UIP path".into(),
    );
    assumptions.insert(
        "ca_sustainability".into(),
        "Sustainable < 3%, Watch 3-5%, Unsustainable > 5%".into(),
    );

    Ok(InternationalOutput {
        purchasing_power_parity: ppp,
        interest_rate_parity: irp,
        real_exchange_rate: rer,
        balance_of_payments: bop,
        projected_rates,
        methodology: "Relative PPP for fair-value estimation, CIP/UIP for forward rates, \
                       weighted blend (50/50 PPP reversion + UIP) for projections. \
                       Balance of payments sustainability thresholds at 3% and 5% of GDP."
            .into(),
        assumptions,
        warnings,
    })
}

// ---------------------------------------------------------------------------
// Helpers: integer power for Decimal
// ---------------------------------------------------------------------------

/// Compute base^exp via iterative multiplication (exp must be non-negative).
fn decimal_pow(base: Decimal, exp: u32) -> Decimal {
    let mut result = Decimal::ONE;
    for _ in 0..exp {
        result *= base;
    }
    result
}

// ---------------------------------------------------------------------------
// Internal: PPP
// ---------------------------------------------------------------------------

fn compute_ppp(input: &InternationalInput) -> PppResult {
    let reversion_rate_annual = dec!(0.15);

    // PPP-implied rate
    let ppp_implied_rate = if let Some(base) = input.ppp_base_rate {
        // Multi-year relative PPP from a base rate
        let domestic_factor =
            decimal_pow(Decimal::ONE + input.domestic_inflation, input.years_forward);
        let foreign_factor =
            decimal_pow(Decimal::ONE + input.foreign_inflation, input.years_forward);
        if foreign_factor.is_zero() {
            base
        } else {
            base * domestic_factor / foreign_factor
        }
    } else {
        // One-period relative PPP
        let domestic_factor = Decimal::ONE + input.domestic_inflation;
        let foreign_factor = Decimal::ONE + input.foreign_inflation;
        if foreign_factor.is_zero() {
            input.spot_exchange_rate
        } else {
            input.spot_exchange_rate * domestic_factor / foreign_factor
        }
    };

    // Misalignment = (spot - ppp) / ppp * 100
    let misalignment_pct = if ppp_implied_rate.is_zero() {
        Decimal::ZERO
    } else {
        (input.spot_exchange_rate - ppp_implied_rate) / ppp_implied_rate * dec!(100)
    };

    // Narrative
    let big_mac_equivalent = if misalignment_pct > dec!(5) {
        format!(
            "The {} appears overvalued by {:.1}% relative to PPP fair value against the {}.",
            input.domestic_country, misalignment_pct, input.foreign_country
        )
    } else if misalignment_pct < dec!(-5) {
        let abs_mis = -misalignment_pct;
        format!(
            "The {} appears undervalued by {:.1}% relative to PPP fair value against the {}.",
            input.domestic_country, abs_mis, input.foreign_country
        )
    } else {
        format!(
            "The {}/{} rate is broadly aligned with PPP fair value (misalignment: {:.1}%).",
            input.domestic_country, input.foreign_country, misalignment_pct
        )
    };

    PppResult {
        ppp_implied_rate,
        current_rate: input.spot_exchange_rate,
        misalignment_pct,
        big_mac_equivalent,
        reversion_rate_annual,
    }
}

// ---------------------------------------------------------------------------
// Internal: Interest Rate Parity
// ---------------------------------------------------------------------------

fn compute_interest_parity(input: &InternationalInput) -> InterestParityResult {
    let t = Decimal::from(input.years_forward);

    // CIP: F = S * (1 + r_dom)^T / (1 + r_for)^T
    let dom_factor = decimal_pow(
        Decimal::ONE + input.domestic_interest_rate,
        input.years_forward,
    );
    let for_factor = decimal_pow(
        Decimal::ONE + input.foreign_interest_rate,
        input.years_forward,
    );

    let covered_forward = if for_factor.is_zero() {
        input.spot_exchange_rate
    } else {
        input.spot_exchange_rate * dom_factor / for_factor
    };

    // UIP: E(S_T) = same formula as CIP (assuming risk-neutral)
    let uncovered_expected = covered_forward;

    // Forward premium = (F - S) / S * (1/T) annualised
    let forward_premium_pct = if input.spot_exchange_rate.is_zero() || t.is_zero() {
        Decimal::ZERO
    } else {
        (covered_forward - input.spot_exchange_rate) / input.spot_exchange_rate / t
    };

    // Carry trade: borrow in low-rate currency, invest in high-rate
    // Return = r_foreign - r_domestic + expected depreciation
    // Under UIP, expected depreciation offsets the differential, so carry = 0 in theory.
    // But in practice, UIP often fails, so we compute the raw differential.
    let carry_trade_return = input.foreign_interest_rate - input.domestic_interest_rate;

    // Carry trade risk based on rate differential magnitude
    let abs_diff = if carry_trade_return < Decimal::ZERO {
        -carry_trade_return
    } else {
        carry_trade_return
    };
    let carry_trade_risk = if abs_diff > dec!(0.05) {
        "High".to_string()
    } else if abs_diff > dec!(0.02) {
        "Moderate".to_string()
    } else {
        "Low".to_string()
    };

    InterestParityResult {
        covered_forward,
        uncovered_expected,
        forward_premium_pct,
        carry_trade_return,
        carry_trade_risk,
    }
}

// ---------------------------------------------------------------------------
// Internal: Real Exchange Rate
// ---------------------------------------------------------------------------

fn compute_real_exchange_rate(input: &InternationalInput) -> RealExchangeResult {
    // RER = NER * (P_foreign / P_domestic)
    // Using inflation as proxy for price level changes:
    // P_foreign = 1 + foreign_inflation, P_domestic = 1 + domestic_inflation
    let p_foreign = Decimal::ONE + input.foreign_inflation;
    let p_domestic = Decimal::ONE + input.domestic_inflation;

    let real_rate = if p_domestic.is_zero() {
        input.spot_exchange_rate
    } else {
        input.spot_exchange_rate * p_foreign / p_domestic
    };

    // Real appreciation = (NER - RER) / RER * 100
    // Positive = domestic currency has appreciated in real terms
    let real_appreciation_pct = if real_rate.is_zero() {
        Decimal::ZERO
    } else {
        (input.spot_exchange_rate - real_rate) / real_rate * dec!(100)
    };

    // Terms of trade proxy: ratio of domestic to foreign inflation
    // Higher domestic inflation implies worsening terms of trade
    let terms_of_trade_proxy = if p_foreign.is_zero() {
        Decimal::ONE
    } else {
        p_domestic / p_foreign
    };

    RealExchangeResult {
        nominal_rate: input.spot_exchange_rate,
        real_rate,
        real_appreciation_pct,
        terms_of_trade_proxy,
    }
}

// ---------------------------------------------------------------------------
// Internal: Balance of Payments
// ---------------------------------------------------------------------------

fn compute_balance_of_payments(input: &InternationalInput) -> BalanceOfPaymentsResult {
    let ca = input.current_account_pct_gdp;
    let abs_ca = if ca < Decimal::ZERO { -ca } else { ca };

    let sustainability_assessment = if abs_ca > dec!(5) {
        "Unsustainable".to_string()
    } else if abs_ca > dec!(3) {
        "Watch".to_string()
    } else {
        "Sustainable".to_string()
    };

    let implied_fx_pressure = if ca > dec!(3) {
        "Appreciation".to_string()
    } else if ca < dec!(-3) {
        "Depreciation".to_string()
    } else {
        "Neutral".to_string()
    };

    // Twin deficit: current account deficit AND growth below potential (fiscal proxy)
    // We use: CA < -2% AND domestic GDP growth below foreign GDP growth as a proxy
    let twin_deficit_risk = ca < dec!(-2) && input.domestic_gdp_growth < input.foreign_gdp_growth;

    BalanceOfPaymentsResult {
        current_account_pct: ca,
        sustainability_assessment,
        implied_fx_pressure,
        twin_deficit_risk,
    }
}

// ---------------------------------------------------------------------------
// Internal: Projections
// ---------------------------------------------------------------------------

fn compute_projections(
    input: &InternationalInput,
    ppp: &PppResult,
    _irp: &InterestParityResult,
) -> Vec<(u32, Decimal)> {
    let mut projections = Vec::with_capacity(input.years_forward as usize);
    let reversion = ppp.reversion_rate_annual;

    for year in 1..=input.years_forward {
        // PPP path: spot moves toward PPP at 15% annual reversion
        // ppp_rate_t = spot + reversion_rate * (ppp_implied - spot) * year
        // More precisely: each year closes 15% of remaining gap
        let ppp_gap = ppp.ppp_implied_rate - input.spot_exchange_rate;
        let ppp_path = input.spot_exchange_rate
            + ppp_gap * (Decimal::ONE - decimal_pow(Decimal::ONE - reversion, year));

        // UIP path: S_t = S * ((1+r_dom)/(1+r_for))^t
        let dom_factor = decimal_pow(Decimal::ONE + input.domestic_interest_rate, year);
        let for_factor = decimal_pow(Decimal::ONE + input.foreign_interest_rate, year);
        let uip_path = if for_factor.is_zero() {
            input.spot_exchange_rate
        } else {
            input.spot_exchange_rate * dom_factor / for_factor
        };

        // Blended: 50% PPP + 50% UIP
        let blended = dec!(0.5) * ppp_path + dec!(0.5) * uip_path;
        projections.push((year, blended));
    }

    projections
}

// ---------------------------------------------------------------------------
// Internal: validation
// ---------------------------------------------------------------------------

fn validate_input(input: &InternationalInput) -> CorpFinanceResult<Vec<String>> {
    let mut warnings = Vec::new();

    if input.spot_exchange_rate <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "spot_exchange_rate".into(),
            reason: "Spot exchange rate must be positive.".into(),
        });
    }
    if input.years_forward < 1 {
        return Err(CorpFinanceError::InvalidInput {
            field: "years_forward".into(),
            reason: "Projection horizon must be at least 1 year.".into(),
        });
    }

    // Interest rate range checks
    if input.domestic_interest_rate < dec!(-0.05) || input.domestic_interest_rate > dec!(0.50) {
        warnings.push(format!(
            "Domestic interest rate {} is outside typical range [-5%, 50%].",
            input.domestic_interest_rate
        ));
    }
    if input.foreign_interest_rate < dec!(-0.05) || input.foreign_interest_rate > dec!(0.50) {
        warnings.push(format!(
            "Foreign interest rate {} is outside typical range [-5%, 50%].",
            input.foreign_interest_rate
        ));
    }

    // Inflation range checks
    if input.domestic_inflation < dec!(-0.10) || input.domestic_inflation > dec!(1.0) {
        warnings.push(format!(
            "Domestic inflation {} is outside typical range [-10%, 100%].",
            input.domestic_inflation
        ));
    }
    if input.foreign_inflation < dec!(-0.10) || input.foreign_inflation > dec!(1.0) {
        warnings.push(format!(
            "Foreign inflation {} is outside typical range [-10%, 100%].",
            input.foreign_inflation
        ));
    }

    Ok(warnings)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    // -- Test helpers --------------------------------------------------------

    /// Standard USD/EUR scenario.
    fn usd_eur_input() -> InternationalInput {
        InternationalInput {
            domestic_country: "US".into(),
            foreign_country: "Eurozone".into(),
            spot_exchange_rate: dec!(1.10),       // 1.10 USD per EUR
            domestic_inflation: dec!(0.035),      // 3.5% US inflation
            foreign_inflation: dec!(0.025),       // 2.5% Eurozone inflation
            domestic_interest_rate: dec!(0.0525), // 5.25% Fed rate
            foreign_interest_rate: dec!(0.04),    // 4% ECB rate
            domestic_gdp_growth: dec!(0.025),
            foreign_gdp_growth: dec!(0.01),
            forward_exchange_rate: Some(dec!(1.12)),
            ppp_base_rate: None,
            current_account_pct_gdp: dec!(-3.5), // US current account deficit
            years_forward: 1,
        }
    }

    /// Emerging market scenario (higher rates, bigger differentials).
    fn em_input() -> InternationalInput {
        InternationalInput {
            domestic_country: "Brazil".into(),
            foreign_country: "US".into(),
            spot_exchange_rate: dec!(5.0),        // 5 BRL per USD
            domestic_inflation: dec!(0.06),       // 6%
            foreign_inflation: dec!(0.035),       // 3.5%
            domestic_interest_rate: dec!(0.1375), // 13.75% Selic
            foreign_interest_rate: dec!(0.0525),  // 5.25% Fed
            domestic_gdp_growth: dec!(0.015),
            foreign_gdp_growth: dec!(0.025),
            forward_exchange_rate: None,
            ppp_base_rate: Some(dec!(4.0)),
            current_account_pct_gdp: dec!(-2.5),
            years_forward: 5,
        }
    }

    // -- PPP tests -----------------------------------------------------------

    #[test]
    fn test_ppp_relative_no_base_rate() {
        let input = usd_eur_input();
        let result = analyze_international(&input).unwrap();
        // PPP = 1.10 * (1.035) / (1.025) = 1.10 * 1.0097... = 1.1107...
        let expected = dec!(1.10) * dec!(1.035) / dec!(1.025);
        assert_eq!(result.purchasing_power_parity.ppp_implied_rate, expected);
    }

    #[test]
    fn test_ppp_with_base_rate() {
        let input = em_input();
        let result = analyze_international(&input).unwrap();
        // PPP = 4.0 * (1.06)^5 / (1.035)^5
        let dom_factor = decimal_pow(dec!(1.06), 5);
        let for_factor = decimal_pow(dec!(1.035), 5);
        let expected = dec!(4.0) * dom_factor / for_factor;
        assert_eq!(result.purchasing_power_parity.ppp_implied_rate, expected);
    }

    #[test]
    fn test_ppp_overvaluation_detection() {
        // Use a ppp_base_rate to anchor the fair value. Spot well above PPP.
        let mut input = usd_eur_input();
        input.ppp_base_rate = Some(dec!(1.0)); // PPP base = 1.0
                                               // PPP implied = 1.0 * (1.035)/(1.025) ~ 1.0097
                                               // Spot = 1.10, well above 1.0097 => overvalued
        let result = analyze_international(&input).unwrap();
        assert!(result.purchasing_power_parity.misalignment_pct > Decimal::ZERO);
        assert!(result
            .purchasing_power_parity
            .big_mac_equivalent
            .contains("overvalued"));
    }

    #[test]
    fn test_ppp_undervaluation_detection() {
        let mut input = usd_eur_input();
        input.ppp_base_rate = Some(dec!(1.5)); // PPP base = 1.5
                                               // PPP implied = 1.5 * (1.035)/(1.025) ~ 1.5146
                                               // Spot = 1.10, well below 1.5146 => undervalued
        let result = analyze_international(&input).unwrap();
        assert!(result.purchasing_power_parity.misalignment_pct < Decimal::ZERO);
        assert!(result
            .purchasing_power_parity
            .big_mac_equivalent
            .contains("undervalued"));
    }

    #[test]
    fn test_ppp_aligned() {
        let mut input = usd_eur_input();
        // Set spot to PPP value: 1.10 * 1.035/1.025
        let ppp_rate = dec!(1.10) * dec!(1.035) / dec!(1.025);
        input.spot_exchange_rate = ppp_rate;
        let result = analyze_international(&input).unwrap();
        assert!(result.purchasing_power_parity.misalignment_pct.abs() < dec!(1));
        assert!(result
            .purchasing_power_parity
            .big_mac_equivalent
            .contains("aligned"));
    }

    #[test]
    fn test_ppp_reversion_rate() {
        let input = usd_eur_input();
        let result = analyze_international(&input).unwrap();
        assert_eq!(
            result.purchasing_power_parity.reversion_rate_annual,
            dec!(0.15)
        );
    }

    // -- CIP / UIP tests -----------------------------------------------------

    #[test]
    fn test_cip_forward_calculation() {
        let input = usd_eur_input();
        let result = analyze_international(&input).unwrap();
        // F = 1.10 * (1.0525)^1 / (1.04)^1 = 1.10 * 1.0525 / 1.04
        let expected = dec!(1.10) * dec!(1.0525) / dec!(1.04);
        assert_eq!(result.interest_rate_parity.covered_forward, expected);
    }

    #[test]
    fn test_uip_equals_cip() {
        let input = usd_eur_input();
        let result = analyze_international(&input).unwrap();
        assert_eq!(
            result.interest_rate_parity.uncovered_expected,
            result.interest_rate_parity.covered_forward
        );
    }

    #[test]
    fn test_forward_premium_positive() {
        // When domestic rate > foreign rate, forward > spot => premium
        let input = usd_eur_input();
        let result = analyze_international(&input).unwrap();
        assert!(result.interest_rate_parity.forward_premium_pct > Decimal::ZERO);
    }

    #[test]
    fn test_forward_discount_when_domestic_lower() {
        let mut input = usd_eur_input();
        input.domestic_interest_rate = dec!(0.01);
        input.foreign_interest_rate = dec!(0.05);
        let result = analyze_international(&input).unwrap();
        assert!(result.interest_rate_parity.forward_premium_pct < Decimal::ZERO);
    }

    #[test]
    fn test_carry_trade_positive() {
        // r_foreign > r_domestic
        let mut input = usd_eur_input();
        input.domestic_interest_rate = dec!(0.01);
        input.foreign_interest_rate = dec!(0.05);
        let result = analyze_international(&input).unwrap();
        assert!(result.interest_rate_parity.carry_trade_return > Decimal::ZERO);
    }

    #[test]
    fn test_carry_trade_negative() {
        let input = usd_eur_input();
        let result = analyze_international(&input).unwrap();
        // r_foreign(4%) < r_domestic(5.25%)
        assert!(result.interest_rate_parity.carry_trade_return < Decimal::ZERO);
    }

    #[test]
    fn test_carry_trade_risk_high() {
        let input = em_input();
        let result = analyze_international(&input).unwrap();
        // Differential = 5.25% - 13.75% = -8.5% => |8.5%| > 5% => High
        assert_eq!(result.interest_rate_parity.carry_trade_risk, "High");
    }

    #[test]
    fn test_carry_trade_risk_low() {
        let mut input = usd_eur_input();
        input.domestic_interest_rate = dec!(0.04);
        input.foreign_interest_rate = dec!(0.04);
        let result = analyze_international(&input).unwrap();
        assert_eq!(result.interest_rate_parity.carry_trade_risk, "Low");
    }

    #[test]
    fn test_equal_interest_rates_no_premium() {
        let mut input = usd_eur_input();
        input.domestic_interest_rate = dec!(0.04);
        input.foreign_interest_rate = dec!(0.04);
        let result = analyze_international(&input).unwrap();
        assert_eq!(
            result.interest_rate_parity.forward_premium_pct,
            Decimal::ZERO
        );
        assert_eq!(
            result.interest_rate_parity.covered_forward,
            input.spot_exchange_rate
        );
    }

    // -- Real Exchange Rate tests --------------------------------------------

    #[test]
    fn test_real_exchange_rate_computation() {
        let input = usd_eur_input();
        let result = analyze_international(&input).unwrap();
        // RER = 1.10 * (1.025) / (1.035)
        let expected = dec!(1.10) * dec!(1.025) / dec!(1.035);
        assert_eq!(result.real_exchange_rate.real_rate, expected);
    }

    #[test]
    fn test_real_appreciation_when_domestic_inflation_higher() {
        // Higher domestic inflation => RER < NER => real appreciation > 0
        let input = usd_eur_input();
        let result = analyze_international(&input).unwrap();
        // domestic inflation (3.5%) > foreign (2.5%)
        assert!(result.real_exchange_rate.real_appreciation_pct > Decimal::ZERO);
    }

    #[test]
    fn test_real_depreciation_when_foreign_inflation_higher() {
        let mut input = usd_eur_input();
        input.domestic_inflation = dec!(0.01);
        input.foreign_inflation = dec!(0.05);
        let result = analyze_international(&input).unwrap();
        // RER = S * P_for/P_dom > S when P_for > P_dom
        // appreciation = (S - RER)/RER < 0 => depreciation
        assert!(result.real_exchange_rate.real_appreciation_pct < Decimal::ZERO);
    }

    #[test]
    fn test_nominal_rate_in_output() {
        let input = usd_eur_input();
        let result = analyze_international(&input).unwrap();
        assert_eq!(result.real_exchange_rate.nominal_rate, dec!(1.10));
    }

    #[test]
    fn test_terms_of_trade_proxy() {
        let input = usd_eur_input();
        let result = analyze_international(&input).unwrap();
        let expected = dec!(1.035) / dec!(1.025);
        assert_eq!(result.real_exchange_rate.terms_of_trade_proxy, expected);
    }

    // -- Balance of Payments tests -------------------------------------------

    #[test]
    fn test_bop_deficit_depreciation_pressure() {
        let input = usd_eur_input();
        let result = analyze_international(&input).unwrap();
        // CA = -3.5% < -3%
        assert_eq!(
            result.balance_of_payments.implied_fx_pressure,
            "Depreciation"
        );
    }

    #[test]
    fn test_bop_surplus_appreciation_pressure() {
        let mut input = usd_eur_input();
        input.current_account_pct_gdp = dec!(5.0);
        let result = analyze_international(&input).unwrap();
        assert_eq!(
            result.balance_of_payments.implied_fx_pressure,
            "Appreciation"
        );
    }

    #[test]
    fn test_bop_neutral() {
        let mut input = usd_eur_input();
        input.current_account_pct_gdp = dec!(1.0);
        let result = analyze_international(&input).unwrap();
        assert_eq!(result.balance_of_payments.implied_fx_pressure, "Neutral");
    }

    #[test]
    fn test_bop_sustainable() {
        let mut input = usd_eur_input();
        input.current_account_pct_gdp = dec!(2.0);
        let result = analyze_international(&input).unwrap();
        assert_eq!(
            result.balance_of_payments.sustainability_assessment,
            "Sustainable"
        );
    }

    #[test]
    fn test_bop_watch() {
        let input = usd_eur_input();
        let result = analyze_international(&input).unwrap();
        // |CA| = 3.5 > 3 but < 5
        assert_eq!(
            result.balance_of_payments.sustainability_assessment,
            "Watch"
        );
    }

    #[test]
    fn test_bop_unsustainable() {
        let mut input = usd_eur_input();
        input.current_account_pct_gdp = dec!(-6.0);
        let result = analyze_international(&input).unwrap();
        assert_eq!(
            result.balance_of_payments.sustainability_assessment,
            "Unsustainable"
        );
    }

    #[test]
    fn test_twin_deficit_detection() {
        let mut input = usd_eur_input();
        input.current_account_pct_gdp = dec!(-3.0);
        input.domestic_gdp_growth = dec!(0.01); // below foreign 0.01 is equal, need below
        input.foreign_gdp_growth = dec!(0.03);
        let result = analyze_international(&input).unwrap();
        assert!(result.balance_of_payments.twin_deficit_risk);
    }

    #[test]
    fn test_no_twin_deficit_with_surplus() {
        let mut input = usd_eur_input();
        input.current_account_pct_gdp = dec!(2.0);
        let result = analyze_international(&input).unwrap();
        assert!(!result.balance_of_payments.twin_deficit_risk);
    }

    // -- Projection tests ----------------------------------------------------

    #[test]
    fn test_projections_count() {
        let input = em_input();
        let result = analyze_international(&input).unwrap();
        assert_eq!(result.projected_rates.len(), 5);
    }

    #[test]
    fn test_projections_years_sequential() {
        let input = em_input();
        let result = analyze_international(&input).unwrap();
        for (i, (year, _)) in result.projected_rates.iter().enumerate() {
            assert_eq!(*year, (i as u32) + 1);
        }
    }

    #[test]
    fn test_projections_1yr() {
        let input = usd_eur_input();
        let result = analyze_international(&input).unwrap();
        assert_eq!(result.projected_rates.len(), 1);
        let (year, _rate) = &result.projected_rates[0];
        assert_eq!(*year, 1);
    }

    #[test]
    fn test_projections_5yr() {
        let input = em_input();
        let result = analyze_international(&input).unwrap();
        assert_eq!(result.projected_rates.len(), 5);
        // Rates should generally increase for EM with higher inflation
        // (domestic currency depreciates)
        let first_rate = result.projected_rates[0].1;
        let last_rate = result.projected_rates[4].1;
        assert!(
            last_rate > first_rate,
            "EM currency should depreciate over time"
        );
    }

    // -- Zero differential tests ---------------------------------------------

    #[test]
    fn test_zero_inflation_differential() {
        let mut input = usd_eur_input();
        input.domestic_inflation = dec!(0.02);
        input.foreign_inflation = dec!(0.02);
        let result = analyze_international(&input).unwrap();
        // PPP should equal spot when inflation differential is zero
        assert_eq!(
            result.purchasing_power_parity.ppp_implied_rate,
            input.spot_exchange_rate
        );
        assert_eq!(
            result.purchasing_power_parity.misalignment_pct,
            Decimal::ZERO
        );
    }

    // -- Misalignment magnitude test -----------------------------------------

    #[test]
    fn test_misalignment_magnitude() {
        let mut input = usd_eur_input();
        input.spot_exchange_rate = dec!(1.50);
        input.ppp_base_rate = Some(dec!(1.10));
        let result = analyze_international(&input).unwrap();
        // PPP implied = 1.10 * 1.035/1.025 ~ 1.1107
        // misalignment = (1.50 - 1.1107) / 1.1107 * 100 ~ 35%
        assert!(result.purchasing_power_parity.misalignment_pct > dec!(30));
    }

    // -- Validation tests ----------------------------------------------------

    #[test]
    fn test_validation_negative_spot_rate() {
        let mut input = usd_eur_input();
        input.spot_exchange_rate = dec!(-1.0);
        assert!(analyze_international(&input).is_err());
    }

    #[test]
    fn test_validation_zero_spot_rate() {
        let mut input = usd_eur_input();
        input.spot_exchange_rate = Decimal::ZERO;
        assert!(analyze_international(&input).is_err());
    }

    #[test]
    fn test_validation_zero_years() {
        let mut input = usd_eur_input();
        input.years_forward = 0;
        assert!(analyze_international(&input).is_err());
    }

    #[test]
    fn test_warning_extreme_interest_rate() {
        let mut input = usd_eur_input();
        input.domestic_interest_rate = dec!(0.60);
        let result = analyze_international(&input).unwrap();
        assert!(result
            .warnings
            .iter()
            .any(|w| w.contains("Domestic interest rate")));
    }

    #[test]
    fn test_warning_extreme_inflation() {
        let mut input = usd_eur_input();
        input.foreign_inflation = dec!(1.5);
        let result = analyze_international(&input).unwrap();
        assert!(result
            .warnings
            .iter()
            .any(|w| w.contains("Foreign inflation")));
    }

    // -- Output structure tests ----------------------------------------------

    #[test]
    fn test_methodology_not_empty() {
        let input = usd_eur_input();
        let result = analyze_international(&input).unwrap();
        assert!(!result.methodology.is_empty());
    }

    #[test]
    fn test_assumptions_contain_key_fields() {
        let input = usd_eur_input();
        let result = analyze_international(&input).unwrap();
        assert!(result.assumptions.contains_key("ppp_reversion"));
        assert!(result.assumptions.contains_key("projection_weights"));
    }

    #[test]
    fn test_serialization_roundtrip() {
        let input = usd_eur_input();
        let result = analyze_international(&input).unwrap();
        let json = serde_json::to_string(&result).unwrap();
        let _deserialized: InternationalOutput = serde_json::from_str(&json).unwrap();
    }

    // -- Large rate differential (EM) test ------------------------------------

    #[test]
    fn test_em_large_rate_differential() {
        let input = em_input();
        let result = analyze_international(&input).unwrap();
        // CIP forward should be significantly higher than spot for EM
        // (domestic currency depreciates due to higher rate)
        assert!(result.interest_rate_parity.covered_forward > input.spot_exchange_rate);
    }

    #[test]
    fn test_em_forward_premium_positive() {
        let input = em_input();
        let result = analyze_international(&input).unwrap();
        assert!(result.interest_rate_parity.forward_premium_pct > Decimal::ZERO);
    }
}
