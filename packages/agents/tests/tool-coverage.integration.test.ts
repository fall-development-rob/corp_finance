// Integration test — validates full MCP tool coverage across all CFA agents
// Requires: MCP server built (packages/mcp-server/dist/index.js)

import { describe, it, expect, beforeAll, afterAll } from 'vitest';
import { resolveToolName } from '../config/tool-name-resolver.js';
import { McpBridge, createToolCaller } from '../bridge/mcp-client.js';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

const __dirname = dirname(fileURLToPath(import.meta.url));
const mcpServerPath = join(__dirname, '..', '..', 'mcp-server', 'dist', 'index.js');

let serverExists = false;
try {
  const { existsSync } = await import('node:fs');
  serverExists = existsSync(mcpServerPath);
} catch {
  serverExists = false;
}

// All 215 MCP tool names
const ALL_MCP_TOOLS = new Set([
  'wacc_calculator', 'dcf_model', 'comps_analysis', 'sotp_valuation', 'target_price',
  'credit_metrics', 'debt_capacity', 'covenant_compliance',
  'credit_scorecard', 'merton_pd', 'intensity_model', 'pd_calibration', 'scoring_validation',
  'bond_pricer', 'bond_yield', 'bootstrap_spot_curve', 'nelson_siegel_fit', 'bond_duration', 'credit_spreads',
  'option_pricer', 'implied_volatility', 'forward_pricer', 'forward_position_value',
  'futures_basis_analysis', 'interest_rate_swap', 'currency_swap', 'option_strategy',
  'three_statement_model', 'monte_carlo_simulation', 'monte_carlo_dcf',
  'factor_model', 'black_litterman', 'risk_parity', 'stress_test',
  'sensitivity_matrix', 'scenario_analysis',
  'recovery_analysis', 'distressed_debt_analysis',
  'property_valuation', 'project_finance_model',
  'fx_forward', 'cross_rate', 'commodity_forward', 'commodity_curve',
  'abs_cashflow_model', 'tranching_analysis',
  'funding_round', 'dilution_analysis', 'convertible_note', 'safe_conversion', 'venture_fund_model',
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
  'beneish_mscore', 'piotroski_fscore', 'accrual_quality', 'revenue_quality', 'earnings_quality_composite',
  'commodity_spread', 'storage_economics',
  'returns_calculator', 'debt_schedule', 'sources_uses', 'lbo_model', 'waterfall_calculator', 'altman_zscore',
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
  'h_model_ddm', 'multistage_ddm', 'buyback_analysis', 'payout_sustainability', 'total_shareholder_return',
  'portfolio_credit_risk', 'credit_migration',
  'benfords_law', 'dupont_analysis', 'zscore_models', 'peer_benchmarking', 'red_flag_scoring',
  'nim_analysis', 'camels_rating', 'cecl_provisioning', 'deposit_beta', 'loan_book_analysis',
  'pairs_trading', 'momentum_analysis',
  'index_weighting', 'index_rebalancing', 'tracking_error', 'smart_beta', 'index_reconstitution',
  'spread_analysis', 'optimal_execution',
  'prospect_theory', 'market_sentiment',
  'monetary_policy', 'international_economics',
  'country_risk_premium', 'political_risk', 'capital_controls', 'em_bond_analysis', 'em_equity_premium',
  'risk_adjusted_returns', 'risk_metrics', 'kelly_sizing',
  'variance_analysis', 'breakeven_analysis', 'working_capital', 'rolling_forecast',
  'cash_management', 'hedge_effectiveness',
  'loss_reserving', 'premium_pricing', 'combined_ratio', 'solvency_scr',
  'fund_fee_calculator', 'gaap_ifrs_reconcile', 'withholding_tax', 'nav_calculator',
  'gp_economics', 'investor_net_returns', 'ubti_screening',
  'economic_capital', 'raroc_calculation', 'euler_allocation', 'shapley_allocation', 'limit_management',
  'carbon_credit_pricing', 'ets_compliance', 'cbam_analysis', 'offset_valuation', 'shadow_carbon_price',
  'j_curve_model', 'commitment_pacing', 'manager_selection', 'secondaries_pricing', 'fof_portfolio',
  'concentrated_stock', 'philanthropic_vehicles', 'wealth_transfer', 'direct_indexing', 'family_governance',
  'best_execution', 'gips_report',
  'kyc_risk_assessment', 'sanctions_screening',
  'fatca_crs_reporting', 'entity_classification',
  'treaty_network', 'treaty_structure_optimization',
  'intercompany_pricing', 'beps_compliance',
  'economic_substance', 'jurisdiction_substance_test',
  'aifmd_reporting', 'sec_cftc_reporting',
  'us_fund_structure', 'uk_eu_fund_structure',
  'cayman_fund_structure', 'lux_ireland_fund_structure',
  'clo_waterfall', 'clo_coverage_tests', 'clo_reinvestment', 'clo_tranche_analytics', 'clo_scenario',
]);

describe('Tool Coverage — resolver completeness', () => {
  it('expected 215 MCP tools in reference set', () => {
    expect(ALL_MCP_TOOLS.size).toBe(215);
  });

  it('every MCP tool is resolvable via exact match or AGENT_TO_MCP', () => {
    // Tools that are exact matches (agent can call them by MCP name directly)
    // are automatically resolved via Tier 1. The rest must be in AGENT_TO_MCP.
    // We just verify that calling resolveToolName with the MCP name returns itself.
    for (const tool of ALL_MCP_TOOLS) {
      const resolved = resolveToolName(tool, ALL_MCP_TOOLS);
      expect(resolved).toBe(tool);
    }
  });
});

describe.skipIf(!serverExists)('Tool Coverage — live MCP server', () => {
  let bridge: McpBridge;

  beforeAll(async () => {
    const result = await createToolCaller({ serverPath: mcpServerPath });
    bridge = result.bridge;
  }, 30_000);

  afterAll(async () => {
    if (bridge?.isConnected) {
      await bridge.disconnect();
    }
  });

  it('MCP server exposes all 215 expected tools', async () => {
    const tools = await bridge.listTools();
    const serverTools = new Set(tools.map(t => t.name));

    // Check every expected tool exists on the server
    const missing: string[] = [];
    for (const expected of ALL_MCP_TOOLS) {
      if (!serverTools.has(expected)) {
        missing.push(expected);
      }
    }

    expect(missing).toEqual([]);
    expect(serverTools.size).toBeGreaterThanOrEqual(215);
  }, 15_000);
});
