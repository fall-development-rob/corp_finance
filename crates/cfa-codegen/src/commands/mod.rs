//! CLI subcommands that don't simply emit a generated artifact.
//!
//! `wasm-bindings`, `mcp-tools`, `zod-schemas`, `examples` are direct
//! emitter invocations and live in `main.rs`. The two below have richer
//! logic worth their own modules.

pub mod diff_schemas;
pub mod lint_skills;
