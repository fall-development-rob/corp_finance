// Quantitative Risk Specialist Agent
// Covers: VaR, factor models, portfolio optimization, risk budgeting, stress testing

import { randomUUID } from 'node:crypto';
import type { Finding } from '../types/agents.js';
import { BaseAnalyst, type AnalystContext, type ReasoningState } from './base-analyst.js';

export class QuantRiskAnalyst extends BaseAnalyst {
  constructor() {
    super('quant-risk-analyst');
  }

  protected async think(
    ctx: AnalystContext,
    state: ReasoningState,
  ): Promise<Array<{ toolName: string; params: Record<string, unknown> }>> {
    const task = ctx.task.toLowerCase();
    const plan: Array<{ toolName: string; params: Record<string, unknown> }> = [];
    const baseParams = { query: ctx.task };

    if (state.iteration === 1) {
      // Primary analysis pass — pattern-match on keywords
      if (task.includes('var') || task.includes('value at risk')) {
        plan.push(
          { toolName: 'quant_risk_var_calculation', params: { ...baseParams } },
          { toolName: 'quant_risk_expected_shortfall', params: { ...baseParams } },
        );
      }

      if (task.includes('portfolio') || task.includes('optimization') || task.includes('allocation')) {
        plan.push(
          { toolName: 'portfolio_optimization_mean_variance', params: { ...baseParams } },
          { toolName: 'risk_budgeting_risk_parity', params: { ...baseParams } },
        );
      }

      if (task.includes('factor') || task.includes('beta') || task.includes('exposure')) {
        plan.push({ toolName: 'quant_risk_factor_analysis', params: { ...baseParams } });
      }

      if (task.includes('stress') || task.includes('scenario')) {
        plan.push({ toolName: 'scenarios_stress_test', params: { ...baseParams } });
      }

      if (task.includes('sharpe') || task.includes('performance')) {
        plan.push({ toolName: 'performance_attribution_factor_based', params: { ...baseParams } });
      }

      // Default fallback when no keywords matched
      if (plan.length === 0) {
        plan.push(
          { toolName: 'quant_risk_var_calculation', params: { ...baseParams } },
          { toolName: 'quant_risk_factor_analysis', params: { ...baseParams } },
        );
      }
    } else {
      // Iteration 2+: deeper analysis
      const priorTools = new Set(state.toolResults.map(t => t.toolName));

      if (!priorTools.has('quant_strategies_momentum') && (task.includes('momentum') || task.includes('strategy'))) {
        plan.push({ toolName: 'quant_strategies_momentum', params: { ...baseParams } });
      }

      if (!priorTools.has('index_construction_methodology') && (task.includes('index') || task.includes('benchmark'))) {
        plan.push({ toolName: 'index_construction_methodology', params: { ...baseParams } });
      }

      if (!priorTools.has('market_microstructure_liquidity') && (task.includes('liquidity') || task.includes('microstructure'))) {
        plan.push({ toolName: 'market_microstructure_liquidity', params: { ...baseParams } });
      }

      // Fallback deeper tool if nothing else matched in iteration 2+
      if (plan.length === 0 && !priorTools.has('scenarios_stress_test')) {
        plan.push({ toolName: 'scenarios_stress_test', params: { ...baseParams } });
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
        statement: 'Quantitative risk analysis could not be completed: no tool calls succeeded.',
        supportingData: {},
        confidence: 0,
        methodology: 'fallback',
        citations: [],
      });
    }

    return findings;
  }
}
