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
      if (task.includes('capacity') || task.includes('leverage')) {
        tools.push({ toolName: 'credit_debt_capacity', params: buildToolParams('credit_debt_capacity', state.metrics) });
      }
      if (task.includes('metric') || task.includes('ratio')) {
        tools.push({ toolName: 'credit_metrics_analysis', params: buildToolParams('credit_metrics_analysis', state.metrics) });
      }
      if (task.includes('intensity') || task.includes('hazard')) {
        tools.push({ toolName: 'credit_scoring_intensity', params: buildToolParams('credit_scoring_intensity', state.metrics) });
      }
      if (task.includes('calibrat') || task.includes('validation')) {
        tools.push({ toolName: 'credit_scoring_pd_calibration', params: buildToolParams('credit_scoring_pd_calibration', state.metrics) });
      }
      if (task.includes('validation') || task.includes('accuracy')) {
        tools.push({ toolName: 'credit_scoring_validation', params: buildToolParams('credit_scoring_validation', state.metrics) });
      }
      if (task.includes('altman') || task.includes('z-score') || task.includes('zscore')) {
        tools.push({ toolName: 'pe_altman_zscore', params: buildToolParams('pe_altman_zscore', state.metrics) });
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
      if (task.includes('migration') || task.includes('transition')) {
        tools.push({ toolName: 'credit_portfolio_migration', params: buildToolParams('credit_portfolio_migration', state.metrics) });
      }
      if (task.includes('cva') || task.includes('counterparty')) {
        tools.push({ toolName: 'credit_derivatives_cva', params: buildToolParams('credit_derivatives_cva', state.metrics) });
      }
      if (task.includes('cecl') || task.includes('provision') || task.includes('expected loss')) {
        tools.push({ toolName: 'bank_analytics_cecl', params: buildToolParams('bank_analytics_cecl', state.metrics) });
      }
      if (task.includes('nim') || task.includes('net interest')) {
        tools.push({ toolName: 'bank_analytics_nim', params: buildToolParams('bank_analytics_nim', state.metrics) });
      }
      if (task.includes('deposit') || task.includes('beta')) {
        tools.push({ toolName: 'bank_analytics_deposit_beta', params: buildToolParams('bank_analytics_deposit_beta', state.metrics) });
      }
      if (task.includes('loan book') || task.includes('loan portfolio')) {
        tools.push({ toolName: 'bank_analytics_loan_book', params: buildToolParams('bank_analytics_loan_book', state.metrics) });
      }
      if (task.includes('z-score model') || task.includes('bankruptcy')) {
        tools.push({ toolName: 'financial_forensics_zscore', params: buildToolParams('financial_forensics_zscore', state.metrics) });
      }
      if (task.includes('peer') || task.includes('benchmark')) {
        tools.push({ toolName: 'financial_forensics_peer_benchmarking', params: buildToolParams('financial_forensics_peer_benchmarking', state.metrics) });
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
