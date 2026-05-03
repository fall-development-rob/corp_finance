//! Emit `plugins/cfa-core/mcp/src/schemas.{ts,json}` from parsed structs.
//!
//! Mirrors `scripts/gen-zod-schemas.py` semantics:
//! - For each NAPI binding, look up the Input struct.
//! - For each field, map Rust type → zod expression via `rust_to_zod`.
//! - Apply `.optional()` if the type is `Option<T>` or has serde
//!   skip-if-none, deduplicating where the type fallback already added
//!   `.optional()`.
//! - Doc comments become `.describe(...)`. If the field's type fell
//!   through to the `Rust type:` escape hatch, drop that describe in
//!   favour of the doc comment.
//! - Output a `TOOL_SCHEMAS` object literal sorted by tool name.
//! - Also emit a `schemas.json` manifest with the tool list + missing entries.

use std::collections::{BTreeMap, HashSet};

use serde_json::{json, Map, Value};

use crate::parsers::napi::NapiBinding;
use crate::parsers::rust_structs::{FieldInfo, StructIndex, StructInfo};

pub const HEADER: &str = r#"/**
 * cfa-core MCP tool input schemas.
 *
 * GENERATED FROM crates/corp-finance-core/src/ via scripts/gen-zod-schemas.py.
 * Do not edit by hand — re-run the generator if Input structs change.
 *
 * Each entry is a zod schema for one MCP tool's input. Tools without an
 * extractable schema fall back to the passthrough record schema in tools.ts.
 */
import { z } from "zod";

// Decimal-like financial values: rust_decimal::Decimal serialises to a string
// via the `serde-with-str` feature, but we accept numbers for ergonomics.
const decimalLike = z
  .union([z.string(), z.number()])
  .describe("Decimal value (string preferred for full precision; number accepted)");

export type ToolSchema = z.ZodObject<Record<string, z.ZodTypeAny>>;

export const TOOL_SCHEMAS: Record<string, ToolSchema> = {
"#;

pub const FOOTER: &str = "};\n";

const DECIMAL_LIKE: &[&str] = &["Decimal", "Rate", "Money", "Multiple", "f64", "f32"];
const INT_LIKE: &[&str] = &[
    "u32", "u64", "u16", "u8", "i32", "i64", "i16", "i8", "usize", "isize",
];
const STRING_LIKE: &[&str] = &["String", "&str", "str"];
const BOOL_LIKE: &[&str] = &["bool"];
const DATE_LIKE: &[&str] = &[
    "NaiveDate",
    "chrono::NaiveDate",
    "DateTime",
    "chrono::DateTime",
];

/// Map a Rust type string to its zod expression.
///
/// Mirrors `gen-zod-schemas.py:rust_to_zod`.
pub fn rust_to_zod(rust_type: &str) -> String {
    let t = rust_type.trim();

    // Bare name (everything after the last `::`).
    let bare = t.rsplit("::").next().unwrap_or(t);

    if DECIMAL_LIKE.contains(&bare) {
        return "decimalLike".to_string();
    }
    if INT_LIKE.contains(&bare) {
        return "z.number().int()".to_string();
    }
    if STRING_LIKE.contains(&bare) {
        return "z.string()".to_string();
    }
    if BOOL_LIKE.contains(&bare) {
        return "z.boolean()".to_string();
    }
    if DATE_LIKE.contains(&bare) {
        return "z.string().describe(\"ISO-8601 date\")".to_string();
    }

    if let Some(inner) = strip_wrapper(t, "Option<") {
        return format!("{}.optional()", rust_to_zod(inner));
    }
    if let Some(inner) = strip_wrapper(t, "Vec<") {
        return format!("z.array({})", rust_to_zod(inner));
    }
    if let Some(inner) = strip_wrapper(t, "HashMap<") {
        let parts = split_top_level_comma(inner);
        if parts.len() == 2 && STRING_LIKE.contains(&parts[0].trim()) {
            return format!("z.record({})", rust_to_zod(parts[1].trim()));
        }
        return format!(
            "z.record(z.any()).describe(\"HashMap<{}>\")",
            inner
        );
    }
    if let Some(inner) = strip_wrapper(t, "BTreeMap<") {
        let parts = split_top_level_comma(inner);
        if parts.len() == 2 && STRING_LIKE.contains(&parts[0].trim()) {
            return format!("z.record({})", rust_to_zod(parts[1].trim()));
        }
        return format!(
            "z.record(z.any()).describe(\"BTreeMap<{}>\")",
            inner
        );
    }

    if t.starts_with('(') && t.ends_with(')') {
        let inner = &t[1..t.len() - 1];
        let parts = split_top_level_comma(inner);
        if !parts.is_empty() {
            let mapped: Vec<String> = parts
                .iter()
                .map(|p| rust_to_zod(p.trim()))
                .collect();
            return format!("z.tuple([{}])", mapped.join(", "));
        }
    }

    // Unknown / domain type → escape hatch with original type in description.
    let safe = t.replace('`', "'");
    format!("z.any().describe(\"Rust type: {}\")", safe)
}

fn strip_wrapper<'a>(t: &'a str, prefix: &str) -> Option<&'a str> {
    if t.starts_with(prefix) && t.ends_with('>') {
        Some(&t[prefix.len()..t.len() - 1])
    } else {
        None
    }
}

