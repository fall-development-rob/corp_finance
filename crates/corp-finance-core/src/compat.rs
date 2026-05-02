//! Cross-platform compatibility shims.
//!
//! `std::time::Instant::now()` panics on `wasm32-unknown-unknown` (compiles to
//! `unreachable`). `web-time` provides a drop-in replacement that delegates to
//! `performance.now()` in browsers and to the JS host's clock in Node.
//!
//! All modules use `crate::compat::Instant` instead of `std::time::Instant` so
//! the same source compiles cleanly for native, NAPI, and WASM targets.

#[cfg(not(target_arch = "wasm32"))]
pub use std::time::Instant;

#[cfg(target_arch = "wasm32")]
pub use web_time::Instant;
