//! EU ETS compliance analytics.
//!
//! Covers:
//! 1. **Allowance shortfall/surplus** -- gap between verified emissions and held allowances.
//! 2. **Compliance cost** -- cost to close the shortfall at current prices.
//! 3. **Price volatility** -- standard deviation of historical allowance prices.
//! 4. **Value-at-Risk (95%)** -- 1.645 x vol x sqrt(days/365) x portfolio_value.
//! 5. **Benchmark positioning** -- actual emission factor vs industry benchmark.
//! 6. **Abatement value** -- savings if the company reduced to benchmark level.
//!
//! All arithmetic uses `rust_decimal::Decimal`. No `f64`.

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

use crate::error::CorpFinanceError;
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Newton's method square root for Decimal.
fn decimal_sqrt(x: Decimal) -> Decimal {
    if x <= Decimal::ZERO {
        return Decimal::ZERO;
    }
    let mut guess = x / dec!(2);
    if guess == Decimal::ZERO {
        guess = Decimal::ONE;
    }
    for _ in 0..30 {
        guess = (guess + x / guess) / dec!(2);
    }
    guess
}

// ---------------------------------------------------------------------------
// Input / Output
// ---------------------------------------------------------------------------

/// Input for ETS compliance analytics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EtsComplianceInput {
    /// Total tonnes CO2e emitted (verified).
    pub verified_emissions: Decimal,
    /// Allocated free allowances (EUAs).
    pub free_allowances: Decimal,
    /// Already purchased allowances.
    pub purchased_allowances: Decimal,
    /// Current EUA price per tonne.
    pub allowance_price: Decimal,
    /// Recent historical prices for volatility calculation.
    pub historical_prices: Vec<Decimal>,
    /// Days until surrender deadline.
    pub compliance_deadline_days: u32,
    /// Industry benchmark emission factor (tonnes per unit of output).
    pub benchmark_emission_factor: Decimal,
    /// Company's actual emission factor.
    pub actual_emission_factor: Decimal,
}

/// Output of ETS compliance analytics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EtsComplianceOutput {
    /// Total requirement equals verified emissions.
    pub total_requirement: Decimal,
    /// Total held allowances (free + purchased).
    pub total_allowances: Decimal,
    /// Shortfall: max(0, requirement - allowances).
    pub shortfall: Decimal,
    /// Surplus: max(0, allowances - requirement).
    pub surplus: Decimal,
    /// Cost to close the shortfall.
    pub compliance_cost: Decimal,
    /// Free allocation as percentage of requirement.
    pub free_allocation_pct: Decimal,
    /// Standard deviation of historical prices.
    pub price_volatility: Decimal,
    /// 95% Value-at-Risk on the allowance portfolio.
    pub var_95: Decimal,
    /// Benchmark positioning label.
    pub benchmark_position: String,
    /// Abatement value if reduced to benchmark.
    pub abatement_value: Decimal,
}

// ---------------------------------------------------------------------------
// Core calculation
// ---------------------------------------------------------------------------