/// Split a comma-separated list, treating angle brackets and parens as
/// nesting markers (the bug the Python regex couldn't handle).
pub fn split_top_level_comma(s: &str) -> Vec<String> {
    let mut parts: Vec<String> = Vec::new();
    let mut depth = 0i32;
    let mut cur = String::new();
    for ch in s.chars() {
        match ch {
            '<' | '(' | '[' => {
                depth += 1;
                cur.push(ch);
            }
            '>' | ')' | ']' => {
                depth -= 1;
                cur.push(ch);
            }
            ',' if depth == 0 => {
                parts.push(std::mem::take(&mut cur));
            }
            _ => cur.push(ch),
        }
    }
    if !cur.is_empty() {
        parts.push(cur);
    }
    parts
}

/// Render one `<tool>: z.object({ ... }),` entry.
pub fn emit_schema(tool: &str, fields: &[FieldInfo]) -> String {
    let mut lines: Vec<String> = Vec::new();
    lines.push(format!("  {}: z.object({{", tool));
    for f in fields {
        let mut zod = rust_to_zod(&f.rust_type);

        // Add .optional() if the field is serde skip-if-none and the zod
        // expression doesn't already include it (Option<T> already does).
        if f.is_serde_skip_none && !zod.contains(".optional()") {
            zod = format!("{}.optional()", zod);
        }

        if !f.doc.is_empty() {
            let doc = escape_doc_for_describe(&f.doc);
            // Drop the placeholder "Rust type: ..." describe added by the
            // unknown-type fallback — the doc comment is more useful.
            if zod.ends_with(".optional()") {
                let base = &zod[..zod.len() - ".optional()".len()];
                let base_clean = strip_rust_type_describe(base);
                zod = format!("{}.describe(\"{}\").optional()", base_clean, doc);
            } else {
                let base_clean = strip_rust_type_describe(&zod);
                zod = format!("{}.describe(\"{}\")", base_clean, doc);
            }
        }

        lines.push(format!("    {}: {},", f.name, zod));
    }
    lines.push("  }),".to_string());
    lines.join("\n")
}

/// Escape doc comment text for embedding inside a zod `.describe("...")` literal.
/// Mirrors the Python order: backslash → backtick → double-quote → newline → CR.
fn escape_doc_for_describe(doc: &str) -> String {
    doc.replace('\\', "\\\\")
        .replace('`', "'")
        .replace('"', "\\\"")
        .replace(['\n', '\r'], " ")
}

/// Remove a single `.describe("Rust type: ...")` substring from a zod
/// expression. Mirrors the Python `re.sub` call.
fn strip_rust_type_describe(s: &str) -> String {
    // Find `.describe("Rust type: ` and the matching closing `")`.
    if let Some(start) = s.find(".describe(\"Rust type: ") {
        // Closing quote-paren follows; find next `")` after start.
        let after = &s[start + ".describe(\"Rust type: ".len()..];
        if let Some(end_in_after) = after.find("\")") {
            let end_idx = start + ".describe(\"Rust type: ".len() + end_in_after + 2;
            let mut out = String::with_capacity(s.len());
            out.push_str(&s[..start]);
            out.push_str(&s[end_idx..]);
            return out;
        }
    }
    s.to_string()
}

/// Emit the full `schemas.ts` body and `schemas.json` manifest.
pub struct ZodOutput {
    pub schemas_ts: String,
    pub schemas_json: String,
    pub tool_count: usize,
    pub schema_count: usize,
    pub missing: Vec<(String, String)>,
}

