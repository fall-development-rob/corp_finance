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
    }

    return tools;
  }

  protected async reflect(_ctx: AnalystContext, state: ReasoningState) {
    const successCount = state.toolResults.filter(t => !t.error).length;
    const shouldIterate = state.iteration === 1 && successCount > 0 && state.toolResults.length < 4;

    return {
      summary: `Iteration ${state.iteration}: ${successCount}/${state.toolResults.length} tools succeeded`,
      shouldIterate,
    };
  }

  protected async synthesize(_ctx: AnalystContext, state: ReasoningState): Promise<Finding[]> {
    const findings: Finding[] = [];
    const successful = state.toolResults.filter(t => !t.error && t.result);

    for (const invocation of successful) {
      findings.push({
        statement: `${invocation.toolName}: ${JSON.stringify(invocation.result).slice(0, 500)}`,
        supportingData: invocation.result as Record<string, unknown>,
        confidence: 0.8,
        methodology: invocation.toolName.replace(/_/g, ' '),
        citations: [{
          invocationId: invocation.invocationId,
          toolName: invocation.toolName,
          relevantOutput: JSON.stringify(invocation.result).slice(0, 200),
        }],
      });
    }

    if (findings.length === 0) {
      findings.push({
        statement: `Unable to complete analysis — no tool results available`,
        supportingData: {},
        confidence: 0,
        methodology: 'N/A',
        citations: [],
      });
    }

    return findings;
  }
}
