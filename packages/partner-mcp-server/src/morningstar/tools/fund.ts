import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { msFetch, CacheTTL } from '../client.js';
import {
  FundRatingSchema,
  FundHoldingsSchema,
  FundPerformanceSchema,
  HistoricalNavSchema,
  ExpenseSchema,
} from '../schemas/fund.js';

function wrapResponse(data: unknown) {
  return { content: [{ type: 'text' as const, text: JSON.stringify(data, null, 2) }] };
}

export function registerFundTools(server: McpServer) {
  server.tool(
    'ms_fund_rating',
    'Get Morningstar star rating and analyst medal (Gold/Silver/Bronze). Returns quantitative star rating (1-5), analyst medal, risk-adjusted return rank, and category assignment.',
    FundRatingSchema.shape,
    async (params) => {
      const { fund_id, isin, ticker } = FundRatingSchema.parse(params);
      const data = await msFetch('fund/rating', {
        fund_id, isin, ticker,
      }, { cacheTtl: CacheTTL.MEDIUM });
      return wrapResponse(data);
    },
  );

  server.tool(
    'ms_fund_holdings',
    'Get fund top holdings with sector and geographic allocation. Returns top holdings by weight, sector breakdown, geographic exposure, and asset class distribution.',
    FundHoldingsSchema.shape,
    async (params) => {
      const { fund_id, isin, ticker } = FundHoldingsSchema.parse(params);
      const data = await msFetch('fund/holdings', {
        fund_id, isin, ticker,
      }, { cacheTtl: CacheTTL.MEDIUM });
      return wrapResponse(data);
    },
  );

  server.tool(
    'ms_fund_performance',
    'Get fund performance returns across time periods. Returns 1M, 3M, 6M, YTD, 1Y, 3Y, 5Y, 10Y total returns, benchmark comparison, and risk-adjusted metrics.',
    FundPerformanceSchema.shape,
    async (params) => {
      const { fund_id, isin, ticker } = FundPerformanceSchema.parse(params);
      const data = await msFetch('fund/performance', {
        fund_id, isin, ticker,
      }, { cacheTtl: CacheTTL.SHORT });
      return wrapResponse(data);
    },
  );

  server.tool(
    'ms_historical_nav',
    'Get historical NAV time series. Returns daily or periodic net asset value data for charting and analysis over a specified date range.',
    HistoricalNavSchema.shape,
    async (params) => {
      const { fund_id, isin, ticker, start_date, end_date } = HistoricalNavSchema.parse(params);
      const data = await msFetch('fund/nav/historical', {
        fund_id, isin, ticker, start_date, end_date,
      }, { cacheTtl: CacheTTL.SHORT });
      return wrapResponse(data);
    },
  );

  server.tool(
    'ms_expense_analysis',
    'Get expense ratio analysis and fee breakdown. Returns gross/net expense ratio, management fee, 12b-1 fee, load charges, and fee comparison against category median.',
    ExpenseSchema.shape,
    async (params) => {
      const { fund_id, isin, ticker } = ExpenseSchema.parse(params);
      const data = await msFetch('fund/expenses', {
        fund_id, isin, ticker,
      }, { cacheTtl: CacheTTL.STATIC });
      return wrapResponse(data);
    },
  );
}
