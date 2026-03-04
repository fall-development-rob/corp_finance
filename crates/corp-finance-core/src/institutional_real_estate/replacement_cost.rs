use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::error::CorpFinanceError;
use crate::types::{with_metadata, ComputationOutput, Money, Rate};
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

/// Structural classification for Marshall & Swift base cost lookup.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BuildingClass {
    /// Steel frame / concrete
    A,
    /// Reinforced concrete
    B,
    /// Masonry bearing wall
    C,
    /// Wood frame
    D,
    /// Pre-engineered metal
    S,
}

/// Primary occupancy / use type for base cost lookup.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OccupancyType {
    Office,
    Retail,
    Industrial,
    Multifamily,
    Hospitality,
}

/// Whether the cost estimate targets a modern equivalent (replacement) or
/// an exact replica (reproduction).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CostType {
    /// Modern equivalent with same utility
    Replacement,
    /// Exact replica of the existing improvement
    Reproduction,
}

// ---------------------------------------------------------------------------
// Marshall & Swift Base Cost Table ($/SF)
// ---------------------------------------------------------------------------

/// Row in the base-cost lookup table.  Derive Serialize only because
/// `&'static` slices cannot derive Deserialize.
#[derive(Debug, Clone, Serialize)]
pub struct BaseCostRow {
    pub class: &'static str,
    pub office: Decimal,
    pub retail: Decimal,
    pub industrial: Decimal,
    pub multifamily: Decimal,
    pub hospitality: Decimal,
}

/// Static base-cost table (cost per SF by class and occupancy).
pub static BASE_COSTS: &[BaseCostRow] = &[
    BaseCostRow {
        class: "A",
        office: dec!(185),
        retail: dec!(165),
        industrial: dec!(125),
        multifamily: dec!(175),
        hospitality: dec!(210),
    },
    BaseCostRow {
        class: "B",
        office: dec!(155),
        retail: dec!(140),
        industrial: dec!(105),
        multifamily: dec!(150),
        hospitality: dec!(180),
    },
    BaseCostRow {
        class: "C",
        office: dec!(125),
        retail: dec!(115),
        industrial: dec!(85),
        multifamily: dec!(120),
        hospitality: dec!(150),
    },
    BaseCostRow {
        class: "D",
        office: dec!(95),
        retail: dec!(90),
        industrial: dec!(70),
        multifamily: dec!(95),
        hospitality: dec!(120),
    },
    BaseCostRow {
        class: "S",
        office: dec!(75),
        retail: dec!(70),
        industrial: dec!(55),
        multifamily: dec!(0),
        hospitality: dec!(0),
    },
];

fn lookup_base_cost(
    class: &BuildingClass,
    occupancy: &OccupancyType,
) -> CorpFinanceResult<Decimal> {
    let idx = match class {
        BuildingClass::A => 0,
        BuildingClass::B => 1,
        BuildingClass::C => 2,
        BuildingClass::D => 3,
        BuildingClass::S => 4,
    };
    let row = &BASE_COSTS[idx];
    let cost = match occupancy {
        OccupancyType::Office => row.office,
        OccupancyType::Retail => row.retail,
        OccupancyType::Industrial => row.industrial,
        OccupancyType::Multifamily => row.multifamily,
        OccupancyType::Hospitality => row.hospitality,
    };
    if cost == dec!(0) {
        return Err(CorpFinanceError::InvalidInput {
            field: "building_class / occupancy_type".into(),
            reason: format!(
                "Class {:?} is not available for {:?} occupancy",
                class, occupancy
            ),
        });
    }
    Ok(cost)
}

// ---------------------------------------------------------------------------
// Input / Output Types
// ---------------------------------------------------------------------------

/// Input for the full cost approach valuation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostApproachInput {
    pub property_name: String,
    pub cost_type: CostType,
    /// Land value from comparable sales or residual extraction.
    pub land_value: Money,
    /// Replacement (or reproduction) cost new of improvements.
    pub replacement_cost_new: Money,
    /// Physical depreciation as a fraction of RCN (0..1).
    pub physical_depreciation_pct: Rate,
    /// Functional obsolescence as a dollar amount.
    pub functional_obsolescence: Money,
    /// External obsolescence as a dollar amount.
    pub external_obsolescence: Money,
    /// Optional entrepreneurial incentive / developer profit (added to RCN).
    pub entrepreneurial_incentive: Option<Money>,
    /// Gross building area in SF.
    pub gross_area_sf: Decimal,
}

/// Output of the cost approach.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostApproachOutput {
    pub property_name: String,
    pub cost_type: CostType,
    pub land_value: Money,
    pub replacement_cost_new: Money,
    pub entrepreneurial_incentive: Money,
    pub rcn_plus_incentive: Money,
    pub physical_depreciation: Money,
    pub functional_obsolescence: Money,
    pub external_obsolescence: Money,
    pub total_depreciation: Money,
    pub depreciated_improvement_value: Money,
    pub indicated_value: Money,
    pub value_per_sf: Money,
    pub total_depreciation_pct: Rate,
}

/// Input for a three-tier depreciation schedule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepreciationScheduleInput {
    pub replacement_cost_new: Money,
    /// Effective age of the improvements (years).
    pub effective_age: Decimal,
    /// Total economic life of the improvements (years).
    pub total_economic_life: Decimal,
    /// Curable functional obsolescence: cost to cure the deficiency.
    pub curable_functional: Money,
    /// Incurable functional obsolescence: capitalized rent loss per year.
    pub incurable_functional_annual_loss: Money,
    /// Cap rate used to capitalize incurable functional and external losses.
    pub cap_rate: Rate,
    /// External obsolescence from paired sales (as fraction of RCN, 0..1).
    /// If provided, takes precedence over capitalized income loss.
    pub external_paired_sales_pct: Option<Rate>,
    /// External obsolescence: annual income loss to capitalize (alternative).
    pub external_annual_income_loss: Option<Money>,
}

