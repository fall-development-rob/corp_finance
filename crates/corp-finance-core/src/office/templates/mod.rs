//! Institutional tearsheet generators — one module per model type.
//!
//! Each sub-module converts a computed result struct into a [`crate::office::types::WorkbookSpec`]
//! ready for serialisation by [`crate::office::xlsx::write_workbook`].

#[cfg(all(feature = "office", feature = "valuation"))]
pub mod comps;

#[cfg(all(feature = "office", feature = "valuation"))]
pub mod dcf;

#[cfg(all(feature = "office", feature = "pe"))]
pub mod lbo;

#[cfg(all(feature = "office", feature = "three_statement"))]
pub mod three_statement;

#[cfg(feature = "office")]
pub mod ic_memo;

#[cfg(feature = "office")]
pub mod ic_presentation;

#[cfg(feature = "office")]
pub mod pitch_deck;

#[cfg(feature = "office")]
pub mod research_init;

// ---------------------------------------------------------------------------
// JSON-envelope dispatcher (Phase 29 Wave 6, Task B1)
// ---------------------------------------------------------------------------

#[cfg(feature = "office")]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "schema_gen", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum TemplateKind {
    Dcf,
    Comps,
    Lbo,
    ThreeStatement,
}

#[cfg(feature = "office")]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "schema_gen", derive(schemars::JsonSchema))]
pub struct RenderTemplateInput {
    pub kind: TemplateKind,
    pub result_json: String,
}

/// Parse a [`RenderTemplateInput`] from `input_json`, deserialise `result_json`
/// into the appropriate compute-result type, and dispatch to the matching
/// per-template function.  Returns a [`crate::office::WorkbookSpec`].
///
/// Feature gates per branch:
/// - `dcf` / `comps` require the `valuation` feature.
/// - `lbo` requires the `pe` feature.
/// - `three_statement` requires the `three_statement` feature.
#[cfg(feature = "office")]
pub fn render_template_from_json(
    input_json: &str,
) -> crate::CorpFinanceResult<crate::office::WorkbookSpec> {
    let input: RenderTemplateInput = serde_json::from_str(input_json).map_err(|e| {
        crate::CorpFinanceError::InvalidInput {
            field: "input_json".into(),
            reason: e.to_string(),
        }
    })?;

    match input.kind {
        #[cfg(feature = "valuation")]
        TemplateKind::Dcf => {
            let result: crate::valuation::dcf::DcfOutput =
                serde_json::from_str(&input.result_json).map_err(|e| {
                    crate::CorpFinanceError::InvalidInput {
                        field: "result_json".into(),
                        reason: e.to_string(),
                    }
                })?;
            Ok(dcf::dcf_to_workbook(&result))
        }
        #[cfg(not(feature = "valuation"))]
        TemplateKind::Dcf => Err(crate::CorpFinanceError::InvalidInput {
            field: "kind".into(),
            reason: "feature valuation not enabled at compile time".into(),
        }),

        #[cfg(feature = "valuation")]
        TemplateKind::Comps => {
            let result: crate::valuation::comps::CompsOutput =
                serde_json::from_str(&input.result_json).map_err(|e| {
                    crate::CorpFinanceError::InvalidInput {
                        field: "result_json".into(),
                        reason: e.to_string(),
                    }
                })?;
            Ok(comps::comps_to_workbook(&result))
        }
        #[cfg(not(feature = "valuation"))]
        TemplateKind::Comps => Err(crate::CorpFinanceError::InvalidInput {
            field: "kind".into(),
            reason: "feature valuation not enabled at compile time".into(),
        }),

        #[cfg(feature = "pe")]
        TemplateKind::Lbo => {
            let result: crate::pe::lbo::LboOutput =
                serde_json::from_str(&input.result_json).map_err(|e| {
                    crate::CorpFinanceError::InvalidInput {
                        field: "result_json".into(),
                        reason: e.to_string(),
                    }
                })?;
            Ok(lbo::lbo_to_workbook(&result))
        }
        #[cfg(not(feature = "pe"))]
        TemplateKind::Lbo => Err(crate::CorpFinanceError::InvalidInput {
            field: "kind".into(),
            reason: "feature pe not enabled at compile time".into(),
        }),

        #[cfg(feature = "three_statement")]
        TemplateKind::ThreeStatement => {
            let result: crate::three_statement::model::ThreeStatementOutput =
                serde_json::from_str(&input.result_json).map_err(|e| {
                    crate::CorpFinanceError::InvalidInput {
                        field: "result_json".into(),
                        reason: e.to_string(),
                    }
                })?;
            Ok(three_statement::three_statement_to_workbook(&result))
        }
        #[cfg(not(feature = "three_statement"))]
        TemplateKind::ThreeStatement => Err(crate::CorpFinanceError::InvalidInput {
            field: "kind".into(),
            reason: "feature three_statement not enabled at compile time".into(),
        }),
    }
}

