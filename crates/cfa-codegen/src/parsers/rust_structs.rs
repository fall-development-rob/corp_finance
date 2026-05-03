//! Walk `crates/corp-finance-core/src/` and extract every `pub struct` via
//! a syn AST parse.
//!
//! This replaces the regex-based parser in `scripts/gen-zod-schemas.py`.
//! syn handles bracket-balanced generic types correctly, so problematic
//! types like `Vec<(String, Decimal)>` and `HashMap<String, Vec<Decimal>>`
//! parse cleanly without the corner-cases the Python regex hit.
//!
//! Output shape mirrors the Python `index_structs()` return value: a map
//! from struct name to `{ module_path, fields }`. We record the *first*
//! definition we see for a given name (matches `setdefault` semantics).

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Context, Result};
use quote::ToTokens;
use syn::{Attribute, Expr, Fields, Item, ItemEnum, Lit, Meta, Type};
use walkdir::WalkDir;

/// Output of `index_with_paths` — both the by-name index (for parity with
/// the Python script) and the by-fully-qualified-path index (for accurate
/// lookup when struct names collide across modules).
pub struct StructIndex {
    /// First-occurrence-wins by struct name (matches `gen-zod-schemas.py`).
    pub by_name: BTreeMap<String, StructInfo>,
    /// All occurrences keyed by `corp_finance_core::path::to::module::StructName`.
    /// This is the precise map — use it when you have the fully-qualified
    /// `input_type` from the NAPI binding.
    pub by_path: BTreeMap<String, StructInfo>,
    /// First-occurrence-wins by enum name (only enums whose name ends in
    /// `Input` are captured — they're the ones the NAPI bindings reference).
    pub enums_by_name: BTreeMap<String, EnumInfo>,
    /// All enum occurrences keyed by fully-qualified module path.
    pub enums_by_path: BTreeMap<String, EnumInfo>,
}

#[derive(Debug, Clone)]
pub struct FieldInfo {
    pub name: String,
    /// The Rust type as a string, with whitespace normalised to a single
    /// space between tokens. Matches what the Python regex produces.
    pub rust_type: String,
    /// Concatenated `///` doc comments (joined with spaces).
    pub doc: String,
    /// Whether the field has `#[serde(skip_serializing_if = "Option::is_none")]`.
    pub is_serde_skip_none: bool,
}

#[derive(Debug, Clone)]
pub struct StructInfo {
    pub module_path: String,
    pub fields: Vec<FieldInfo>,
}

/// One variant of an enum-shaped Input type, e.g.
/// `MbsAnalyticsInput::PassThrough(PassThroughInput)`.
#[derive(Debug, Clone)]
pub struct EnumVariant {
    /// Variant name (e.g. `PassThrough`).
    pub name: String,
    /// For tuple-style variants like `PassThrough(PassThroughInput)`, the
    /// bare name of the single inner type (e.g. `PassThroughInput`). `None`
    /// for unit variants or variants we can't simply destructure.
    pub inner_type: Option<String>,
}

/// An enum-shaped Input type. Currently we only capture enums whose name
/// ends in `Input` (the ones NAPI bindings dispatch on, like
/// `MbsAnalyticsInput`).
#[derive(Debug, Clone)]
pub struct EnumInfo {
    pub module_path: String,
    pub variants: Vec<EnumVariant>,
    /// If the enum carries `#[serde(tag = "...")]` we capture the tag name
    /// so the emitter can pick a discriminated-union shape. `None` means
    /// the default external tagging applies (`{ "Variant": {...} }`).
    pub serde_tag: Option<String>,
}

