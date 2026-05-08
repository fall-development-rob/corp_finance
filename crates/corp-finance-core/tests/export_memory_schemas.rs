//! Phase 29 Wave 13 — Export JSON Schemas for all public memory types.
//!
//! Run with:
//!   cargo test -p corp-finance-core --features schema_gen,memory --test export_memory_schemas
//!
//! Outputs land in `target/schemas/memory/<TypeName>.json`.

#[cfg(all(test, feature = "schema_gen", feature = "memory"))]
mod memory_schemas {
    use corp_finance_core::memory::{
        CfaSession, EntityKind, EntityRef, MemoryQuery, RunSummary, Surface,
    };

    fn emit<T: schemars::JsonSchema>(type_name: &str) {
        let settings = schemars::generate::SchemaSettings::default().with(|s| {
            s.inline_subschemas = true;
        });
        let schema = settings.into_generator().into_root_schema_for::<T>();
        let json = serde_json::to_string_pretty(&schema).unwrap();
        std::fs::create_dir_all("target/schemas/memory").unwrap();
        std::fs::write(format!("target/schemas/memory/{}.json", type_name), json).unwrap();
    }

    #[test]
    fn export_surface() {
        emit::<Surface>("Surface");
    }

    #[test]
    fn export_entity_kind() {
        emit::<EntityKind>("EntityKind");
    }

    #[test]
    fn export_entity_ref() {
        emit::<EntityRef>("EntityRef");
    }

    #[test]
    fn export_run_summary() {
        emit::<RunSummary>("RunSummary");
    }

    #[test]
    fn export_cfa_session() {
        emit::<CfaSession>("CfaSession");
    }

    #[test]
    fn export_memory_query() {
        emit::<MemoryQuery>("MemoryQuery");
    }
}
