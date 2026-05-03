//! Static lookup tables ported from the Python generator scripts.
//!
//! These mirror the constants previously embedded in:
//! - `scripts/gen-mcp-tools.py` (DESCRIPTION_OVERRIDES, SKIP, LEGACY_ALIASES, RUNTIME_BASELINES)
//! - `scripts/gen-examples.py`  (RICH_EXAMPLES)
//! - `scripts/link-skills-tools.py` (EXTERNAL_TOOL_PREFIXES, KNOWN_NON_TOOLS, LEGACY_ALIASES)
//!
//! Keeping them as Rust constants makes them part of the build, so any drift
//! between the data and the emitter logic surfaces as a compile error.

pub mod descriptions;
pub mod examples;
pub mod ext_prefixes;
pub mod runtime_baselines;
pub mod wasm_features;
