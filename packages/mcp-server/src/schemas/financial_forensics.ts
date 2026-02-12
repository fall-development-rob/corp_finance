import { z } from "zod";

export const BenfordsLawSchema = z.object({
  data_points: z.array(z.coerce.number()).describe("Array of numeric data points to test"),
  test_type: z.string().describe("Test type (first_digit, second_digit, first_two_digits)"),
  significance_level: z.coerce.number().describe("Statistical significance level (e.g., 0.05)"),
});

export const DupontSchema = z.object({
  net_income: z.coerce.number().describe("Net income"),
  revenue: z.coerce.number().describe("Total revenue"),
  total_assets: z.coerce.number().describe("Total assets"),
  shareholders_equity: z.coerce.number().describe("Total shareholders equity"),
  ebt: z.coerce.number().describe("Earnings before tax"),
  ebit: z.coerce.number().describe("Earnings before interest and tax"),
  interest_expense: z.coerce.number().describe("Interest expense"),
  tax_expense: z.coerce.number().describe("Tax expense"),
  prior_net_income: z.coerce.number().optional().describe("Prior period net income (for trend analysis)"),
  prior_revenue: z.coerce.number().optional().describe("Prior period revenue (for trend analysis)"),
  prior_total_assets: z.coerce.number().optional().describe("Prior period total assets (for trend analysis)"),
  prior_equity: z.coerce.number().optional().describe("Prior period shareholders equity (for trend analysis)"),
});

export const ZScoreModelsSchema = z.object({
  working_capital: z.coerce.number().describe("Working capital (current assets - current liabilities)"),
  total_assets: z.coerce.number().describe("Total assets"),
  retained_earnings: z.coerce.number().describe("Retained earnings"),
  ebit: z.coerce.number().describe("Earnings before interest and tax"),
  market_cap: z.coerce.number().describe("Market capitalization"),
  book_equity: z.coerce.number().describe("Book value of equity"),
  total_liabilities: z.coerce.number().describe("Total liabilities"),
  revenue: z.coerce.number().describe("Total revenue"),
  net_income: z.coerce.number().describe("Net income"),
  total_debt: z.coerce.number().describe("Total debt"),
  current_assets: z.coerce.number().describe("Current assets"),
  current_liabilities: z.coerce.number().describe("Current liabilities"),
  cash_flow_operations: z.coerce.number().describe("Cash flow from operations"),
  is_public: z.boolean().describe("Whether the company is publicly traded"),
  is_manufacturing: z.boolean().describe("Whether the company is in manufacturing sector"),
});

export const PeerBenchmarkingSchema = z.object({
  company: z.object({
    name: z.string().describe("Company name"),
    metrics: z.array(z.object({
      metric_name: z.string().describe("Metric name (e.g., ROE, D/E, revenue_growth)"),
      value: z.coerce.number().describe("Metric value"),
    })).describe("Array of company metrics"),
  }).describe("Target company metrics"),
  peers: z.array(z.object({
    name: z.string().describe("Peer company name"),
    metrics: z.array(z.object({
      metric_name: z.string().describe("Metric name"),
      value: z.coerce.number().describe("Metric value"),
    })).describe("Array of peer metrics"),
  })).describe("Array of peer companies for benchmarking"),
  higher_is_better: z.array(z.string()).describe("Metric names where higher values are better (e.g., ROE, margins)"),
  lower_is_better: z.array(z.string()).describe("Metric names where lower values are better (e.g., D/E, cost ratios)"),
});

export const RedFlagScoringSchema = z.object({
  beneish_m_score: z.coerce.number().optional().describe("Beneish M-Score value (if available)"),
  altman_z_score: z.coerce.number().optional().describe("Altman Z-Score value (if available)"),
  piotroski_f_score: z.coerce.number().int().optional().describe("Piotroski F-Score 0-9 (if available)"),
  cfo_to_net_income: z.coerce.number().describe("Cash flow from operations to net income ratio"),
  revenue_growth: z.coerce.number().describe("Revenue growth rate as decimal"),
  receivables_growth: z.coerce.number().describe("Receivables growth rate as decimal"),
  inventory_growth: z.coerce.number().describe("Inventory growth rate as decimal"),
  sga_to_revenue_change: z.coerce.number().describe("Change in SGA-to-revenue ratio"),
  debt_to_equity: z.coerce.number().describe("Debt-to-equity ratio"),
  interest_coverage: z.coerce.number().describe("Interest coverage ratio (EBIT/interest)"),
  audit_opinion: z.string().describe("Audit opinion type (unqualified, qualified, adverse, disclaimer)"),
  auditor_change: z.boolean().describe("Whether there was a recent auditor change"),
  related_party_transactions: z.boolean().describe("Whether significant related-party transactions exist"),
  restatement_history: z.boolean().describe("Whether the company has recent restatement history"),
});
