//! Phase 29 Wave 13 — Export JSON Schemas for all public audit-domain types.
//!
//! Run with:
//!   cargo test -p corp-finance-core --features schema_gen,audit --test export_audit_schemas
//!
//! Outputs land in `target/schemas/audit/<TypeName>.json`.

#[cfg(all(test, feature = "schema_gen", feature = "audit"))]
mod audit_schemas {
    use corp_finance_core::audit::{AuditManifest, ToolCallRecord};
    use corp_finance_core::audit::surface_audit::{Surface, SurfaceManifest};

    fn emit<T: schemars::JsonSchema>(type_name: &str) {
        let settings = schemars::generate::SchemaSettings::default().with(|s| {
            s.inline_subschemas = true;
        });
        let schema = settings.into_generator().into_root_schema_for::<T>();
        let json = serde_json::to_string_pretty(&schema).unwrap();
        std::fs::create_dir_all("target/schemas/audit").unwrap();
        std::fs::write(format!("target/schemas/audit/{}.json", type_name), json).unwrap();
    }

    #[test]
    fn export_surface() { emit::<Surface>("Surface"); }

    #[test]
    fn export_surface_manifest() { emit::<SurfaceManifest>("SurfaceManifest"); }

    #[test]
    fn export_tool_call_record() { emit::<ToolCallRecord>("ToolCallRecord"); }

    #[test]
    fn export_audit_manifest() { emit::<AuditManifest>("AuditManifest"); }
}
