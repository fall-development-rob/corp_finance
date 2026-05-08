//! Phase 29 Wave 11 — Export JSON Schemas for all public office types.
//!
//! Run with:
//!   cargo test -p corp-finance-core --features schema_gen,office --test export_office_schemas
//!
//! Outputs land in `target/schemas/office/<TypeName>.json`.

#[cfg(all(test, feature = "schema_gen"))]
mod office_schemas {
    use corp_finance_core::office::{
        CellValue, FormulaCell, FrozenPanes, SheetSpec, DefinedName, WorkbookProperties,
        WorkbookSpec, WriteWorkbookResult, CellFormat, FormattedCell, ChartKind, ChartSeries,
        Chart, TextRun, DocBlock, DocSection, WordDocSpec, WriteDocResult, Slide, SlideDeckSpec,
        WriteDeckResult,
    };
    use corp_finance_core::office::templates::{
        TemplateKind, RenderTemplateInput,
        DocTemplateKind, RenderDocTemplateInput,
        DeckTemplateKind, RenderDeckTemplateInput,
    };

    fn emit<T: schemars::JsonSchema>(type_name: &str) {
        let schema = schemars::schema_for!(T);
        let json = serde_json::to_string_pretty(&schema).unwrap();
        std::fs::create_dir_all("target/schemas/office").unwrap();
        std::fs::write(format!("target/schemas/office/{}.json", type_name), json).unwrap();
    }

    #[test]
    fn export_cell_value() { emit::<CellValue>("CellValue"); }

    #[test]
    fn export_formula_cell() { emit::<FormulaCell>("FormulaCell"); }

    #[test]
    fn export_frozen_panes() { emit::<FrozenPanes>("FrozenPanes"); }

    #[test]
    fn export_sheet_spec() { emit::<SheetSpec>("SheetSpec"); }

    #[test]
    fn export_defined_name() { emit::<DefinedName>("DefinedName"); }

    #[test]
    fn export_workbook_properties() { emit::<WorkbookProperties>("WorkbookProperties"); }

    #[test]
    fn export_workbook_spec() { emit::<WorkbookSpec>("WorkbookSpec"); }

    #[test]
    fn export_write_workbook_result() { emit::<WriteWorkbookResult>("WriteWorkbookResult"); }

    #[test]
    fn export_cell_format() { emit::<CellFormat>("CellFormat"); }

    #[test]
    fn export_formatted_cell() { emit::<FormattedCell>("FormattedCell"); }

    #[test]
    fn export_chart_kind() { emit::<ChartKind>("ChartKind"); }

    #[test]
    fn export_chart_series() { emit::<ChartSeries>("ChartSeries"); }

    #[test]
    fn export_chart() { emit::<Chart>("Chart"); }

    #[test]
    fn export_text_run() { emit::<TextRun>("TextRun"); }

    #[test]
    fn export_doc_block() { emit::<DocBlock>("DocBlock"); }

    #[test]
    fn export_doc_section() { emit::<DocSection>("DocSection"); }

    #[test]
    fn export_word_doc_spec() { emit::<WordDocSpec>("WordDocSpec"); }

    #[test]
    fn export_write_doc_result() { emit::<WriteDocResult>("WriteDocResult"); }

    #[test]
    fn export_slide() { emit::<Slide>("Slide"); }

    #[test]
    fn export_slide_deck_spec() { emit::<SlideDeckSpec>("SlideDeckSpec"); }

    #[test]
    fn export_write_deck_result() { emit::<WriteDeckResult>("WriteDeckResult"); }

    #[test]
    fn export_template_kind() { emit::<TemplateKind>("TemplateKind"); }

    #[test]
    fn export_render_template_input() { emit::<RenderTemplateInput>("RenderTemplateInput"); }

    #[test]
    fn export_doc_template_kind() { emit::<DocTemplateKind>("DocTemplateKind"); }

    #[test]
    fn export_render_doc_template_input() { emit::<RenderDocTemplateInput>("RenderDocTemplateInput"); }

    #[test]
    fn export_deck_template_kind() { emit::<DeckTemplateKind>("DeckTemplateKind"); }

    #[test]
    fn export_render_deck_template_input() { emit::<RenderDeckTemplateInput>("RenderDeckTemplateInput"); }

    #[cfg(feature = "office")]
    mod template_inputs {
        use corp_finance_core::office::templates::{ic_memo::IcMemoInput, research_init::ResearchInitInput, pitch_deck::PitchDeckInput, ic_presentation::IcPresentationInput};

        fn emit<T: schemars::JsonSchema>(type_name: &str) {
            let schema = schemars::schema_for!(T);
            let json = serde_json::to_string_pretty(&schema).unwrap();
            std::fs::create_dir_all("target/schemas/office").unwrap();
            std::fs::write(format!("target/schemas/office/{}.json", type_name), json).unwrap();
        }

        #[test]
        fn export_ic_memo_input() { emit::<IcMemoInput>("IcMemoInput"); }

        #[test]
        fn export_research_init_input() { emit::<ResearchInitInput>("ResearchInitInput"); }

        #[test]
        fn export_pitch_deck_input() { emit::<PitchDeckInput>("PitchDeckInput"); }

        #[test]
        fn export_ic_presentation_input() { emit::<IcPresentationInput>("IcPresentationInput"); }
    }
}
