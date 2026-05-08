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

#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}
