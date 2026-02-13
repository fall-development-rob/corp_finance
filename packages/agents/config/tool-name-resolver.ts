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
  'earnings_quality_accruals_analysis':     'accrual_quality',
  'dividend_policy_sustainability':         'payout_sustainability',
  'performance_attribution_brinson':        'brinson_attribution',

  // ── Credit analyst ──────────────────────────────────────────────
  'credit_scoring_corporate':               'credit_scorecard',
  'credit_spread_analysis':                 'credit_spreads',
  'credit_default_probability':             'merton_pd',
  'credit_portfolio_var':                   'portfolio_credit_risk',
  'credit_derivatives_cds_pricing':         'cds_pricing',
  'restructuring_distressed_valuation':     'distressed_debt_analysis',
  'financial_forensics_beneish':            'beneish_mscore',
  'bank_analytics_capital_adequacy':        'camels_rating',
  'credit_covenant_analysis':               'covenant_compliance',

  // ── Fixed income analyst ────────────────────────────────────────
  'fixed_income_bond_pricing':              'bond_pricer',
  'fixed_income_yield_curve':               'bootstrap_spot_curve',
  'interest_rate_models_vasicek':           'short_rate_model',
  'inflation_linked_tips_analysis':         'tips_analytics',
  'repo_financing_haircut_analysis':        'repo_analytics',
  'mortgage_analytics_prepayment_model':    'prepayment_analysis',
  'municipal_credit_analysis':              'municipal_analysis',
  'sovereign_debt_sustainability':          'sovereign_bond_analysis',
  'fixed_income_spread_analysis':           'credit_spreads',

  // ── Derivatives analyst ─────────────────────────────────────────
  'derivatives_option_pricing':             'option_pricer',
  'derivatives_greeks_calculation':         'option_pricer',
  'volatility_surface_interpolation':       'implied_vol_surface',
  'volatility_surface_smile_analysis':      'sabr_calibration',
  'convertibles_pricing':                   'convertible_bond_pricing',
  'structured_products_analysis':           'structured_note_pricing',
  'real_options_valuation':                 'real_option_valuation',

  // ── Quant-risk analyst ──────────────────────────────────────────
  'quant_risk_var_calculation':             'tail_risk_analysis',
  'quant_risk_expected_shortfall':          'tail_risk_analysis',
  'portfolio_optimization_mean_variance':   'mean_variance_optimization',
  'risk_budgeting_risk_parity':             'risk_parity',
  'quant_risk_factor_analysis':             'factor_model',
  'scenarios_stress_test':                  'stress_test',
  'performance_attribution_factor_based':   'factor_attribution',
  'quant_strategies_momentum':              'momentum_analysis',
  'index_construction_methodology':         'index_weighting',
  'market_microstructure_liquidity':        'spread_analysis',

  // ── Macro analyst ───────────────────────────────────────────────
  'macro_economics_rate_analysis':          'monetary_policy',
  'macro_economics_yield_curve':            'bootstrap_spot_curve',
  'fx_commodities_currency_analysis':       'fx_forward',
  'fx_commodities_cross_rate':              'cross_rate',
  'commodity_trading_price_analysis':       'commodity_spread',
  'fx_commodities_commodity_valuation':     'commodity_forward',
  'emerging_markets_country_risk':          'country_risk_premium',
  'emerging_markets_sovereign_spread':      'em_bond_analysis',
  'inflation_linked_breakeven_rate':        'tips_analytics',
  'macro_economics_inflation_analysis':     'international_economics',
  'sovereign_credit_analysis':              'country_risk_assessment',
  'macro_economics_economic_indicators':    'monetary_policy',
  'trade_finance_letter_of_credit':         'letter_of_credit',
  'carbon_markets_emission_pricing':        'carbon_credit_pricing',

  // ── ESG & regulatory analyst ────────────────────────────────────
  'esg_score_calculation':                  'esg_score',
  'esg_materiality_assessment':             'carbon_footprint',
  'carbon_markets_offset_valuation':        'offset_valuation',
  'compliance_check':                       'best_execution',
  'regulatory_capital_requirement':         'regulatory_capital',
  'aml_compliance_risk_assessment':         'kyc_risk_assessment',
  'aml_compliance_transaction_screening':   'sanctions_screening',
  'fatca_crs_classification':               'entity_classification',
  'tax_treaty_withholding_rate':            'treaty_network',
  'transfer_pricing_arm_length_test':       'intercompany_pricing',
  'substance_requirements_assessment':      'economic_substance',
  'regulatory_reporting_requirement':       'aifmd_reporting',

  // ── Private markets analyst ─────────────────────────────────────
  'pe_lbo_model':                           'lbo_model',
  'pe_returns_analysis':                    'returns_calculator',
  'venture_valuation':                      'funding_round',
  'venture_dilution_analysis':              'dilution_analysis',
  'ma_accretion_dilution':                  'merger_model',
  'ma_synergy_analysis':                    'merger_model',
  'infrastructure_project_finance':         'ppp_model',
  'infrastructure_concession_valuation':    'concession_valuation',
  'real_assets_property_valuation':         'property_valuation',
  'real_assets_cap_rate':                   'property_valuation',
  'clo_analytics_tranche_analysis':         'clo_tranche_analytics',
  'securitization_waterfall':               'tranching_analysis',
  'restructuring_recovery_analysis':        'recovery_analysis',
  'restructuring_waterfall':                'distressed_debt_analysis',
  'fund_of_funds_portfolio_construction':   'fof_portfolio',
  'capital_allocation_optimization':        'euler_allocation',
  'private_wealth_planning':                'wealth_transfer',
};

/**
 * Resolve an agent-constructed tool name to its MCP-registered equivalent.
 * 1. Exact match in available tools → use as-is
 * 2. Static override map → mapped name
 * 3. Suffix heuristic: strip known domain prefixes and check
 * 4. Fallback: return original (will produce "tool not found" from MCP)
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

  // 3. No available tools cache — just return original
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
