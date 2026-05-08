//! Three-statement model tearsheet generator.
//!
//! Converts a [`ThreeStatementOutput`] into an institutional [`WorkbookSpec`]
//! with three worksheets: Income Statement, Balance Sheet, and Cash Flow.

use crate::office::types::{CellValue, FrozenPanes, SheetSpec, WorkbookProperties, WorkbookSpec};
use crate::three_statement::model::ThreeStatementOutput;

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Convert a completed three-statement model into an institutional tearsheet
/// [`WorkbookSpec`] ready for xlsx serialisation.
///
/// Produces three sheets: "Income Statement", "Balance Sheet", "Cash Flow".
/// All monetary values use [`CellValue::Decimal`]; year headers use
/// [`CellValue::Number`]; labels use [`CellValue::Text`].
/// Header row + label column (pane split at (1, 1)) are frozen.
pub fn three_statement_to_workbook(result: &ThreeStatementOutput) -> WorkbookSpec {
    let is_sheet = build_income_statement_sheet(&result.income_statements);
    let bs_sheet = build_balance_sheet_sheet(&result.balance_sheets);
    let cf_sheet = build_cash_flow_sheet(&result.cash_flow_statements);

    WorkbookSpec {
        sheets: vec![is_sheet, bs_sheet, cf_sheet],
        defined_names: vec![],
        properties: WorkbookProperties {
            title: Some("3-Statement Model".to_string()),
            author: None,
            company: None,
            subject: Some("Linked IS / BS / CF Projection".to_string()),
        },
    }
}

// ---------------------------------------------------------------------------
// Sheet builders
// ---------------------------------------------------------------------------

fn build_income_statement_sheet(
    stmts: &[crate::three_statement::model::IncomeStatement],
) -> SheetSpec {
    let headers = build_year_headers("Line Item", stmts.iter().map(|s| s.year));

    let rows: Vec<Vec<CellValue>> = vec![
        money_row("Revenue", stmts.iter().map(|s| s.revenue)),
        money_row("COGS", stmts.iter().map(|s| s.cogs)),
        money_row("Gross Profit", stmts.iter().map(|s| s.gross_profit)),
        money_row("SG&A", stmts.iter().map(|s| s.sga)),
        money_row("R&D", stmts.iter().map(|s| s.rnd)),
        money_row("Total Opex", stmts.iter().map(|s| s.total_opex)),
        money_row("EBITDA", stmts.iter().map(|s| s.ebitda)),
        money_row("D&A", stmts.iter().map(|s| s.depreciation)),
        money_row("EBIT", stmts.iter().map(|s| s.ebit)),
        money_row("Interest Expense", stmts.iter().map(|s| s.interest_expense)),
        money_row("EBT", stmts.iter().map(|s| s.ebt)),
        money_row("Taxes", stmts.iter().map(|s| s.taxes)),
        money_row("Net Income", stmts.iter().map(|s| s.net_income)),
    ];

    SheetSpec {
        name: "Income Statement".to_string(),
        headers,
        rows,
        formulas: vec![],
        column_widths: vec![],
        frozen_panes: Some(FrozenPanes { row: 1, col: 1 }),
        ..Default::default()
    }
}

fn build_balance_sheet_sheet(stmts: &[crate::three_statement::model::BalanceSheet]) -> SheetSpec {
    let headers = build_year_headers("Line Item", stmts.iter().map(|s| s.year));

    let rows: Vec<Vec<CellValue>> = vec![
        // Assets
        money_row("Cash", stmts.iter().map(|s| s.cash)),
        money_row(
            "Accounts Receivable",
            stmts.iter().map(|s| s.accounts_receivable),
        ),
        money_row("Inventory", stmts.iter().map(|s| s.inventory)),
        money_row(
            "Total Current Assets",
            stmts.iter().map(|s| s.total_current_assets),
        ),
        money_row("PP&E (Net)", stmts.iter().map(|s| s.ppe_net)),
        money_row("Total Assets", stmts.iter().map(|s| s.total_assets)),
        // Liabilities
        money_row("Accounts Payable", stmts.iter().map(|s| s.accounts_payable)),
        money_row("Current Debt", stmts.iter().map(|s| s.current_debt)),
        money_row(
            "Total Current Liabilities",
            stmts.iter().map(|s| s.total_current_liabilities),
        ),
        money_row("Long-Term Debt", stmts.iter().map(|s| s.long_term_debt)),
        money_row("Total Debt", stmts.iter().map(|s| s.total_debt)),
        money_row(
            "Total Liabilities",
            stmts.iter().map(|s| s.total_liabilities),
        ),
        // Equity
        money_row(
            "Shareholders' Equity",
            stmts.iter().map(|s| s.shareholders_equity),
        ),
        money_row(
            "Retained Earnings (Cumulative)",
            stmts.iter().map(|s| s.retained_earnings_cumulative),
        ),
        money_row(
            "Total Liabilities & Equity",
            stmts.iter().map(|s| s.total_liabilities_and_equity),
        ),
    ];

    SheetSpec {
        name: "Balance Sheet".to_string(),
        headers,
        rows,
        formulas: vec![],
        column_widths: vec![],
        frozen_panes: Some(FrozenPanes { row: 1, col: 1 }),
        ..Default::default()
    }
}

