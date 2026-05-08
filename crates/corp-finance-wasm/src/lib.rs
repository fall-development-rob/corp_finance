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

// ---------------------------------------------------------------------------
// Wave 16y — offshore_structures module (8 tools)
// ---------------------------------------------------------------------------

wasm_tool!(cayman_fund_structure, corp_finance_core::offshore_structures::cayman::CaymanFundInput, corp_finance_core::offshore_structures::cayman::analyze_cayman_structure);
wasm_tool!(lux_ireland_fund_structure, corp_finance_core::offshore_structures::luxembourg::LuxFundInput, corp_finance_core::offshore_structures::luxembourg::analyze_lux_structure);
wasm_tool!(channel_islands_fund_structure, corp_finance_core::offshore_structures::channel_islands::JerseyFundInput, corp_finance_core::offshore_structures::channel_islands::analyze_jersey_fund);
wasm_tool!(singapore_vcc_structure, corp_finance_core::offshore_structures::singapore_vcc::VccInput, corp_finance_core::offshore_structures::singapore_vcc::analyze_vcc_structure);
wasm_tool!(hong_kong_fund_structure, corp_finance_core::offshore_structures::hong_kong_funds::OfcInput, corp_finance_core::offshore_structures::hong_kong_funds::analyze_ofc_structure);
wasm_tool!(middle_east_fund_structure, corp_finance_core::offshore_structures::middle_east_funds::DifcFundInput, corp_finance_core::offshore_structures::middle_east_funds::analyze_difc_fund);
wasm_tool!(jurisdiction_comparison, corp_finance_core::offshore_structures::jurisdiction_comparison::JurisdictionComparisonInput, corp_finance_core::offshore_structures::jurisdiction_comparison::compare_jurisdictions);
wasm_tool!(fund_migration_analysis, corp_finance_core::offshore_structures::fund_migration::MigrationFeasibilityInput, corp_finance_core::offshore_structures::fund_migration::migration_feasibility);

// ---------------------------------------------------------------------------
// Wave 16y — derivatives module (8 tools)
// ---------------------------------------------------------------------------

wasm_tool!(option_pricer, corp_finance_core::derivatives::options::OptionInput, corp_finance_core::derivatives::options::price_option);
wasm_tool!(implied_volatility, corp_finance_core::derivatives::options::ImpliedVolInput, corp_finance_core::derivatives::options::implied_volatility);
wasm_tool!(forward_pricer, corp_finance_core::derivatives::forwards::ForwardInput, corp_finance_core::derivatives::forwards::price_forward);
wasm_tool!(forward_position_value, corp_finance_core::derivatives::forwards::ForwardPositionInput, corp_finance_core::derivatives::forwards::value_forward_position);
wasm_tool!(futures_basis_analysis, corp_finance_core::derivatives::forwards::BasisAnalysisInput, corp_finance_core::derivatives::forwards::futures_basis_analysis);
wasm_tool!(interest_rate_swap, corp_finance_core::derivatives::swaps::IrsInput, corp_finance_core::derivatives::swaps::value_interest_rate_swap);
wasm_tool!(currency_swap, corp_finance_core::derivatives::swaps::CurrencySwapInput, corp_finance_core::derivatives::swaps::value_currency_swap);
wasm_tool!(option_strategy, corp_finance_core::derivatives::strategies::StrategyInput, corp_finance_core::derivatives::strategies::analyze_strategy);

// ---------------------------------------------------------------------------
// Wave 16y — jurisdiction module (7 tools)
// ---------------------------------------------------------------------------

wasm_tool!(fund_fee_calculator, corp_finance_core::jurisdiction::fund_fees::FundFeeInput, corp_finance_core::jurisdiction::fund_fees::calculate_fund_fees);
wasm_tool!(gaap_ifrs_reconcile, corp_finance_core::jurisdiction::reconciliation::ReconciliationInput, corp_finance_core::jurisdiction::reconciliation::reconcile_accounting_standards);
wasm_tool!(withholding_tax, corp_finance_core::jurisdiction::withholding_tax::WhtInput, corp_finance_core::jurisdiction::withholding_tax::calculate_withholding_tax);
wasm_tool!(nav_calculator, corp_finance_core::jurisdiction::nav::NavInput, corp_finance_core::jurisdiction::nav::calculate_nav);
wasm_tool!(gp_economics, corp_finance_core::jurisdiction::gp_economics::GpEconomicsInput, corp_finance_core::jurisdiction::gp_economics::calculate_gp_economics);
wasm_tool!(investor_net_returns, corp_finance_core::jurisdiction::investor_returns::InvestorNetReturnsInput, corp_finance_core::jurisdiction::investor_returns::calculate_investor_net_returns);
wasm_tool!(ubti_screening, corp_finance_core::jurisdiction::ubti::UbtiScreeningInput, corp_finance_core::jurisdiction::ubti::screen_ubti_eci);

