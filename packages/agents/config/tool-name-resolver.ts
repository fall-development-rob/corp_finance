// Tool name resolver — maps domain-prefixed agent tool names to MCP-registered names
// Agent think() methods construct names like 'valuation_dcf_model' while the MCP
// server registers them as 'dcf_model'. This map bridges the gap.

/**
 * Static mapping from agent-constructed tool names to MCP-registered tool names.
 * Names that already match exactly (e.g. 'three_statement_model', 'monte_carlo_simulation')
 * are not included — the resolver falls through to exact match.
 */
const AGENT_TO_MCP: Record<string, string> = {
  // ── Equity analyst ──────────────────────────────────────────────
  'valuation_dcf_model':                    'dcf_model',
  'valuation_wacc_calculation':             'wacc_calculator',
  'valuation_comparable_companies':         'comps_analysis',
  'equity_research_fundamental_analysis':   'sotp_valuation',
  'valuation_sotp_valuation':               'sotp_valuation',
  'valuation_target_price':                 'target_price',
  'valuation_forward_pricer':                'forward_pricer',
  'equity_research_implied_vol':             'implied_volatility',
  'earnings_quality_accruals_analysis':     'accrual_quality',
  'earnings_quality_piotroski':             'piotroski_fscore',
  'earnings_quality_revenue':               'revenue_quality',
  'earnings_quality_composite':             'earnings_quality_composite',
  'dividend_policy_sustainability':         'payout_sustainability',
  'dividend_policy_h_model':                'h_model_ddm',
  'dividend_policy_multistage':             'multistage_ddm',
  'dividend_policy_buyback':                'buyback_analysis',
  'dividend_policy_total_shareholder_return': 'total_shareholder_return',
  'performance_attribution_brinson':        'brinson_attribution',
  'financial_forensics_dupont':             'dupont_analysis',
  'financial_forensics_beneish':            'beneish_mscore',
  'financial_forensics_benfords_law':       'benfords_law',
  'financial_forensics_zscore':             'zscore_models',
  'financial_forensics_peer_benchmarking':  'peer_benchmarking',
  'financial_forensics_red_flags':          'red_flag_scoring',

  // ── Credit analyst ──────────────────────────────────────────────
  'credit_scoring_corporate':               'credit_scorecard',
  'credit_spread_analysis':                 'credit_spreads',
  'credit_default_probability':             'merton_pd',
  'credit_portfolio_var':                   'portfolio_credit_risk',
  'credit_derivatives_cds_pricing':         'cds_pricing',
  'credit_derivatives_cva':                 'cva_calculation',
  'credit_debt_capacity':                   'debt_capacity',
  'credit_metrics_analysis':                'credit_metrics',
  'credit_scoring_intensity':               'intensity_model',
  'credit_scoring_pd_calibration':          'pd_calibration',
  'credit_scoring_validation':              'scoring_validation',
  'credit_portfolio_migration':             'credit_migration',
  'credit_covenant_analysis':               'covenant_compliance',
  'restructuring_distressed_valuation':     'distressed_debt_analysis',
  'restructuring_recovery_analysis':        'recovery_analysis',
  'restructuring_waterfall':                'distressed_debt_analysis',
  'bank_analytics_capital_adequacy':        'camels_rating',
  'bank_analytics_cecl':                    'cecl_provisioning',
  'bank_analytics_nim':                     'nim_analysis',
  'bank_analytics_deposit_beta':            'deposit_beta',
  'bank_analytics_loan_book':               'loan_book_analysis',

  // ── Fixed income analyst ────────────────────────────────────────
  'fixed_income_bond_pricing':              'bond_pricer',
  'fixed_income_bond_yield':                'bond_yield',
  'fixed_income_yield_curve':               'bootstrap_spot_curve',
  'fixed_income_duration':                  'bond_duration',
  'fixed_income_nelson_siegel':             'nelson_siegel_fit',
  'fixed_income_spread_analysis':           'credit_spreads',
  'interest_rate_models_vasicek':           'short_rate_model',
  'interest_rate_models_term_structure':    'term_structure_fit',
  'inflation_linked_tips_analysis':         'tips_analytics',
  'inflation_linked_derivatives':           'inflation_derivatives',
  'repo_financing_haircut_analysis':        'repo_analytics',
  'repo_financing_collateral':              'collateral_analytics',
  'mortgage_analytics_prepayment_model':    'prepayment_analysis',
  'mortgage_analytics_mbs':                 'mbs_analytics',
  'municipal_credit_analysis':              'municipal_analysis',
  'municipal_bond_pricing':                 'muni_bond_pricing',
  'sovereign_debt_sustainability':          'sovereign_bond_analysis',

  // ── Derivatives analyst ─────────────────────────────────────────
  'derivatives_option_pricing':             'option_pricer',
  'derivatives_greeks_calculation':         'option_pricer',
  'derivatives_forward_value':              'forward_position_value',
  'derivatives_forward_pricing':            'forward_pricer',
  'derivatives_futures_basis':              'futures_basis_analysis',
  'derivatives_irs':                        'interest_rate_swap',
  'derivatives_currency_swap':              'currency_swap',
  'derivatives_strategy':                   'option_strategy',
  'derivatives_implied_vol':                'implied_volatility',
  'volatility_surface_interpolation':       'implied_vol_surface',
  'volatility_surface_smile_analysis':      'sabr_calibration',
  'convertibles_pricing':                   'convertible_bond_pricing',
  'convertibles_analysis':                  'convertible_bond_analysis',
  'structured_products_analysis':           'structured_note_pricing',
  'structured_products_exotic':             'exotic_product_pricing',
  'real_options_valuation':                 'real_option_valuation',
  'real_options_decision_tree':             'decision_tree_analysis',

  // ── Quant-risk analyst ──────────────────────────────────────────
  'quant_risk_var_calculation':             'tail_risk_analysis',
  'quant_risk_expected_shortfall':          'tail_risk_analysis',
  'quant_risk_factor_analysis':             'factor_model',
  'quant_risk_black_litterman':             'black_litterman',
  'portfolio_optimization_mean_variance':   'mean_variance_optimization',
  'portfolio_optimization_black_litterman': 'black_litterman_portfolio',
  'risk_budgeting_risk_parity':             'risk_parity',
  'risk_budgeting_factor':                  'factor_risk_budget',
  'scenarios_stress_test':                  'stress_test',
  'scenarios_sensitivity':                  'sensitivity_matrix',
  'scenarios_analysis':                     'scenario_analysis',
  'performance_attribution_factor_based':   'factor_attribution',
  'quant_strategies_momentum':              'momentum_analysis',
  'quant_strategies_pairs':                 'pairs_trading',
  'index_construction_methodology':         'index_weighting',
  'index_construction_rebalancing':         'index_rebalancing',
  'index_construction_tracking_error':      'tracking_error',
  'index_construction_smart_beta':          'smart_beta',
  'index_construction_reconstitution':      'index_reconstitution',
  'market_microstructure_liquidity':        'spread_analysis',
  'market_microstructure_execution':        'optimal_execution',
  'behavioral_prospect_theory':             'prospect_theory',
  'behavioral_sentiment':                   'market_sentiment',
  'portfolio_risk_adjusted_returns':        'risk_adjusted_returns',
  'portfolio_risk_metrics':                 'risk_metrics',
  'portfolio_kelly_sizing':                 'kelly_sizing',

  // ── Macro analyst ───────────────────────────────────────────────
  'macro_economics_rate_analysis':          'monetary_policy',
  'macro_economics_yield_curve':            'bootstrap_spot_curve',
  'macro_economics_inflation_analysis':     'international_economics',
  'macro_economics_economic_indicators':    'monetary_policy',
  'fx_commodities_currency_analysis':       'fx_forward',
  'fx_commodities_cross_rate':              'cross_rate',
  'fx_commodities_commodity_valuation':     'commodity_forward',
  'fx_commodities_commodity_curve':         'commodity_curve',
  'commodity_trading_price_analysis':       'commodity_spread',
  'commodity_trading_storage':              'storage_economics',
  'emerging_markets_country_risk':          'country_risk_premium',
  'emerging_markets_sovereign_spread':      'em_bond_analysis',
  'emerging_markets_political_risk':        'political_risk',
  'emerging_markets_capital_controls':      'capital_controls',
  'emerging_markets_equity_premium':        'em_equity_premium',
  'inflation_linked_breakeven_rate':        'tips_analytics',
  'sovereign_credit_analysis':              'country_risk_assessment',
  'trade_finance_letter_of_credit':         'letter_of_credit',
  'trade_finance_supply_chain':             'supply_chain_finance',
  'carbon_markets_emission_pricing':        'carbon_credit_pricing',
  'carbon_markets_ets_compliance':          'ets_compliance',
  'carbon_markets_cbam':                    'cbam_analysis',
  'carbon_markets_shadow_price':            'shadow_carbon_price',

  // ── ESG & regulatory analyst ────────────────────────────────────
  'esg_score_calculation':                  'esg_score',
  'esg_materiality_assessment':             'carbon_footprint',
  'esg_green_bond':                         'green_bond',
  'esg_sll_covenants':                      'sll_covenants',
  'carbon_markets_offset_valuation':        'offset_valuation',
  'compliance_check':                       'best_execution',
  'compliance_gips':                        'gips_report',
  'regulatory_capital_requirement':         'regulatory_capital',
  'regulatory_lcr':                         'lcr',
  'regulatory_nsfr':                        'nsfr',
  'regulatory_alm':                         'alm_analysis',
  'regulatory_reporting_requirement':       'aifmd_reporting',
  'regulatory_reporting_sec_cftc':          'sec_cftc_reporting',
  'aml_compliance_risk_assessment':         'kyc_risk_assessment',
  'aml_compliance_transaction_screening':   'sanctions_screening',
  'fatca_crs_classification':               'entity_classification',
  'fatca_crs_reporting_requirement':        'fatca_crs_reporting',
  'tax_treaty_withholding_rate':            'treaty_network',
  'tax_treaty_structure_optimization':      'treaty_structure_optimization',
  'transfer_pricing_arm_length_test':       'intercompany_pricing',
  'transfer_pricing_beps':                  'beps_compliance',
  'substance_requirements_assessment':      'economic_substance',
  'substance_requirements_jurisdiction':    'jurisdiction_substance_test',
  'jurisdiction_us_fund':                   'us_fund_structure',
  'jurisdiction_uk_eu_fund':                'uk_eu_fund_structure',
  'jurisdiction_cayman_fund':               'cayman_fund_structure',
  'jurisdiction_lux_ireland_fund':          'lux_ireland_fund_structure',

  // ── Private markets analyst ─────────────────────────────────────
  'pe_lbo_model':                           'lbo_model',
  'pe_returns_analysis':                    'returns_calculator',
  'pe_debt_schedule':                       'debt_schedule',
  'pe_sources_uses':                        'sources_uses',
  'pe_waterfall':                           'waterfall_calculator',
  'pe_altman_zscore':                       'altman_zscore',
  'venture_valuation':                      'funding_round',
  'venture_dilution_analysis':              'dilution_analysis',
  'venture_convertible_note':               'convertible_note',
  'venture_safe_conversion':                'safe_conversion',
  'ma_accretion_dilution':                  'merger_model',
  'ma_synergy_analysis':                    'merger_model',
  'ma_merger_model':                        'merger_model',
  'infrastructure_project_finance':         'ppp_model',
  'infrastructure_project_model':            'project_finance_model',
  'infrastructure_concession_valuation':    'concession_valuation',
  'real_assets_property_valuation':         'property_valuation',
  'real_assets_cap_rate':                   'property_valuation',
  'private_credit_unitranche':              'unitranche_pricing',
  'private_credit_direct_loan':             'direct_loan',
  'private_credit_syndication':             'syndication_analysis',
  'securitization_abs_cashflow':            'abs_cashflow_model',
  'securitization_waterfall':               'tranching_analysis',
  'clo_analytics_waterfall':                'clo_waterfall',
  'clo_analytics_coverage':                 'clo_coverage_tests',
  'clo_analytics_reinvestment':             'clo_reinvestment',
  'clo_analytics_tranche_analysis':         'clo_tranche_analytics',
  'clo_analytics_scenario':                 'clo_scenario',
  'capital_allocation_optimization':        'euler_allocation',
  'capital_allocation_economic':            'economic_capital',
  'capital_allocation_raroc':               'raroc_calculation',
  'capital_allocation_shapley':             'shapley_allocation',
  'capital_allocation_limit':               'limit_management',
  'fund_of_funds_portfolio_construction':   'fof_portfolio',
  'fund_of_funds_j_curve':                  'j_curve_model',
  'fund_of_funds_commitment_pacing':        'commitment_pacing',
  'fund_of_funds_manager_selection':        'manager_selection',
  'fund_of_funds_secondaries':              'secondaries_pricing',
  'private_wealth_planning':                'wealth_transfer',
  'private_wealth_concentrated_stock':      'concentrated_stock',
  'private_wealth_philanthropic':           'philanthropic_vehicles',
  'private_wealth_direct_indexing':         'direct_indexing',
  'private_wealth_family_governance':       'family_governance',
  'lease_accounting_classification':        'lease_classification',
  'lease_accounting_sale_leaseback':        'sale_leaseback_analysis',

  // ── Cross-cutting / multi-agent tools ───────────────────────────
  'fpa_variance':                           'variance_analysis',
  'fpa_breakeven':                          'breakeven_analysis',
  'fpa_working_capital':                    'working_capital',
  'fpa_rolling_forecast':                   'rolling_forecast',
  'treasury_cash_management':               'cash_management',
  'treasury_hedge_effectiveness':           'hedge_effectiveness',
  'insurance_loss_reserving':               'loss_reserving',
  'insurance_premium_pricing':              'premium_pricing',
  'insurance_combined_ratio':               'combined_ratio',
  'insurance_solvency_scr':                 'solvency_scr',
  'pension_funding_analysis':               'pension_funding',
  'pension_ldi_strategy':                   'ldi_strategy',
  'wealth_retirement_planning':             'retirement_planning',
  'wealth_tax_loss_harvesting':             'tax_loss_harvesting',
  'wealth_estate_planning':                 'estate_planning',
  'jurisdiction_fee_calculator':            'fund_fee_calculator',
  'jurisdiction_gaap_ifrs':                 'gaap_ifrs_reconcile',
  'jurisdiction_withholding_tax':           'withholding_tax',
  'jurisdiction_nav':                       'nav_calculator',
  'jurisdiction_gp_economics':              'gp_economics',
  'jurisdiction_investor_returns':          'investor_net_returns',
  'jurisdiction_ubti':                      'ubti_screening',
  'crypto_token_valuation':                 'token_valuation',
  'crypto_defi_analysis':                   'defi_analysis',
};

