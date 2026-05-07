//! LBO tearsheet generator — converts an [`LboOutput`] into a [`WorkbookSpec`]
//! suitable for writing to `.xlsx` via [`crate::office::xlsx::write_workbook`].
//!
//! Three sheets are produced:
//! 1. **Sources & Uses** — two-block layout (sources left, uses right) with totals.
//! 2. **Debt Schedule** — year × tranche grid (opening bal, amort, sweep, closing bal).
//! 3. **Returns** — entry/exit equity, MOIC, IRR, and leverage summary.

#[cfg(all(feature = "office", feature = "pe"))]
use crate::{
    office::types::{CellValue, FrozenPanes, SheetSpec, WorkbookProperties, WorkbookSpec},
    pe::lbo::LboOutput,
};

#[cfg(all(feature = "office", feature = "pe"))]
fn dec_cell(v: rust_decimal::Decimal) -> CellValue {
    CellValue::Decimal {
        value: v.to_string(),
    }
}

#[cfg(all(feature = "office", feature = "pe"))]
fn text(s: &str) -> CellValue {
    CellValue::Text {
        value: s.to_owned(),
    }
}

#[cfg(all(feature = "office", feature = "pe"))]
fn num(n: u32) -> CellValue {
    CellValue::Number { value: n as f64 }
}

/// Build a three-sheet institutional LBO tearsheet from a completed [`LboOutput`].
///
/// Sheet 1 — Sources & Uses
/// Sheet 2 — Debt Schedule
/// Sheet 3 — Returns
#[cfg(all(feature = "office", feature = "pe"))]
pub fn lbo_to_workbook(result: &LboOutput) -> WorkbookSpec {
    WorkbookSpec {
        sheets: vec![
            build_sources_uses_sheet(result),
            build_debt_schedule_sheet(result),
            build_returns_sheet(result),
        ],
        defined_names: vec![],
        properties: WorkbookProperties {
            title: Some("LBO Model".to_owned()),
            author: None,
            company: None,
            subject: Some("Leveraged Buyout Analysis".to_owned()),
        },
    }
}

// ── Sheet 1: Sources & Uses ───────────────────────────────────────────────────

#[cfg(all(feature = "office", feature = "pe"))]
fn build_sources_uses_sheet(result: &LboOutput) -> SheetSpec {
    let su = &result.sources_uses;
    let max_rows = su.sources.len().max(su.uses.len());
    let mut rows: Vec<Vec<CellValue>> = Vec::with_capacity(max_rows + 1);

    for i in 0..max_rows {
        let src = su.sources.get(i);
        let use_ = su.uses.get(i);
        let src_label = src.map(|(n, _)| text(n)).unwrap_or(CellValue::Empty);
        let src_val = src.map(|(_, v)| dec_cell(*v)).unwrap_or(CellValue::Empty);
        let use_label = use_.map(|(n, _)| text(n)).unwrap_or(CellValue::Empty);
        let use_val = use_.map(|(_, v)| dec_cell(*v)).unwrap_or(CellValue::Empty);
        rows.push(vec![src_label, src_val, CellValue::Empty, use_label, use_val]);
    }

    // Total row
    rows.push(vec![
        text("Total Sources"),
        dec_cell(su.total_sources),
        CellValue::Empty,
        text("Total Uses"),
        dec_cell(su.total_uses),
    ]);

    SheetSpec {
        name: "Sources & Uses".to_owned(),
        headers: vec![
            "Sources".to_owned(),
            "Amount".to_owned(),
            "".to_owned(),
            "Uses".to_owned(),
            "Amount".to_owned(),
        ],
        rows,
        column_widths: vec![28.0, 16.0, 4.0, 28.0, 16.0],
        frozen_panes: Some(FrozenPanes { row: 1, col: 0 }),
        ..Default::default()
    }
}

// ── Sheet 2: Debt Schedule ────────────────────────────────────────────────────

#[cfg(all(feature = "office", feature = "pe"))]
fn build_debt_schedule_sheet(result: &LboOutput) -> SheetSpec {
    let mut rows: Vec<Vec<CellValue>> = Vec::new();

    if result.debt_schedules.is_empty() {
        // Fallback: aggregate from projections
        for proj in &result.projections {
            rows.push(vec![
                num(proj.year),
                text("Total Debt"),
                CellValue::Empty,
                dec_cell(proj.mandatory_repayment),
                dec_cell(proj.optional_repayment),
                dec_cell(proj.total_debt_outstanding),
            ]);
        }
    } else {
        for sched in &result.debt_schedules {
            for period in &sched.periods {
                let sweep = period.scheduled_repayment - period.scheduled_repayment; // placeholder zero
                let _ = sweep;
                // Derive cash sweep from projection matching the year/tranche
                let proj_opt = result
                    .projections
                    .iter()
                    .find(|p| p.year == period.year);
                let optional = proj_opt
                    .map(|p| p.optional_repayment)
                    .unwrap_or(rust_decimal::Decimal::ZERO);

                rows.push(vec![
                    num(period.year),
                    text(&sched.tranche_name),
                    dec_cell(period.opening_balance),
                    dec_cell(period.scheduled_repayment),
                    dec_cell(optional),
                    dec_cell(period.closing_balance),
                ]);
            }
        }
    }

    SheetSpec {
        name: "Debt Schedule".to_owned(),
        headers: vec![
            "Year".to_owned(),
            "Tranche".to_owned(),
            "Beginning Balance".to_owned(),
            "Mandatory Amort".to_owned(),
            "Cash Sweep".to_owned(),
            "Ending Balance".to_owned(),
        ],
        rows,
        column_widths: vec![8.0, 24.0, 20.0, 20.0, 16.0, 20.0],
        frozen_panes: Some(FrozenPanes { row: 1, col: 2 }),
        ..Default::default()
    }
}

