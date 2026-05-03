//! Output emitters for the four generated artifact types.
//!
//! Each module owns its file format and delegates parsing to the
//! `parsers` crate. None of these modules touch the filesystem — they
//! return `String` (or structured types) and the `commands` layer
//! handles writing.

pub mod examples;
pub mod mcp_tools;
pub mod wasm;
pub mod zod;