/// Output of the depreciation schedule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepreciationScheduleOutput {
    pub replacement_cost_new: Money,
    pub physical_depreciation_pct: Rate,
    pub physical_depreciation: Money,
    pub curable_functional: Money,
    pub incurable_functional: Money,
    pub total_functional: Money,
    pub external_obsolescence: Money,
    pub total_depreciation: Money,
    pub total_depreciation_pct: Rate,
    pub depreciated_value: Money,
    pub capped_at_max: bool,
}

/// Input for land residual extraction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LandResidualInput {
    /// Total property value (from income or sales approach).
    pub total_property_value: Money,
    /// Depreciated improvement value.
    pub depreciated_improvement_value: Money,
    /// Optional comparable land sales for cross-validation.
    pub comparable_land_sales: Option<Vec<ComparableLandSale>>,
}

/// A single comparable land sale.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparableLandSale {
    pub address: String,
    pub sale_price: Money,
    /// Land area in SF.
    pub land_area_sf: Decimal,
}

/// Output of land residual extraction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LandResidualOutput {
    pub residual_land_value: Money,
    pub comparable_avg_price_psf: Option<Money>,
    pub comparable_implied_land_value: Option<Money>,
    pub variance_to_comps: Option<Rate>,
    pub warnings: Vec<String>,
}

/// Input for Marshall & Swift cost estimation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarshallSwiftInput {
    pub building_class: BuildingClass,
    pub occupancy_type: OccupancyType,
    /// Gross building area in SF.
    pub gross_area_sf: Decimal,
    /// Current cost multiplier (time adjustment from base year).
    pub current_cost_multiplier: Decimal,
    /// Local cost modifier (geographic adjustment, 1.0 = national average).
    pub local_cost_modifier: Decimal,
    /// Height / story multiplier (1.0 for 1-3 stories).
    pub height_multiplier: Decimal,
    /// Perimeter multiplier (shape complexity, 1.0 = rectangular).
    pub perimeter_multiplier: Decimal,
    /// Sprinkler add-on cost per SF.
    pub sprinkler_cost_psf: Money,
    /// HVAC premium as a fraction of base cost (e.g. 0.05 = 5%).
    pub hvac_premium_pct: Rate,
    /// Optional depreciation schedule to compute RCN less depreciation.
    pub depreciation: Option<DepreciationScheduleInput>,
}

/// Output of Marshall & Swift cost estimation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarshallSwiftOutput {
    pub building_class: BuildingClass,
    pub occupancy_type: OccupancyType,
    pub base_cost_psf: Money,
    pub adjusted_cost_psf: Money,
    pub gross_area_sf: Decimal,
    pub replacement_cost_new: Money,
    pub depreciation_detail: Option<DepreciationScheduleOutput>,
    pub rcn_less_depreciation: Option<Money>,
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Maximum depreciation as a fraction of RCN (95% cap, 5% residual floor).
const MAX_DEPRECIATION_PCT: Decimal = dec!(0.95);

// ---------------------------------------------------------------------------
// Public Functions
// ---------------------------------------------------------------------------

/// Full cost approach valuation.
///
/// `indicated_value = land_value + (RCN + entrepreneurial_incentive - total_depreciation)`
///
/// RE-CONTRACT-008: Total depreciation is capped at 95% of RCN.
/// RE-CONTRACT-009: All monetary values in `rust_decimal::Decimal`.
pub fn cost_approach(
    input: &CostApproachInput,
) -> CorpFinanceResult<ComputationOutput<CostApproachOutput>> {
    let start = Instant::now();
    let mut warnings = Vec::new();

    // Validate inputs
    if input.replacement_cost_new <= dec!(0) {
        return Err(CorpFinanceError::InvalidInput {
            field: "replacement_cost_new".into(),
            reason: "RCN must be positive".into(),
        });
    }
    if input.land_value < dec!(0) {
        return Err(CorpFinanceError::InvalidInput {
            field: "land_value".into(),
            reason: "Land value cannot be negative".into(),
        });
    }
    if input.gross_area_sf <= dec!(0) {
        return Err(CorpFinanceError::InvalidInput {
            field: "gross_area_sf".into(),
            reason: "Gross area must be positive".into(),
        });
    }
    if input.physical_depreciation_pct < dec!(0) || input.physical_depreciation_pct > dec!(1) {
        return Err(CorpFinanceError::InvalidInput {
            field: "physical_depreciation_pct".into(),
            reason: "Physical depreciation percentage must be between 0 and 1".into(),
        });
    }

    let incentive = input.entrepreneurial_incentive.unwrap_or(dec!(0));
    let rcn_plus = input.replacement_cost_new + incentive;

    let physical = input.replacement_cost_new * input.physical_depreciation_pct;
    let raw_total = physical + input.functional_obsolescence + input.external_obsolescence;

    // RE-CONTRACT-008: cap total depreciation at 95% of RCN
    let max_dep = input.replacement_cost_new * MAX_DEPRECIATION_PCT;
    let total_depreciation = if raw_total > max_dep {
        warnings.push(format!(
            "Total depreciation ${} exceeds 95% cap; capped at ${}",
            raw_total, max_dep
        ));
        max_dep
    } else {
        raw_total
    };

    let depreciated_improvement = rcn_plus - total_depreciation;
    let indicated_value = input.land_value + depreciated_improvement;
    let value_per_sf = indicated_value / input.gross_area_sf;
    let total_dep_pct = if input.replacement_cost_new > dec!(0) {
        total_depreciation / input.replacement_cost_new
    } else {
        dec!(0)
    };

    let output = CostApproachOutput {
        property_name: input.property_name.clone(),
        cost_type: input.cost_type.clone(),
        land_value: input.land_value,
        replacement_cost_new: input.replacement_cost_new,
        entrepreneurial_incentive: incentive,
        rcn_plus_incentive: rcn_plus,
        physical_depreciation: physical,
        functional_obsolescence: input.functional_obsolescence,
        external_obsolescence: input.external_obsolescence,
        total_depreciation,
        depreciated_improvement_value: depreciated_improvement,
        indicated_value,
        value_per_sf,
        total_depreciation_pct: total_dep_pct,
    };

    let elapsed = start.elapsed().as_micros() as u64;
    Ok(with_metadata(
        "Cost Approach (USPAP / Appraisal Institute)",
        &serde_json::json!({
            "cost_type": format!("{:?}", input.cost_type),
            "depreciation_cap": "95% of RCN",
            "residual_floor": "5% of RCN",
        }),
        warnings,
        elapsed,
        output,
    ))
}