/// Walk `core_src` recursively and parse every `.rs` file into both a
/// by-name index (first wins, mirrors Python `setdefault`) and a
/// by-fully-qualified-path index (for accurate lookup when names collide).
///
/// Sorted (BTreeMap) so insertion order doesn't depend on filesystem walk
/// order — important for reproducible output. Note: the Python script
/// uses `Path.rglob` which is filesystem-order; for byte-identical parity
/// we'd need to match that, but the by_path lookup means downstream
/// emitters can sidestep the ambiguity entirely.
pub fn index_with_paths(core_src: &Path) -> Result<StructIndex> {
    let mut by_name: BTreeMap<String, StructInfo> = BTreeMap::new();
    let mut by_path: BTreeMap<String, StructInfo> = BTreeMap::new();
    let mut enums_by_name: BTreeMap<String, EnumInfo> = BTreeMap::new();
    let mut enums_by_path: BTreeMap<String, EnumInfo> = BTreeMap::new();

    // Collect paths first then sort for deterministic order. (WalkDir order
    // is filesystem-dependent.)
    let mut paths: Vec<_> = WalkDir::new(core_src)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("rs"))
        .map(|e| e.into_path())
        .collect();
    paths.sort();

    for path in paths {
        let text = std::fs::read_to_string(&path)
            .with_context(|| format!("read {}", path.display()))?;
        let module_path = compute_module_path(&path, core_src);
        // Don't fail the whole walk if one file has a parse error — the
        // Python regex parser silently skipped those too. Log and continue.
        let file = match syn::parse_file(&text) {
            Ok(f) => f,
            Err(e) => {
                eprintln!(
                    "warning: failed to parse {}: {} (skipping)",
                    path.display(),
                    e
                );
                continue;
            }
        };

        for item in &file.items {
            match item {
                Item::Struct(s) => {
                    let name = s.ident.to_string();
                    let fields = extract_fields(&s.fields);
                    let info = StructInfo {
                        module_path: module_path.clone(),
                        fields,
                    };
                    let fq = format!("{}::{}", module_path, name);
                    by_path.insert(fq, info.clone());
                    // Match Python `setdefault` — first definition wins.
                    by_name.entry(name).or_insert(info);
                }
                Item::Enum(e) => {
                    let name = e.ident.to_string();
                    // Only enum-shaped *Input types. NAPI bindings only
                    // dispatch on the `*Input` naming convention; widening
                    // here would just bloat the index with output enums.
                    if !name.ends_with("Input") {
                        continue;
                    }
                    let variants = extract_enum_variants(e);
                    let serde_tag = extract_serde_tag(&e.attrs);
                    let info = EnumInfo {
                        module_path: module_path.clone(),
                        variants,
                        serde_tag,
                    };
                    let fq = format!("{}::{}", module_path, name);
                    enums_by_path.insert(fq, info.clone());
                    enums_by_name.entry(name).or_insert(info);
                }
                _ => {}
            }
        }
    }

    Ok(StructIndex {
        by_name,
        by_path,
        enums_by_name,
        enums_by_path,
    })
}

/// Backward-compat alias used by tests: just the by-name index.
pub fn index(core_src: &Path) -> Result<BTreeMap<String, StructInfo>> {
    Ok(index_with_paths(core_src)?.by_name)
}

fn compute_module_path(path: &Path, core_src: &Path) -> String {
    let rel = path.strip_prefix(core_src).unwrap_or(path);
    let mut parts: Vec<String> = rel
        .with_extension("")
        .components()
        .filter_map(|c| c.as_os_str().to_str().map(|s| s.to_string()))
        .collect();
    if parts.last().map(String::as_str) == Some("mod") {
        parts.pop();
    }
    let mut out = String::from("corp_finance_core");
    for p in &parts {
        out.push_str("::");
        out.push_str(p);
    }
    out
}

fn extract_fields(fields: &Fields) -> Vec<FieldInfo> {
    let named = match fields {
        Fields::Named(n) => &n.named,
        _ => return Vec::new(),
    };
    let mut out = Vec::with_capacity(named.len());
    for f in named.iter() {
        // Only emit `pub` fields — the Python regex required `pub`.
        if !matches!(f.vis, syn::Visibility::Public(_)) {
            continue;
        }
        let name = f
            .ident
            .as_ref()
            .map(|i| i.to_string())
            .unwrap_or_default();
        if name.is_empty() {
            continue;
        }
        let rust_type = type_to_string(&f.ty);
        let doc = collect_doc(&f.attrs);
        let is_serde_skip_none = has_serde_skip_none(&f.attrs);
        out.push(FieldInfo {
            name,
            rust_type,
            doc,
            is_serde_skip_none,
        });
    }
    out
}

