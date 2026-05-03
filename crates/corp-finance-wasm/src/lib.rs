//! WebAssembly bindings for corp-finance-core.
//!
//! GENERATED FROM packages/bindings/src/lib.rs via `cargo run -p cfa-codegen -- wasm-bindings`.
//! Do not edit by hand — re-run the generator if NAPI bindings change.
//!
//! Each exported function takes a JSON string (matching the corresponding
//! `*Input` struct in corp-finance-core) and returns a JSON string with the
//! computed `*Output`. The `wasm_tool!` macro keeps each binding to one line.
//!
//! Each binding is gated by `#[cfg(feature = "<domain>")]` so consumers can
//! build a slim WASM artifact with only the domains they need (see this
//! crate's Cargo.toml for the per-domain features and the `slim` preset).

use wasm_bindgen::prelude::*;

// Hand-maintained streaming bindings live here. These wrap *_with_progress
// variants in corp-finance-core and bridge JS callbacks to ProgressSink. The
// generator does not touch streaming.rs, so adding/removing streaming
// functions does not affect this auto-generated file.
mod streaming;

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

#[cfg(feature = "valuation")]
wasm_tool!(calculate_wacc, corp_finance_core::valuation::wacc::WaccInput, corp_finance_core::valuation::wacc::calculate_wacc);
#[cfg(feature = "valuation")]
wasm_tool!(build_dcf, corp_finance_core::valuation::dcf::DcfInput, corp_finance_core::valuation::dcf::calculate_dcf);
#[cfg(feature = "valuation")]
wasm_tool!(comps_analysis, corp_finance_core::valuation::comps::CompsInput, corp_finance_core::valuation::comps::calculate_comps);

// ---------------------------------------------------------------------------
// Credit
// ---------------------------------------------------------------------------

#[cfg(feature = "credit")]
wasm_tool!(credit_metrics, corp_finance_core::credit::metrics::CreditMetricsInput, corp_finance_core::credit::metrics::calculate_credit_metrics);
#[cfg(feature = "credit")]
wasm_tool!(debt_capacity, corp_finance_core::credit::capacity::DebtCapacityInput, corp_finance_core::credit::capacity::calculate_debt_capacity);
#[cfg(feature = "credit")]
wasm_tool!(covenant_compliance, corp_finance_core::credit::covenants::CovenantTestInput, corp_finance_core::credit::covenants::test_covenants);

// ---------------------------------------------------------------------------
// Private Equity
// ---------------------------------------------------------------------------

#[cfg(feature = "pe")]
wasm_tool!(calculate_returns, corp_finance_core::pe::returns::ReturnsInput, corp_finance_core::pe::returns::calculate_returns);
#[cfg(feature = "pe")]
wasm_tool!(build_debt_schedule, corp_finance_core::pe::debt_schedule::DebtTrancheInput, corp_finance_core::pe::debt_schedule::build_debt_schedule);
#[cfg(feature = "pe")]
wasm_tool!(sources_and_uses, corp_finance_core::pe::sources_uses::SourcesUsesInput, corp_finance_core::pe::sources_uses::build_sources_uses);

// ---------------------------------------------------------------------------
// Private Equity — Phase 2
// ---------------------------------------------------------------------------

#[cfg(feature = "pe")]
wasm_tool!(build_lbo, corp_finance_core::pe::lbo::LboInput, corp_finance_core::pe::lbo::build_lbo);
#[cfg(feature = "pe")]
wasm_tool!(calculate_waterfall, corp_finance_core::pe::waterfall::WaterfallInput, corp_finance_core::pe::waterfall::calculate_waterfall);

// ---------------------------------------------------------------------------
// M&A
// ---------------------------------------------------------------------------

#[cfg(feature = "ma")]
wasm_tool!(analyze_merger, corp_finance_core::ma::merger_model::MergerInput, corp_finance_core::ma::merger_model::analyze_merger);

// ---------------------------------------------------------------------------
// Credit — Phase 2
// ---------------------------------------------------------------------------

#[cfg(feature = "credit")]
wasm_tool!(altman_zscore, corp_finance_core::credit::altman::AltmanInput, corp_finance_core::credit::altman::calculate_altman_zscore);

// ---------------------------------------------------------------------------
// Jurisdiction / Fund
// ---------------------------------------------------------------------------

#[cfg(feature = "jurisdiction")]
wasm_tool!(calculate_fund_fees, corp_finance_core::jurisdiction::fund_fees::FundFeeInput, corp_finance_core::jurisdiction::fund_fees::calculate_fund_fees);
#[cfg(feature = "jurisdiction")]
wasm_tool!(reconcile_accounting, corp_finance_core::jurisdiction::reconciliation::ReconciliationInput, corp_finance_core::jurisdiction::reconciliation::reconcile_accounting_standards);
#[cfg(feature = "jurisdiction")]
wasm_tool!(calculate_wht, corp_finance_core::jurisdiction::withholding_tax::WhtInput, corp_finance_core::jurisdiction::withholding_tax::calculate_withholding_tax);
#[cfg(feature = "jurisdiction")]
wasm_tool!(calculate_portfolio_wht, corp_finance_core::jurisdiction::withholding_tax::PortfolioWhtInput, corp_finance_core::jurisdiction::withholding_tax::calculate_portfolio_wht);
#[cfg(feature = "jurisdiction")]
wasm_tool!(calculate_nav, corp_finance_core::jurisdiction::nav::NavInput, corp_finance_core::jurisdiction::nav::calculate_nav);
#[cfg(feature = "jurisdiction")]
wasm_tool!(calculate_gp_economics, corp_finance_core::jurisdiction::gp_economics::GpEconomicsInput, corp_finance_core::jurisdiction::gp_economics::calculate_gp_economics);
#[cfg(feature = "jurisdiction")]
wasm_tool!(calculate_investor_net_returns, corp_finance_core::jurisdiction::investor_returns::InvestorNetReturnsInput, corp_finance_core::jurisdiction::investor_returns::calculate_investor_net_returns);
#[cfg(feature = "jurisdiction")]
wasm_tool!(screen_ubti_eci, corp_finance_core::jurisdiction::ubti::UbtiScreeningInput, corp_finance_core::jurisdiction::ubti::screen_ubti_eci);

// ---------------------------------------------------------------------------
// Fixed Income
// ---------------------------------------------------------------------------

#[cfg(feature = "fixed_income")]
wasm_tool!(price_bond, corp_finance_core::fixed_income::bonds::BondPricingInput, corp_finance_core::fixed_income::bonds::price_bond);
#[cfg(feature = "fixed_income")]
wasm_tool!(calculate_bond_yield, corp_finance_core::fixed_income::yields::BondYieldInput, corp_finance_core::fixed_income::yields::calculate_bond_yield);
#[cfg(feature = "fixed_income")]
wasm_tool!(bootstrap_spot_curve, corp_finance_core::fixed_income::yields::BootstrapInput, corp_finance_core::fixed_income::yields::bootstrap_spot_curve);
#[cfg(feature = "fixed_income")]
wasm_tool!(fit_nelson_siegel, corp_finance_core::fixed_income::yields::NelsonSiegelInput, corp_finance_core::fixed_income::yields::fit_nelson_siegel);
#[cfg(feature = "fixed_income")]
wasm_tool!(calculate_duration, corp_finance_core::fixed_income::duration::DurationInput, corp_finance_core::fixed_income::duration::calculate_duration);
#[cfg(feature = "fixed_income")]
wasm_tool!(calculate_credit_spreads, corp_finance_core::fixed_income::spreads::CreditSpreadInput, corp_finance_core::fixed_income::spreads::calculate_credit_spreads);

// ---------------------------------------------------------------------------
// Derivatives
// ---------------------------------------------------------------------------