/// Three-tier depreciation schedule: physical + functional + external.
///
/// - Physical: age-life method = effective_age / total_economic_life.
/// - Functional: curable (cost_to_cure) + incurable (capitalized rent loss).
/// - External: paired sales percentage of RCN **or** capitalized income loss.
///
/// RE-CONTRACT-008: Total depreciation capped at 95% of RCN.
pub fn depreciation_schedule(
    input: &DepreciationScheduleInput,
) -> CorpFinanceResult<ComputationOutput<DepreciationScheduleOutput>> {
    let start = Instant::now();
    let mut warnings = Vec::new();

    if input.replacement_cost_new <= dec!(0) {
        return Err(CorpFinanceError::InvalidInput {
            field: "replacement_cost_new".into(),
            reason: "RCN must be positive".into(),
        });
    }
    if input.total_economic_life <= dec!(0) {
        return Err(CorpFinanceError::InvalidInput {
            field: "total_economic_life".into(),
            reason: "Total economic life must be positive".into(),
        });
    }
    if input.effective_age < dec!(0) {
        return Err(CorpFinanceError::InvalidInput {
            field: "effective_age".into(),
            reason: "Effective age cannot be negative".into(),
        });
    }
    if input.cap_rate <= dec!(0) {
        return Err(CorpFinanceError::InvalidInput {
            field: "cap_rate".into(),
            reason: "Cap rate must be positive for capitalization".into(),
        });
    }

    // 1. Physical: age-life
    let physical_pct = (input.effective_age / input.total_economic_life).min(dec!(1));
    let physical = input.replacement_cost_new * physical_pct;

    // 2. Functional: curable + incurable
    let curable_fn = input.curable_functional;
    let incurable_fn = input.incurable_functional_annual_loss / input.cap_rate;
    let total_functional = curable_fn + incurable_fn;

    // 3. External: paired sales pct OR capitalized income loss
    let external = if let Some(pct) = input.external_paired_sales_pct {
        input.replacement_cost_new * pct
    } else if let Some(annual_loss) = input.external_annual_income_loss {
        annual_loss / input.cap_rate
    } else {
        dec!(0)
    };

    // Sum and cap
    let raw_total = physical + total_functional + external;
    let max_dep = input.replacement_cost_new * MAX_DEPRECIATION_PCT;
    let capped = raw_total > max_dep;
    let total_depreciation = if capped {
        warnings.push(format!(
            "Total depreciation ${} exceeds 95% cap; capped at ${}",
            raw_total, max_dep
        ));
        max_dep
    } else {
        raw_total
    };

    let dep_pct = total_depreciation / input.replacement_cost_new;
    let depreciated_value = input.replacement_cost_new - total_depreciation;

    let output = DepreciationScheduleOutput {
        replacement_cost_new: input.replacement_cost_new,
        physical_depreciation_pct: physical_pct,
        physical_depreciation: physical,
        curable_functional: curable_fn,
        incurable_functional: incurable_fn,
        total_functional,
        external_obsolescence: external,
        total_depreciation,
        total_depreciation_pct: dep_pct,
        depreciated_value,
        capped_at_max: capped,
    };

    let elapsed = start.elapsed().as_micros() as u64;
    Ok(with_metadata(
        "Three-Tier Depreciation (Age-Life / Capitalized Loss)",
        &serde_json::json!({
            "physical_method": "age-life (effective_age / total_economic_life)",
            "functional_curable": "cost to cure",
            "functional_incurable": "capitalized rent loss",
            "external_method": if input.external_paired_sales_pct.is_some() {
                "paired sales percentage"
            } else {
                "capitalized income loss"
            },
            "depreciation_cap": "95% of RCN",
        }),
        warnings,
        elapsed,
        output,
    ))
}

