import { z } from "zod";

export const BeneishMscoreSchema = z.object({
  current_receivables: z.coerce.number().describe("Current period accounts receivable"),
  prior_receivables: z.coerce.number().describe("Prior period accounts receivable"),
  current_revenue: z.coerce.number().describe("Current period revenue"),
  prior_revenue: z.coerce.number().describe("Prior period revenue"),
  current_cogs: z.coerce.number().describe("Current period cost of goods sold"),
  prior_cogs: z.coerce.number().describe("Prior period cost of goods sold"),
  current_total_assets: z.coerce.number().describe("Current period total assets"),
  prior_total_assets: z.coerce.number().describe("Prior period total assets"),
  current_ppe: z.coerce.number().describe("Current period property, plant & equipment"),
  prior_ppe: z.coerce.number().describe("Prior period property, plant & equipment"),
  current_depreciation: z.coerce.number().describe("Current period depreciation expense"),
  prior_depreciation: z.coerce.number().describe("Prior period depreciation expense"),
  current_sga: z.coerce.number().describe("Current period selling, general & administrative expense"),
  prior_sga: z.coerce.number().describe("Prior period selling, general & administrative expense"),
  current_total_debt: z.coerce.number().describe("Current period total debt"),
  prior_total_debt: z.coerce.number().describe("Prior period total debt"),
  current_net_income: z.coerce.number().describe("Current period net income"),
  current_cfo: z.coerce.number().describe("Current period cash flow from operations"),
});

export const PiotroskiFscoreSchema = z.object({
  net_income: z.coerce.number().describe("Net income"),
  total_assets: z.coerce.number().describe("Current period total assets"),
  prior_total_assets: z.coerce.number().describe("Prior period total assets"),
  cfo: z.coerce.number().describe("Cash flow from operations"),
  prior_net_income: z.coerce.number().describe("Prior period net income"),
  prior_cfo: z.coerce.number().describe("Prior period cash flow from operations"),
  current_long_term_debt: z.coerce.number().describe("Current period long-term debt"),
  prior_long_term_debt: z.coerce.number().describe("Prior period long-term debt"),
  current_current_assets: z.coerce.number().describe("Current period current assets"),
  current_current_liabilities: z.coerce.number().describe("Current period current liabilities"),
  prior_current_assets: z.coerce.number().describe("Prior period current assets"),
  prior_current_liabilities: z.coerce.number().describe("Prior period current liabilities"),
  shares_outstanding: z.coerce.number().describe("Current shares outstanding"),
  prior_shares_outstanding: z.coerce.number().describe("Prior period shares outstanding"),
  current_gross_margin: z.coerce.number().describe("Current period gross margin ratio"),
  prior_gross_margin: z.coerce.number().describe("Prior period gross margin ratio"),
  current_asset_turnover: z.coerce.number().describe("Current period asset turnover ratio"),
  prior_asset_turnover: z.coerce.number().describe("Prior period asset turnover ratio"),
});

export const AccrualQualitySchema = z.object({
  net_income: z.coerce.number().describe("Net income"),
  cfo: z.coerce.number().describe("Cash flow from operations"),
  total_assets: z.coerce.number().describe("Current period total assets"),
  prior_total_assets: z.coerce.number().describe("Prior period total assets"),
  current_assets: z.coerce.number().describe("Current period current assets"),
  prior_current_assets: z.coerce.number().describe("Prior period current assets"),
  current_liabilities: z.coerce.number().describe("Current period current liabilities"),
  prior_current_liabilities: z.coerce.number().describe("Prior period current liabilities"),
  depreciation: z.coerce.number().describe("Depreciation and amortization expense"),
  revenue: z.coerce.number().describe("Current period revenue"),
  prior_revenue: z.coerce.number().describe("Prior period revenue"),
  ppe: z.coerce.number().describe("Current period property, plant & equipment"),
  prior_ppe: z.coerce.number().describe("Prior period property, plant & equipment"),
});

export const RevenueQualitySchema = z.object({
  current_receivables: z.coerce.number().describe("Current period accounts receivable"),
  prior_receivables: z.coerce.number().describe("Prior period accounts receivable"),
  current_revenue: z.coerce.number().describe("Current period revenue"),
  prior_revenue: z.coerce.number().describe("Prior period revenue"),
  current_deferred_revenue: z.coerce.number().describe("Current period deferred revenue"),
  prior_deferred_revenue: z.coerce.number().describe("Prior period deferred revenue"),
  allowance_for_doubtful: z.coerce.number().describe("Allowance for doubtful accounts"),
  revenue_segments: z.array(z.object({
    name: z.string().describe("Segment name"),
    revenue: z.coerce.number().describe("Segment revenue"),
  })).describe("Revenue breakdown by segment for concentration analysis"),
});

export const EarningsQualityCompositeSchema = z.object({
  beneish_m_score: z.coerce.number().describe("Beneish M-Score value"),
  piotroski_f_score: z.coerce.number().int().describe("Piotroski F-Score (0-9)"),
  sloan_ratio: z.coerce.number().describe("Sloan accrual ratio"),
  cash_conversion: z.coerce.number().describe("Cash conversion ratio (CFO/Net Income)"),
  revenue_quality_score: z.coerce.number().describe("Revenue quality score (0-100)"),
  weight_beneish: z.coerce.number().optional().describe("Weight for Beneish component (default 0.25)"),
  weight_piotroski: z.coerce.number().optional().describe("Weight for Piotroski component (default 0.25)"),
  weight_accrual: z.coerce.number().optional().describe("Weight for accrual quality component (default 0.25)"),
  weight_revenue: z.coerce.number().optional().describe("Weight for revenue quality component (default 0.25)"),
});
