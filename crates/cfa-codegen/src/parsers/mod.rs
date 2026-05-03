//! Parsers for the two source-of-truth Rust files:
//! - NAPI bindings (regex-based — file is mechanically generated)
//! - corp-finance-core struct definitions (syn AST — durable against
//!   bracket-balanced generic types like `Vec<(String, Decimal)>`)

pub mod napi;
pub mod rust_structs;
