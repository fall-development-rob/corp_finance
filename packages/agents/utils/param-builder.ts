// Build properly-typed tool parameters from extracted financial metrics
// Each builder returns params that match the MCP server's Zod schema

import type { ExtractedMetrics } from './financial-parser.js';
import { resolveToolName } from '../config/tool-name-resolver.js';

// Re-export shared types
export type { ParamQuality } from './param-builders/types.js';
export { trackQuality } from './param-builders/types.js';

type ParamBuilder = (m: ExtractedMetrics) => Record<string, unknown>;

// Import domain-specific builders
import * as valuation from './param-builders/valuation.js';
import * as credit from './param-builders/credit.js';
import * as fixedIncome from './param-builders/fixed-income.js';
import * as derivatives from './param-builders/derivatives.js';
import * as quantRisk from './param-builders/quant-risk.js';
import * as macro from './param-builders/macro.js';
import * as esgRegulatory from './param-builders/esg-regulatory.js';
import * as privateMarkets from './param-builders/private-markets.js';

// ─── Registry ────────────────────────────────────────────────────────────────

const BUILDERS: Record<string, ParamBuilder> = {
  // Valuation
  dcf_model: valuation.dcf_model,
  wacc_calculator: valuation.wacc_calculator,
  comps_analysis: valuation.comps_analysis,
  sotp_valuation: valuation.sotp_valuation,
  // Credit
  credit_metrics: credit.credit_metrics,
  credit_scorecard: credit.credit_scorecard,
  credit_spreads: credit.credit_spreads,
  merton_pd: credit.merton_pd,
  portfolio_credit_risk: credit.portfolio_credit_risk,
  cds_pricing: credit.cds_pricing,
  distressed_debt_analysis: credit.distressed_debt_analysis,
  beneish_mscore: credit.beneish_mscore,
  camels_rating: credit.camels_rating,
  covenant_compliance: credit.covenant_compliance,
  // Fixed Income
  bond_pricer: fixedIncome.bond_pricer,
  bootstrap_spot_curve: fixedIncome.bootstrap_spot_curve,
  short_rate_model: fixedIncome.short_rate_model,
  tips_analytics: fixedIncome.tips_analytics,
  repo_analytics: fixedIncome.repo_analytics,
  prepayment_analysis: fixedIncome.prepayment_analysis,
  municipal_analysis: fixedIncome.municipal_analysis,
  sovereign_bond_analysis: fixedIncome.sovereign_bond_analysis,
  // Derivatives
  option_pricer: derivatives.option_pricer,
  implied_vol_surface: derivatives.implied_vol_surface,
  sabr_calibration: derivatives.sabr_calibration,
  monte_carlo_simulation: derivatives.monte_carlo_simulation,
  convertible_bond_pricing: derivatives.convertible_bond_pricing,
  structured_note_pricing: derivatives.structured_note_pricing,
  real_option_valuation: derivatives.real_option_valuation,
  // Quant Risk
  tail_risk_analysis: quantRisk.tail_risk_analysis,
  mean_variance_optimization: quantRisk.mean_variance_optimization,
  risk_parity: quantRisk.risk_parity,
  factor_model: quantRisk.factor_model,
  stress_test: quantRisk.stress_test,
  factor_attribution: quantRisk.factor_attribution,
  brinson_attribution: quantRisk.brinson_attribution,
  momentum_analysis: quantRisk.momentum_analysis,
  index_weighting: quantRisk.index_weighting,
  spread_analysis: quantRisk.spread_analysis,
  // Macro
  monetary_policy: macro.monetary_policy,
  international_economics: macro.international_economics,
  fx_forward: macro.fx_forward,
  cross_rate: macro.cross_rate,
  commodity_spread: macro.commodity_spread,
  commodity_forward: macro.commodity_forward,
  country_risk_premium: macro.country_risk_premium,
  em_bond_analysis: macro.em_bond_analysis,
  country_risk_assessment: macro.country_risk_assessment,
  letter_of_credit: macro.letter_of_credit,
  carbon_credit_pricing: macro.carbon_credit_pricing,
  // ESG & Regulatory
  esg_score: esgRegulatory.esg_score,
  carbon_footprint: esgRegulatory.carbon_footprint,
  offset_valuation: esgRegulatory.offset_valuation,
  best_execution: esgRegulatory.best_execution,
  regulatory_capital: esgRegulatory.regulatory_capital,
  kyc_risk_assessment: esgRegulatory.kyc_risk_assessment,
  sanctions_screening: esgRegulatory.sanctions_screening,
  entity_classification: esgRegulatory.entity_classification,
  treaty_network: esgRegulatory.treaty_network,
  intercompany_pricing: esgRegulatory.intercompany_pricing,
  economic_substance: esgRegulatory.economic_substance,
  aifmd_reporting: esgRegulatory.aifmd_reporting,
  // Private Markets
  lbo_model: privateMarkets.lbo_model,
  returns_calculator: privateMarkets.returns_calculator,
  funding_round: privateMarkets.funding_round,
  dilution_analysis: privateMarkets.dilution_analysis,
  merger_model: privateMarkets.merger_model,
  ppp_model: privateMarkets.ppp_model,
  concession_valuation: privateMarkets.concession_valuation,
  property_valuation: privateMarkets.property_valuation,
  clo_tranche_analytics: privateMarkets.clo_tranche_analytics,
  tranching_analysis: privateMarkets.tranching_analysis,
  recovery_analysis: privateMarkets.recovery_analysis,
  fof_portfolio: privateMarkets.fof_portfolio,
  euler_allocation: privateMarkets.euler_allocation,
  wealth_transfer: privateMarkets.wealth_transfer,
  // Cross-cutting
  three_statement_model: privateMarkets.three_statement_model,
  payout_sustainability: privateMarkets.payout_sustainability,
  accrual_quality: privateMarkets.accrual_quality,
};

/**
 * Build valid tool parameters for a given tool name using extracted metrics.
 * Accepts both agent-constructed names and MCP names.
 */
export function buildToolParams(
  toolName: string,
  metrics: ExtractedMetrics,
): Record<string, unknown> {
  // Resolve agent name → MCP name
  const mcpName = resolveToolName(toolName);
  const builder = BUILDERS[mcpName];
  if (builder) {
    const params = builder(metrics);
    // Strip undefined values to avoid sending nulls to MCP tools
    return Object.fromEntries(
      Object.entries(params).filter(([_, v]) => v !== undefined)
    );
  }

  // Fallback: return metrics as-is (will likely fail validation but at least has data)
  const { _raw, _company, ...rest } = metrics;
  return rest as Record<string, unknown>;
}
