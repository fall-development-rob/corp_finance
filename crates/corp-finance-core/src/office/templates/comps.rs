//! Trading Comps tearsheet generator.
//!
//! Converts a [`CompsOutput`] into a two-sheet [`WorkbookSpec`]:
//!   - **Peers**   — one row per comparable with raw multiples.
//!   - **Summary** — aggregate stats (mean / median / min / max) per multiple.

#[cfg(all(feature = "office", feature = "valuation"))]
use crate::office::types::{CellValue, FrozenPanes, SheetSpec, WorkbookProperties, WorkbookSpec};
#[cfg(all(feature = "office", feature = "valuation"))]
use crate::valuation::comps::{CompsOutput, MultipleStatistics, MultipleType};

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Build an institutional-quality Trading Comps [`WorkbookSpec`] from a
/// [`CompsOutput`].  Produces two sheets: **Peers** and **Summary**.
#[cfg(all(feature = "office", feature = "valuation"))]
pub fn comps_to_workbook(result: &CompsOutput) -> WorkbookSpec {
    WorkbookSpec {
        sheets: vec![build_peers_sheet(result), build_summary_sheet(result)],
        defined_names: vec![],
        properties: WorkbookProperties {
            title: Some("Trading Comps".to_string()),
            author: None,
            company: None,
            subject: None,
        },
    }
}

// ---------------------------------------------------------------------------
// Sheet builders
// ---------------------------------------------------------------------------

#[cfg(all(feature = "office", feature = "valuation"))]
fn build_peers_sheet(result: &CompsOutput) -> SheetSpec {
    // Determine which multiples we have stats for (preserves original order).
    let mult_headers = multiple_column_headers(&result.multiple_statistics);

    let mut headers = vec![
        "Company".to_string(),
        "Market Cap".to_string(),
        "EV".to_string(),
        "Revenue".to_string(),
        "EBITDA".to_string(),
    ];
    headers.extend(mult_headers.iter().cloned());

    // Build one row per peer from each MultipleStatistics.values list.
    // We need a unified peer -> column map.
    let peer_names = collect_peer_names(&result.multiple_statistics);

    let rows: Vec<Vec<CellValue>> = peer_names
        .iter()
        .map(|name| build_peer_row(name, result))
        .collect();

    SheetSpec {
        name: "Peers".to_string(),
        headers,
        rows,
        formulas: vec![],
        column_widths: vec![],
        frozen_panes: Some(FrozenPanes { row: 1, col: 1 }),
        cell_formats: vec![],
        charts: vec![],
    }
}

#[cfg(all(feature = "office", feature = "valuation"))]
fn build_summary_sheet(result: &CompsOutput) -> SheetSpec {
    let headers = vec![
        "Multiple".to_string(),
        "Mean".to_string(),
        "Median".to_string(),
        "Min".to_string(),
        "Max".to_string(),
    ];

    let rows: Vec<Vec<CellValue>> = result
        .multiple_statistics
        .iter()
        .map(build_summary_row)
        .collect();

    SheetSpec {
        name: "Summary".to_string(),
        headers,
        rows,
        formulas: vec![],
        column_widths: vec![],
        frozen_panes: Some(FrozenPanes { row: 1, col: 0 }),
        cell_formats: vec![],
        charts: vec![],
    }
}

// ---------------------------------------------------------------------------
// Row helpers
// ---------------------------------------------------------------------------

/// Return the display label for each multiple type present in the stats.
#[cfg(all(feature = "office", feature = "valuation"))]
fn multiple_column_headers(stats: &[MultipleStatistics]) -> Vec<String> {
    stats
        .iter()
        .map(|s| multiple_label(&s.multiple_type))
        .collect()
}

#[cfg(all(feature = "office", feature = "valuation"))]
fn multiple_label(mt: &MultipleType) -> String {
    match mt {
        MultipleType::EvEbitda => "EV/EBITDA",
        MultipleType::EvRevenue => "EV/Revenue",
        MultipleType::EvEbit => "EV/EBIT",
        MultipleType::PriceEarnings => "P/E",
        MultipleType::PriceBook => "P/B",
        MultipleType::Peg => "PEG",
    }
    .to_string()
}

/// Collect every peer name that appears in any multiple stats block.
#[cfg(all(feature = "office", feature = "valuation"))]
fn collect_peer_names(stats: &[MultipleStatistics]) -> Vec<String> {
    let mut seen: Vec<String> = Vec::new();
    for s in stats {
        for (name, _) in &s.values {
            if !seen.contains(name) {
                seen.push(name.clone());
            }
        }
    }
    seen
}

