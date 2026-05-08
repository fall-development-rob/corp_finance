use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Top-level description of an Excel workbook to be written.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkbookSpec {
    pub sheets: Vec<SheetSpec>,
    #[serde(default)]
    pub defined_names: Vec<DefinedName>,
    #[serde(default)]
    pub properties: WorkbookProperties,
}

/// Description of one worksheet tab.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct SheetSpec {
    /// Worksheet tab name (max 31 chars per Excel).
    pub name: String,
    /// First row, rendered bold.
    #[serde(default)]
    pub headers: Vec<String>,
    /// Data rows written beneath the headers row.
    #[serde(default)]
    pub rows: Vec<Vec<CellValue>>,
    /// Ad-hoc formula cells (can reference any row/col on the sheet).
    #[serde(default)]
    pub formulas: Vec<FormulaCell>,
    /// Per-column widths in Excel units; empty = auto.
    #[serde(default)]
    pub column_widths: Vec<f64>,
    /// Freeze rows above + cols left of (row, col).
    #[serde(default)]
    pub frozen_panes: Option<FrozenPanes>,
    /// Cell-level format overlays applied after all data/formula writes.
    #[serde(default)]
    pub cell_formats: Vec<FormattedCell>,
    /// Charts to embed in this worksheet.
    #[serde(default)]
    pub charts: Vec<Chart>,
}

/// Freeze row/col coordinates (0-indexed).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FrozenPanes {
    pub row: u32,
    pub col: u32,
}

/// A single cell value in a data row.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CellValue {
    Text { value: String },
    Number { value: f64 },
    /// Decimal preserved on the wire as a string; converted to f64 at the
    /// xlsx cell boundary. Excel is f64-native; this conversion is the
    /// documented exception to the no-f64 invariant.
    Decimal { value: String },
    Bool { value: bool },
    /// RFC3339 timestamp string. Parsed at write time and rendered as an
    /// Excel date number with the format defined by `excel_format`.
    DateTime {
        value: String,
        excel_format: Option<String>,
    },
    Empty,
}

/// A formula written at an explicit (row, col) position on the sheet.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FormulaCell {
    /// 0-indexed worksheet row.
    pub row: u32,
    /// 0-indexed worksheet column.
    pub col: u32,
    /// Excel formula text without the leading `=`. e.g. "SUM(A2:A10)".
    pub formula: String,
    /// Optional cached numeric result. When supplied, opens-without-recalc
    /// shows the value; absent means Excel must recalculate on open.
    pub cached_result: Option<f64>,
}

/// Minimal cell-level format overlay applied after value writes.
/// `num_format` follows Excel number format syntax (e.g. `"$#,##0.00"`, `"0.00%"`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CellFormat {
    #[serde(default)]
    pub num_format: Option<String>,
    #[serde(default)]
    pub bold: bool,
    #[serde(default)]
    pub italic: bool,
}

/// A format overlay applied to a specific (row, col) after all data writes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FormattedCell {
    /// 0-indexed worksheet row.
    pub row: u32,
    /// 0-indexed worksheet column.
    pub col: u32,
    /// Format to apply at this coordinate.
    pub format: CellFormat,
}

/// Chart variety supported by the xlsx writer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChartKind {
    Line,
    Bar,
    Column,
    Pie,
}

/// One data series within a chart, specified via A1-style absolute ranges.
/// Example: `categories_range = "Sheet1!$A$2:$A$10"`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChartSeries {
    /// Series display name.
    pub name: String,
    /// A1 absolute range for the category (X-axis) data.
    pub categories_range: String,
    /// A1 absolute range for the values (Y-axis) data.
    pub values_range: String,
}

/// A chart object embedded in a worksheet at `(anchor_row, anchor_col)`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Chart {
    pub kind: ChartKind,
    /// 0-indexed row of the chart's top-left anchor cell.
    pub anchor_row: u32,
    /// 0-indexed column of the chart's top-left anchor cell.
    pub anchor_col: u32,
    #[serde(default)]
    pub title: Option<String>,
    /// At least one series is required for a valid chart.
    pub series: Vec<ChartSeries>,
}

/// A workbook-level named range / defined name.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DefinedName {
    /// e.g. "WACC", "DCF_FAIR_VALUE"
    pub name: String,
    /// Range in A1 form, e.g. "Sheet1!$B$5" or "Comps!$D$2:$D$10".
    pub range: String,
}

/// Workbook document properties written into the xlsx metadata.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct WorkbookProperties {
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub company: Option<String>,
    #[serde(default)]
    pub subject: Option<String>,
}

/// Returned by [`crate::office::xlsx::write_workbook`] on success.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WriteWorkbookResult {
    pub output_path: PathBuf,
    pub bytes_written: u64,
    /// SHA-256 of the written file, 64 lowercase hex chars.
    pub sha256: String,
    pub sheet_count: usize,
}

// ---------------------------------------------------------------------------
// Phase 29 Wave 7 — Word / .docx types
// ---------------------------------------------------------------------------

/// A styled text run within a paragraph.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextRun {
    pub text: String,
    #[serde(default)]
    pub bold: bool,
    #[serde(default)]
    pub italic: bool,
}

/// A single content block within a document section.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DocBlock {
    Heading { level: u8, text: String },
    Paragraph { runs: Vec<TextRun> },
    Table { headers: Vec<String>, rows: Vec<Vec<String>> },
    BulletList { items: Vec<String> },
    NumberedList { items: Vec<String> },
    PageBreak,
}

/// A contiguous section of content blocks.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DocSection {
    pub blocks: Vec<DocBlock>,
}

/// Top-level description of a Word document to be written.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WordDocSpec {
    pub sections: Vec<DocSection>,
    #[serde(default)]
    pub properties: WorkbookProperties,
}

/// Returned by [`crate::office::docx::write_word_doc`] on success.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WriteDocResult {
    pub output_path: PathBuf,
    pub bytes_written: u64,
    /// SHA-256 of the written file, 64 lowercase hex chars.
    pub sha256: String,
    pub section_count: usize,
}

// ---------------------------------------------------------------------------
// Phase 29 Wave 8 — PowerPoint / .pptx types
// ---------------------------------------------------------------------------

/// A single slide in a deck. Tagged union on `kind` (snake_case).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Slide {
    /// Full-bleed title slide: large title + optional subtitle.
    Title {
        title: String,
        #[serde(default)]
        subtitle: Option<String>,
    },
    /// Section divider slide with a single centred heading.
    Section { heading: String },
    /// Title at top + bulleted body text.
    Content {
        title: String,
        #[serde(default)]
        bullets: Vec<String>,
    },
    /// Title at top + a simple table (header row + N data rows).
    Table {
        title: String,
        headers: Vec<String>,
        rows: Vec<Vec<String>>,
    },
}

/// Top-level description of a PowerPoint deck to be written.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SlideDeckSpec {
    pub slides: Vec<Slide>,
    #[serde(default)]
    pub properties: WorkbookProperties,
}

/// Returned by [`crate::office::pptx::write_slide_deck`] on success.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WriteDeckResult {
    pub output_path: PathBuf,
    pub bytes_written: u64,
    /// SHA-256 of the written file, 64 lowercase hex chars.
    pub sha256: String,
    pub slide_count: usize,
}
