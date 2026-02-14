// ADR-006: Batch Portfolio Analysis
// Runs N analyses in parallel with concurrency control and produces
// individual reports + a comparative summary.

import { Orchestrator, type OrchestratorConfig } from './coordinator.js';
import type { AnalysisResult } from '../types/agents.js';
import type { Priority } from '../types/analysis.js';

export interface BatchOptions {
  /** Max concurrent analyses (default: 3) */
  concurrency?: number;
  /** Priority for each analysis */
  priority?: Priority;
  /** Progress callback */
  onProgress?: (progress: BatchProgress) => void;
}

export interface BatchProgress {
  completed: number;
  total: number;
  current: string;
  status: 'running' | 'completed' | 'failed';
  error?: string;
}

export interface CompanyResult {
  company: string;
  report: string;
  results: AnalysisResult[];
  error?: string;
  durationMs: number;
}

export interface BatchResult {
  companies: CompanyResult[];
  comparative: string;
  totalDurationMs: number;
}

export class BatchAnalyzer {
  private orchestrator: Orchestrator;
  private config: OrchestratorConfig;

  constructor(config: OrchestratorConfig) {
    this.config = config;
    this.orchestrator = new Orchestrator(config);
  }

  /**
   * Analyze multiple companies in parallel with concurrency control.
   */
  async analyze(
    companies: string[],
    queryTemplate: string,
    options: BatchOptions = {},
  ): Promise<BatchResult> {
    const { concurrency = 3, priority = 'STANDARD', onProgress } = options;
    const totalStart = Date.now();
    const results: CompanyResult[] = [];

    // Process in batches respecting concurrency limit
    for (let i = 0; i < companies.length; i += concurrency) {
      const batch = companies.slice(i, i + concurrency);

      const batchPromises = batch.map(async (company): Promise<CompanyResult> => {
        const companyStart = Date.now();
        const query = this.buildQuery(queryTemplate, company);

        onProgress?.({
          completed: results.length,
          total: companies.length,
          current: company,
          status: 'running',
        });

        try {
          const result = await this.orchestrator.analyze(query, priority);
          const companyResult: CompanyResult = {
            company,
            report: result.report,
            results: result.results,
            durationMs: Date.now() - companyStart,
          };

          onProgress?.({
            completed: results.length + 1,
            total: companies.length,
            current: company,
            status: 'completed',
          });

          return companyResult;
        } catch (err) {
          const companyResult: CompanyResult = {
            company,
            report: '',
            results: [],
            error: err instanceof Error ? err.message : String(err),
            durationMs: Date.now() - companyStart,
          };

          onProgress?.({
            completed: results.length + 1,
            total: companies.length,
            current: company,
            status: 'failed',
            error: companyResult.error,
          });

          return companyResult;
        }
      });

      const batchResults = await Promise.all(batchPromises);
      results.push(...batchResults);
    }

    // Build comparative summary
    const comparative = this.buildComparative(results, queryTemplate);

    return {
      companies: results,
      comparative,
      totalDurationMs: Date.now() - totalStart,
    };
  }

  /**
   * Build a company-specific query from the template.
   */
  private buildQuery(template: string, company: string): string {
    // If template already mentions a company placeholder, replace it
    if (template.includes('{company}')) {
      return template.replace(/\{company\}/g, company);
    }
    // Otherwise prepend the company name
    return `${template} for ${company}`;
  }

  /**
   * Build a comparative summary from individual results.
   */
  private buildComparative(results: CompanyResult[], queryContext: string): string {
    const successful = results.filter(r => !r.error);
    const failed = results.filter(r => r.error);

    if (successful.length === 0) {
      return '## Comparative Analysis\n\nNo companies were successfully analyzed.';
    }

    const lines: string[] = [
      '## Comparative Portfolio Analysis',
      '',
      `**Query:** ${queryContext}`,
      `**Companies analyzed:** ${successful.length}/${results.length}`,
      '',
    ];

    // Summary table
    lines.push('### Results Summary');
    lines.push('');
    lines.push('| Company | Specialists | Avg Confidence | Duration |');
    lines.push('|---------|-----------|----------------|----------|');

    for (const r of successful) {
      const avgConf = r.results.length > 0
        ? (r.results.reduce((s, res) => s + res.confidence, 0) / r.results.length).toFixed(2)
        : 'N/A';
      const agents = r.results.map(res => res.agentType).join(', ');
      lines.push(`| ${r.company} | ${agents} | ${avgConf} | ${(r.durationMs / 1000).toFixed(1)}s |`);
    }

    lines.push('');

    // Individual summaries
    lines.push('### Individual Analyses');
    lines.push('');

    for (const r of successful) {
      lines.push(`#### ${r.company}`);
      lines.push('');
      // Include just the first 500 chars of each report as summary
      const summary = r.report.length > 500
        ? r.report.slice(0, 497) + '...'
        : r.report;
      lines.push(summary);
      lines.push('');
    }

    // Failed companies
    if (failed.length > 0) {
      lines.push('### Failed Analyses');
      lines.push('');
      for (const r of failed) {
        lines.push(`- **${r.company}**: ${r.error}`);
      }
      lines.push('');
    }

    return lines.join('\n');
  }
}
