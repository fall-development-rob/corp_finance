import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { fmpFetch, CacheTTL } from '../client.js';
import {
  SymbolOnlySchema,
  SymbolPeriodLimitSchema,
  LatestStatementsSchema,
  FinancialReportsSchema,
  AsReportedSchema,
} from '../schemas/financials-extended.js';

function wrapResponse(data: unknown) {
  return { content: [{ type: 'text' as const, text: JSON.stringify(data, null, 2) }] };
}

export function registerFinancialExtendedTools(server: McpServer) {
  // ── TTM endpoints ──────────────────────────────────────────────────

  server.tool(
    'fmp_balance_sheet_ttm',
    'Get trailing twelve months (TTM) balance sheet. Provides the most current annualized view of assets, liabilities, and equity.',
    SymbolOnlySchema.shape,
    async (params) => {
      const { symbol } = SymbolOnlySchema.parse(params);
      const data = await fmpFetch('balance-sheet-statement-ttm', { symbol }, { cacheTtl: CacheTTL.MEDIUM });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_cash_flow_ttm',
    'Get trailing twelve months (TTM) cash flow statement. Provides the most current annualized view of operating, investing, and financing cash flows.',
    SymbolOnlySchema.shape,
    async (params) => {
      const { symbol } = SymbolOnlySchema.parse(params);
      const data = await fmpFetch('cash-flow-statement-ttm', { symbol }, { cacheTtl: CacheTTL.MEDIUM });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_key_metrics_ttm',
    'Get trailing twelve months (TTM) key financial metrics: PE, PB, EV/EBITDA, ROE, ROA, and 50+ metrics on a rolling annual basis.',
    SymbolOnlySchema.shape,
    async (params) => {
      const { symbol } = SymbolOnlySchema.parse(params);
      const data = await fmpFetch('key-metrics-ttm', { symbol }, { cacheTtl: CacheTTL.MEDIUM });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_ratios_ttm',
    'Get trailing twelve months (TTM) financial ratios: profitability, liquidity, leverage, efficiency, and valuation ratios on a rolling annual basis.',
    SymbolOnlySchema.shape,
    async (params) => {
      const { symbol } = SymbolOnlySchema.parse(params);
      const data = await fmpFetch('ratios-ttm', { symbol }, { cacheTtl: CacheTTL.MEDIUM });
      return wrapResponse(data);
    },
  );

  // ── Scoring & valuation ────────────────────────────────────────────

  server.tool(
    'fmp_financial_scores',
    'Get financial health scores: Altman Z-Score (bankruptcy risk), Piotroski F-Score (financial strength), and other quantitative scoring models.',
    SymbolOnlySchema.shape,
    async (params) => {
      const { symbol } = SymbolOnlySchema.parse(params);
      const data = await fmpFetch('financial-scores', { symbol }, { cacheTtl: CacheTTL.MEDIUM });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_owner_earnings',
    'Get Buffett-style owner earnings: net income adjusted for depreciation, capex, and working capital changes. Measures true cash-generating power.',
    SymbolOnlySchema.shape,
    async (params) => {
      const { symbol } = SymbolOnlySchema.parse(params);
      const data = await fmpFetch('owner-earnings', { symbol }, { cacheTtl: CacheTTL.MEDIUM });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_enterprise_values',
    'Get enterprise value history: market cap, total debt, cash, and resulting EV over time. Annual or quarterly.',
    SymbolPeriodLimitSchema.shape,
    async (params) => {
      const { symbol, period, limit } = SymbolPeriodLimitSchema.parse(params);
      const data = await fmpFetch('enterprise-values', { symbol, period, limit }, { cacheTtl: CacheTTL.MEDIUM });
      return wrapResponse(data);
    },
  );

  // ── Growth rates ───────────────────────────────────────────────────

  server.tool(
    'fmp_income_growth',
    'Get income statement growth rates: YoY or QoQ growth in revenue, gross profit, operating income, net income, and EPS.',
    SymbolPeriodLimitSchema.shape,
    async (params) => {
      const { symbol, period, limit } = SymbolPeriodLimitSchema.parse(params);
      const data = await fmpFetch('income-statement-growth', { symbol, period, limit }, { cacheTtl: CacheTTL.MEDIUM });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_balance_sheet_growth',
    'Get balance sheet growth rates: YoY or QoQ growth in total assets, liabilities, equity, cash, debt, and working capital.',
    SymbolPeriodLimitSchema.shape,
    async (params) => {
      const { symbol, period, limit } = SymbolPeriodLimitSchema.parse(params);
      const data = await fmpFetch('balance-sheet-statement-growth', { symbol, period, limit }, { cacheTtl: CacheTTL.MEDIUM });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_cash_flow_growth',
    'Get cash flow growth rates: YoY or QoQ growth in operating, investing, and financing cash flows, capex, and free cash flow.',
    SymbolPeriodLimitSchema.shape,
    async (params) => {
      const { symbol, period, limit } = SymbolPeriodLimitSchema.parse(params);
      const data = await fmpFetch('cash-flow-statement-growth', { symbol, period, limit }, { cacheTtl: CacheTTL.MEDIUM });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_financial_growth',
    'Get comprehensive financial growth rates: combined revenue, earnings, cash flow, margin, and per-share growth metrics over time.',
    SymbolPeriodLimitSchema.shape,
    async (params) => {
      const { symbol, period, limit } = SymbolPeriodLimitSchema.parse(params);
      const data = await fmpFetch('financial-growth', { symbol, period, limit }, { cacheTtl: CacheTTL.MEDIUM });
      return wrapResponse(data);
    },
  );

  // ── Financial reports & segments ───────────────────────────────────

  server.tool(
    'fmp_financial_reports_dates',
    'Get available financial report filing dates for a company. Returns all available 10-K and 10-Q filing dates.',
    SymbolOnlySchema.shape,
    async (params) => {
      const { symbol } = SymbolOnlySchema.parse(params);
      const data = await fmpFetch('financial-reports-dates', { symbol }, { cacheTtl: CacheTTL.LONG });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_financial_reports_json',
    'Get full 10-K or 10-Q financial report as structured JSON. Includes all line items exactly as filed with the SEC.',
    FinancialReportsSchema.shape,
    async (params) => {
      const { symbol, year, period } = FinancialReportsSchema.parse(params);
      const data = await fmpFetch('financial-reports-json', { symbol, year, period }, { cacheTtl: CacheTTL.LONG });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_revenue_product_segments',
    'Get revenue breakdown by product segment. Shows how much revenue each product line or business unit generates.',
    SymbolOnlySchema.shape,
    async (params) => {
      const { symbol } = SymbolOnlySchema.parse(params);
      const data = await fmpFetch('revenue-product-segmentation', { symbol }, { cacheTtl: CacheTTL.MEDIUM });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_revenue_geo_segments',
    'Get revenue breakdown by geographic region. Shows revenue distribution across countries and regions.',
    SymbolOnlySchema.shape,
    async (params) => {
      const { symbol } = SymbolOnlySchema.parse(params);
      const data = await fmpFetch('revenue-geographic-segmentation', { symbol }, { cacheTtl: CacheTTL.MEDIUM });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_latest_financial_statements',
    'Get the latest financial statement filings across all companies. Returns the most recent filings with pagination.',
    LatestStatementsSchema.shape,
    async (params) => {
      const { page, limit } = LatestStatementsSchema.parse(params);
      const data = await fmpFetch('latest-financial-statements', { page, limit }, { cacheTtl: CacheTTL.SHORT });
      return wrapResponse(data);
    },
  );

  // ── As-reported statements ─────────────────────────────────────────

  server.tool(
    'fmp_income_as_reported',
    'Get income statement as reported in SEC filings. Shows exact line items and values from the original filing, before any standardization.',
    AsReportedSchema.shape,
    async (params) => {
      const { symbol, period, limit } = AsReportedSchema.parse(params);
      const data = await fmpFetch('income-statement-as-reported', { symbol, period, limit }, { cacheTtl: CacheTTL.MEDIUM });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_balance_as_reported',
    'Get balance sheet as reported in SEC filings. Shows exact line items and values from the original filing, before any standardization.',
    AsReportedSchema.shape,
    async (params) => {
      const { symbol, period, limit } = AsReportedSchema.parse(params);
      const data = await fmpFetch('balance-sheet-statement-as-reported', { symbol, period, limit }, { cacheTtl: CacheTTL.MEDIUM });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_cash_flow_as_reported',
    'Get cash flow statement as reported in SEC filings. Shows exact line items and values from the original filing, before any standardization.',
    AsReportedSchema.shape,
    async (params) => {
      const { symbol, period, limit } = AsReportedSchema.parse(params);
      const data = await fmpFetch('cash-flow-statement-as-reported', { symbol, period, limit }, { cacheTtl: CacheTTL.MEDIUM });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_full_statement_as_reported',
    'Get full financial statement as reported in SEC filings. Combines income, balance sheet, and cash flow with original line items.',
    AsReportedSchema.shape,
    async (params) => {
      const { symbol, period, limit } = AsReportedSchema.parse(params);
      const data = await fmpFetch('financial-statement-full-as-reported', { symbol, period, limit }, { cacheTtl: CacheTTL.MEDIUM });
      return wrapResponse(data);
    },
  );
}
