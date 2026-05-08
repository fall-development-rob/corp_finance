//! WebAssembly bindings for corp-finance-core.
//!
//! Each exported function takes a JSON string (matching the corresponding
//! `*Input` struct in corp-finance-core) and returns a JSON string with the
//! computed `*Output`. This mirrors the NAPI bindings in
//! `packages/bindings/src/lib.rs` so the MCP server adapter stays simple.
//!
//! v0.1 surface = timing-free feature subset. v0.2 will add modules that use
//! `std::time::Instant` once the `web-time` shim is wired in.

use wasm_bindgen::prelude::*;

fn err_to_js(e: impl std::fmt::Display) -> JsValue {
    JsValue::from_str(&e.to_string())
}

macro_rules! wasm_tool {
    ($name:ident, $input_path:path, $fn_path:path) => {
        #[wasm_bindgen]
        pub fn $name(input_json: &str) -> Result<String, JsValue> {
            let input: $input_path = serde_json::from_str(input_json).map_err(err_to_js)?;
            let output = $fn_path(&input).map_err(err_to_js)?;
            serde_json::to_string(&output).map_err(err_to_js)
        }
    };
}

// ---------------------------------------------------------------------------
// Valuation
// ---------------------------------------------------------------------------

wasm_tool!(
    calculate_wacc,
    corp_finance_core::valuation::wacc::WaccInput,
    corp_finance_core::valuation::wacc::calculate_wacc
);

wasm_tool!(
    build_dcf,
    corp_finance_core::valuation::dcf::DcfInput,
    corp_finance_core::valuation::dcf::calculate_dcf
);

wasm_tool!(
    comps_analysis,
    corp_finance_core::valuation::comps::CompsInput,
    corp_finance_core::valuation::comps::calculate_comps
);

// ---------------------------------------------------------------------------
// Credit
// ---------------------------------------------------------------------------

wasm_tool!(
    credit_metrics,
    corp_finance_core::credit::metrics::CreditMetricsInput,
    corp_finance_core::credit::metrics::calculate_credit_metrics
);

wasm_tool!(
    debt_capacity,
    corp_finance_core::credit::capacity::DebtCapacityInput,
    corp_finance_core::credit::capacity::calculate_debt_capacity
);

wasm_tool!(
    covenant_compliance,
    corp_finance_core::credit::covenants::CovenantTestInput,
    corp_finance_core::credit::covenants::test_covenants
);

// ---------------------------------------------------------------------------
// FP&A (Wave 16a pilot port)
// ---------------------------------------------------------------------------

wasm_tool!(
    variance_analysis,
    corp_finance_core::fpa::variance::VarianceInput,
    corp_finance_core::fpa::variance::analyze_variance
);

wasm_tool!(
    breakeven_analysis,
    corp_finance_core::fpa::variance::BreakevenInput,
    corp_finance_core::fpa::variance::analyze_breakeven
);

wasm_tool!(
    working_capital,
    corp_finance_core::fpa::working_capital::WorkingCapitalInput,
    corp_finance_core::fpa::working_capital::analyze_working_capital
);

wasm_tool!(
    rolling_forecast,
    corp_finance_core::fpa::working_capital::RollingForecastInput,
    corp_finance_core::fpa::working_capital::build_rolling_forecast
);

// ---------------------------------------------------------------------------
// Wave 16x — behavioral module (2 tools)
// ---------------------------------------------------------------------------

wasm_tool!(
    prospect_theory,
    corp_finance_core::behavioral::prospect_theory::ProspectTheoryInput,
    corp_finance_core::behavioral::prospect_theory::analyze_prospect_theory
);

wasm_tool!(
    market_sentiment,
    corp_finance_core::behavioral::sentiment::SentimentInput,
    corp_finance_core::behavioral::sentiment::analyze_sentiment
);

// ---------------------------------------------------------------------------
// Wave 16x — performance_attribution module (2 tools)
// ---------------------------------------------------------------------------

wasm_tool!(
    brinson_attribution,
    corp_finance_core::performance_attribution::brinson::BrinsonInput,
    corp_finance_core::performance_attribution::brinson::brinson_attribution
);

wasm_tool!(
    factor_attribution,
    corp_finance_core::performance_attribution::factor_attribution::FactorAttributionInput,
    corp_finance_core::performance_attribution::factor_attribution::factor_attribution
);

// ---------------------------------------------------------------------------
// Wave 16x — quant_strategies module (2 tools)
// ---------------------------------------------------------------------------

wasm_tool!(
    pairs_trading,
    corp_finance_core::quant_strategies::pairs_trading::PairsTradingInput,
    corp_finance_core::quant_strategies::pairs_trading::analyze_pairs_trading
);

wasm_tool!(
    momentum_analysis,
    corp_finance_core::quant_strategies::momentum::MomentumInput,
    corp_finance_core::quant_strategies::momentum::analyze_momentum
);

// ---------------------------------------------------------------------------
// Wave 16x — equity_research module (2 tools)
// ---------------------------------------------------------------------------

wasm_tool!(
    sotp_valuation,
    corp_finance_core::equity_research::sotp::SotpInput,
    corp_finance_core::equity_research::sotp::calculate_sotp
);

wasm_tool!(
    target_price,
    corp_finance_core::equity_research::target_price::TargetPriceInput,
    corp_finance_core::equity_research::target_price::calculate_target_price
);

// ---------------------------------------------------------------------------
// Wave 16x — commodity_trading module (2 tools)
// ---------------------------------------------------------------------------

wasm_tool!(
    commodity_spread,
    corp_finance_core::commodity_trading::spreads::CommoditySpreadInput,
    corp_finance_core::commodity_trading::spreads::analyze_commodity_spread
);

wasm_tool!(
    storage_economics,
    corp_finance_core::commodity_trading::storage::StorageEconomicsInput,
    corp_finance_core::commodity_trading::storage::analyze_storage_economics
);

// ---------------------------------------------------------------------------
// Wave 16x — dividend_policy module (5 tools)
// ---------------------------------------------------------------------------

wasm_tool!(
    h_model_ddm,
    corp_finance_core::dividend_policy::h_model::HModelInput,
    corp_finance_core::dividend_policy::h_model::calculate_h_model
);

wasm_tool!(
    multistage_ddm,
    corp_finance_core::dividend_policy::multistage_ddm::MultistageDdmInput,
    corp_finance_core::dividend_policy::multistage_ddm::calculate_multistage_ddm
);

wasm_tool!(
    buyback_analysis,
    corp_finance_core::dividend_policy::buyback::BuybackInput,
    corp_finance_core::dividend_policy::buyback::calculate_buyback
);

wasm_tool!(
    payout_sustainability,
    corp_finance_core::dividend_policy::payout_sustainability::PayoutSustainabilityInput,
    corp_finance_core::dividend_policy::payout_sustainability::calculate_payout_sustainability
);

wasm_tool!(
    total_shareholder_return,
    corp_finance_core::dividend_policy::total_shareholder_return::TotalShareholderReturnInput,
    corp_finance_core::dividend_policy::total_shareholder_return::calculate_total_shareholder_return
);

#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}
