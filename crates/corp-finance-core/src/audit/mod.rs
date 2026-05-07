//! Audit subsystem for surface-event provenance (Phase 26 / ADR-017).
//!
//! Two responsibilities:
//!
//! - [`surface_audit`] — djb2 audit-hash function library over a canonical
//!   surface-event manifest. The hash is the identity primitive for "did the
//!   surface event manifest change between this run and that run". Algorithm
//!   is byte-identical to the workflow-level djb2 in
//!   `corp_finance_core::workflows::audit` (ADR-009).
//! - [`audit_manifest`] — `<output_path>.audit.json` companion-file writer.
//!   Triggered by the plugin `Write`/`Edit` PostToolUse hook on output files
//!   that contain numeric recommendations, per ADR-017 §2.
//!
//! Both submodules are pure functions: no I/O outside [`audit_manifest`]'s
//! disk writer, no global state, no async. Wiring into the CLI / MCP / plugin
//! surfaces happens in the binding crates (handled post-build).
//!
//! Feature gate: this whole module is exposed only when the `audit` Cargo
//! feature is enabled. The `audit` feature pulls in `sha2` (output content
//! integrity hashing) and `uuid` (run-id generation).

pub mod audit_manifest;
pub mod surface_audit;

pub use audit_manifest::{
    output_path_to_audit_path, read_audit_manifest, write_audit_manifest, AuditManifest,
    ToolCallRecord,
};
pub use surface_audit::{compute_surface_audit_hash, verify_audit_hash, Surface, SurfaceManifest};
