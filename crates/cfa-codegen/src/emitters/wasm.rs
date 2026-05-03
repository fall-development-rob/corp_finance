//! Emit `crates/corp-finance-wasm/src/lib.rs` from parsed NAPI bindings.
//!
//! Output format mirrors `scripts/gen-wasm-bindings.py` byte-for-byte:
//! a fixed HEADER, then alternating section comment blocks and one-line
//! `wasm_tool!(...)` macro invocations in document order, then a fixed
//! FOOTER with the `version()` export.

use crate::parsers::napi::{Item, NapiBinding};

pub const HEADER: &str = r#"//! WebAssembly bindings for corp-finance-core.
//!
//! GENERATED FROM packages/bindings/src/lib.rs via `cargo run -p cfa-codegen -- wasm-bindings`.
//! Do not edit by hand — re-run the generator if NAPI bindings change.
//!
//! Each exported function takes a JSON string (matching the corresponding
//! `*Input` struct in corp-finance-core) and returns a JSON string with the
//! computed `*Output`. The `wasm_tool!` macro keeps each binding to one line.

use wasm_bindgen::prelude::*;

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
    format!("wasm_tool!({}, {}, {});\n", t.name, t.input_type, t.fn_path)
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
