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
    let output =
        corp_finance_core::valuation::wacc::calculate_wacc(&input).map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn build_dcf(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::valuation::dcf::DcfInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::valuation::dcf::calculate_dcf(&input).map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn comps_analysis(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::valuation::comps::CompsInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output =
        corp_finance_core::valuation::comps::calculate_comps(&input).map_err(to_napi_error)?;
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
    let output =
        corp_finance_core::credit::covenants::test_covenants(&input).map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

// ---------------------------------------------------------------------------
// Private Equity
// ---------------------------------------------------------------------------

#[napi]
pub fn calculate_returns(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::pe::returns::ReturnsInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output =
        corp_finance_core::pe::returns::calculate_returns(&input).map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn build_debt_schedule(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::pe::debt_schedule::DebtTrancheInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output =
        corp_finance_core::pe::debt_schedule::build_debt_schedule(&input).map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn sources_and_uses(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::pe::sources_uses::SourcesUsesInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output =
        corp_finance_core::pe::sources_uses::build_sources_uses(&input).map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

// ---------------------------------------------------------------------------
// Private Equity — Phase 2
// ---------------------------------------------------------------------------

#[napi]
pub fn build_lbo(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::pe::lbo::LboInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::pe::lbo::build_lbo(&input).map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn calculate_waterfall(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::pe::waterfall::WaterfallInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output =
        corp_finance_core::pe::waterfall::calculate_waterfall(&input).map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

// ---------------------------------------------------------------------------
// M&A
// ---------------------------------------------------------------------------

#[napi]
pub fn analyze_merger(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::ma::merger_model::MergerInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output =
        corp_finance_core::ma::merger_model::analyze_merger(&input).map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

// ---------------------------------------------------------------------------
// Credit — Phase 2
// ---------------------------------------------------------------------------

#[napi]
pub fn altman_zscore(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::credit::altman::AltmanInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::credit::altman::calculate_altman_zscore(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

// ---------------------------------------------------------------------------
// Jurisdiction / Fund
// ---------------------------------------------------------------------------

#[napi]
pub fn calculate_fund_fees(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::jurisdiction::fund_fees::FundFeeInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::jurisdiction::fund_fees::calculate_fund_fees(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn reconcile_accounting(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::jurisdiction::reconciliation::ReconciliationInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output =
        corp_finance_core::jurisdiction::reconciliation::reconcile_accounting_standards(&input)
            .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn calculate_wht(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::jurisdiction::withholding_tax::WhtInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output =
        corp_finance_core::jurisdiction::withholding_tax::calculate_withholding_tax(&input)
            .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn calculate_portfolio_wht(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::jurisdiction::withholding_tax::PortfolioWhtInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::jurisdiction::withholding_tax::calculate_portfolio_wht(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn calculate_nav(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::jurisdiction::nav::NavInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output =
        corp_finance_core::jurisdiction::nav::calculate_nav(&input).map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn calculate_gp_economics(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::jurisdiction::gp_economics::GpEconomicsInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::jurisdiction::gp_economics::calculate_gp_economics(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn calculate_investor_net_returns(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::jurisdiction::investor_returns::InvestorNetReturnsInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output =
        corp_finance_core::jurisdiction::investor_returns::calculate_investor_net_returns(&input)
            .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn screen_ubti_eci(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::jurisdiction::ubti::UbtiScreeningInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output =
        corp_finance_core::jurisdiction::ubti::screen_ubti_eci(&input).map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

// ---------------------------------------------------------------------------
// Fixed Income
// ---------------------------------------------------------------------------

#[napi]
pub fn price_bond(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::fixed_income::bonds::BondPricingInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output =
        corp_finance_core::fixed_income::bonds::price_bond(&input).map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn calculate_bond_yield(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::fixed_income::yields::BondYieldInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::fixed_income::yields::calculate_bond_yield(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn bootstrap_spot_curve(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::fixed_income::yields::BootstrapInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::fixed_income::yields::bootstrap_spot_curve(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn fit_nelson_siegel(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::fixed_income::yields::NelsonSiegelInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::fixed_income::yields::fit_nelson_siegel(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn calculate_duration(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::fixed_income::duration::DurationInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::fixed_income::duration::calculate_duration(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn calculate_credit_spreads(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::fixed_income::spreads::CreditSpreadInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::fixed_income::spreads::calculate_credit_spreads(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

// ---------------------------------------------------------------------------
// Derivatives
// ---------------------------------------------------------------------------

#[napi]
pub fn price_option(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::derivatives::options::OptionInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output =
        corp_finance_core::derivatives::options::price_option(&input).map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn implied_volatility(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::derivatives::options::ImpliedVolInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::derivatives::options::implied_volatility(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn price_forward(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::derivatives::forwards::ForwardInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output =
        corp_finance_core::derivatives::forwards::price_forward(&input).map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn value_forward_position(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::derivatives::forwards::ForwardPositionInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::derivatives::forwards::value_forward_position(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn futures_basis_analysis(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::derivatives::forwards::BasisAnalysisInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::derivatives::forwards::futures_basis_analysis(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn value_interest_rate_swap(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::derivatives::swaps::IrsInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::derivatives::swaps::value_interest_rate_swap(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn value_currency_swap(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::derivatives::swaps::CurrencySwapInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::derivatives::swaps::value_currency_swap(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn analyze_strategy(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::derivatives::strategies::StrategyInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::derivatives::strategies::analyze_strategy(&input)
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
    let output =
        corp_finance_core::portfolio::sizing::calculate_kelly(&input).map_err(to_napi_error)?;
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
