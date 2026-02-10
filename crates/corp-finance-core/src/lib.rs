pub mod error;
pub mod time_value;
pub mod types;

#[cfg(feature = "valuation")]
pub mod valuation;

#[cfg(feature = "credit")]
pub mod credit;

#[cfg(feature = "pe")]
pub mod pe;

#[cfg(feature = "portfolio")]
pub mod portfolio;

#[cfg(feature = "ma")]
pub mod ma;

#[cfg(feature = "jurisdiction")]
pub mod jurisdiction;

#[cfg(feature = "scenarios")]
pub mod scenarios;

#[cfg(feature = "fixed_income")]
pub mod fixed_income;

#[cfg(feature = "derivatives")]
pub mod derivatives;

#[cfg(feature = "three_statement")]
pub mod three_statement;

#[cfg(feature = "monte_carlo")]
pub mod monte_carlo;

#[cfg(feature = "quant_risk")]
pub mod quant_risk;

#[cfg(feature = "restructuring")]
pub mod restructuring;

#[cfg(feature = "real_assets")]
pub mod real_assets;

#[cfg(feature = "fx_commodities")]
pub mod fx_commodities;

#[cfg(feature = "securitization")]
pub mod securitization;

#[cfg(feature = "venture")]
pub mod venture;

#[cfg(feature = "esg")]
pub mod esg;

#[cfg(feature = "regulatory")]
pub mod regulatory;

#[cfg(feature = "insurance")]
pub mod insurance;

#[cfg(feature = "private_credit")]
pub mod private_credit;

#[cfg(feature = "fpa")]
pub mod fpa;

#[cfg(feature = "wealth")]
pub mod wealth;

pub use error::CorpFinanceError;
pub use types::*;

/// Standard result type for all corp-finance operations
pub type CorpFinanceResult<T> = Result<T, CorpFinanceError>;
