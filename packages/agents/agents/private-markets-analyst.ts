// Private Markets Specialist — PE, VC, M&A, infrastructure, real assets, CLO, restructuring
// Tool domains: pe, venture, private_credit, private_wealth, infrastructure, real_assets,
//               fund_of_funds, clo_analytics, securitization, ma, capital_allocation, lease_accounting

import { randomUUID } from 'node:crypto';
import type { Finding } from '../types/agents.js';
import { BaseAnalyst, type AnalystContext, type ReasoningState } from './base-analyst.js';
import { buildToolParams } from '../utils/param-builder.js';

export class PrivateMarketsAnalyst extends BaseAnalyst {
  constructor() {
    super('private-markets-analyst');
  }

  protected async think(ctx: AnalystContext, state: ReasoningState) {
    const task = ctx.task.toLowerCase();
    const tools: Array<{ toolName: string; params: Record<string, unknown> }> = [];

    if (state.iteration === 1) {
      if (task.includes('lbo') || task.includes('buyout') || task.includes('leveraged')) {
        tools.push({ toolName: 'pe_lbo_model', params: buildToolParams('pe_lbo_model', state.metrics) });
        tools.push({ toolName: 'pe_returns_analysis', params: buildToolParams('pe_returns_analysis', state.metrics) });
      }
      if (task.includes('venture') || task.includes('startup') || task.includes('vc')) {
        tools.push({ toolName: 'venture_valuation', params: buildToolParams('venture_valuation', state.metrics) });
        tools.push({ toolName: 'venture_dilution_analysis', params: buildToolParams('venture_dilution_analysis', state.metrics) });
      }
      if (task.includes('m&a') || task.includes('merger') || task.includes('acquisition')) {
        tools.push({ toolName: 'ma_accretion_dilution', params: buildToolParams('ma_accretion_dilution', state.metrics) });
        tools.push({ toolName: 'ma_synergy_analysis', params: buildToolParams('ma_synergy_analysis', state.metrics) });
      }
      if (task.includes('infrastructure') || task.includes('project')) {
        tools.push({ toolName: 'infrastructure_project_finance', params: buildToolParams('infrastructure_project_finance', state.metrics) });
        tools.push({ toolName: 'infrastructure_concession_valuation', params: buildToolParams('infrastructure_concession_valuation', state.metrics) });
      }
      if (task.includes('real estate') || task.includes('reit') || task.includes('property')) {
        tools.push({ toolName: 'real_assets_property_valuation', params: buildToolParams('real_assets_property_valuation', state.metrics) });
        tools.push({ toolName: 'real_assets_cap_rate', params: buildToolParams('real_assets_cap_rate', state.metrics) });
      }
      if (task.includes('clo') || task.includes('securitiz')) {
        tools.push({ toolName: 'clo_analytics_tranche_analysis', params: buildToolParams('clo_analytics_tranche_analysis', state.metrics) });
        tools.push({ toolName: 'securitization_waterfall', params: buildToolParams('securitization_waterfall', state.metrics) });
      }
      if (task.includes('restructur') || task.includes('distress')) {
        tools.push({ toolName: 'restructuring_recovery_analysis', params: buildToolParams('restructuring_recovery_analysis', state.metrics) });
        tools.push({ toolName: 'restructuring_waterfall', params: buildToolParams('restructuring_waterfall', state.metrics) });
      }
      if (task.includes('debt schedule') || task.includes('amortization')) {
        tools.push({ toolName: 'pe_debt_schedule', params: buildToolParams('pe_debt_schedule', state.metrics) });
      }
      if (task.includes('sources') || task.includes('uses of funds')) {
        tools.push({ toolName: 'pe_sources_uses', params: buildToolParams('pe_sources_uses', state.metrics) });
      }
      if (task.includes('waterfall') || task.includes('distribution') || task.includes('carry')) {
        tools.push({ toolName: 'pe_waterfall', params: buildToolParams('pe_waterfall', state.metrics) });
      }
      if (task.includes('altman') || task.includes('z-score')) {
        tools.push({ toolName: 'pe_altman_zscore', params: buildToolParams('pe_altman_zscore', state.metrics) });
      }
      if (task.includes('convertible note') || task.includes('cap')) {
        tools.push({ toolName: 'venture_convertible_note', params: buildToolParams('venture_convertible_note', state.metrics) });
      }
      if (task.includes('safe') || task.includes('post-money')) {
        tools.push({ toolName: 'venture_safe_conversion', params: buildToolParams('venture_safe_conversion', state.metrics) });
      }
      if (task.includes('unitranche') || task.includes('folo')) {
        tools.push({ toolName: 'private_credit_unitranche', params: buildToolParams('private_credit_unitranche', state.metrics) });
      }
      if (task.includes('direct lend') || task.includes('pik')) {
        tools.push({ toolName: 'private_credit_direct_loan', params: buildToolParams('private_credit_direct_loan', state.metrics) });
      }
      if (task.includes('syndicat')) {
        tools.push({ toolName: 'private_credit_syndication', params: buildToolParams('private_credit_syndication', state.metrics) });
      }
      // Default: PE + M&A overview
      if (tools.length === 0) {
        tools.push({ toolName: 'pe_lbo_model', params: buildToolParams('pe_lbo_model', state.metrics) });
        tools.push({ toolName: 'ma_accretion_dilution', params: buildToolParams('ma_accretion_dilution', state.metrics) });
      }
    } else {
      // Iteration 2+: fund-of-funds, capital allocation, private wealth
      if (task.includes('fund') || task.includes('fof') || task.includes('allocation')) {
        tools.push({ toolName: 'fund_of_funds_portfolio_construction', params: buildToolParams('fund_of_funds_portfolio_construction', state.metrics) });
        tools.push({ toolName: 'capital_allocation_optimization', params: buildToolParams('capital_allocation_optimization', state.metrics) });
      }
      if (task.includes('wealth') || task.includes('family') || task.includes('estate')) {
        tools.push({ toolName: 'private_wealth_planning', params: buildToolParams('private_wealth_planning', state.metrics) });
      }
      if (task.includes('abs') || task.includes('asset-backed') || task.includes('securitiz')) {
        tools.push({ toolName: 'securitization_abs_cashflow', params: buildToolParams('securitization_abs_cashflow', state.metrics) });
      }
      if (task.includes('clo waterfall') || task.includes('payment priority')) {
        tools.push({ toolName: 'clo_analytics_waterfall', params: buildToolParams('clo_analytics_waterfall', state.metrics) });
      }
      if (task.includes('coverage test') || task.includes('oc test') || task.includes('ic test')) {
        tools.push({ toolName: 'clo_analytics_coverage', params: buildToolParams('clo_analytics_coverage', state.metrics) });
      }
      if (task.includes('reinvestment') || task.includes('ramp')) {
        tools.push({ toolName: 'clo_analytics_reinvestment', params: buildToolParams('clo_analytics_reinvestment', state.metrics) });
      }
      if (task.includes('clo scenario') || task.includes('clo stress')) {
        tools.push({ toolName: 'clo_analytics_scenario', params: buildToolParams('clo_analytics_scenario', state.metrics) });
      }
      if (task.includes('economic capital')) {
        tools.push({ toolName: 'capital_allocation_economic', params: buildToolParams('capital_allocation_economic', state.metrics) });
      }
      if (task.includes('raroc') || task.includes('rorac')) {
        tools.push({ toolName: 'capital_allocation_raroc', params: buildToolParams('capital_allocation_raroc', state.metrics) });
      }
      if (task.includes('shapley') || task.includes('marginal contribution')) {
        tools.push({ toolName: 'capital_allocation_shapley', params: buildToolParams('capital_allocation_shapley', state.metrics) });
      }
      if (task.includes('limit') || task.includes('exposure limit')) {
        tools.push({ toolName: 'capital_allocation_limit', params: buildToolParams('capital_allocation_limit', state.metrics) });
      }
      if (task.includes('j-curve') || task.includes('j curve')) {
        tools.push({ toolName: 'fund_of_funds_j_curve', params: buildToolParams('fund_of_funds_j_curve', state.metrics) });
      }
      if (task.includes('commitment') || task.includes('pacing')) {
        tools.push({ toolName: 'fund_of_funds_commitment_pacing', params: buildToolParams('fund_of_funds_commitment_pacing', state.metrics) });
      }
      if (task.includes('manager selection') || task.includes('due diligence')) {
        tools.push({ toolName: 'fund_of_funds_manager_selection', params: buildToolParams('fund_of_funds_manager_selection', state.metrics) });
      }
      if (task.includes('secondar') || task.includes('discount')) {
        tools.push({ toolName: 'fund_of_funds_secondaries', params: buildToolParams('fund_of_funds_secondaries', state.metrics) });
      }
      if (task.includes('concentrated') || task.includes('single stock')) {
        tools.push({ toolName: 'private_wealth_concentrated_stock', params: buildToolParams('private_wealth_concentrated_stock', state.metrics) });
      }
      if (task.includes('philanthropi') || task.includes('foundation') || task.includes('daf')) {
        tools.push({ toolName: 'private_wealth_philanthropic', params: buildToolParams('private_wealth_philanthropic', state.metrics) });
      }
      if (task.includes('direct index') || task.includes('tax alpha')) {
        tools.push({ toolName: 'private_wealth_direct_indexing', params: buildToolParams('private_wealth_direct_indexing', state.metrics) });
      }
      if (task.includes('family') || task.includes('succession') || task.includes('governance')) {
        tools.push({ toolName: 'private_wealth_family_governance', params: buildToolParams('private_wealth_family_governance', state.metrics) });
      }
      if (task.includes('lease classif') || task.includes('rou') || task.includes('right of use')) {
        tools.push({ toolName: 'lease_accounting_classification', params: buildToolParams('lease_accounting_classification', state.metrics) });
      }
      if (task.includes('sale-leaseback') || task.includes('sale leaseback')) {
        tools.push({ toolName: 'lease_accounting_sale_leaseback', params: buildToolParams('lease_accounting_sale_leaseback', state.metrics) });
      }
      if (task.includes('decision tree') || task.includes('expand option')) {
        tools.push({ toolName: 'real_options_decision_tree', params: buildToolParams('real_options_decision_tree', state.metrics) });
      }
    }

    return tools;
  }