#[cfg(feature = "derivatives")]
wasm_tool!(price_option, corp_finance_core::derivatives::options::OptionInput, corp_finance_core::derivatives::options::price_option);
#[cfg(feature = "derivatives")]
wasm_tool!(implied_volatility, corp_finance_core::derivatives::options::ImpliedVolInput, corp_finance_core::derivatives::options::implied_volatility);
#[cfg(feature = "derivatives")]
wasm_tool!(price_forward, corp_finance_core::derivatives::forwards::ForwardInput, corp_finance_core::derivatives::forwards::price_forward);
#[cfg(feature = "derivatives")]
wasm_tool!(value_forward_position, corp_finance_core::derivatives::forwards::ForwardPositionInput, corp_finance_core::derivatives::forwards::value_forward_position);
#[cfg(feature = "derivatives")]
wasm_tool!(futures_basis_analysis, corp_finance_core::derivatives::forwards::BasisAnalysisInput, corp_finance_core::derivatives::forwards::futures_basis_analysis);
#[cfg(feature = "derivatives")]
wasm_tool!(value_interest_rate_swap, corp_finance_core::derivatives::swaps::IrsInput, corp_finance_core::derivatives::swaps::value_interest_rate_swap);
#[cfg(feature = "derivatives")]
wasm_tool!(value_currency_swap, corp_finance_core::derivatives::swaps::CurrencySwapInput, corp_finance_core::derivatives::swaps::value_currency_swap);
#[cfg(feature = "derivatives")]
wasm_tool!(analyze_strategy, corp_finance_core::derivatives::strategies::StrategyInput, corp_finance_core::derivatives::strategies::analyze_strategy);

// ---------------------------------------------------------------------------
// Portfolio
// ---------------------------------------------------------------------------

#[cfg(feature = "portfolio")]
wasm_tool!(risk_adjusted_returns, corp_finance_core::portfolio::returns::RiskAdjustedInput, corp_finance_core::portfolio::returns::calculate_risk_adjusted_returns);
#[cfg(feature = "portfolio")]
wasm_tool!(risk_metrics, corp_finance_core::portfolio::risk::RiskMetricsInput, corp_finance_core::portfolio::risk::calculate_risk_metrics);
#[cfg(feature = "portfolio")]
wasm_tool!(kelly_sizing, corp_finance_core::portfolio::sizing::KellyInput, corp_finance_core::portfolio::sizing::calculate_kelly);

// ---------------------------------------------------------------------------
// Scenarios
// ---------------------------------------------------------------------------

#[cfg(feature = "scenarios")]
wasm_tool!(build_sensitivity_grid, corp_finance_core::scenarios::sensitivity::SensitivityInput, corp_finance_core::scenarios::sensitivity::build_sensitivity_grid);

// ---------------------------------------------------------------------------
// Three-Statement Model
// ---------------------------------------------------------------------------

#[cfg(feature = "three_statement")]
wasm_tool!(build_three_statement, corp_finance_core::three_statement::model::ThreeStatementInput, corp_finance_core::three_statement::model::build_three_statement_model);

// ---------------------------------------------------------------------------
// Monte Carlo
// ---------------------------------------------------------------------------

#[cfg(feature = "monte_carlo")]
wasm_tool!(run_monte_carlo, corp_finance_core::monte_carlo::simulation::MonteCarloInput, corp_finance_core::monte_carlo::simulation::run_monte_carlo_simulation);
#[cfg(feature = "monte_carlo")]
wasm_tool!(run_mc_dcf, corp_finance_core::monte_carlo::simulation::McDcfInput, corp_finance_core::monte_carlo::simulation::run_monte_carlo_dcf);

// ---------------------------------------------------------------------------
// Quant Risk
// ---------------------------------------------------------------------------

#[cfg(feature = "quant_risk")]
wasm_tool!(run_factor_model, corp_finance_core::quant_risk::factor_models::FactorModelInput, corp_finance_core::quant_risk::factor_models::run_factor_model);
#[cfg(feature = "quant_risk")]
wasm_tool!(run_black_litterman, corp_finance_core::quant_risk::black_litterman::BlackLittermanInput, corp_finance_core::quant_risk::black_litterman::run_black_litterman);
#[cfg(feature = "quant_risk")]
wasm_tool!(calculate_risk_parity, corp_finance_core::quant_risk::risk_parity::RiskParityInput, corp_finance_core::quant_risk::risk_parity::calculate_risk_parity);
#[cfg(feature = "quant_risk")]
wasm_tool!(run_stress_test, corp_finance_core::quant_risk::stress_testing::StressTestInput, corp_finance_core::quant_risk::stress_testing::run_stress_test);

// ---------------------------------------------------------------------------
// Restructuring
// ---------------------------------------------------------------------------

#[cfg(feature = "restructuring")]
wasm_tool!(analyze_recovery, corp_finance_core::restructuring::recovery::RecoveryAnalysisInput, corp_finance_core::restructuring::recovery::analyze_recovery);
#[cfg(feature = "restructuring")]
wasm_tool!(analyze_distressed_debt, corp_finance_core::restructuring::distressed_debt::DistressedDebtInput, corp_finance_core::restructuring::distressed_debt::analyze_distressed_debt);

// ---------------------------------------------------------------------------
// Real Assets
// ---------------------------------------------------------------------------

#[cfg(feature = "real_assets")]
wasm_tool!(value_property, corp_finance_core::real_assets::real_estate::PropertyValuationInput, corp_finance_core::real_assets::real_estate::value_property);
#[cfg(feature = "real_assets")]
wasm_tool!(model_project_finance, corp_finance_core::real_assets::project_finance::ProjectFinanceInput, corp_finance_core::real_assets::project_finance::model_project_finance);

// ---------------------------------------------------------------------------
// Institutional Real Estate
// ---------------------------------------------------------------------------

#[cfg(feature = "institutional_real_estate")]
wasm_tool!(tenant_schedule, corp_finance_core::institutional_real_estate::rent_roll::TenantScheduleInput, corp_finance_core::institutional_real_estate::rent_roll::tenant_schedule);
#[cfg(feature = "institutional_real_estate")]
wasm_tool!(lease_rollover, corp_finance_core::institutional_real_estate::rent_roll::LeaseRolloverInput, corp_finance_core::institutional_real_estate::rent_roll::lease_rollover);
#[cfg(feature = "institutional_real_estate")]
wasm_tool!(comp_adjustment_grid, corp_finance_core::institutional_real_estate::comparable_sales::CompAdjustmentInput, corp_finance_core::institutional_real_estate::comparable_sales::comp_adjustment_grid);
#[cfg(feature = "institutional_real_estate")]
wasm_tool!(comp_reconciliation, corp_finance_core::institutional_real_estate::comparable_sales::ReconciliationInput, corp_finance_core::institutional_real_estate::comparable_sales::reconciliation);
#[cfg(feature = "institutional_real_estate")]
wasm_tool!(hbu_analysis, corp_finance_core::institutional_real_estate::highest_best_use::HbuAnalysisInput, corp_finance_core::institutional_real_estate::highest_best_use::hbu_analysis);
#[cfg(feature = "institutional_real_estate")]
wasm_tool!(financially_feasible, corp_finance_core::institutional_real_estate::highest_best_use::FinanciallyFeasibleInput, corp_finance_core::institutional_real_estate::highest_best_use::financially_feasible);
#[cfg(feature = "institutional_real_estate")]
wasm_tool!(cost_approach, corp_finance_core::institutional_real_estate::replacement_cost::CostApproachInput, corp_finance_core::institutional_real_estate::replacement_cost::cost_approach);
#[cfg(feature = "institutional_real_estate")]
wasm_tool!(marshall_swift, corp_finance_core::institutional_real_estate::replacement_cost::MarshallSwiftInput, corp_finance_core::institutional_real_estate::replacement_cost::marshall_swift);
#[cfg(feature = "institutional_real_estate")]
wasm_tool!(ncreif_attribution, corp_finance_core::institutional_real_estate::benchmark::NcreifAttributionInput, corp_finance_core::institutional_real_estate::benchmark::ncreif_attribution);
#[cfg(feature = "institutional_real_estate")]
wasm_tool!(odce_comparison, corp_finance_core::institutional_real_estate::benchmark::OdceComparisonInput, corp_finance_core::institutional_real_estate::benchmark::odce_comparison);
#[cfg(feature = "institutional_real_estate")]
wasm_tool!(acquisition_model, corp_finance_core::institutional_real_estate::acquisition::AcquisitionModelInput, corp_finance_core::institutional_real_estate::acquisition::acquisition_model);
#[cfg(feature = "institutional_real_estate")]
wasm_tool!(development_feasibility, corp_finance_core::institutional_real_estate::acquisition::DevelopmentFeasibilityInput, corp_finance_core::institutional_real_estate::acquisition::development_feasibility);