/**
 * Domain prefixes used by agent think() methods to namespace tool names.
 * Ordered longest-first within each family to avoid partial-prefix matches.
 */
const DOMAIN_PREFIXES = [
  'valuation_', 'equity_research_', 'earnings_quality_', 'dividend_policy_',
  'credit_scoring_', 'credit_portfolio_', 'credit_derivatives_', 'credit_',
  'fixed_income_', 'interest_rate_models_', 'inflation_linked_',
  'mortgage_analytics_', 'repo_financing_', 'municipal_', 'sovereign_',
  'derivatives_', 'volatility_surface_', 'convertibles_', 'structured_products_',
  'real_options_', 'quant_risk_', 'quant_strategies_', 'portfolio_optimization_',
  'risk_budgeting_', 'market_microstructure_', 'index_construction_',
  'behavioral_', 'scenarios_',
  'macro_economics_', 'fx_commodities_', 'commodity_trading_', 'emerging_markets_',
  'trade_finance_', 'carbon_markets_',
  'esg_', 'regulatory_', 'compliance_', 'aml_compliance_', 'regulatory_reporting_',
  'fatca_crs_', 'substance_requirements_', 'tax_treaty_', 'transfer_pricing_',
  'pe_', 'venture_', 'private_credit_', 'private_wealth_', 'infrastructure_',
  'real_assets_', 'fund_of_funds_', 'clo_analytics_', 'securitization_',
  'ma_', 'capital_allocation_', 'lease_accounting_',
  'financial_forensics_', 'bank_analytics_', 'restructuring_',
  'performance_attribution_', 'treasury_', 'insurance_', 'pension_',
  'fpa_', 'jurisdiction_', 'crypto_', 'wealth_', 'portfolio_',
];

