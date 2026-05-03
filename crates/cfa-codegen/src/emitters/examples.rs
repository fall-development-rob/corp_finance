//! Emit `plugins/cfa-core/examples/<tool>.json` per MCP tool.
//!
//! Mirrors `scripts/gen-examples.py`:
//! - Hand-written rich examples from `data::examples::lookup` win.
//! - Otherwise, synthesise placeholder values from the parsed Input
//!   struct fields (skipping Option<T> and serde skip-if-none).
//! - Tools whose Input struct can't be parsed (enum-shaped, etc.) get
//!   no example file.

use std::path::Path;

use anyhow::{Context, Result};
use serde_json::{json, Map, Value};

use crate::data::examples;
use crate::parsers::napi::NapiBinding;
use crate::parsers::rust_structs::{FieldInfo, StructIndex, StructInfo};

const DECIMAL_LIKE: &[&str] = &["Decimal", "Rate", "Money", "Multiple", "f64", "f32"];
const INT_LIKE: &[&str] = &[
    "u32", "u64", "u16", "u8", "i32", "i64", "i16", "i8", "usize", "isize",
];
const STRING_LIKE: &[&str] = &["String", "&str", "str"];
const DATE_LIKE: &[&str] = &["NaiveDate", "chrono::NaiveDate"];

/// Build a placeholder value for one Rust type expression.
///
/// Mirrors `gen-examples.py:placeholder`. Returns `Value::Null` to mean
/// "skip this field" (matches Python `None`). The caller filters those out.
pub fn placeholder(rust_type: &str) -> Value {
    let t = rust_type.trim();
    let bare = t.rsplit("::").next().unwrap_or(t);

    if t.starts_with("Option<") {
        // Optional fields are omitted entirely from examples.
        return Value::Null;
    }

    if DECIMAL_LIKE.contains(&bare) {
        return Value::String("0.05".to_string());
    }
    if INT_LIKE.contains(&bare) {
        return Value::Number(serde_json::Number::from(1));
    }
    if STRING_LIKE.contains(&bare) {
        return Value::String("EXAMPLE".to_string());
    }
    if bare == "bool" {
        return Value::Bool(false);
    }
    if DATE_LIKE.contains(&bare) {
        return Value::String("2026-01-01".to_string());
    }

    if t.starts_with("Vec<") && t.ends_with('>') {
        let inner = &t[4..t.len() - 1];
        let elem = placeholder(inner);
        if elem.is_null() {
            return Value::Array(Vec::new());
        }
        return Value::Array(vec![elem]);
    }

    if (t.starts_with("HashMap<") || t.starts_with("BTreeMap<")) && t.ends_with('>') {
        return Value::Object(Map::new());
    }

    if t.starts_with('(') {
        // Naive: pair of ["EXAMPLE", "0.0"]
        return json!(["EXAMPLE", "0.0"]);
    }

    // Unknown domain type — emit a placeholder string flagging the type.
    Value::String(format!("<{}>", bare))
}

/// Build a placeholder input JSON from a list of parsed fields. Optional
/// fields and serde skip-if-none fields are dropped.
///
/// Returns a JSON object that preserves field declaration order — this is
/// what the Python script produces (Python dict insertion order).
pub fn synth_example(fields: &[FieldInfo]) -> Value {
    let mut out = Map::new();
    for f in fields {
        if f.rust_type.starts_with("Option<") || f.is_serde_skip_none {
            continue;
        }
        let val = placeholder(&f.rust_type);
        if !val.is_null() {
            out.insert(f.name.clone(), val);
        }
    }
    Value::Object(out)
}

pub struct ExamplesOutput {
    pub written: Vec<(String, Value)>,
    pub rich: usize,
    pub skipped: Vec<String>,
}

pub fn emit(
    napi_bindings: &[NapiBinding],
    structs: &StructIndex,
) -> ExamplesOutput {
    let mut written: Vec<(String, Value)> = Vec::new();
    let mut rich = 0usize;
    let mut skipped: Vec<String> = Vec::new();

    for binding in napi_bindings {
        let struct_name = binding
            .input_type
            .rsplit("::")
            .next()
            .unwrap_or(&binding.input_type);

        if let Some(rich_example) = examples::lookup(&binding.name) {
            written.push((binding.name.clone(), rich_example));
            rich += 1;
            continue;
        }

        // Module-path-aware lookup (correct on duplicate struct names),
        // falling back to by-name (matches Python parity on the common case).
        let info: Option<&StructInfo> = structs
            .by_path
            .get(&binding.input_type)
            .or_else(|| structs.by_name.get(struct_name));

        match info {
            Some(info) if !info.fields.is_empty() => {
                written.push((binding.name.clone(), synth_example(&info.fields)));
            }
            _ => {
                skipped.push(binding.name.clone());
            }
        }
    }

    ExamplesOutput {
        written,
        rich,
        skipped,
    }
}

