//! Security bounded context for the CFA agent platform (Phase 26).
//!
//! Implements ADR-017 §5 (PII / prompt-injection hardening) and the
//! `feature_audit_observability.yml` security contracts (RUF-SEC-001..006
//! and the two RUF-SEC-INV invariants).
//!
//! Two scanners ship in this module:
//!
//! - [`pii_scanner::scan_for_pii`] — 14-category PII detector.
//!   Categories defined in [`types::PiiCategory`]; see PRD-phase26 FR-26-09.
//!   Categories with checksums (CreditCard / Iban / RoutingNumber) use the
//!   regex as a candidate filter and a real verifier (Luhn / mod-97 / ABA)
//!   as the gate.
//! - [`injection_detector::detect_injection`] — heuristic prompt-injection
//!   detector. Four kinds in [`types::InjectionKind`]: jailbreak,
//!   ignore-previous, role-switch, system-prompt-leak.
//!
//! Both scanners return `Vec<Finding>` ordered by `span_start` ascending
//! and are deterministic (RUF-SEC-005). Spans are well-formed
//! (RUF-SEC-006).
//!
//! ## Wiring (handled post-build)
//!
//! The plugin hook layer at `plugins/cfa-core/hooks/hooks.json` invokes
//! the `surface_pii_scan` MCP tool (which calls these functions) at three
//! hook points: `PreToolUse`, `PreMemoryWrite`, and `PostToolUse`. The
//! brief notes that the existing fifth hook (an SSN-only regex) should
//! be replaced with this scanner; see the build report for the proposed
//! `hooks.json` JSON.
//!
//! ## Feature gating
//!
//! The whole module is gated behind the `security` cargo feature at the
//! crate root (`lib.rs`). Internal files do not repeat the gate.

pub mod injection_detector;
pub mod pii_scanner;
pub mod types;

#[cfg(test)]
mod tests;

pub use injection_detector::detect_injection;
pub use pii_scanner::scan_for_pii;
pub use types::{Finding, FindingCategory, FindingKind, InjectionKind, PiiCategory, Severity};
