//! cfa-codegen — durable replacement for the 6 Python codegen scripts.
//!
//! See `crates/cfa-codegen/README.md` for the architecture overview and
//! the migration path away from `scripts/gen-*.py`.
//!
//! Modules:
//! - `parsers::napi` — extract `(name, input_type, fn_path)` triples from
//!   `packages/bindings/src/lib.rs`.
//! - `parsers::rust_structs` — `syn`-based AST walker over
//!   `crates/corp-finance-core/src/` that captures struct field metadata.
//! - `emitters::{wasm,mcp_tools,zod,examples}` — convert the parsed
//!   intermediate representation into the four artifact strings.
//! - `commands::{diff_schemas,lint_skills}` — operations that do more than
//!   simple emit.
//! - `data::*` — the four lookup tables previously embedded in the Python
//!   scripts (descriptions, examples, runtime baselines, ext_prefixes).

pub mod commands;
pub mod data;
pub mod emitters;
pub mod parsers;
