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

#[cfg(feature = "trading")]
pub mod trading;

pub use error::CorpFinanceError;
pub use types::*;

/// Standard result type for all corp-finance operations
pub type CorpFinanceResult<T> = Result<T, CorpFinanceError>;