// ---------------------------------------------------------------------------
// FX & Commodities
// ---------------------------------------------------------------------------

#[cfg(feature = "fx_commodities")]
wasm_tool!(price_fx_forward, corp_finance_core::fx_commodities::fx::FxForwardInput, corp_finance_core::fx_commodities::fx::price_fx_forward);
#[cfg(feature = "fx_commodities")]
wasm_tool!(calculate_cross_rate, corp_finance_core::fx_commodities::fx::CrossRateInput, corp_finance_core::fx_commodities::fx::calculate_cross_rate);
#[cfg(feature = "fx_commodities")]
wasm_tool!(price_commodity_forward, corp_finance_core::fx_commodities::commodities::CommodityForwardInput, corp_finance_core::fx_commodities::commodities::price_commodity_forward);
#[cfg(feature = "fx_commodities")]
wasm_tool!(analyze_commodity_curve, corp_finance_core::fx_commodities::commodities::CommodityCurveInput, corp_finance_core::fx_commodities::commodities::analyze_commodity_curve);

// ---------------------------------------------------------------------------
// Scenarios
// ---------------------------------------------------------------------------


// ---------------------------------------------------------------------------
// Securitization
// ---------------------------------------------------------------------------

#[cfg(feature = "securitization")]
wasm_tool!(model_abs_cashflows, corp_finance_core::securitization::abs_mbs::AbsMbsInput, corp_finance_core::securitization::abs_mbs::model_abs_cashflows);
#[cfg(feature = "securitization")]
wasm_tool!(analyze_tranching, corp_finance_core::securitization::tranching::TranchingInput, corp_finance_core::securitization::tranching::analyze_tranching);

// ---------------------------------------------------------------------------
// Venture Capital
// ---------------------------------------------------------------------------

#[cfg(feature = "venture")]
wasm_tool!(model_funding_round, corp_finance_core::venture::valuation::FundingRoundInput, corp_finance_core::venture::valuation::model_funding_round);
#[cfg(feature = "venture")]
wasm_tool!(analyze_dilution, corp_finance_core::venture::valuation::DilutionInput, corp_finance_core::venture::valuation::analyze_dilution);
#[cfg(feature = "venture")]
wasm_tool!(convert_note, corp_finance_core::venture::instruments::ConvertibleNoteInput, corp_finance_core::venture::instruments::convert_note);
#[cfg(feature = "venture")]
wasm_tool!(convert_safe, corp_finance_core::venture::instruments::SafeInput, corp_finance_core::venture::instruments::convert_safe);
#[cfg(feature = "venture")]
wasm_tool!(model_venture_fund, corp_finance_core::venture::returns::VentureFundInput, corp_finance_core::venture::returns::model_venture_fund);

// ---------------------------------------------------------------------------
// ESG
// ---------------------------------------------------------------------------

#[cfg(feature = "esg")]
wasm_tool!(calculate_esg_score, corp_finance_core::esg::scoring::EsgScoreInput, corp_finance_core::esg::scoring::calculate_esg_score);
#[cfg(feature = "esg")]
wasm_tool!(analyze_carbon_footprint, corp_finance_core::esg::climate::CarbonFootprintInput, corp_finance_core::esg::climate::analyze_carbon_footprint);
#[cfg(feature = "esg")]
wasm_tool!(analyze_green_bond, corp_finance_core::esg::climate::GreenBondInput, corp_finance_core::esg::climate::analyze_green_bond);
#[cfg(feature = "esg")]
wasm_tool!(test_sll_covenants, corp_finance_core::esg::climate::SllInput, corp_finance_core::esg::climate::test_sll_covenants);

// ---------------------------------------------------------------------------
// Regulatory
// ---------------------------------------------------------------------------

#[cfg(feature = "regulatory")]
wasm_tool!(calculate_regulatory_capital, corp_finance_core::regulatory::capital::RegulatoryCapitalInput, corp_finance_core::regulatory::capital::calculate_regulatory_capital);
#[cfg(feature = "regulatory")]
wasm_tool!(calculate_lcr, corp_finance_core::regulatory::liquidity::LcrInput, corp_finance_core::regulatory::liquidity::calculate_lcr);
#[cfg(feature = "regulatory")]
wasm_tool!(calculate_nsfr, corp_finance_core::regulatory::liquidity::NsfrInput, corp_finance_core::regulatory::liquidity::calculate_nsfr);
#[cfg(feature = "regulatory")]
wasm_tool!(analyze_alm, corp_finance_core::regulatory::alm::AlmInput, corp_finance_core::regulatory::alm::analyze_alm);

// ---------------------------------------------------------------------------
// Private Credit
// ---------------------------------------------------------------------------

#[cfg(feature = "private_credit")]
wasm_tool!(price_unitranche, corp_finance_core::private_credit::unitranche::UnitrancheInput, corp_finance_core::private_credit::unitranche::price_unitranche);
#[cfg(feature = "private_credit")]
wasm_tool!(model_direct_loan, corp_finance_core::private_credit::direct_lending::DirectLoanInput, corp_finance_core::private_credit::direct_lending::model_direct_loan);
#[cfg(feature = "private_credit")]
wasm_tool!(analyze_syndication, corp_finance_core::private_credit::direct_lending::SyndicationInput, corp_finance_core::private_credit::direct_lending::analyze_syndication);

// ---------------------------------------------------------------------------
// Insurance
// ---------------------------------------------------------------------------

#[cfg(feature = "insurance")]
wasm_tool!(estimate_reserves, corp_finance_core::insurance::reserving::ReservingInput, corp_finance_core::insurance::reserving::estimate_reserves);
#[cfg(feature = "insurance")]
wasm_tool!(price_premium, corp_finance_core::insurance::pricing::PremiumPricingInput, corp_finance_core::insurance::pricing::price_premium);
#[cfg(feature = "insurance")]
wasm_tool!(analyze_combined_ratio, corp_finance_core::insurance::pricing::CombinedRatioInput, corp_finance_core::insurance::pricing::analyze_combined_ratio);
#[cfg(feature = "insurance")]
wasm_tool!(calculate_scr, corp_finance_core::insurance::pricing::ScrInput, corp_finance_core::insurance::pricing::calculate_scr);

// ---------------------------------------------------------------------------
// FP&A (Financial Planning & Analysis)
// ---------------------------------------------------------------------------

#[cfg(feature = "fpa")]
wasm_tool!(analyze_variance, corp_finance_core::fpa::variance::VarianceInput, corp_finance_core::fpa::variance::analyze_variance);
#[cfg(feature = "fpa")]
wasm_tool!(analyze_breakeven, corp_finance_core::fpa::variance::BreakevenInput, corp_finance_core::fpa::variance::analyze_breakeven);
#[cfg(feature = "fpa")]
wasm_tool!(analyze_working_capital, corp_finance_core::fpa::working_capital::WorkingCapitalInput, corp_finance_core::fpa::working_capital::analyze_working_capital);
#[cfg(feature = "fpa")]
wasm_tool!(build_rolling_forecast, corp_finance_core::fpa::working_capital::RollingForecastInput, corp_finance_core::fpa::working_capital::build_rolling_forecast);

// ---------------------------------------------------------------------------
// Wealth Management
// ---------------------------------------------------------------------------

#[cfg(feature = "wealth")]
wasm_tool!(plan_retirement, corp_finance_core::wealth::retirement::RetirementInput, corp_finance_core::wealth::retirement::plan_retirement);
#[cfg(feature = "wealth")]
wasm_tool!(simulate_tax_loss_harvesting, corp_finance_core::wealth::tax_estate::TlhInput, corp_finance_core::wealth::tax_estate::simulate_tax_loss_harvesting);
#[cfg(feature = "wealth")]
wasm_tool!(plan_estate, corp_finance_core::wealth::tax_estate::EstatePlanInput, corp_finance_core::wealth::tax_estate::plan_estate);

// ---------------------------------------------------------------------------
// Crypto / Digital Assets — Phase 8
// ---------------------------------------------------------------------------

#[cfg(feature = "crypto")]
wasm_tool!(value_token, corp_finance_core::crypto::valuation::TokenValuationInput, corp_finance_core::crypto::valuation::value_token);
#[cfg(feature = "crypto")]
wasm_tool!(analyze_defi, corp_finance_core::crypto::defi::DefiYieldInput, corp_finance_core::crypto::defi::analyze_defi);

