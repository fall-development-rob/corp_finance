// Equity Analyst — DCF, comps, earnings quality, dividends
// Tool domains: equity_research, valuation, earnings_quality, dividend_policy,
//               behavioral, performance_attribution, three_statement, fpa

import type { Finding } from '../types/agents.js';
import { BaseAnalyst, type AnalystContext, type ReasoningState } from './base-analyst.js';

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
        tools.push({ toolName: 'valuation_dcf_model', params: { query: ctx.task } });
        tools.push({ toolName: 'valuation_wacc_calculation', params: { query: ctx.task } });
      }
      if (task.includes('comp') || task.includes('multiple') || task.includes('peer')) {
        tools.push({ toolName: 'valuation_comparable_companies', params: { query: ctx.task } });
      }
      if (task.includes('equity') || task.includes('stock') || task.includes('research')) {
        tools.push({ toolName: 'equity_research_fundamental_analysis', params: { query: ctx.task } });
      }
      // Default: at least run fundamental analysis
      if (tools.length === 0) {
        tools.push({ toolName: 'equity_research_fundamental_analysis', params: { query: ctx.task } });
        tools.push({ toolName: 'valuation_dcf_model', params: { query: ctx.task } });
      }
    } else {
      // Deeper dives on subsequent iterations
      if (task.includes('earning') || task.includes('quality')) {
        tools.push({ toolName: 'earnings_quality_accruals_analysis', params: { query: ctx.task } });
      }
      if (task.includes('dividend')) {
        tools.push({ toolName: 'dividend_policy_sustainability', params: { query: ctx.task } });
      }
      if (task.includes('attribution') || task.includes('performance')) {
        tools.push({ toolName: 'performance_attribution_brinson', params: { query: ctx.task } });
      }
      if (task.includes('financial') || task.includes('statement')) {
        tools.push({ toolName: 'three_statement_model', params: { query: ctx.task } });
      }
    }

    return tools;
  }

  protected async reflect(_ctx: AnalystContext, state: ReasoningState) {
    const successCount = state.toolResults.filter(t => !t.error).length;
    const hasResults = successCount > 0;
    const shouldIterate = state.iteration === 1 && hasResults && state.toolResults.length < 4;

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
        statement: 'Unable to complete equity analysis — no tool results available',
        supportingData: {},
        confidence: 0,
        methodology: 'N/A',
        citations: [],
      });
    }

    return findings;
  }
}