// ---------------------------------------------------------------------------
// Wave 16y — pe module (6 tools)
// ---------------------------------------------------------------------------

wasm_tool!(returns_calculator, corp_finance_core::pe::returns::ReturnsInput, corp_finance_core::pe::returns::calculate_returns);
wasm_tool!(debt_schedule, corp_finance_core::pe::debt_schedule::DebtTrancheInput, corp_finance_core::pe::debt_schedule::build_debt_schedule);
wasm_tool!(sources_uses, corp_finance_core::pe::sources_uses::SourcesUsesInput, corp_finance_core::pe::sources_uses::build_sources_uses);
wasm_tool!(lbo_model, corp_finance_core::pe::lbo::LboInput, corp_finance_core::pe::lbo::build_lbo);
wasm_tool!(waterfall_calculator, corp_finance_core::pe::waterfall::WaterfallInput, corp_finance_core::pe::waterfall::calculate_waterfall);
wasm_tool!(altman_zscore, corp_finance_core::credit::altman::AltmanInput, corp_finance_core::credit::altman::calculate_altman_zscore);

// ---------------------------------------------------------------------------
// Wave 16y — fixed_income module (6 tools)
// ---------------------------------------------------------------------------

wasm_tool!(bond_pricer, corp_finance_core::fixed_income::bonds::BondPricingInput, corp_finance_core::fixed_income::bonds::price_bond);
wasm_tool!(bond_yield, corp_finance_core::fixed_income::yields::BondYieldInput, corp_finance_core::fixed_income::yields::calculate_bond_yield);
wasm_tool!(bootstrap_spot_curve, corp_finance_core::fixed_income::yields::BootstrapInput, corp_finance_core::fixed_income::yields::bootstrap_spot_curve);
wasm_tool!(nelson_siegel_fit, corp_finance_core::fixed_income::yields::NelsonSiegelInput, corp_finance_core::fixed_income::yields::fit_nelson_siegel);
wasm_tool!(bond_duration, corp_finance_core::fixed_income::duration::DurationInput, corp_finance_core::fixed_income::duration::calculate_duration);
wasm_tool!(credit_spreads, corp_finance_core::fixed_income::spreads::CreditSpreadInput, corp_finance_core::fixed_income::spreads::calculate_credit_spreads);

// ---------------------------------------------------------------------------
// Wave 16y — institutional_real_estate module (6 tools)
// ---------------------------------------------------------------------------

wasm_tool!(institutional_rent_roll, corp_finance_core::institutional_real_estate::rent_roll::TenantScheduleInput, corp_finance_core::institutional_real_estate::rent_roll::tenant_schedule);
wasm_tool!(institutional_comparable_sales, corp_finance_core::institutional_real_estate::comparable_sales::CompAdjustmentInput, corp_finance_core::institutional_real_estate::comparable_sales::comp_adjustment_grid);
wasm_tool!(institutional_hbu_analysis, corp_finance_core::institutional_real_estate::highest_best_use::HbuAnalysisInput, corp_finance_core::institutional_real_estate::highest_best_use::hbu_analysis);
wasm_tool!(institutional_replacement_cost, corp_finance_core::institutional_real_estate::replacement_cost::CostApproachInput, corp_finance_core::institutional_real_estate::replacement_cost::cost_approach);
wasm_tool!(institutional_benchmark, corp_finance_core::institutional_real_estate::benchmark::NcreifAttributionInput, corp_finance_core::institutional_real_estate::benchmark::ncreif_attribution);
wasm_tool!(institutional_acquisition, corp_finance_core::institutional_real_estate::acquisition::AcquisitionModelInput, corp_finance_core::institutional_real_estate::acquisition::acquisition_model);

// ---------------------------------------------------------------------------
// Wave 16y — earnings_quality module (5 tools)
// ---------------------------------------------------------------------------