/// Convert a syn `Type` to a string, normalising whitespace so it matches
/// what the Python regex would have captured.
///
/// `to_token_stream().to_string()` adds spaces around every token; we
/// collapse `< T >` → `<T>` and `T ,` → `T,` so types like
/// `Option<Decimal>` round-trip cleanly.
pub fn type_to_string(ty: &Type) -> String {
    let raw = ty.to_token_stream().to_string();
    normalise_type(&raw)
}

fn normalise_type(raw: &str) -> String {
    // syn's `to_token_stream().to_string()` puts spaces around every token,
    // e.g. `Vec < ( String , Decimal ) >`. We collapse this into the form
    // people actually write: `Vec<(String, Decimal)>` — bracket-adjacent
    // spaces dropped, single space after commas, no spaces around `::`.
    let mut out = String::with_capacity(raw.len());
    let chars: Vec<char> = raw.chars().collect();
    for (i, &c) in chars.iter().enumerate() {
        if c != ' ' {
            out.push(c);
            continue;
        }
        let last = out.chars().last();
        let next = chars.get(i + 1).copied();
        let drop = matches!(last, Some('<') | Some('(') | Some('['))
            || matches!(next, Some('>') | Some(')') | Some(']') | Some(','))
            // Drop space before any opening bracket: `Vec < T >` → `Vec<T>`.
            || matches!(next, Some('<') | Some('(') | Some('['))
            // No space around `::` — syn emits `chrono :: NaiveDate`.
            || out.ends_with(':')
            || matches!(next, Some(':'))
            // Drop spaces between adjacent close-then-open brackets like `>>`
            // or `> >` arising from nested generics — Python regex output never
            // had this case anyway, normalising helps compare.
            || (matches!(last, Some('>') | Some(')') | Some(']'))
                && matches!(next, Some('<') | Some('(') | Some('[') | Some('>')));
        if !drop {
            out.push(' ');
        }
    }
    out.trim().to_string()
}

fn collect_doc(attrs: &[Attribute]) -> String {
    let mut parts: Vec<String> = Vec::new();
    for attr in attrs {
        if !attr.path().is_ident("doc") {
            continue;
        }
        if let Meta::NameValue(nv) = &attr.meta {
            if let Expr::Lit(lit) = &nv.value {
                if let Lit::Str(s) = &lit.lit {
                    let v = s.value();
                    // Doc literal usually has a leading space — strip it.
                    let v = v.strip_prefix(' ').unwrap_or(&v).to_string();
                    if !v.is_empty() {
                        parts.push(v);
                    }
                }
            }
        }
    }
    parts.join(" ")
}

/// Extract variants from an enum-shaped Input type. We currently only emit
/// useful information for unit variants and tuple-struct variants with a
/// single inner type — the shapes serde defaults round-trip cleanly. Named
/// (struct-style) variants and multi-field tuples are recorded with no
/// inner type and the emitter falls back to `z.any()` for them.
fn extract_enum_variants(e: &ItemEnum) -> Vec<EnumVariant> {
    let mut out = Vec::with_capacity(e.variants.len());
    for v in e.variants.iter() {
        let name = v.ident.to_string();
        let inner_type = match &v.fields {
            Fields::Unit => None,
            Fields::Unnamed(unnamed) if unnamed.unnamed.len() == 1 => {
                let ty = &unnamed.unnamed.first().unwrap().ty;
                let raw = type_to_string(ty);
                // Take the bare name for the inner type — the inner struct
                // is parsed elsewhere into `by_name`/`by_path`, and the
                // emitter looks it up by bare name.
                let bare = raw.rsplit("::").next().unwrap_or(&raw).to_string();
                Some(bare)
            }
            _ => None,
        };
        out.push(EnumVariant { name, inner_type });
    }
    out
}