// ---------------------------------------------------------------------------
// Municipal Bonds — Phase 8
// ---------------------------------------------------------------------------

#[cfg(feature = "municipal")]
wasm_tool!(price_muni_bond, corp_finance_core::municipal::bonds::MuniBondInput, corp_finance_core::municipal::bonds::price_muni_bond);
#[cfg(feature = "municipal")]
wasm_tool!(analyze_municipal, corp_finance_core::municipal::analysis::MuniAnalysisInput, corp_finance_core::municipal::analysis::analyze_municipal);

// ---------------------------------------------------------------------------
// Structured Products — Phase 8
// ---------------------------------------------------------------------------

#[cfg(feature = "structured_products")]
wasm_tool!(price_structured_note, corp_finance_core::structured_products::notes::StructuredNoteInput, corp_finance_core::structured_products::notes::price_structured_note);
#[cfg(feature = "structured_products")]
wasm_tool!(price_exotic, corp_finance_core::structured_products::exotic::ExoticProductInput, corp_finance_core::structured_products::exotic::price_exotic);

// ---------------------------------------------------------------------------
// Trade Finance — Phase 8
// ---------------------------------------------------------------------------

#[cfg(feature = "trade_finance")]
wasm_tool!(price_letter_of_credit, corp_finance_core::trade_finance::letter_of_credit::LetterOfCreditInput, corp_finance_core::trade_finance::letter_of_credit::price_letter_of_credit);
#[cfg(feature = "trade_finance")]
wasm_tool!(analyze_supply_chain_finance, corp_finance_core::trade_finance::supply_chain::SupplyChainFinanceInput, corp_finance_core::trade_finance::supply_chain::analyze_supply_chain_finance);

// ---------------------------------------------------------------------------
// Credit Derivatives — Phase 9
// ---------------------------------------------------------------------------

#[cfg(feature = "credit_derivatives")]
wasm_tool!(price_cds, corp_finance_core::credit_derivatives::cds::CdsInput, corp_finance_core::credit_derivatives::cds::price_cds);
#[cfg(feature = "credit_derivatives")]
wasm_tool!(calculate_cva, corp_finance_core::credit_derivatives::cva::CvaInput, corp_finance_core::credit_derivatives::cva::calculate_cva);

// ---------------------------------------------------------------------------
// Convertible Bonds — Phase 9
// ---------------------------------------------------------------------------

#[cfg(feature = "convertibles")]
wasm_tool!(price_convertible, corp_finance_core::convertibles::pricing::ConvertibleBondInput, corp_finance_core::convertibles::pricing::price_convertible);
#[cfg(feature = "convertibles")]
wasm_tool!(analyze_convertible, corp_finance_core::convertibles::analysis::ConvertibleAnalysisInput, corp_finance_core::convertibles::analysis::analyze_convertible);

// ---------------------------------------------------------------------------
// Lease Accounting — Phase 9
// ---------------------------------------------------------------------------

#[cfg(feature = "lease_accounting")]
wasm_tool!(classify_lease, corp_finance_core::lease_accounting::classification::LeaseInput, corp_finance_core::lease_accounting::classification::classify_lease);
#[cfg(feature = "lease_accounting")]
wasm_tool!(analyze_sale_leaseback, corp_finance_core::lease_accounting::sale_leaseback::SaleLeasebackInput, corp_finance_core::lease_accounting::sale_leaseback::analyze_sale_leaseback);

// ---------------------------------------------------------------------------
// Pension & LDI — Phase 9
// ---------------------------------------------------------------------------

#[cfg(feature = "pension")]
wasm_tool!(analyze_pension_funding, corp_finance_core::pension::funding::PensionFundingInput, corp_finance_core::pension::funding::analyze_pension_funding);
#[cfg(feature = "pension")]
wasm_tool!(design_ldi_strategy, corp_finance_core::pension::ldi::LdiInput, corp_finance_core::pension::ldi::design_ldi_strategy);

// ---------------------------------------------------------------------------
// Sovereign — Phase 10
// ---------------------------------------------------------------------------

#[cfg(feature = "sovereign")]
wasm_tool!(analyze_sovereign_bond, corp_finance_core::sovereign::sovereign_bonds::SovereignBondInput, corp_finance_core::sovereign::sovereign_bonds::analyze_sovereign_bond);
#[cfg(feature = "sovereign")]
wasm_tool!(assess_country_risk, corp_finance_core::sovereign::country_risk::CountryRiskInput, corp_finance_core::sovereign::country_risk::assess_country_risk);

// ---------------------------------------------------------------------------
// Real Options — Phase 10
// ---------------------------------------------------------------------------

#[cfg(feature = "real_options")]
wasm_tool!(value_real_option, corp_finance_core::real_options::valuation::RealOptionInput, corp_finance_core::real_options::valuation::value_real_option);
#[cfg(feature = "real_options")]
wasm_tool!(analyze_decision_tree, corp_finance_core::real_options::decision_tree::DecisionTreeInput, corp_finance_core::real_options::decision_tree::analyze_decision_tree);

// ---------------------------------------------------------------------------
// Equity Research — Phase 10
// ---------------------------------------------------------------------------

#[cfg(feature = "equity_research")]
wasm_tool!(calculate_sotp, corp_finance_core::equity_research::sotp::SotpInput, corp_finance_core::equity_research::sotp::calculate_sotp);
#[cfg(feature = "equity_research")]
wasm_tool!(calculate_target_price, corp_finance_core::equity_research::target_price::TargetPriceInput, corp_finance_core::equity_research::target_price::calculate_target_price);

// ---------------------------------------------------------------------------
// Commodity Trading — Phase 10
// ---------------------------------------------------------------------------

#[cfg(feature = "commodity_trading")]
wasm_tool!(analyze_commodity_spread, corp_finance_core::commodity_trading::spreads::CommoditySpreadInput, corp_finance_core::commodity_trading::spreads::analyze_commodity_spread);
#[cfg(feature = "commodity_trading")]
wasm_tool!(analyze_storage_economics, corp_finance_core::commodity_trading::storage::StorageEconomicsInput, corp_finance_core::commodity_trading::storage::analyze_storage_economics);

// ---------------------------------------------------------------------------
// Quant Strategies — Phase 11
// ---------------------------------------------------------------------------

#[cfg(feature = "quant_strategies")]
wasm_tool!(analyze_pairs_trading, corp_finance_core::quant_strategies::pairs_trading::PairsTradingInput, corp_finance_core::quant_strategies::pairs_trading::analyze_pairs_trading);
#[cfg(feature = "quant_strategies")]
wasm_tool!(analyze_momentum, corp_finance_core::quant_strategies::momentum::MomentumInput, corp_finance_core::quant_strategies::momentum::analyze_momentum);

// ---------------------------------------------------------------------------
// Treasury — Phase 11
// ---------------------------------------------------------------------------

#[cfg(feature = "treasury")]
wasm_tool!(analyze_cash_management, corp_finance_core::treasury::cash_management::CashManagementInput, corp_finance_core::treasury::cash_management::analyze_cash_management);
#[cfg(feature = "treasury")]
wasm_tool!(analyze_hedging, corp_finance_core::treasury::hedging::HedgingInput, corp_finance_core::treasury::hedging::analyze_hedging);

// ---------------------------------------------------------------------------
// Infrastructure — Phase 11
// ---------------------------------------------------------------------------

#[cfg(feature = "infrastructure")]
wasm_tool!(model_ppp, corp_finance_core::infrastructure::ppp_model::PppModelInput, corp_finance_core::infrastructure::ppp_model::model_ppp);
#[cfg(feature = "infrastructure")]
wasm_tool!(value_concession, corp_finance_core::infrastructure::concession::ConcessionInput, corp_finance_core::infrastructure::concession::value_concession);

// ---------------------------------------------------------------------------
// Behavioral Finance — Phase 11
// ---------------------------------------------------------------------------

#[cfg(feature = "behavioral")]
wasm_tool!(analyze_prospect_theory, corp_finance_core::behavioral::prospect_theory::ProspectTheoryInput, corp_finance_core::behavioral::prospect_theory::analyze_prospect_theory);
#[cfg(feature = "behavioral")]
wasm_tool!(analyze_sentiment, corp_finance_core::behavioral::sentiment::SentimentInput, corp_finance_core::behavioral::sentiment::analyze_sentiment);

