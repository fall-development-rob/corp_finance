import { z } from 'zod';

export const ChronographListPortfoliosSchema = z.object({
  fund_id: z.string().optional().describe('Fund identifier filter'),
  status: z.enum(['active', 'realized', 'all']).default('active'),
  limit: z.number().int().min(1).max(100).default(20),
});

export const ChronographGetCompanySchema = z.object({
  company_id: z.string().min(1).describe('Portfolio company identifier'),
  include_metrics: z.boolean().default(true).describe('Include latest operational metrics in response'),
});

export const ChronographCapitalAccountSchema = z.object({
  fund_id: z.string().min(1).describe('Fund identifier'),
  lp_id: z.string().optional().describe('LP filter; omit for fund-wide aggregate'),
  as_of_date: z.string().optional().describe('YYYY-MM-DD; defaults to latest available'),
});

export const ChronographKpiPullSchema = z.object({
  company_id: z.string().min(1).describe('Portfolio company identifier'),
  metric_codes: z.array(z.string()).min(1).describe('KPI codes to retrieve, e.g. ["arr","cac","gross_margin"]'),
  period_start: z.string().optional().describe('YYYY-MM-DD; start of KPI time series window'),
  period_end: z.string().optional().describe('YYYY-MM-DD; end of KPI time series window'),
});
