//! Parse `packages/bindings/src/lib.rs` to extract NAPI tool bindings.
//!
//! This stays regex-based (same as `scripts/gen-wasm-bindings.py:NAPI_RE`)
//! because the NAPI file is mechanically structured: every binding has the
//! exact shape
//!
//! ```text
//! #[napi]
//! pub fn <name>(input_json: String) -> NapiResult<String> {
//!     let input: <InputType> =
//!         serde_json::from_str(&input_json).map_err(to_napi_error)?;
//!     let output = <function>(&input).map_err(to_napi_error)?;
//!     serde_json::to_string(&output).map_err(to_napi_error)
//! }
//! ```
//!
//! Whitespace tolerance follows the relaxed regex from
//! `gen-wasm-bindings.py` (allows wrapped `(\n &input,\n)` arguments).

use anyhow::Result;
use regex::Regex;

/// One parsed NAPI binding.
#[derive(Debug, Clone)]
pub struct NapiBinding {
    /// Tool name (the `pub fn <name>` identifier).
    pub name: String,
    /// Fully-qualified Input struct path (e.g. `corp_finance_core::valuation::wacc::WaccInput`).
    pub input_type: String,
    /// Fully-qualified function path (e.g. `corp_finance_core::valuation::wacc::calculate_wacc`).
    pub fn_path: String,
}

/// Section marker found between bindings (the `// ---- ... ----` blocks).
#[derive(Debug, Clone)]
pub struct Section {
    pub title: String,
}

/// Item interleaved in document order — used to preserve the original
/// section/binding sequence when emitting WASM and TS output.
#[derive(Debug, Clone)]
pub enum Item {
    Section(Section),
    Tool(NapiBinding),
}

/// Parse the full NAPI source, returning every binding and section in
/// document order plus a flat list of just the bindings.
pub fn parse(src: &str) -> Result<Vec<Item>> {
    // Whitespace and the optional trailing comma in `(&input ,)` are tolerant —
    // matches the relaxed Python regex.
    let pattern = concat!(
        r"(?s)#\[napi\]\s+",
        r"pub\s+fn\s+(\w+)\s*\(\s*input_json\s*:\s*String\s*\)\s*->\s*NapiResult<String>\s*\{\s*",
        r"let\s+input\s*:\s*([\w:]+)\s*=\s*",
        r"serde_json::from_str\(\s*&input_json\s*\)\s*\.map_err\(\s*to_napi_error\s*\)\s*\?\s*;\s*",
        r"let\s+output\s*=\s*",
        r"([\w:]+)\s*\(\s*&input\s*,?\s*\)\s*\.map_err\(\s*to_napi_error\s*\)\s*\?\s*;\s*",
        r"serde_json::to_string\(\s*&output\s*\)\s*\.map_err\(\s*to_napi_error\s*\)\s*",
        r"\}",
    );
    let napi_re = Regex::new(pattern)?;
    let section_re = Regex::new(r"(?m)// -+\s*\n// ([^\n]+)\s*\n// -+")?;

    let mut items: Vec<(usize, Item)> = Vec::new();

    for caps in napi_re.captures_iter(src) {
        let m = caps.get(0).unwrap();
        items.push((
            m.start(),
            Item::Tool(NapiBinding {
                name: caps.get(1).unwrap().as_str().to_string(),
                input_type: caps.get(2).unwrap().as_str().to_string(),
                fn_path: caps.get(3).unwrap().as_str().to_string(),
            }),
        ));
    }

    for caps in section_re.captures_iter(src) {
        let m = caps.get(0).unwrap();
        let title = caps.get(1).unwrap().as_str().trim().to_string();
        items.push((m.start(), Item::Section(Section { title })));
    }

    items.sort_by_key(|(pos, _)| *pos);
    Ok(items.into_iter().map(|(_, item)| item).collect())
}

/// Convenience: just the bindings, in document order.
pub fn parse_bindings(src: &str) -> Result<Vec<NapiBinding>> {
    Ok(parse(src)?
        .into_iter()
        .filter_map(|item| match item {
            Item::Tool(t) => Some(t),
            Item::Section(_) => None,
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
// ---------------------------------------------------------------------------
// Valuation
// ---------------------------------------------------------------------------

#[napi]
pub fn calculate_wacc(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::valuation::wacc::WaccInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output =
        corp_finance_core::valuation::wacc::calculate_wacc(&input).map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}
"#;

    #[test]
    fn parses_simple_binding() {
        let items = parse(SAMPLE).unwrap();
        assert_eq!(items.len(), 2);
        match &items[0] {
            Item::Section(s) => assert_eq!(s.title, "Valuation"),
            _ => panic!("expected Section first"),
        }
        match &items[1] {
            Item::Tool(t) => {
                assert_eq!(t.name, "calculate_wacc");
                assert_eq!(t.input_type, "corp_finance_core::valuation::wacc::WaccInput");
                assert_eq!(
                    t.fn_path,
                    "corp_finance_core::valuation::wacc::calculate_wacc"
                );
            }
            _ => panic!("expected Tool second"),
        }
    }
}
