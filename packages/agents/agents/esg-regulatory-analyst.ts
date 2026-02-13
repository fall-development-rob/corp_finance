// ESG & Regulatory Specialist — ESG scoring, compliance, AML, FATCA/CRS, carbon markets
// Tool domains: esg, regulatory, compliance, aml_compliance, regulatory_reporting,
//               fatca_crs, substance_requirements, tax_treaty, transfer_pricing, carbon_markets

import { randomUUID } from 'node:crypto';
import type { Finding } from '../types/agents.js';
import { BaseAnalyst, type AnalystContext, type ReasoningState } from './base-analyst.js';

export class EsgRegulatoryAnalyst extends BaseAnalyst {
  constructor() {
    super('esg-regulatory-analyst');
  }

  protected async think(ctx: AnalystContext, state: ReasoningState) {
    const task = ctx.task.toLowerCase();
    const tools: Array<{ toolName: string; params: Record<string, unknown> }> = [];

    if (state.iteration === 1) {
      if (task.includes('esg') || task.includes('environmental') || task.includes('social') || task.includes('governance')) {
        tools.push({ toolName: 'esg_score_calculation', params: { query: ctx.task } });
        tools.push({ toolName: 'esg_materiality_assessment', params: { query: ctx.task } });
      }
      if (task.includes('carbon') || task.includes('emission') || task.includes('climate')) {
        tools.push({ toolName: 'carbon_markets_emission_pricing', params: { query: ctx.task } });
        tools.push({ toolName: 'carbon_markets_offset_valuation', params: { query: ctx.task } });
      }
      if (task.includes('compliance') || task.includes('regulatory')) {
        tools.push({ toolName: 'compliance_check', params: { query: ctx.task } });
        tools.push({ toolName: 'regulatory_capital_requirement', params: { query: ctx.task } });
      }
      if (task.includes('aml') || task.includes('money laundering') || task.includes('kyc')) {
        tools.push({ toolName: 'aml_compliance_risk_assessment', params: { query: ctx.task } });
        tools.push({ toolName: 'aml_compliance_transaction_screening', params: { query: ctx.task } });
      }
      if (task.includes('fatca') || task.includes('crs') || task.includes('tax')) {
        tools.push({ toolName: 'fatca_crs_classification', params: { query: ctx.task } });
        tools.push({ toolName: 'tax_treaty_withholding_rate', params: { query: ctx.task } });
      }
      if (task.includes('transfer pricing')) {
        tools.push({ toolName: 'transfer_pricing_arm_length_test', params: { query: ctx.task } });
      }
      // Default: ESG + compliance overview
      if (tools.length === 0) {
        tools.push({ toolName: 'esg_score_calculation', params: { query: ctx.task } });
        tools.push({ toolName: 'compliance_check', params: { query: ctx.task } });
      }
    } else {
      // Iteration 2+: substance requirements and regulatory reporting
      if (task.includes('substance') || task.includes('jurisdiction')) {
        tools.push({ toolName: 'substance_requirements_assessment', params: { query: ctx.task } });
      }
      if (task.includes('report') || task.includes('filing') || task.includes('disclosure')) {
        tools.push({ toolName: 'regulatory_reporting_requirement', params: { query: ctx.task } });
      }
    }

    return tools;
  }

  protected async reflect(_ctx: AnalystContext, state: ReasoningState) {
    const successCount = state.toolResults.filter(t => !t.error).length;
    const shouldIterate = state.iteration === 1 && successCount > 0 && state.toolResults.length < 4;

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
        statement: `Unable to complete analysis — no tool results available`,
        supportingData: {},
        confidence: 0,
        methodology: 'N/A',
        citations: [],
      });
    }

    return findings;
  }
}
