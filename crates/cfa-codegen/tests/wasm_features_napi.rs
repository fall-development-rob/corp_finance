//! Integration test: validate the WASM feature gating against the *real*
//! NAPI bindings file.
//!
//! Catches three kinds of drift the unit tests can't see because they use
//! synthetic inputs:
//!
//! 1. A new section header in `packages/bindings/src/lib.rs` that has no
//!    matching `SECTION_FEATURE_MAP` entry.
//! 2. A binding whose `fn_path` derives a feature different from the one
//!    its enclosing section's title implies (typo / wrong section).
//! 3. Any binding whose `fn_path` doesn't begin with a known feature
//!    module — the WASM emitter would silently emit it un-gated.

use std::path::PathBuf;

use cfa_codegen::data::wasm_features::{feature_for_section, normalize_section_title};
use cfa_codegen::emitters::wasm;
use cfa_codegen::parsers::napi::{parse, Item};

fn napi_src() -> String {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // crates/cfa-codegen → repo root → packages/bindings/src/lib.rs
    p.pop(); // crates/
    p.pop(); // repo root
    p.push("packages/bindings/src/lib.rs");
    std::fs::read_to_string(&p)
        .unwrap_or_else(|e| panic!("read {}: {e}", p.display()))
}

#[test]
fn every_napi_binding_has_a_known_feature() {
    let src = napi_src();
    let items = parse(&src).expect("parse NAPI bindings");
    let missing = wasm::tools_without_feature(&items);
    assert!(
        missing.is_empty(),
        "{} NAPI binding(s) lack a feature mapping; first 5: {:?}",
        missing.len(),
        missing
            .iter()
            .take(5)
            .map(|t| (&t.name, &t.fn_path))
            .collect::<Vec<_>>()
    );
}

#[test]
fn section_titles_agree_with_fn_paths() {
    let src = napi_src();
    let items = parse(&src).expect("parse NAPI bindings");
    let mismatches = wasm::section_vs_fn_path_mismatches(&items);
    assert!(
        mismatches.is_empty(),
        "{} section/fn-path mismatch(es): {:?}",
        mismatches.len(),
        mismatches
    );
}

#[test]
fn every_napi_section_has_a_feature_entry() {
    let src = napi_src();
    let items = parse(&src).expect("parse NAPI bindings");
    let missing: Vec<String> = items
        .iter()
        .filter_map(|i| match i {
            Item::Section(s) => Some(s.title.clone()),
            _ => None,
        })
        .filter(|title| feature_for_section(title).is_none())
        .collect();
    assert!(
        missing.is_empty(),
        "{} section(s) missing from SECTION_FEATURE_MAP: {:?}",
        missing.len(),
        missing
            .iter()
            .map(|t| normalize_section_title(t))
            .collect::<Vec<_>>()
    );
}

#[test]
fn regenerated_wasm_lib_has_cfg_attribute_per_tool() {
    let src = napi_src();
    let items = parse(&src).expect("parse NAPI bindings");
    let body = wasm::emit(&items);
    let tool_count = wasm::tool_count(&items);
    // Count line-anchored attribute lines so we don't pick up substring
    // mentions in the file header doc-comment.
    let cfg_count = body
        .lines()
        .filter(|l| l.starts_with("#[cfg(feature = \""))
        .count();
    assert_eq!(
        cfg_count, tool_count,
        "expected {tool_count} #[cfg(feature)] attributes, got {cfg_count}"
    );
}
