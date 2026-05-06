//! Managed-agent deployment tooling for CFA agents.
//!
//! Provides three core operations, each as a pure function:
//!
//! - [`validate::validate_manifest`] — schema-validate a cookbook (no network)
//! - [`deploy::build_deploy_payload`] — assemble POST-ready JSON payload (no network)
//! - [`orchestrate::route_event`] — route handoff events in an event loop (no network)
//! - [`sync::sync_skills`] — coverage/freshness audit between cookbooks and skills (no network)
//! - [`list::list_cookbooks`] — enumerate cookbooks by cost tier (no I/O)
//!
//! Follows the pattern established in `crates/corp-finance-core/src/workflows/`.

pub mod deploy;
pub mod list;
pub mod orchestrate;
pub mod sync;
pub mod types;
pub mod validate;
