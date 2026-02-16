// Equity Analyst — DCF, comps, earnings quality, dividends
// Tool domains: equity_research, valuation, earnings_quality, dividend_policy,
//               behavioral, performance_attribution, three_statement, fpa

import type { Finding } from '../types/agents.js';
import { BaseAnalyst, type AnalystContext, type ReasoningState } from './base-analyst.js';
import { buildToolParams } from '../utils/param-builder.js';

export class EquityAnalyst extends BaseAnalyst {
  constructor() {
    super('equity-analyst');
  }

  protected async think(ctx: AnalystContext, state: ReasoningState) {
    const task = ctx.task.toLowerCase();
    const tools: Array<{ toolName: string; params: Record<string, unknown> }> = [];

    if (state.iteration === 1) {
      // Core valuation tools
      if (task.includes('dcf') || task.includes('valuation') || task.includes('fair value')) {
        tools.push({ toolName: 'valuation_dcf_model', params: buildToolParams('valuation_dcf_model', state.metrics) });
        tools.push({ toolName: 'valuation_wacc_calculation', params: buildToolParams('valuation_wacc_calculation', state.metrics) });
      }
      if (task.includes('comp') || task.includes('multiple') || task.includes('peer')) {
        tools.push({ toolName: 'valuation_comparable_companies', params: buildToolParams('valuation_comparable_companies', state.metrics) });
      }
      if (task.includes('equity') || task.includes('stock') || task.includes('research')) {
        tools.push({ toolName: 'equity_research_fundamental_analysis', params: buildToolParams('equity_research_fundamental_analysis', state.metrics) });
      }
      if (task.includes('sotp') || task.includes('sum of parts') || task.includes('segment')) {
        tools.push({ toolName: 'valuation_sotp_valuation', params: buildToolParams('valuation_sotp_valuation', state.metrics) });
      }
      if (task.includes('target price') || task.includes('price target')) {
        tools.push({ toolName: 'valuation_target_price', params: buildToolParams('valuation_target_price', state.metrics) });
      }
      // Default: at least run fundamental analysis
      if (tools.length === 0) {
        tools.push({ toolName: 'equity_research_fundamental_analysis', params: buildToolParams('equity_research_fundamental_analysis', state.metrics) });
        tools.push({ toolName: 'valuation_dcf_model', params: buildToolParams('valuation_dcf_model', state.metrics) });
      }
    } else {
      // Deeper dives on subsequent iterations
      if (task.includes('earning') || task.includes('quality')) {
        tools.push({ toolName: 'earnings_quality_accruals_analysis', params: buildToolParams('earnings_quality_accruals_analysis', state.metrics) });
      }
      if (task.includes('dividend')) {
        tools.push({ toolName: 'dividend_policy_sustainability', params: buildToolParams('dividend_policy_sustainability', state.metrics) });
      }
      if (task.includes('attribution') || task.includes('performance')) {
        tools.push({ toolName: 'performance_attribution_brinson', params: buildToolParams('performance_attribution_brinson', state.metrics) });
      }
      if (task.includes('financial') || task.includes('statement')) {
        tools.push({ toolName: 'three_statement_model', params: buildToolParams('three_statement_model', state.metrics) });
      }
      if (task.includes('piotroski') || task.includes('f-score') || task.includes('strength')) {
        tools.push({ toolName: 'earnings_quality_piotroski', params: buildToolParams('earnings_quality_piotroski', state.metrics) });
      }
      if (task.includes('revenue quality')) {
        tools.push({ toolName: 'earnings_quality_revenue', params: buildToolParams('earnings_quality_revenue', state.metrics) });
      }
      if (task.includes('earnings quality') || task.includes('quality composite')) {
        tools.push({ toolName: 'earnings_quality_composite', params: buildToolParams('earnings_quality_composite', state.metrics) });
      }
      if (task.includes('ddm') || task.includes('dividend discount')) {
        tools.push({ toolName: 'dividend_policy_h_model', params: buildToolParams('dividend_policy_h_model', state.metrics) });
      }
      if (task.includes('buyback') || task.includes('repurchase')) {
        tools.push({ toolName: 'dividend_policy_buyback', params: buildToolParams('dividend_policy_buyback', state.metrics) });
      }
      if (task.includes('total return') || task.includes('tsr')) {
        tools.push({ toolName: 'dividend_policy_total_shareholder_return', params: buildToolParams('dividend_policy_total_shareholder_return', state.metrics) });
      }
      if (task.includes('dupont') || task.includes('roe decomposition')) {
        tools.push({ toolName: 'financial_forensics_dupont', params: buildToolParams('financial_forensics_dupont', state.metrics) });
      }
      if (task.includes('peer') || task.includes('benchmark')) {
        tools.push({ toolName: 'financial_forensics_peer_benchmarking', params: buildToolParams('financial_forensics_peer_benchmarking', state.metrics) });
      }
      if (task.includes('red flag') || task.includes('warning sign')) {
        tools.push({ toolName: 'financial_forensics_red_flags', params: buildToolParams('financial_forensics_red_flags', state.metrics) });
      }
      if (task.includes('benford') || task.includes('digit analysis')) {
        tools.push({ toolName: 'financial_forensics_benfords_law', params: buildToolParams('financial_forensics_benfords_law', state.metrics) });
      }
      if (task.includes('variance') || task.includes('budget')) {
        tools.push({ toolName: 'fpa_variance', params: buildToolParams('fpa_variance', state.metrics) });
      }
      if (task.includes('breakeven') || task.includes('break-even')) {
        tools.push({ toolName: 'fpa_breakeven', params: buildToolParams('fpa_breakeven', state.metrics) });
      }
      if (task.includes('working capital') || task.includes('dso') || task.includes('dio')) {
        tools.push({ toolName: 'fpa_working_capital', params: buildToolParams('fpa_working_capital', state.metrics) });
      }
      if (task.includes('forecast') || task.includes('rolling')) {
        tools.push({ toolName: 'fpa_rolling_forecast', params: buildToolParams('fpa_rolling_forecast', state.metrics) });
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