// ---------------------------------------------------------------------------
// Unit tests (all four kinds)
// ---------------------------------------------------------------------------

#[cfg(all(
    test,
    feature = "office",
    feature = "valuation",
    feature = "pe",
    feature = "three_statement"
))]
mod tests {
    use super::render_template_from_json;
    use rust_decimal::Decimal;
    use rust_decimal_macros::dec;

    // ---- DCF ----------------------------------------------------------------

    fn dcf_input_json() -> String {
        use crate::valuation::dcf::{DcfOutput, DcfYearProjection};
        use crate::types::ProjectionPeriod;
        let output = DcfOutput {
            projections: vec![DcfYearProjection {
                period: ProjectionPeriod { year: 1, label: "Year 1".into(), is_terminal: false },
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
        };
        let result_json = serde_json::to_string(&output).unwrap();
        serde_json::json!({ "kind": "dcf", "result_json": result_json }).to_string()
    }

    #[test]
    fn render_dcf_returns_three_sheets() {
        let wb = render_template_from_json(&dcf_input_json()).unwrap();
        assert_eq!(wb.sheets.len(), 3);
    }

    // ---- Comps --------------------------------------------------------------

    fn comps_input_json() -> String {
        use crate::valuation::comps::{CompsOutput, MultipleStatistics, MultipleType};
        let stats = MultipleStatistics {
            multiple_type: MultipleType::EvEbitda,
            values: vec![("PeerA".into(), dec!(10.0)), ("PeerB".into(), dec!(12.0))],
            mean: dec!(11.0),
            median: dec!(12.0),
            high: dec!(12.0),
            low: dec!(10.0),
            std_dev: Decimal::ZERO,
            count: 2,
        };
        let output = CompsOutput {
            multiple_statistics: vec![stats],
            implied_valuations: vec![],
            companies_included: 2,
            companies_excluded: 0,
        };
        let result_json = serde_json::to_string(&output).unwrap();
        serde_json::json!({ "kind": "comps", "result_json": result_json }).to_string()
    }

    #[test]
    fn render_comps_returns_two_sheets() {
        let wb = render_template_from_json(&comps_input_json()).unwrap();
        assert_eq!(wb.sheets.len(), 2);
    }

    // ---- LBO ----------------------------------------------------------------

    fn lbo_input_json() -> String {
        use crate::pe::{
            debt_schedule::{AmortisationType, DebtTrancheInput},
            lbo::{LboInput, build_lbo},
        };
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
        let output = build_lbo(&input).unwrap().result;
        let result_json = serde_json::to_string(&output).unwrap();
        serde_json::json!({ "kind": "lbo", "result_json": result_json }).to_string()
    }

    #[test]
    fn render_lbo_returns_three_sheets() {
        let wb = render_template_from_json(&lbo_input_json()).unwrap();
        assert_eq!(wb.sheets.len(), 3);
    }

    // ---- Three-statement ----------------------------------------------------

    fn three_statement_input_json() -> String {
        use crate::three_statement::model::{
            BalanceSheet, CashFlowStatement, IncomeStatement, ProjectionSummary,
            ThreeStatementOutput,
        };
        let is = IncomeStatement {
            year: 1,
            revenue: Decimal::ZERO, cogs: Decimal::ZERO, gross_profit: Decimal::ZERO,
            gross_margin: Decimal::ZERO, sga: Decimal::ZERO, rnd: Decimal::ZERO,
            total_opex: Decimal::ZERO, ebitda: Decimal::ZERO, ebitda_margin: Decimal::ZERO,
            depreciation: Decimal::ZERO, ebit: Decimal::ZERO, ebit_margin: Decimal::ZERO,
            interest_expense: Decimal::ZERO, ebt: Decimal::ZERO, taxes: Decimal::ZERO,
            net_income: Decimal::ZERO, net_margin: Decimal::ZERO,
        };
        let bs = BalanceSheet {
            year: 1,
            cash: Decimal::ZERO, accounts_receivable: Decimal::ZERO,
            inventory: Decimal::ZERO, total_current_assets: Decimal::ZERO,
            ppe_net: Decimal::ZERO, total_assets: Decimal::ZERO,
            accounts_payable: Decimal::ZERO, current_debt: Decimal::ZERO,
            total_current_liabilities: Decimal::ZERO, long_term_debt: Decimal::ZERO,
            total_debt: Decimal::ZERO, total_liabilities: Decimal::ZERO,
            shareholders_equity: Decimal::ZERO, retained_earnings_cumulative: Decimal::ZERO,
            total_liabilities_and_equity: Decimal::ZERO,
        };
        let cf = CashFlowStatement {
            year: 1,
            net_income: Decimal::ZERO, depreciation: Decimal::ZERO,
            change_in_receivables: Decimal::ZERO, change_in_inventory: Decimal::ZERO,
            change_in_payables: Decimal::ZERO, cash_from_operations: Decimal::ZERO,
            capex: Decimal::ZERO, cash_from_investing: Decimal::ZERO,
            debt_repayment: Decimal::ZERO, new_debt: Decimal::ZERO,
            dividends: Decimal::ZERO, cash_from_financing: Decimal::ZERO,
            net_change_in_cash: Decimal::ZERO, ending_cash: Decimal::ZERO,
            fcf: Decimal::ZERO, fcfe: Decimal::ZERO,
        };
        let output = ThreeStatementOutput {
            income_statements: vec![is],
            balance_sheets: vec![bs],
            cash_flow_statements: vec![cf],
            summary: ProjectionSummary {
                total_years: 1,
                revenue_cagr: Decimal::ZERO,
                avg_ebitda_margin: Decimal::ZERO,
                avg_net_margin: Decimal::ZERO,
                ending_debt: Decimal::ZERO,
                ending_leverage: Decimal::ZERO,
                cumulative_fcf: Decimal::ZERO,
            },
        };
        let result_json = serde_json::to_string(&output).unwrap();
        serde_json::json!({ "kind": "three_statement", "result_json": result_json }).to_string()
    }

    #[test]
    fn render_three_statement_returns_three_sheets() {
        let wb = render_template_from_json(&three_statement_input_json()).unwrap();
        assert_eq!(wb.sheets.len(), 3);
    }
}

// ---------------------------------------------------------------------------
// Doc-template dispatcher (Phase 29 Wave 9, docx deliverables)
// ---------------------------------------------------------------------------

#[cfg(feature = "office")]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "schema_gen", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum DocTemplateKind {
    IcMemo,
    ResearchInit,
}

#[cfg(feature = "office")]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "schema_gen", derive(schemars::JsonSchema))]
pub struct RenderDocTemplateInput {
    pub kind: DocTemplateKind,
    pub input_json: String,
}

#[cfg(feature = "office")]
pub fn render_doc_template_from_json(
    input_json: &str,
) -> crate::CorpFinanceResult<crate::office::WordDocSpec> {
    let envelope: RenderDocTemplateInput = serde_json::from_str(input_json).map_err(|e| {
        crate::CorpFinanceError::InvalidInput {
            field: "input_json".into(),
            reason: e.to_string(),
        }
    })?;

    match envelope.kind {
        DocTemplateKind::IcMemo => {
            let input: ic_memo::IcMemoInput =
                serde_json::from_str(&envelope.input_json).map_err(|e| {
                    crate::CorpFinanceError::InvalidInput {
                        field: "input_json.payload".into(),
                        reason: e.to_string(),
                    }
                })?;
            Ok(ic_memo::ic_memo_to_doc(&input))
        }
        DocTemplateKind::ResearchInit => {
            let input: research_init::ResearchInitInput =
                serde_json::from_str(&envelope.input_json).map_err(|e| {
                    crate::CorpFinanceError::InvalidInput {
                        field: "input_json.payload".into(),
                        reason: e.to_string(),
                    }
                })?;
            Ok(research_init::research_init_to_doc(&input))
        }
    }
}

// ---------------------------------------------------------------------------
// Deck-template dispatcher (Phase 29 Wave 9, pptx deliverables)
// ---------------------------------------------------------------------------

#[cfg(feature = "office")]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "schema_gen", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum DeckTemplateKind {
    PitchDeck,
    IcPresentation,
}

