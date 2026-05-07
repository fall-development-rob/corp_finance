use std::collections::HashSet;
use std::fmt::Write as FmtWrite;
use std::path::Path;

use rust_xlsxwriter::{Chart as XlsxChart, ChartType, ExcelDateTime, Format, Formula, Workbook};
use sha2::{Digest, Sha256};

use crate::error::CorpFinanceError;
use crate::CorpFinanceResult;

use super::types::{CellValue, Chart, ChartKind, FormattedCell, SheetSpec, WorkbookSpec, WriteWorkbookResult};

/// Write a [`WorkbookSpec`] to disk as an .xlsx file.
///
/// # Errors
///
/// - `InvalidInput` when `sheets` is empty, a sheet name exceeds 31 chars,
///   or a duplicate sheet name appears.
/// - `InvalidInput` when a [`CellValue::Decimal`] payload fails `Decimal::from_str`.
/// - `InvalidInput` when a [`CellValue::DateTime`] payload fails RFC 3339 parse.
/// - `SerializationError` on filesystem failures.
pub fn write_workbook(spec: &WorkbookSpec, output_path: &Path) -> CorpFinanceResult<WriteWorkbookResult> {
    validate_spec(spec)?;

    let mut wb = Workbook::new();
    apply_properties(&mut wb, spec);

    for sheet_spec in &spec.sheets {
        write_sheet(&mut wb, sheet_spec)?;
    }

    for dn in &spec.defined_names {
        wb.define_name(&dn.name, &dn.range)
            .map_err(|e| CorpFinanceError::SerializationError(e.to_string()))?;
    }

    let bytes = wb
        .save_to_buffer()
        .map_err(|e| CorpFinanceError::SerializationError(e.to_string()))?;

    std::fs::write(output_path, &bytes)
        .map_err(|e| CorpFinanceError::SerializationError(e.to_string()))?;

    let sha256 = sha256_bytes(&bytes);
    let bytes_written = bytes.len() as u64;

    Ok(WriteWorkbookResult {
        output_path: output_path.to_path_buf(),
        bytes_written,
        sha256,
        sheet_count: spec.sheets.len(),
    })
}

