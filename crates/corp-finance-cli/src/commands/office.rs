//! CLI subcommands for `cfa office <subgroup> <verb>` (Phase 29 Wave 6 + Wave 7 + Wave 8).
//!
//! Leaf verbs:
//! - `cfa office xlsx write --spec <PATH> --out <PATH>`
//!   Reads a WorkbookSpec JSON from `--spec`, writes an .xlsx file to `--out`,
//!   and prints a pretty-printed JSON summary to stdout.
//! - `cfa office docx write --spec <PATH> --out <PATH>`
//!   Reads a WordDocSpec JSON from `--spec`, writes a .docx file to `--out`,
//!   and prints a pretty-printed JSON summary to stdout.
//! - `cfa office pptx write --spec <PATH> --out <PATH>`
//!   Reads a SlideDeckSpec JSON from `--spec`, writes a .pptx file to `--out`,
//!   and prints a pretty-printed JSON summary to stdout.
//! - `cfa office render-template --kind <dcf|comps|lbo|three-statement> --result <PATH>`
//!   Reads a compute-result JSON from `--result`, dispatches to the matching
//!   template generator, and prints the WorkbookSpec as pretty JSON to stdout.

#[cfg(feature = "office")]
mod inner {
    use std::path::Path;

    use clap::{Args, Subcommand, ValueEnum};
    use serde_json::Value;

    /// Top-level arg group for the `office` subcommand group.
    #[derive(Args)]
    pub struct OfficeArgs {
        #[command(subcommand)]
        pub command: OfficeCommands,
    }

    #[derive(Subcommand)]
    pub enum OfficeCommands {
        /// XLSX subcommands
        Xlsx(XlsxArgs),
        /// DOCX subcommands
        Docx(DocxArgs),
        /// PPTX subcommands (Phase 29 Wave 8)
        Pptx(PptxArgs),
        /// Render a compute result into a WorkbookSpec (prints JSON to stdout).
        RenderTemplate(RenderTemplateArgs),
        /// Render a docx deliverable template into a WordDocSpec (Phase 29 Wave 9).
        RenderDocTemplate(RenderDocTemplateArgs),
        /// Render a pptx deliverable template into a SlideDeckSpec (Phase 29 Wave 9).
        RenderDeckTemplate(RenderDeckTemplateArgs),
    }

    #[derive(Args)]
    pub struct XlsxArgs {
        #[command(subcommand)]
        pub command: XlsxCommands,
    }

    #[derive(Subcommand)]
    pub enum XlsxCommands {
        /// Write an .xlsx workbook from a WorkbookSpec JSON file.
        Write(XlsxWriteArgs),
    }

    #[derive(Args)]
    pub struct XlsxWriteArgs {
        /// Path to the WorkbookSpec JSON file.
        #[arg(long)]
        pub spec: String,

        /// Output .xlsx file path.
        #[arg(long)]
        pub out: String,
    }

    #[derive(Args)]
    pub struct DocxArgs {
        #[command(subcommand)]
        pub command: DocxCommands,
    }

    #[derive(Subcommand)]
    pub enum DocxCommands {
        /// Write a .docx document from a WordDocSpec JSON file.
        Write(DocxWriteArgs),
    }

    #[derive(Args)]
    pub struct DocxWriteArgs {
        /// Path to the WordDocSpec JSON file.
        #[arg(long)]
        pub spec: String,

        /// Output .docx file path.
        #[arg(long)]
        pub out: String,
    }

    #[derive(Args)]
    pub struct PptxArgs {
        #[command(subcommand)]
        pub command: PptxCommands,
    }

    #[derive(Subcommand)]
    pub enum PptxCommands {
        /// Write a .pptx slide deck from a SlideDeckSpec JSON file.
        Write(PptxWriteArgs),
    }

    #[derive(Args)]
    pub struct PptxWriteArgs {
        /// Path to the SlideDeckSpec JSON file.
        #[arg(long)]
        pub spec: String,

        /// Output .pptx file path.
        #[arg(long)]
        pub out: String,
    }

    /// Which template to render.
    #[derive(Clone, ValueEnum)]
    pub enum TemplateKindArg {
        Dcf,
        Comps,
        Lbo,
        #[value(name = "three-statement")]
        ThreeStatement,
    }

    #[derive(Args)]
    pub struct RenderTemplateArgs {
        /// Template kind: dcf, comps, lbo, or three-statement.
        #[arg(long)]
        pub kind: TemplateKindArg,

        /// Path to the compute-result JSON file (e.g. DcfOutput, CompsOutput, …).
        #[arg(long)]
        pub result: String,
    }

    #[derive(clap::ValueEnum, Clone)]
    pub enum DocTemplateKindArg {
        #[value(name = "ic-memo")]
        IcMemo,
        #[value(name = "research-init")]
        ResearchInit,
    }

    #[derive(Args)]
    pub struct RenderDocTemplateArgs {
        /// Doc-template kind: ic-memo or research-init.
        #[arg(long)]
        pub kind: DocTemplateKindArg,

        /// Path to the deliverable input JSON (e.g. IcMemoInput, ResearchInitInput).
        #[arg(long)]
        pub input: String,
    }

