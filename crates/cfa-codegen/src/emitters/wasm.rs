//! Emit `crates/corp-finance-wasm/src/lib.rs` from parsed NAPI bindings.
//!
//! Output format: a fixed HEADER, then alternating section comment blocks
//! and one-line `wasm_tool!(...)` macro invocations in document order, then
//! a fixed FOOTER with the `version()` export.
//!
//! Every emitted `wasm_tool!` line is preceded by a
//! `#[cfg(feature = "<domain>")]` attribute so downstream consumers can
//! build a slim WASM artifact with only the domains they need. The
//! feature name is derived from the first segment of the binding's
//! function path (`corp_finance_core::<domain>::…`) — see
//! `crate::data::wasm_features::feature_for_tool`.

use crate::data::wasm_features::{feature_for_section, feature_for_tool, normalize_section_title};
use crate::parsers::napi::{Item, NapiBinding};

pub const HEADER: &str = r#"//! WebAssembly bindings for corp-finance-core.
//!
//! GENERATED FROM packages/bindings/src/lib.rs via `cargo run -p cfa-codegen -- wasm-bindings`.
//! Do not edit by hand — re-run the generator if NAPI bindings change.
//!
//! Each exported function takes a JSON string (matching the corresponding
//! `*Input` struct in corp-finance-core) and returns a JSON string with the
//! computed `*Output`. The `wasm_tool!` macro keeps each binding to one line.
//!
//! Each binding is gated by `#[cfg(feature = "<domain>")]` so consumers can
//! build a slim WASM artifact with only the domains they need (see this
//! crate's Cargo.toml for the per-domain features and the `slim` preset).

use wasm_bindgen::prelude::*;

// Hand-maintained streaming bindings live here. These wrap *_with_progress
// variants in corp-finance-core and bridge JS callbacks to ProgressSink. The
// generator does not touch streaming.rs, so adding/removing streaming
// functions does not affect this auto-generated file.
mod streaming;

fn err_to_js(e: impl std::fmt::Display) -> JsValue {
    JsValue::from_str(&e.to_string())
}

macro_rules! wasm_tool {
    ($name:ident, $input_path:path, $fn_path:path) => {
        #[wasm_bindgen]
        pub fn $name(input_json: &str) -> Result<String, JsValue> {
            let input: $input_path = serde_json::from_str(input_json).map_err(err_to_js)?;
            let output = $fn_path(&input).map_err(err_to_js)?;
            serde_json::to_string(&output).map_err(err_to_js)
        }
    };
}

"#;

