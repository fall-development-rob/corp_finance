// Private Markets Specialist — PE, VC, M&A, infrastructure, real assets, CLO, restructuring
// Tool domains: pe, venture, private_credit, private_wealth, infrastructure, real_assets,
//               fund_of_funds, clo_analytics, securitization, ma, capital_allocation, lease_accounting

import { randomUUID } from 'node:crypto';
import type { Finding } from '../types/agents.js';
import { BaseAnalyst, type AnalystContext, type ReasoningState } from './base-analyst.js';

export class PrivateMarketsAnalyst extends BaseAnalyst {
  constructor() {
    super('private-markets-analyst');
  }

  protected async think(ctx: AnalystContext, state: ReasoningState) {
    const task = ctx.task.toLowerCase();
    const tools: Array<{ toolName: string; params: Record<string, unknown> }> = [];

    if (state.iteration === 1) {
      if (task.includes('lbo') || task.includes('buyout') || task.includes('leveraged')) {
        tools.push({ toolName: 'pe_lbo_model', params: { query: ctx.task } });
        tools.push({ toolName: 'pe_returns_analysis', params: { query: ctx.task } });
      }
      if (task.includes('venture') || task.includes('startup') || task.includes('vc')) {
        tools.push({ toolName: 'venture_valuation', params: { query: ctx.task } });
        tools.push({ toolName: 'venture_dilution_analysis', params: { query: ctx.task } });
      }
      if (task.includes('m&a') || task.includes('merger') || task.includes('acquisition')) {
        tools.push({ toolName: 'ma_accretion_dilution', params: { query: ctx.task } });
        tools.push({ toolName: 'ma_synergy_analysis', params: { query: ctx.task } });
      }
      if (task.includes('infrastructure') || task.includes('project')) {
        tools.push({ toolName: 'infrastructure_project_finance', params: { query: ctx.task } });
        tools.push({ toolName: 'infrastructure_concession_valuation', params: { query: ctx.task } });
      }
      if (task.includes('real estate') || task.includes('reit') || task.includes('property')) {
        tools.push({ toolName: 'real_assets_property_valuation', params: { query: ctx.task } });
        tools.push({ toolName: 'real_assets_cap_rate', params: { query: ctx.task } });
      }
      if (task.includes('clo') || task.includes('securitiz')) {
        tools.push({ toolName: 'clo_analytics_tranche_analysis', params: { query: ctx.task } });
        tools.push({ toolName: 'securitization_waterfall', params: { query: ctx.task } });
      }
      if (task.includes('restructur') || task.includes('distress')) {
        tools.push({ toolName: 'restructuring_recovery_analysis', params: { query: ctx.task } });
        tools.push({ toolName: 'restructuring_waterfall', params: { query: ctx.task } });
      }
      // Default: PE + M&A overview
      if (tools.length === 0) {
        tools.push({ toolName: 'pe_lbo_model', params: { query: ctx.task } });
        tools.push({ toolName: 'ma_accretion_dilution', params: { query: ctx.task } });
      }
    } else {
      // Iteration 2+: fund-of-funds, capital allocation, private wealth
      if (task.includes('fund') || task.includes('fof') || task.includes('allocation')) {
        tools.push({ toolName: 'fund_of_funds_portfolio_construction', params: { query: ctx.task } });
        tools.push({ toolName: 'capital_allocation_optimization', params: { query: ctx.task } });
      }
      if (task.includes('wealth') || task.includes('family') || task.includes('estate')) {
        tools.push({ toolName: 'private_wealth_planning', params: { query: ctx.task } });
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