wasm_tool!(beneish_mscore, corp_finance_core::earnings_quality::beneish::BeneishInput, corp_finance_core::earnings_quality::beneish::calculate_beneish_m_score);
wasm_tool!(piotroski_fscore, corp_finance_core::earnings_quality::piotroski::PiotroskiInput, corp_finance_core::earnings_quality::piotroski::calculate_piotroski_f_score);
wasm_tool!(accrual_quality, corp_finance_core::earnings_quality::accrual_quality::AccrualQualityInput, corp_finance_core::earnings_quality::accrual_quality::calculate_accrual_quality);
wasm_tool!(revenue_quality, corp_finance_core::earnings_quality::revenue_quality::RevenueQualityInput, corp_finance_core::earnings_quality::revenue_quality::calculate_revenue_quality);
wasm_tool!(earnings_quality_composite, corp_finance_core::earnings_quality::composite::EarningsQualityCompositeInput, corp_finance_core::earnings_quality::composite::calculate_earnings_quality_composite);

// ---------------------------------------------------------------------------
// Wave 16y — financial_forensics module (5 tools)
// ---------------------------------------------------------------------------

wasm_tool!(benfords_law, corp_finance_core::financial_forensics::benfords_law::BenfordsLawInput, corp_finance_core::financial_forensics::benfords_law::analyze_benfords_law);
wasm_tool!(dupont_analysis, corp_finance_core::financial_forensics::dupont_analysis::DupontInput, corp_finance_core::financial_forensics::dupont_analysis::calculate_dupont);
wasm_tool!(zscore_models, corp_finance_core::financial_forensics::zscore_models::ZScoreModelsInput, corp_finance_core::financial_forensics::zscore_models::calculate_zscore_models);
wasm_tool!(peer_benchmarking, corp_finance_core::financial_forensics::peer_benchmarking::PeerBenchmarkingInput, corp_finance_core::financial_forensics::peer_benchmarking::calculate_peer_benchmarking);
wasm_tool!(red_flag_scoring, corp_finance_core::financial_forensics::red_flag_scoring::RedFlagScoringInput, corp_finance_core::financial_forensics::red_flag_scoring::calculate_red_flag_scoring);

// ---------------------------------------------------------------------------
// Wave 16y — bank_analytics module (5 tools)
// ---------------------------------------------------------------------------

wasm_tool!(nim_analysis, corp_finance_core::bank_analytics::nim_analysis::NimAnalysisInput, corp_finance_core::bank_analytics::nim_analysis::analyze_nim);
wasm_tool!(camels_rating, corp_finance_core::bank_analytics::camels::CamelsInput, corp_finance_core::bank_analytics::camels::calculate_camels);
wasm_tool!(cecl_provisioning, corp_finance_core::bank_analytics::cecl_provisioning::CeclProvisioningInput, corp_finance_core::bank_analytics::cecl_provisioning::calculate_cecl);
wasm_tool!(deposit_beta, corp_finance_core::bank_analytics::deposit_beta::DepositBetaInput, corp_finance_core::bank_analytics::deposit_beta::analyze_deposit_beta);
wasm_tool!(loan_book_analysis, corp_finance_core::bank_analytics::loan_book::LoanBookInput, corp_finance_core::bank_analytics::loan_book::analyze_loan_book);

// ---------------------------------------------------------------------------
// Wave 16y — emerging_markets module (5 tools)
// ---------------------------------------------------------------------------

wasm_tool!(country_risk_premium, corp_finance_core::emerging_markets::country_risk_premium::CountryRiskPremiumInput, corp_finance_core::emerging_markets::country_risk_premium::calculate_country_risk_premium);
wasm_tool!(political_risk, corp_finance_core::emerging_markets::political_risk::PoliticalRiskInput, corp_finance_core::emerging_markets::political_risk::assess_political_risk);
wasm_tool!(capital_controls, corp_finance_core::emerging_markets::capital_controls::CapitalControlsInput, corp_finance_core::emerging_markets::capital_controls::analyse_capital_controls);
wasm_tool!(em_bond_analysis, corp_finance_core::emerging_markets::em_bond_analysis::EmBondAnalysisInput, corp_finance_core::emerging_markets::em_bond_analysis::analyse_em_bonds);
wasm_tool!(em_equity_premium, corp_finance_core::emerging_markets::em_equity_premium::EmEquityPremiumInput, corp_finance_core::emerging_markets::em_equity_premium::calculate_em_equity_premium);

// ---------------------------------------------------------------------------
// Wave 16y — carbon_markets module (5 tools)
// ---------------------------------------------------------------------------

