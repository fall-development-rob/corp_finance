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

// ---------------------------------------------------------------------------
// Three-Statement Model
// ---------------------------------------------------------------------------

#[napi]
pub fn build_three_statement(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::three_statement::model::ThreeStatementInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::three_statement::model::build_three_statement_model(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

// ---------------------------------------------------------------------------
// Monte Carlo
// ---------------------------------------------------------------------------

#[napi]
pub fn run_monte_carlo(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::monte_carlo::simulation::MonteCarloInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::monte_carlo::simulation::run_monte_carlo_simulation(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn run_mc_dcf(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::monte_carlo::simulation::McDcfInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::monte_carlo::simulation::run_monte_carlo_dcf(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

// ---------------------------------------------------------------------------
// Quant Risk
// ---------------------------------------------------------------------------

#[napi]
pub fn run_factor_model(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::quant_risk::factor_models::FactorModelInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::quant_risk::factor_models::run_factor_model(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn run_black_litterman(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::quant_risk::black_litterman::BlackLittermanInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::quant_risk::black_litterman::run_black_litterman(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn calculate_risk_parity(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::quant_risk::risk_parity::RiskParityInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::quant_risk::risk_parity::calculate_risk_parity(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn run_stress_test(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::quant_risk::stress_testing::StressTestInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::quant_risk::stress_testing::run_stress_test(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

// ---------------------------------------------------------------------------
// Restructuring
// ---------------------------------------------------------------------------

#[napi]
pub fn analyze_recovery(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::restructuring::recovery::RecoveryAnalysisInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::restructuring::recovery::analyze_recovery(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn analyze_distressed_debt(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::restructuring::distressed_debt::DistressedDebtInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::restructuring::distressed_debt::analyze_distressed_debt(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

// ---------------------------------------------------------------------------
// Real Assets
// ---------------------------------------------------------------------------

#[napi]
pub fn value_property(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::real_assets::real_estate::PropertyValuationInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::real_assets::real_estate::value_property(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn model_project_finance(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::real_assets::project_finance::ProjectFinanceInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::real_assets::project_finance::model_project_finance(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

// ---------------------------------------------------------------------------
// FX & Commodities
// ---------------------------------------------------------------------------

#[napi]
pub fn price_fx_forward(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::fx_commodities::fx::FxForwardInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output =
        corp_finance_core::fx_commodities::fx::price_fx_forward(&input).map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn calculate_cross_rate(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::fx_commodities::fx::CrossRateInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::fx_commodities::fx::calculate_cross_rate(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn price_commodity_forward(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::fx_commodities::commodities::CommodityForwardInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::fx_commodities::commodities::price_commodity_forward(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn analyze_commodity_curve(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::fx_commodities::commodities::CommodityCurveInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::fx_commodities::commodities::analyze_commodity_curve(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

// ---------------------------------------------------------------------------
// Scenarios
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Securitization
// ---------------------------------------------------------------------------

#[napi]
pub fn model_abs_cashflows(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::securitization::abs_mbs::AbsMbsInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::securitization::abs_mbs::model_abs_cashflows(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn analyze_tranching(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::securitization::tranching::TranchingInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::securitization::tranching::analyze_tranching(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

// ---------------------------------------------------------------------------
// Venture Capital
// ---------------------------------------------------------------------------

#[napi]
pub fn model_funding_round(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::venture::valuation::FundingRoundInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::venture::valuation::model_funding_round(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn analyze_dilution(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::venture::valuation::DilutionInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output =
        corp_finance_core::venture::valuation::analyze_dilution(&input).map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn convert_note(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::venture::instruments::ConvertibleNoteInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output =
        corp_finance_core::venture::instruments::convert_note(&input).map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn convert_safe(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::venture::instruments::SafeInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output =
        corp_finance_core::venture::instruments::convert_safe(&input).map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn model_venture_fund(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::venture::returns::VentureFundInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output =
        corp_finance_core::venture::returns::model_venture_fund(&input).map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

// ---------------------------------------------------------------------------
// ESG
// ---------------------------------------------------------------------------

#[napi]
pub fn calculate_esg_score(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::esg::scoring::EsgScoreInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output =
        corp_finance_core::esg::scoring::calculate_esg_score(&input).map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn analyze_carbon_footprint(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::esg::climate::CarbonFootprintInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output =
        corp_finance_core::esg::climate::analyze_carbon_footprint(&input).map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn analyze_green_bond(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::esg::climate::GreenBondInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output =
        corp_finance_core::esg::climate::analyze_green_bond(&input).map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn test_sll_covenants(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::esg::climate::SllInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output =
        corp_finance_core::esg::climate::test_sll_covenants(&input).map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

// ---------------------------------------------------------------------------
// Regulatory
// ---------------------------------------------------------------------------

#[napi]
pub fn calculate_regulatory_capital(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::regulatory::capital::RegulatoryCapitalInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::regulatory::capital::calculate_regulatory_capital(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn calculate_lcr(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::regulatory::liquidity::LcrInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output =
        corp_finance_core::regulatory::liquidity::calculate_lcr(&input).map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn calculate_nsfr(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::regulatory::liquidity::NsfrInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output =
        corp_finance_core::regulatory::liquidity::calculate_nsfr(&input).map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn analyze_alm(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::regulatory::alm::AlmInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::regulatory::alm::analyze_alm(&input).map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

// ---------------------------------------------------------------------------
// Private Credit
// ---------------------------------------------------------------------------

#[napi]
pub fn price_unitranche(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::private_credit::unitranche::UnitrancheInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::private_credit::unitranche::price_unitranche(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn model_direct_loan(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::private_credit::direct_lending::DirectLoanInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::private_credit::direct_lending::model_direct_loan(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn analyze_syndication(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::private_credit::direct_lending::SyndicationInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::private_credit::direct_lending::analyze_syndication(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

// ---------------------------------------------------------------------------
// Insurance
// ---------------------------------------------------------------------------

#[napi]
pub fn estimate_reserves(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::insurance::reserving::ReservingInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::insurance::reserving::estimate_reserves(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn price_premium(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::insurance::pricing::PremiumPricingInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output =
        corp_finance_core::insurance::pricing::price_premium(&input).map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn analyze_combined_ratio(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::insurance::pricing::CombinedRatioInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::insurance::pricing::analyze_combined_ratio(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn calculate_scr(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::insurance::pricing::ScrInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output =
        corp_finance_core::insurance::pricing::calculate_scr(&input).map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

// ---------------------------------------------------------------------------
// FP&A (Financial Planning & Analysis)
// ---------------------------------------------------------------------------

#[napi]
pub fn analyze_variance(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::fpa::variance::VarianceInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output =
        corp_finance_core::fpa::variance::analyze_variance(&input).map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn analyze_breakeven(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::fpa::variance::BreakevenInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output =
        corp_finance_core::fpa::variance::analyze_breakeven(&input).map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn analyze_working_capital(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::fpa::working_capital::WorkingCapitalInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::fpa::working_capital::analyze_working_capital(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn build_rolling_forecast(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::fpa::working_capital::RollingForecastInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::fpa::working_capital::build_rolling_forecast(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

// ---------------------------------------------------------------------------
// Wealth Management
// ---------------------------------------------------------------------------

#[napi]
pub fn plan_retirement(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::wealth::retirement::RetirementInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output =
        corp_finance_core::wealth::retirement::plan_retirement(&input).map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn simulate_tax_loss_harvesting(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::wealth::tax_estate::TlhInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::wealth::tax_estate::simulate_tax_loss_harvesting(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn plan_estate(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::wealth::tax_estate::EstatePlanInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output =
        corp_finance_core::wealth::tax_estate::plan_estate(&input).map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

// ---------------------------------------------------------------------------
// Crypto / Digital Assets — Phase 8
// ---------------------------------------------------------------------------

#[napi]
pub fn value_token(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::crypto::valuation::TokenValuationInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output =
        corp_finance_core::crypto::valuation::value_token(&input).map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn analyze_defi(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::crypto::defi::DefiYieldInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::crypto::defi::analyze_defi(&input).map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

// ---------------------------------------------------------------------------
// Municipal Bonds — Phase 8
// ---------------------------------------------------------------------------

#[napi]
pub fn price_muni_bond(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::municipal::bonds::MuniBondInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output =
        corp_finance_core::municipal::bonds::price_muni_bond(&input).map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn analyze_municipal(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::municipal::analysis::MuniAnalysisInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output =
        corp_finance_core::municipal::analysis::analyze_municipal(&input).map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

// ---------------------------------------------------------------------------
// Structured Products — Phase 8
// ---------------------------------------------------------------------------

#[napi]
pub fn price_structured_note(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::structured_products::notes::StructuredNoteInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::structured_products::notes::price_structured_note(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn price_exotic(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::structured_products::exotic::ExoticProductInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::structured_products::exotic::price_exotic(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

// ---------------------------------------------------------------------------
// Trade Finance — Phase 8
// ---------------------------------------------------------------------------

#[napi]
pub fn price_letter_of_credit(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::trade_finance::letter_of_credit::LetterOfCreditInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::trade_finance::letter_of_credit::price_letter_of_credit(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn analyze_supply_chain_finance(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::trade_finance::supply_chain::SupplyChainFinanceInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output =
        corp_finance_core::trade_finance::supply_chain::analyze_supply_chain_finance(&input)
            .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

// ---------------------------------------------------------------------------
// Credit Derivatives — Phase 9
// ---------------------------------------------------------------------------

#[napi]
pub fn price_cds(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::credit_derivatives::cds::CdsInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output =
        corp_finance_core::credit_derivatives::cds::price_cds(&input).map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn calculate_cva(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::credit_derivatives::cva::CvaInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output =
        corp_finance_core::credit_derivatives::cva::calculate_cva(&input).map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

// ---------------------------------------------------------------------------
// Convertible Bonds — Phase 9
// ---------------------------------------------------------------------------

#[napi]
pub fn price_convertible(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::convertibles::pricing::ConvertibleBondInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::convertibles::pricing::price_convertible(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn analyze_convertible(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::convertibles::analysis::ConvertibleAnalysisInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::convertibles::analysis::analyze_convertible(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

// ---------------------------------------------------------------------------
// Lease Accounting — Phase 9
// ---------------------------------------------------------------------------

#[napi]
pub fn classify_lease(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::lease_accounting::classification::LeaseInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::lease_accounting::classification::classify_lease(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn analyze_sale_leaseback(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::lease_accounting::sale_leaseback::SaleLeasebackInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output =
        corp_finance_core::lease_accounting::sale_leaseback::analyze_sale_leaseback(&input)
            .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

// ---------------------------------------------------------------------------
// Pension & LDI — Phase 9
// ---------------------------------------------------------------------------

#[napi]
pub fn analyze_pension_funding(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::pension::funding::PensionFundingInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::pension::funding::analyze_pension_funding(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn design_ldi_strategy(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::pension::ldi::LdiInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output =
        corp_finance_core::pension::ldi::design_ldi_strategy(&input).map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

// ---------------------------------------------------------------------------
// Sovereign — Phase 10
// ---------------------------------------------------------------------------

#[napi]
pub fn analyze_sovereign_bond(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::sovereign::sovereign_bonds::SovereignBondInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::sovereign::sovereign_bonds::analyze_sovereign_bond(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn assess_country_risk(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::sovereign::country_risk::CountryRiskInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::sovereign::country_risk::assess_country_risk(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

// ---------------------------------------------------------------------------
// Real Options — Phase 10
// ---------------------------------------------------------------------------

#[napi]
pub fn value_real_option(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::real_options::valuation::RealOptionInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::real_options::valuation::value_real_option(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn analyze_decision_tree(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::real_options::decision_tree::DecisionTreeInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::real_options::decision_tree::analyze_decision_tree(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

// ---------------------------------------------------------------------------
// Equity Research — Phase 10
// ---------------------------------------------------------------------------

#[napi]
pub fn calculate_sotp(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::equity_research::sotp::SotpInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output =
        corp_finance_core::equity_research::sotp::calculate_sotp(&input).map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn calculate_target_price(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::equity_research::target_price::TargetPriceInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::equity_research::target_price::calculate_target_price(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

// ---------------------------------------------------------------------------
// Commodity Trading — Phase 10
// ---------------------------------------------------------------------------

#[napi]
pub fn analyze_commodity_spread(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::commodity_trading::spreads::CommoditySpreadInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::commodity_trading::spreads::analyze_commodity_spread(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn analyze_storage_economics(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::commodity_trading::storage::StorageEconomicsInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::commodity_trading::storage::analyze_storage_economics(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}
