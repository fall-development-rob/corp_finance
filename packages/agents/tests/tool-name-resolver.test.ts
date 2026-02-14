import { describe, it, expect } from 'vitest';
import { resolveToolName, reverseResolve } from '../config/tool-name-resolver.js';

describe('Tool name resolver', () => {
  // All 215 MCP-registered tool names
  const mcpTools = new Set([
    'wacc_calculator', 'dcf_model', 'comps_analysis', 'sotp_valuation', 'target_price',
    'credit_metrics', 'debt_capacity', 'covenant_compliance', 'credit_scorecard',
    'merton_pd', 'intensity_model', 'pd_calibration', 'scoring_validation',
    'bond_pricer', 'bond_yield', 'bootstrap_spot_curve', 'nelson_siegel_fit',
    'bond_duration', 'credit_spreads',
    'option_pricer', 'implied_volatility', 'forward_pricer', 'forward_position_value',
    'futures_basis_analysis', 'interest_rate_swap', 'currency_swap', 'option_strategy',
    'three_statement_model', 'monte_carlo_simulation', 'monte_carlo_dcf',
    'factor_model', 'black_litterman', 'risk_parity', 'stress_test',
    'sensitivity_matrix', 'scenario_analysis',
    'recovery_analysis', 'distressed_debt_analysis',
    'property_valuation', 'project_finance_model',
    'fx_forward', 'cross_rate', 'commodity_forward', 'commodity_curve',
    'abs_cashflow_model', 'tranching_analysis',
    'funding_round', 'dilution_analysis', 'convertible_note', 'safe_conversion',
    'venture_fund_model',
    'esg_score', 'carbon_footprint', 'green_bond', 'sll_covenants',
    'regulatory_capital', 'lcr', 'nsfr', 'alm_analysis',
    'unitranche_pricing', 'direct_loan', 'syndication_analysis',
    'retirement_planning', 'tax_loss_harvesting', 'estate_planning',
    'token_valuation', 'defi_analysis',
    'muni_bond_pricing', 'municipal_analysis',
    'structured_note_pricing', 'exotic_product_pricing',
    'letter_of_credit', 'supply_chain_finance',
    'cds_pricing', 'cva_calculation',
    'convertible_bond_pricing', 'convertible_bond_analysis',
    'lease_classification', 'sale_leaseback_analysis',
    'pension_funding', 'ldi_strategy',
    'sovereign_bond_analysis', 'country_risk_assessment',
    'real_option_valuation', 'decision_tree_analysis',
    'beneish_mscore', 'piotroski_fscore', 'accrual_quality', 'revenue_quality',
    'earnings_quality_composite',
    'commodity_spread', 'storage_economics',
    'returns_calculator', 'debt_schedule', 'sources_uses', 'lbo_model',
    'waterfall_calculator', 'altman_zscore',
    'ppp_model', 'concession_valuation',
    'merger_model',
    'mean_variance_optimization', 'black_litterman_portfolio',
    'factor_risk_budget', 'tail_risk_analysis',
    'brinson_attribution', 'factor_attribution',
    'implied_vol_surface', 'sabr_calibration',
    'short_rate_model', 'term_structure_fit',
    'prepayment_analysis', 'mbs_analytics',
    'tips_analytics', 'inflation_derivatives',
    'repo_analytics', 'collateral_analytics',
    'h_model_ddm', 'multistage_ddm', 'buyback_analysis', 'payout_sustainability',
    'total_shareholder_return',
    'portfolio_credit_risk', 'credit_migration',
    'benfords_law', 'dupont_analysis', 'zscore_models', 'peer_benchmarking',
    'red_flag_scoring',
    'nim_analysis', 'camels_rating', 'cecl_provisioning', 'deposit_beta',
    'loan_book_analysis',
    'pairs_trading', 'momentum_analysis',
    'index_weighting', 'index_rebalancing', 'tracking_error', 'smart_beta',
    'index_reconstitution',
    'spread_analysis', 'optimal_execution',
    'prospect_theory', 'market_sentiment',
    'monetary_policy', 'international_economics',
    'country_risk_premium', 'political_risk', 'capital_controls',
    'em_bond_analysis', 'em_equity_premium',
    'risk_adjusted_returns', 'risk_metrics', 'kelly_sizing',
    'variance_analysis', 'breakeven_analysis', 'working_capital', 'rolling_forecast',
    'cash_management', 'hedge_effectiveness',
    'loss_reserving', 'premium_pricing', 'combined_ratio', 'solvency_scr',
    'fund_fee_calculator', 'gaap_ifrs_reconcile', 'withholding_tax', 'nav_calculator',
    'gp_economics', 'investor_net_returns', 'ubti_screening',
    'economic_capital', 'raroc_calculation', 'euler_allocation', 'shapley_allocation',
    'limit_management',
    'carbon_credit_pricing', 'ets_compliance', 'cbam_analysis', 'offset_valuation',
    'shadow_carbon_price',
    'j_curve_model', 'commitment_pacing', 'manager_selection', 'secondaries_pricing',
    'fof_portfolio',
    'concentrated_stock', 'philanthropic_vehicles', 'wealth_transfer',
    'direct_indexing', 'family_governance',
    'best_execution', 'gips_report',
    'kyc_risk_assessment', 'sanctions_screening',
    'fatca_crs_reporting', 'entity_classification',
    'treaty_network', 'treaty_structure_optimization',
    'intercompany_pricing', 'beps_compliance',
    'economic_substance', 'jurisdiction_substance_test',
    'aifmd_reporting', 'sec_cftc_reporting',
    'us_fund_structure', 'uk_eu_fund_structure',
    'cayman_fund_structure', 'lux_ireland_fund_structure',
    'clo_waterfall', 'clo_coverage_tests', 'clo_reinvestment',
    'clo_tranche_analytics', 'clo_scenario',
  ]);

  describe('exact matches pass through', () => {
    it('returns three_statement_model as-is', () => {
      expect(resolveToolName('three_statement_model', mcpTools)).toBe('three_statement_model');
    });

    it('returns monte_carlo_simulation as-is', () => {
      expect(resolveToolName('monte_carlo_simulation', mcpTools)).toBe('monte_carlo_simulation');
    });
  });

  describe('equity analyst mappings', () => {
    const cases: [string, string][] = [
      ['valuation_dcf_model', 'dcf_model'],
      ['valuation_wacc_calculation', 'wacc_calculator'],
      ['valuation_comparable_companies', 'comps_analysis'],
      ['equity_research_fundamental_analysis', 'sotp_valuation'],
      ['earnings_quality_accruals_analysis', 'accrual_quality'],
      ['dividend_policy_sustainability', 'payout_sustainability'],
      ['performance_attribution_brinson', 'brinson_attribution'],
      ['valuation_sotp_valuation', 'sotp_valuation'],
      ['valuation_target_price', 'target_price'],
      ['dividend_policy_h_model', 'h_model_ddm'],
      ['dividend_policy_buyback', 'buyback_analysis'],
      ['dividend_policy_total_shareholder_return', 'total_shareholder_return'],
      ['earnings_quality_piotroski', 'piotroski_fscore'],
      ['earnings_quality_revenue', 'revenue_quality'],
      ['earnings_quality_composite', 'earnings_quality_composite'],
      ['financial_forensics_dupont', 'dupont_analysis'],
      ['financial_forensics_peer_benchmarking', 'peer_benchmarking'],
      ['financial_forensics_red_flags', 'red_flag_scoring'],
      ['financial_forensics_benfords_law', 'benfords_law'],
      ['financial_forensics_zscore', 'zscore_models'],
    ];
    it.each(cases)('%s → %s', (agent, mcp) => {
      expect(resolveToolName(agent, mcpTools)).toBe(mcp);
    });
  });

  describe('credit analyst mappings', () => {
    const cases: [string, string][] = [
      ['credit_scoring_corporate', 'credit_scorecard'],
      ['credit_spread_analysis', 'credit_spreads'],
      ['credit_default_probability', 'merton_pd'],
      ['credit_portfolio_var', 'portfolio_credit_risk'],
      ['credit_derivatives_cds_pricing', 'cds_pricing'],
      ['restructuring_distressed_valuation', 'distressed_debt_analysis'],
      ['financial_forensics_beneish', 'beneish_mscore'],
      ['bank_analytics_capital_adequacy', 'camels_rating'],
      ['credit_covenant_analysis', 'covenant_compliance'],
    ];
    it.each(cases)('%s → %s', (agent, mcp) => {
      expect(resolveToolName(agent, mcpTools)).toBe(mcp);
    });
  });

  describe('fixed income analyst mappings', () => {
    const cases: [string, string][] = [
      ['fixed_income_bond_pricing', 'bond_pricer'],
      ['fixed_income_yield_curve', 'bootstrap_spot_curve'],
      ['interest_rate_models_vasicek', 'short_rate_model'],
      ['inflation_linked_tips_analysis', 'tips_analytics'],
      ['repo_financing_haircut_analysis', 'repo_analytics'],
      ['mortgage_analytics_prepayment_model', 'prepayment_analysis'],
      ['municipal_credit_analysis', 'municipal_analysis'],
      ['sovereign_debt_sustainability', 'sovereign_bond_analysis'],
      ['fixed_income_spread_analysis', 'credit_spreads'],
    ];
    it.each(cases)('%s → %s', (agent, mcp) => {
      expect(resolveToolName(agent, mcpTools)).toBe(mcp);
    });
  });

  describe('derivatives analyst mappings', () => {
    const cases: [string, string][] = [
      ['derivatives_option_pricing', 'option_pricer'],
      ['derivatives_greeks_calculation', 'option_pricer'],
      ['volatility_surface_interpolation', 'implied_vol_surface'],
      ['volatility_surface_smile_analysis', 'sabr_calibration'],
      ['convertibles_pricing', 'convertible_bond_pricing'],
      ['structured_products_analysis', 'structured_note_pricing'],
      ['real_options_valuation', 'real_option_valuation'],
    ];
    it.each(cases)('%s → %s', (agent, mcp) => {
      expect(resolveToolName(agent, mcpTools)).toBe(mcp);
    });
  });

  describe('quant-risk analyst mappings', () => {
    const cases: [string, string][] = [
      ['quant_risk_var_calculation', 'tail_risk_analysis'],
      ['quant_risk_expected_shortfall', 'tail_risk_analysis'],
      ['portfolio_optimization_mean_variance', 'mean_variance_optimization'],
      ['risk_budgeting_risk_parity', 'risk_parity'],
      ['quant_risk_factor_analysis', 'factor_model'],
      ['scenarios_stress_test', 'stress_test'],
      ['performance_attribution_factor_based', 'factor_attribution'],
      ['quant_strategies_momentum', 'momentum_analysis'],
      ['index_construction_methodology', 'index_weighting'],
      ['market_microstructure_liquidity', 'spread_analysis'],
    ];
    it.each(cases)('%s → %s', (agent, mcp) => {
      expect(resolveToolName(agent, mcpTools)).toBe(mcp);
    });
  });

  describe('macro analyst mappings', () => {
    const cases: [string, string][] = [
      ['macro_economics_rate_analysis', 'monetary_policy'],
      ['macro_economics_yield_curve', 'bootstrap_spot_curve'],
      ['fx_commodities_currency_analysis', 'fx_forward'],
      ['fx_commodities_cross_rate', 'cross_rate'],
      ['commodity_trading_price_analysis', 'commodity_spread'],
      ['fx_commodities_commodity_valuation', 'commodity_forward'],
      ['emerging_markets_country_risk', 'country_risk_premium'],
      ['emerging_markets_sovereign_spread', 'em_bond_analysis'],
      ['inflation_linked_breakeven_rate', 'tips_analytics'],
      ['macro_economics_inflation_analysis', 'international_economics'],
      ['sovereign_credit_analysis', 'country_risk_assessment'],
      ['macro_economics_economic_indicators', 'monetary_policy'],
      ['trade_finance_letter_of_credit', 'letter_of_credit'],
      ['carbon_markets_emission_pricing', 'carbon_credit_pricing'],
    ];
    it.each(cases)('%s → %s', (agent, mcp) => {
      expect(resolveToolName(agent, mcpTools)).toBe(mcp);
    });
  });

  describe('ESG & regulatory analyst mappings', () => {
    const cases: [string, string][] = [
      ['esg_score_calculation', 'esg_score'],
      ['esg_materiality_assessment', 'carbon_footprint'],
      ['carbon_markets_offset_valuation', 'offset_valuation'],
      ['compliance_check', 'best_execution'],
      ['regulatory_capital_requirement', 'regulatory_capital'],
      ['aml_compliance_risk_assessment', 'kyc_risk_assessment'],
      ['aml_compliance_transaction_screening', 'sanctions_screening'],
      ['fatca_crs_classification', 'entity_classification'],
      ['tax_treaty_withholding_rate', 'treaty_network'],
      ['transfer_pricing_arm_length_test', 'intercompany_pricing'],
      ['substance_requirements_assessment', 'economic_substance'],
      ['regulatory_reporting_requirement', 'aifmd_reporting'],
    ];
    it.each(cases)('%s → %s', (agent, mcp) => {
      expect(resolveToolName(agent, mcpTools)).toBe(mcp);
    });
  });

  describe('private markets analyst mappings', () => {
    const cases: [string, string][] = [
      ['pe_lbo_model', 'lbo_model'],
      ['pe_returns_analysis', 'returns_calculator'],
      ['venture_valuation', 'funding_round'],
      ['venture_dilution_analysis', 'dilution_analysis'],
      ['ma_accretion_dilution', 'merger_model'],
      ['ma_synergy_analysis', 'merger_model'],
      ['infrastructure_project_finance', 'ppp_model'],
      ['infrastructure_concession_valuation', 'concession_valuation'],
      ['real_assets_property_valuation', 'property_valuation'],
      ['real_assets_cap_rate', 'property_valuation'],
      ['clo_analytics_tranche_analysis', 'clo_tranche_analytics'],
      ['securitization_waterfall', 'tranching_analysis'],
      ['restructuring_recovery_analysis', 'recovery_analysis'],
      ['restructuring_waterfall', 'distressed_debt_analysis'],
      ['fund_of_funds_portfolio_construction', 'fof_portfolio'],
      ['capital_allocation_optimization', 'euler_allocation'],
      ['private_wealth_planning', 'wealth_transfer'],
    ];
    it.each(cases)('%s → %s', (agent, mcp) => {
      expect(resolveToolName(agent, mcpTools)).toBe(mcp);
    });
  });

  describe('prefix stripping fallback', () => {
    it('strips known prefix when no static mapping exists', () => {
      const tools = new Set(['some_new_tool']);
      expect(resolveToolName('valuation_some_new_tool', tools)).toBe('some_new_tool');
    });

    it('prefers static map over prefix stripping', () => {
      expect(resolveToolName('valuation_dcf_model', mcpTools)).toBe('dcf_model');
    });

    it('returns original if prefix strip yields no match', () => {
      const tools = new Set(['bond_pricer']);
      expect(resolveToolName('valuation_nonexistent', tools)).toBe('valuation_nonexistent');
    });
  });

  describe('complete coverage', () => {
    it('all AGENT_TO_MCP targets exist in MCP tools', () => {
      const allAgentNames = [
        // Equity analyst
        'valuation_dcf_model', 'valuation_wacc_calculation', 'valuation_comparable_companies',
        'equity_research_fundamental_analysis', 'earnings_quality_accruals_analysis',
        'dividend_policy_sustainability', 'performance_attribution_brinson',
        'valuation_sotp_valuation', 'valuation_target_price',
        'dividend_policy_h_model', 'dividend_policy_buyback',
        'dividend_policy_total_shareholder_return',
        'earnings_quality_piotroski', 'earnings_quality_revenue', 'earnings_quality_composite',
        'financial_forensics_dupont', 'financial_forensics_peer_benchmarking',
        'financial_forensics_red_flags', 'financial_forensics_benfords_law',
        'financial_forensics_zscore',
        // Credit analyst
        'credit_scoring_corporate', 'credit_spread_analysis', 'credit_default_probability',
        'credit_portfolio_var', 'credit_derivatives_cds_pricing', 'restructuring_distressed_valuation',
        'financial_forensics_beneish', 'bank_analytics_capital_adequacy', 'credit_covenant_analysis',
        // Fixed income analyst
        'fixed_income_bond_pricing', 'fixed_income_yield_curve', 'interest_rate_models_vasicek',
        'inflation_linked_tips_analysis', 'repo_financing_haircut_analysis',
        'mortgage_analytics_prepayment_model', 'municipal_credit_analysis',
        'sovereign_debt_sustainability', 'fixed_income_spread_analysis',
        // Derivatives analyst
        'derivatives_option_pricing', 'derivatives_greeks_calculation',
        'volatility_surface_interpolation', 'volatility_surface_smile_analysis',
        'convertibles_pricing', 'structured_products_analysis', 'real_options_valuation',
        // Quant-risk analyst
        'quant_risk_var_calculation', 'quant_risk_expected_shortfall',
        'portfolio_optimization_mean_variance', 'risk_budgeting_risk_parity',
        'quant_risk_factor_analysis', 'scenarios_stress_test',
        'performance_attribution_factor_based', 'quant_strategies_momentum',
        'index_construction_methodology', 'market_microstructure_liquidity',
        // Macro analyst
        'macro_economics_rate_analysis', 'macro_economics_yield_curve',
        'fx_commodities_currency_analysis', 'fx_commodities_cross_rate',
        'commodity_trading_price_analysis', 'fx_commodities_commodity_valuation',
        'emerging_markets_country_risk', 'emerging_markets_sovereign_spread',
        'inflation_linked_breakeven_rate', 'macro_economics_inflation_analysis',
        'sovereign_credit_analysis', 'macro_economics_economic_indicators',
        'trade_finance_letter_of_credit', 'carbon_markets_emission_pricing',
        // ESG & regulatory analyst
        'esg_score_calculation', 'esg_materiality_assessment',
        'carbon_markets_offset_valuation', 'compliance_check',
        'regulatory_capital_requirement', 'aml_compliance_risk_assessment',
        'aml_compliance_transaction_screening', 'fatca_crs_classification',
        'tax_treaty_withholding_rate', 'transfer_pricing_arm_length_test',
        'substance_requirements_assessment', 'regulatory_reporting_requirement',
        // Private markets analyst
        'pe_lbo_model', 'pe_returns_analysis', 'venture_valuation',
        'venture_dilution_analysis', 'ma_accretion_dilution', 'ma_synergy_analysis',
        'infrastructure_project_finance', 'infrastructure_concession_valuation',
        'real_assets_property_valuation', 'real_assets_cap_rate',
        'clo_analytics_tranche_analysis', 'securitization_waterfall',
        'restructuring_recovery_analysis', 'restructuring_waterfall',
        'fund_of_funds_portfolio_construction', 'capital_allocation_optimization',
        'private_wealth_planning',
      ];

      for (const agentName of allAgentNames) {
        const resolved = resolveToolName(agentName, mcpTools);
        expect(mcpTools.has(resolved), `${agentName} → ${resolved} not in MCP tools`).toBe(true);
      }
    });
  });

  describe('reverseResolve', () => {
    it('finds agent names for a given MCP name', () => {
      const agents = reverseResolve('option_pricer');
      expect(agents).toContain('derivatives_option_pricing');
      expect(agents).toContain('derivatives_greeks_calculation');
    });

    it('returns empty array for unresolvable names', () => {
      expect(reverseResolve('nonexistent_tool')).toEqual([]);
    });
  });

  describe('unknown names pass through', () => {
    it('returns unknown name unchanged', () => {
      expect(resolveToolName('unknown_tool_xyz')).toBe('unknown_tool_xyz');
    });
  });
});
