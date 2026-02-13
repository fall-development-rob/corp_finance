// Macro Strategist — rates, FX, commodities, EM, sovereign, inflation
// Tool domains: macro_economics, fx_commodities, commodity_trading, emerging_markets,
//               trade_finance, carbon_markets, sovereign, inflation_linked

import { randomUUID } from 'node:crypto';
import type { Finding } from '../types/agents.js';
import { BaseAnalyst, type AnalystContext, type ReasoningState } from './base-analyst.js';

export class MacroAnalyst extends BaseAnalyst {
  constructor() {
    super('macro-analyst');
  }

  protected async think(ctx: AnalystContext, state: ReasoningState) {
    const task = ctx.task.toLowerCase();
    const tools: Array<{ toolName: string; params: Record<string, unknown> }> = [];

    if (state.iteration === 1) {
      if (task.includes('rate') || task.includes('interest') || task.includes('yield')) {
        tools.push({ toolName: 'macro_economics_rate_analysis', params: { query: ctx.task } });
        tools.push({ toolName: 'macro_economics_yield_curve', params: { query: ctx.task } });
      }
      if (task.includes('fx') || task.includes('currency') || task.includes('exchange')) {
        tools.push({ toolName: 'fx_commodities_currency_analysis', params: { query: ctx.task } });
        tools.push({ toolName: 'fx_commodities_cross_rate', params: { query: ctx.task } });
      }
      if (task.includes('commodity') || task.includes('oil') || task.includes('gold')) {
        tools.push({ toolName: 'commodity_trading_price_analysis', params: { query: ctx.task } });
        tools.push({ toolName: 'fx_commodities_commodity_valuation', params: { query: ctx.task } });
      }
      if (task.includes('emerging') || task.includes('em') || task.includes('frontier')) {
        tools.push({ toolName: 'emerging_markets_country_risk', params: { query: ctx.task } });
        tools.push({ toolName: 'emerging_markets_sovereign_spread', params: { query: ctx.task } });
      }
      if (task.includes('inflation') || task.includes('cpi')) {
        tools.push({ toolName: 'inflation_linked_breakeven_rate', params: { query: ctx.task } });
        tools.push({ toolName: 'macro_economics_inflation_analysis', params: { query: ctx.task } });
      }
      if (task.includes('sovereign')) {
        tools.push({ toolName: 'sovereign_debt_sustainability', params: { query: ctx.task } });
        tools.push({ toolName: 'sovereign_credit_analysis', params: { query: ctx.task } });
      }
      // Default: broad macro overview
      if (tools.length === 0) {
        tools.push({ toolName: 'macro_economics_rate_analysis', params: { query: ctx.task } });
        tools.push({ toolName: 'macro_economics_economic_indicators', params: { query: ctx.task } });
      }
    } else {
      // Iteration 2+: deeper dives into trade finance and carbon markets
      if (task.includes('trade') || task.includes('export') || task.includes('import')) {
        tools.push({ toolName: 'trade_finance_letter_of_credit', params: { query: ctx.task } });
      }
      if (task.includes('carbon') || task.includes('emission') || task.includes('climate')) {
        tools.push({ toolName: 'carbon_markets_emission_pricing', params: { query: ctx.task } });
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