// ---------------------------------------------------------------------------
// Performance Attribution — Phase 12
// ---------------------------------------------------------------------------

#[cfg(feature = "performance_attribution")]
wasm_tool!(brinson_attribution, corp_finance_core::performance_attribution::brinson::BrinsonInput, corp_finance_core::performance_attribution::brinson::brinson_attribution);
#[cfg(feature = "performance_attribution")]
wasm_tool!(factor_attribution, corp_finance_core::performance_attribution::factor_attribution::FactorAttributionInput, corp_finance_core::performance_attribution::factor_attribution::factor_attribution);

// ---------------------------------------------------------------------------
// Credit Portfolio — Phase 12
// ---------------------------------------------------------------------------

#[cfg(feature = "credit_portfolio")]
wasm_tool!(calculate_portfolio_credit_risk, corp_finance_core::credit_portfolio::portfolio_risk::PortfolioRiskInput, corp_finance_core::credit_portfolio::portfolio_risk::calculate_portfolio_risk);
#[cfg(feature = "credit_portfolio")]
wasm_tool!(calculate_migration, corp_finance_core::credit_portfolio::migration::MigrationInput, corp_finance_core::credit_portfolio::migration::calculate_migration);

// ---------------------------------------------------------------------------
// Macro Economics — Phase 12
// ---------------------------------------------------------------------------

#[cfg(feature = "macro_economics")]
wasm_tool!(analyze_monetary_policy, corp_finance_core::macro_economics::monetary_policy::MonetaryPolicyInput, corp_finance_core::macro_economics::monetary_policy::analyze_monetary_policy);
#[cfg(feature = "macro_economics")]
wasm_tool!(analyze_international, corp_finance_core::macro_economics::international::InternationalInput, corp_finance_core::macro_economics::international::analyze_international);

// ---------------------------------------------------------------------------
// Compliance — Phase 12
// ---------------------------------------------------------------------------

#[cfg(feature = "compliance")]
wasm_tool!(analyze_best_execution, corp_finance_core::compliance::best_execution::BestExecutionInput, corp_finance_core::compliance::best_execution::analyze_best_execution);
#[cfg(feature = "compliance")]
wasm_tool!(generate_gips_report, corp_finance_core::compliance::reporting::GipsInput, corp_finance_core::compliance::reporting::generate_gips_report);

// ---------------------------------------------------------------------------
// Onshore Structures — Phase 13
// ---------------------------------------------------------------------------

#[cfg(feature = "onshore_structures")]
wasm_tool!(analyze_us_fund_structure, corp_finance_core::onshore_structures::us_funds::UsFundInput, corp_finance_core::onshore_structures::us_funds::analyze_us_fund_structure);
#[cfg(feature = "onshore_structures")]
wasm_tool!(analyze_uk_eu_fund, corp_finance_core::onshore_structures::uk_eu_funds::UkEuFundInput, corp_finance_core::onshore_structures::uk_eu_funds::analyze_uk_eu_fund);

// ---------------------------------------------------------------------------
// Offshore Structures — Phase 13
// ---------------------------------------------------------------------------

#[cfg(feature = "offshore_structures")]
wasm_tool!(analyze_cayman_structure, corp_finance_core::offshore_structures::cayman::CaymanFundInput, corp_finance_core::offshore_structures::cayman::analyze_cayman_structure);
#[cfg(feature = "offshore_structures")]
wasm_tool!(analyze_lux_structure, corp_finance_core::offshore_structures::luxembourg::LuxFundInput, corp_finance_core::offshore_structures::luxembourg::analyze_lux_structure);

// ---------------------------------------------------------------------------
// Offshore Structures Expansion — Phase 23
// ---------------------------------------------------------------------------

#[cfg(feature = "offshore_structures")]
wasm_tool!(analyze_jersey_fund, corp_finance_core::offshore_structures::channel_islands::JerseyFundInput, corp_finance_core::offshore_structures::channel_islands::analyze_jersey_fund);
#[cfg(feature = "offshore_structures")]
wasm_tool!(analyze_guernsey_fund, corp_finance_core::offshore_structures::channel_islands::GuernseyFundInput, corp_finance_core::offshore_structures::channel_islands::analyze_guernsey_fund);
#[cfg(feature = "offshore_structures")]
wasm_tool!(cell_company_analysis, corp_finance_core::offshore_structures::channel_islands::CellCompanyInput, corp_finance_core::offshore_structures::channel_islands::cell_company_analysis);
#[cfg(feature = "offshore_structures")]
wasm_tool!(analyze_vcc_structure, corp_finance_core::offshore_structures::singapore_vcc::VccInput, corp_finance_core::offshore_structures::singapore_vcc::analyze_vcc_structure);
#[cfg(feature = "offshore_structures")]
wasm_tool!(vcc_tax_incentive_analysis, corp_finance_core::offshore_structures::singapore_vcc::TaxIncentiveInput, corp_finance_core::offshore_structures::singapore_vcc::tax_incentive_analysis);
#[cfg(feature = "offshore_structures")]
wasm_tool!(analyze_ofc_structure, corp_finance_core::offshore_structures::hong_kong_funds::OfcInput, corp_finance_core::offshore_structures::hong_kong_funds::analyze_ofc_structure);
#[cfg(feature = "offshore_structures")]
wasm_tool!(analyze_lpf_structure, corp_finance_core::offshore_structures::hong_kong_funds::LpfInput, corp_finance_core::offshore_structures::hong_kong_funds::analyze_lpf_structure);
#[cfg(feature = "offshore_structures")]
wasm_tool!(hk_carried_interest_concession, corp_finance_core::offshore_structures::hong_kong_funds::CarriedInterestInput, corp_finance_core::offshore_structures::hong_kong_funds::carried_interest_concession);
#[cfg(feature = "offshore_structures")]
wasm_tool!(analyze_difc_fund, corp_finance_core::offshore_structures::middle_east_funds::DifcFundInput, corp_finance_core::offshore_structures::middle_east_funds::analyze_difc_fund);
#[cfg(feature = "offshore_structures")]
wasm_tool!(analyze_adgm_fund, corp_finance_core::offshore_structures::middle_east_funds::AdgmFundInput, corp_finance_core::offshore_structures::middle_east_funds::analyze_adgm_fund);
#[cfg(feature = "offshore_structures")]
wasm_tool!(compare_jurisdictions, corp_finance_core::offshore_structures::jurisdiction_comparison::JurisdictionComparisonInput, corp_finance_core::offshore_structures::jurisdiction_comparison::compare_jurisdictions);
#[cfg(feature = "offshore_structures")]
wasm_tool!(migration_feasibility, corp_finance_core::offshore_structures::fund_migration::MigrationFeasibilityInput, corp_finance_core::offshore_structures::fund_migration::migration_feasibility);

// ---------------------------------------------------------------------------
// Transfer Pricing — Phase 13
// ---------------------------------------------------------------------------

#[cfg(feature = "transfer_pricing")]
wasm_tool!(analyze_beps_compliance, corp_finance_core::transfer_pricing::beps::BepsInput, corp_finance_core::transfer_pricing::beps::analyze_beps_compliance);
#[cfg(feature = "transfer_pricing")]
wasm_tool!(analyze_intercompany, corp_finance_core::transfer_pricing::intercompany::IntercompanyInput, corp_finance_core::transfer_pricing::intercompany::analyze_intercompany);

// ---------------------------------------------------------------------------
// Tax Treaty — Phase 13
// ---------------------------------------------------------------------------

#[cfg(feature = "tax_treaty")]
wasm_tool!(analyze_treaty_network, corp_finance_core::tax_treaty::treaty_network::TreatyNetworkInput, corp_finance_core::tax_treaty::treaty_network::analyze_treaty_network);
#[cfg(feature = "tax_treaty")]
wasm_tool!(optimize_treaty_structure, corp_finance_core::tax_treaty::optimization::TreatyOptInput, corp_finance_core::tax_treaty::optimization::optimize_treaty_structure);

// ---------------------------------------------------------------------------
// FATCA/CRS — Phase 14
// ---------------------------------------------------------------------------

