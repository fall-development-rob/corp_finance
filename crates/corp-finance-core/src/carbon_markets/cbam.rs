//! Carbon Border Adjustment Mechanism (CBAM) analytics.
//!
//! Covers:
//! 1. **Embedded emissions** -- total CO2e for each imported good.
//! 2. **Gross CBAM cost** -- emissions x EU ETS price.
//! 3. **Origin credit** -- credit for carbon price already paid in origin country.
//! 4. **Free allocation credit** -- credit for EU ETS free allocations.
//! 5. **Net CBAM cost** -- gross minus credits (floor zero).
//! 6. **Certificates required** -- total net cost / EU ETS price.
//!
//! All arithmetic uses `rust_decimal::Decimal`. No `f64`.

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::error::CorpFinanceError;
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Input / Output
// ---------------------------------------------------------------------------

/// A single imported good subject to CBAM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CbamGood {
    /// Product description.
    pub product: String,
    /// Quantity in tonnes.
    pub quantity_tonnes: Decimal,
    /// Embedded emissions per tonne of product (tCO2e/t).
    pub embedded_emissions: Decimal,
    /// Country of origin (ISO or descriptive).
    pub origin_country: String,
    /// Carbon price already paid in origin country (EUR/tCO2e).
    pub origin_carbon_price: Decimal,
}

/// Input for CBAM analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CbamInput {
    /// List of imported goods.
    pub imported_goods: Vec<CbamGood>,
    /// Current EU ETS price (EUR/tCO2e).
    pub eu_ets_price: Decimal,
    /// Percentage of free allocation for this sector (0-1).
    pub eu_free_allocation_pct: Decimal,
}

/// Per-good CBAM result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CbamGoodResult {
    /// Product description.
    pub product: String,
    /// Total embedded emissions (quantity x embedded_emissions).
    pub total_emissions: Decimal,
    /// Gross CBAM cost before credits.
    pub gross_cbam_cost: Decimal,
    /// Credit for carbon price paid in origin country.
    pub origin_credit: Decimal,
    /// Credit for EU free allocation.
    pub free_allocation_credit: Decimal,
    /// Net CBAM cost (floor zero).
    pub net_cbam_cost: Decimal,
    /// Effective carbon price after adjustments.
    pub effective_carbon_price: Decimal,
    /// EU ETS price minus origin carbon price.
    pub price_differential: Decimal,
}

/// Aggregate CBAM output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CbamOutput {
    /// Sum of all goods' total embedded emissions.
    pub total_embedded_emissions: Decimal,
    /// Per-good results.
    pub goods_results: Vec<CbamGoodResult>,
    /// Aggregate gross cost.
    pub total_gross_cost: Decimal,
    /// Aggregate net cost.
    pub total_net_cost: Decimal,
    /// Total origin credits.
    pub total_origin_credits: Decimal,
    /// CBAM certificates to purchase (total_net_cost / eu_ets_price).
    pub certificates_required: Decimal,
    /// Weighted average effective carbon price.
    pub average_effective_price: Decimal,
}

// ---------------------------------------------------------------------------
// Core calculation
// ---------------------------------------------------------------------------