wasm_tool!(carbon_credit_pricing, corp_finance_core::carbon_markets::carbon_pricing::CarbonPricingInput, corp_finance_core::carbon_markets::carbon_pricing::calculate_carbon_pricing);
wasm_tool!(ets_compliance, corp_finance_core::carbon_markets::ets_compliance::EtsComplianceInput, corp_finance_core::carbon_markets::ets_compliance::calculate_ets_compliance);
wasm_tool!(cbam_analysis, corp_finance_core::carbon_markets::cbam::CbamInput, corp_finance_core::carbon_markets::cbam::calculate_cbam);
wasm_tool!(offset_valuation, corp_finance_core::carbon_markets::offset_valuation::OffsetValuationInput, corp_finance_core::carbon_markets::offset_valuation::calculate_offset_valuation);
wasm_tool!(shadow_carbon_price, corp_finance_core::carbon_markets::shadow_carbon::ShadowCarbonInput, corp_finance_core::carbon_markets::shadow_carbon::calculate_shadow_carbon);

// ---------------------------------------------------------------------------
// Wave 16y — clo_analytics module (5 tools)
// ---------------------------------------------------------------------------

wasm_tool!(clo_waterfall, corp_finance_core::clo_analytics::waterfall::WaterfallInput, corp_finance_core::clo_analytics::waterfall::calculate_waterfall);
wasm_tool!(clo_coverage_tests, corp_finance_core::clo_analytics::coverage_tests::CoverageTestInput, corp_finance_core::clo_analytics::coverage_tests::calculate_coverage_tests);
wasm_tool!(clo_reinvestment, corp_finance_core::clo_analytics::reinvestment::ReinvestmentInput, corp_finance_core::clo_analytics::reinvestment::calculate_reinvestment);
wasm_tool!(clo_tranche_analytics, corp_finance_core::clo_analytics::tranche_analytics::TrancheAnalyticsInput, corp_finance_core::clo_analytics::tranche_analytics::calculate_tranche_analytics);
wasm_tool!(clo_scenario, corp_finance_core::clo_analytics::scenario::CloScenarioInput, corp_finance_core::clo_analytics::scenario::calculate_clo_scenario);

// ---------------------------------------------------------------------------
// Wave 16z — fund_of_funds module (5 tools)
// ---------------------------------------------------------------------------

wasm_tool!(j_curve_model, corp_finance_core::fund_of_funds::j_curve::JCurveInput, corp_finance_core::fund_of_funds::j_curve::calculate_j_curve);
wasm_tool!(commitment_pacing, corp_finance_core::fund_of_funds::commitment_pacing::CommitmentPacingInput, corp_finance_core::fund_of_funds::commitment_pacing::calculate_commitment_pacing);
wasm_tool!(manager_selection, corp_finance_core::fund_of_funds::manager_selection::ManagerSelectionInput, corp_finance_core::fund_of_funds::manager_selection::analyze_manager_selection);
wasm_tool!(secondaries_pricing, corp_finance_core::fund_of_funds::secondaries::SecondariesPricingInput, corp_finance_core::fund_of_funds::secondaries::calculate_secondaries_pricing);
wasm_tool!(fof_portfolio, corp_finance_core::fund_of_funds::portfolio_construction::FofPortfolioInput, corp_finance_core::fund_of_funds::portfolio_construction::analyze_fof_portfolio);

// ---------------------------------------------------------------------------
// Wave 16z — private_wealth module (5 tools)
// ---------------------------------------------------------------------------

wasm_tool!(concentrated_stock, corp_finance_core::private_wealth::concentrated_stock::ConcentratedStockInput, corp_finance_core::private_wealth::concentrated_stock::analyze_concentrated_stock);
wasm_tool!(philanthropic_vehicles, corp_finance_core::private_wealth::philanthropic_vehicles::PhilanthropicInput, corp_finance_core::private_wealth::philanthropic_vehicles::compare_philanthropic_vehicles);
wasm_tool!(wealth_transfer, corp_finance_core::private_wealth::wealth_transfer::WealthTransferInput, corp_finance_core::private_wealth::wealth_transfer::analyze_wealth_transfer);
wasm_tool!(direct_indexing, corp_finance_core::private_wealth::direct_indexing::DirectIndexingInput, corp_finance_core::private_wealth::direct_indexing::analyze_direct_indexing);
wasm_tool!(family_governance, corp_finance_core::private_wealth::family_governance::FamilyGovernanceInput, corp_finance_core::private_wealth::family_governance::evaluate_family_governance);