/// Write directly from a JSON-string spec. Convenience for NAPI / CLI callers.
pub fn write_workbook_from_json(spec_json: &str, output_path: &Path) -> CorpFinanceResult<WriteWorkbookResult> {
    let spec: WorkbookSpec = serde_json::from_str(spec_json)?;
    write_workbook(&spec, output_path)
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

fn validate_spec(spec: &WorkbookSpec) -> CorpFinanceResult<()> {
    if spec.sheets.is_empty() {
        return Err(CorpFinanceError::InvalidInput {
            field: "sheets".into(),
            reason: "workbook must contain at least one sheet".into(),
        });
    }
    let mut seen: HashSet<&str> = HashSet::new();
    for sheet in &spec.sheets {
        if sheet.name.len() > 31 {
            return Err(CorpFinanceError::InvalidInput {
                field: "sheet.name".into(),
                reason: format!("sheet name '{}' exceeds 31 characters", sheet.name),
            });
        }
        if !seen.insert(sheet.name.as_str()) {
            return Err(CorpFinanceError::InvalidInput {
                field: "sheet.name".into(),
                reason: format!("duplicate sheet name '{}'", sheet.name),
            });
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Workbook properties
// ---------------------------------------------------------------------------

fn apply_properties(wb: &mut Workbook, spec: &WorkbookSpec) {
    let p = &spec.properties;
    let mut props = rust_xlsxwriter::DocProperties::new();
    if let Some(t) = &p.title {
        props = props.set_title(t);
    }
    if let Some(a) = &p.author {
        props = props.set_author(a);
    }
    if let Some(c) = &p.company {
        props = props.set_company(c);
    }
    if let Some(s) = &p.subject {
        props = props.set_subject(s);
    }
    wb.set_properties(&props);
}

// ---------------------------------------------------------------------------
// Sheet writing
// ---------------------------------------------------------------------------

fn write_sheet(wb: &mut Workbook, spec: &SheetSpec) -> CorpFinanceResult<()> {
    let ws = wb.add_worksheet();
    ws.set_name(&spec.name)
        .map_err(|e| CorpFinanceError::SerializationError(e.to_string()))?;

    let bold_fmt = Format::new().set_bold();

    // Headers in row 0
    if !spec.headers.is_empty() {
        for (col, header) in spec.headers.iter().enumerate() {
            ws.write_string_with_format(0, col as u16, header, &bold_fmt)
                .map_err(|e| CorpFinanceError::SerializationError(e.to_string()))?;
        }
    }

    // Data rows start at row 1 when headers are present, else row 0
    let data_row_offset: u32 = if spec.headers.is_empty() { 0 } else { 1 };

    for (row_idx, row) in spec.rows.iter().enumerate() {
        let xlsx_row = data_row_offset + row_idx as u32;
        for (col_idx, cell) in row.iter().enumerate() {
            write_cell(ws, xlsx_row, col_idx as u16, cell)?;
        }
    }

    // Formula cells
    for fc in &spec.formulas {
        let formula_text = ensure_leading_eq(&fc.formula);
        let formula = if let Some(result) = fc.cached_result {
            Formula::new(&formula_text).set_result(result.to_string())
        } else {
            Formula::new(&formula_text)
        };
        ws.write_formula_with_format(fc.row, fc.col as u16, formula, &Format::new())
            .map_err(|e| CorpFinanceError::SerializationError(e.to_string()))?;
    }

    // Column widths
    for (idx, &width) in spec.column_widths.iter().enumerate() {
        ws.set_column_width(idx as u16, width)
            .map_err(|e| CorpFinanceError::SerializationError(e.to_string()))?;
    }

    // Frozen panes
    if let Some(fp) = &spec.frozen_panes {
        ws.set_freeze_panes(fp.row, fp.col as u16)
            .map_err(|e| CorpFinanceError::SerializationError(e.to_string()))?;
    }

    // Cell format overlays
    for fc in &spec.cell_formats {
        apply_cell_format(ws, fc, spec, data_row_offset)?;
    }

    // Charts
    for chart_spec in &spec.charts {
        let chart = build_chart(chart_spec)?;
        ws.insert_chart(chart_spec.anchor_row, chart_spec.anchor_col as u16, &chart)
            .map_err(|e| CorpFinanceError::SerializationError(e.to_string()))?;
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Cell writing
// ---------------------------------------------------------------------------

fn write_cell(
    ws: &mut rust_xlsxwriter::Worksheet,
    row: u32,
    col: u16,
    cell: &CellValue,
) -> CorpFinanceResult<()> {
    match cell {
        CellValue::Text { value } => {
            ws.write_string(row, col, value)
                .map_err(|e| CorpFinanceError::SerializationError(e.to_string()))?;
        }
        CellValue::Number { value } => {
            ws.write_number(row, col, *value)
                .map_err(|e| CorpFinanceError::SerializationError(e.to_string()))?;
        }
        CellValue::Decimal { value } => {
            // Parse the wire-string into Decimal for validation, then convert to f64
            // at the xlsx cell boundary. Excel is f64-native; this is the documented
            // exception to the no-f64 invariant — lossy conversion is appropriate here.
            let d: rust_decimal::Decimal = value
                .parse()
                .map_err(|e| CorpFinanceError::InvalidInput {
                    field: "cell_value.decimal".into(),
                    reason: format!("'{}' is not a valid decimal: {}", value, e),
                })?;
            let f: f64 = d.try_into().map_err(|_| CorpFinanceError::InvalidInput {
                field: "cell_value.decimal".into(),
                reason: format!("decimal '{}' overflows f64", value),
            })?;
            ws.write_number(row, col, f)
                .map_err(|e| CorpFinanceError::SerializationError(e.to_string()))?;
        }
        CellValue::Bool { value } => {
            ws.write_boolean(row, col, *value)
                .map_err(|e| CorpFinanceError::SerializationError(e.to_string()))?;
        }
        CellValue::DateTime { value, excel_format } => {
            let dt = chrono::DateTime::parse_from_rfc3339(value).map_err(|e| {
                CorpFinanceError::InvalidInput {
                    field: "cell_value.datetime".into(),
                    reason: format!("'{}' is not a valid RFC3339 datetime: {}", value, e),
                }
            })?;
            let naive = dt.naive_utc();
            // Convert chrono NaiveDateTime to ExcelDateTime via the ISO string path.
            let iso = naive.format("%Y-%m-%dT%H:%M:%S").to_string();
            let excel_dt = ExcelDateTime::parse_from_str(&iso)
                .map_err(|e| CorpFinanceError::SerializationError(e.to_string()))?;
            let num_fmt = excel_format
                .as_deref()
                .unwrap_or("yyyy-mm-dd hh:mm:ss");
            let fmt = Format::new().set_num_format(num_fmt);
            ws.write_datetime_with_format(row, col, excel_dt, &fmt)
                .map_err(|e| CorpFinanceError::SerializationError(e.to_string()))?;
        }
        CellValue::Empty => {
            // No-op: empty cell is left blank in the worksheet.
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Cell format overlay
// ---------------------------------------------------------------------------

/// Parse a cell value to an f64 (Number/Decimal), returning None for non-numeric.
fn cell_to_f64(cell: &CellValue) -> CorpFinanceResult<Option<f64>> {
    match cell {
        CellValue::Number { value } => Ok(Some(*value)),
        CellValue::Decimal { value } => {
            let d: rust_decimal::Decimal =
                value.parse().map_err(|e| CorpFinanceError::InvalidInput {
                    field: "cell_value.decimal".into(),
                    reason: format!("'{}' is not a valid decimal: {}", value, e),
                })?;
            let f: f64 = d.try_into().map_err(|_| CorpFinanceError::InvalidInput {
                field: "cell_value.decimal".into(),
                reason: format!("decimal '{}' overflows f64", value),
            })?;
            Ok(Some(f))
        }
        _ => Ok(None),
    }
}

/// Apply a [`FormattedCell`] overlay: rewrite the cell with the requested format.
fn apply_cell_format(
    ws: &mut rust_xlsxwriter::Worksheet,
    fc: &FormattedCell,
    spec: &SheetSpec,
    data_row_offset: u32,
) -> CorpFinanceResult<()> {
    let mut fmt = Format::new();
    if let Some(nf) = &fc.format.num_format {
        fmt = fmt.set_num_format(nf);
    }
    if fc.format.bold {
        fmt = fmt.set_bold();
    }
    if fc.format.italic {
        fmt = fmt.set_italic();
    }

    // Locate the source CellValue to rewrite it with the new format.
    let data_row = fc.row.saturating_sub(data_row_offset) as usize;
    let col_idx = fc.col as usize;
    let cell = spec
        .rows
        .get(data_row)
        .and_then(|r| r.get(col_idx));

    if let Some(cell) = cell {
        if let Some(f) = cell_to_f64(cell)? {
            ws.write_number_with_format(fc.row, fc.col as u16, f, &fmt)
                .map_err(|e| CorpFinanceError::SerializationError(e.to_string()))?;
        } else if let CellValue::Text { value } = cell {
            ws.write_string_with_format(fc.row, fc.col as u16, value, &fmt)
                .map_err(|e| CorpFinanceError::SerializationError(e.to_string()))?;
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Chart builder
// ---------------------------------------------------------------------------

fn build_chart(spec: &Chart) -> CorpFinanceResult<XlsxChart> {
    let chart_type = match spec.kind {
        ChartKind::Line => ChartType::Line,
        ChartKind::Bar => ChartType::Bar,
        ChartKind::Column => ChartType::Column,
        ChartKind::Pie => ChartType::Pie,
    };
    let mut chart = XlsxChart::new(chart_type);
    if let Some(title) = &spec.title {
        chart.title().set_name(title);
    }
    for series in &spec.series {
        chart
            .add_series()
            .set_name(series.name.as_str())
            .set_categories(series.categories_range.as_str())
            .set_values(series.values_range.as_str());
    }
    Ok(chart)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn ensure_leading_eq(formula: &str) -> String {
    if formula.starts_with('=') {
        formula.to_string()
    } else {
        format!("={formula}")
    }
}

fn sha256_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    let mut hex = String::with_capacity(64);
    for b in digest {
        let _ = write!(hex, "{b:02x}");
    }
    hex
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use std::io::Read;

    use tempfile::TempDir;

    use super::*;
    use crate::office::types::{
        CellFormat, CellValue, Chart, ChartKind, ChartSeries, DefinedName, FormulaCell,
        FormattedCell, FrozenPanes, SheetSpec, WorkbookProperties, WorkbookSpec,
    };

    fn minimal_spec(name: &str) -> WorkbookSpec {
        WorkbookSpec {
            sheets: vec![SheetSpec {
                name: name.to_string(),
                headers: vec!["Company".into(), "Value".into()],
                rows: vec![vec![
                    CellValue::Text { value: "ACME Corp".into() },
                    CellValue::Number { value: 1_000_000.0 },
                ]],
                ..SheetSpec::default()
            }],
            defined_names: vec![],
            properties: WorkbookProperties::default(),
        }
    }

    fn assert_file_nonempty(path: &Path) {
        let meta = std::fs::metadata(path).expect("file should exist");
        assert!(meta.len() > 0, "file should be non-empty");
    }

    fn assert_sha256_format(s: &str) {
        assert_eq!(s.len(), 64, "sha256 should be 64 chars");
        assert!(s.chars().all(|c| c.is_ascii_hexdigit()), "sha256 should be lowercase hex");
        // Ensure lowercase
        assert_eq!(&s.to_lowercase(), s, "sha256 should be lowercase");
    }

    fn zip_contains_bytes(path: &Path, needle: &[u8]) -> bool {
        let file = std::fs::File::open(path).unwrap();
        let mut archive = zip::ZipArchive::new(file).unwrap();
        for i in 0..archive.len() {
            let mut entry = archive.by_index(i).unwrap();
            let mut contents = Vec::new();
            entry.read_to_end(&mut contents).unwrap();
            if contents.windows(needle.len()).any(|w| w == needle) {
                return true;
            }
        }
        false
    }

    #[test]
    fn write_minimal_workbook_one_sheet_one_row() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("minimal.xlsx");
        let spec = minimal_spec("Summary");

        let result = write_workbook(&spec, &path).unwrap();

        assert_file_nonempty(&path);
        assert_eq!(result.bytes_written, std::fs::metadata(&path).unwrap().len());
        assert_sha256_format(&result.sha256);
        assert_eq!(result.sheet_count, 1);

        // Verify zip magic bytes PK\x03\x04
        let bytes = std::fs::read(&path).unwrap();
        assert_eq!(&bytes[..4], b"PK\x03\x04");

        // SHA-256 is stable across reruns (determinism check via second write)
        let dir2 = TempDir::new().unwrap();
        let path2 = dir2.path().join("minimal2.xlsx");
        let result2 = write_workbook(&spec, &path2).unwrap();
        assert_eq!(result.sha256, result2.sha256, "sha256 should be stable for identical spec");
    }

    #[test]
    fn write_workbook_with_decimal_cells() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("decimal.xlsx");
        let spec = WorkbookSpec {
            sheets: vec![SheetSpec {
                name: "Decimals".into(),
                rows: vec![vec![CellValue::Decimal { value: "1234.56789".into() }]],
                ..SheetSpec::default()
            }],
            defined_names: vec![],
            properties: WorkbookProperties::default(),
        };
        let result = write_workbook(&spec, &path).unwrap();
        assert_file_nonempty(&path);
        assert_sha256_format(&result.sha256);
    }

    #[test]
    fn write_workbook_with_decimal_overflowing_f64_errors() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("overflow.xlsx");
        let spec = WorkbookSpec {
            sheets: vec![SheetSpec {
                name: "Overflow".into(),
                rows: vec![vec![CellValue::Decimal {
                    // rust_decimal parses this but the f64 conversion overflows to inf
                    value: "99999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999".into(),
                }]],
                ..SheetSpec::default()
            }],
            defined_names: vec![],
            properties: WorkbookProperties::default(),
        };
        let err = write_workbook(&spec, &path).unwrap_err();
        assert!(
            matches!(err, CorpFinanceError::InvalidInput { .. }),
            "expected InvalidInput, got: {err}"
        );
    }

    #[test]
    fn write_workbook_with_invalid_decimal_string_errors() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("bad_decimal.xlsx");
        let spec = WorkbookSpec {
            sheets: vec![SheetSpec {
                name: "BadDecimal".into(),
                rows: vec![vec![CellValue::Decimal { value: "not-a-number".into() }]],
                ..SheetSpec::default()
            }],
            defined_names: vec![],
            properties: WorkbookProperties::default(),
        };
        let err = write_workbook(&spec, &path).unwrap_err();
        assert!(
            matches!(err, CorpFinanceError::InvalidInput { .. }),
            "expected InvalidInput, got: {err}"
        );
    }

    #[test]
    fn write_workbook_with_datetime_cell() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("datetime.xlsx");
        let spec = WorkbookSpec {
            sheets: vec![SheetSpec {
                name: "Dates".into(),
                headers: vec!["Timestamp".into()],
                rows: vec![vec![CellValue::DateTime {
                    value: "2026-05-07T12:00:00Z".into(),
                    excel_format: Some("yyyy-mm-dd".into()),
                }]],
                ..SheetSpec::default()
            }],
            defined_names: vec![],
            properties: WorkbookProperties::default(),
        };
        let result = write_workbook(&spec, &path).unwrap();
        assert_file_nonempty(&path);
        assert_sha256_format(&result.sha256);
    }

    #[test]
    fn write_workbook_with_invalid_datetime_string_errors() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("bad_dt.xlsx");
        let spec = WorkbookSpec {
            sheets: vec![SheetSpec {
                name: "BadDT".into(),
                rows: vec![vec![CellValue::DateTime {
                    value: "yesterday".into(),
                    excel_format: None,
                }]],
                ..SheetSpec::default()
            }],
            defined_names: vec![],
            properties: WorkbookProperties::default(),
        };
        let err = write_workbook(&spec, &path).unwrap_err();
        assert!(
            matches!(err, CorpFinanceError::InvalidInput { .. }),
            "expected InvalidInput, got: {err}"
        );
    }

    #[test]
    fn write_workbook_with_formula() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("formula.xlsx");
        let spec = WorkbookSpec {
            sheets: vec![SheetSpec {
                name: "Formulas".into(),
                headers: vec!["Label".into(), "Amount".into()],
                rows: vec![
                    vec![CellValue::Text { value: "A".into() }, CellValue::Number { value: 10.0 }],
                    vec![CellValue::Text { value: "B".into() }, CellValue::Number { value: 20.0 }],
                    vec![CellValue::Text { value: "C".into() }, CellValue::Number { value: 30.0 }],
                    vec![CellValue::Text { value: "D".into() }, CellValue::Number { value: 40.0 }],
                ],
                formulas: vec![FormulaCell {
                    row: 5,
                    col: 1,
                    formula: "SUM(B2:B5)".into(),
                    cached_result: Some(100.0),
                }],
                ..SheetSpec::default()
            }],
            defined_names: vec![],
            properties: WorkbookProperties::default(),
        };
        let result = write_workbook(&spec, &path).unwrap();
        assert_file_nonempty(&path);
        assert_sha256_format(&result.sha256);
        // Verify formula text present in the xlsx zip contents
        assert!(
            zip_contains_bytes(&path, b"SUM(B2:B5)"),
            "formula should appear in xlsx xml"
        );
    }

    #[test]
    fn write_workbook_with_defined_name() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("defined_names.xlsx");
        let spec = WorkbookSpec {
            sheets: vec![SheetSpec {
                name: "Summary".into(),
                headers: vec!["Label".into(), "WACC".into()],
                rows: vec![vec![
                    CellValue::Text { value: "WACC".into() },
                    CellValue::Number { value: 0.085 },
                ]],
                ..SheetSpec::default()
            }],
            defined_names: vec![DefinedName {
                name: "WACC".into(),
                range: "Summary!$B$2".into(),
            }],
            properties: WorkbookProperties::default(),
        };
        let result = write_workbook(&spec, &path).unwrap();
        assert_file_nonempty(&path);
        assert_sha256_format(&result.sha256);
        // Verify defined name present in the xlsx zip contents
        assert!(
            zip_contains_bytes(&path, b"WACC"),
            "defined name should appear in xlsx xml"
        );
    }

    #[test]
    fn write_workbook_rejects_empty_sheets() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("empty.xlsx");
        let spec = WorkbookSpec {
            sheets: vec![],
            defined_names: vec![],
            properties: WorkbookProperties::default(),
        };
        let err = write_workbook(&spec, &path).unwrap_err();
        assert!(
            matches!(err, CorpFinanceError::InvalidInput { .. }),
            "expected InvalidInput, got: {err}"
        );
    }

    #[test]
    fn write_workbook_rejects_duplicate_sheet_names() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("dup.xlsx");
        let spec = WorkbookSpec {
            sheets: vec![
                SheetSpec { name: "Summary".into(), ..SheetSpec::default() },
                SheetSpec { name: "Summary".into(), ..SheetSpec::default() },
            ],
            defined_names: vec![],
            properties: WorkbookProperties::default(),
        };
        let err = write_workbook(&spec, &path).unwrap_err();
        assert!(
            matches!(err, CorpFinanceError::InvalidInput { .. }),
            "expected InvalidInput, got: {err}"
        );
    }

    #[test]
    fn write_workbook_rejects_sheet_name_over_31_chars() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("longname.xlsx");
        let long_name = "A".repeat(32);
        let spec = WorkbookSpec {
            sheets: vec![SheetSpec {
                name: long_name,
                ..SheetSpec::default()
            }],
            defined_names: vec![],
            properties: WorkbookProperties::default(),
        };
        let err = write_workbook(&spec, &path).unwrap_err();
        assert!(
            matches!(err, CorpFinanceError::InvalidInput { .. }),
            "expected InvalidInput, got: {err}"
        );
    }

    #[test]
    fn write_workbook_from_json_round_trips() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("json_roundtrip.xlsx");
        let json = r#"{
            "sheets": [{
                "name": "DCF",
                "headers": ["Year", "FCF", "PV"],
                "rows": [
                    [{"kind":"number","value":1}, {"kind":"number","value":500000.0}, {"kind":"decimal","value":"454545.45"}],
                    [{"kind":"number","value":2}, {"kind":"number","value":550000.0}, {"kind":"decimal","value":"454545.45"}]
                ],
                "formulas": [],
                "column_widths": [8.0, 14.0, 14.0],
                "frozen_panes": {"row": 1, "col": 0}
            }],
            "defined_names": [],
            "properties": {"title": "DCF Model"}
        }"#;

        let result = write_workbook_from_json(json, &path).unwrap();
        assert_file_nonempty(&path);
        assert_sha256_format(&result.sha256);
        assert_eq!(result.sheet_count, 1);
    }

    #[test]
    fn write_workbook_freezes_panes() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("frozen.xlsx");
        let spec = WorkbookSpec {
            sheets: vec![SheetSpec {
                name: "Analysis".into(),
                headers: vec!["Company".into(), "EV/EBITDA".into()],
                rows: vec![vec![
                    CellValue::Text { value: "Target Corp".into() },
                    CellValue::Number { value: 12.5 },
                ]],
                column_widths: vec![20.0, 12.0],
                frozen_panes: Some(FrozenPanes { row: 1, col: 0 }),
                ..SheetSpec::default()
            }],
            defined_names: vec![],
            properties: WorkbookProperties::default(),
        };
        let result = write_workbook(&spec, &path).unwrap();
        assert_file_nonempty(&path);
        assert_sha256_format(&result.sha256);
        assert_eq!(result.sheet_count, 1);
    }

    // -----------------------------------------------------------------------
    // Wave 6 new tests: cell_formats + charts
    // -----------------------------------------------------------------------

    #[test]
    fn write_workbook_with_currency_format() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("currency.xlsx");
        let spec = WorkbookSpec {
            sheets: vec![SheetSpec {
                name: "Comps".into(),
                headers: vec!["Company".into(), "EV".into()],
                rows: vec![vec![
                    CellValue::Text { value: "ACME".into() },
                    CellValue::Number { value: 1234.56 },
                ]],
                // row 1 (data_row_offset=1 because headers present), col 1
                cell_formats: vec![FormattedCell {
                    row: 1,
                    col: 1,
                    format: CellFormat {
                        num_format: Some("$#,##0.00".into()),
                        bold: false,
                        italic: false,
                    },
                }],
                ..SheetSpec::default()
            }],
            defined_names: vec![],
            properties: WorkbookProperties::default(),
        };
        let result = write_workbook(&spec, &path).unwrap();
        assert_file_nonempty(&path);
        assert_sha256_format(&result.sha256);
        assert!(zip_contains_bytes(&path, b"$#,##0.00"), "currency format string should appear in xlsx xml");
    }

    #[test]
    fn write_workbook_with_percent_format() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("percent.xlsx");
        let spec = WorkbookSpec {
            sheets: vec![SheetSpec {
                name: "Rates".into(),
                headers: vec!["Label".into(), "Rate".into()],
                rows: vec![vec![
                    CellValue::Text { value: "WACC".into() },
                    CellValue::Number { value: 0.085 },
                ]],
                cell_formats: vec![FormattedCell {
                    row: 1,
                    col: 1,
                    format: CellFormat {
                        num_format: Some("0.00%".into()),
                        bold: false,
                        italic: false,
                    },
                }],
                ..SheetSpec::default()
            }],
            defined_names: vec![],
            properties: WorkbookProperties::default(),
        };
        let result = write_workbook(&spec, &path).unwrap();
        assert_file_nonempty(&path);
        assert_sha256_format(&result.sha256);
        assert!(zip_contains_bytes(&path, b"0.00%"), "percent format string should appear in xlsx xml");
    }

    fn chart_data_spec(sheet_name: &str, kind: ChartKind) -> WorkbookSpec {
        WorkbookSpec {
            sheets: vec![SheetSpec {
                name: sheet_name.into(),
                headers: vec!["Period".into(), "Revenue".into()],
                rows: vec![
                    vec![CellValue::Text { value: "Q1".into() }, CellValue::Number { value: 100.0 }],
                    vec![CellValue::Text { value: "Q2".into() }, CellValue::Number { value: 120.0 }],
                    vec![CellValue::Text { value: "Q3".into() }, CellValue::Number { value: 140.0 }],
                    vec![CellValue::Text { value: "Q4".into() }, CellValue::Number { value: 160.0 }],
                    vec![CellValue::Text { value: "Q5".into() }, CellValue::Number { value: 180.0 }],
                ],
                charts: vec![Chart {
                    kind,
                    anchor_row: 0,
                    anchor_col: 3,
                    title: Some("Revenue Trend".into()),
                    series: vec![ChartSeries {
                        name: "Revenue".into(),
                        categories_range: format!("{sheet_name}!$A$2:$A$6"),
                        values_range: format!("{sheet_name}!$B$2:$B$6"),
                    }],
                }],
                ..SheetSpec::default()
            }],
            defined_names: vec![],
            properties: WorkbookProperties::default(),
        }
    }

    #[test]
    fn write_workbook_with_line_chart() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("line_chart.xlsx");
        let spec = chart_data_spec("LineData", ChartKind::Line);
        let result = write_workbook(&spec, &path).unwrap();
        assert_file_nonempty(&path);
        assert_sha256_format(&result.sha256);
        // Chart XML embeds the series value range
        assert!(
            zip_contains_bytes(&path, b"LineData"),
            "chart series range sheet name should appear in xlsx xml"
        );
    }

    #[test]
    fn write_workbook_with_bar_chart() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("bar_chart.xlsx");
        let spec = chart_data_spec("BarData", ChartKind::Bar);
        let result = write_workbook(&spec, &path).unwrap();
        assert_file_nonempty(&path);
        assert_sha256_format(&result.sha256);
        assert!(
            zip_contains_bytes(&path, b"BarData"),
            "chart series range sheet name should appear in xlsx xml"
        );
    }
}
