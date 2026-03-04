pub mod rent_roll;
pub mod comparable_sales;
pub mod highest_best_use;
pub mod replacement_cost;
pub mod benchmark;
pub mod acquisition;

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::types::{Money, Rate};

/// Property type classification for institutional real estate.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PropertyType {
    Office,
    Retail,
    Industrial,
    Multifamily,
    Hotel,
    MixedUse,
    DataCenter,
    LifeScience,
    SelfStorage,
    SeniorHousing,
    StudentHousing,
    Other(String),
}

/// Property class based on quality, age, and location.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PropertyClass {
    ClassA,
    ClassB,
    ClassC,
}

/// Geographic market identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Market {
    pub city: String,
    pub submarket: Option<String>,
    pub state_or_region: Option<String>,
    pub country: String,
}

/// Core property summary used across sub-modules.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropertySummary {
    pub name: String,
    pub property_type: PropertyType,
    pub property_class: PropertyClass,
    pub market: Market,
    /// Gross building area in square feet (or square metres).
    pub gross_area_sf: Decimal,
    /// Net rentable area in square feet (or square metres).
    pub net_rentable_area_sf: Decimal,
    /// Year built or substantially renovated.
    pub year_built: i32,
    /// Number of units (apartments) or floors, if applicable.
    pub unit_count: Option<u32>,
}

/// Standard valuation metrics shared across approaches.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValuationMetrics {
    /// Indicated value from a given approach.
    pub indicated_value: Money,
    /// Value per square foot of net rentable area.
    pub value_per_sf: Money,
    /// Implied capitalisation rate (NOI / Value).
    pub implied_cap_rate: Rate,
    /// Going-in yield if different from cap rate.
    pub going_in_yield: Option<Rate>,
}