  protected async reflect(_ctx: AnalystContext, state: ReasoningState) {
    const successCount = state.toolResults.filter(t => !t.error).length;
    const failCount = state.toolResults.filter(t => !!t.error).length;
    const hasResults = successCount > 0;

    // Iterate if: first pass, got some results, and either had failures or few successes
    const shouldIterate = state.iteration === 1
      && hasResults
      && (failCount > 0 || successCount < 3);

    const summary = failCount > 0
      ? `Iteration ${state.iteration}: ${successCount} succeeded, ${failCount} failed — will retry with alternatives`
      : `Iteration ${state.iteration}: ${successCount} tools succeeded`;

    return { summary, shouldIterate };
  }

  protected async synthesize(_ctx: AnalystContext, state: ReasoningState): Promise<Finding[]> {
    const findings: Finding[] = [];
    const successful = state.toolResults.filter(t => !t.error && t.result);

    for (const invocation of successful) {
      const statement = this.extractStatement(invocation.toolName, invocation.result);
      const confidence = this.assessFindingConfidence(invocation, state);

      findings.push({
        statement,
        supportingData: invocation.result as Record<string, unknown>,
        confidence,
        methodology: invocation.toolName.replace(/_/g, ' '),
        citations: [{
          invocationId: invocation.invocationId,
          toolName: invocation.toolName,
          relevantOutput: statement.slice(0, 200),
        }],
      });
    }

    if (findings.length === 0) {
      findings.push({
        statement: `Unable to complete ${this.agentType.replace(/-/g, ' ')} — no tool results available`,
        supportingData: {},
        confidence: 0,
        methodology: 'N/A',
        citations: [],
      });
    }

    return findings;
  }

