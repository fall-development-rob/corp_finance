//! Phase 29 Wave 13 — Export JSON Schemas for all public cost types.
//!
//! Run with:
//!   cargo test -p corp-finance-core --features schema_gen,cost --test export_cost_schemas
//!
//! Outputs land in `target/schemas/cost/<TypeName>.json`.

#[cfg(all(test, feature = "schema_gen", feature = "cost"))]
mod cost_schemas {
    use corp_finance_core::cost::{
        BudgetPeriod, BudgetStatus, CostBudget, CostSummary, CostSummaryRow, GroupBy, TierTag,
    };
    use corp_finance_core::cost::types::CostEvent;
    use corp_finance_core::surface::Surface;

    fn emit<T: schemars::JsonSchema>(type_name: &str) {
        let settings = schemars::generate::SchemaSettings::default().with(|s| {
            s.inline_subschemas = true;
        });
        let schema = settings.into_generator().into_root_schema_for::<T>();
        let json = serde_json::to_string_pretty(&schema).unwrap();
        std::fs::create_dir_all("target/schemas/cost").unwrap();
        std::fs::write(format!("target/schemas/cost/{}.json", type_name), json).unwrap();
    }

    #[test]
    fn export_surface() { emit::<Surface>("Surface"); }

    #[test]
    fn export_tier_tag() { emit::<TierTag>("TierTag"); }

    #[test]
    fn export_cost_event() { emit::<CostEvent>("CostEvent"); }

    #[test]
    fn export_budget_period() { emit::<BudgetPeriod>("BudgetPeriod"); }

    #[test]
    fn export_cost_budget() { emit::<CostBudget>("CostBudget"); }

    #[test]
    fn export_budget_status() { emit::<BudgetStatus>("BudgetStatus"); }

    #[test]
    fn export_group_by() { emit::<GroupBy>("GroupBy"); }

    #[test]
    fn export_cost_summary_row() { emit::<CostSummaryRow>("CostSummaryRow"); }

    #[test]
    fn export_cost_summary() { emit::<CostSummary>("CostSummary"); }
}