#[cfg(feature = "fatca_crs")]
wasm_tool!(analyze_fatca_crs_reporting, corp_finance_core::fatca_crs::reporting::FatcaCrsReportingInput, corp_finance_core::fatca_crs::reporting::analyze_fatca_crs_reporting);
#[cfg(feature = "fatca_crs")]
wasm_tool!(classify_entity, corp_finance_core::fatca_crs::classification::EntityClassificationInput, corp_finance_core::fatca_crs::classification::classify_entity);

// ---------------------------------------------------------------------------
// Substance Requirements — Phase 14
// ---------------------------------------------------------------------------

#[cfg(feature = "substance_requirements")]
wasm_tool!(analyze_economic_substance, corp_finance_core::substance_requirements::economic_substance::EconomicSubstanceInput, corp_finance_core::substance_requirements::economic_substance::analyze_economic_substance);
#[cfg(feature = "substance_requirements")]
wasm_tool!(run_jurisdiction_substance_test, corp_finance_core::substance_requirements::jurisdiction_tests::JurisdictionTestInput, corp_finance_core::substance_requirements::jurisdiction_tests::run_jurisdiction_substance_test);

// ---------------------------------------------------------------------------
// Regulatory Reporting — Phase 14
// ---------------------------------------------------------------------------

#[cfg(feature = "regulatory_reporting")]
wasm_tool!(generate_aifmd_report, corp_finance_core::regulatory_reporting::aifmd_reporting::AifmdReportingInput, corp_finance_core::regulatory_reporting::aifmd_reporting::generate_aifmd_report);
#[cfg(feature = "regulatory_reporting")]
wasm_tool!(generate_sec_cftc_report, corp_finance_core::regulatory_reporting::sec_cftc_reporting::SecCftcReportingInput, corp_finance_core::regulatory_reporting::sec_cftc_reporting::generate_sec_cftc_report);

// ---------------------------------------------------------------------------
// AML Compliance — Phase 14
// ---------------------------------------------------------------------------

#[cfg(feature = "aml_compliance")]
wasm_tool!(assess_kyc_risk, corp_finance_core::aml_compliance::kyc_scoring::KycRiskInput, corp_finance_core::aml_compliance::kyc_scoring::assess_kyc_risk);
#[cfg(feature = "aml_compliance")]
wasm_tool!(screen_sanctions, corp_finance_core::aml_compliance::sanctions_screening::SanctionsScreeningInput, corp_finance_core::aml_compliance::sanctions_screening::screen_sanctions);

// ---------------------------------------------------------------------------
// Volatility Surface — Phase 15
// ---------------------------------------------------------------------------

#[cfg(feature = "volatility_surface")]
wasm_tool!(build_implied_vol_surface, corp_finance_core::volatility_surface::implied_vol_surface::ImpliedVolSurfaceInput, corp_finance_core::volatility_surface::implied_vol_surface::build_implied_vol_surface);
#[cfg(feature = "volatility_surface")]
wasm_tool!(calibrate_sabr, corp_finance_core::volatility_surface::sabr_model::SabrCalibrationInput, corp_finance_core::volatility_surface::sabr_model::calibrate_sabr);

// ---------------------------------------------------------------------------
// Portfolio Optimization — Phase 15
// ---------------------------------------------------------------------------

#[cfg(feature = "portfolio_optimization")]
wasm_tool!(optimize_mean_variance, corp_finance_core::portfolio_optimization::mean_variance::MeanVarianceInput, corp_finance_core::portfolio_optimization::mean_variance::optimize_mean_variance);
#[cfg(feature = "portfolio_optimization")]
wasm_tool!(optimize_black_litterman_portfolio, corp_finance_core::portfolio_optimization::black_litterman_portfolio::BlackLittermanInput, corp_finance_core::portfolio_optimization::black_litterman_portfolio::optimize_black_litterman);

// ---------------------------------------------------------------------------
// Risk Budgeting — Phase 15
// ---------------------------------------------------------------------------

#[cfg(feature = "risk_budgeting")]
wasm_tool!(analyze_factor_risk_budget, corp_finance_core::risk_budgeting::factor_risk_budget::FactorRiskBudgetInput, corp_finance_core::risk_budgeting::factor_risk_budget::analyze_factor_risk_budget);
#[cfg(feature = "risk_budgeting")]
wasm_tool!(analyze_tail_risk, corp_finance_core::risk_budgeting::tail_risk::TailRiskInput, corp_finance_core::risk_budgeting::tail_risk::analyze_tail_risk);

// ---------------------------------------------------------------------------
// Market Microstructure — Phase 15
// ---------------------------------------------------------------------------

#[cfg(feature = "market_microstructure")]
wasm_tool!(analyze_spreads, corp_finance_core::market_microstructure::spread_analysis::SpreadAnalysisInput, corp_finance_core::market_microstructure::spread_analysis::analyze_spreads);
#[cfg(feature = "market_microstructure")]
wasm_tool!(optimize_execution, corp_finance_core::market_microstructure::optimal_execution::OptimalExecutionInput, corp_finance_core::market_microstructure::optimal_execution::optimize_execution);

// ---------------------------------------------------------------------------
// Interest Rate Models — Phase 16
// ---------------------------------------------------------------------------

#[cfg(feature = "interest_rate_models")]
wasm_tool!(analyze_short_rate, corp_finance_core::interest_rate_models::short_rate::ShortRateInput, corp_finance_core::interest_rate_models::short_rate::analyze_short_rate);
#[cfg(feature = "interest_rate_models")]
wasm_tool!(fit_term_structure, corp_finance_core::interest_rate_models::term_structure::TermStructureInput, corp_finance_core::interest_rate_models::term_structure::fit_term_structure);

// ---------------------------------------------------------------------------
// Mortgage Analytics — Phase 16
// ---------------------------------------------------------------------------

#[cfg(feature = "mortgage_analytics")]
wasm_tool!(analyze_prepayment, corp_finance_core::mortgage_analytics::prepayment::PrepaymentInput, corp_finance_core::mortgage_analytics::prepayment::analyze_prepayment);
#[cfg(feature = "mortgage_analytics")]
wasm_tool!(analyze_mbs, corp_finance_core::mortgage_analytics::mbs_analytics::MbsAnalyticsInput, corp_finance_core::mortgage_analytics::mbs_analytics::analyze_mbs);

// ---------------------------------------------------------------------------
// Inflation-Linked — Phase 16
// ---------------------------------------------------------------------------

#[cfg(feature = "inflation_linked")]
wasm_tool!(analyze_tips, corp_finance_core::inflation_linked::tips_pricing::TipsAnalyticsInput, corp_finance_core::inflation_linked::tips_pricing::analyze_tips);
#[cfg(feature = "inflation_linked")]
wasm_tool!(analyze_inflation_derivatives, corp_finance_core::inflation_linked::inflation_derivatives::InflationDerivativeInput, corp_finance_core::inflation_linked::inflation_derivatives::analyze_inflation_derivatives);

// ---------------------------------------------------------------------------
// Repo Financing — Phase 16
// ---------------------------------------------------------------------------

#[cfg(feature = "repo_financing")]
wasm_tool!(analyze_repo, corp_finance_core::repo_financing::repo_rates::RepoAnalyticsInput, corp_finance_core::repo_financing::repo_rates::analyze_repo);
#[cfg(feature = "repo_financing")]
wasm_tool!(analyze_collateral, corp_finance_core::repo_financing::collateral_management::CollateralInput, corp_finance_core::repo_financing::collateral_management::analyze_collateral);

// ---------------------------------------------------------------------------
// Credit Scoring — Phase 18
// ---------------------------------------------------------------------------

#[cfg(feature = "credit_scoring")]
wasm_tool!(calculate_scorecard, corp_finance_core::credit_scoring::scorecard::ScorecardInput, corp_finance_core::credit_scoring::scorecard::calculate_scorecard);
#[cfg(feature = "credit_scoring")]
wasm_tool!(calculate_merton, corp_finance_core::credit_scoring::structural_model::MertonInput, corp_finance_core::credit_scoring::structural_model::calculate_merton);
#[cfg(feature = "credit_scoring")]
wasm_tool!(calculate_intensity_model, corp_finance_core::credit_scoring::intensity_model::IntensityModelInput, corp_finance_core::credit_scoring::intensity_model::calculate_intensity_model);
#[cfg(feature = "credit_scoring")]
wasm_tool!(calculate_calibration, corp_finance_core::credit_scoring::calibration::CalibrationInput, corp_finance_core::credit_scoring::calibration::calculate_calibration);
#[cfg(feature = "credit_scoring")]
wasm_tool!(calculate_scoring_validation, corp_finance_core::credit_scoring::validation::ValidationInput, corp_finance_core::credit_scoring::validation::calculate_validation);