#[cfg(feature = "office")]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "schema_gen", derive(schemars::JsonSchema))]
pub struct RenderDeckTemplateInput {
    pub kind: DeckTemplateKind,
    pub input_json: String,
}

#[cfg(feature = "office")]
pub fn render_deck_template_from_json(
    input_json: &str,
) -> crate::CorpFinanceResult<crate::office::SlideDeckSpec> {
    let envelope: RenderDeckTemplateInput = serde_json::from_str(input_json).map_err(|e| {
        crate::CorpFinanceError::InvalidInput {
            field: "input_json".into(),
            reason: e.to_string(),
        }
    })?;

    match envelope.kind {
        DeckTemplateKind::PitchDeck => {
            let input: pitch_deck::PitchDeckInput =
                serde_json::from_str(&envelope.input_json).map_err(|e| {
                    crate::CorpFinanceError::InvalidInput {
                        field: "input_json.payload".into(),
                        reason: e.to_string(),
                    }
                })?;
            Ok(pitch_deck::pitch_deck_to_deck(&input))
        }
        DeckTemplateKind::IcPresentation => {
            let input: ic_presentation::IcPresentationInput =
                serde_json::from_str(&envelope.input_json).map_err(|e| {
                    crate::CorpFinanceError::InvalidInput {
                        field: "input_json.payload".into(),
                        reason: e.to_string(),
                    }
                })?;
            Ok(ic_presentation::ic_presentation_to_deck(&input))
        }
    }
}

