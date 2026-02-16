// Derivatives & Volatility Specialist Agent
// Covers: options pricing, vol surfaces, convertibles, structured products, Monte Carlo

import { randomUUID } from 'node:crypto';
import type { Finding } from '../types/agents.js';
import { BaseAnalyst, type AnalystContext, type ReasoningState } from './base-analyst.js';
import { buildToolParams } from '../utils/param-builder.js';

export class DerivativesAnalyst extends BaseAnalyst {
  constructor() {
    super('derivatives-analyst');
  }

  protected async think(
    ctx: AnalystContext,
    state: ReasoningState,
  ): Promise<Array<{ toolName: string; params: Record<string, unknown> }>> {
    const task = ctx.task.toLowerCase();
    const plan: Array<{ toolName: string; params: Record<string, unknown> }> = [];
    // baseParams removed — now using buildToolParams()

    if (state.iteration === 1) {
      // Primary analysis pass — pattern-match on keywords
      if (task.includes('option') || task.includes('greeks') || task.includes('black-scholes')) {
        plan.push(
          { toolName: 'derivatives_option_pricing', params: buildToolParams('derivatives_option_pricing', state.metrics) },
          { toolName: 'derivatives_greeks_calculation', params: buildToolParams('derivatives_greeks_calculation', state.metrics) },
        );
      }

      if (task.includes('volatility') || task.includes('vol surface') || task.includes('skew')) {
        plan.push(
          { toolName: 'volatility_surface_interpolation', params: buildToolParams('volatility_surface_interpolation', state.metrics) },
          { toolName: 'volatility_surface_smile_analysis', params: buildToolParams('volatility_surface_smile_analysis', state.metrics) },
        );
      }

      if (task.includes('monte carlo') || task.includes('simulation')) {
        plan.push({ toolName: 'monte_carlo_simulation', params: buildToolParams('monte_carlo_simulation', state.metrics) });
      }

      if (task.includes('convertible')) {
        plan.push({ toolName: 'convertibles_pricing', params: buildToolParams('convertibles_pricing', state.metrics) });
      }

      if (task.includes('structured') || task.includes('note')) {
        plan.push({ toolName: 'structured_products_analysis', params: buildToolParams('structured_products_analysis', state.metrics) });
      }


      if (task.includes('forward value') || task.includes('position value')) {
        plan.push({ toolName: 'derivatives_forward_value', params: buildToolParams('derivatives_forward_value', state.metrics) });
      }

      if (task.includes('futures') || task.includes('basis')) {
        plan.push({ toolName: 'derivatives_futures_basis', params: buildToolParams('derivatives_futures_basis', state.metrics) });
      }

      if (task.includes('interest rate swap') || task.includes('irs') || task.includes('swap rate')) {
        plan.push({ toolName: 'derivatives_irs', params: buildToolParams('derivatives_irs', state.metrics) });
      }

      if (task.includes('currency swap') || task.includes('cross-currency')) {
        plan.push({ toolName: 'derivatives_currency_swap', params: buildToolParams('derivatives_currency_swap', state.metrics) });
      }

      if (task.includes('strategy') || task.includes('straddle') || task.includes('strangle') || task.includes('spread')) {
        plan.push({ toolName: 'derivatives_strategy', params: buildToolParams('derivatives_strategy', state.metrics) });
      }

      // Default fallback when no keywords matched
      if (plan.length === 0) {
        plan.push(
          { toolName: 'derivatives_option_pricing', params: buildToolParams('derivatives_option_pricing', state.metrics) },
          { toolName: 'monte_carlo_simulation', params: buildToolParams('monte_carlo_simulation', state.metrics) },
        );
      }
    } else {
      // Iteration 2+: deeper analysis
      const priorTools = new Set(state.toolResults.map(t => t.toolName));

      if (!priorTools.has('real_options_valuation') && (task.includes('real option') || task.includes('project'))) {
        plan.push({ toolName: 'real_options_valuation', params: buildToolParams('real_options_valuation', state.metrics) });
      }

      if (!priorTools.has('credit_derivatives_cds_pricing') && (task.includes('credit') || task.includes('cds'))) {
        plan.push({ toolName: 'credit_derivatives_cds_pricing', params: buildToolParams('credit_derivatives_cds_pricing', state.metrics) });
      }


      if (!priorTools.has('convertibles_analysis') && (task.includes('convertible analysis') || task.includes('conversion premium'))) {
        plan.push({ toolName: 'convertibles_analysis', params: buildToolParams('convertibles_analysis', state.metrics) });
      }

      if (!priorTools.has('structured_products_exotic') && (task.includes('exotic') || task.includes('barrier') || task.includes('autocall') || task.includes('digital'))) {
        plan.push({ toolName: 'structured_products_exotic', params: buildToolParams('structured_products_exotic', state.metrics) });
      }

      if (!priorTools.has('real_options_decision_tree') && (task.includes('decision tree') || task.includes('binomial') || task.includes('real option'))) {
        plan.push({ toolName: 'real_options_decision_tree', params: buildToolParams('real_options_decision_tree', state.metrics) });
      }

      if (!priorTools.has('monte_carlo_simulation') && plan.length === 0) {
        plan.push({ toolName: 'monte_carlo_simulation', params: buildToolParams('monte_carlo_simulation', state.metrics) });
      }
    }

    state.thoughts.push(
      `Iteration ${state.iteration}: planned ${plan.length} tool calls — ${plan.map(p => p.toolName).join(', ')}`,
    );

    return plan;
  }

  protected async reflect(
    _ctx: AnalystContext,
    state: ReasoningState,
  ): Promise<{ summary: string; shouldIterate: boolean }> {
    const successCount = state.toolResults.filter(t => !t.error).length;
    const shouldIterate = state.iteration === 1 && successCount < 3;

    return {
      summary: `Iteration ${state.iteration}: ${successCount} successful tool results. ${shouldIterate ? 'Iterating for deeper analysis.' : 'Sufficient data gathered.'}`,
      shouldIterate,
    };
  }

  protected async synthesize(
    _ctx: AnalystContext,
    state: ReasoningState,
  ): Promise<Finding[]> {
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