  /** Extract a human-readable statement from tool output */
  private extractStatement(toolName: string, result: unknown): string {
    if (!result || typeof result !== 'object') {
      return `${toolName}: ${String(result).slice(0, 300)}`;
    }

    const data = result as Record<string, unknown>;

    // Try to find the most meaningful fields in the result
    const keyMetrics: string[] = [];

    // Extract numeric results with labels
    for (const [key, val] of Object.entries(data)) {
      if (typeof val === 'number' && !isNaN(val)) {
        const label = key.replace(/_/g, ' ');
        if (Math.abs(val) >= 1e6) {
          keyMetrics.push(`${label}: $${(val / 1e6).toFixed(1)}M`);
        } else if (Math.abs(val) < 1 && val !== 0) {
          keyMetrics.push(`${label}: ${(val * 100).toFixed(2)}%`);
        } else {
          keyMetrics.push(`${label}: ${val.toFixed(2)}`);
        }
      } else if (typeof val === 'string' && val.length < 100) {
        keyMetrics.push(`${key.replace(/_/g, ' ')}: ${val}`);
      }
      if (keyMetrics.length >= 6) break; // Limit output
    }

    const methodLabel = toolName.replace(/_/g, ' ');
    if (keyMetrics.length > 0) {
      return `${methodLabel}: ${keyMetrics.join(', ')}`;
    }
    return `${methodLabel}: ${JSON.stringify(data).slice(0, 300)}`;
  }

  /** Assess confidence for a single finding based on data quality */
  private assessFindingConfidence(invocation: { duration?: number; error?: string }, state: ReasoningState): number {
    let confidence = 0.75; // base confidence for any successful tool call

    // Boost for fast responses (likely cached or complete data)
    if (invocation.duration !== undefined && invocation.duration < 5000) {
      confidence += 0.1;
    }

    // Boost for FMP-enriched data
    if (state.metrics._dataSource === 'fmp-enriched') {
      confidence += 0.1;
    }

    // Slight penalty for text-only data
    if (!state.metrics._dataSource || state.metrics._dataSource === 'text-only') {
      confidence -= 0.1;
    }

    return Math.min(1, Math.max(0, Math.round(confidence * 100) / 100));
  }
}