/// Land residual extraction.
///
/// `residual_land_value = total_property_value - depreciated_improvement_value`
///
/// Optionally cross-validates against comparable land sales when provided.
pub fn land_residual(
    input: &LandResidualInput,
) -> CorpFinanceResult<ComputationOutput<LandResidualOutput>> {
    let start = Instant::now();
    let mut warnings = Vec::new();

    if input.total_property_value <= dec!(0) {
        return Err(CorpFinanceError::InvalidInput {
            field: "total_property_value".into(),
            reason: "Total property value must be positive".into(),
        });
    }
    if input.depreciated_improvement_value < dec!(0) {
        return Err(CorpFinanceError::InvalidInput {
            field: "depreciated_improvement_value".into(),
            reason: "Depreciated improvement value cannot be negative".into(),
        });
    }

    let residual = input.total_property_value - input.depreciated_improvement_value;

    if residual < dec!(0) {
        warnings.push(
            "Residual land value is negative; improvements exceed total property value".into(),
        );
    }

    // Cross-validate with comparable land sales
    let (comp_avg_psf, comp_implied, variance) = if let Some(ref comps) =
        input.comparable_land_sales
    {
        if comps.is_empty() {
            (None, None, None)
        } else {
            let valid: Vec<_> = comps.iter().filter(|c| c.land_area_sf > dec!(0)).collect();
            if valid.is_empty() {
                warnings.push("All comparable land sales have zero area".into());
                (None, None, None)
            } else {
                let total_psf: Decimal = valid.iter().map(|c| c.sale_price / c.land_area_sf).sum();
                let avg_psf = total_psf / Decimal::from(valid.len() as u32);

                // For cross-validation we take the average total comp value
                let total_comp_val: Decimal = valid.iter().map(|c| c.sale_price).sum();
                let avg_comp_val = total_comp_val / Decimal::from(valid.len() as u32);

                let var = if avg_comp_val > dec!(0) {
                    (residual - avg_comp_val) / avg_comp_val
                } else {
                    dec!(0)
                };

                if var.abs() > dec!(0.25) {
                    warnings.push(format!(
                            "Residual land value deviates {:.1}% from comparable average; review assumptions",
                            var * dec!(100)
                        ));
                }

                (Some(avg_psf), Some(avg_comp_val), Some(var))
            }
        }
    } else {
        (None, None, None)
    };

    let output = LandResidualOutput {
        residual_land_value: residual,
        comparable_avg_price_psf: comp_avg_psf,
        comparable_implied_land_value: comp_implied,
        variance_to_comps: variance,
        warnings: warnings.clone(),
    };

    let elapsed = start.elapsed().as_micros() as u64;
    Ok(with_metadata(
        "Land Residual Extraction",
        &serde_json::json!({
            "formula": "total_property_value - depreciated_improvement_value",
            "cross_validation": input.comparable_land_sales.is_some(),
        }),
        warnings,
        elapsed,
        output,
    ))
}