/// Look for `#[serde(tag = "...")]` on an enum to detect internal tagging.
/// Returns the tag string when present. We deliberately don't try to handle
/// `#[serde(tag, content)]` (adjacent tagging) — none of the current
/// MbsAnalyticsInput-style enums use it; if we ever add one we'll extend.
fn extract_serde_tag(attrs: &[Attribute]) -> Option<String> {
    for attr in attrs {
        if !attr.path().is_ident("serde") {
            continue;
        }
        let tokens = attr.meta.to_token_stream().to_string();
        // Token stream looks like `serde (tag = "kind")` after to_string().
        // Find `tag =` and pull the next quoted literal.
        if let Some(idx) = tokens.find("tag") {
            let rest = &tokens[idx..];
            if let Some(eq) = rest.find('=') {
                let after_eq = &rest[eq + 1..];
                if let Some(start) = after_eq.find('"') {
                    let from_quote = &after_eq[start + 1..];
                    if let Some(end) = from_quote.find('"') {
                        return Some(from_quote[..end].to_string());
                    }
                }
            }
        }
    }
    None
}

fn has_serde_skip_none(attrs: &[Attribute]) -> bool {
    for attr in attrs {
        if !attr.path().is_ident("serde") {
            continue;
        }
        // Walk the tokens for `skip_serializing_if = "Option::is_none"`
        let tokens = attr.meta.to_token_stream().to_string();
        if tokens.contains("skip_serializing_if") && tokens.contains("Option :: is_none") {
            return true;
        }
        if tokens.contains("skip_serializing_if") && tokens.contains("Option::is_none") {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn normalises_simple_type() {
        let ty: Type = parse_quote! { Decimal };
        assert_eq!(type_to_string(&ty), "Decimal");
    }

    #[test]
    fn normalises_optional_type() {
        let ty: Type = parse_quote! { Option<Decimal> };
        assert_eq!(type_to_string(&ty), "Option<Decimal>");
    }

    #[test]
    fn normalises_vec_of_tuple() {
        // The case that broke the Python regex.
        let ty: Type = parse_quote! { Vec<(String, Decimal)> };
        assert_eq!(type_to_string(&ty), "Vec<(String, Decimal)>");
    }

    #[test]
    fn normalises_hashmap() {
        let ty: Type = parse_quote! { HashMap<String, Decimal> };
        assert_eq!(type_to_string(&ty), "HashMap<String, Decimal>");
    }

    #[test]
    fn normalises_qualified_path() {
        let ty: Type = parse_quote! { chrono::NaiveDate };
        assert_eq!(type_to_string(&ty), "chrono::NaiveDate");
    }

    #[test]
    fn extracts_externally_tagged_enum_variants() {
        let e: ItemEnum = parse_quote! {
            pub enum MbsAnalyticsInput {
                PassThrough(PassThroughInput),
                Oas(OasInput),
                Duration(MbsDurationInput),
            }
        };
        let variants = extract_enum_variants(&e);
        assert_eq!(variants.len(), 3);
        assert_eq!(variants[0].name, "PassThrough");
        assert_eq!(variants[0].inner_type.as_deref(), Some("PassThroughInput"));
        assert_eq!(variants[1].name, "Oas");
        assert_eq!(variants[1].inner_type.as_deref(), Some("OasInput"));
        assert_eq!(variants[2].name, "Duration");
        assert_eq!(variants[2].inner_type.as_deref(), Some("MbsDurationInput"));
        assert_eq!(extract_serde_tag(&e.attrs), None);
    }

    #[test]
    fn extracts_internally_tagged_enum_serde_tag() {
        let e: ItemEnum = parse_quote! {
            #[serde(tag = "kind")]
            pub enum FooInput {
                Bar(BarInput),
                Baz(BazInput),
            }
        };
        assert_eq!(extract_serde_tag(&e.attrs).as_deref(), Some("kind"));
    }

    #[test]
    fn unit_and_struct_variants_have_no_inner_type() {
        let e: ItemEnum = parse_quote! {
            pub enum WeirdInput {
                Bare,
                Pair(A, B),
                Named { x: u32 },
            }
        };
        let variants = extract_enum_variants(&e);
        assert_eq!(variants.len(), 3);
        assert!(variants[0].inner_type.is_none());
        assert!(variants[1].inner_type.is_none());
        assert!(variants[2].inner_type.is_none());
    }

    #[test]
    fn inner_type_is_bare_name_not_qualified() {
        let e: ItemEnum = parse_quote! {
            pub enum QualInput {
                Inner(crate::module::SubInput),
            }
        };
        let variants = extract_enum_variants(&e);
        assert_eq!(variants[0].inner_type.as_deref(), Some("SubInput"));
    }
}
