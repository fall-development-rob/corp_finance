//! Carbon credit pricing analytics.
//!
//! Covers:
//! 1. **Cost-of-carry forward pricing** -- F = S x (1 + (r + c - y) x T) linear approx.
//! 2. **Vintage adjustment** -- age-based discount on older credits.
//! 3. **Registry premium** -- quality spread by registry (Gold Standard, Verra, etc.).
//! 4. **Credit type premium** -- spread by underlying project type.
//! 5. **Basis calculation** -- forward minus spot.
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

/// Input for carbon credit pricing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CarbonPricingInput {
    /// Current carbon credit price per tonne CO2e.
    pub spot_price: Decimal,
    /// Annualized risk-free rate (e.g. 0.04 for 4%).
    pub risk_free_rate: Decimal,
    /// Cost of carry (registry fees, etc.) as annual percentage.
    pub storage_cost: Decimal,
    /// Convenience yield as annual percentage.
    pub convenience_yield: Decimal,
    /// Time to delivery in years.
    pub time_to_delivery: Decimal,
    /// Vintage year of the credit.
    pub vintage_year: u32,
    /// Reference (current) year.
    pub current_year: u32,
    /// Registry: "verra", "gold_standard", "acr", "car".
    pub registry: String,
    /// Credit type: "nature_based", "renewable_energy", "industrial", "technology".
    pub credit_type: String,
}

/// Output of carbon credit pricing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CarbonPricingOutput {
    /// Cost-of-carry forward price.
    pub forward_price: Decimal,
    /// Premium/discount for vintage age.
    pub vintage_adjustment: Decimal,
    /// Final adjusted price (forward + vintage + registry + type).
    pub adjusted_price: Decimal,
    /// Premium for registry quality.
    pub registry_premium: Decimal,
    /// Premium for credit type.
    pub type_premium: Decimal,
    /// Total carry cost over delivery period.
    pub carry_cost: Decimal,
    /// Forward minus spot.
    pub basis: Decimal,
}

// ---------------------------------------------------------------------------
// Core calculation
// ---------------------------------------------------------------------------