// ---------------------------------------------------------------------------
// Capital Allocation — Phase 18
// ---------------------------------------------------------------------------

#[cfg(feature = "capital_allocation")]
wasm_tool!(calculate_economic_capital, corp_finance_core::capital_allocation::economic_capital::EconomicCapitalInput, corp_finance_core::capital_allocation::economic_capital::calculate_economic_capital);
#[cfg(feature = "capital_allocation")]
wasm_tool!(calculate_raroc, corp_finance_core::capital_allocation::raroc::RarocInput, corp_finance_core::capital_allocation::raroc::calculate_raroc);
#[cfg(feature = "capital_allocation")]
wasm_tool!(calculate_euler_allocation, corp_finance_core::capital_allocation::euler_allocation::EulerAllocationInput, corp_finance_core::capital_allocation::euler_allocation::calculate_euler_allocation);
#[cfg(feature = "capital_allocation")]
wasm_tool!(calculate_shapley_allocation, corp_finance_core::capital_allocation::shapley_allocation::ShapleyAllocationInput, corp_finance_core::capital_allocation::shapley_allocation::calculate_shapley_allocation);
#[cfg(feature = "capital_allocation")]
wasm_tool!(evaluate_limits, corp_finance_core::capital_allocation::limit_management::LimitManagementInput, corp_finance_core::capital_allocation::limit_management::evaluate_limits);

// ---------------------------------------------------------------------------
// CLO Analytics — Phase 18
// ---------------------------------------------------------------------------

#[cfg(feature = "clo_analytics")]
wasm_tool!(calculate_clo_waterfall, corp_finance_core::clo_analytics::waterfall::WaterfallInput, corp_finance_core::clo_analytics::waterfall::calculate_waterfall);
#[cfg(feature = "clo_analytics")]
wasm_tool!(calculate_coverage_tests, corp_finance_core::clo_analytics::coverage_tests::CoverageTestInput, corp_finance_core::clo_analytics::coverage_tests::calculate_coverage_tests);
#[cfg(feature = "clo_analytics")]
wasm_tool!(calculate_reinvestment, corp_finance_core::clo_analytics::reinvestment::ReinvestmentInput, corp_finance_core::clo_analytics::reinvestment::calculate_reinvestment);
#[cfg(feature = "clo_analytics")]
wasm_tool!(calculate_tranche_analytics, corp_finance_core::clo_analytics::tranche_analytics::TrancheAnalyticsInput, corp_finance_core::clo_analytics::tranche_analytics::calculate_tranche_analytics);
#[cfg(feature = "clo_analytics")]
wasm_tool!(calculate_clo_scenario, corp_finance_core::clo_analytics::scenario::CloScenarioInput, corp_finance_core::clo_analytics::scenario::calculate_clo_scenario);

// ---------------------------------------------------------------------------
// Fund of Funds — Phase 18
// ---------------------------------------------------------------------------

#[cfg(feature = "fund_of_funds")]
wasm_tool!(calculate_j_curve, corp_finance_core::fund_of_funds::j_curve::JCurveInput, corp_finance_core::fund_of_funds::j_curve::calculate_j_curve);
#[cfg(feature = "fund_of_funds")]
wasm_tool!(calculate_commitment_pacing, corp_finance_core::fund_of_funds::commitment_pacing::CommitmentPacingInput, corp_finance_core::fund_of_funds::commitment_pacing::calculate_commitment_pacing);
#[cfg(feature = "fund_of_funds")]
wasm_tool!(analyze_manager_selection, corp_finance_core::fund_of_funds::manager_selection::ManagerSelectionInput, corp_finance_core::fund_of_funds::manager_selection::analyze_manager_selection);
#[cfg(feature = "fund_of_funds")]
wasm_tool!(calculate_secondaries_pricing, corp_finance_core::fund_of_funds::secondaries::SecondariesPricingInput, corp_finance_core::fund_of_funds::secondaries::calculate_secondaries_pricing);
#[cfg(feature = "fund_of_funds")]
wasm_tool!(analyze_fof_portfolio, corp_finance_core::fund_of_funds::portfolio_construction::FofPortfolioInput, corp_finance_core::fund_of_funds::portfolio_construction::analyze_fof_portfolio);

// ---------------------------------------------------------------------------
// Earnings Quality — Phase 19
// ---------------------------------------------------------------------------

#[cfg(feature = "earnings_quality")]
wasm_tool!(calculate_beneish_mscore, corp_finance_core::earnings_quality::beneish::BeneishInput, corp_finance_core::earnings_quality::beneish::calculate_beneish_m_score);
#[cfg(feature = "earnings_quality")]
wasm_tool!(calculate_piotroski_fscore, corp_finance_core::earnings_quality::piotroski::PiotroskiInput, corp_finance_core::earnings_quality::piotroski::calculate_piotroski_f_score);
#[cfg(feature = "earnings_quality")]
wasm_tool!(calculate_accrual_quality, corp_finance_core::earnings_quality::accrual_quality::AccrualQualityInput, corp_finance_core::earnings_quality::accrual_quality::calculate_accrual_quality);
#[cfg(feature = "earnings_quality")]
wasm_tool!(calculate_revenue_quality, corp_finance_core::earnings_quality::revenue_quality::RevenueQualityInput, corp_finance_core::earnings_quality::revenue_quality::calculate_revenue_quality);
#[cfg(feature = "earnings_quality")]
wasm_tool!(calculate_earnings_quality_composite, corp_finance_core::earnings_quality::composite::EarningsQualityCompositeInput, corp_finance_core::earnings_quality::composite::calculate_earnings_quality_composite);

// ---------------------------------------------------------------------------
// Bank Analytics — Phase 19
// ---------------------------------------------------------------------------

#[cfg(feature = "bank_analytics")]
wasm_tool!(analyze_nim, corp_finance_core::bank_analytics::nim_analysis::NimAnalysisInput, corp_finance_core::bank_analytics::nim_analysis::analyze_nim);
#[cfg(feature = "bank_analytics")]
wasm_tool!(calculate_camels_rating, corp_finance_core::bank_analytics::camels::CamelsInput, corp_finance_core::bank_analytics::camels::calculate_camels);
#[cfg(feature = "bank_analytics")]
wasm_tool!(calculate_cecl_provision, corp_finance_core::bank_analytics::cecl_provisioning::CeclProvisioningInput, corp_finance_core::bank_analytics::cecl_provisioning::calculate_cecl);
#[cfg(feature = "bank_analytics")]
wasm_tool!(analyze_deposit_beta, corp_finance_core::bank_analytics::deposit_beta::DepositBetaInput, corp_finance_core::bank_analytics::deposit_beta::analyze_deposit_beta);
#[cfg(feature = "bank_analytics")]
wasm_tool!(analyze_loan_book, corp_finance_core::bank_analytics::loan_book::LoanBookInput, corp_finance_core::bank_analytics::loan_book::analyze_loan_book);

// ---------------------------------------------------------------------------
// Dividend Policy — Phase 19
// ---------------------------------------------------------------------------

#[cfg(feature = "dividend_policy")]
wasm_tool!(calculate_h_model_ddm, corp_finance_core::dividend_policy::h_model::HModelInput, corp_finance_core::dividend_policy::h_model::calculate_h_model);
#[cfg(feature = "dividend_policy")]
wasm_tool!(calculate_multistage_ddm, corp_finance_core::dividend_policy::multistage_ddm::MultistageDdmInput, corp_finance_core::dividend_policy::multistage_ddm::calculate_multistage_ddm);
#[cfg(feature = "dividend_policy")]
wasm_tool!(analyze_buyback, corp_finance_core::dividend_policy::buyback::BuybackInput, corp_finance_core::dividend_policy::buyback::calculate_buyback);
#[cfg(feature = "dividend_policy")]
wasm_tool!(analyze_payout_sustainability, corp_finance_core::dividend_policy::payout_sustainability::PayoutSustainabilityInput, corp_finance_core::dividend_policy::payout_sustainability::calculate_payout_sustainability);
#[cfg(feature = "dividend_policy")]
wasm_tool!(calculate_total_shareholder_return, corp_finance_core::dividend_policy::total_shareholder_return::TotalShareholderReturnInput, corp_finance_core::dividend_policy::total_shareholder_return::calculate_total_shareholder_return);

