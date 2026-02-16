// ESG & Regulatory Specialist — ESG scoring, compliance, AML, FATCA/CRS, carbon markets
// Tool domains: esg, regulatory, compliance, aml_compliance, regulatory_reporting,
//               fatca_crs, substance_requirements, tax_treaty, transfer_pricing, carbon_markets

import { randomUUID } from 'node:crypto';
import type { Finding } from '../types/agents.js';
import { BaseAnalyst, type AnalystContext, type ReasoningState } from './base-analyst.js';
import { buildToolParams } from '../utils/param-builder.js';

export class EsgRegulatoryAnalyst extends BaseAnalyst {
  constructor() {
    super('esg-regulatory-analyst');
  }

  protected async think(ctx: AnalystContext, state: ReasoningState) {
    const task = ctx.task.toLowerCase();
    const tools: Array<{ toolName: string; params: Record<string, unknown> }> = [];

    if (state.iteration === 1) {
      if (task.includes('esg') || task.includes('environmental') || task.includes('social') || task.includes('governance')) {
        tools.push({ toolName: 'esg_score_calculation', params: buildToolParams('esg_score_calculation', state.metrics) });
        tools.push({ toolName: 'esg_materiality_assessment', params: buildToolParams('esg_materiality_assessment', state.metrics) });
      }
      if (task.includes('carbon') || task.includes('emission') || task.includes('climate')) {
        tools.push({ toolName: 'carbon_markets_emission_pricing', params: buildToolParams('carbon_markets_emission_pricing', state.metrics) });
        tools.push({ toolName: 'carbon_markets_offset_valuation', params: buildToolParams('carbon_markets_offset_valuation', state.metrics) });
      }
      if (task.includes('compliance') || task.includes('regulatory')) {
        tools.push({ toolName: 'compliance_check', params: buildToolParams('compliance_check', state.metrics) });
        tools.push({ toolName: 'regulatory_capital_requirement', params: buildToolParams('regulatory_capital_requirement', state.metrics) });
      }
      if (task.includes('aml') || task.includes('money laundering') || task.includes('kyc')) {
        tools.push({ toolName: 'aml_compliance_risk_assessment', params: buildToolParams('aml_compliance_risk_assessment', state.metrics) });
        tools.push({ toolName: 'aml_compliance_transaction_screening', params: buildToolParams('aml_compliance_transaction_screening', state.metrics) });
      }
      if (task.includes('fatca') || task.includes('crs') || task.includes('tax')) {
        tools.push({ toolName: 'fatca_crs_classification', params: buildToolParams('fatca_crs_classification', state.metrics) });
        tools.push({ toolName: 'tax_treaty_withholding_rate', params: buildToolParams('tax_treaty_withholding_rate', state.metrics) });
      }
      if (task.includes('transfer pricing')) {
        tools.push({ toolName: 'transfer_pricing_arm_length_test', params: buildToolParams('transfer_pricing_arm_length_test', state.metrics) });
      }
      if (task.includes('green bond') || task.includes('sustainable bond')) {
        tools.push({ toolName: 'esg_green_bond', params: buildToolParams('esg_green_bond', state.metrics) });
      }
      if (task.includes('sll') || task.includes('sustainability-linked')) {
        tools.push({ toolName: 'esg_sll_covenants', params: buildToolParams('esg_sll_covenants', state.metrics) });
      }
      if (task.includes('lcr') || task.includes('liquidity coverage')) {
        tools.push({ toolName: 'regulatory_lcr', params: buildToolParams('regulatory_lcr', state.metrics) });
      }
      if (task.includes('nsfr') || task.includes('stable funding')) {
        tools.push({ toolName: 'regulatory_nsfr', params: buildToolParams('regulatory_nsfr', state.metrics) });
      }
      if (task.includes('alm') || task.includes('asset liability')) {
        tools.push({ toolName: 'regulatory_alm', params: buildToolParams('regulatory_alm', state.metrics) });
      }
      // Default: ESG + compliance overview
      if (tools.length === 0) {
        tools.push({ toolName: 'esg_score_calculation', params: buildToolParams('esg_score_calculation', state.metrics) });
        tools.push({ toolName: 'compliance_check', params: buildToolParams('compliance_check', state.metrics) });
      }
    } else {
      // Iteration 2+: substance requirements and regulatory reporting
      if (task.includes('substance') || task.includes('jurisdiction')) {
        tools.push({ toolName: 'substance_requirements_assessment', params: buildToolParams('substance_requirements_assessment', state.metrics) });
      }
      if (task.includes('report') || task.includes('filing') || task.includes('disclosure')) {
        tools.push({ toolName: 'regulatory_reporting_requirement', params: buildToolParams('regulatory_reporting_requirement', state.metrics) });
      }
      if (task.includes('gips') || task.includes('performance standard')) {
        tools.push({ toolName: 'compliance_gips', params: buildToolParams('compliance_gips', state.metrics) });
      }
      if (task.includes('fatca report') || task.includes('crs report')) {
        tools.push({ toolName: 'fatca_crs_reporting_requirement', params: buildToolParams('fatca_crs_reporting_requirement', state.metrics) });
      }
      if (task.includes('treaty optim') || task.includes('holding structure')) {
        tools.push({ toolName: 'tax_treaty_structure_optimization', params: buildToolParams('tax_treaty_structure_optimization', state.metrics) });
      }
      if (task.includes('beps') || task.includes('pillar two') || task.includes('country-by-country')) {
        tools.push({ toolName: 'transfer_pricing_beps', params: buildToolParams('transfer_pricing_beps', state.metrics) });
      }
      if (task.includes('substance test') || task.includes('jurisdiction')) {
        tools.push({ toolName: 'substance_requirements_jurisdiction', params: buildToolParams('substance_requirements_jurisdiction', state.metrics) });
      }
      if (task.includes('sec report') || task.includes('cftc') || task.includes('form pf')) {
        tools.push({ toolName: 'regulatory_reporting_sec_cftc', params: buildToolParams('regulatory_reporting_sec_cftc', state.metrics) });
      }
      if (task.includes('us fund') || task.includes('delaware') || task.includes('reit')) {
        tools.push({ toolName: 'jurisdiction_us_fund', params: buildToolParams('jurisdiction_us_fund', state.metrics) });
      }
      if (task.includes('uk fund') || task.includes('ucits') || task.includes('sicav')) {
        tools.push({ toolName: 'jurisdiction_uk_eu_fund', params: buildToolParams('jurisdiction_uk_eu_fund', state.metrics) });
      }
      if (task.includes('cayman') || task.includes('offshore fund') || task.includes('spc')) {
        tools.push({ toolName: 'jurisdiction_cayman_fund', params: buildToolParams('jurisdiction_cayman_fund', state.metrics) });
      }
      if (task.includes('luxembourg') || task.includes('ireland') || task.includes('raif') || task.includes('icav')) {
        tools.push({ toolName: 'jurisdiction_lux_ireland_fund', params: buildToolParams('jurisdiction_lux_ireland_fund', state.metrics) });
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
