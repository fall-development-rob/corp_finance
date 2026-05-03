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
use crate::parsers::rust_structs::{EnumInfo, FieldInfo, StructIndex, StructInfo};

pub const HEADER: &str = r#"/**
 * cfa-core MCP tool input schemas.
 *
 * GENERATED FROM crates/corp-finance-core/src/ via `cargo run -p cfa-codegen -- zod-schemas`.
 * Do not edit by hand — re-run the generator if Input structs change.
 *
 * Each entry is a zod schema for one MCP tool's input. Tools without an
 * extractable schema fall back to the passthrough record schema in tools.ts.
 *
 * Tools whose Rust input type is an enum (e.g. `MbsAnalyticsInput` with
 * `PassThrough` / `Oas` / `Duration` variants) emit a `z.union(...)` of
 * single-key wrappers — serde's default external tagging maps each variant
 * to `{ "<Variant>": <inner schema> }`. The inner struct schemas live in
 * `TOOL_SCHEMAS_INNER` since they're not directly callable tools but are
 * referenced from the union.
 */
import { z } from "zod";

// Decimal-like financial values: rust_decimal::Decimal serialises to a string
// via the `serde-with-str` feature, but we accept numbers for ergonomics.
const decimalLike = z
  .union([z.string(), z.number()])
  .describe("Decimal value (string preferred for full precision; number accepted)");

// Schemas for inner structs referenced by enum-shaped tool inputs. These
// are not registered as tools themselves; they exist so the discriminated
// unions in TOOL_SCHEMAS can reference a real schema rather than `z.any()`.
export const TOOL_SCHEMAS_INNER: Record<string, z.ZodObject<Record<string, z.ZodTypeAny>>> = {
"#;

pub const HEADER_MIDDLE: &str = r#"};

// Permits both struct-shaped (z.object) and enum-shaped (z.union of
// single-key wrappers) tool inputs.
export type ToolSchema = z.ZodTypeAny;

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
        lines.push(format!("    {}: {},", f.name, render_field_zod(f)));
    }
    lines.push("  }),".to_string());
    lines.join("\n")
}

/// Render a single field's zod expression, applying optional-coercion and
/// doc-comment substitution. Shared by struct and inner-struct emitters.
fn render_field_zod(f: &FieldInfo) -> String {
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

    zod
}

/// Render an inner-struct schema entry (used inside `TOOL_SCHEMAS_INNER`).
/// Same shape as `emit_schema` but the keying is by struct name rather
/// than tool name.
pub fn emit_inner_schema(struct_name: &str, fields: &[FieldInfo]) -> String {
    let mut lines: Vec<String> = Vec::new();
    lines.push(format!("  {}: z.object({{", struct_name));
    for f in fields {
        lines.push(format!("    {}: {},", f.name, render_field_zod(f)));
    }
    lines.push("  }),".to_string());
    lines.join("\n")
}