/**
 * Resolve an agent-constructed tool name to its MCP-registered equivalent.
 *
 * Resolution order:
 * 1. Exact match — agentName is already a valid MCP tool name
 * 2. Static map — explicit override from AGENT_TO_MCP
 * 3. Prefix stripping — remove known domain prefix and check MCP registry
 * 4. Fallback — return original (will produce "tool not found" from MCP)
 */
export function resolveToolName(
  agentName: string,
  availableTools?: Set<string>,
): string {
  // 1. Already a valid MCP tool name
  if (availableTools?.has(agentName)) return agentName;

  // 2. Static override
  const mapped = AGENT_TO_MCP[agentName];
  if (mapped) return mapped;

  // 3. Prefix stripping fallback
  if (availableTools) {
    for (const prefix of DOMAIN_PREFIXES) {
      if (agentName.startsWith(prefix)) {
        const stripped = agentName.slice(prefix.length);
        if (availableTools.has(stripped)) return stripped;
      }
    }
  }

  // 4. No match — return original
  return agentName;
}

/**
 * Reverse lookup: given an MCP tool name, find the agent name(s) that map to it.
 * Useful for debugging.
 */
export function reverseResolve(mcpName: string): string[] {
  return Object.entries(AGENT_TO_MCP)
    .filter(([, v]) => v === mcpName)
    .map(([k]) => k);
}
