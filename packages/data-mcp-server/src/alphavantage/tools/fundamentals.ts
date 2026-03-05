import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { avFetch, CacheTTL } from '../client.js';
import { SymbolSchema, EarningsCalendarSchema } from '../schemas/common.js';

function wrapResponse(data: unknown) {
  return { content: [{ type: 'text' as const, text: JSON.stringify(data, null, 2) }] };
}

export function registerFundamentalTools(server: McpServer) {
  server.tool(
    'av_company_overview',
    'Get comprehensive company profile from Alpha Vantage: description, sector, industry, market cap, PE, EPS, book value, dividend yield, 52-week range, analyst target, and 50+ fundamental metrics.',
    SymbolSchema.shape,
    async (params) => {
      const { symbol } = SymbolSchema.parse(params);
      const data = await avFetch({ function: 'COMPANY_OVERVIEW', symbol }, { cacheTtl: CacheTTL.LONG });
      return wrapResponse(data);
    },
  );

  server.tool(
    'av_income_statement',
    'Get annual and quarterly income statements: revenue, gross profit, operating income, EBITDA, net income, EPS. Up to 5 years annual and 20 quarters.',
    SymbolSchema.shape,
    async (params) => {
      const { symbol } = SymbolSchema.parse(params);
      const data = await avFetch({ function: 'INCOME_STATEMENT', symbol }, { cacheTtl: CacheTTL.LONG });
      return wrapResponse(data);
    },
  );

  server.tool(
    'av_balance_sheet',
    'Get annual and quarterly balance sheets: total assets, liabilities, equity, cash, debt, current ratio. Up to 5 years annual and 20 quarters.',
    SymbolSchema.shape,
    async (params) => {
      const { symbol } = SymbolSchema.parse(params);
      const data = await avFetch({ function: 'BALANCE_SHEET', symbol }, { cacheTtl: CacheTTL.LONG });
      return wrapResponse(data);
    },
  );

  server.tool(
    'av_cash_flow',
    'Get annual and quarterly cash flow statements: operating, investing, financing cash flows, capex, dividends, free cash flow. Up to 5 years annual and 20 quarters.',
    SymbolSchema.shape,
    async (params) => {
      const { symbol } = SymbolSchema.parse(params);
      const data = await avFetch({ function: 'CASH_FLOW', symbol }, { cacheTtl: CacheTTL.LONG });
      return wrapResponse(data);
    },
  );

  server.tool(
    'av_earnings',
    'Get annual and quarterly earnings: reported EPS, estimated EPS, surprise, and surprise percentage. Essential for earnings analysis and estimate tracking.',
    SymbolSchema.shape,
    async (params) => {
      const { symbol } = SymbolSchema.parse(params);
      const data = await avFetch({ function: 'EARNINGS', symbol }, { cacheTtl: CacheTTL.LONG });
      return wrapResponse(data);
    },
  );

  server.tool(
    'av_earnings_calendar',
    'Get upcoming earnings dates and EPS estimates across the market or for a specific symbol. Look ahead 3, 6, or 12 months.',
    EarningsCalendarSchema.shape,
    async (params) => {
      const { horizon, symbol } = EarningsCalendarSchema.parse(params);
      const fetchParams: Record<string, string> = { function: 'EARNINGS_CALENDAR', horizon };
      if (symbol) fetchParams.symbol = symbol;
      const data = await avFetch(fetchParams, { cacheTtl: CacheTTL.MEDIUM });
      return wrapResponse(data);
    },
  );

  server.tool(
    'av_ipo_calendar',
    'Get upcoming IPO dates with company name, symbol, price range, and exchange. Useful for tracking new market listings.',
    {},
    async () => {
      const data = await avFetch({ function: 'IPO_CALENDAR' }, { cacheTtl: CacheTTL.MEDIUM });
      return wrapResponse(data);
    },
  );
}