/// Render the union expression for an enum-shaped Input.
///
/// External tagging (serde default): each variant maps to a single-key
/// wrapper object, e.g. `{ "PassThrough": PassThroughInput }`.
///
/// Internal tagging (`#[serde(tag = "...")]`): each variant inlines the
/// inner struct's fields plus a discriminator string. Emitted as a
/// `z.discriminatedUnion(tag, [...])` so zod can pick the right branch
/// before validating.
pub fn emit_enum_schema(tool: &str, enum_info: &EnumInfo) -> String {
    let serde_tag = enum_info.serde_tag.as_deref();
    let mut variant_lines: Vec<String> = Vec::new();
    for v in &enum_info.variants {
        let inner = match &v.inner_type {
            Some(t) => format!("TOOL_SCHEMAS_INNER.{}", t),
            None => "z.object({})".to_string(),
        };
        if let Some(tag) = serde_tag {
            // Internally tagged: merge a literal discriminator with the
            // inner struct fields. We use `z.intersection` rather than
            // `z.discriminatedUnion` so the inner schema's fields validate
            // alongside the literal string tag.
            variant_lines.push(format!(
                "    z.intersection(z.object({{ {}: z.literal(\"{}\") }}), {})",
                tag, v.name, inner
            ));
        } else {
            // Externally tagged (serde default): `{ "Variant": {...} }`.
            variant_lines.push(format!("    z.object({{ {}: {} }})", v.name, inner));
        }
    }

    let mut lines: Vec<String> = Vec::new();
    lines.push(format!("  {}: z.union([", tool));
    for (i, vl) in variant_lines.iter().enumerate() {
        let comma = if i + 1 < variant_lines.len() { "," } else { "" };
        lines.push(format!("{}{}", vl, comma));
    }
    lines.push("  ]),".to_string());
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
    // Tool-name → (struct_name, struct_fields). Struct-shaped tools only.
    let mut schemas: BTreeMap<String, (String, Vec<FieldInfo>)> = BTreeMap::new();
    // Tool-name → enum_info. Enum-shaped tools (e.g. `analyze_mbs`).
    let mut enum_schemas: BTreeMap<String, EnumInfo> = BTreeMap::new();
    // Inner structs referenced by enum variants. We emit these into
    // `TOOL_SCHEMAS_INNER` so the union expressions can reference them
    // by name.
    let mut inner_schemas: BTreeMap<String, Vec<FieldInfo>> = BTreeMap::new();
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
        if let Some(info) = info {
            if !info.fields.is_empty() {
                schemas.insert(
                    binding.name.clone(),
                    (struct_name.clone(), info.fields.clone()),
                );
                continue;
            }
        }

        // Struct lookup failed (or the struct has no public fields). Try
        // the enum index — the input type might be an enum like
        // `MbsAnalyticsInput`.
        let enum_info: Option<&EnumInfo> = structs
            .enums_by_path
            .get(&binding.input_type)
            .or_else(|| structs.enums_by_name.get(&struct_name));
        match enum_info {
            Some(e) if !e.variants.is_empty() => {
                // Pull every inner struct referenced by the enum into
                // `inner_schemas`. Look it up by bare name first
                // (variants record the bare name); if that misses the
                // emitter falls back to `z.any()` later.
                for v in &e.variants {
                    if let Some(inner_name) = &v.inner_type {
                        if !inner_schemas.contains_key(inner_name) {
                            if let Some(inner_struct) = structs.by_name.get(inner_name) {
                                if !inner_struct.fields.is_empty() {
                                    inner_schemas.insert(
                                        inner_name.clone(),
                                        inner_struct.fields.clone(),
                                    );
                                }
                            }
                        }
                    }
                }
                enum_schemas.insert(binding.name.clone(), e.clone());
            }
            _ => missing.push((binding.name.clone(), struct_name)),
        }
    }

    // schemas.ts (sorted by tool name).
    let mut ts = String::with_capacity(128 * 1024);
    ts.push_str(HEADER);
    // TOOL_SCHEMAS_INNER block (sorted by inner struct name for stability).
    for (struct_name, fields) in &inner_schemas {
        ts.push_str(&emit_inner_schema(struct_name, fields));
        ts.push('\n');
    }
    ts.push_str(HEADER_MIDDLE);
    // Merge struct- and enum-shaped tool schemas, sort by tool name so the
    // output stays stable regardless of which variety wins for a given tool.
    let mut all_tool_names: Vec<&String> = schemas.keys().chain(enum_schemas.keys()).collect();
    all_tool_names.sort();
    all_tool_names.dedup();
    for tool in all_tool_names {
        if let Some((_struct, fields)) = schemas.get(tool) {
            ts.push_str(&emit_schema(tool, fields));
            ts.push('\n');
        } else if let Some(e) = enum_schemas.get(tool) {
            ts.push_str(&emit_enum_schema(tool, e));
            ts.push('\n');
        }
    }
    ts.push_str(FOOTER);

    // schemas.json. Use serde_json with preserve_order so we control key order
    // exactly (Python uses insertion order).
    let total_schema_count = schemas.len() + enum_schemas.len();
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
        Value::Number(serde_json::Number::from(total_schema_count)),
    );
    let mut tools_set: BTreeMap<&String, ()> = BTreeMap::new();
    for k in schemas.keys() {
        tools_set.insert(k, ());
    }
    for k in enum_schemas.keys() {
        tools_set.insert(k, ());
    }
    let tools_arr: Vec<Value> = tools_set
        .keys()
        .map(|k| Value::String((*k).clone()))
        .collect();
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
        schema_count: total_schema_count,
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

    #[test]
    fn emits_externally_tagged_enum_union() {
        use crate::parsers::rust_structs::{EnumInfo, EnumVariant};
        let e = EnumInfo {
            module_path: "corp_finance_core::mortgage_analytics::mbs_analytics".to_string(),
            variants: vec![
                EnumVariant {
                    name: "PassThrough".to_string(),
                    inner_type: Some("PassThroughInput".to_string()),
                },
                EnumVariant {
                    name: "Oas".to_string(),
                    inner_type: Some("OasInput".to_string()),
                },
                EnumVariant {
                    name: "Duration".to_string(),
                    inner_type: Some("MbsDurationInput".to_string()),
                },
            ],
            serde_tag: None,
        };
        let s = emit_enum_schema("analyze_mbs", &e);
        assert!(s.contains("analyze_mbs:"));
        assert!(s.contains("z.union(["));
        assert!(s.contains("z.object({ PassThrough: TOOL_SCHEMAS_INNER.PassThroughInput })"));
        assert!(s.contains("z.object({ Oas: TOOL_SCHEMAS_INNER.OasInput })"));
        assert!(s.contains("z.object({ Duration: TOOL_SCHEMAS_INNER.MbsDurationInput })"));
        // Trailing comma after the union, so it slots into TOOL_SCHEMAS.
        assert!(s.trim_end().ends_with("]),"));
    }

    #[test]
    fn emits_internally_tagged_enum_union() {
        use crate::parsers::rust_structs::{EnumInfo, EnumVariant};
        let e = EnumInfo {
            module_path: "corp_finance_core::test::module".to_string(),
            variants: vec![
                EnumVariant {
                    name: "Bar".to_string(),
                    inner_type: Some("BarInput".to_string()),
                },
                EnumVariant {
                    name: "Baz".to_string(),
                    inner_type: Some("BazInput".to_string()),
                },
            ],
            serde_tag: Some("kind".to_string()),
        };
        let s = emit_enum_schema("analyze_foo", &e);
        assert!(s.contains("z.intersection(z.object({ kind: z.literal(\"Bar\") }), TOOL_SCHEMAS_INNER.BarInput)"));
        assert!(s.contains("z.intersection(z.object({ kind: z.literal(\"Baz\") }), TOOL_SCHEMAS_INNER.BazInput)"));
    }

    #[test]
    fn unit_variant_falls_back_to_empty_object() {
        use crate::parsers::rust_structs::{EnumInfo, EnumVariant};
        let e = EnumInfo {
            module_path: "corp_finance_core::test".to_string(),
            variants: vec![EnumVariant {
                name: "Bare".to_string(),
                inner_type: None,
            }],
            serde_tag: None,
        };
        let s = emit_enum_schema("noop", &e);
        assert!(s.contains("z.object({ Bare: z.object({}) })"));
    }
}