/// Marshall & Swift cost estimation.
///
/// Computes replacement cost new (RCN) from base cost tables with adjustments:
/// `adjusted_psf = base_cost * current_cost_multiplier * local_cost_modifier
///                 * height_multiplier * perimeter_multiplier * (1 + hvac_premium_pct)
///                 + sprinkler_cost_psf`
/// `RCN = adjusted_psf * gross_area_sf`
///
/// Optionally applies a depreciation schedule to produce RCN less depreciation.
pub fn marshall_swift(
    input: &MarshallSwiftInput,
) -> CorpFinanceResult<ComputationOutput<MarshallSwiftOutput>> {
    let start = Instant::now();
    let warnings = Vec::new();

    if input.gross_area_sf <= dec!(0) {
        return Err(CorpFinanceError::InvalidInput {
            field: "gross_area_sf".into(),
            reason: "Gross area must be positive".into(),
        });
    }
    if input.current_cost_multiplier <= dec!(0) {
        return Err(CorpFinanceError::InvalidInput {
            field: "current_cost_multiplier".into(),
            reason: "Current cost multiplier must be positive".into(),
        });
    }
    if input.local_cost_modifier <= dec!(0) {
        return Err(CorpFinanceError::InvalidInput {
            field: "local_cost_modifier".into(),
            reason: "Local cost modifier must be positive".into(),
        });
    }

    let base_psf = lookup_base_cost(&input.building_class, &input.occupancy_type)?;

    let adjusted_psf = base_psf
        * input.current_cost_multiplier
        * input.local_cost_modifier
        * input.height_multiplier
        * input.perimeter_multiplier
        * (dec!(1) + input.hvac_premium_pct)
        + input.sprinkler_cost_psf;

    let rcn = adjusted_psf * input.gross_area_sf;

    // Optionally apply depreciation
    let (dep_detail, rcn_less_dep) = if let Some(ref dep_input) = input.depreciation {
        let dep_with_rcn = DepreciationScheduleInput {
            replacement_cost_new: rcn,
            ..dep_input.clone()
        };
        let dep_result = depreciation_schedule(&dep_with_rcn)?;
        let dep_out = dep_result.result;
        let less_dep = dep_out.depreciated_value;
        (Some(dep_out), Some(less_dep))
    } else {
        (None, None)
    };

    let output = MarshallSwiftOutput {
        building_class: input.building_class.clone(),
        occupancy_type: input.occupancy_type.clone(),
        base_cost_psf: base_psf,
        adjusted_cost_psf: adjusted_psf,
        gross_area_sf: input.gross_area_sf,
        replacement_cost_new: rcn,
        depreciation_detail: dep_detail,
        rcn_less_depreciation: rcn_less_dep,
    };

    let elapsed = start.elapsed().as_micros() as u64;
    Ok(with_metadata(
        "Marshall & Swift Cost Estimation",
        &serde_json::json!({
            "base_year": "2024",
            "building_class": format!("{:?}", input.building_class),
            "occupancy_type": format!("{:?}", input.occupancy_type),
            "adjustments": [
                "current_cost_multiplier",
                "local_cost_modifier",
                "height_multiplier",
                "perimeter_multiplier",
                "hvac_premium_pct",
                "sprinkler_cost_psf",
            ],
        }),
        warnings,
        elapsed,
        output,
    ))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    fn default_cost_approach_input() -> CostApproachInput {
        CostApproachInput {
            property_name: "Test Office".into(),
            cost_type: CostType::Replacement,
            land_value: dec!(2_000_000),
            replacement_cost_new: dec!(10_000_000),
            physical_depreciation_pct: dec!(0.20),
            functional_obsolescence: dec!(200_000),
            external_obsolescence: dec!(100_000),
            entrepreneurial_incentive: Some(dec!(500_000)),
            gross_area_sf: dec!(50_000),
        }
    }

    fn default_depreciation_input() -> DepreciationScheduleInput {
        DepreciationScheduleInput {
            replacement_cost_new: dec!(10_000_000),
            effective_age: dec!(10),
            total_economic_life: dec!(50),
            curable_functional: dec!(100_000),
            incurable_functional_annual_loss: dec!(25_000),
            cap_rate: dec!(0.08),
            external_paired_sales_pct: Some(dec!(0.05)),
            external_annual_income_loss: None,
        }
    }

    fn default_marshall_swift_input() -> MarshallSwiftInput {
        MarshallSwiftInput {
            building_class: BuildingClass::A,
            occupancy_type: OccupancyType::Office,
            gross_area_sf: dec!(100_000),
            current_cost_multiplier: dec!(1.12),
            local_cost_modifier: dec!(1.05),
            height_multiplier: dec!(1.0),
            perimeter_multiplier: dec!(1.0),
            sprinkler_cost_psf: dec!(3.50),
            hvac_premium_pct: dec!(0.05),
            depreciation: None,
        }
    }

    // -----------------------------------------------------------------------
    // cost_approach tests
    // -----------------------------------------------------------------------

    #[test]
    fn cost_approach_basic() {
        let input = default_cost_approach_input();
        let result = cost_approach(&input).unwrap();
        let o = &result.result;

        // Physical = 10M * 0.20 = 2M
        assert_eq!(o.physical_depreciation, dec!(2_000_000));
        // Total dep = 2M + 200k + 100k = 2.3M
        assert_eq!(o.total_depreciation, dec!(2_300_000));
        // RCN + incentive = 10.5M
        assert_eq!(o.rcn_plus_incentive, dec!(10_500_000));
        // Depreciated improvement = 10.5M - 2.3M = 8.2M
        assert_eq!(o.depreciated_improvement_value, dec!(8_200_000));
        // Indicated = 2M + 8.2M = 10.2M
        assert_eq!(o.indicated_value, dec!(10_200_000));
        // Value/SF = 10.2M / 50k = 204
        assert_eq!(o.value_per_sf, dec!(204));
    }

    #[test]
    fn cost_approach_no_incentive() {
        let mut input = default_cost_approach_input();
        input.entrepreneurial_incentive = None;
        let result = cost_approach(&input).unwrap();
        assert_eq!(result.result.entrepreneurial_incentive, dec!(0));
        assert_eq!(result.result.rcn_plus_incentive, dec!(10_000_000));
    }

    #[test]
    fn cost_approach_depreciation_cap() {
        let mut input = default_cost_approach_input();
        // Force total > 95%: physical 80% + functional 1M + external 1M = 10M total
        input.physical_depreciation_pct = dec!(0.80);
        input.functional_obsolescence = dec!(1_000_000);
        input.external_obsolescence = dec!(1_000_000);
        let result = cost_approach(&input).unwrap();
        let o = &result.result;

        // Raw = 8M + 1M + 1M = 10M (100%), capped at 9.5M (95%)
        assert_eq!(o.total_depreciation, dec!(9_500_000));
        assert!(result.warnings.len() >= 1);
    }

    #[test]
    fn cost_approach_reproduction_type() {
        let mut input = default_cost_approach_input();
        input.cost_type = CostType::Reproduction;
        let result = cost_approach(&input).unwrap();
        assert_eq!(result.result.cost_type, CostType::Reproduction);
    }

    #[test]
    fn cost_approach_invalid_rcn() {
        let mut input = default_cost_approach_input();
        input.replacement_cost_new = dec!(0);
        let err = cost_approach(&input).unwrap_err();
        assert!(err.to_string().contains("RCN must be positive"));
    }

    #[test]
    fn cost_approach_negative_land() {
        let mut input = default_cost_approach_input();
        input.land_value = dec!(-100);
        let err = cost_approach(&input).unwrap_err();
        assert!(err.to_string().contains("Land value cannot be negative"));
    }

    #[test]
    fn cost_approach_invalid_area() {
        let mut input = default_cost_approach_input();
        input.gross_area_sf = dec!(0);
        let err = cost_approach(&input).unwrap_err();
        assert!(err.to_string().contains("Gross area must be positive"));
    }

    #[test]
    fn cost_approach_invalid_depreciation_pct() {
        let mut input = default_cost_approach_input();
        input.physical_depreciation_pct = dec!(1.5);
        let err = cost_approach(&input).unwrap_err();
        assert!(err.to_string().contains("between 0 and 1"));
    }

    #[test]
    fn cost_approach_zero_depreciation() {
        let mut input = default_cost_approach_input();
        input.physical_depreciation_pct = dec!(0);
        input.functional_obsolescence = dec!(0);
        input.external_obsolescence = dec!(0);
        let result = cost_approach(&input).unwrap();
        assert_eq!(result.result.total_depreciation, dec!(0));
        // Value = land(2M) + RCN(10M) + incentive(500k) = 12.5M
        assert_eq!(result.result.indicated_value, dec!(12_500_000));
    }

    #[test]
    fn cost_approach_methodology() {
        let result = cost_approach(&default_cost_approach_input()).unwrap();
        assert!(result.methodology.contains("Cost Approach"));
    }

    // -----------------------------------------------------------------------
    // depreciation_schedule tests
    // -----------------------------------------------------------------------

    #[test]
    fn depreciation_schedule_basic() {
        let input = default_depreciation_input();
        let result = depreciation_schedule(&input).unwrap();
        let o = &result.result;

        // Physical = 10/50 = 20% = 2M
        assert_eq!(o.physical_depreciation_pct, dec!(0.2));
        assert_eq!(o.physical_depreciation, dec!(2_000_000));
        // Curable functional = 100k
        assert_eq!(o.curable_functional, dec!(100_000));
        // Incurable = 25k / 0.08 = 312,500
        assert_eq!(o.incurable_functional, dec!(312_500));
        // Total functional = 412,500
        assert_eq!(o.total_functional, dec!(412_500));
        // External = 10M * 0.05 = 500k
        assert_eq!(o.external_obsolescence, dec!(500_000));
        // Total = 2M + 412.5k + 500k = 2,912,500
        assert_eq!(o.total_depreciation, dec!(2_912_500));
        assert!(!o.capped_at_max);
    }

    #[test]
    fn depreciation_external_capitalized_income() {
        let mut input = default_depreciation_input();
        input.external_paired_sales_pct = None;
        input.external_annual_income_loss = Some(dec!(40_000));
        let result = depreciation_schedule(&input).unwrap();
        // External = 40k / 0.08 = 500k
        assert_eq!(result.result.external_obsolescence, dec!(500_000));
    }

    #[test]
    fn depreciation_no_external() {
        let mut input = default_depreciation_input();
        input.external_paired_sales_pct = None;
        input.external_annual_income_loss = None;
        let result = depreciation_schedule(&input).unwrap();
        assert_eq!(result.result.external_obsolescence, dec!(0));
    }

    #[test]
    fn depreciation_cap_at_95pct() {
        let mut input = default_depreciation_input();
        input.effective_age = dec!(48); // 96% physical alone
        input.curable_functional = dec!(500_000);
        let result = depreciation_schedule(&input).unwrap();
        let o = &result.result;

        assert!(o.capped_at_max);
        assert_eq!(o.total_depreciation, dec!(9_500_000));
        assert_eq!(o.depreciated_value, dec!(500_000));
    }

    #[test]
    fn depreciation_effective_age_exceeds_life() {
        let mut input = default_depreciation_input();
        input.effective_age = dec!(60); // exceeds 50yr life
        let result = depreciation_schedule(&input).unwrap();
        // Physical capped at 100%
        assert_eq!(result.result.physical_depreciation_pct, dec!(1));
        assert_eq!(result.result.physical_depreciation, dec!(10_000_000));
    }

    #[test]
    fn depreciation_invalid_rcn() {
        let mut input = default_depreciation_input();
        input.replacement_cost_new = dec!(-1);
        assert!(depreciation_schedule(&input).is_err());
    }

    #[test]
    fn depreciation_invalid_life() {
        let mut input = default_depreciation_input();
        input.total_economic_life = dec!(0);
        assert!(depreciation_schedule(&input).is_err());
    }

    #[test]
    fn depreciation_negative_age() {
        let mut input = default_depreciation_input();
        input.effective_age = dec!(-5);
        assert!(depreciation_schedule(&input).is_err());
    }

    #[test]
    fn depreciation_zero_cap_rate() {
        let mut input = default_depreciation_input();
        input.cap_rate = dec!(0);
        assert!(depreciation_schedule(&input).is_err());
    }

    #[test]
    fn depreciation_pct_calculation() {
        let input = default_depreciation_input();
        let result = depreciation_schedule(&input).unwrap();
        let o = &result.result;
        let expected_pct = o.total_depreciation / o.replacement_cost_new;
        assert_eq!(o.total_depreciation_pct, expected_pct);
    }

    // -----------------------------------------------------------------------
    // land_residual tests
    // -----------------------------------------------------------------------

    #[test]
    fn land_residual_basic() {
        let input = LandResidualInput {
            total_property_value: dec!(10_000_000),
            depreciated_improvement_value: dec!(7_000_000),
            comparable_land_sales: None,
        };
        let result = land_residual(&input).unwrap();
        assert_eq!(result.result.residual_land_value, dec!(3_000_000));
        assert!(result.result.comparable_avg_price_psf.is_none());
    }

    #[test]
    fn land_residual_with_comps() {
        let input = LandResidualInput {
            total_property_value: dec!(10_000_000),
            depreciated_improvement_value: dec!(7_000_000),
            comparable_land_sales: Some(vec![
                ComparableLandSale {
                    address: "123 Main".into(),
                    sale_price: dec!(2_800_000),
                    land_area_sf: dec!(20_000),
                },
                ComparableLandSale {
                    address: "456 Oak".into(),
                    sale_price: dec!(3_200_000),
                    land_area_sf: dec!(25_000),
                },
            ]),
        };
        let result = land_residual(&input).unwrap();
        let o = &result.result;
        assert_eq!(o.residual_land_value, dec!(3_000_000));
        assert!(o.comparable_avg_price_psf.is_some());
        assert!(o.comparable_implied_land_value.is_some());
        assert!(o.variance_to_comps.is_some());
    }

    #[test]
    fn land_residual_negative_warns() {
        let input = LandResidualInput {
            total_property_value: dec!(5_000_000),
            depreciated_improvement_value: dec!(6_000_000),
            comparable_land_sales: None,
        };
        let result = land_residual(&input).unwrap();
        assert_eq!(result.result.residual_land_value, dec!(-1_000_000));
        assert!(result
            .result
            .warnings
            .iter()
            .any(|w| w.contains("negative")));
    }

    #[test]
    fn land_residual_large_variance_warning() {
        let input = LandResidualInput {
            total_property_value: dec!(10_000_000),
            depreciated_improvement_value: dec!(7_000_000),
            comparable_land_sales: Some(vec![ComparableLandSale {
                address: "Remote Lot".into(),
                sale_price: dec!(1_000_000), // avg comp = 1M vs residual 3M => 200% var
                land_area_sf: dec!(10_000),
            }]),
        };
        let result = land_residual(&input).unwrap();
        assert!(result
            .result
            .warnings
            .iter()
            .any(|w| w.contains("deviates")));
    }

    #[test]
    fn land_residual_empty_comps() {
        let input = LandResidualInput {
            total_property_value: dec!(10_000_000),
            depreciated_improvement_value: dec!(7_000_000),
            comparable_land_sales: Some(vec![]),
        };
        let result = land_residual(&input).unwrap();
        assert!(result.result.comparable_avg_price_psf.is_none());
    }

    #[test]
    fn land_residual_zero_area_comps() {
        let input = LandResidualInput {
            total_property_value: dec!(10_000_000),
            depreciated_improvement_value: dec!(7_000_000),
            comparable_land_sales: Some(vec![ComparableLandSale {
                address: "Bad Data".into(),
                sale_price: dec!(1_000_000),
                land_area_sf: dec!(0),
            }]),
        };
        let result = land_residual(&input).unwrap();
        assert!(result
            .result
            .warnings
            .iter()
            .any(|w| w.contains("zero area")));
    }

    #[test]
    fn land_residual_invalid_property_value() {
        let input = LandResidualInput {
            total_property_value: dec!(0),
            depreciated_improvement_value: dec!(0),
            comparable_land_sales: None,
        };
        assert!(land_residual(&input).is_err());
    }

    #[test]
    fn land_residual_negative_improvement() {
        let input = LandResidualInput {
            total_property_value: dec!(5_000_000),
            depreciated_improvement_value: dec!(-100),
            comparable_land_sales: None,
        };
        assert!(land_residual(&input).is_err());
    }

    // -----------------------------------------------------------------------
    // marshall_swift tests
    // -----------------------------------------------------------------------

    #[test]
    fn marshall_swift_class_a_office() {
        let input = default_marshall_swift_input();
        let result = marshall_swift(&input).unwrap();
        let o = &result.result;

        assert_eq!(o.base_cost_psf, dec!(185));
        // adjusted = 185 * 1.12 * 1.05 * 1.0 * 1.0 * 1.05 + 3.50
        let expected_psf =
            dec!(185) * dec!(1.12) * dec!(1.05) * dec!(1.0) * dec!(1.0) * dec!(1.05) + dec!(3.50);
        assert_eq!(o.adjusted_cost_psf, expected_psf);
        assert_eq!(o.replacement_cost_new, expected_psf * dec!(100_000));
    }

    #[test]
    fn marshall_swift_class_b_retail() {
        let mut input = default_marshall_swift_input();
        input.building_class = BuildingClass::B;
        input.occupancy_type = OccupancyType::Retail;
        let result = marshall_swift(&input).unwrap();
        assert_eq!(result.result.base_cost_psf, dec!(140));
    }

    #[test]
    fn marshall_swift_class_c_industrial() {
        let mut input = default_marshall_swift_input();
        input.building_class = BuildingClass::C;
        input.occupancy_type = OccupancyType::Industrial;
        let result = marshall_swift(&input).unwrap();
        assert_eq!(result.result.base_cost_psf, dec!(85));
    }

    #[test]
    fn marshall_swift_class_d_multifamily() {
        let mut input = default_marshall_swift_input();
        input.building_class = BuildingClass::D;
        input.occupancy_type = OccupancyType::Multifamily;
        let result = marshall_swift(&input).unwrap();
        assert_eq!(result.result.base_cost_psf, dec!(95));
    }

    #[test]
    fn marshall_swift_class_s_industrial() {
        let mut input = default_marshall_swift_input();
        input.building_class = BuildingClass::S;
        input.occupancy_type = OccupancyType::Industrial;
        let result = marshall_swift(&input).unwrap();
        assert_eq!(result.result.base_cost_psf, dec!(55));
    }

    #[test]
    fn marshall_swift_class_s_multifamily_invalid() {
        let mut input = default_marshall_swift_input();
        input.building_class = BuildingClass::S;
        input.occupancy_type = OccupancyType::Multifamily;
        let err = marshall_swift(&input).unwrap_err();
        assert!(err.to_string().contains("not available"));
    }

    #[test]
    fn marshall_swift_class_s_hospitality_invalid() {
        let mut input = default_marshall_swift_input();
        input.building_class = BuildingClass::S;
        input.occupancy_type = OccupancyType::Hospitality;
        assert!(marshall_swift(&input).is_err());
    }

    #[test]
    fn marshall_swift_with_depreciation() {
        let mut input = default_marshall_swift_input();
        input.depreciation = Some(DepreciationScheduleInput {
            replacement_cost_new: dec!(0), // will be overridden by RCN
            effective_age: dec!(10),
            total_economic_life: dec!(50),
            curable_functional: dec!(0),
            incurable_functional_annual_loss: dec!(0),
            cap_rate: dec!(0.08),
            external_paired_sales_pct: None,
            external_annual_income_loss: None,
        });
        let result = marshall_swift(&input).unwrap();
        let o = &result.result;

        assert!(o.depreciation_detail.is_some());
        assert!(o.rcn_less_depreciation.is_some());
        let dep = o.depreciation_detail.as_ref().unwrap();
        // Physical 20% of the computed RCN
        assert_eq!(dep.physical_depreciation_pct, dec!(0.2));
        let expected_less = o.replacement_cost_new - dep.total_depreciation;
        assert_eq!(o.rcn_less_depreciation.unwrap(), expected_less);
    }

    #[test]
    fn marshall_swift_no_depreciation() {
        let input = default_marshall_swift_input();
        let result = marshall_swift(&input).unwrap();
        assert!(result.result.depreciation_detail.is_none());
        assert!(result.result.rcn_less_depreciation.is_none());
    }

    #[test]
    fn marshall_swift_zero_area() {
        let mut input = default_marshall_swift_input();
        input.gross_area_sf = dec!(0);
        assert!(marshall_swift(&input).is_err());
    }

    #[test]
    fn marshall_swift_zero_multiplier() {
        let mut input = default_marshall_swift_input();
        input.current_cost_multiplier = dec!(0);
        assert!(marshall_swift(&input).is_err());
    }

    #[test]
    fn marshall_swift_zero_local_modifier() {
        let mut input = default_marshall_swift_input();
        input.local_cost_modifier = dec!(0);
        assert!(marshall_swift(&input).is_err());
    }

    #[test]
    fn marshall_swift_hospitality_class_a() {
        let mut input = default_marshall_swift_input();
        input.occupancy_type = OccupancyType::Hospitality;
        let result = marshall_swift(&input).unwrap();
        assert_eq!(result.result.base_cost_psf, dec!(210));
    }

    // -----------------------------------------------------------------------
    // lookup_base_cost tests
    // -----------------------------------------------------------------------

    #[test]
    fn base_cost_all_class_a() {
        assert_eq!(
            lookup_base_cost(&BuildingClass::A, &OccupancyType::Office).unwrap(),
            dec!(185)
        );
        assert_eq!(
            lookup_base_cost(&BuildingClass::A, &OccupancyType::Retail).unwrap(),
            dec!(165)
        );
        assert_eq!(
            lookup_base_cost(&BuildingClass::A, &OccupancyType::Industrial).unwrap(),
            dec!(125)
        );
        assert_eq!(
            lookup_base_cost(&BuildingClass::A, &OccupancyType::Multifamily).unwrap(),
            dec!(175)
        );
        assert_eq!(
            lookup_base_cost(&BuildingClass::A, &OccupancyType::Hospitality).unwrap(),
            dec!(210)
        );
    }

    #[test]
    fn base_cost_class_d_all() {
        assert_eq!(
            lookup_base_cost(&BuildingClass::D, &OccupancyType::Office).unwrap(),
            dec!(95)
        );
        assert_eq!(
            lookup_base_cost(&BuildingClass::D, &OccupancyType::Retail).unwrap(),
            dec!(90)
        );
        assert_eq!(
            lookup_base_cost(&BuildingClass::D, &OccupancyType::Industrial).unwrap(),
            dec!(70)
        );
        assert_eq!(
            lookup_base_cost(&BuildingClass::D, &OccupancyType::Multifamily).unwrap(),
            dec!(95)
        );
        assert_eq!(
            lookup_base_cost(&BuildingClass::D, &OccupancyType::Hospitality).unwrap(),
            dec!(120)
        );
    }

    #[test]
    fn base_cost_class_s_office() {
        assert_eq!(
            lookup_base_cost(&BuildingClass::S, &OccupancyType::Office).unwrap(),
            dec!(75)
        );
    }

    #[test]
    fn base_cost_class_s_zero_entries() {
        assert!(lookup_base_cost(&BuildingClass::S, &OccupancyType::Multifamily).is_err());
        assert!(lookup_base_cost(&BuildingClass::S, &OccupancyType::Hospitality).is_err());
    }

    // -----------------------------------------------------------------------
    // Integration-style / edge cases
    // -----------------------------------------------------------------------

    #[test]
    fn cost_approach_depreciation_pct_boundary_zero() {
        let mut input = default_cost_approach_input();
        input.physical_depreciation_pct = dec!(0);
        input.functional_obsolescence = dec!(0);
        input.external_obsolescence = dec!(0);
        input.entrepreneurial_incentive = None;
        let result = cost_approach(&input).unwrap();
        assert_eq!(result.result.total_depreciation_pct, dec!(0));
    }

    #[test]
    fn cost_approach_depreciation_pct_boundary_max() {
        let mut input = default_cost_approach_input();
        input.physical_depreciation_pct = dec!(1.0);
        input.functional_obsolescence = dec!(0);
        input.external_obsolescence = dec!(0);
        input.entrepreneurial_incentive = None;
        let result = cost_approach(&input).unwrap();
        // 100% physical > 95% cap => capped
        assert_eq!(result.result.total_depreciation, dec!(9_500_000));
    }

    #[test]
    fn depreciation_schedule_methodology_string() {
        let result = depreciation_schedule(&default_depreciation_input()).unwrap();
        assert!(result.methodology.contains("Three-Tier Depreciation"));
    }

    #[test]
    fn marshall_swift_methodology_string() {
        let result = marshall_swift(&default_marshall_swift_input()).unwrap();
        assert!(result.methodology.contains("Marshall & Swift"));
    }

    #[test]
    fn land_residual_methodology_string() {
        let input = LandResidualInput {
            total_property_value: dec!(10_000_000),
            depreciated_improvement_value: dec!(7_000_000),
            comparable_land_sales: None,
        };
        let result = land_residual(&input).unwrap();
        assert!(result.methodology.contains("Land Residual"));
    }
}