pub fn emit(
    napi_bindings: &[NapiBinding],
    structs: &StructIndex,
    napi_source_rel: &str,
) -> ZodOutput {
    let mut schemas: BTreeMap<String, (String, Vec<FieldInfo>)> = BTreeMap::new();
    let mut missing: Vec<(String, String)> = Vec::new();
    let mut seen_tools: HashSet<String> = HashSet::new();

    // Match Python iteration: dict iteration is insertion order since 3.7,
    // and gen-zod-schemas.py iterates `napi_inputs.items()` which mirrors
    // NAPI document order. Our `napi_bindings` slice is also document-ordered.
    for binding in napi_bindings {
        if !seen_tools.insert(binding.name.clone()) {
            continue;
        }
        let struct_name = binding
            .input_type
            .rsplit("::")
            .next()
            .unwrap_or(&binding.input_type)
            .to_string();
        // Prefer fully-qualified path lookup (correct when struct names
        // collide across modules); fall back to by-name (matches Python's
        // setdefault behaviour for parity on non-colliding names).
        let info: Option<&StructInfo> = structs
            .by_path
            .get(&binding.input_type)
            .or_else(|| structs.by_name.get(&struct_name));
        match info {
            None => missing.push((binding.name.clone(), struct_name)),
            Some(info) if info.fields.is_empty() => {
                missing.push((binding.name.clone(), struct_name))
            }
            Some(info) => {
                schemas.insert(
                    binding.name.clone(),
                    (struct_name, info.fields.clone()),
                );
            }
        }
    }

    // schemas.ts (sorted by tool name).
    let mut ts = String::with_capacity(128 * 1024);
    ts.push_str(HEADER);
    for (tool, (_struct, fields)) in &schemas {
        ts.push_str(&emit_schema(tool, fields));
        ts.push('\n');
    }
    ts.push_str(FOOTER);

    // schemas.json. Use serde_json with preserve_order so we control key order
    // exactly (Python uses insertion order).
    let mut manifest = Map::new();
    manifest.insert("schema_version".to_string(), Value::String("1.0.0".to_string()));
    manifest.insert(
        "generated_from".to_string(),
        Value::String(napi_source_rel.to_string()),
    );
    manifest.insert(
        "tool_count".to_string(),
        Value::Number(serde_json::Number::from(seen_tools.len())),
    );
    manifest.insert(
        "schema_count".to_string(),
        Value::Number(serde_json::Number::from(schemas.len())),
    );
    let tools_arr: Vec<Value> = schemas.keys().map(|k| Value::String(k.clone())).collect();
    manifest.insert("tools".to_string(), Value::Array(tools_arr));
    let missing_arr: Vec<Value> = missing
        .iter()
        .map(|(t, s)| json!({ "tool": t, "struct": s }))
        .collect();
    manifest.insert("missing".to_string(), Value::Array(missing_arr));

    let manifest_value = Value::Object(manifest);
    let mut json_out = serde_json::to_string_pretty(&manifest_value).unwrap();
    json_out.push('\n');

    ZodOutput {
        schemas_ts: ts,
        schemas_json: json_out,
        tool_count: seen_tools.len(),
        schema_count: schemas.len(),
        missing,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_decimal_like() {
        assert_eq!(rust_to_zod("Decimal"), "decimalLike");
        assert_eq!(rust_to_zod("Rate"), "decimalLike");
        assert_eq!(rust_to_zod("f64"), "decimalLike");
    }

    #[test]
    fn maps_option_decimal() {
        assert_eq!(rust_to_zod("Option<Decimal>"), "decimalLike.optional()");
    }

    #[test]
    fn maps_vec_string() {
        assert_eq!(rust_to_zod("Vec<String>"), "z.array(z.string())");
    }

    #[test]
    fn maps_vec_of_tuple() {
        // The case the Python regex couldn't handle.
        let z = rust_to_zod("Vec<(String, Decimal)>");
        assert_eq!(z, "z.array(z.tuple([z.string(), decimalLike]))");
    }

    #[test]
    fn maps_hashmap_string_decimal() {
        let z = rust_to_zod("HashMap<String, Decimal>");
        assert_eq!(z, "z.record(decimalLike)");
    }

    #[test]
    fn maps_qualified_naivedate() {
        let z = rust_to_zod("chrono::NaiveDate");
        assert_eq!(z, "z.string().describe(\"ISO-8601 date\")");
    }

    #[test]
    fn unknown_type_falls_back() {
        let z = rust_to_zod("MyEnum");
        assert_eq!(z, "z.any().describe(\"Rust type: MyEnum\")");
    }

    #[test]
    fn doc_replaces_rust_type_describe() {
        let f = FieldInfo {
            name: "country".to_string(),
            rust_type: "Country".to_string(),
            doc: "ISO country code.".to_string(),
            is_serde_skip_none: false,
        };
        let s = emit_schema("test", std::slice::from_ref(&f));
        assert!(s.contains("z.any().describe(\"ISO country code.\")"));
        assert!(!s.contains("Rust type:"));
    }

    #[test]
    fn doc_with_optional_serde_skip() {
        let f = FieldInfo {
            name: "country".to_string(),
            rust_type: "Country".to_string(),
            doc: "ISO code.".to_string(),
            is_serde_skip_none: true,
        };
        let s = emit_schema("test", std::slice::from_ref(&f));
        assert!(s.contains("z.any().describe(\"ISO code.\").optional()"));
    }
}
