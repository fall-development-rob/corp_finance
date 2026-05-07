//! Cost ledger module for the CFA agent runtime.
//!
//! Per ADR-017 (Audit / Cost / Observability / Security) and the Phase 26
//! PRD, this module implements the native `rusqlite`-backed per-surface
//! cost ledger that backs `cfa cost summary`, `cfa cost budget set`, and
//! `cfa cost budget get`.
//!
//! Tier classification is consumed verbatim from the existing
//! `managed_agent::types::CookbookTier` and `mcp_servers::types::McpServerTier`
//! enums (per RUF-COST-002, RUF-COST-INV-002); this module does not
//! introduce a parallel tiering system.
//!
//! All money is integer `i64` cents to keep the ledger exact.

pub mod budget;
pub mod ledger;
pub mod reporter;
pub mod types;

#[cfg(test)]
mod tests;

pub use budget::{check_threshold, get_budget, set_budget, BudgetFilter, BudgetStatus};
pub use ledger::{CostFilter, CostLedger};
pub use reporter::{summary, CostSummary, CostSummaryRow, GroupBy};
pub use types::{
    cost_for, BudgetPeriod, CostBudget, CostEvent, Surface, TierTag, MODEL_RATES_PER_MTOK,
};
