// Quantitative Risk Specialist Agent
// Covers: VaR, factor models, portfolio optimization, risk budgeting, stress testing

import { randomUUID } from 'node:crypto';
import type { Finding } from '../types/agents.js';
import { BaseAnalyst, type AnalystContext, type ReasoningState } from './base-analyst.js';
import { buildToolParams } from '../utils/param-builder.js';

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

    if (state.iteration === 1) {
      // Primary analysis pass — pattern-match on keywords
      if (task.includes('var') || task.includes('value at risk')) {
        plan.push(
          { toolName: 'quant_risk_var_calculation', params: buildToolParams('quant_risk_var_calculation', state.metrics) },
          { toolName: 'quant_risk_expected_shortfall', params: buildToolParams('quant_risk_expected_shortfall', state.metrics) },
        );
      }

      if (task.includes('portfolio') || task.includes('optimization') || task.includes('allocation')) {
        plan.push(
          { toolName: 'portfolio_optimization_mean_variance', params: buildToolParams('portfolio_optimization_mean_variance', state.metrics) },
          { toolName: 'risk_budgeting_risk_parity', params: buildToolParams('risk_budgeting_risk_parity', state.metrics) },
        );
      }

      if (task.includes('factor') || task.includes('beta') || task.includes('exposure')) {
        plan.push({ toolName: 'quant_risk_factor_analysis', params: buildToolParams('quant_risk_factor_analysis', state.metrics) });
      }

      if (task.includes('stress') || task.includes('scenario')) {
        plan.push({ toolName: 'scenarios_stress_test', params: buildToolParams('scenarios_stress_test', state.metrics) });
      }

      if (task.includes('sharpe') || task.includes('performance')) {
        plan.push({ toolName: 'performance_attribution_factor_based', params: buildToolParams('performance_attribution_factor_based', state.metrics) });
      }


      if (task.includes('black-litterman') || task.includes('investor views')) {
        plan.push({ toolName: 'portfolio_optimization_black_litterman', params: buildToolParams('portfolio_optimization_black_litterman', state.metrics) });
      }

      if (task.includes('factor risk') || task.includes('risk budget')) {
        plan.push({ toolName: 'risk_budgeting_factor', params: buildToolParams('risk_budgeting_factor', state.metrics) });
      }

      if (task.includes('bayesian') || task.includes('prior') || task.includes('posterior')) {
        plan.push({ toolName: 'quant_risk_black_litterman', params: buildToolParams('quant_risk_black_litterman', state.metrics) });
      }

      if (task.includes('prospect') || task.includes('loss aversion') || task.includes('behavioral')) {
        plan.push({ toolName: 'behavioral_prospect_theory', params: buildToolParams('behavioral_prospect_theory', state.metrics) });
      }

      if (task.includes('sentiment') || task.includes('fear') || task.includes('greed')) {
        plan.push({ toolName: 'behavioral_sentiment', params: buildToolParams('behavioral_sentiment', state.metrics) });
      }

      if (task.includes('sensitivity') || task.includes('tornado')) {
        plan.push({ toolName: 'scenarios_sensitivity', params: buildToolParams('scenarios_sensitivity', state.metrics) });
      }

      if (task.includes('scenario') && !task.includes('stress')) {
        plan.push({ toolName: 'scenarios_analysis', params: buildToolParams('scenarios_analysis', state.metrics) });
      }

      // Default fallback when no keywords matched
      if (plan.length === 0) {
        plan.push(
          { toolName: 'quant_risk_var_calculation', params: buildToolParams('quant_risk_var_calculation', state.metrics) },
          { toolName: 'quant_risk_factor_analysis', params: buildToolParams('quant_risk_factor_analysis', state.metrics) },
        );
      }
    } else {
      // Iteration 2+: deeper analysis
      const priorTools = new Set(state.toolResults.map(t => t.toolName));

      if (!priorTools.has('quant_strategies_momentum') && (task.includes('momentum') || task.includes('strategy'))) {
        plan.push({ toolName: 'quant_strategies_momentum', params: buildToolParams('quant_strategies_momentum', state.metrics) });
      }

      if (!priorTools.has('index_construction_methodology') && (task.includes('index') || task.includes('benchmark'))) {
        plan.push({ toolName: 'index_construction_methodology', params: buildToolParams('index_construction_methodology', state.metrics) });
      }

      if (!priorTools.has('market_microstructure_liquidity') && (task.includes('liquidity') || task.includes('microstructure'))) {
        plan.push({ toolName: 'market_microstructure_liquidity', params: buildToolParams('market_microstructure_liquidity', state.metrics) });
      }


      if (!priorTools.has('quant_strategies_pairs') && (task.includes('pairs') || task.includes('cointegration') || task.includes('mean reversion'))) {
        plan.push({ toolName: 'quant_strategies_pairs', params: buildToolParams('quant_strategies_pairs', state.metrics) });
      }

      if (!priorTools.has('index_construction_rebalancing') && task.includes('rebalanc')) {
        plan.push({ toolName: 'index_construction_rebalancing', params: buildToolParams('index_construction_rebalancing', state.metrics) });
      }

      if (!priorTools.has('index_construction_tracking_error') && task.includes('tracking error')) {
        plan.push({ toolName: 'index_construction_tracking_error', params: buildToolParams('index_construction_tracking_error', state.metrics) });
      }

      if (!priorTools.has('index_construction_smart_beta') && (task.includes('smart beta') || task.includes('factor tilt'))) {
        plan.push({ toolName: 'index_construction_smart_beta', params: buildToolParams('index_construction_smart_beta', state.metrics) });
      }

      if (!priorTools.has('index_construction_reconstitution') && (task.includes('reconstitution') || task.includes('additions'))) {
        plan.push({ toolName: 'index_construction_reconstitution', params: buildToolParams('index_construction_reconstitution', state.metrics) });
      }

      if (!priorTools.has('market_microstructure_execution') && (task.includes('execution') || task.includes('impact') || task.includes('slippage'))) {
        plan.push({ toolName: 'market_microstructure_execution', params: buildToolParams('market_microstructure_execution', state.metrics) });
      }

      if (!priorTools.has('portfolio_risk_adjusted_returns') && (task.includes('sharpe') || task.includes('sortino') || task.includes('risk-adjusted'))) {
        plan.push({ toolName: 'portfolio_risk_adjusted_returns', params: buildToolParams('portfolio_risk_adjusted_returns', state.metrics) });
      }

      if (!priorTools.has('portfolio_risk_metrics') && (task.includes('var') || task.includes('cvar') || task.includes('risk metric'))) {
        plan.push({ toolName: 'portfolio_risk_metrics', params: buildToolParams('portfolio_risk_metrics', state.metrics) });
      }

      if (!priorTools.has('portfolio_kelly_sizing') && (task.includes('kelly') || task.includes('position size'))) {
        plan.push({ toolName: 'portfolio_kelly_sizing', params: buildToolParams('portfolio_kelly_sizing', state.metrics) });
      }

      // Fallback deeper tool if nothing else matched in iteration 2+
      if (plan.length === 0 && !priorTools.has('scenarios_stress_test')) {
        plan.push({ toolName: 'scenarios_stress_test', params: buildToolParams('scenarios_stress_test', state.metrics) });
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
