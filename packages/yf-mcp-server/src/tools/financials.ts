import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { yfFetch, CacheTTL, quoteSummaryUrl, extractQuoteSummary } from '../client.js';
import { SymbolSchema } from '../schemas/common.js';

function wrapResponse(data: unknown) {
  return { content: [{ type: 'text' as const, text: JSON.stringify(data, null, 2) }] };
}

export function registerFinancialTools(server: McpServer) {
  server.tool(
    'yf_income_statement',
    '[UNOFFICIAL Yahoo Finance] Get income statement data (annual + quarterly): revenue, gross profit, EBITDA, net income, EPS. May break without notice.',
    SymbolSchema.shape,
    async (params) => {
      const { symbol } = SymbolSchema.parse(params);
      const url = quoteSummaryUrl(symbol, [
        'incomeStatementHistory',
        'incomeStatementHistoryQuarterly',
      ]);
      const raw = await yfFetch<Record<string, unknown>>(url, { cacheTtl: CacheTTL.MEDIUM });
      const data = extractQuoteSummary(raw);
      return wrapResponse(data);
    },
  );

  server.tool(
    'yf_balance_sheet',
    '[UNOFFICIAL Yahoo Finance] Get balance sheet data (annual + quarterly): assets, liabilities, equity, cash, debt. May break without notice.',
    SymbolSchema.shape,
    async (params) => {
      const { symbol } = SymbolSchema.parse(params);
      const url = quoteSummaryUrl(symbol, [
        'balanceSheetHistory',
        'balanceSheetHistoryQuarterly',
      ]);
      const raw = await yfFetch<Record<string, unknown>>(url, { cacheTtl: CacheTTL.MEDIUM });
      const data = extractQuoteSummary(raw);
      return wrapResponse(data);
    },
  );

  server.tool(
    'yf_cash_flow',
    '[UNOFFICIAL Yahoo Finance] Get cash flow statement (annual + quarterly): operating, investing, financing flows, capex, free cash flow. May break without notice.',
    SymbolSchema.shape,
    async (params) => {
      const { symbol } = SymbolSchema.parse(params);
      const url = quoteSummaryUrl(symbol, [
        'cashflowStatementHistory',
        'cashflowStatementHistoryQuarterly',
      ]);
      const raw = await yfFetch<Record<string, unknown>>(url, { cacheTtl: CacheTTL.MEDIUM });
      const data = extractQuoteSummary(raw);
      return wrapResponse(data);
    },
  );

  server.tool(
    'yf_earnings',
    '[UNOFFICIAL Yahoo Finance] Get earnings history and trends: actual vs estimate EPS, surprise %, forward estimates. May break without notice.',
    SymbolSchema.shape,
    async (params) => {
      const { symbol } = SymbolSchema.parse(params);
      const url = quoteSummaryUrl(symbol, [
        'earningsHistory',
        'earningsTrend',
      ]);
      const raw = await yfFetch<Record<string, unknown>>(url, { cacheTtl: CacheTTL.MEDIUM });
      const data = extractQuoteSummary(raw);
      return wrapResponse(data);
    },
  );
}
