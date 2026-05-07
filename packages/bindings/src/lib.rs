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
// Institutional Real Estate
// ---------------------------------------------------------------------------

#[napi]
pub fn tenant_schedule(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::institutional_real_estate::rent_roll::TenantScheduleInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::institutional_real_estate::rent_roll::tenant_schedule(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn lease_rollover(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::institutional_real_estate::rent_roll::LeaseRolloverInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::institutional_real_estate::rent_roll::lease_rollover(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn comp_adjustment_grid(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::institutional_real_estate::comparable_sales::CompAdjustmentInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output =
        corp_finance_core::institutional_real_estate::comparable_sales::comp_adjustment_grid(
            &input,
        )
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn comp_reconciliation(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::institutional_real_estate::comparable_sales::ReconciliationInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output =
        corp_finance_core::institutional_real_estate::comparable_sales::reconciliation(&input)
            .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn hbu_analysis(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::institutional_real_estate::highest_best_use::HbuAnalysisInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output =
        corp_finance_core::institutional_real_estate::highest_best_use::hbu_analysis(&input)
            .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn financially_feasible(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::institutional_real_estate::highest_best_use::FinanciallyFeasibleInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output =
        corp_finance_core::institutional_real_estate::highest_best_use::financially_feasible(
            &input,
        )
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn cost_approach(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::institutional_real_estate::replacement_cost::CostApproachInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output =
        corp_finance_core::institutional_real_estate::replacement_cost::cost_approach(&input)
            .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn marshall_swift(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::institutional_real_estate::replacement_cost::MarshallSwiftInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output =
        corp_finance_core::institutional_real_estate::replacement_cost::marshall_swift(&input)
            .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn ncreif_attribution(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::institutional_real_estate::benchmark::NcreifAttributionInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output =
        corp_finance_core::institutional_real_estate::benchmark::ncreif_attribution(&input)
            .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn odce_comparison(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::institutional_real_estate::benchmark::OdceComparisonInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::institutional_real_estate::benchmark::odce_comparison(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn acquisition_model(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::institutional_real_estate::acquisition::AcquisitionModelInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output =
        corp_finance_core::institutional_real_estate::acquisition::acquisition_model(&input)
            .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn development_feasibility(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::institutional_real_estate::acquisition::DevelopmentFeasibilityInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output =
        corp_finance_core::institutional_real_estate::acquisition::development_feasibility(&input)
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

// ---------------------------------------------------------------------------
// Quant Strategies — Phase 11
// ---------------------------------------------------------------------------

#[napi]
pub fn analyze_pairs_trading(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::quant_strategies::pairs_trading::PairsTradingInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::quant_strategies::pairs_trading::analyze_pairs_trading(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn analyze_momentum(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::quant_strategies::momentum::MomentumInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::quant_strategies::momentum::analyze_momentum(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

// ---------------------------------------------------------------------------
// Treasury — Phase 11
// ---------------------------------------------------------------------------

#[napi]
pub fn analyze_cash_management(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::treasury::cash_management::CashManagementInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::treasury::cash_management::analyze_cash_management(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn analyze_hedging(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::treasury::hedging::HedgingInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output =
        corp_finance_core::treasury::hedging::analyze_hedging(&input).map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

// ---------------------------------------------------------------------------
// Infrastructure — Phase 11
// ---------------------------------------------------------------------------

#[napi]
pub fn model_ppp(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::infrastructure::ppp_model::PppModelInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output =
        corp_finance_core::infrastructure::ppp_model::model_ppp(&input).map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn value_concession(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::infrastructure::concession::ConcessionInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::infrastructure::concession::value_concession(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

// ---------------------------------------------------------------------------
// Behavioral Finance — Phase 11
// ---------------------------------------------------------------------------

#[napi]
pub fn analyze_prospect_theory(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::behavioral::prospect_theory::ProspectTheoryInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::behavioral::prospect_theory::analyze_prospect_theory(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn analyze_sentiment(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::behavioral::sentiment::SentimentInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::behavioral::sentiment::analyze_sentiment(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

// ---------------------------------------------------------------------------
// Performance Attribution — Phase 12
// ---------------------------------------------------------------------------

#[napi]
pub fn brinson_attribution(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::performance_attribution::brinson::BrinsonInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::performance_attribution::brinson::brinson_attribution(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn factor_attribution(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::performance_attribution::factor_attribution::FactorAttributionInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output =
        corp_finance_core::performance_attribution::factor_attribution::factor_attribution(&input)
            .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

// ---------------------------------------------------------------------------
// Credit Portfolio — Phase 12
// ---------------------------------------------------------------------------

#[napi]
pub fn calculate_portfolio_credit_risk(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::credit_portfolio::portfolio_risk::PortfolioRiskInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output =
        corp_finance_core::credit_portfolio::portfolio_risk::calculate_portfolio_risk(&input)
            .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn calculate_migration(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::credit_portfolio::migration::MigrationInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::credit_portfolio::migration::calculate_migration(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

// ---------------------------------------------------------------------------
// Macro Economics — Phase 12
// ---------------------------------------------------------------------------

#[napi]
pub fn analyze_monetary_policy(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::macro_economics::monetary_policy::MonetaryPolicyInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output =
        corp_finance_core::macro_economics::monetary_policy::analyze_monetary_policy(&input)
            .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn analyze_international(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::macro_economics::international::InternationalInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::macro_economics::international::analyze_international(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

// ---------------------------------------------------------------------------
// Compliance — Phase 12
// ---------------------------------------------------------------------------

#[napi]
pub fn analyze_best_execution(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::compliance::best_execution::BestExecutionInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::compliance::best_execution::analyze_best_execution(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn generate_gips_report(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::compliance::reporting::GipsInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::compliance::reporting::generate_gips_report(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

// ---------------------------------------------------------------------------
// Onshore Structures — Phase 13
// ---------------------------------------------------------------------------

#[napi]
pub fn analyze_us_fund_structure(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::onshore_structures::us_funds::UsFundInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::onshore_structures::us_funds::analyze_us_fund_structure(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn analyze_uk_eu_fund(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::onshore_structures::uk_eu_funds::UkEuFundInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::onshore_structures::uk_eu_funds::analyze_uk_eu_fund(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

// ---------------------------------------------------------------------------
// Offshore Structures — Phase 13
// ---------------------------------------------------------------------------

#[napi]
pub fn analyze_cayman_structure(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::offshore_structures::cayman::CaymanFundInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::offshore_structures::cayman::analyze_cayman_structure(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn analyze_lux_structure(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::offshore_structures::luxembourg::LuxFundInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::offshore_structures::luxembourg::analyze_lux_structure(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

// ---------------------------------------------------------------------------
// Offshore Structures Expansion — Phase 23
// ---------------------------------------------------------------------------

#[napi]
pub fn analyze_jersey_fund(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::offshore_structures::channel_islands::JerseyFundInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output =
        corp_finance_core::offshore_structures::channel_islands::analyze_jersey_fund(&input)
            .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn analyze_guernsey_fund(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::offshore_structures::channel_islands::GuernseyFundInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output =
        corp_finance_core::offshore_structures::channel_islands::analyze_guernsey_fund(&input)
            .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn cell_company_analysis(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::offshore_structures::channel_islands::CellCompanyInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output =
        corp_finance_core::offshore_structures::channel_islands::cell_company_analysis(&input)
            .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn analyze_vcc_structure(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::offshore_structures::singapore_vcc::VccInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output =
        corp_finance_core::offshore_structures::singapore_vcc::analyze_vcc_structure(&input)
            .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn vcc_tax_incentive_analysis(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::offshore_structures::singapore_vcc::TaxIncentiveInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output =
        corp_finance_core::offshore_structures::singapore_vcc::tax_incentive_analysis(&input)
            .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn analyze_ofc_structure(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::offshore_structures::hong_kong_funds::OfcInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output =
        corp_finance_core::offshore_structures::hong_kong_funds::analyze_ofc_structure(&input)
            .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn analyze_lpf_structure(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::offshore_structures::hong_kong_funds::LpfInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output =
        corp_finance_core::offshore_structures::hong_kong_funds::analyze_lpf_structure(&input)
            .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn hk_carried_interest_concession(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::offshore_structures::hong_kong_funds::CarriedInterestInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output =
        corp_finance_core::offshore_structures::hong_kong_funds::carried_interest_concession(
            &input,
        )
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn analyze_difc_fund(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::offshore_structures::middle_east_funds::DifcFundInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output =
        corp_finance_core::offshore_structures::middle_east_funds::analyze_difc_fund(&input)
            .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn analyze_adgm_fund(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::offshore_structures::middle_east_funds::AdgmFundInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output =
        corp_finance_core::offshore_structures::middle_east_funds::analyze_adgm_fund(&input)
            .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn compare_jurisdictions(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::offshore_structures::jurisdiction_comparison::JurisdictionComparisonInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output =
        corp_finance_core::offshore_structures::jurisdiction_comparison::compare_jurisdictions(
            &input,
        )
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn migration_feasibility(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::offshore_structures::fund_migration::MigrationFeasibilityInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output =
        corp_finance_core::offshore_structures::fund_migration::migration_feasibility(&input)
            .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

// ---------------------------------------------------------------------------
// Transfer Pricing — Phase 13
// ---------------------------------------------------------------------------

#[napi]
pub fn analyze_beps_compliance(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::transfer_pricing::beps::BepsInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::transfer_pricing::beps::analyze_beps_compliance(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn analyze_intercompany(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::transfer_pricing::intercompany::IntercompanyInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::transfer_pricing::intercompany::analyze_intercompany(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

// ---------------------------------------------------------------------------
// Tax Treaty — Phase 13
// ---------------------------------------------------------------------------

#[napi]
pub fn analyze_treaty_network(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::tax_treaty::treaty_network::TreatyNetworkInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::tax_treaty::treaty_network::analyze_treaty_network(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn optimize_treaty_structure(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::tax_treaty::optimization::TreatyOptInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::tax_treaty::optimization::optimize_treaty_structure(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

// ---------------------------------------------------------------------------
// FATCA/CRS — Phase 14
// ---------------------------------------------------------------------------

#[napi]
pub fn analyze_fatca_crs_reporting(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::fatca_crs::reporting::FatcaCrsReportingInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::fatca_crs::reporting::analyze_fatca_crs_reporting(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn classify_entity(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::fatca_crs::classification::EntityClassificationInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::fatca_crs::classification::classify_entity(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

// ---------------------------------------------------------------------------
// Substance Requirements — Phase 14
// ---------------------------------------------------------------------------

#[napi]
pub fn analyze_economic_substance(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::substance_requirements::economic_substance::EconomicSubstanceInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output =
        corp_finance_core::substance_requirements::economic_substance::analyze_economic_substance(
            &input,
        )
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn run_jurisdiction_substance_test(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::substance_requirements::jurisdiction_tests::JurisdictionTestInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::substance_requirements::jurisdiction_tests::run_jurisdiction_substance_test(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

// ---------------------------------------------------------------------------
// Regulatory Reporting — Phase 14
// ---------------------------------------------------------------------------

#[napi]
pub fn generate_aifmd_report(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::regulatory_reporting::aifmd_reporting::AifmdReportingInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output =
        corp_finance_core::regulatory_reporting::aifmd_reporting::generate_aifmd_report(&input)
            .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn generate_sec_cftc_report(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::regulatory_reporting::sec_cftc_reporting::SecCftcReportingInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output =
        corp_finance_core::regulatory_reporting::sec_cftc_reporting::generate_sec_cftc_report(
            &input,
        )
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

// ---------------------------------------------------------------------------
// AML Compliance — Phase 14
// ---------------------------------------------------------------------------

#[napi]
pub fn assess_kyc_risk(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::aml_compliance::kyc_scoring::KycRiskInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::aml_compliance::kyc_scoring::assess_kyc_risk(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn screen_sanctions(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::aml_compliance::sanctions_screening::SanctionsScreeningInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::aml_compliance::sanctions_screening::screen_sanctions(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

// ---------------------------------------------------------------------------
// Volatility Surface — Phase 15
// ---------------------------------------------------------------------------

#[napi]
pub fn build_implied_vol_surface(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::volatility_surface::implied_vol_surface::ImpliedVolSurfaceInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output =
        corp_finance_core::volatility_surface::implied_vol_surface::build_implied_vol_surface(
            &input,
        )
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn calibrate_sabr(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::volatility_surface::sabr_model::SabrCalibrationInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::volatility_surface::sabr_model::calibrate_sabr(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

// ---------------------------------------------------------------------------
// Portfolio Optimization — Phase 15
// ---------------------------------------------------------------------------

#[napi]
pub fn optimize_mean_variance(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::portfolio_optimization::mean_variance::MeanVarianceInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output =
        corp_finance_core::portfolio_optimization::mean_variance::optimize_mean_variance(&input)
            .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn optimize_black_litterman_portfolio(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::portfolio_optimization::black_litterman_portfolio::BlackLittermanInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::portfolio_optimization::black_litterman_portfolio::optimize_black_litterman(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

// ---------------------------------------------------------------------------
// Risk Budgeting — Phase 15
// ---------------------------------------------------------------------------

#[napi]
pub fn analyze_factor_risk_budget(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::risk_budgeting::factor_risk_budget::FactorRiskBudgetInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output =
        corp_finance_core::risk_budgeting::factor_risk_budget::analyze_factor_risk_budget(&input)
            .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn analyze_tail_risk(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::risk_budgeting::tail_risk::TailRiskInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::risk_budgeting::tail_risk::analyze_tail_risk(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

// ---------------------------------------------------------------------------
// Market Microstructure — Phase 15
// ---------------------------------------------------------------------------

#[napi]
pub fn analyze_spreads(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::market_microstructure::spread_analysis::SpreadAnalysisInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::market_microstructure::spread_analysis::analyze_spreads(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn optimize_execution(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::market_microstructure::optimal_execution::OptimalExecutionInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output =
        corp_finance_core::market_microstructure::optimal_execution::optimize_execution(&input)
            .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

// ---------------------------------------------------------------------------
// Interest Rate Models — Phase 16
// ---------------------------------------------------------------------------

#[napi]
pub fn analyze_short_rate(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::interest_rate_models::short_rate::ShortRateInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::interest_rate_models::short_rate::analyze_short_rate(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn fit_term_structure(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::interest_rate_models::term_structure::TermStructureInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output =
        corp_finance_core::interest_rate_models::term_structure::fit_term_structure(&input)
            .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

// ---------------------------------------------------------------------------
// Mortgage Analytics — Phase 16
// ---------------------------------------------------------------------------

#[napi]
pub fn analyze_prepayment(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::mortgage_analytics::prepayment::PrepaymentInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::mortgage_analytics::prepayment::analyze_prepayment(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn analyze_mbs(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::mortgage_analytics::mbs_analytics::MbsAnalyticsInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::mortgage_analytics::mbs_analytics::analyze_mbs(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

// ---------------------------------------------------------------------------
// Inflation-Linked — Phase 16
// ---------------------------------------------------------------------------

#[napi]
pub fn analyze_tips(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::inflation_linked::tips_pricing::TipsAnalyticsInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::inflation_linked::tips_pricing::analyze_tips(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn analyze_inflation_derivatives(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::inflation_linked::inflation_derivatives::InflationDerivativeInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output =
        corp_finance_core::inflation_linked::inflation_derivatives::analyze_inflation_derivatives(
            &input,
        )
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

// ---------------------------------------------------------------------------
// Repo Financing — Phase 16
// ---------------------------------------------------------------------------

#[napi]
pub fn analyze_repo(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::repo_financing::repo_rates::RepoAnalyticsInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::repo_financing::repo_rates::analyze_repo(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn analyze_collateral(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::repo_financing::collateral_management::CollateralInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output =
        corp_finance_core::repo_financing::collateral_management::analyze_collateral(&input)
            .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

// ---------------------------------------------------------------------------
// Credit Scoring — Phase 18
// ---------------------------------------------------------------------------

#[napi]
pub fn calculate_scorecard(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::credit_scoring::scorecard::ScorecardInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::credit_scoring::scorecard::calculate_scorecard(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn calculate_merton(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::credit_scoring::structural_model::MertonInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::credit_scoring::structural_model::calculate_merton(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn calculate_intensity_model(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::credit_scoring::intensity_model::IntensityModelInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output =
        corp_finance_core::credit_scoring::intensity_model::calculate_intensity_model(&input)
            .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn calculate_calibration(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::credit_scoring::calibration::CalibrationInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::credit_scoring::calibration::calculate_calibration(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn calculate_scoring_validation(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::credit_scoring::validation::ValidationInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::credit_scoring::validation::calculate_validation(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

// ---------------------------------------------------------------------------
// Capital Allocation — Phase 18
// ---------------------------------------------------------------------------

#[napi]
pub fn calculate_economic_capital(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::capital_allocation::economic_capital::EconomicCapitalInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output =
        corp_finance_core::capital_allocation::economic_capital::calculate_economic_capital(&input)
            .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn calculate_raroc(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::capital_allocation::raroc::RarocInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::capital_allocation::raroc::calculate_raroc(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn calculate_euler_allocation(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::capital_allocation::euler_allocation::EulerAllocationInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output =
        corp_finance_core::capital_allocation::euler_allocation::calculate_euler_allocation(&input)
            .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn calculate_shapley_allocation(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::capital_allocation::shapley_allocation::ShapleyAllocationInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output =
        corp_finance_core::capital_allocation::shapley_allocation::calculate_shapley_allocation(
            &input,
        )
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn evaluate_limits(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::capital_allocation::limit_management::LimitManagementInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::capital_allocation::limit_management::evaluate_limits(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

// ---------------------------------------------------------------------------
// CLO Analytics — Phase 18
// ---------------------------------------------------------------------------

#[napi]
pub fn calculate_clo_waterfall(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::clo_analytics::waterfall::WaterfallInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::clo_analytics::waterfall::calculate_waterfall(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn calculate_coverage_tests(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::clo_analytics::coverage_tests::CoverageTestInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::clo_analytics::coverage_tests::calculate_coverage_tests(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn calculate_reinvestment(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::clo_analytics::reinvestment::ReinvestmentInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::clo_analytics::reinvestment::calculate_reinvestment(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn calculate_tranche_analytics(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::clo_analytics::tranche_analytics::TrancheAnalyticsInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output =
        corp_finance_core::clo_analytics::tranche_analytics::calculate_tranche_analytics(&input)
            .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn calculate_clo_scenario(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::clo_analytics::scenario::CloScenarioInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::clo_analytics::scenario::calculate_clo_scenario(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

// ---------------------------------------------------------------------------
// Fund of Funds — Phase 18
// ---------------------------------------------------------------------------

#[napi]
pub fn calculate_j_curve(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::fund_of_funds::j_curve::JCurveInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::fund_of_funds::j_curve::calculate_j_curve(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn calculate_commitment_pacing(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::fund_of_funds::commitment_pacing::CommitmentPacingInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output =
        corp_finance_core::fund_of_funds::commitment_pacing::calculate_commitment_pacing(&input)
            .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn analyze_manager_selection(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::fund_of_funds::manager_selection::ManagerSelectionInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output =
        corp_finance_core::fund_of_funds::manager_selection::analyze_manager_selection(&input)
            .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn calculate_secondaries_pricing(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::fund_of_funds::secondaries::SecondariesPricingInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output =
        corp_finance_core::fund_of_funds::secondaries::calculate_secondaries_pricing(&input)
            .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn analyze_fof_portfolio(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::fund_of_funds::portfolio_construction::FofPortfolioInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output =
        corp_finance_core::fund_of_funds::portfolio_construction::analyze_fof_portfolio(&input)
            .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

// ---------------------------------------------------------------------------
// Earnings Quality — Phase 19
// ---------------------------------------------------------------------------

#[napi]
pub fn calculate_beneish_mscore(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::earnings_quality::beneish::BeneishInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::earnings_quality::beneish::calculate_beneish_m_score(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn calculate_piotroski_fscore(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::earnings_quality::piotroski::PiotroskiInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output =
        corp_finance_core::earnings_quality::piotroski::calculate_piotroski_f_score(&input)
            .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn calculate_accrual_quality(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::earnings_quality::accrual_quality::AccrualQualityInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output =
        corp_finance_core::earnings_quality::accrual_quality::calculate_accrual_quality(&input)
            .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn calculate_revenue_quality(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::earnings_quality::revenue_quality::RevenueQualityInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output =
        corp_finance_core::earnings_quality::revenue_quality::calculate_revenue_quality(&input)
            .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn calculate_earnings_quality_composite(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::earnings_quality::composite::EarningsQualityCompositeInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output =
        corp_finance_core::earnings_quality::composite::calculate_earnings_quality_composite(
            &input,
        )
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

// ---------------------------------------------------------------------------
// Bank Analytics — Phase 19
// ---------------------------------------------------------------------------

#[napi]
pub fn analyze_nim(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::bank_analytics::nim_analysis::NimAnalysisInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::bank_analytics::nim_analysis::analyze_nim(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn calculate_camels_rating(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::bank_analytics::camels::CamelsInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::bank_analytics::camels::calculate_camels(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn calculate_cecl_provision(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::bank_analytics::cecl_provisioning::CeclProvisioningInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::bank_analytics::cecl_provisioning::calculate_cecl(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn analyze_deposit_beta(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::bank_analytics::deposit_beta::DepositBetaInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::bank_analytics::deposit_beta::analyze_deposit_beta(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn analyze_loan_book(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::bank_analytics::loan_book::LoanBookInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::bank_analytics::loan_book::analyze_loan_book(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

// ---------------------------------------------------------------------------
// Dividend Policy — Phase 19
// ---------------------------------------------------------------------------

#[napi]
pub fn calculate_h_model_ddm(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::dividend_policy::h_model::HModelInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::dividend_policy::h_model::calculate_h_model(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn calculate_multistage_ddm(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::dividend_policy::multistage_ddm::MultistageDdmInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output =
        corp_finance_core::dividend_policy::multistage_ddm::calculate_multistage_ddm(&input)
            .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn analyze_buyback(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::dividend_policy::buyback::BuybackInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::dividend_policy::buyback::calculate_buyback(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn analyze_payout_sustainability(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::dividend_policy::payout_sustainability::PayoutSustainabilityInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output =
        corp_finance_core::dividend_policy::payout_sustainability::calculate_payout_sustainability(
            &input,
        )
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn calculate_total_shareholder_return(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::dividend_policy::total_shareholder_return::TotalShareholderReturnInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output =
        corp_finance_core::dividend_policy::total_shareholder_return::calculate_total_shareholder_return(
            &input,
        )
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

// ---------------------------------------------------------------------------
// Carbon Markets — Phase 19
// ---------------------------------------------------------------------------

#[napi]
pub fn price_carbon_credit(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::carbon_markets::carbon_pricing::CarbonPricingInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output =
        corp_finance_core::carbon_markets::carbon_pricing::calculate_carbon_pricing(&input)
            .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn analyze_ets_compliance(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::carbon_markets::ets_compliance::EtsComplianceInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output =
        corp_finance_core::carbon_markets::ets_compliance::calculate_ets_compliance(&input)
            .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn analyze_cbam(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::carbon_markets::cbam::CbamInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output =
        corp_finance_core::carbon_markets::cbam::calculate_cbam(&input).map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn value_carbon_offset(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::carbon_markets::offset_valuation::OffsetValuationInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output =
        corp_finance_core::carbon_markets::offset_valuation::calculate_offset_valuation(&input)
            .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn calculate_shadow_carbon_price(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::carbon_markets::shadow_carbon::ShadowCarbonInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::carbon_markets::shadow_carbon::calculate_shadow_carbon(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

// ---------------------------------------------------------------------------
// Private Wealth — Phase 20
// ---------------------------------------------------------------------------

#[napi]
pub fn analyze_concentrated_stock(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::private_wealth::concentrated_stock::ConcentratedStockInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output =
        corp_finance_core::private_wealth::concentrated_stock::analyze_concentrated_stock(&input)
            .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn compare_philanthropic_vehicles(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::private_wealth::philanthropic_vehicles::PhilanthropicInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output =
        corp_finance_core::private_wealth::philanthropic_vehicles::compare_philanthropic_vehicles(
            &input,
        )
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn analyze_wealth_transfer(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::private_wealth::wealth_transfer::WealthTransferInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output =
        corp_finance_core::private_wealth::wealth_transfer::analyze_wealth_transfer(&input)
            .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn analyze_direct_indexing(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::private_wealth::direct_indexing::DirectIndexingInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output =
        corp_finance_core::private_wealth::direct_indexing::analyze_direct_indexing(&input)
            .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn evaluate_family_governance(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::private_wealth::family_governance::FamilyGovernanceInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output =
        corp_finance_core::private_wealth::family_governance::evaluate_family_governance(&input)
            .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

// ---------------------------------------------------------------------------
// Emerging Markets — Phase 20
// ---------------------------------------------------------------------------

#[napi]
pub fn calculate_country_risk_premium(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::emerging_markets::country_risk_premium::CountryRiskPremiumInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output =
        corp_finance_core::emerging_markets::country_risk_premium::calculate_country_risk_premium(
            &input,
        )
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn assess_political_risk(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::emerging_markets::political_risk::PoliticalRiskInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::emerging_markets::political_risk::assess_political_risk(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn analyse_capital_controls(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::emerging_markets::capital_controls::CapitalControlsInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output =
        corp_finance_core::emerging_markets::capital_controls::analyse_capital_controls(&input)
            .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn analyse_em_bonds(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::emerging_markets::em_bond_analysis::EmBondAnalysisInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::emerging_markets::em_bond_analysis::analyse_em_bonds(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn calculate_em_equity_premium(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::emerging_markets::em_equity_premium::EmEquityPremiumInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output =
        corp_finance_core::emerging_markets::em_equity_premium::calculate_em_equity_premium(&input)
            .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

// ---------------------------------------------------------------------------
// Index Construction — Phase 20
// ---------------------------------------------------------------------------

#[napi]
pub fn calculate_weighting(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::index_construction::weighting::WeightingInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::index_construction::weighting::calculate_weighting(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn calculate_rebalancing(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::index_construction::rebalancing::RebalancingInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::index_construction::rebalancing::calculate_rebalancing(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn calculate_tracking_error(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::index_construction::tracking_error::TrackingErrorInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output =
        corp_finance_core::index_construction::tracking_error::calculate_tracking_error(&input)
            .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn calculate_smart_beta(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::index_construction::smart_beta::SmartBetaInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::index_construction::smart_beta::calculate_smart_beta(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn calculate_reconstitution(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::index_construction::reconstitution::ReconstitutionInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output =
        corp_finance_core::index_construction::reconstitution::calculate_reconstitution(&input)
            .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

// ---------------------------------------------------------------------------
// Financial Forensics — Phase 20
// ---------------------------------------------------------------------------

#[napi]
pub fn analyze_benfords_law(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::financial_forensics::benfords_law::BenfordsLawInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::financial_forensics::benfords_law::analyze_benfords_law(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn calculate_dupont(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::financial_forensics::dupont_analysis::DupontInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::financial_forensics::dupont_analysis::calculate_dupont(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn calculate_zscore_models(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::financial_forensics::zscore_models::ZScoreModelsInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output =
        corp_finance_core::financial_forensics::zscore_models::calculate_zscore_models(&input)
            .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn calculate_peer_benchmarking(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::financial_forensics::peer_benchmarking::PeerBenchmarkingInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output =
        corp_finance_core::financial_forensics::peer_benchmarking::calculate_peer_benchmarking(
            &input,
        )
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn calculate_red_flag_scoring(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::financial_forensics::red_flag_scoring::RedFlagScoringInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output =
        corp_finance_core::financial_forensics::red_flag_scoring::calculate_red_flag_scoring(
            &input,
        )
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

// ---------------------------------------------------------------------------
// Workflows
// ---------------------------------------------------------------------------

#[napi]
pub fn workflow_list(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::workflows::types::WorkflowListInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output =
        corp_finance_core::workflows::types::list_workflows(&input).map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn workflow_describe(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::workflows::types::WorkflowDescribeInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output =
        corp_finance_core::workflows::types::describe_workflow(&input).map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn workflow_validate(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::workflows::types::WorkflowValidateInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output =
        corp_finance_core::workflows::types::validate_workflow(&input).map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn workflow_quality_check(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::workflows::types::WorkflowQualityCheckInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::workflows::types::quality_check_workflow(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn workflow_audit(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::workflows::audit::WorkflowAuditInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output =
        corp_finance_core::workflows::audit::generate_audit_trail(&input).map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

// ---------------------------------------------------------------------------
// Managed-agent cookbook tooling (validate / check-all / list / sync /
// deploy / orchestrate). Pattern mirrors workflow_* above.
// ---------------------------------------------------------------------------

#[napi]
pub fn managed_agent_list(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::managed_agent::types::ListInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output =
        corp_finance_core::managed_agent::list::list_cookbooks(&input).map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn managed_agent_validate(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::managed_agent::types::ValidateInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::managed_agent::validate::validate_manifest(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn managed_agent_check_all(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::managed_agent::types::CheckAllInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output =
        corp_finance_core::managed_agent::validate::validate_all(&input).map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn managed_agent_sync(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::managed_agent::types::SyncInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output =
        corp_finance_core::managed_agent::sync::sync_skills(&input).map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn managed_agent_deploy(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::managed_agent::types::DeployInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::managed_agent::deploy::build_deploy_payload(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn managed_agent_orchestrate(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::managed_agent::types::OrchestrateInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output = corp_finance_core::managed_agent::orchestrate::route_event(&input)
        .map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

// ---------------------------------------------------------------------------
// MCP-server tier registry. Pure data — lets users discover which MCP
// servers are free vs paid-vendor without paying for any subscription.
// ---------------------------------------------------------------------------

#[napi]
pub fn mcp_server_list(input_json: String) -> NapiResult<String> {
    let input: corp_finance_core::mcp_servers::types::McpListInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let output =
        corp_finance_core::mcp_servers::list::list_servers(&input).map_err(to_napi_error)?;
    serde_json::to_string(&output).map_err(to_napi_error)
}

// ---------------------------------------------------------------------------
// Memory bindings (Phase 26 — ADR-016).
//
// HNSW + BM25 hybrid retrieval over `RunSummary` records, plus portable
// `.cfa-session` archive save/restore. The HNSW and BM25 indexes are
// serialised to disk between calls (HNSW via the gzip+JSON envelope written
// by `HnswMemoryIndex::save_to`; BM25 is rebuilt from the HNSW summary side
// store on each call — Phase 26 BM25 is in-RAM only, see module docs).
// ---------------------------------------------------------------------------

#[derive(serde::Deserialize)]
struct MemoryIngestInput {
    run_summary: corp_finance_core::memory::RunSummary,
    /// Optional path to a persisted HNSW index (gzip+JSON envelope). When
    /// supplied, the index is loaded from disk, the new summary is appended,
    /// and the index is rewritten in place. When omitted, only the
    /// in-process side store is exercised — useful for tests / dry runs.
    #[serde(default)]
    hnsw_path: Option<String>,
    /// Embedding dimensionality used when creating a fresh index. Required
    /// when `hnsw_path` does not yet exist.
    #[serde(default)]
    embedding_dim: Option<usize>,
}

#[derive(serde::Serialize)]
struct MemoryIngestOutput {
    run_id: uuid::Uuid,
    indexed_at: chrono::DateTime<chrono::Utc>,
}

#[napi]
pub fn surface_memory_ingest(input_json: String) -> NapiResult<String> {
    let input: MemoryIngestInput = serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let run_id = input.run_summary.run_id;

    if let Some(path_str) = input.hnsw_path.as_deref() {
        let path = std::path::Path::new(path_str);
        let mut idx = if path.exists() {
            corp_finance_core::memory::HnswMemoryIndex::load_from(path).map_err(to_napi_error)?
        } else {
            let dim = input
                .embedding_dim
                .unwrap_or(input.run_summary.embedding.len());
            corp_finance_core::memory::HnswMemoryIndex::new(dim)
        };
        idx.ingest(&input.run_summary).map_err(to_napi_error)?;
        idx.save_to(path).map_err(to_napi_error)?;
    }

    let output = MemoryIngestOutput {
        run_id,
        indexed_at: chrono::Utc::now(),
    };
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[derive(serde::Deserialize)]
struct MemoryFindInput {
    query: corp_finance_core::memory::MemoryQuery,
    /// Optional path to a persisted HNSW index. When omitted the result is
    /// an empty hit list (no in-process state survives between calls).
    #[serde(default)]
    hnsw_path: Option<String>,
}

#[derive(serde::Serialize)]
struct MemoryHit {
    run_summary: corp_finance_core::memory::RunSummary,
    score: f32,
}

#[derive(serde::Serialize)]
struct MemoryFindOutput {
    hits: Vec<MemoryHit>,
}

#[napi]
pub fn surface_memory_find(input_json: String) -> NapiResult<String> {
    let input: MemoryFindInput = serde_json::from_str(&input_json).map_err(to_napi_error)?;

    // Hybrid retrieval: HNSW for embeddings, BM25 for text. Phase 26 BM25 is
    // in-RAM only and rebuilt from the HNSW side store on each call so the
    // two retrievers operate over the same `RunSummary` corpus.
    let mut hits: Vec<MemoryHit> = Vec::new();

    if let Some(path_str) = input.hnsw_path.as_deref() {
        let path = std::path::Path::new(path_str);
        if path.exists() {
            let idx = corp_finance_core::memory::HnswMemoryIndex::load_from(path)
                .map_err(to_napi_error)?;
            let surface_filter = input.query.surface;
            let tenant_filter = input.query.tenant_id.clone();
            let filter = |s: &corp_finance_core::memory::RunSummary| -> bool {
                if let Some(want) = surface_filter {
                    if s.surface != want {
                        return false;
                    }
                }
                if let Some(ref want) = tenant_filter {
                    if s.tenant_id.as_deref() != Some(want.as_str()) {
                        return false;
                    }
                }
                true
            };

            // Vector recall.
            if let Some(ref emb) = input.query.embedding {
                for (s, d) in idx.query(emb, input.query.limit, filter) {
                    hits.push(MemoryHit {
                        run_summary: s,
                        score: d,
                    });
                }
            }

            // Text recall via BM25 over the same side store.
            if let Some(ref text) = input.query.text {
                let mut bm25 =
                    corp_finance_core::memory::BM25MemoryIndex::new().map_err(to_napi_error)?;
                // We don't have direct access to the side store, so re-run
                // the embedding query with a permissive filter just to walk
                // the records. Skip this branch when only text is supplied
                // (no embedding) — the v1 retriever degrades gracefully.
                if let Some(ref emb) = input.query.embedding {
                    let walk = idx.query(emb, idx.len(), |_| true);
                    for (s, _) in &walk {
                        bm25.ingest(s).map_err(to_napi_error)?;
                    }
                    let results = bm25
                        .query(text, input.query.limit, input.query.tenant_id.as_deref())
                        .map_err(to_napi_error)?;
                    // Phase 28 cleanup widened the BM25 query return type
                    // from `Vec<RunSummary>` to `Vec<BM25Hit>` so the
                    // tantivy-emitted relevance score is no longer dropped
                    // on the wire. Map each hit's `bm25_score` straight onto
                    // `MemoryHit::score` so JS callers can rank by it.
                    for hit in results {
                        hits.push(MemoryHit {
                            run_summary: hit.run_summary,
                            score: hit.bm25_score,
                        });
                    }
                }
            }
        }
    }

    // Cap to limit and dedupe by run_id (vector + text recall can overlap).
    let mut seen: std::collections::HashSet<uuid::Uuid> = std::collections::HashSet::new();
    let mut deduped: Vec<MemoryHit> = Vec::with_capacity(hits.len());
    for h in hits {
        if seen.insert(h.run_summary.run_id) {
            deduped.push(h);
            if deduped.len() >= input.query.limit {
                break;
            }
        }
    }

    let output = MemoryFindOutput { hits: deduped };
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[derive(serde::Deserialize)]
struct SessionSaveInput {
    session: corp_finance_core::memory::CfaSession,
    path: String,
}

#[derive(serde::Serialize)]
struct SessionSaveOutput {
    path: String,
    sha256: String,
}

#[napi]
pub fn surface_session_save(input_json: String) -> NapiResult<String> {
    let input: SessionSaveInput = serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let path = std::path::PathBuf::from(&input.path);
    corp_finance_core::memory::save_session(&input.session, &path).map_err(to_napi_error)?;
    // SHA-256 of the on-disk archive — useful as a content fingerprint for
    // the audit manifest.
    let sha256 =
        corp_finance_core::audit::audit_manifest::sha256_file(&path).map_err(to_napi_error)?;
    let output = SessionSaveOutput {
        path: input.path,
        sha256,
    };
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[derive(serde::Deserialize)]
struct SessionRestoreInput {
    path: String,
}

#[derive(serde::Serialize)]
struct SessionRestoreOutput {
    session: corp_finance_core::memory::CfaSession,
}

#[napi]
pub fn surface_session_restore(input_json: String) -> NapiResult<String> {
    let input: SessionRestoreInput = serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let path = std::path::PathBuf::from(&input.path);
    let session = corp_finance_core::memory::restore_session(&path).map_err(to_napi_error)?;
    let output = SessionRestoreOutput { session };
    serde_json::to_string(&output).map_err(to_napi_error)
}

// ---------------------------------------------------------------------------
// Audit bindings (Phase 26 — ADR-017 §1).
//
// djb2 surface-event audit hashing. The hash is the identity primitive for
// "did the surface event manifest change between this run and that run".
// ---------------------------------------------------------------------------

#[derive(serde::Deserialize)]
struct SurfaceAuditInput {
    manifest: corp_finance_core::audit::SurfaceManifest,
}

#[derive(serde::Serialize)]
struct SurfaceAuditOutput {
    surface_audit_hash: String,
}

#[napi]
pub fn surface_audit_compute(input_json: String) -> NapiResult<String> {
    let input: SurfaceAuditInput = serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let hash = corp_finance_core::audit::compute_surface_audit_hash(&input.manifest);
    let output = SurfaceAuditOutput {
        surface_audit_hash: hash,
    };
    serde_json::to_string(&output).map_err(to_napi_error)
}

// ---------------------------------------------------------------------------
// Cost bindings (Phase 26 — ADR-017 §3).
//
// rusqlite-backed per-surface cost ledger plus budget set / get / threshold
// checking. The ledger path is supplied per call; the binding opens the
// SQLite handle, runs the operation, and closes it. No global state.
// ---------------------------------------------------------------------------

/// Wire form for `CostFilter` (the underlying type does not derive serde).
#[derive(serde::Deserialize, Default)]
struct CostFilterWire {
    #[serde(default)]
    surface: Option<corp_finance_core::cost::Surface>,
    #[serde(default)]
    tier: Option<corp_finance_core::cost::TierTag>,
    #[serde(default)]
    tenant_id: Option<String>,
    #[serde(default)]
    since: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(default)]
    until: Option<chrono::DateTime<chrono::Utc>>,
}

impl From<CostFilterWire> for corp_finance_core::cost::CostFilter {
    fn from(w: CostFilterWire) -> Self {
        corp_finance_core::cost::CostFilter {
            surface: w.surface,
            tier: w.tier,
            tenant_id: w.tenant_id,
            since: w.since,
            until: w.until,
        }
    }
}

/// Wire form for `BudgetFilter` (the underlying type does not derive serde).
#[derive(serde::Deserialize, Default)]
struct BudgetFilterWire {
    #[serde(default)]
    surface_filter: Option<corp_finance_core::cost::Surface>,
    #[serde(default)]
    tier_filter: Option<corp_finance_core::cost::TierTag>,
    #[serde(default)]
    period: Option<corp_finance_core::cost::BudgetPeriod>,
}

impl From<BudgetFilterWire> for corp_finance_core::cost::BudgetFilter {
    fn from(w: BudgetFilterWire) -> Self {
        corp_finance_core::cost::BudgetFilter {
            surface_filter: w.surface_filter,
            tier_filter: w.tier_filter,
            period: w.period,
        }
    }
}

#[derive(serde::Deserialize)]
struct CostSummaryInput {
    ledger_path: String,
    #[serde(default)]
    filter: CostFilterWire,
    group_by: corp_finance_core::cost::GroupBy,
}

#[derive(serde::Serialize)]
struct CostSummaryOutput {
    summary: corp_finance_core::cost::CostSummary,
}

#[napi]
pub fn surface_cost_summary(input_json: String) -> NapiResult<String> {
    let input: CostSummaryInput = serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let ledger =
        corp_finance_core::cost::CostLedger::open(std::path::Path::new(&input.ledger_path))
            .map_err(to_napi_error)?;
    let filter: corp_finance_core::cost::CostFilter = input.filter.into();
    let summary = corp_finance_core::cost::summary(&ledger, &filter, input.group_by)
        .map_err(to_napi_error)?;
    let output = CostSummaryOutput { summary };
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[derive(serde::Deserialize)]
struct CostBudgetSetInput {
    ledger_path: String,
    budget: corp_finance_core::cost::CostBudget,
}

#[derive(serde::Serialize)]
struct CostBudgetSetOutput {
    ok: bool,
}

#[napi]
pub fn surface_cost_budget_set(input_json: String) -> NapiResult<String> {
    let input: CostBudgetSetInput = serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let ledger =
        corp_finance_core::cost::CostLedger::open(std::path::Path::new(&input.ledger_path))
            .map_err(to_napi_error)?;
    corp_finance_core::cost::set_budget(&ledger, &input.budget).map_err(to_napi_error)?;
    let output = CostBudgetSetOutput { ok: true };
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[derive(serde::Deserialize)]
struct CostBudgetGetInput {
    ledger_path: String,
    #[serde(default)]
    filter: BudgetFilterWire,
}

#[derive(serde::Serialize)]
struct CostBudgetGetOutput {
    budget: Option<corp_finance_core::cost::CostBudget>,
    status: Option<corp_finance_core::cost::BudgetStatus>,
}

#[napi]
pub fn surface_cost_budget_get(input_json: String) -> NapiResult<String> {
    let input: CostBudgetGetInput = serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let ledger =
        corp_finance_core::cost::CostLedger::open(std::path::Path::new(&input.ledger_path))
            .map_err(to_napi_error)?;
    let filter: corp_finance_core::cost::BudgetFilter = input.filter.into();
    let budget = corp_finance_core::cost::get_budget(&ledger, &filter).map_err(to_napi_error)?;
    let status = match &budget {
        Some(b) => {
            Some(corp_finance_core::cost::check_threshold(&ledger, b).map_err(to_napi_error)?)
        }
        None => None,
    };
    let output = CostBudgetGetOutput { budget, status };
    serde_json::to_string(&output).map_err(to_napi_error)
}

// ---------------------------------------------------------------------------
// Security bindings (Phase 26 — ADR-017 §5).
//
// 14-category PII scanner + 4-kind prompt-injection detector. Both are pure
// functions over `&str`; spans are byte offsets and the result is sorted by
// `span_start` ascending.
// ---------------------------------------------------------------------------

#[derive(serde::Deserialize)]
struct ScanInput {
    text: String,
}

#[derive(serde::Serialize)]
struct ScanOutput {
    findings: Vec<corp_finance_core::security::Finding>,
}

#[napi]
pub fn surface_pii_scan(input_json: String) -> NapiResult<String> {
    let input: ScanInput = serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let mut findings = corp_finance_core::security::scan_for_pii(&input.text);
    findings.extend(corp_finance_core::security::detect_injection(&input.text));
    findings.sort_by_key(|f| f.span_start);
    let output = ScanOutput { findings };
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[napi]
pub fn surface_injection_detect(input_json: String) -> NapiResult<String> {
    let input: ScanInput = serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let findings = corp_finance_core::security::detect_injection(&input.text);
    let output = ScanOutput { findings };
    serde_json::to_string(&output).map_err(to_napi_error)
}

// ---------------------------------------------------------------------------
// Multi-agent coordination bindings (Phase 27 — ADR-018).
//
// Five NAPI bindings covering the chief-analyst orchestration surface:
// plan emission, replanning, entity-graph pattern detection, agent-tool
// invocation recording, and (stub) trace lookup. All cross the JSON-string
// boundary; petgraph-backed `EntityGraph` is reconstructed from a
// wire-friendly `EntityGraphPayload` because petgraph types are not
// `Serialize`.
// ---------------------------------------------------------------------------

#[derive(serde::Deserialize)]
struct ChiefPlanEmitInput {
    goal: String,
}

#[derive(serde::Serialize)]
struct ChiefPlanEmitOutput {
    plan: corp_finance_core::multi_agent::GoapPlan,
}

#[napi]
pub fn chief_plan_emit(input_json: String) -> NapiResult<String> {
    let input: ChiefPlanEmitInput = serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let catalogue = corp_finance_core::multi_agent::load_action_catalogue();
    let plan =
        corp_finance_core::multi_agent::plan(&input.goal, &catalogue).map_err(to_napi_error)?;
    serde_json::to_string(&ChiefPlanEmitOutput { plan }).map_err(to_napi_error)
}

#[derive(serde::Deserialize)]
struct ChiefPlanReplanInput {
    plan: corp_finance_core::multi_agent::GoapPlan,
    failed_step: uuid::Uuid,
    reason: String,
}

#[derive(serde::Serialize)]
struct ChiefPlanReplanOutput {
    plan: corp_finance_core::multi_agent::GoapPlan,
    replan_count: u8,
}

#[napi]
pub fn chief_plan_replan(input_json: String) -> NapiResult<String> {
    let input: ChiefPlanReplanInput = serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let mut plan = input.plan;
    corp_finance_core::multi_agent::replan(&mut plan, input.failed_step, &input.reason)
        .map_err(to_napi_error)?;
    let replan_count = plan.replan_count;
    let output = ChiefPlanReplanOutput { plan, replan_count };
    serde_json::to_string(&output).map_err(to_napi_error)
}

/// Wire-friendly representation of an `EntityGraph` for the JS boundary.
///
/// The in-process `EntityGraph` wraps a `petgraph::Graph` whose internal
/// types are not `Serialize`. Callers therefore pass a flat list of
/// entities and directed relations; the binding rebuilds the graph in
/// memory via `EntityGraph::add_relation` before running the query.
#[derive(serde::Deserialize)]
struct EntityGraphPayload {
    #[serde(default)]
    entities: Vec<corp_finance_core::multi_agent::EntityRef>,
    #[serde(default)]
    relations: Vec<corp_finance_core::multi_agent::EntityRelation>,
}

#[derive(serde::Deserialize)]
struct ChiefPatternDetectInput {
    graph: EntityGraphPayload,
    threshold: usize,
}

#[derive(serde::Serialize)]
struct ChiefPatternDetectOutput {
    entities: Vec<corp_finance_core::multi_agent::EntityRef>,
}

#[napi]
pub fn chief_pattern_detect(input_json: String) -> NapiResult<String> {
    let input: ChiefPatternDetectInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let mut graph = corp_finance_core::multi_agent::EntityGraph::new();
    for entity in input.graph.entities {
        graph.add_entity(entity);
    }
    for rel in input.graph.relations {
        graph.add_relation(rel.from, rel.to, rel.kind);
    }
    let entities = graph.pattern_detected(input.threshold);
    serde_json::to_string(&ChiefPatternDetectOutput { entities }).map_err(to_napi_error)
}

// ---------------------------------------------------------------------------
// Phase 29 Wave 2: audit-backed agent invocation bindings (ADR-021).
//
// `agent_invoke_record` replaces the Phase 27 stub with a full
// record_invocation_with_audit call that persists both the event file and
// the companion .audit.json under caller-supplied manifest_root.
// `agent_invoke_complete` is new — it calls complete_invocation_with_audit.
// Both return InvocationAuditPaths (event_path, audit_path, surface_audit_hash).
// ---------------------------------------------------------------------------

/// Flat wire input for `agent_invoke_record` (Phase 29 Wave 2).
///
/// Builds an `AgentInvocation` in-process from the primitive fields rather
/// than requiring the caller to pre-assemble the struct.
#[derive(serde::Deserialize)]
struct AgentInvokeRecordInput {
    invocation_id: String,
    target_agent: String,
    input_summary: String,
    parent_invocation_id: Option<String>,
    tenant_id: Option<String>,
    manifest_root: String,
}

#[napi]
pub fn agent_invoke_record(input_json: String) -> NapiResult<String> {
    let input: AgentInvokeRecordInput = serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let invocation_id = uuid::Uuid::parse_str(&input.invocation_id).map_err(to_napi_error)?;
    let parent_id = input
        .parent_invocation_id
        .as_deref()
        .map(uuid::Uuid::parse_str)
        .transpose()
        .map_err(to_napi_error)?;
    let invocation = corp_finance_core::multi_agent::AgentInvocation {
        invocation_id,
        target_agent: input.target_agent,
        parent_invocation_id: parent_id,
        input_summary: input.input_summary,
        tenant_id: input.tenant_id,
        ts: chrono::Utc::now(),
        status: corp_finance_core::multi_agent::InvocationStatus::Pending,
    };
    let paths = corp_finance_core::multi_agent::record_invocation_with_audit(
        &invocation,
        std::path::Path::new(&input.manifest_root),
    )
    .map_err(to_napi_error)?;
    serde_json::to_string(&paths).map_err(to_napi_error)
}

/// Wire input for `agent_invoke_complete` (Phase 29 Wave 2).
#[derive(serde::Deserialize)]
struct AgentInvokeCompleteInput {
    invocation_id: String,
    output_summary: String,
    #[serde(default)]
    entities: Vec<corp_finance_core::multi_agent::EntityRef>,
    tenant_id: Option<String>,
    manifest_root: String,
}

#[napi]
pub fn agent_invoke_complete(input_json: String) -> NapiResult<String> {
    let input: AgentInvokeCompleteInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let invocation_id = uuid::Uuid::parse_str(&input.invocation_id).map_err(to_napi_error)?;
    let paths = corp_finance_core::multi_agent::complete_invocation_with_audit(
        invocation_id,
        &input.output_summary,
        &input.entities,
        input.tenant_id.as_deref(),
        std::path::Path::new(&input.manifest_root),
    )
    .map_err(to_napi_error)?;
    serde_json::to_string(&paths).map_err(to_napi_error)
}

#[derive(serde::Deserialize)]
struct AgentTraceGetInput {
    invocation_id: uuid::Uuid,
}

/// `AgentTraceGet` is a v1 stub that always returns `null`.
///
/// Phase 27 records invocations through `record_invocation` but does not
/// own a trace store; trace persistence belongs to the audit / memory
/// pipeline (Phase 28 follow-up). The shape is fixed so JS callers can
/// rely on `{ invocation, status }` once the backing store ships.
#[derive(serde::Serialize)]
struct AgentTraceGetOutput {
    invocation: Option<corp_finance_core::multi_agent::AgentInvocation>,
    status: Option<corp_finance_core::multi_agent::InvocationStatus>,
}

#[napi]
pub fn agent_trace_get(input_json: String) -> NapiResult<String> {
    // TODO(phase-28): wire to AgentDB / memory module trace store. For v1 we
    // accept the request shape and return null so the JS surface contract is
    // stable.
    let input: AgentTraceGetInput = serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let _ = input.invocation_id;
    let output = AgentTraceGetOutput {
        invocation: None,
        status: None,
    };
    serde_json::to_string(&output).map_err(to_napi_error)
}

// ---------------------------------------------------------------------------
// Federation bindings (Phase 27 — ADR-019).
//
// Five NAPI bindings covering the multi-tenant simple-tenancy surface:
// tenant provisioning, CLI surface resolution, tenant-scoped path
// composition, outbound PII redaction, and behavioral trust scoring.
// `RedactionResult` is not `Serialize` upstream, so the binding maps it
// onto a wire-friendly `RedactionResultDto`.
// ---------------------------------------------------------------------------

#[derive(serde::Deserialize)]
struct TenantProvisionInput {
    tenant: corp_finance_core::federation::Tenant,
    base_dir: String,
}

#[derive(serde::Serialize)]
struct TenantProvisionOutput {
    context: corp_finance_core::federation::TenantContext,
}

#[napi]
pub fn tenant_provision(input_json: String) -> NapiResult<String> {
    let input: TenantProvisionInput = serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let context = corp_finance_core::federation::provision_tenant(
        &input.tenant,
        std::path::Path::new(&input.base_dir),
    )
    .map_err(to_napi_error)?;
    serde_json::to_string(&TenantProvisionOutput { context }).map_err(to_napi_error)
}

#[derive(serde::Deserialize)]
struct TenantResolveCliInput {
    args: Vec<String>,
}

#[derive(serde::Serialize)]
struct TenantResolveCliOutput {
    context: Option<corp_finance_core::federation::TenantContext>,
}

#[napi]
pub fn tenant_resolve_cli(input_json: String) -> NapiResult<String> {
    let input: TenantResolveCliInput = serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let context = corp_finance_core::federation::resolve_tenant_for_cli(&input.args);
    serde_json::to_string(&TenantResolveCliOutput { context }).map_err(to_napi_error)
}

/// Wire-friendly mirror of `federation::tenant::ResourceKind`. The upstream
/// enum is `Copy`-only and not `Serialize` to keep the trait surface small;
/// we map the four-variant DTO onto it at the binding boundary.
#[derive(serde::Deserialize)]
#[serde(rename_all = "snake_case")]
enum ResourceKindDto {
    Output,
    Memory,
    CostLedger,
    Session,
    Audit,
}

impl From<ResourceKindDto> for corp_finance_core::federation::ResourceKind {
    fn from(value: ResourceKindDto) -> Self {
        use corp_finance_core::federation::ResourceKind;
        match value {
            ResourceKindDto::Output => ResourceKind::Output,
            ResourceKindDto::Memory => ResourceKind::Memory,
            ResourceKindDto::CostLedger => ResourceKind::CostLedger,
            ResourceKindDto::Session => ResourceKind::Session,
            ResourceKindDto::Audit => ResourceKind::Audit,
        }
    }
}

#[derive(serde::Deserialize)]
struct TenantScopedPathInput {
    context: corp_finance_core::federation::TenantContext,
    kind: ResourceKindDto,
    name: String,
}

#[derive(serde::Serialize)]
struct TenantScopedPathOutput {
    path: String,
}

#[napi]
pub fn tenant_scoped_path(input_json: String) -> NapiResult<String> {
    let input: TenantScopedPathInput = serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let path = corp_finance_core::federation::tenant_scoped_path(
        &input.context,
        input.kind.into(),
        &input.name,
    );
    let output = TenantScopedPathOutput {
        path: path.to_string_lossy().into_owned(),
    };
    serde_json::to_string(&output).map_err(to_napi_error)
}

/// Wire-friendly mirror of `federation::pii_redaction::RedactionResult`.
///
/// The upstream type is `Debug + Clone + PartialEq + Eq` only (no
/// `Serialize`); the DTO flattens its `HashMap<PiiCategory, usize>` into
/// a string-keyed map for JSON friendliness.
#[derive(serde::Serialize)]
struct RedactionResultDto {
    redacted_text: String,
    findings_count: usize,
    action_taken: corp_finance_core::federation::RedactionAction,
    by_category: std::collections::HashMap<String, usize>,
}

impl From<corp_finance_core::federation::RedactionResult> for RedactionResultDto {
    fn from(value: corp_finance_core::federation::RedactionResult) -> Self {
        let by_category = value
            .by_category
            .into_iter()
            .map(|(cat, count)| (cat.as_str().to_string(), count))
            .collect();
        Self {
            redacted_text: value.redacted_text,
            findings_count: value.findings_count,
            action_taken: value.action_taken,
            by_category,
        }
    }
}

#[derive(serde::Deserialize)]
struct PiiRedactApplyInput {
    text: String,
    policy: corp_finance_core::federation::PIIRedactionPolicy,
}

#[derive(serde::Serialize)]
struct PiiRedactApplyOutput {
    redaction: RedactionResultDto,
}

#[napi]
pub fn pii_redact_apply(input_json: String) -> NapiResult<String> {
    let input: PiiRedactApplyInput = serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let result = corp_finance_core::federation::apply_policy(&input.text, &input.policy)
        .map_err(to_napi_error)?;
    let output = PiiRedactApplyOutput {
        redaction: result.into(),
    };
    serde_json::to_string(&output).map_err(to_napi_error)
}

#[derive(serde::Deserialize)]
struct TrustScoreComputeInput {
    success_rate: f32,
    uptime: f32,
    threat_score: f32,
    integrity_score: f32,
}

#[derive(serde::Serialize)]
struct TrustScoreComputeOutput {
    score: corp_finance_core::federation::TrustScore,
}

#[napi]
pub fn trust_score_compute(input_json: String) -> NapiResult<String> {
    let input: TrustScoreComputeInput = serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let score = corp_finance_core::federation::compute_trust_score(
        input.success_rate,
        input.uptime,
        input.threat_score,
        input.integrity_score,
    );
    serde_json::to_string(&TrustScoreComputeOutput { score }).map_err(to_napi_error)
}

// ---------------------------------------------------------------------------
// Self-Learning bindings (Phase 28 — ADR-020).
//
// Six NAPI bindings covering the self-learning surface:
// trajectory capture / completion / retrieval, replay-driven contract
// tests, drift detection, and golden-set freeze. All cross the JSON
// string boundary; types that are not `Serialize` upstream
// (`ReplayResult`, `ReplayFailure`, `TrajectoryFilter`) are mirrored by
// wire-friendly DTOs declared inline.
// ---------------------------------------------------------------------------

#[derive(serde::Deserialize)]
struct LearnTrajectoryCaptureInput {
    surface: corp_finance_core::surface::Surface,
    surface_event_id: String,
    step: corp_finance_core::self_learning::SurfaceEventRef,
}

#[derive(serde::Serialize)]
struct LearnTrajectoryCaptureOutput {
    ok: bool,
}

#[napi]
pub fn learn_trajectory_capture(input_json: String) -> NapiResult<String> {
    let input: LearnTrajectoryCaptureInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    corp_finance_core::self_learning::capture_trajectory_step(
        input.surface,
        &input.surface_event_id,
        input.step,
    )
    .map_err(to_napi_error)?;
    serde_json::to_string(&LearnTrajectoryCaptureOutput { ok: true }).map_err(to_napi_error)
}

#[derive(serde::Deserialize)]
struct LearnTrajectoryCompleteInput {
    surface: corp_finance_core::surface::Surface,
    surface_event_id: String,
    #[serde(default)]
    eval_grade: Option<corp_finance_core::self_learning::EvalGrade>,
}

#[derive(serde::Serialize)]
struct LearnTrajectoryCompleteOutput {
    trajectory: corp_finance_core::self_learning::Trajectory,
}

#[napi]
pub fn learn_trajectory_complete(input_json: String) -> NapiResult<String> {
    let input: LearnTrajectoryCompleteInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let trajectory = corp_finance_core::self_learning::complete_trajectory(
        input.surface,
        &input.surface_event_id,
        input.eval_grade,
    )
    .map_err(to_napi_error)?;
    serde_json::to_string(&LearnTrajectoryCompleteOutput { trajectory }).map_err(to_napi_error)
}

/// Wire-friendly mirror of `self_learning::trajectory::TrajectoryFilter`.
///
/// The upstream type is intentionally not `Deserialize` (it's a builder).
/// Callers pass the three filter dimensions directly; the binding
/// reconstructs the filter via the builder API.
#[derive(serde::Deserialize, Default)]
struct TrajectoryFilterDto {
    #[serde(default)]
    surface: Option<corp_finance_core::surface::Surface>,
    #[serde(default)]
    eval_grade_min: Option<corp_finance_core::self_learning::EvalGrade>,
    #[serde(default)]
    tenant_id: Option<String>,
}

#[derive(serde::Deserialize)]
struct LearnTrajectoryRetrieveInput {
    query_embedding: Vec<f32>,
    #[serde(default)]
    filter: TrajectoryFilterDto,
    limit: usize,
}

#[derive(serde::Serialize)]
struct LearnTrajectoryRetrieveOutput {
    trajectories: Vec<corp_finance_core::self_learning::Trajectory>,
}

#[napi]
pub fn learn_trajectory_retrieve(input_json: String) -> NapiResult<String> {
    let input: LearnTrajectoryRetrieveInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let mut filter = corp_finance_core::self_learning::TrajectoryFilter::new();
    if let Some(s) = input.filter.surface {
        filter = filter.with_surface(s);
    }
    if let Some(g) = input.filter.eval_grade_min {
        filter = filter.with_eval_grade_min(g);
    }
    if let Some(t) = input.filter.tenant_id {
        filter = filter.with_tenant_id(t);
    }
    let trajectories = corp_finance_core::self_learning::retrieve_similar(
        &input.query_embedding,
        &filter,
        input.limit,
    )
    .map_err(to_napi_error)?;
    serde_json::to_string(&LearnTrajectoryRetrieveOutput { trajectories }).map_err(to_napi_error)
}

/// One pre-recorded `(input_id, output)` pair supplied by the JS caller.
///
/// The Rust replay API takes a dispatcher closure; the NAPI boundary
/// cannot ferry a JS callable across into Rust as a `Fn`, so v1 accepts a
/// flat list of recorded outputs and the binding builds the closure as a
/// HashMap lookup keyed on `input_id` (UUID rendered as string).
#[derive(serde::Deserialize)]
struct ReplayRecordedOutput {
    input_id: String,
    output: serde_json::Value,
}

#[derive(serde::Deserialize)]
struct LearnReplayRunInput {
    golden_set: corp_finance_core::self_learning::GoldenSet,
    #[serde(default)]
    recorded_outputs: Vec<ReplayRecordedOutput>,
}

/// Wire-friendly mirror of `self_learning::replay::ReplayFailure`.
///
/// Upstream is not `Serialize`. We project it onto string-keyed JSON.
#[derive(serde::Serialize)]
struct ReplayFailureDto {
    input_id: uuid::Uuid,
    expected_digest: String,
    actual_digest: String,
    structural_delta: Option<corp_finance_core::self_learning::DriftReport>,
}

/// Wire-friendly mirror of `self_learning::replay::ReplayResult`.
#[derive(serde::Serialize)]
struct ReplayResultDto {
    passed: usize,
    failed: usize,
    failures: Vec<ReplayFailureDto>,
}

#[derive(serde::Serialize)]
struct LearnReplayRunOutput {
    result: ReplayResultDto,
}

#[napi]
pub fn learn_replay_run(input_json: String) -> NapiResult<String> {
    let input: LearnReplayRunInput = serde_json::from_str(&input_json).map_err(to_napi_error)?;
    // Build a lookup keyed on the `input_id` UUID rendered as string.
    let mut recorded: std::collections::HashMap<String, serde_json::Value> =
        std::collections::HashMap::with_capacity(input.recorded_outputs.len());
    for rec in input.recorded_outputs {
        recorded.insert(rec.input_id, rec.output);
    }
    let golden_set = input.golden_set;
    let result = corp_finance_core::self_learning::run_replay(&golden_set, |input_value| {
        // Identify the current input by matching `input_json` against the
        // golden inputs. This is the dispatcher-replacement path: the
        // closure receives `&input.input_json`, so we walk `golden_set.inputs`
        // for a value match and use the matching `input_id` as the lookup
        // key into `recorded`.
        let id = golden_set
            .inputs
            .iter()
            .find(|gi| &gi.input_json == input_value)
            .map(|gi| gi.input_id.to_string());
        match id.and_then(|k| recorded.get(&k).cloned()) {
            Some(out) => Ok(out),
            None => Err(
                corp_finance_core::error::CorpFinanceError::InsufficientData(
                    "no recorded_outputs entry for replay input".into(),
                ),
            ),
        }
    })
    .map_err(to_napi_error)?;
    let dto = ReplayResultDto {
        passed: result.passed,
        failed: result.failed,
        failures: result
            .failures
            .into_iter()
            .map(|f| ReplayFailureDto {
                input_id: f.input_id,
                expected_digest: f.expected_digest,
                actual_digest: f.actual_digest,
                structural_delta: f.structural_delta,
            })
            .collect(),
    };
    serde_json::to_string(&LearnReplayRunOutput { result: dto }).map_err(to_napi_error)
}

#[derive(serde::Deserialize)]
struct LearnDriftDetectInput {
    baseline: serde_json::Value,
    current: serde_json::Value,
    tolerance_pct: f64,
}

#[derive(serde::Serialize)]
struct LearnDriftDetectOutput {
    report: corp_finance_core::self_learning::DriftReport,
}

#[napi]
pub fn learn_drift_detect(input_json: String) -> NapiResult<String> {
    let input: LearnDriftDetectInput = serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let report = corp_finance_core::self_learning::detect_drift(
        &input.baseline,
        &input.current,
        input.tolerance_pct,
    );
    serde_json::to_string(&LearnDriftDetectOutput { report }).map_err(to_napi_error)
}

#[derive(serde::Deserialize)]
struct LearnGoldenSetFreezeInput {
    surface: corp_finance_core::surface::Surface,
    surface_event_id: String,
    inputs: Vec<corp_finance_core::self_learning::GoldenInput>,
    output_dir: String,
    /// Directory that holds `signing.key` / `verifying.key`. Per
    /// `RUF-LEARN-007`, the keypair is generated on first run if absent.
    signing_key_path: String,
}

#[derive(serde::Serialize)]
struct LearnGoldenSetFreezeOutput {
    golden_set: corp_finance_core::self_learning::GoldenSet,
}

#[napi]
pub fn learn_golden_set_freeze(input_json: String) -> NapiResult<String> {
    let input: LearnGoldenSetFreezeInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let keys_dir = std::path::PathBuf::from(&input.signing_key_path);
    let (signing_key, _vk) =
        corp_finance_core::self_learning::ensure_keypair(&keys_dir).map_err(to_napi_error)?;
    let output_dir = std::path::PathBuf::from(&input.output_dir);
    let golden_set = corp_finance_core::self_learning::freeze_golden_set(
        input.surface,
        &input.surface_event_id,
        input.inputs,
        &output_dir,
        &signing_key,
    )
    .map_err(to_napi_error)?;
    serde_json::to_string(&LearnGoldenSetFreezeOutput { golden_set }).map_err(to_napi_error)
}

// ---------------------------------------------------------------------------
// Phase 29 Wave 4: trust attestation bindings (ADR-019 v2, RUF-FED-006..009).
//
// Five NAPI bindings covering cross-tenant trust attestation:
//   attest_issue   — issue + persist a signed attestation
//   attest_verify  — load + crypto-verify + revocation-check
//   attest_list    — list all attestations for a subject tenant
//   attest_revoke  — record a revocation tombstone
//   attest_check   — check whether a subject has a granted capability
// ---------------------------------------------------------------------------

/// Open (or create) the SQLite attestation database at `path`, initialising
/// the schema on first use. Creates parent directories as needed.
fn open_attestation_db(path: &str) -> NapiResult<rusqlite::Connection> {
    let pb = std::path::PathBuf::from(path);
    if let Some(parent) = pb.parent() {
        std::fs::create_dir_all(parent).map_err(|e| napi::Error::from_reason(e.to_string()))?;
    }
    let conn =
        rusqlite::Connection::open(&pb).map_err(|e| napi::Error::from_reason(e.to_string()))?;
    corp_finance_core::federation::attestation::init_attestation_schema(&conn)
        .map_err(to_napi_error)?;
    Ok(conn)
}

#[derive(serde::Deserialize)]
struct AttestIssueInput {
    issuer_tenant: String,
    subject_tenant: String,
    capabilities: Vec<String>,
    #[serde(default = "default_valid_for_hours")]
    valid_for_hours: i64,
    db_path: String,
    keys_dir: String,
}

fn default_valid_for_hours() -> i64 {
    24
}

#[napi]
pub fn attest_issue(input_json: String) -> NapiResult<String> {
    let input: AttestIssueInput = serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let keys_dir = std::path::Path::new(&input.keys_dir);
    let (signing_key, _vk) =
        corp_finance_core::self_learning::ensure_keypair(keys_dir).map_err(to_napi_error)?;
    let valid_for = chrono::Duration::hours(input.valid_for_hours);
    let attestation = corp_finance_core::federation::attestation::issue_attestation(
        &input.issuer_tenant,
        &input.subject_tenant,
        input.capabilities,
        valid_for,
        &signing_key,
    )
    .map_err(to_napi_error)?;
    let conn = open_attestation_db(&input.db_path)?;
    corp_finance_core::federation::attestation::save_attestation(&conn, &attestation)
        .map_err(to_napi_error)?;
    serde_json::to_string(&attestation).map_err(to_napi_error)
}

#[derive(serde::Deserialize)]
struct AttestVerifyInput {
    attestation_id: String,
    expected_issuer: String,
    db_path: String,
}

#[derive(serde::Serialize)]
struct AttestVerifyOutput {
    attestation_id: String,
    expected_issuer: String,
    status: corp_finance_core::federation::types::AttestationStatus,
}

#[napi]
pub fn attest_verify(input_json: String) -> NapiResult<String> {
    let input: AttestVerifyInput = serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let id = uuid::Uuid::parse_str(&input.attestation_id).map_err(to_napi_error)?;
    let conn = open_attestation_db(&input.db_path)?;
    let attestation = corp_finance_core::federation::attestation::load_attestation(&conn, id)
        .map_err(to_napi_error)?
        .ok_or_else(|| napi::Error::from_reason(format!("attestation {} not found", id)))?;
    let status = corp_finance_core::federation::attestation::status_with_revocations(
        &conn,
        &attestation,
        &input.expected_issuer,
    )
    .map_err(to_napi_error)?;
    serde_json::to_string(&AttestVerifyOutput {
        attestation_id: input.attestation_id,
        expected_issuer: input.expected_issuer,
        status,
    })
    .map_err(to_napi_error)
}

#[derive(serde::Deserialize)]
struct AttestListInput {
    subject_tenant: String,
    db_path: String,
}

#[derive(serde::Serialize)]
struct AttestListOutput {
    subject_tenant: String,
    count: usize,
    attestations: Vec<corp_finance_core::federation::types::TrustAttestation>,
}

#[napi]
pub fn attest_list(input_json: String) -> NapiResult<String> {
    let input: AttestListInput = serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let conn = open_attestation_db(&input.db_path)?;
    let attestations = corp_finance_core::federation::attestation::list_attestations_by_subject(
        &conn,
        &input.subject_tenant,
    )
    .map_err(to_napi_error)?;
    let count = attestations.len();
    serde_json::to_string(&AttestListOutput {
        subject_tenant: input.subject_tenant,
        count,
        attestations,
    })
    .map_err(to_napi_error)
}

#[derive(serde::Deserialize)]
struct AttestRevokeInput {
    attestation_id: String,
    revoked_by: String,
    reason: String,
    db_path: String,
}

#[derive(serde::Serialize)]
struct AttestRevokeOutput {
    attestation_id: String,
    revoked_by: String,
    reason: String,
    revoked_at: String,
}

#[napi]
pub fn attest_revoke(input_json: String) -> NapiResult<String> {
    let input: AttestRevokeInput = serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let id = uuid::Uuid::parse_str(&input.attestation_id).map_err(to_napi_error)?;
    let conn = open_attestation_db(&input.db_path)?;
    corp_finance_core::federation::attestation::revoke_attestation(
        &conn,
        id,
        &input.revoked_by,
        &input.reason,
    )
    .map_err(to_napi_error)?;
    let revoked_at = chrono::Utc::now().to_rfc3339();
    serde_json::to_string(&AttestRevokeOutput {
        attestation_id: input.attestation_id,
        revoked_by: input.revoked_by,
        reason: input.reason,
        revoked_at,
    })
    .map_err(to_napi_error)
}

#[derive(serde::Deserialize)]
struct AttestCheckInput {
    subject_tenant: String,
    capability: String,
    db_path: String,
}

#[derive(serde::Serialize)]
struct AttestCheckOutput {
    subject_tenant: String,
    capability: String,
    granted: bool,
}

#[napi]
pub fn attest_check(input_json: String) -> NapiResult<String> {
    let input: AttestCheckInput = serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let conn = open_attestation_db(&input.db_path)?;
    let granted = corp_finance_core::federation::attestation::check_capability(
        &conn,
        &input.subject_tenant,
        &input.capability,
    )
    .map_err(to_napi_error)?;
    serde_json::to_string(&AttestCheckOutput {
        subject_tenant: input.subject_tenant,
        capability: input.capability,
        granted,
    })
    .map_err(to_napi_error)
}

// ---------------------------------------------------------------------------
// Office — XLSX writer (Phase 29 Wave 6)
// ---------------------------------------------------------------------------

#[derive(serde::Deserialize)]
struct WriteXlsxWorkbookInput {
    spec: corp_finance_core::office::WorkbookSpec,
    output_path: String,
}

#[napi]
pub fn write_xlsx_workbook(input_json: String) -> NapiResult<String> {
    let input: WriteXlsxWorkbookInput =
        serde_json::from_str(&input_json).map_err(to_napi_error)?;
    let result = corp_finance_core::office::write_workbook(
        &input.spec,
        std::path::Path::new(&input.output_path),
    )
    .map_err(to_napi_error)?;
    serde_json::to_string(&result).map_err(to_napi_error)
}

/// Render a corporate-finance compute result into a WorkbookSpec JSON string.
///
/// Accepts `{ "kind": "dcf"|"comps"|"lbo"|"three_statement", "result_json": "<serialised output>" }`.
/// Returns the WorkbookSpec as a JSON string — the caller should pipe the result
/// directly into `write_xlsx_workbook` to produce a .xlsx file (composable two-step).
#[napi]
pub fn render_office_template(input_json: String) -> NapiResult<String> {
    let workbook =
        corp_finance_core::office::templates::render_template_from_json(&input_json)
            .map_err(to_napi_error)?;
    serde_json::to_string(&workbook).map_err(to_napi_error)
}