fn build_cash_flow_sheet(stmts: &[crate::three_statement::model::CashFlowStatement]) -> SheetSpec {
    let headers = build_year_headers("Line Item", stmts.iter().map(|s| s.year));

    let rows: Vec<Vec<CellValue>> = vec![
        // Operating
        money_row("Net Income", stmts.iter().map(|s| s.net_income)),
        money_row("D&A", stmts.iter().map(|s| s.depreciation)),
        money_row(
            "Change in Receivables",
            stmts.iter().map(|s| s.change_in_receivables),
        ),
        money_row(
            "Change in Inventory",
            stmts.iter().map(|s| s.change_in_inventory),
        ),
        money_row(
            "Change in Payables",
            stmts.iter().map(|s| s.change_in_payables),
        ),
        money_row(
            "Cash from Operations",
            stmts.iter().map(|s| s.cash_from_operations),
        ),
        // Investing
        money_row("Capex", stmts.iter().map(|s| s.capex)),
        money_row(
            "Cash from Investing",
            stmts.iter().map(|s| s.cash_from_investing),
        ),
        // Financing
        money_row("Debt Repayment", stmts.iter().map(|s| s.debt_repayment)),
        money_row("New Debt", stmts.iter().map(|s| s.new_debt)),
        money_row("Dividends", stmts.iter().map(|s| s.dividends)),
        money_row(
            "Cash from Financing",
            stmts.iter().map(|s| s.cash_from_financing),
        ),
        // Summary
        money_row(
            "Net Change in Cash",
            stmts.iter().map(|s| s.net_change_in_cash),
        ),
        money_row("Ending Cash", stmts.iter().map(|s| s.ending_cash)),
    ];

    SheetSpec {
        name: "Cash Flow".to_string(),
        headers,
        rows,
        formulas: vec![],
        column_widths: vec![],
        frozen_panes: Some(FrozenPanes { row: 1, col: 1 }),
        ..Default::default()
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Build the header row: first entry is the label column header, then one
/// [`String`] per projected year number.
fn build_year_headers(label: &str, years: impl Iterator<Item = i32>) -> Vec<String> {
    let mut headers = vec![label.to_string()];
    headers.extend(years.map(|y| format!("Year {y}")));
    headers
}

/// Build one data row: label cell followed by one [`CellValue::Decimal`] per
/// value in `values`. `Money` is `rust_decimal::Decimal`; `.to_string()`
/// produces the canonical wire representation.
fn money_row(label: &str, values: impl Iterator<Item = rust_decimal::Decimal>) -> Vec<CellValue> {
    let mut row = vec![CellValue::Text {
        value: label.to_string(),
    }];
    row.extend(values.map(|v| CellValue::Decimal {
        value: v.to_string(),
    }));
    row
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::three_statement::model::{
        BalanceSheet, CashFlowStatement, IncomeStatement, ProjectionSummary, ThreeStatementOutput,
    };
    use rust_decimal::Decimal;

    fn zero_income_statement(year: i32) -> IncomeStatement {
        IncomeStatement {
            year,
            revenue: Decimal::ZERO,
            cogs: Decimal::ZERO,
            gross_profit: Decimal::ZERO,
            gross_margin: Decimal::ZERO,
            sga: Decimal::ZERO,
            rnd: Decimal::ZERO,
            total_opex: Decimal::ZERO,
            ebitda: Decimal::ZERO,
            ebitda_margin: Decimal::ZERO,
            depreciation: Decimal::ZERO,
            ebit: Decimal::ZERO,
            ebit_margin: Decimal::ZERO,
            interest_expense: Decimal::ZERO,
            ebt: Decimal::ZERO,
            taxes: Decimal::ZERO,
            net_income: Decimal::ZERO,
            net_margin: Decimal::ZERO,
        }
    }

    fn zero_balance_sheet(year: i32) -> BalanceSheet {
        BalanceSheet {
            year,
            cash: Decimal::ZERO,
            accounts_receivable: Decimal::ZERO,
            inventory: Decimal::ZERO,
            total_current_assets: Decimal::ZERO,
            ppe_net: Decimal::ZERO,
            total_assets: Decimal::ZERO,
            accounts_payable: Decimal::ZERO,
            current_debt: Decimal::ZERO,
            total_current_liabilities: Decimal::ZERO,
            long_term_debt: Decimal::ZERO,
            total_debt: Decimal::ZERO,
            total_liabilities: Decimal::ZERO,
            shareholders_equity: Decimal::ZERO,
            retained_earnings_cumulative: Decimal::ZERO,
            total_liabilities_and_equity: Decimal::ZERO,
        }
    }

    fn zero_cash_flow_statement(year: i32) -> CashFlowStatement {
        CashFlowStatement {
            year,
            net_income: Decimal::ZERO,
            depreciation: Decimal::ZERO,
            change_in_receivables: Decimal::ZERO,
            change_in_inventory: Decimal::ZERO,
            change_in_payables: Decimal::ZERO,
            cash_from_operations: Decimal::ZERO,
            capex: Decimal::ZERO,
            cash_from_investing: Decimal::ZERO,
            debt_repayment: Decimal::ZERO,
            new_debt: Decimal::ZERO,
            dividends: Decimal::ZERO,
            cash_from_financing: Decimal::ZERO,
            net_change_in_cash: Decimal::ZERO,
            ending_cash: Decimal::ZERO,
            fcf: Decimal::ZERO,
            fcfe: Decimal::ZERO,
        }
    }

    fn minimal_output() -> ThreeStatementOutput {
        ThreeStatementOutput {
            income_statements: vec![zero_income_statement(1), zero_income_statement(2)],
            balance_sheets: vec![zero_balance_sheet(1), zero_balance_sheet(2)],
            cash_flow_statements: vec![zero_cash_flow_statement(1), zero_cash_flow_statement(2)],
            summary: ProjectionSummary {
                total_years: 2,
                revenue_cagr: Decimal::ZERO,
                avg_ebitda_margin: Decimal::ZERO,
                avg_net_margin: Decimal::ZERO,
                ending_debt: Decimal::ZERO,
                ending_leverage: Decimal::ZERO,
                cumulative_fcf: Decimal::ZERO,
            },
        }
    }

    #[test]
    fn three_statement_to_workbook_basic() {
        let output = minimal_output();
        let wb = three_statement_to_workbook(&output);

        assert_eq!(wb.sheets.len(), 3, "Expected 3 sheets");
        assert_eq!(wb.sheets[0].name, "Income Statement");
        assert_eq!(wb.sheets[1].name, "Balance Sheet");
        assert_eq!(wb.sheets[2].name, "Cash Flow");

        // Each sheet must have frozen panes at (1, 1)
        for sheet in &wb.sheets {
            let fp = sheet.frozen_panes.as_ref().expect("frozen panes missing");
            assert_eq!(fp.row, 1);
            assert_eq!(fp.col, 1);
        }

        // Workbook title
        assert_eq!(wb.properties.title.as_deref(), Some("3-Statement Model"));
    }

    #[test]
    fn three_statement_to_workbook_round_trips_through_writer() {
        use crate::office::xlsx::write_workbook;
        use std::fs;

        let output = minimal_output();
        let wb = three_statement_to_workbook(&output);

        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("three_statement_tearsheet.xlsx");

        let result = write_workbook(&wb, &path).expect("write_workbook failed");

        assert!(path.exists(), "Output file must exist");
        assert!(
            result.bytes_written > 0,
            "Written file must have nonzero bytes"
        );
        assert_eq!(result.sheet_count, 3);

        // Verify on disk
        let meta = fs::metadata(&path).expect("fs::metadata");
        assert!(meta.len() > 0, "File on disk must be nonzero");
    }
}