    #[derive(clap::ValueEnum, Clone)]
    pub enum DeckTemplateKindArg {
        #[value(name = "pitch-deck")]
        PitchDeck,
        #[value(name = "ic-presentation")]
        IcPresentation,
    }

    #[derive(Args)]
    pub struct RenderDeckTemplateArgs {
        /// Deck-template kind: pitch-deck or ic-presentation.
        #[arg(long)]
        pub kind: DeckTemplateKindArg,

        /// Path to the deliverable input JSON (e.g. PitchDeckInput, IcPresentationInput).
        #[arg(long)]
        pub input: String,
    }

    pub fn run_xlsx_write(args: XlsxWriteArgs) -> Result<Value, Box<dyn std::error::Error>> {
        let spec_json = std::fs::read_to_string(&args.spec)
            .map_err(|e| format!("failed to read spec file '{}': {}", args.spec, e))?;

        let result = corp_finance_core::office::write_workbook_from_json(
            &spec_json,
            Path::new(&args.out),
        )?;

        let json = serde_json::to_string_pretty(&result)?;
        let value: Value = serde_json::from_str(&json)?;
        Ok(value)
    }

    pub fn run_docx_write(args: DocxWriteArgs) -> Result<Value, Box<dyn std::error::Error>> {
        let spec_json = std::fs::read_to_string(&args.spec)
            .map_err(|e| format!("failed to read spec file '{}': {}", args.spec, e))?;

        let result = corp_finance_core::office::write_word_doc_from_json(
            &spec_json,
            Path::new(&args.out),
        )?;

        let json = serde_json::to_string_pretty(&result)?;
        let value: Value = serde_json::from_str(&json)?;
        Ok(value)
    }

    pub fn run_pptx_write(args: PptxWriteArgs) -> Result<Value, Box<dyn std::error::Error>> {
        let spec_json = std::fs::read_to_string(&args.spec)
            .map_err(|e| format!("failed to read spec file '{}': {}", args.spec, e))?;

        let result = corp_finance_core::office::write_slide_deck_from_json(
            &spec_json,
            Path::new(&args.out),
        )?;

        let json = serde_json::to_string_pretty(&result)?;
        let value: Value = serde_json::from_str(&json)?;
        Ok(value)
    }

    pub fn run_render_template(args: RenderTemplateArgs) -> Result<Value, Box<dyn std::error::Error>> {
        let result_json = std::fs::read_to_string(&args.result)
            .map_err(|e| format!("failed to read result file '{}': {}", args.result, e))?;

        let kind_str = match args.kind {
            TemplateKindArg::Dcf => "dcf",
            TemplateKindArg::Comps => "comps",
            TemplateKindArg::Lbo => "lbo",
            TemplateKindArg::ThreeStatement => "three_statement",
        };

        let input_json = serde_json::json!({
            "kind": kind_str,
            "result_json": result_json,
        })
        .to_string();

        let workbook =
            corp_finance_core::office::templates::render_template_from_json(&input_json)?;

        let json = serde_json::to_string_pretty(&workbook)?;
        let value: Value = serde_json::from_str(&json)?;
        Ok(value)
    }

    pub fn run_render_doc_template(
        args: RenderDocTemplateArgs,
    ) -> Result<Value, Box<dyn std::error::Error>> {
        let payload_json = std::fs::read_to_string(&args.input)
            .map_err(|e| format!("failed to read input file '{}': {}", args.input, e))?;

        let kind_str = match args.kind {
            DocTemplateKindArg::IcMemo => "ic_memo",
            DocTemplateKindArg::ResearchInit => "research_init",
        };

        let envelope_json = serde_json::json!({
            "kind": kind_str,
            "input_json": payload_json,
        })
        .to_string();

        let doc =
            corp_finance_core::office::templates::render_doc_template_from_json(&envelope_json)?;

        let json = serde_json::to_string_pretty(&doc)?;
        let value: Value = serde_json::from_str(&json)?;
        Ok(value)
    }

    pub fn run_render_deck_template(
        args: RenderDeckTemplateArgs,
    ) -> Result<Value, Box<dyn std::error::Error>> {
        let payload_json = std::fs::read_to_string(&args.input)
            .map_err(|e| format!("failed to read input file '{}': {}", args.input, e))?;

        let kind_str = match args.kind {
            DeckTemplateKindArg::PitchDeck => "pitch_deck",
            DeckTemplateKindArg::IcPresentation => "ic_presentation",
        };

        let envelope_json = serde_json::json!({
            "kind": kind_str,
            "input_json": payload_json,
        })
        .to_string();

        let deck =
            corp_finance_core::office::templates::render_deck_template_from_json(&envelope_json)?;

        let json = serde_json::to_string_pretty(&deck)?;
        let value: Value = serde_json::from_str(&json)?;
        Ok(value)
    }
}

#[cfg(feature = "office")]
pub use inner::{
    DocxCommands, OfficeArgs, OfficeCommands, PptxCommands, XlsxCommands,
    run_docx_write, run_pptx_write, run_render_deck_template, run_render_doc_template,
    run_render_template, run_xlsx_write,
};
