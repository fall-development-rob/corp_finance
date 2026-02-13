import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { fmpFetch, CacheTTL } from '../client.js';
import {
  IncomeStatementSchema, BalanceSheetSchema, CashFlowSchema,
  IncomeTtmSchema, KeyMetricsSchema, FinancialRatiosSchema,
} from '../schemas/financials.js';

function wrapResponse(data: unknown) {
  return { content: [{ type: 'text' as const, text: JSON.stringify(data, null, 2) }] };
}

export function registerFinancialTools(server: McpServer) {
  server.tool(
    'fmp_income_statement',
    'Get income statement data: revenue, cost of revenue, gross profit, operating expenses, EBITDA, net income, EPS. Annual or quarterly periods.',
    IncomeStatementSchema.shape,
    async (params) => {
      const { symbol, period, limit } = IncomeStatementSchema.parse(params);
      const data = await fmpFetch('income-statement', { symbol, period, limit }, { cacheTtl: CacheTTL.MEDIUM });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_balance_sheet',
    'Get balance sheet data: total assets, total liabilities, equity, cash, debt, inventory, receivables, payables. Annual or quarterly.',
    BalanceSheetSchema.shape,
    async (params) => {
      const { symbol, period, limit } = BalanceSheetSchema.parse(params);
      const data = await fmpFetch('balance-sheet-statement', { symbol, period, limit }, { cacheTtl: CacheTTL.MEDIUM });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_cash_flow',
    'Get cash flow statement: operating, investing, and financing cash flows, capex, free cash flow, dividends paid. Annual or quarterly.',
    CashFlowSchema.shape,
    async (params) => {
      const { symbol, period, limit } = CashFlowSchema.parse(params);
      const data = await fmpFetch('cash-flow-statement', { symbol, period, limit }, { cacheTtl: CacheTTL.MEDIUM });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_income_ttm',
    'Get trailing twelve months (TTM) income statement. Provides the most current annualized view of the income statement.',
    IncomeTtmSchema.shape,
    async (params) => {
      const { symbol } = IncomeTtmSchema.parse(params);
      const data = await fmpFetch('income-statement-ttm', { symbol }, { cacheTtl: CacheTTL.MEDIUM });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_key_metrics',
    'Get key financial metrics: PE ratio, PB ratio, EV/EBITDA, debt-to-equity, ROE, ROA, current ratio, dividend yield, and 50+ more. Annual or quarterly.',
    KeyMetricsSchema.shape,
    async (params) => {
      const { symbol, period, limit } = KeyMetricsSchema.parse(params);
      const data = await fmpFetch('key-metrics', { symbol, period, limit }, { cacheTtl: CacheTTL.MEDIUM });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_financial_ratios',
    'Get comprehensive financial ratios: profitability, liquidity, leverage, efficiency, valuation, and growth ratios. Annual or quarterly.',
    FinancialRatiosSchema.shape,
    async (params) => {
      const { symbol, period, limit } = FinancialRatiosSchema.parse(params);
      const data = await fmpFetch('ratios', { symbol, period, limit }, { cacheTtl: CacheTTL.MEDIUM });
      return wrapResponse(data);
    },
  );
}
