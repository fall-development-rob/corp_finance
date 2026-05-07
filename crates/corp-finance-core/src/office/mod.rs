//! Office / OOXML serialisation (Phase 29 Wave 6 + Wave 7).
//!
//! Wave 6 ships .xlsx; Wave 7 adds .docx. The feature is opt-in via the
//! `office` cargo feature so the headless-CLI stance is preserved by default.

pub mod docx;
pub mod templates;
pub mod types;
pub mod xlsx;

pub use types::{
    CellFormat, CellValue, Chart, ChartKind, ChartSeries, DefinedName, DocBlock, DocSection,
    FormulaCell, FormattedCell, FrozenPanes, SheetSpec, TextRun, WordDocSpec, WorkbookProperties,
    WorkbookSpec, WriteDocResult, WriteWorkbookResult,
};
pub use xlsx::{write_workbook, write_workbook_from_json};
pub use docx::{write_word_doc, write_word_doc_from_json};