/// Build the full row for one peer: name | market_cap | ev | revenue | ebitda | multiples…
/// The money columns are intentionally left Empty because `CompsOutput` only
/// carries computed multiples (not the raw monetary metrics). Callers can
/// enrich by building from `CompsInput` if needed; this function works
/// solely from the canonical result type.
#[cfg(all(feature = "office", feature = "valuation"))]
fn build_peer_row(peer_name: &str, result: &CompsOutput) -> Vec<CellValue> {
    let mut row = vec![
        CellValue::Text {
            value: peer_name.to_string(),
        },
        CellValue::Empty, // Market Cap — not present in CompsOutput
        CellValue::Empty, // EV         — not present in CompsOutput
        CellValue::Empty, // Revenue    — not present in CompsOutput
        CellValue::Empty, // EBITDA     — not present in CompsOutput
    ];

    // One column per multiple; Empty when the peer has no value for it.
    for stats in &result.multiple_statistics {
        let cell = stats
            .values
            .iter()
            .find(|(name, _)| name == peer_name)
            .map(|(_, val)| CellValue::Decimal {
                value: val.to_string(),
            })
            .unwrap_or(CellValue::Empty);
        row.push(cell);
    }

    row
}

/// Build the summary row for one multiple: label | mean | median | min | max.
#[cfg(all(feature = "office", feature = "valuation"))]
fn build_summary_row(stats: &MultipleStatistics) -> Vec<CellValue> {
    vec![
        CellValue::Text {
            value: multiple_label(&stats.multiple_type),
        },
        CellValue::Decimal {
            value: stats.mean.to_string(),
        },
        CellValue::Decimal {
            value: stats.median.to_string(),
        },
        CellValue::Decimal {
            value: stats.low.to_string(),
        },
        CellValue::Decimal {
            value: stats.high.to_string(),
        },
    ]
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(all(test, feature = "office", feature = "valuation"))]
mod tests {
    use super::*;
    use crate::valuation::comps::{MultipleStatistics, MultipleType};
    use rust_decimal::Decimal;
    use rust_decimal_macros::dec;

    fn make_stats(mt: MultipleType, peers: &[(&str, Decimal)]) -> MultipleStatistics {
        let values: Vec<(String, Decimal)> =
            peers.iter().map(|(n, v)| (n.to_string(), *v)).collect();
        let n = values.len();
        let sum: Decimal = values.iter().map(|(_, v)| *v).sum();
        let mean = sum / Decimal::from(n as i64);
        let mut sorted: Vec<Decimal> = values.iter().map(|(_, v)| *v).collect();
        sorted.sort();
        let median = sorted[n / 2];
        let high = sorted[n - 1];
        let low = sorted[0];
        MultipleStatistics {
            multiple_type: mt,
            values,
            mean,
            median,
            high,
            low,
            std_dev: Decimal::ZERO,
            count: n,
        }
    }

    fn minimal_result() -> CompsOutput {
        CompsOutput {
            multiple_statistics: vec![
                make_stats(
                    MultipleType::EvEbitda,
                    &[("PeerA", dec!(10.0)), ("PeerB", dec!(12.0))],
                ),
                make_stats(
                    MultipleType::EvRevenue,
                    &[("PeerA", dec!(2.5)), ("PeerB", dec!(3.0))],
                ),
            ],
            implied_valuations: vec![],
            companies_included: 2,
            companies_excluded: 0,
        }
    }

    #[test]
    fn comps_to_workbook_basic() {
        let result = minimal_result();
        let wb = comps_to_workbook(&result);

        assert_eq!(wb.sheets.len(), 2, "expected exactly 2 sheets");
        assert_eq!(wb.sheets[1].name, "Summary");
        assert_eq!(wb.properties.title.as_deref(), Some("Trading Comps"));

        // Peers sheet has 2 data rows (one per peer).
        assert_eq!(wb.sheets[0].rows.len(), 2);

        // Summary sheet has 2 rows (one per multiple).
        assert_eq!(wb.sheets[1].rows.len(), 2);

        // First summary row label should be EV/EBITDA.
        match &wb.sheets[1].rows[0][0] {
            CellValue::Text { value } => assert_eq!(value, "EV/EBITDA"),
            other => panic!("expected Text, got {other:?}"),
        }
    }

    #[test]
    fn comps_to_workbook_round_trips_through_writer() {
        use crate::office::xlsx::write_workbook;
        use std::fs;

        let result = minimal_result();
        let wb = comps_to_workbook(&result);

        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("comps_test.xlsx");

        let write_result = write_workbook(&wb, &path).expect("write_workbook");

        assert!(path.exists(), "xlsx file should exist");
        assert!(
            write_result.bytes_written > 0,
            "file should have nonzero bytes"
        );

        let meta = fs::metadata(&path).expect("metadata");
        assert!(meta.len() > 0, "file size should be nonzero");
    }
}
