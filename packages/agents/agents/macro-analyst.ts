// Macro Strategist — rates, FX, commodities, EM, sovereign, inflation
// Tool domains: macro_economics, fx_commodities, commodity_trading, emerging_markets,
//               trade_finance, carbon_markets, sovereign, inflation_linked

import { randomUUID } from 'node:crypto';
import type { Finding } from '../types/agents.js';
import { BaseAnalyst, type AnalystContext, type ReasoningState } from './base-analyst.js';
import { buildToolParams } from '../utils/param-builder.js';

export class MacroAnalyst extends BaseAnalyst {
  constructor() {
    super('macro-analyst');
  }

  protected async think(ctx: AnalystContext, state: ReasoningState) {
    const task = ctx.task.toLowerCase();
    const tools: Array<{ toolName: string; params: Record<string, unknown> }> = [];

    if (state.iteration === 1) {
      if (task.includes('rate') || task.includes('interest') || task.includes('yield')) {
        tools.push({ toolName: 'macro_economics_rate_analysis', params: buildToolParams('macro_economics_rate_analysis', state.metrics) });
        tools.push({ toolName: 'macro_economics_yield_curve', params: buildToolParams('macro_economics_yield_curve', state.metrics) });
      }
      if (task.includes('fx') || task.includes('currency') || task.includes('exchange')) {
        tools.push({ toolName: 'fx_commodities_currency_analysis', params: buildToolParams('fx_commodities_currency_analysis', state.metrics) });
        tools.push({ toolName: 'fx_commodities_cross_rate', params: buildToolParams('fx_commodities_cross_rate', state.metrics) });
      }
      if (task.includes('commodity') || task.includes('oil') || task.includes('gold')) {
        tools.push({ toolName: 'commodity_trading_price_analysis', params: buildToolParams('commodity_trading_price_analysis', state.metrics) });
        tools.push({ toolName: 'fx_commodities_commodity_valuation', params: buildToolParams('fx_commodities_commodity_valuation', state.metrics) });
      }
      if (task.includes('emerging') || task.includes('em') || task.includes('frontier')) {
        tools.push({ toolName: 'emerging_markets_country_risk', params: buildToolParams('emerging_markets_country_risk', state.metrics) });
        tools.push({ toolName: 'emerging_markets_sovereign_spread', params: buildToolParams('emerging_markets_sovereign_spread', state.metrics) });
      }
      if (task.includes('inflation') || task.includes('cpi')) {
        tools.push({ toolName: 'inflation_linked_breakeven_rate', params: buildToolParams('inflation_linked_breakeven_rate', state.metrics) });
        tools.push({ toolName: 'macro_economics_inflation_analysis', params: buildToolParams('macro_economics_inflation_analysis', state.metrics) });
      }
      if (task.includes('sovereign')) {
        tools.push({ toolName: 'sovereign_debt_sustainability', params: buildToolParams('sovereign_debt_sustainability', state.metrics) });
        tools.push({ toolName: 'sovereign_credit_analysis', params: buildToolParams('sovereign_credit_analysis', state.metrics) });
      }
      if (task.includes('political') || task.includes('geopolitical')) {
        tools.push({ toolName: 'emerging_markets_political_risk', params: buildToolParams('emerging_markets_political_risk', state.metrics) });
      }
      if (task.includes('capital control') || task.includes('repatriation')) {
        tools.push({ toolName: 'emerging_markets_capital_controls', params: buildToolParams('emerging_markets_capital_controls', state.metrics) });
      }
      if (task.includes('equity premium') || task.includes('em premium')) {
        tools.push({ toolName: 'emerging_markets_equity_premium', params: buildToolParams('emerging_markets_equity_premium', state.metrics) });
      }
      if (task.includes('term structure') || task.includes('contango') || task.includes('backwardation')) {
        tools.push({ toolName: 'fx_commodities_commodity_curve', params: buildToolParams('fx_commodities_commodity_curve', state.metrics) });
      }
      // Default: broad macro overview
      if (tools.length === 0) {
        tools.push({ toolName: 'macro_economics_rate_analysis', params: buildToolParams('macro_economics_rate_analysis', state.metrics) });
        tools.push({ toolName: 'macro_economics_economic_indicators', params: buildToolParams('macro_economics_economic_indicators', state.metrics) });
      }
    } else {
      // Iteration 2+: deeper dives into trade finance and carbon markets
      if (task.includes('trade') || task.includes('export') || task.includes('import')) {
        tools.push({ toolName: 'trade_finance_letter_of_credit', params: buildToolParams('trade_finance_letter_of_credit', state.metrics) });
      }
      if (task.includes('carbon') || task.includes('emission') || task.includes('climate')) {
        tools.push({ toolName: 'carbon_markets_emission_pricing', params: buildToolParams('carbon_markets_emission_pricing', state.metrics) });
      }
      if (task.includes('storage') || task.includes('carry cost') || task.includes('convenience yield')) {
        tools.push({ toolName: 'commodity_trading_storage', params: buildToolParams('commodity_trading_storage', state.metrics) });
      }
      if (task.includes('ets') || task.includes('emission trading')) {
        tools.push({ toolName: 'carbon_markets_ets_compliance', params: buildToolParams('carbon_markets_ets_compliance', state.metrics) });
      }
      if (task.includes('cbam') || task.includes('carbon border')) {
        tools.push({ toolName: 'carbon_markets_cbam', params: buildToolParams('carbon_markets_cbam', state.metrics) });
      }
      if (task.includes('shadow carbon') || task.includes('internal carbon')) {
        tools.push({ toolName: 'carbon_markets_shadow_price', params: buildToolParams('carbon_markets_shadow_price', state.metrics) });
      }
      if (task.includes('supply chain') || task.includes('factoring') || task.includes('forfaiting')) {
        tools.push({ toolName: 'trade_finance_supply_chain', params: buildToolParams('trade_finance_supply_chain', state.metrics) });
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
