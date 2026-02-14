// ADR-006: Comparative Reporter
// Produces cross-company comparison tables and relative rankings
// from individual analysis results.

import type { AnalysisResult, Finding } from '../types/agents.js';

export interface CompanyAnalysis {
  company: string;
  results: AnalysisResult[];
}

export interface ComparisonMetric {
  name: string;
  values: Map<string, number | string>;
  unit?: string;
}

/**
 * Extract comparable metrics from findings across companies.
 */
export function extractMetrics(analyses: CompanyAnalysis[]): ComparisonMetric[] {
  const metricMap = new Map<string, Map<string, number | string>>();

  for (const analysis of analyses) {
    for (const result of analysis.results) {
      for (const finding of result.findings) {
        // Extract numeric values from supportingData
        for (const [key, value] of Object.entries(finding.supportingData)) {
          if (typeof value === 'number') {
            if (!metricMap.has(key)) {
              metricMap.set(key, new Map());
            }
            metricMap.get(key)!.set(analysis.company, value);
          }
        }
      }
    }
  }

  // Only include metrics that have values for 2+ companies
  const metrics: ComparisonMetric[] = [];
  for (const [name, values] of metricMap) {
    if (values.size >= 2) {
      metrics.push({ name, values });
    }
  }

  return metrics;
}

/**
 * Format comparison metrics into a markdown table.
 */
export function formatComparisonTable(
  metrics: ComparisonMetric[],
  companies: string[],
): string {
  if (metrics.length === 0) return '';

  const lines: string[] = [];
  const header = `| Metric | ${companies.join(' | ')} |`;
  const divider = `|--------|${companies.map(() => '--------').join('|')}|`;

  lines.push(header);
  lines.push(divider);

  for (const metric of metrics) {
    const values = companies.map(c => {
      const val = metric.values.get(c);
      if (val === undefined) return '-';
      if (typeof val === 'number') return formatNumber(val);
      return String(val);
    });
    lines.push(`| ${metric.name} | ${values.join(' | ')} |`);
  }

  return lines.join('\n');
}

/**
 * Rank companies by a specific metric (higher = better by default).
 */
export function rankByMetric(
  metrics: ComparisonMetric[],
  metricName: string,
  ascending = false,
): Array<{ company: string; value: number; rank: number }> {
  const metric = metrics.find(m => m.name === metricName);
  if (!metric) return [];

  const entries: Array<{ company: string; value: number }> = [];
  for (const [company, value] of metric.values) {
    if (typeof value === 'number') {
      entries.push({ company, value });
    }
  }

  entries.sort((a, b) => ascending ? a.value - b.value : b.value - a.value);

  return entries.map((e, i) => ({ ...e, rank: i + 1 }));
}

/**
 * Identify outliers in metrics (values > 2 standard deviations from mean).
 */
export function findOutliers(
  metrics: ComparisonMetric[],
): Array<{ metric: string; company: string; value: number; direction: 'high' | 'low' }> {
  const outliers: Array<{ metric: string; company: string; value: number; direction: 'high' | 'low' }> = [];

  for (const metric of metrics) {
    const numericValues: Array<{ company: string; value: number }> = [];
    for (const [company, value] of metric.values) {
      if (typeof value === 'number') {
        numericValues.push({ company, value });
      }
    }

    if (numericValues.length < 3) continue;

    const mean = numericValues.reduce((s, v) => s + v.value, 0) / numericValues.length;
    const variance = numericValues.reduce((s, v) => s + (v.value - mean) ** 2, 0) / numericValues.length;
    const stdDev = Math.sqrt(variance);

    if (stdDev === 0) continue;

    for (const { company, value } of numericValues) {
      const zScore = (value - mean) / stdDev;
      if (Math.abs(zScore) > 2) {
        outliers.push({
          metric: metric.name,
          company,
          value,
          direction: zScore > 0 ? 'high' : 'low',
        });
      }
    }
  }

  return outliers;
}

/**
 * Build a full comparative report.
 */
export function buildComparativeReport(analyses: CompanyAnalysis[]): string {
  const companies = analyses.map(a => a.company);
  const metrics = extractMetrics(analyses);

  const lines: string[] = [
    '## Cross-Company Comparison',
    '',
  ];

  // Metrics table
  const table = formatComparisonTable(metrics, companies);
  if (table) {
    lines.push('### Key Metrics');
    lines.push('');
    lines.push(table);
    lines.push('');
  }

  // Outliers
  const outliers = findOutliers(metrics);
  if (outliers.length > 0) {
    lines.push('### Notable Outliers');
    lines.push('');
    for (const o of outliers) {
      const direction = o.direction === 'high' ? 'above' : 'below';
      lines.push(`- **${o.company}**: ${o.metric} = ${formatNumber(o.value)} (significantly ${direction} peer average)`);
    }
    lines.push('');
  }

  // Confidence comparison
  lines.push('### Analysis Confidence');
  lines.push('');
  lines.push('| Company | Avg Confidence | Agent Count |');
  lines.push('|---------|---------------|-------------|');

  for (const analysis of analyses) {
    const confs = analysis.results.map(r => r.confidence);
    const avg = confs.length > 0
      ? (confs.reduce((s, c) => s + c, 0) / confs.length).toFixed(2)
      : 'N/A';
    lines.push(`| ${analysis.company} | ${avg} | ${analysis.results.length} |`);
  }

  lines.push('');

  return lines.join('\n');
}

function formatNumber(n: number): string {
  if (Math.abs(n) >= 1e9) return `${(n / 1e9).toFixed(1)}B`;
  if (Math.abs(n) >= 1e6) return `${(n / 1e6).toFixed(1)}M`;
  if (Math.abs(n) >= 1e3) return `${(n / 1e3).toFixed(1)}K`;
  if (Number.isInteger(n)) return n.toString();
  return n.toFixed(2);
}
