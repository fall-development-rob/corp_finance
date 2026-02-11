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

#[cfg(feature = "crypto")]
pub mod crypto;

#[cfg(feature = "trade_finance")]
pub mod trade_finance;

#[cfg(feature = "structured_products")]
pub mod structured_products;

#[cfg(feature = "municipal")]
pub mod municipal;

#[cfg(feature = "credit_derivatives")]
pub mod credit_derivatives;

#[cfg(feature = "convertibles")]
pub mod convertibles;

#[cfg(feature = "lease_accounting")]
pub mod lease_accounting;

#[cfg(feature = "pension")]
pub mod pension;

#[cfg(feature = "sovereign")]
pub mod sovereign;

#[cfg(feature = "real_options")]
pub mod real_options;

#[cfg(feature = "equity_research")]
pub mod equity_research;

#[cfg(feature = "commodity_trading")]
pub mod commodity_trading;

#[cfg(feature = "quant_strategies")]
pub mod quant_strategies;

#[cfg(feature = "treasury")]
pub mod treasury;

#[cfg(feature = "infrastructure")]
pub mod infrastructure;

#[cfg(feature = "behavioral")]
pub mod behavioral;

#[cfg(feature = "performance_attribution")]
pub mod performance_attribution;

#[cfg(feature = "credit_portfolio")]
pub mod credit_portfolio;

#[cfg(feature = "macro_economics")]
pub mod macro_economics;

#[cfg(feature = "compliance")]
pub mod compliance;

#[cfg(feature = "onshore_structures")]
pub mod onshore_structures;

#[cfg(feature = "offshore_structures")]
pub mod offshore_structures;

#[cfg(feature = "transfer_pricing")]
pub mod transfer_pricing;

#[cfg(feature = "tax_treaty")]
pub mod tax_treaty;

#[cfg(feature = "fatca_crs")]
pub mod fatca_crs;

#[cfg(feature = "substance_requirements")]
pub mod substance_requirements;

#[cfg(feature = "regulatory_reporting")]
pub mod regulatory_reporting;

#[cfg(feature = "aml_compliance")]
pub mod aml_compliance;

pub use error::CorpFinanceError;
pub use types::*;

/// Standard result type for all corp-finance operations
pub type CorpFinanceResult<T> = Result<T, CorpFinanceError>;
