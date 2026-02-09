use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// All monetary values. Wraps Decimal to prevent accidental f64 usage.
pub type Money = Decimal;

/// Rates expressed as decimals (0.05 = 5%). Never as percentages.
pub type Rate = Decimal;

/// Multiples (e.g., 8.5x EV/EBITDA)
pub type Multiple = Decimal;

/// Year fractions or counts
pub type Years = Decimal;

/// Currency code
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Currency {
    GBP,
    #[default]
    USD,
    EUR,
    CHF,
    JPY,
    CAD,
    AUD,
    HKD,
    SGD,
    Other(String),
}

/// A single cash flow at a point in time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CashFlow {
    pub date: NaiveDate,
    pub amount: Money,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

/// A series of cash flows
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CashFlowSeries {
    pub flows: Vec<CashFlow>,
    pub currency: Currency,
}

/// A single period in a financial projection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectionPeriod {
    pub year: i32,
    pub label: String,
    pub is_terminal: bool,
}

/// Sensitivity variable specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensitivityVariable {
    pub name: String,
    pub min: Decimal,
    pub max: Decimal,
    pub step: Decimal,
}

/// Scenario definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scenario {
    pub name: String,
    pub probability: Rate,
    pub overrides: serde_json::Value,
}

/// Standard computation output envelope
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputationOutput<T: Serialize> {
    pub result: T,
    pub methodology: String,
    pub assumptions: serde_json::Value,
    pub warnings: Vec<String>,
    pub metadata: ComputationMetadata,
}

/// Metadata for every computation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputationMetadata {
    pub version: String,
    pub computation_time_us: u64,
    pub precision: String,
}

/// Helper to wrap computation results with metadata
pub fn with_metadata<T: Serialize>(
    methodology: &str,
    assumptions: &impl Serialize,
    warnings: Vec<String>,
    elapsed_us: u64,
    result: T,
) -> ComputationOutput<T> {
    ComputationOutput {
        result,
        methodology: methodology.to_string(),
        assumptions: serde_json::to_value(assumptions).unwrap_or_default(),
        warnings,
        metadata: ComputationMetadata {
            version: env!("CARGO_PKG_VERSION").to_string(),
            computation_time_us: elapsed_us,
            precision: "rust_decimal_128bit".to_string(),
        },
    }
}
