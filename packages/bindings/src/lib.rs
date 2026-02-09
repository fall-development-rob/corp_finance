use napi::Result as NapiResult;
use napi_derive::napi;

/// Convert any Display error into a napi::Error.
fn to_napi_error(e: impl std::fmt::Display) -> napi::Error {
    napi::Error::from_reason(e.to_string())
}

// ---------------------------------------------------------------------------
// Valuation
// ---------------------------------------------------------------------------

#[napi]
pub fn calculate_wacc(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::valuation::wacc::WaccInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::valuation::wacc::calculate_wacc(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn build_dcf(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::valuation::dcf::DcfInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::valuation::dcf::calculate_dcf(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn comps_analysis(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::valuation::comps::CompsInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::valuation::comps::calculate_comps(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

// ---------------------------------------------------------------------------
// Credit
// ---------------------------------------------------------------------------

#[napi]
pub fn credit_metrics(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::credit::metrics::CreditMetricsInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::credit::metrics::calculate_credit_metrics(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn debt_capacity(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::credit::capacity::DebtCapacityInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::credit::capacity::calculate_debt_capacity(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn covenant_compliance(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::credit::covenants::CovenantTestInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::credit::covenants::test_covenants(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

// ---------------------------------------------------------------------------
// Private Equity
// ---------------------------------------------------------------------------

#[napi]
pub fn calculate_returns(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::pe::returns::ReturnsInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::pe::returns::calculate_returns(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn build_debt_schedule(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::pe::debt_schedule::DebtTrancheInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::pe::debt_schedule::build_debt_schedule(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn sources_and_uses(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::pe::sources_uses::SourcesUsesInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::pe::sources_uses::build_sources_uses(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

// ---------------------------------------------------------------------------
// Portfolio
// ---------------------------------------------------------------------------

#[napi]
pub fn risk_adjusted_returns(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::portfolio::returns::RiskAdjustedInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::portfolio::returns::calculate_risk_adjusted_returns(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn risk_metrics(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::portfolio::risk::RiskMetricsInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::portfolio::risk::calculate_risk_metrics(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn kelly_sizing(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::portfolio::sizing::KellyInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::portfolio::sizing::calculate_kelly(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

// ---------------------------------------------------------------------------
// Scenarios
// ---------------------------------------------------------------------------

#[napi]
pub fn build_sensitivity_grid(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::scenarios::sensitivity::SensitivityInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::scenarios::sensitivity::build_sensitivity_grid(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[derive(serde::Deserialize)]
struct ScenarioBindingInput {
    #[serde(flatten)]
    input: corp_finance_core::scenarios::scenario::ScenarioInput,
    output_values: Vec<rust_decimal::Decimal>,
    base_case_value: rust_decimal::Decimal,
}

#[napi]
pub fn scenario_analysis(input_json: String) -> NapiResult<String> {
    let binding_input: ScenarioBindingInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::scenarios::scenario::analyze_scenarios(
        &binding_input.input,
        &binding_input.output_values,
        binding_input.base_case_value,
    )
    .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}
