//! DCF tearsheet generator — turns a [`DcfOutput`] into an institutional
//! [`WorkbookSpec`] in one call.
//!
//! Gated on both `office` and `valuation` features so the dependency graph
//! is kept clean. The caller is responsible for resolving [`DcfInput`] into
//! a [`ComputationOutput<DcfOutput>`] via [`crate::valuation::dcf::calculate_dcf`]
//! and then passing the inner `result` field here.

#[cfg(all(feature = "office", feature = "valuation"))]
use crate::office::types::{
    CellValue, DefinedName, FrozenPanes, SheetSpec, WorkbookProperties, WorkbookSpec,
};

#[cfg(all(feature = "office", feature = "valuation"))]
use crate::valuation::dcf::DcfOutput;

/// Convert a computed [`DcfOutput`] into a three-sheet institutional tearsheet.
///
/// Sheet layout:
/// - **Assumptions** — WACC, terminal growth rate, projection horizon, mid-year flag.
/// - **Forecast** — Year / Revenue / FCFF / Discount Factor / PV per projection year.
/// - **Valuation** — Enterprise value bridge down to equity per share.
///
/// All monetary and ratio values use [`CellValue::Decimal`] to preserve the
/// 128-bit precision through the string wire form. Year integers use
/// [`CellValue::Number`].
#[cfg(all(feature = "office", feature = "valuation"))]
pub fn dcf_to_workbook(result: &DcfOutput) -> WorkbookSpec {
    WorkbookSpec {
        sheets: vec![
            build_assumptions_sheet(result),
            build_forecast_sheet(result),
            build_valuation_sheet(result),
        ],
        defined_names: build_defined_names(),
        properties: WorkbookProperties {
            title: Some("DCF Model".into()),
            author: Some("corp-finance-core".into()),
            company: None,
            subject: Some("Discounted Cash Flow Valuation".into()),
        },
    }
}

// ---------------------------------------------------------------------------
// Sheet builders
// ---------------------------------------------------------------------------

#[cfg(all(feature = "office", feature = "valuation"))]
fn build_assumptions_sheet(result: &DcfOutput) -> SheetSpec {
    let horizon = result.projections.len();

    let terminal_method = if result.terminal_value_gordon.is_some() {
        "Gordon Growth"
    } else {
        "Exit Multiple"
    };

    let rows = vec![
        assumption_row("WACC", result.wacc_used.to_string()),
        assumption_text_row("Terminal Method", terminal_method.into()),
        assumption_row("Projection Horizon (years)", horizon.to_string()),
        assumption_row(
            "Terminal Value % of EV",
            result.terminal_value_pct.to_string(),
        ),
        assumption_row(
            "Implied EV/EBITDA Exit Multiple",
            result.implied_exit_multiple.to_string(),
        ),
    ];

    SheetSpec {
        name: "Assumptions".into(),
        headers: vec!["Parameter".into(), "Value".into()],
        rows,
        formulas: vec![],
        column_widths: vec![30.0, 20.0],
        frozen_panes: Some(FrozenPanes { row: 1, col: 0 }),
        cell_formats: vec![],
        charts: vec![],
    }
}

#[cfg(all(feature = "office", feature = "valuation"))]
fn assumption_row(label: &str, value: String) -> Vec<CellValue> {
    vec![
        CellValue::Text {
            value: label.into(),
        },
        CellValue::Decimal { value },
    ]
}

#[cfg(all(feature = "office", feature = "valuation"))]
fn assumption_text_row(label: &str, value: String) -> Vec<CellValue> {
    vec![
        CellValue::Text {
            value: label.into(),
        },
        CellValue::Text { value },
    ]
}

#[cfg(all(feature = "office", feature = "valuation"))]
fn build_forecast_sheet(result: &DcfOutput) -> SheetSpec {
    let rows = result
        .projections
        .iter()
        .map(|p| {
            vec![
                CellValue::Number {
                    value: p.period.year as f64,
                },
                CellValue::Decimal {
                    value: p.revenue.to_string(),
                },
                CellValue::Decimal {
                    value: p.fcff.to_string(),
                },
                CellValue::Decimal {
                    value: p.discount_factor.to_string(),
                },
                CellValue::Decimal {
                    value: p.pv_fcff.to_string(),
                },
            ]
        })
        .collect();

    SheetSpec {
        name: "Forecast".into(),
        headers: vec![
            "Year".into(),
            "Revenue".into(),
            "FCFF".into(),
            "Discount Factor".into(),
            "PV of FCFF".into(),
        ],
        rows,
        formulas: vec![],
        column_widths: vec![8.0, 18.0, 18.0, 18.0, 18.0],
        frozen_panes: Some(FrozenPanes { row: 1, col: 1 }),
        cell_formats: vec![],
        charts: vec![],
    }
}

