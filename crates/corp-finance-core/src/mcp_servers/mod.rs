//! MCP-server tier registry for CFA agents.
//!
//! Mirrors the cookbook-tier pattern in `crate::managed_agent` but for the
//! upstream MCP servers themselves. Lets a free user discover which MCP
//! servers they can use without paying for a vendor subscription:
//!
//! - `FreeNative`             — runs offline (cfa-core)
//! - `FreePublicWithApiKey`   — free public data, some sources need a free API key
//! - `Freemium`               — vendor offers a free tier (FMP)
//! - `PaidVendor`             — requires a paid vendor subscription
//!
//! Pure data + a single pure list function. No I/O.

pub mod list;
pub mod types;
