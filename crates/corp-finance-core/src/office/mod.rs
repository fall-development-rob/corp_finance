//! Office / OOXML serialisation (Phase 29 Wave 6).
//!
//! Wave 6 ships .xlsx; .docx and .pptx defer to later waves. The feature
//! is opt-in via the `office` cargo feature so the headless-CLI stance
//! is preserved by default.

pub mod templates;
pub mod types;
pub mod xlsx;

pub use types::{
    CellFormat, CellValue, Chart, ChartKind, ChartSeries, DefinedName, FormulaCell,
    FormattedCell, FrozenPanes, SheetSpec, WorkbookProperties, WorkbookSpec, WriteWorkbookResult,
};
pub use xlsx::{write_workbook, write_workbook_from_json};