// ---------------------------------------------------------------------------
// Wave 16z — venture module (5 tools)
// ---------------------------------------------------------------------------

wasm_tool!(funding_round, corp_finance_core::venture::valuation::FundingRoundInput, corp_finance_core::venture::valuation::model_funding_round);
wasm_tool!(dilution_analysis, corp_finance_core::venture::valuation::DilutionInput, corp_finance_core::venture::valuation::analyze_dilution);
wasm_tool!(convertible_note, corp_finance_core::venture::instruments::ConvertibleNoteInput, corp_finance_core::venture::instruments::convert_note);
wasm_tool!(safe_conversion, corp_finance_core::venture::instruments::SafeInput, corp_finance_core::venture::instruments::convert_safe);
wasm_tool!(venture_fund_model, corp_finance_core::venture::returns::VentureFundInput, corp_finance_core::venture::returns::model_venture_fund);

// ---------------------------------------------------------------------------
// Wave 16z — credit_scoring module (5 tools)
// ---------------------------------------------------------------------------

wasm_tool!(credit_scorecard, corp_finance_core::credit_scoring::scorecard::ScorecardInput, corp_finance_core::credit_scoring::scorecard::calculate_scorecard);
wasm_tool!(merton_pd, corp_finance_core::credit_scoring::structural_model::MertonInput, corp_finance_core::credit_scoring::structural_model::calculate_merton);
wasm_tool!(intensity_model, corp_finance_core::credit_scoring::intensity_model::IntensityModelInput, corp_finance_core::credit_scoring::intensity_model::calculate_intensity_model);
wasm_tool!(pd_calibration, corp_finance_core::credit_scoring::calibration::CalibrationInput, corp_finance_core::credit_scoring::calibration::calculate_calibration);
wasm_tool!(scoring_validation, corp_finance_core::credit_scoring::validation::ValidationInput, corp_finance_core::credit_scoring::validation::calculate_validation);

// ---------------------------------------------------------------------------
// Wave 16z — capital_allocation module (5 tools)
// ---------------------------------------------------------------------------

wasm_tool!(economic_capital, corp_finance_core::capital_allocation::economic_capital::EconomicCapitalInput, corp_finance_core::capital_allocation::economic_capital::calculate_economic_capital);
wasm_tool!(raroc_calculation, corp_finance_core::capital_allocation::raroc::RarocInput, corp_finance_core::capital_allocation::raroc::calculate_raroc);
wasm_tool!(euler_allocation, corp_finance_core::capital_allocation::euler_allocation::EulerAllocationInput, corp_finance_core::capital_allocation::euler_allocation::calculate_euler_allocation);
wasm_tool!(shapley_allocation, corp_finance_core::capital_allocation::shapley_allocation::ShapleyAllocationInput, corp_finance_core::capital_allocation::shapley_allocation::calculate_shapley_allocation);
wasm_tool!(limit_management, corp_finance_core::capital_allocation::limit_management::LimitManagementInput, corp_finance_core::capital_allocation::limit_management::evaluate_limits);

// ---------------------------------------------------------------------------
// Wave 16z — index_construction module (5 tools)
// ---------------------------------------------------------------------------

wasm_tool!(index_weighting, corp_finance_core::index_construction::weighting::WeightingInput, corp_finance_core::index_construction::weighting::calculate_weighting);
wasm_tool!(index_rebalancing, corp_finance_core::index_construction::rebalancing::RebalancingInput, corp_finance_core::index_construction::rebalancing::calculate_rebalancing);
wasm_tool!(tracking_error, corp_finance_core::index_construction::tracking_error::TrackingErrorInput, corp_finance_core::index_construction::tracking_error::calculate_tracking_error);
wasm_tool!(smart_beta, corp_finance_core::index_construction::smart_beta::SmartBetaInput, corp_finance_core::index_construction::smart_beta::calculate_smart_beta);
wasm_tool!(index_reconstitution, corp_finance_core::index_construction::reconstitution::ReconstitutionInput, corp_finance_core::index_construction::reconstitution::calculate_reconstitution);

// ---------------------------------------------------------------------------
// Wave 16z — regulatory module (4 tools)
// ---------------------------------------------------------------------------

