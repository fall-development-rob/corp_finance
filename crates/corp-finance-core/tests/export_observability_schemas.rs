//! Phase 29 Wave 13 — Export JSON Schemas for all public observability types.
//!
//! Run with:
//!   cargo test -p corp-finance-core --features schema_gen,observability --test export_observability_schemas
//!
//! Outputs land in `target/schemas/observability/<TypeName>.json`.

#[cfg(all(test, feature = "schema_gen", feature = "observability"))]
mod observability_schemas {
    use corp_finance_core::observability::types::{SpanContext, Surface};

    fn emit<T: schemars::JsonSchema>(type_name: &str) {
        let settings = schemars::generate::SchemaSettings::default().with(|s| {
            s.inline_subschemas = true;
        });
        let schema = settings.into_generator().into_root_schema_for::<T>();
        let json = serde_json::to_string_pretty(&schema).unwrap();
        std::fs::create_dir_all("target/schemas/observability").unwrap();
        std::fs::write(
            format!("target/schemas/observability/{}.json", type_name),
            json,
        )
        .unwrap();
    }

    #[test]
    fn export_surface() {
        emit::<Surface>("Surface");
    }

    #[test]
    fn export_span_context() {
        emit::<SpanContext>("SpanContext");
    }
}
