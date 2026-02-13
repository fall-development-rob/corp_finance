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

    for (const invocation of state.toolResults) {
      if (invocation.error) continue;

      const output = typeof invocation.result === 'string'
        ? invocation.result
        : JSON.stringify(invocation.result);

      const snippet = output.length > 300 ? output.slice(0, 300) + '...' : output;

      findings.push({
        statement: `[${invocation.toolName}] ${snippet}`,
        supportingData: { raw: invocation.result },
        confidence: invocation.duration !== undefined && invocation.duration < 10000 ? 0.85 : 0.7,
        methodology: invocation.toolName.replace(/_/g, ' '),
        citations: [
          {
            invocationId: invocation.invocationId,
            toolName: invocation.toolName,
            relevantOutput: snippet,
          },
        ],
      });
    }

    // If no tools succeeded, produce a fallback finding
    if (findings.length === 0) {
      findings.push({
        statement: 'Derivatives analysis could not be completed: no tool calls succeeded.',
        supportingData: {},
        confidence: 0,
        methodology: 'fallback',
        citations: [],
      });
    }

    return findings;
  }
}