/// Compute CBAM costs for a set of imported goods.
pub fn calculate_cbam(input: &CbamInput) -> CorpFinanceResult<CbamOutput> {
    // --- Validation ---
    if input.imported_goods.is_empty() {
        return Err(CorpFinanceError::InsufficientData(
            "At least one imported good is required".into(),
        ));
    }
    if input.eu_ets_price < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "eu_ets_price".into(),
            reason: "EU ETS price cannot be negative".into(),
        });
    }
    if input.eu_free_allocation_pct < Decimal::ZERO || input.eu_free_allocation_pct > Decimal::ONE {
        return Err(CorpFinanceError::InvalidInput {
            field: "eu_free_allocation_pct".into(),
            reason: "Free allocation percentage must be between 0 and 1".into(),
        });
    }
    for (i, good) in input.imported_goods.iter().enumerate() {
        if good.quantity_tonnes < Decimal::ZERO {
            return Err(CorpFinanceError::InvalidInput {
                field: format!("imported_goods[{}].quantity_tonnes", i),
                reason: "Quantity cannot be negative".into(),
            });
        }
        if good.embedded_emissions < Decimal::ZERO {
            return Err(CorpFinanceError::InvalidInput {
                field: format!("imported_goods[{}].embedded_emissions", i),
                reason: "Embedded emissions cannot be negative".into(),
            });
        }
        if good.origin_carbon_price < Decimal::ZERO {
            return Err(CorpFinanceError::InvalidInput {
                field: format!("imported_goods[{}].origin_carbon_price", i),
                reason: "Origin carbon price cannot be negative".into(),
            });
        }
    }

    // --- Per-good calculations ---
    let mut goods_results = Vec::with_capacity(input.imported_goods.len());
    let mut total_embedded_emissions = Decimal::ZERO;
    let mut total_gross_cost = Decimal::ZERO;
    let mut total_net_cost = Decimal::ZERO;
    let mut total_origin_credits = Decimal::ZERO;

    for good in &input.imported_goods {
        let total_emissions = good.quantity_tonnes * good.embedded_emissions;
        let gross_cbam_cost = total_emissions * input.eu_ets_price;
        let origin_credit = total_emissions * good.origin_carbon_price;
        let free_allocation_credit = gross_cbam_cost * input.eu_free_allocation_pct;

        let net_raw = gross_cbam_cost - origin_credit - free_allocation_credit;
        let net_cbam_cost = if net_raw > Decimal::ZERO {
            net_raw
        } else {
            Decimal::ZERO
        };

        let effective_carbon_price = if total_emissions > Decimal::ZERO {
            net_cbam_cost / total_emissions
        } else {
            Decimal::ZERO
        };

        let price_differential = input.eu_ets_price - good.origin_carbon_price;

        total_embedded_emissions += total_emissions;
        total_gross_cost += gross_cbam_cost;
        total_net_cost += net_cbam_cost;
        total_origin_credits += origin_credit;

        goods_results.push(CbamGoodResult {
            product: good.product.clone(),
            total_emissions,
            gross_cbam_cost,
            origin_credit,
            free_allocation_credit,
            net_cbam_cost,
            effective_carbon_price,
            price_differential,
        });
    }

    // --- Certificates required ---
    let certificates_required = if input.eu_ets_price > Decimal::ZERO {
        total_net_cost / input.eu_ets_price
    } else {
        Decimal::ZERO
    };

    // --- Average effective price ---
    let average_effective_price = if total_embedded_emissions > Decimal::ZERO {
        total_net_cost / total_embedded_emissions
    } else {
        Decimal::ZERO
    };

    Ok(CbamOutput {
        total_embedded_emissions,
        goods_results,
        total_gross_cost,
        total_net_cost,
        total_origin_credits,
        certificates_required,
        average_effective_price,
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn steel_good() -> CbamGood {
        CbamGood {
            product: "Steel".into(),
            quantity_tonnes: dec!(1000),
            embedded_emissions: dec!(2.0),
            origin_country: "China".into(),
            origin_carbon_price: dec!(10),
        }
    }

    fn cement_good() -> CbamGood {
        CbamGood {
            product: "Cement".into(),
            quantity_tonnes: dec!(500),
            embedded_emissions: dec!(0.8),
            origin_country: "Turkey".into(),
            origin_carbon_price: dec!(5),
        }
    }

    fn base_input() -> CbamInput {
        CbamInput {
            imported_goods: vec![steel_good()],
            eu_ets_price: dec!(80),
            eu_free_allocation_pct: dec!(0.10),
        }
    }

    #[test]
    fn test_single_product_total_emissions() {
        let input = base_input();
        let out = calculate_cbam(&input).unwrap();
        // 1000 * 2.0 = 2000 tCO2e
        assert_eq!(out.total_embedded_emissions, dec!(2000));
    }

    #[test]
    fn test_single_product_gross_cost() {
        let input = base_input();
        let out = calculate_cbam(&input).unwrap();
        // 2000 * 80 = 160000
        assert_eq!(out.goods_results[0].gross_cbam_cost, dec!(160000));
    }

    #[test]
    fn test_single_product_origin_credit() {
        let input = base_input();
        let out = calculate_cbam(&input).unwrap();
        // 2000 * 10 = 20000
        assert_eq!(out.goods_results[0].origin_credit, dec!(20000));
    }

    #[test]
    fn test_single_product_free_allocation_credit() {
        let input = base_input();
        let out = calculate_cbam(&input).unwrap();
        // 160000 * 0.10 = 16000
        assert_eq!(out.goods_results[0].free_allocation_credit, dec!(16000));
    }

    #[test]
    fn test_single_product_net_cost() {
        let input = base_input();
        let out = calculate_cbam(&input).unwrap();
        // 160000 - 20000 - 16000 = 124000
        assert_eq!(out.goods_results[0].net_cbam_cost, dec!(124000));
    }

    #[test]
    fn test_effective_carbon_price() {
        let input = base_input();
        let out = calculate_cbam(&input).unwrap();
        // 124000 / 2000 = 62
        assert_eq!(out.goods_results[0].effective_carbon_price, dec!(62));
    }

    #[test]
    fn test_price_differential() {
        let input = base_input();
        let out = calculate_cbam(&input).unwrap();
        // 80 - 10 = 70
        assert_eq!(out.goods_results[0].price_differential, dec!(70));
    }

    #[test]
    fn test_certificates_required() {
        let input = base_input();
        let out = calculate_cbam(&input).unwrap();
        // 124000 / 80 = 1550
        assert_eq!(out.certificates_required, dec!(1550));
    }

    #[test]
    fn test_multiple_products() {
        let mut input = base_input();
        input.imported_goods.push(cement_good());
        let out = calculate_cbam(&input).unwrap();
        // Steel: 2000 tCO2e, Cement: 500*0.8 = 400 tCO2e
        assert_eq!(out.total_embedded_emissions, dec!(2400));
        assert_eq!(out.goods_results.len(), 2);
    }

    #[test]
    fn test_multiple_products_total_gross() {
        let mut input = base_input();
        input.imported_goods.push(cement_good());
        let out = calculate_cbam(&input).unwrap();
        // Steel gross: 160000, Cement gross: 400 * 80 = 32000
        assert_eq!(out.total_gross_cost, dec!(192000));
    }

    #[test]
    fn test_high_origin_price_zero_net() {
        let mut input = base_input();
        input.imported_goods[0].origin_carbon_price = dec!(100);
        let out = calculate_cbam(&input).unwrap();
        // origin_credit = 2000 * 100 = 200000 > gross 160000, net = 0
        assert_eq!(out.goods_results[0].net_cbam_cost, Decimal::ZERO);
    }

    #[test]
    fn test_zero_origin_price_full_cbam() {
        let mut input = base_input();
        input.imported_goods[0].origin_carbon_price = Decimal::ZERO;
        let out = calculate_cbam(&input).unwrap();
        // gross = 160000, origin_credit = 0, free_alloc = 16000
        // net = 160000 - 0 - 16000 = 144000
        assert_eq!(out.goods_results[0].net_cbam_cost, dec!(144000));
    }

    #[test]
    fn test_zero_free_allocation() {
        let mut input = base_input();
        input.eu_free_allocation_pct = Decimal::ZERO;
        let out = calculate_cbam(&input).unwrap();
        // net = 160000 - 20000 - 0 = 140000
        assert_eq!(out.goods_results[0].net_cbam_cost, dec!(140000));
    }

    #[test]
    fn test_full_free_allocation() {
        let mut input = base_input();
        input.eu_free_allocation_pct = Decimal::ONE;
        let out = calculate_cbam(&input).unwrap();
        // free_alloc = 160000 * 1.0 = 160000
        // net = 160000 - 20000 - 160000 = -20000 -> 0
        assert_eq!(out.goods_results[0].net_cbam_cost, Decimal::ZERO);
    }

    #[test]
    fn test_empty_goods_rejected() {
        let input = CbamInput {
            imported_goods: vec![],
            eu_ets_price: dec!(80),
            eu_free_allocation_pct: dec!(0.10),
        };
        let result = calculate_cbam(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_negative_ets_price_rejected() {
        let mut input = base_input();
        input.eu_ets_price = dec!(-10);
        let result = calculate_cbam(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_free_allocation_out_of_range_rejected() {
        let mut input = base_input();
        input.eu_free_allocation_pct = dec!(1.5);
        let result = calculate_cbam(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_negative_quantity_rejected() {
        let mut input = base_input();
        input.imported_goods[0].quantity_tonnes = dec!(-100);
        let result = calculate_cbam(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_negative_embedded_emissions_rejected() {
        let mut input = base_input();
        input.imported_goods[0].embedded_emissions = dec!(-1);
        let result = calculate_cbam(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_negative_origin_price_rejected() {
        let mut input = base_input();
        input.imported_goods[0].origin_carbon_price = dec!(-5);
        let result = calculate_cbam(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_average_effective_price() {
        let input = base_input();
        let out = calculate_cbam(&input).unwrap();
        // total_net_cost / total_embedded_emissions = 124000 / 2000 = 62
        assert_eq!(out.average_effective_price, dec!(62));
    }

    #[test]
    fn test_mixed_origins() {
        let mut input = base_input();
        input.imported_goods.push(CbamGood {
            product: "Aluminium".into(),
            quantity_tonnes: dec!(200),
            embedded_emissions: dec!(5.0),
            origin_country: "Norway".into(),
            origin_carbon_price: dec!(75),
        });
        let out = calculate_cbam(&input).unwrap();
        // Aluminium: 1000 tCO2e, gross = 80000
        // origin_credit = 1000*75 = 75000, free_alloc = 80000*0.10 = 8000
        // net = 80000 - 75000 - 8000 = -3000 -> 0
        assert_eq!(out.goods_results[1].net_cbam_cost, Decimal::ZERO);
        // Total should only include steel net
        assert_eq!(out.total_net_cost, dec!(124000));
    }

    #[test]
    fn test_zero_eu_ets_price() {
        let mut input = base_input();
        input.eu_ets_price = Decimal::ZERO;
        let out = calculate_cbam(&input).unwrap();
        assert_eq!(out.total_gross_cost, Decimal::ZERO);
        assert_eq!(out.certificates_required, Decimal::ZERO);
    }
}