/// Compute ETS compliance analytics.
pub fn calculate_ets_compliance(
    input: &EtsComplianceInput,
) -> CorpFinanceResult<EtsComplianceOutput> {
    // --- Validation ---
    if input.verified_emissions < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "verified_emissions".into(),
            reason: "Verified emissions cannot be negative".into(),
        });
    }
    if input.free_allowances < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "free_allowances".into(),
            reason: "Free allowances cannot be negative".into(),
        });
    }
    if input.purchased_allowances < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "purchased_allowances".into(),
            reason: "Purchased allowances cannot be negative".into(),
        });
    }
    if input.allowance_price < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "allowance_price".into(),
            reason: "Allowance price cannot be negative".into(),
        });
    }
    if input.benchmark_emission_factor < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "benchmark_emission_factor".into(),
            reason: "Benchmark emission factor cannot be negative".into(),
        });
    }
    if input.actual_emission_factor < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "actual_emission_factor".into(),
            reason: "Actual emission factor cannot be negative".into(),
        });
    }

    // --- Shortfall / surplus ---
    let total_requirement = input.verified_emissions;
    let total_allowances = input.free_allowances + input.purchased_allowances;
    let shortfall = if total_requirement > total_allowances {
        total_requirement - total_allowances
    } else {
        Decimal::ZERO
    };
    let surplus = if total_allowances > total_requirement {
        total_allowances - total_requirement
    } else {
        Decimal::ZERO
    };
    let compliance_cost = shortfall * input.allowance_price;

    // --- Free allocation percentage ---
    let free_allocation_pct = if total_requirement > Decimal::ZERO {
        input.free_allowances / total_requirement
    } else {
        Decimal::ZERO
    };

    // --- Price volatility (standard deviation) ---
    let price_volatility = if input.historical_prices.len() >= 2 {
        let n = Decimal::from(input.historical_prices.len() as u32);
        let sum: Decimal = input.historical_prices.iter().copied().sum();
        let mean = sum / n;
        let var_sum: Decimal = input
            .historical_prices
            .iter()
            .map(|p| {
                let diff = *p - mean;
                diff * diff
            })
            .sum();
        // Population std dev for price history
        let variance = var_sum / n;
        decimal_sqrt(variance)
    } else {
        Decimal::ZERO
    };

    // --- VaR 95% ---
    // portfolio_value = total_allowances * allowance_price
    let portfolio_value = total_allowances * input.allowance_price;
    let days_frac = Decimal::from(input.compliance_deadline_days) / dec!(365);
    let time_sqrt = decimal_sqrt(days_frac);
    // Normalised volatility: vol as fraction of price
    let vol_pct = if input.allowance_price > Decimal::ZERO {
        price_volatility / input.allowance_price
    } else {
        Decimal::ZERO
    };
    let var_95 = dec!(1.645) * vol_pct * time_sqrt * portfolio_value;

    // --- Benchmark positioning ---
    let benchmark_position = if input.actual_emission_factor < input.benchmark_emission_factor {
        "Below Benchmark".to_string()
    } else if input.actual_emission_factor == input.benchmark_emission_factor {
        "At Benchmark".to_string()
    } else {
        "Above Benchmark".to_string()
    };

    // --- Abatement value ---
    // If actual > benchmark, compute savings from reducing to benchmark.
    // output_implied = verified_emissions / actual_emission_factor
    // abatement = (actual - benchmark) * output_implied * price
    let abatement_value = if input.actual_emission_factor > input.benchmark_emission_factor
        && input.actual_emission_factor > Decimal::ZERO
    {
        let output_implied = input.verified_emissions / input.actual_emission_factor;
        let excess = input.actual_emission_factor - input.benchmark_emission_factor;
        excess * output_implied * input.allowance_price
    } else {
        Decimal::ZERO
    };

    Ok(EtsComplianceOutput {
        total_requirement,
        total_allowances,
        shortfall,
        surplus,
        compliance_cost,
        free_allocation_pct,
        price_volatility,
        var_95,
        benchmark_position,
        abatement_value,
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn base_input() -> EtsComplianceInput {
        EtsComplianceInput {
            verified_emissions: dec!(100000),
            free_allowances: dec!(60000),
            purchased_allowances: dec!(20000),
            allowance_price: dec!(80),
            historical_prices: vec![dec!(75), dec!(78), dec!(80), dec!(82), dec!(85)],
            compliance_deadline_days: 90,
            benchmark_emission_factor: dec!(0.5),
            actual_emission_factor: dec!(0.6),
        }
    }

    #[test]
    fn test_shortfall_scenario() {
        let input = base_input();
        let out = calculate_ets_compliance(&input).unwrap();
        // total_allowances = 60000 + 20000 = 80000
        // shortfall = 100000 - 80000 = 20000
        assert_eq!(out.total_allowances, dec!(80000));
        assert_eq!(out.shortfall, dec!(20000));
        assert_eq!(out.surplus, Decimal::ZERO);
        assert_eq!(out.compliance_cost, dec!(1600000)); // 20000 * 80
    }

    #[test]
    fn test_surplus_scenario() {
        let mut input = base_input();
        input.verified_emissions = dec!(50000);
        let out = calculate_ets_compliance(&input).unwrap();
        assert_eq!(out.shortfall, Decimal::ZERO);
        assert_eq!(out.surplus, dec!(30000)); // 80000 - 50000
    }

    #[test]
    fn test_exact_match_no_shortfall_no_surplus() {
        let mut input = base_input();
        input.verified_emissions = dec!(80000);
        let out = calculate_ets_compliance(&input).unwrap();
        assert_eq!(out.shortfall, Decimal::ZERO);
        assert_eq!(out.surplus, Decimal::ZERO);
    }

    #[test]
    fn test_zero_free_allocation() {
        let mut input = base_input();
        input.free_allowances = Decimal::ZERO;
        let out = calculate_ets_compliance(&input).unwrap();
        assert_eq!(out.free_allocation_pct, Decimal::ZERO);
        assert_eq!(out.shortfall, dec!(80000)); // 100000 - 20000
    }

    #[test]
    fn test_free_allocation_pct() {
        let input = base_input();
        let out = calculate_ets_compliance(&input).unwrap();
        // 60000 / 100000 = 0.6
        assert_eq!(out.free_allocation_pct, dec!(0.6));
    }

    #[test]
    fn test_price_volatility_computed() {
        let input = base_input();
        let out = calculate_ets_compliance(&input).unwrap();
        // mean = (75+78+80+82+85)/5 = 80
        // var = ((25+4+0+4+25)/5) = 58/5 = 11.6
        // std = sqrt(11.6) ~ 3.406
        assert!(out.price_volatility > dec!(3.4));
        assert!(out.price_volatility < dec!(3.5));
    }

    #[test]
    fn test_volatility_single_price() {
        let mut input = base_input();
        input.historical_prices = vec![dec!(80)];
        let out = calculate_ets_compliance(&input).unwrap();
        assert_eq!(out.price_volatility, Decimal::ZERO);
    }

    #[test]
    fn test_volatility_empty_prices() {
        let mut input = base_input();
        input.historical_prices = vec![];
        let out = calculate_ets_compliance(&input).unwrap();
        assert_eq!(out.price_volatility, Decimal::ZERO);
    }

    #[test]
    fn test_var_95_positive() {
        let input = base_input();
        let out = calculate_ets_compliance(&input).unwrap();
        assert!(out.var_95 > Decimal::ZERO);
    }

    #[test]
    fn test_var_95_zero_when_no_allowances() {
        let mut input = base_input();
        input.free_allowances = Decimal::ZERO;
        input.purchased_allowances = Decimal::ZERO;
        let out = calculate_ets_compliance(&input).unwrap();
        assert_eq!(out.var_95, Decimal::ZERO);
    }

    #[test]
    fn test_benchmark_below() {
        let mut input = base_input();
        input.actual_emission_factor = dec!(0.4);
        let out = calculate_ets_compliance(&input).unwrap();
        assert_eq!(out.benchmark_position, "Below Benchmark");
    }

    #[test]
    fn test_benchmark_at() {
        let mut input = base_input();
        input.actual_emission_factor = dec!(0.5);
        let out = calculate_ets_compliance(&input).unwrap();
        assert_eq!(out.benchmark_position, "At Benchmark");
    }

    #[test]
    fn test_benchmark_above() {
        let input = base_input();
        let out = calculate_ets_compliance(&input).unwrap();
        assert_eq!(out.benchmark_position, "Above Benchmark");
    }

    #[test]
    fn test_abatement_value_above_benchmark() {
        let input = base_input();
        let out = calculate_ets_compliance(&input).unwrap();
        // output_implied = 100000 / 0.6 = 166666.666...
        // excess = 0.6 - 0.5 = 0.1
        // abatement = 0.1 * 166666.66... * 80 = 1333333.33...
        assert!(out.abatement_value > dec!(1333333));
        assert!(out.abatement_value < dec!(1333334));
    }

    #[test]
    fn test_abatement_value_below_benchmark() {
        let mut input = base_input();
        input.actual_emission_factor = dec!(0.4);
        let out = calculate_ets_compliance(&input).unwrap();
        assert_eq!(out.abatement_value, Decimal::ZERO);
    }

    #[test]
    fn test_negative_emissions_rejected() {
        let mut input = base_input();
        input.verified_emissions = dec!(-100);
        let result = calculate_ets_compliance(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_negative_free_allowances_rejected() {
        let mut input = base_input();
        input.free_allowances = dec!(-100);
        let result = calculate_ets_compliance(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_negative_price_rejected() {
        let mut input = base_input();
        input.allowance_price = dec!(-10);
        let result = calculate_ets_compliance(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_zero_emissions_zero_requirement() {
        let mut input = base_input();
        input.verified_emissions = Decimal::ZERO;
        let out = calculate_ets_compliance(&input).unwrap();
        assert_eq!(out.total_requirement, Decimal::ZERO);
        assert!(out.surplus > Decimal::ZERO);
        assert_eq!(out.free_allocation_pct, Decimal::ZERO);
    }

    #[test]
    fn test_high_volatility_prices() {
        let mut input = base_input();
        input.historical_prices = vec![dec!(40), dec!(120), dec!(40), dec!(120)];
        let out = calculate_ets_compliance(&input).unwrap();
        // mean = 80, var = (1600+1600+1600+1600)/4 = 1600, std = 40
        assert!(out.price_volatility > dec!(39.9));
        assert!(out.price_volatility < dec!(40.1));
    }

    #[test]
    fn test_constant_prices_zero_volatility() {
        let mut input = base_input();
        input.historical_prices = vec![dec!(80), dec!(80), dec!(80)];
        let out = calculate_ets_compliance(&input).unwrap();
        assert_eq!(out.price_volatility, Decimal::ZERO);
    }

    #[test]
    fn test_negative_benchmark_rejected() {
        let mut input = base_input();
        input.benchmark_emission_factor = dec!(-0.1);
        let result = calculate_ets_compliance(&input);
        assert!(result.is_err());
    }
}
