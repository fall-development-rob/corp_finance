// Fixed Income Analyst — bonds, yield curves, rates, mortgage analytics, municipal, sovereign
// Tool domains: fixed_income, interest_rate_models, inflation_linked, mortgage_analytics,
//               repo_financing, municipal, sovereign, three_statement

import type { Finding } from '../types/agents.js';
import { BaseAnalyst, type AnalystContext, type ReasoningState } from './base-analyst.js';
import { buildToolParams } from '../utils/param-builder.js';

export class FixedIncomeAnalyst extends BaseAnalyst {
  constructor() {
    super('fixed-income-analyst');
  }

  protected async think(ctx: AnalystContext, state: ReasoningState) {
    const task = ctx.task.toLowerCase();
    const tools: Array<{ toolName: string; params: Record<string, unknown> }> = [];

    if (state.iteration === 1) {
      // Core fixed income tools
      if (task.includes('bond') || task.includes('pricing') || task.includes('duration')) {
        tools.push({ toolName: 'fixed_income_bond_pricing', params: buildToolParams('fixed_income_bond_pricing', state.metrics) });
      }
      if (task.includes('yield') || task.includes('curve') || task.includes('term structure')) {
        tools.push({ toolName: 'fixed_income_yield_curve', params: buildToolParams('fixed_income_yield_curve', state.metrics) });
      }
      if (task.includes('rate') || task.includes('vasicek') || task.includes('hull-white')) {
        tools.push({ toolName: 'interest_rate_models_vasicek', params: buildToolParams('interest_rate_models_vasicek', state.metrics) });
      }
      if (task.includes('inflation') || task.includes('tips') || task.includes('linker')) {
        tools.push({ toolName: 'inflation_linked_tips_analysis', params: buildToolParams('inflation_linked_tips_analysis', state.metrics) });
      }
      if (task.includes('repo') || task.includes('financing') || task.includes('collateral')) {
        tools.push({ toolName: 'repo_financing_haircut_analysis', params: buildToolParams('repo_financing_haircut_analysis', state.metrics) });
      }
      if (task.includes('yield') || task.includes('ytm')) {
        tools.push({ toolName: 'fixed_income_bond_yield', params: buildToolParams('fixed_income_bond_yield', state.metrics) });
      }
      if (task.includes('duration') || task.includes('convexity') || task.includes('dv01')) {
        tools.push({ toolName: 'fixed_income_duration', params: buildToolParams('fixed_income_duration', state.metrics) });
      }
      if (task.includes('nelson') || task.includes('siegel') || task.includes('curve fitting')) {
        tools.push({ toolName: 'fixed_income_nelson_siegel', params: buildToolParams('fixed_income_nelson_siegel', state.metrics) });
      }
      if (task.includes('term structure') || task.includes('hull-white')) {
        tools.push({ toolName: 'interest_rate_models_term_structure', params: buildToolParams('interest_rate_models_term_structure', state.metrics) });
      }
      // Default: at least run bond pricing and yield curve
      if (tools.length === 0) {
        tools.push({ toolName: 'fixed_income_bond_pricing', params: buildToolParams('fixed_income_bond_pricing', state.metrics) });
        tools.push({ toolName: 'fixed_income_yield_curve', params: buildToolParams('fixed_income_yield_curve', state.metrics) });
      }
    } else {
      // Deeper dives on subsequent iterations
      if (task.includes('mortgage') || task.includes('mbs') || task.includes('prepayment')) {
        tools.push({ toolName: 'mortgage_analytics_prepayment_model', params: buildToolParams('mortgage_analytics_prepayment_model', state.metrics) });
      }
      if (task.includes('municipal') || task.includes('muni') || task.includes('tax-exempt')) {
        tools.push({ toolName: 'municipal_credit_analysis', params: buildToolParams('municipal_credit_analysis', state.metrics) });
      }
      if (task.includes('sovereign') || task.includes('government') || task.includes('country')) {
        tools.push({ toolName: 'sovereign_debt_sustainability', params: buildToolParams('sovereign_debt_sustainability', state.metrics) });
      }
      if (task.includes('convexity') || task.includes('spread') || task.includes('oas')) {
        tools.push({ toolName: 'fixed_income_spread_analysis', params: buildToolParams('fixed_income_spread_analysis', state.metrics) });
      }
      if (task.includes('financial') || task.includes('statement')) {
        tools.push({ toolName: 'three_statement_model', params: buildToolParams('three_statement_model', state.metrics) });
      }
      if (task.includes('mbs') || task.includes('pass-through') || task.includes('oas')) {
        tools.push({ toolName: 'mortgage_analytics_mbs', params: buildToolParams('mortgage_analytics_mbs', state.metrics) });
      }
      if (task.includes('inflation swap') || task.includes('inflation cap')) {
        tools.push({ toolName: 'inflation_linked_derivatives', params: buildToolParams('inflation_linked_derivatives', state.metrics) });
      }
      if (task.includes('collateral') || task.includes('haircut') || task.includes('rehypothecation')) {
        tools.push({ toolName: 'repo_financing_collateral', params: buildToolParams('repo_financing_collateral', state.metrics) });
      }
      if (task.includes('muni') || task.includes('municipal') || task.includes('tax-exempt')) {
        tools.push({ toolName: 'municipal_bond_pricing', params: buildToolParams('municipal_bond_pricing', state.metrics) });
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
        statement: 'Unable to complete fixed income analysis — no tool results available',
        supportingData: {},
        confidence: 0,
        methodology: 'N/A',
        citations: [],
      });
    }

    return findings;
  }
}