// ── Sheet 3: Returns ──────────────────────────────────────────────────────────

#[cfg(all(feature = "office", feature = "pe"))]
fn build_returns_sheet(result: &LboOutput) -> SheetSpec {
    // Entry equity is derived: entry EV minus total initial debt.
    // We expose what the result directly gives us.
    let rows = vec![
        vec![text("Exit Enterprise Value"), dec_cell(result.exit_ev)],
        vec![text("Exit Net Debt"), dec_cell(result.exit_net_debt)],
        vec![text("Exit Equity Value"), dec_cell(result.exit_equity_value)],
        vec![text("MoM (MOIC)"), dec_cell(result.moic)],
        vec![text("Cash-on-Cash"), dec_cell(result.cash_on_cash)],
        vec![text("Sponsor IRR"), dec_cell(result.irr)],
        vec![text("Entry Leverage (x)"), dec_cell(result.entry_leverage)],
        vec![text("Exit Leverage (x)"), dec_cell(result.exit_leverage)],
    ];

    SheetSpec {
        name: "Returns".to_owned(),
        headers: vec!["Metric".to_owned(), "Value".to_owned()],
        rows,
        column_widths: vec![28.0, 20.0],
        frozen_panes: Some(FrozenPanes { row: 1, col: 0 }),
        ..Default::default()
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    #[cfg(all(feature = "office", feature = "pe"))]
    use super::*;

    #[cfg(all(feature = "office", feature = "pe"))]
    fn minimal_lbo_output() -> crate::pe::lbo::LboOutput {
        use crate::pe::{
            debt_schedule::{AmortisationType, DebtTrancheInput},
            lbo::{LboInput, build_lbo},
        };
        use rust_decimal_macros::dec;

        let input = LboInput {
            entry_ev: dec!(1000),
            entry_ebitda: dec!(200),
            revenue_growth: vec![dec!(0.05); 5],
            ebitda_margin: vec![dec!(0.20); 5],
            capex_as_pct_revenue: dec!(0.03),
            nwc_as_pct_revenue: dec!(0.05),
            tax_rate: dec!(0.25),
            da_as_pct_revenue: dec!(0.02),
            base_revenue: dec!(1000),
            tranches: vec![DebtTrancheInput {
                name: "Senior Term Loan".into(),
                amount: dec!(600),
                interest_rate: dec!(0.05),
                is_floating: false,
                base_rate: None,
                spread: None,
                amortisation: AmortisationType::StraightLine(dec!(0.05)),
                maturity_years: 7,
                pik_rate: None,
                seniority: 1,
                commitment_fee: None,
                is_revolver: false,
            }],
            equity_contribution: dec!(400),
            cash_sweep_pct: None,
            exit_year: 5,
            exit_multiple: dec!(6.0),
            transaction_fees: None,
            financing_fees: None,
            management_rollover: None,
            currency: None,
            minimum_cash: None,
        };
        build_lbo(&input).unwrap().result
    }

    #[cfg(all(feature = "office", feature = "pe"))]
    #[test]
    fn lbo_to_workbook_basic() {
        let result = minimal_lbo_output();
        let wb = lbo_to_workbook(&result);

        assert_eq!(wb.sheets.len(), 3, "expected 3 sheets");
        assert_eq!(wb.sheets[0].name, "Sources & Uses");
        assert_eq!(wb.sheets[1].name, "Debt Schedule");
        assert_eq!(wb.sheets[2].name, "Returns");
        assert_eq!(wb.properties.title.as_deref(), Some("LBO Model"));
    }

    #[cfg(all(feature = "office", feature = "pe"))]
    #[test]
    fn lbo_to_workbook_round_trips_through_writer() {
        use std::fs;
        use tempfile::NamedTempFile;

        let result = minimal_lbo_output();
        let wb = lbo_to_workbook(&result);

        let tmp = NamedTempFile::new().expect("tempfile");
        let path = tmp.path().to_path_buf();
        // keep file alive via into_temp_path so we can stat it
        let _tp = tmp.into_temp_path();

        crate::office::xlsx::write_workbook(&wb, &path).expect("write_workbook failed");

        let meta = fs::metadata(&path).expect("stat");
        assert!(meta.len() > 0, "xlsx output must have nonzero bytes");
    }
}
