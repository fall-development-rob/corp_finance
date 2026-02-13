// Credit Analyst — ratings, spreads, default probability, covenants, restructuring
// Tool domains: credit, credit_scoring, credit_portfolio, credit_derivatives,
//               restructuring, financial_forensics, three_statement, bank_analytics

import type { Finding } from '../types/agents.js';
import { BaseAnalyst, type AnalystContext, type ReasoningState } from './base-analyst.js';
import { buildToolParams } from '../utils/param-builder.js';

export class CreditAnalyst extends BaseAnalyst {
  constructor() {
    super('credit-analyst');
  }

  protected async think(ctx: AnalystContext, state: ReasoningState) {
    const task = ctx.task.toLowerCase();
    const tools: Array<{ toolName: string; params: Record<string, unknown> }> = [];

    if (state.iteration === 1) {
      // Core credit analysis tools
      if (task.includes('score') || task.includes('rating') || task.includes('credit')) {
        tools.push({ toolName: 'credit_scoring_corporate', params: buildToolParams('credit_scoring_corporate', state.metrics) });
      }
      if (task.includes('spread') || task.includes('premium') || task.includes('oas')) {
        tools.push({ toolName: 'credit_spread_analysis', params: buildToolParams('credit_spread_analysis', state.metrics) });
      }
      if (task.includes('default') || task.includes('probability') || task.includes('pd')) {
        tools.push({ toolName: 'credit_default_probability', params: buildToolParams('credit_default_probability', state.metrics) });
      }
      if (task.includes('portfolio') || task.includes('concentration')) {
        tools.push({ toolName: 'credit_portfolio_var', params: buildToolParams('credit_portfolio_var', state.metrics) });
      }
      if (task.includes('cds') || task.includes('derivative') || task.includes('swap')) {
        tools.push({ toolName: 'credit_derivatives_cds_pricing', params: buildToolParams('credit_derivatives_cds_pricing', state.metrics) });
      }
      // Default: at least run corporate scoring and spread analysis
      if (tools.length === 0) {
        tools.push({ toolName: 'credit_scoring_corporate', params: buildToolParams('credit_scoring_corporate', state.metrics) });
        tools.push({ toolName: 'credit_spread_analysis', params: buildToolParams('credit_spread_analysis', state.metrics) });
      }
    } else {
      // Deeper dives on subsequent iterations
      if (task.includes('restructur') || task.includes('distress') || task.includes('recovery')) {
        tools.push({ toolName: 'restructuring_distressed_valuation', params: buildToolParams('restructuring_distressed_valuation', state.metrics) });
      }
      if (task.includes('forensic') || task.includes('fraud') || task.includes('manipulation')) {
        tools.push({ toolName: 'financial_forensics_beneish', params: buildToolParams('financial_forensics_beneish', state.metrics) });
      }
      if (task.includes('bank') || task.includes('capital adequacy') || task.includes('tier')) {
        tools.push({ toolName: 'bank_analytics_capital_adequacy', params: buildToolParams('bank_analytics_capital_adequacy', state.metrics) });
      }
      if (task.includes('covenant') || task.includes('leverage') || task.includes('coverage')) {
        tools.push({ toolName: 'credit_covenant_analysis', params: buildToolParams('credit_covenant_analysis', state.metrics) });
      }
      if (task.includes('financial') || task.includes('statement')) {
        tools.push({ toolName: 'three_statement_model', params: buildToolParams('three_statement_model', state.metrics) });
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
        statement: 'Unable to complete credit analysis — no tool results available',
        supportingData: {},
        confidence: 0,
        methodology: 'N/A',
        citations: [],
      });
    }

    return findings;
  }
}