pub const FOOTER: &str = "
#[wasm_bindgen]
pub fn version() -> String {
    env!(\"CARGO_PKG_VERSION\").to_string()
}
";

/// Render the full WASM file body from parsed NAPI items.
///
/// Tools missing a known feature mapping fall back to an *un-gated*
/// `wasm_tool!` line so we never silently drop a binding. The
/// `tools_without_feature()` helper lets callers warn or fail in CI.
pub fn emit(items: &[Item]) -> String {
    let mut out = String::with_capacity(64 * 1024);
    out.push_str(HEADER);

    for item in items {
        match item {
            Item::Section(s) => {
                let bar = "-".repeat(75);
                out.push('\n');
                out.push_str("// ");
                out.push_str(&bar);
                out.push('\n');
                out.push_str("// ");
                out.push_str(&s.title);
                out.push('\n');
                out.push_str("// ");
                out.push_str(&bar);
                out.push_str("\n\n");
            }
            Item::Tool(t) => {
                out.push_str(&render_tool_line(t));
            }
        }
    }

    out.push_str(FOOTER);
    out
}

fn render_tool_line(t: &NapiBinding) -> String {
    match feature_for_tool(&t.fn_path) {
        Some(feature) => format!(
            "#[cfg(feature = \"{}\")]\nwasm_tool!({}, {}, {});\n",
            feature, t.name, t.input_type, t.fn_path
        ),
        None => format!("wasm_tool!({}, {}, {});\n", t.name, t.input_type, t.fn_path),
    }
}

/// Count the tool entries in an items slice (skipping sections).
pub fn tool_count(items: &[Item]) -> usize {
    items
        .iter()
        .filter(|i| matches!(i, Item::Tool(_)))
        .count()
}

/// Count the section headers in an items slice.
pub fn section_count(items: &[Item]) -> usize {
    items
        .iter()
        .filter(|i| matches!(i, Item::Section(_)))
        .count()
}

/// Return any tools whose `fn_path` does not resolve to a known feature.
/// In a healthy generation this slice is empty; if not, the WASM emitter
/// falls back to an un-gated binding (always compiled in) so coverage
/// is preserved while callers fix the mapping.
pub fn tools_without_feature(items: &[Item]) -> Vec<&NapiBinding> {
    items
        .iter()
        .filter_map(|i| match i {
            Item::Tool(t) if feature_for_tool(&t.fn_path).is_none() => Some(t),
            _ => None,
        })
        .collect()
}

/// Return `(section title, fn-derived feature, section-derived feature)`
/// triples where the two disagree. A non-empty result indicates the
/// `SECTION_FEATURE_MAP` table in `data::wasm_features` is out of sync
/// with the actual module layout. Used by the integration test below
/// and surfaced by callers that want a hard failure.
pub fn section_vs_fn_path_mismatches(items: &[Item]) -> Vec<(String, &'static str, &'static str)> {
    let mut current_section: Option<&str> = None;
    let mut mismatches = Vec::new();
    for item in items {
        match item {
            Item::Section(s) => current_section = Some(&s.title),
            Item::Tool(t) => {
                let fn_feat = match feature_for_tool(&t.fn_path) {
                    Some(f) => f,
                    None => continue,
                };
                if let Some(title) = current_section {
                    let normalized = normalize_section_title(title);
                    if let Some(sec_feat) = feature_for_section(title) {
                        if sec_feat != fn_feat {
                            mismatches.push((normalized.to_string(), fn_feat, sec_feat));
                        }
                    }
                }
            }
        }
    }
    mismatches
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parsers::napi::Section;

    fn binding(name: &str, input: &str, fn_path: &str) -> Item {
        Item::Tool(NapiBinding {
            name: name.to_string(),
            input_type: input.to_string(),
            fn_path: fn_path.to_string(),
        })
    }

    fn section(title: &str) -> Item {
        Item::Section(Section {
            title: title.to_string(),
        })
    }

    #[test]
    fn render_tool_line_emits_cfg_for_known_feature() {
        let t = NapiBinding {
            name: "calculate_wacc".to_string(),
            input_type: "corp_finance_core::valuation::wacc::WaccInput".to_string(),
            fn_path: "corp_finance_core::valuation::wacc::calculate_wacc".to_string(),
        };
        let line = render_tool_line(&t);
        assert!(line.starts_with("#[cfg(feature = \"valuation\")]\n"));
        assert!(line.contains("wasm_tool!(calculate_wacc, "));
    }

    #[test]
    fn render_tool_line_falls_back_when_unknown() {
        let t = NapiBinding {
            name: "weird_tool".to_string(),
            input_type: "weird::Input".to_string(),
            fn_path: "weird::path::weird_tool".to_string(),
        };
        let line = render_tool_line(&t);
        assert_eq!(line, "wasm_tool!(weird_tool, weird::Input, weird::path::weird_tool);\n");
    }

    #[test]
    fn tools_without_feature_finds_unknown_paths() {
        let items = vec![
            binding(
                "calculate_wacc",
                "corp_finance_core::valuation::wacc::WaccInput",
                "corp_finance_core::valuation::wacc::calculate_wacc",
            ),
            binding(
                "mystery",
                "corp_finance_core::not_a_module::Input",
                "corp_finance_core::not_a_module::call",
            ),
        ];
        let missing = tools_without_feature(&items);
        assert_eq!(missing.len(), 1);
        assert_eq!(missing[0].name, "mystery");
    }

    #[test]
    fn section_vs_fn_path_mismatches_empty_for_consistent_input() {
        let items = vec![
            section("Valuation"),
            binding(
                "calculate_wacc",
                "corp_finance_core::valuation::wacc::WaccInput",
                "corp_finance_core::valuation::wacc::calculate_wacc",
            ),
            section("Credit"),
            binding(
                "credit_metrics",
                "corp_finance_core::credit::metrics::CreditMetricsInput",
                "corp_finance_core::credit::metrics::calculate_credit_metrics",
            ),
            section("Private Equity"),
            binding(
                "build_lbo",
                "corp_finance_core::pe::lbo::LboInput",
                "corp_finance_core::pe::lbo::build_lbo",
            ),
        ];
        assert!(section_vs_fn_path_mismatches(&items).is_empty());
    }

    #[test]
    fn section_vs_fn_path_mismatches_detects_drift() {
        // "Valuation" section but the binding lives under credit::* —
        // simulates an edit that put a credit tool under the wrong header.
        let items = vec![
            section("Valuation"),
            binding(
                "credit_metrics",
                "corp_finance_core::credit::metrics::CreditMetricsInput",
                "corp_finance_core::credit::metrics::calculate_credit_metrics",
            ),
        ];
        let mm = section_vs_fn_path_mismatches(&items);
        assert_eq!(mm.len(), 1);
        assert_eq!(mm[0].1, "credit"); // fn-derived (truth)
        assert_eq!(mm[0].2, "valuation"); // section-derived (wrong)
    }

    #[test]
    fn emit_produces_cfg_gated_lines() {
        let items = vec![
            section("Valuation"),
            binding(
                "calculate_wacc",
                "corp_finance_core::valuation::wacc::WaccInput",
                "corp_finance_core::valuation::wacc::calculate_wacc",
            ),
        ];
        let body = emit(&items);
        assert!(body.contains("#[cfg(feature = \"valuation\")]\nwasm_tool!(calculate_wacc,"));
    }
}