// ---------------------------------------------------------------------------
// Wave 9 dispatcher tests
// ---------------------------------------------------------------------------

#[cfg(all(test, feature = "office"))]
mod wave9_tests {
    use super::*;

    fn ic_memo_envelope() -> String {
        let payload = serde_json::json!({
            "deal_name": "Project Atlas",
            "target_company": "Atlas Industries",
            "date": "2026-05-08",
            "recommendation": "APPROVE",
            "author": "Test Author",
            "investment_thesis": "Compelling market position.",
            "key_metrics": [["Revenue", "100M"]],
            "financial_summary": [["Year", "Revenue"], ["2024", "100M"]],
            "risks": [],
            "mitigants": [],
            "conclusion": "Recommend approval."
        }).to_string();
        serde_json::json!({"kind": "ic_memo", "input_json": payload}).to_string()
    }

    fn research_init_envelope() -> String {
        let payload = serde_json::json!({
            "ticker": "AAPL",
            "company": "Apple Inc.",
            "date": "2026-05-08",
            "rating": "BUY",
            "target_price": "$200.00",
            "current_price": "$185.00",
            "upside_pct": "+8.1%",
            "author": "Test Analyst",
            "exec_summary": "Initiating with BUY.",
            "investment_thesis": "Premium franchise.",
            "business_description": "Consumer electronics.",
            "financial_highlights": [["Year", "Revenue"], ["FY24", "$400B"]],
            "valuation_summary": [["DCF", "$210"]],
            "catalysts": [],
            "risks": []
        }).to_string();
        serde_json::json!({"kind": "research_init", "input_json": payload}).to_string()
    }

    fn pitch_deck_envelope() -> String {
        let payload = serde_json::json!({
            "deal_name": "Project Atlas",
            "subtitle": "Confidential",
            "date": "2026-05-08",
            "agenda": ["Overview", "Financials"],
            "executive_summary": ["Bullet 1"],
            "market_overview": ["Bullet 1"],
            "business_overview": ["Bullet 1"],
            "financial_highlights": [["Year", "Revenue"], ["2024", "100M"]],
            "returns_summary": [["MOIC", "3.0x"]],
            "process_timeline": [["IOI", "2026-06"]],
            "conclusion": ["Bullet 1"]
        }).to_string();
        serde_json::json!({"kind": "pitch_deck", "input_json": payload}).to_string()
    }

    fn ic_presentation_envelope() -> String {
        let payload = serde_json::json!({
            "deal_name": "Project Atlas",
            "recommendation": "APPROVE",
            "date": "2026-05-08",
            "presenter": "Test Presenter",
            "investment_thesis": ["Bullet 1"],
            "key_metrics": [["Revenue", "100M"]],
            "returns_table": [["Scenario", "MOIC"], ["Base", "3.0x"]],
            "risks": [],
            "mitigants": [],
            "timeline": []
        }).to_string();
        serde_json::json!({"kind": "ic_presentation", "input_json": payload}).to_string()
    }

    #[test]
    fn render_ic_memo_returns_doc_with_sections() {
        let doc = render_doc_template_from_json(&ic_memo_envelope()).unwrap();
        assert!(doc.sections.len() >= 5);
    }

    #[test]
    fn render_research_init_returns_doc_with_sections() {
        let doc = render_doc_template_from_json(&research_init_envelope()).unwrap();
        assert!(doc.sections.len() >= 5);
    }

    #[test]
    fn render_pitch_deck_returns_deck_with_slides() {
        let deck = render_deck_template_from_json(&pitch_deck_envelope()).unwrap();
        assert!(deck.slides.len() >= 5);
    }

    #[test]
    fn render_ic_presentation_returns_deck_with_slides() {
        let deck = render_deck_template_from_json(&ic_presentation_envelope()).unwrap();
        assert!(deck.slides.len() >= 5);
    }

    #[test]
    fn render_doc_template_invalid_kind_errors() {
        let bad = r#"{"kind":"not_a_kind","input_json":"{}"}"#;
        assert!(render_doc_template_from_json(bad).is_err());
    }

    #[test]
    fn render_deck_template_invalid_kind_errors() {
        let bad = r#"{"kind":"not_a_kind","input_json":"{}"}"#;
        assert!(render_deck_template_from_json(bad).is_err());
    }
}
