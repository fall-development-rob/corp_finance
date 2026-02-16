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