#[cfg(all(feature = "office", feature = "valuation"))]
fn build_valuation_sheet(result: &DcfOutput) -> SheetSpec {
    let mut rows: Vec<Vec<CellValue>> = vec![
        label_decimal_row("PV of Explicit FCFFs", result.pv_of_fcff.to_string()),
        label_decimal_row("PV of Terminal Value", result.pv_of_terminal.to_string()),
        label_decimal_row("Enterprise Value", result.enterprise_value.to_string()),
    ];

    if let Some(eq) = result.equity_value {
        rows.push(label_decimal_row("Equity Value", eq.to_string()));
    }

    if let Some(per_share) = result.equity_value_per_share {
        rows.push(label_decimal_row(
            "Equity Value Per Share",
            per_share.to_string(),
        ));
    }

    SheetSpec {
        name: "Valuation".into(),
        headers: vec!["Line Item".into(), "Amount".into()],
        rows,
        formulas: vec![],
        column_widths: vec![28.0, 20.0],
        frozen_panes: Some(FrozenPanes { row: 1, col: 0 }),
        cell_formats: vec![],
        charts: vec![],
    }
}

#[cfg(all(feature = "office", feature = "valuation"))]
fn label_decimal_row(label: &str, value: String) -> Vec<CellValue> {
    vec![
        CellValue::Text {
            value: label.into(),
        },
        CellValue::Decimal { value },
    ]
}

#[cfg(all(feature = "office", feature = "valuation"))]
fn build_defined_names() -> Vec<DefinedName> {
    vec![
        DefinedName {
            name: "DCF_ENTERPRISE_VALUE".into(),
            range: "Valuation!$B$3".into(),
        },
        DefinedName {
            name: "DCF_PV_FCFF".into(),
            range: "Valuation!$B$1".into(),
        },
        DefinedName {
            name: "DCF_PV_TERMINAL".into(),
            range: "Valuation!$B$2".into(),
        },
    ]
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(all(test, feature = "office", feature = "valuation"))]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn minimal_dcf_output() -> DcfOutput {
        use crate::types::ProjectionPeriod;
        use crate::valuation::dcf::DcfYearProjection;

        DcfOutput {
            projections: vec![DcfYearProjection {
                period: ProjectionPeriod {
                    year: 1,
                    label: "Year 1".into(),
                    is_terminal: false,
                },
                revenue: dec!(1000),
                ebitda: dec!(250),
                ebit: dec!(200),
                nopat: dec!(150),
                plus_da: dec!(50),
                less_capex: dec!(40),
                less_nwc_change: dec!(10),
                fcff: dec!(150),
                discount_factor: dec!(0.95),
                pv_fcff: dec!(142.5),
            }],
            terminal_value_gordon: Some(dec!(2000)),
            terminal_value_exit: None,
            terminal_value_used: dec!(2000),
            pv_of_fcff: dec!(142.5),
            pv_of_terminal: dec!(1800),
            enterprise_value: dec!(1942.5),
            equity_value: Some(dec!(1742.5)),
            equity_value_per_share: Some(dec!(17.425)),
            implied_exit_multiple: dec!(8),
            terminal_value_pct: dec!(0.927),
            wacc_used: dec!(0.10),
        }
    }

    #[test]
    fn dcf_to_workbook_basic() {
        let output = minimal_dcf_output();
        let wb = dcf_to_workbook(&output);

        assert_eq!(wb.sheets.len(), 3, "expected 3 sheets");
        assert_eq!(wb.sheets[0].name, "Assumptions");
        assert_eq!(wb.sheets[1].name, "Forecast");
        assert_eq!(wb.sheets[2].name, "Valuation");

        assert_eq!(
            wb.properties.title.as_deref(),
            Some("DCF Model"),
            "workbook title mismatch"
        );
        assert_eq!(
            wb.properties.author.as_deref(),
            Some("corp-finance-core"),
            "workbook author mismatch"
        );

        // Forecast sheet has one row per projection year
        assert_eq!(
            wb.sheets[1].rows.len(),
            output.projections.len(),
            "forecast row count mismatch"
        );
    }

    #[test]
    fn dcf_to_workbook_round_trips_through_writer() {
        use crate::office::xlsx::write_workbook;
        use std::path::PathBuf;
        use tempfile::tempdir;

        let output = minimal_dcf_output();
        let wb = dcf_to_workbook(&output);

        let dir = tempdir().expect("tempdir creation failed");
        let path: PathBuf = dir.path().join("dcf_tearsheet.xlsx");

        let result = write_workbook(&wb, &path).expect("write_workbook failed");

        assert!(path.exists(), "output file does not exist");
        assert!(
            result.bytes_written > 0,
            "expected nonzero bytes written, got {}",
            result.bytes_written
        );
        assert_eq!(
            result.sheet_count, 3,
            "sheet count in WriteWorkbookResult mismatch"
        );
    }
}