// ---------------------------------------------------------------------------
// Carbon Markets — Phase 19
// ---------------------------------------------------------------------------

#[cfg(feature = "carbon_markets")]
wasm_tool!(price_carbon_credit, corp_finance_core::carbon_markets::carbon_pricing::CarbonPricingInput, corp_finance_core::carbon_markets::carbon_pricing::calculate_carbon_pricing);
#[cfg(feature = "carbon_markets")]
wasm_tool!(analyze_ets_compliance, corp_finance_core::carbon_markets::ets_compliance::EtsComplianceInput, corp_finance_core::carbon_markets::ets_compliance::calculate_ets_compliance);
#[cfg(feature = "carbon_markets")]
wasm_tool!(analyze_cbam, corp_finance_core::carbon_markets::cbam::CbamInput, corp_finance_core::carbon_markets::cbam::calculate_cbam);
#[cfg(feature = "carbon_markets")]
wasm_tool!(value_carbon_offset, corp_finance_core::carbon_markets::offset_valuation::OffsetValuationInput, corp_finance_core::carbon_markets::offset_valuation::calculate_offset_valuation);
#[cfg(feature = "carbon_markets")]
wasm_tool!(calculate_shadow_carbon_price, corp_finance_core::carbon_markets::shadow_carbon::ShadowCarbonInput, corp_finance_core::carbon_markets::shadow_carbon::calculate_shadow_carbon);

// ---------------------------------------------------------------------------
// Private Wealth — Phase 20
// ---------------------------------------------------------------------------

#[cfg(feature = "private_wealth")]
wasm_tool!(analyze_concentrated_stock, corp_finance_core::private_wealth::concentrated_stock::ConcentratedStockInput, corp_finance_core::private_wealth::concentrated_stock::analyze_concentrated_stock);
#[cfg(feature = "private_wealth")]
wasm_tool!(compare_philanthropic_vehicles, corp_finance_core::private_wealth::philanthropic_vehicles::PhilanthropicInput, corp_finance_core::private_wealth::philanthropic_vehicles::compare_philanthropic_vehicles);
#[cfg(feature = "private_wealth")]
wasm_tool!(analyze_wealth_transfer, corp_finance_core::private_wealth::wealth_transfer::WealthTransferInput, corp_finance_core::private_wealth::wealth_transfer::analyze_wealth_transfer);
#[cfg(feature = "private_wealth")]
wasm_tool!(analyze_direct_indexing, corp_finance_core::private_wealth::direct_indexing::DirectIndexingInput, corp_finance_core::private_wealth::direct_indexing::analyze_direct_indexing);
#[cfg(feature = "private_wealth")]
wasm_tool!(evaluate_family_governance, corp_finance_core::private_wealth::family_governance::FamilyGovernanceInput, corp_finance_core::private_wealth::family_governance::evaluate_family_governance);

// ---------------------------------------------------------------------------
// Emerging Markets — Phase 20
// ---------------------------------------------------------------------------

#[cfg(feature = "emerging_markets")]
wasm_tool!(calculate_country_risk_premium, corp_finance_core::emerging_markets::country_risk_premium::CountryRiskPremiumInput, corp_finance_core::emerging_markets::country_risk_premium::calculate_country_risk_premium);
#[cfg(feature = "emerging_markets")]
wasm_tool!(assess_political_risk, corp_finance_core::emerging_markets::political_risk::PoliticalRiskInput, corp_finance_core::emerging_markets::political_risk::assess_political_risk);
#[cfg(feature = "emerging_markets")]
wasm_tool!(analyse_capital_controls, corp_finance_core::emerging_markets::capital_controls::CapitalControlsInput, corp_finance_core::emerging_markets::capital_controls::analyse_capital_controls);
#[cfg(feature = "emerging_markets")]
wasm_tool!(analyse_em_bonds, corp_finance_core::emerging_markets::em_bond_analysis::EmBondAnalysisInput, corp_finance_core::emerging_markets::em_bond_analysis::analyse_em_bonds);
#[cfg(feature = "emerging_markets")]
wasm_tool!(calculate_em_equity_premium, corp_finance_core::emerging_markets::em_equity_premium::EmEquityPremiumInput, corp_finance_core::emerging_markets::em_equity_premium::calculate_em_equity_premium);

// ---------------------------------------------------------------------------
// Index Construction — Phase 20
// ---------------------------------------------------------------------------

#[cfg(feature = "index_construction")]
wasm_tool!(calculate_weighting, corp_finance_core::index_construction::weighting::WeightingInput, corp_finance_core::index_construction::weighting::calculate_weighting);
#[cfg(feature = "index_construction")]
wasm_tool!(calculate_rebalancing, corp_finance_core::index_construction::rebalancing::RebalancingInput, corp_finance_core::index_construction::rebalancing::calculate_rebalancing);
#[cfg(feature = "index_construction")]
wasm_tool!(calculate_tracking_error, corp_finance_core::index_construction::tracking_error::TrackingErrorInput, corp_finance_core::index_construction::tracking_error::calculate_tracking_error);
#[cfg(feature = "index_construction")]
wasm_tool!(calculate_smart_beta, corp_finance_core::index_construction::smart_beta::SmartBetaInput, corp_finance_core::index_construction::smart_beta::calculate_smart_beta);
#[cfg(feature = "index_construction")]
wasm_tool!(calculate_reconstitution, corp_finance_core::index_construction::reconstitution::ReconstitutionInput, corp_finance_core::index_construction::reconstitution::calculate_reconstitution);

// ---------------------------------------------------------------------------
// Financial Forensics — Phase 20
// ---------------------------------------------------------------------------

#[cfg(feature = "financial_forensics")]
wasm_tool!(analyze_benfords_law, corp_finance_core::financial_forensics::benfords_law::BenfordsLawInput, corp_finance_core::financial_forensics::benfords_law::analyze_benfords_law);
#[cfg(feature = "financial_forensics")]
wasm_tool!(calculate_dupont, corp_finance_core::financial_forensics::dupont_analysis::DupontInput, corp_finance_core::financial_forensics::dupont_analysis::calculate_dupont);
#[cfg(feature = "financial_forensics")]
wasm_tool!(calculate_zscore_models, corp_finance_core::financial_forensics::zscore_models::ZScoreModelsInput, corp_finance_core::financial_forensics::zscore_models::calculate_zscore_models);
#[cfg(feature = "financial_forensics")]
wasm_tool!(calculate_peer_benchmarking, corp_finance_core::financial_forensics::peer_benchmarking::PeerBenchmarkingInput, corp_finance_core::financial_forensics::peer_benchmarking::calculate_peer_benchmarking);
#[cfg(feature = "financial_forensics")]
wasm_tool!(calculate_red_flag_scoring, corp_finance_core::financial_forensics::red_flag_scoring::RedFlagScoringInput, corp_finance_core::financial_forensics::red_flag_scoring::calculate_red_flag_scoring);

// ---------------------------------------------------------------------------
// Workflows
// ---------------------------------------------------------------------------

#[cfg(feature = "workflows")]
wasm_tool!(workflow_list, corp_finance_core::workflows::types::WorkflowListInput, corp_finance_core::workflows::types::list_workflows);
#[cfg(feature = "workflows")]
wasm_tool!(workflow_describe, corp_finance_core::workflows::types::WorkflowDescribeInput, corp_finance_core::workflows::types::describe_workflow);
#[cfg(feature = "workflows")]
wasm_tool!(workflow_validate, corp_finance_core::workflows::types::WorkflowValidateInput, corp_finance_core::workflows::types::validate_workflow);
#[cfg(feature = "workflows")]
wasm_tool!(workflow_quality_check, corp_finance_core::workflows::types::WorkflowQualityCheckInput, corp_finance_core::workflows::types::quality_check_workflow);
#[cfg(feature = "workflows")]
wasm_tool!(workflow_audit, corp_finance_core::workflows::audit::WorkflowAuditInput, corp_finance_core::workflows::audit::generate_audit_trail);

#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}