/// Write each example to `<dir>/<tool>.json` and emit a README pointing at
/// them. Returns the number of files written.
pub fn write_files(
    dir: &Path,
    out: &ExamplesOutput,
    napi_total: usize,
) -> Result<usize> {
    std::fs::create_dir_all(dir)
        .with_context(|| format!("create examples dir {}", dir.display()))?;

    let mut count = 0usize;
    for (tool, value) in &out.written {
        let path = dir.join(format!("{}.json", tool));
        let mut json = serde_json::to_string_pretty(value)?;
        json.push('\n');
        std::fs::write(&path, json)
            .with_context(|| format!("write {}", path.display()))?;
        count += 1;
    }

    let readme = render_readme(napi_total, count, out.rich, out.skipped.len());
    std::fs::write(dir.join("README.md"), readme)?;
    Ok(count)
}

fn render_readme(napi_total: usize, written: usize, rich: usize, skipped: usize) -> String {
    let auto = written - rich;
    format!(
        "# cfa-core MCP tool examples\n\n\
{written} of {napi_total} tools have a sample input here. They live in this directory as `<tool_name>.json` and pair with the per-tool zod schemas in `mcp/src/schemas.ts`.\n\n\
## Two flavours\n\n\
- **{rich} rich examples** — hand-written realistic financial scenarios (WACC for a typical mid-cap, 5-year DCF with 8→4% growth taper, 5x EBITDA LBO, ATM Black-Scholes call, etc.). These run end-to-end through the MCP tool and produce sensible outputs. Use them as starting points for your own analyses. See `scripts/gen-examples.py:RICH_EXAMPLES`.\n\
- **{auto} auto-generated shape examples** — minimum-valid inputs derived from the Rust struct definitions. Optional fields are omitted; required fields get type-appropriate placeholders (`\"0.05\"` for decimals, `\"EXAMPLE\"` for strings, `1` for ints). These pass the zod type check but **most will fail Rust validation** — domain enums (TerminalMethod, OptionType) need real values and business invariants (settlement before maturity, weights summing to 1) need realistic numbers. Treat them as field-name discovery aids, not turnkey inputs.\n\
- **{skipped} tools without examples** — input struct couldn't be auto-extracted (enum-shaped, e.g. `MbsAnalyticsInput`). v0.3 enum-aware parser pass.\n\n\
## Usage\n\n\
```bash\n\
# In any Claude Code conversation:\n\
#   \"Run calculate_wacc with the example from plugins/cfa-core/examples/calculate_wacc.json\"\n\
# Claude reads the JSON and calls the MCP tool with it.\n\n\
# Or pipe directly via the CLI test harness:\n\
cat plugins/cfa-core/examples/build_lbo.json \\\n  | jq '{{input: .}}' \\\n  | scripts/mcp-call cfa-core build_lbo\n\
```\n\n\
## Regenerating\n\n\
```bash\n\
python3 scripts/gen-examples.py\n\
```\n\n\
Re-run after any change to `crates/corp-finance-core/src/` Input structs or after editing `RICH_EXAMPLES` in the script.\n",
        written = written,
        napi_total = napi_total,
        rich = rich,
        auto = auto,
        skipped = skipped,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placeholder_decimal() {
        assert_eq!(placeholder("Decimal"), Value::String("0.05".to_string()));
        assert_eq!(placeholder("f64"), Value::String("0.05".to_string()));
    }

    #[test]
    fn placeholder_option_is_null() {
        assert_eq!(placeholder("Option<Decimal>"), Value::Null);
    }

    #[test]
    fn placeholder_vec() {
        assert_eq!(
            placeholder("Vec<Decimal>"),
            Value::Array(vec![Value::String("0.05".to_string())])
        );
    }

    #[test]
    fn synth_skips_optional() {
        let fields = vec![
            FieldInfo {
                name: "a".into(),
                rust_type: "Decimal".into(),
                doc: String::new(),
                is_serde_skip_none: false,
            },
            FieldInfo {
                name: "b".into(),
                rust_type: "Option<Decimal>".into(),
                doc: String::new(),
                is_serde_skip_none: false,
            },
            FieldInfo {
                name: "c".into(),
                rust_type: "Decimal".into(),
                doc: String::new(),
                is_serde_skip_none: true,
            },
        ];
        let v = synth_example(&fields);
        let obj = v.as_object().unwrap();
        assert_eq!(obj.len(), 1);
        assert!(obj.contains_key("a"));
    }
}