/// Compute carbon credit forward price with adjustments.
pub fn calculate_carbon_pricing(
    input: &CarbonPricingInput,
) -> CorpFinanceResult<CarbonPricingOutput> {
    // --- Validation ---
    if input.spot_price < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "spot_price".into(),
            reason: "Spot price cannot be negative".into(),
        });
    }
    if input.time_to_delivery < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "time_to_delivery".into(),
            reason: "Time to delivery cannot be negative".into(),
        });
    }
    if input.vintage_year > input.current_year {
        return Err(CorpFinanceError::InvalidInput {
            field: "vintage_year".into(),
            reason: "Vintage year cannot be in the future".into(),
        });
    }
    let registry_lower = input.registry.to_lowercase();
    let valid_registries = ["verra", "gold_standard", "acr", "car"];
    if !valid_registries.contains(&registry_lower.as_str()) {
        return Err(CorpFinanceError::InvalidInput {
            field: "registry".into(),
            reason: format!(
                "Unknown registry '{}'. Expected one of: {:?}",
                input.registry, valid_registries
            ),
        });
    }
    let type_lower = input.credit_type.to_lowercase();
    let valid_types = [
        "nature_based",
        "renewable_energy",
        "industrial",
        "technology",
    ];
    if !valid_types.contains(&type_lower.as_str()) {
        return Err(CorpFinanceError::InvalidInput {
            field: "credit_type".into(),
            reason: format!(
                "Unknown credit type '{}'. Expected one of: {:?}",
                input.credit_type, valid_types
            ),
        });
    }

    // --- Forward price (linear cost-of-carry approximation) ---
    // F = S * (1 + (r + c - y) * T)
    let net_carry_rate = input.risk_free_rate + input.storage_cost - input.convenience_yield;
    let forward_price = input.spot_price * (Decimal::ONE + net_carry_rate * input.time_to_delivery);
    let carry_cost = forward_price - input.spot_price;

    // --- Vintage adjustment ---
    let age = input.current_year.saturating_sub(input.vintage_year);
    let vintage_discount_rate = dec!(-0.02) * Decimal::from(age);
    // Cap at -20%
    let vintage_discount_rate = if vintage_discount_rate < dec!(-0.20) {
        dec!(-0.20)
    } else {
        vintage_discount_rate
    };
    let vintage_adjustment = forward_price * vintage_discount_rate;

    // --- Registry premium ---
    let registry_premium_pct = match registry_lower.as_str() {
        "gold_standard" => dec!(0.05),
        "verra" => dec!(0.02),
        "acr" => Decimal::ZERO,
        "car" => dec!(-0.02),
        _ => Decimal::ZERO,
    };
    let registry_premium = forward_price * registry_premium_pct;

    // --- Type premium ---
    let type_premium_pct = match type_lower.as_str() {
        "technology" => dec!(0.10),
        "industrial" => dec!(0.03),
        "renewable_energy" => Decimal::ZERO,
        "nature_based" => dec!(-0.05),
        _ => Decimal::ZERO,
    };
    let type_premium = forward_price * type_premium_pct;

    // --- Final adjusted price ---
    let adjusted_price = forward_price + vintage_adjustment + registry_premium + type_premium;

    let basis = forward_price - input.spot_price;

    Ok(CarbonPricingOutput {
        forward_price,
        vintage_adjustment,
        adjusted_price,
        registry_premium,
        type_premium,
        carry_cost,
        basis,
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn base_input() -> CarbonPricingInput {
        CarbonPricingInput {
            spot_price: dec!(50),
            risk_free_rate: dec!(0.04),
            storage_cost: dec!(0.01),
            convenience_yield: dec!(0.02),
            time_to_delivery: dec!(1),
            vintage_year: 2024,
            current_year: 2025,
            registry: "verra".into(),
            credit_type: "renewable_energy".into(),
        }
    }

    #[test]
    fn test_basic_forward_price() {
        let input = base_input();
        let out = calculate_carbon_pricing(&input).unwrap();
        // F = 50 * (1 + (0.04 + 0.01 - 0.02) * 1) = 50 * 1.03 = 51.50
        assert_eq!(out.forward_price, dec!(51.50));
    }

    #[test]
    fn test_basis_equals_carry_cost() {
        let input = base_input();
        let out = calculate_carbon_pricing(&input).unwrap();
        assert_eq!(out.basis, out.carry_cost);
        assert_eq!(out.basis, dec!(1.50));
    }

    #[test]
    fn test_zero_time_to_delivery() {
        let mut input = base_input();
        input.time_to_delivery = Decimal::ZERO;
        let out = calculate_carbon_pricing(&input).unwrap();
        assert_eq!(out.forward_price, dec!(50));
        assert_eq!(out.basis, Decimal::ZERO);
    }

    #[test]
    fn test_vintage_same_year_no_discount() {
        let mut input = base_input();
        input.vintage_year = 2025;
        input.current_year = 2025;
        let out = calculate_carbon_pricing(&input).unwrap();
        // age=0, vintage_adjustment = 0
        assert_eq!(out.vintage_adjustment, Decimal::ZERO);
    }

    #[test]
    fn test_vintage_5_year_discount() {
        let mut input = base_input();
        input.vintage_year = 2020;
        input.current_year = 2025;
        let out = calculate_carbon_pricing(&input).unwrap();
        // age=5, discount = -0.02*5 = -10% of forward
        let expected = out.forward_price * dec!(-0.10);
        assert_eq!(out.vintage_adjustment, expected);
    }

    #[test]
    fn test_vintage_cap_at_20_pct() {
        let mut input = base_input();
        input.vintage_year = 2005;
        input.current_year = 2025;
        let out = calculate_carbon_pricing(&input).unwrap();
        // age=20, discount = -0.02*20 = -40% but capped at -20%
        let expected = out.forward_price * dec!(-0.20);
        assert_eq!(out.vintage_adjustment, expected);
    }

    #[test]
    fn test_registry_gold_standard_premium() {
        let mut input = base_input();
        input.registry = "gold_standard".into();
        let out = calculate_carbon_pricing(&input).unwrap();
        assert_eq!(out.registry_premium, out.forward_price * dec!(0.05));
    }

    #[test]
    fn test_registry_verra_premium() {
        let input = base_input();
        let out = calculate_carbon_pricing(&input).unwrap();
        assert_eq!(out.registry_premium, out.forward_price * dec!(0.02));
    }

    #[test]
    fn test_registry_acr_zero() {
        let mut input = base_input();
        input.registry = "acr".into();
        let out = calculate_carbon_pricing(&input).unwrap();
        assert_eq!(out.registry_premium, Decimal::ZERO);
    }

    #[test]
    fn test_registry_car_discount() {
        let mut input = base_input();
        input.registry = "car".into();
        let out = calculate_carbon_pricing(&input).unwrap();
        assert_eq!(out.registry_premium, out.forward_price * dec!(-0.02));
    }

    #[test]
    fn test_type_technology_premium() {
        let mut input = base_input();
        input.credit_type = "technology".into();
        let out = calculate_carbon_pricing(&input).unwrap();
        assert_eq!(out.type_premium, out.forward_price * dec!(0.10));
    }

    #[test]
    fn test_type_industrial_premium() {
        let mut input = base_input();
        input.credit_type = "industrial".into();
        let out = calculate_carbon_pricing(&input).unwrap();
        assert_eq!(out.type_premium, out.forward_price * dec!(0.03));
    }

    #[test]
    fn test_type_renewable_energy_zero() {
        let input = base_input();
        let out = calculate_carbon_pricing(&input).unwrap();
        assert_eq!(out.type_premium, Decimal::ZERO);
    }

    #[test]
    fn test_type_nature_based_discount() {
        let mut input = base_input();
        input.credit_type = "nature_based".into();
        let out = calculate_carbon_pricing(&input).unwrap();
        assert_eq!(out.type_premium, out.forward_price * dec!(-0.05));
    }

    #[test]
    fn test_adjusted_price_sum() {
        let input = base_input();
        let out = calculate_carbon_pricing(&input).unwrap();
        let expected =
            out.forward_price + out.vintage_adjustment + out.registry_premium + out.type_premium;
        assert_eq!(out.adjusted_price, expected);
    }

    #[test]
    fn test_negative_spot_rejected() {
        let mut input = base_input();
        input.spot_price = dec!(-10);
        let result = calculate_carbon_pricing(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_negative_time_rejected() {
        let mut input = base_input();
        input.time_to_delivery = dec!(-1);
        let result = calculate_carbon_pricing(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_future_vintage_rejected() {
        let mut input = base_input();
        input.vintage_year = 2030;
        input.current_year = 2025;
        let result = calculate_carbon_pricing(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_registry_rejected() {
        let mut input = base_input();
        input.registry = "unknown_registry".into();
        let result = calculate_carbon_pricing(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_credit_type_rejected() {
        let mut input = base_input();
        input.credit_type = "unknown_type".into();
        let result = calculate_carbon_pricing(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_long_dated_forward_3yr() {
        let mut input = base_input();
        input.time_to_delivery = dec!(3);
        let out = calculate_carbon_pricing(&input).unwrap();
        // F = 50 * (1 + 0.03 * 3) = 50 * 1.09 = 54.50
        assert_eq!(out.forward_price, dec!(54.50));
    }

    #[test]
    fn test_negative_basis_possible() {
        let mut input = base_input();
        input.convenience_yield = dec!(0.10);
        // net carry = 0.04 + 0.01 - 0.10 = -0.05
        // F = 50 * (1 + (-0.05) * 1) = 50 * 0.95 = 47.50
        let out = calculate_carbon_pricing(&input).unwrap();
        assert_eq!(out.forward_price, dec!(47.50));
        assert!(out.basis < Decimal::ZERO);
    }

    #[test]
    fn test_zero_spot_price() {
        let mut input = base_input();
        input.spot_price = Decimal::ZERO;
        let out = calculate_carbon_pricing(&input).unwrap();
        assert_eq!(out.forward_price, Decimal::ZERO);
        assert_eq!(out.adjusted_price, Decimal::ZERO);
    }

    #[test]
    fn test_high_spot_price() {
        let mut input = base_input();
        input.spot_price = dec!(200);
        let out = calculate_carbon_pricing(&input).unwrap();
        // F = 200 * 1.03 = 206.00
        assert_eq!(out.forward_price, dec!(206.00));
        assert!(out.adjusted_price > dec!(200));
    }

    #[test]
    fn test_case_insensitive_registry() {
        let mut input = base_input();
        input.registry = "Gold_Standard".into();
        let out = calculate_carbon_pricing(&input).unwrap();
        assert_eq!(out.registry_premium, out.forward_price * dec!(0.05));
    }
}