wasm_tool!(regulatory_capital, corp_finance_core::regulatory::capital::RegulatoryCapitalInput, corp_finance_core::regulatory::capital::calculate_regulatory_capital);
wasm_tool!(lcr, corp_finance_core::regulatory::liquidity::LcrInput, corp_finance_core::regulatory::liquidity::calculate_lcr);
wasm_tool!(nsfr, corp_finance_core::regulatory::liquidity::NsfrInput, corp_finance_core::regulatory::liquidity::calculate_nsfr);
wasm_tool!(alm_analysis, corp_finance_core::regulatory::alm::AlmInput, corp_finance_core::regulatory::alm::analyze_alm);

// ---------------------------------------------------------------------------
// Wave 16z — quant_risk module (4 tools)
// ---------------------------------------------------------------------------

wasm_tool!(factor_model, corp_finance_core::quant_risk::factor_models::FactorModelInput, corp_finance_core::quant_risk::factor_models::run_factor_model);
wasm_tool!(black_litterman, corp_finance_core::quant_risk::black_litterman::BlackLittermanInput, corp_finance_core::quant_risk::black_litterman::run_black_litterman);
wasm_tool!(risk_parity, corp_finance_core::quant_risk::risk_parity::RiskParityInput, corp_finance_core::quant_risk::risk_parity::calculate_risk_parity);
wasm_tool!(stress_test, corp_finance_core::quant_risk::stress_testing::StressTestInput, corp_finance_core::quant_risk::stress_testing::run_stress_test);

// ---------------------------------------------------------------------------
// Wave 16z — esg module (4 tools)
// ---------------------------------------------------------------------------

wasm_tool!(esg_score, corp_finance_core::esg::scoring::EsgScoreInput, corp_finance_core::esg::scoring::calculate_esg_score);
wasm_tool!(carbon_footprint, corp_finance_core::esg::climate::CarbonFootprintInput, corp_finance_core::esg::climate::analyze_carbon_footprint);
wasm_tool!(green_bond, corp_finance_core::esg::climate::GreenBondInput, corp_finance_core::esg::climate::analyze_green_bond);
wasm_tool!(sll_covenants, corp_finance_core::esg::climate::SllInput, corp_finance_core::esg::climate::test_sll_covenants);

// ---------------------------------------------------------------------------
// Wave 16z — insurance module (4 tools)
// ---------------------------------------------------------------------------

wasm_tool!(loss_reserving, corp_finance_core::insurance::reserving::ReservingInput, corp_finance_core::insurance::reserving::estimate_reserves);
wasm_tool!(premium_pricing, corp_finance_core::insurance::pricing::PremiumPricingInput, corp_finance_core::insurance::pricing::price_premium);
wasm_tool!(combined_ratio, corp_finance_core::insurance::pricing::CombinedRatioInput, corp_finance_core::insurance::pricing::analyze_combined_ratio);
wasm_tool!(solvency_scr, corp_finance_core::insurance::pricing::ScrInput, corp_finance_core::insurance::pricing::calculate_scr);

// ---------------------------------------------------------------------------
// Wave 16z — fx_commodities module (4 tools)
// ---------------------------------------------------------------------------

wasm_tool!(fx_forward, corp_finance_core::fx_commodities::fx::FxForwardInput, corp_finance_core::fx_commodities::fx::price_fx_forward);
wasm_tool!(cross_rate, corp_finance_core::fx_commodities::fx::CrossRateInput, corp_finance_core::fx_commodities::fx::calculate_cross_rate);
wasm_tool!(commodity_forward, corp_finance_core::fx_commodities::commodities::CommodityForwardInput, corp_finance_core::fx_commodities::commodities::price_commodity_forward);
wasm_tool!(commodity_curve, corp_finance_core::fx_commodities::commodities::CommodityCurveInput, corp_finance_core::fx_commodities::commodities::analyze_commodity_curve);

// ---------------------------------------------------------------------------
// Wave 16z — private_credit module (3 tools)
// ---------------------------------------------------------------------------

wasm_tool!(unitranche_pricing, corp_finance_core::private_credit::unitranche::UnitrancheInput, corp_finance_core::private_credit::unitranche::price_unitranche);
wasm_tool!(direct_loan, corp_finance_core::private_credit::direct_lending::DirectLoanInput, corp_finance_core::private_credit::direct_lending::model_direct_loan);
wasm_tool!(syndication_analysis, corp_finance_core::private_credit::direct_lending::SyndicationInput, corp_finance_core::private_credit::direct_lending::analyze_syndication);

#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}
