import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { fmpFetch, CacheTTL } from '../client.js';
import {
  EtfSymbolSchema, EtfAssetExposureSchema, FundDisclosureSchema,
  FundDisclosureSearchSchema, FundDisclosureDatesSchema, FundDisclosureLatestSchema,
} from '../schemas/etf.js';

function wrapResponse(data: unknown) {
  return { content: [{ type: 'text' as const, text: JSON.stringify(data, null, 2) }] };
}

export function registerEtfTools(server: McpServer) {
  server.tool(
    'fmp_etf_holdings',
    'Get ETF holdings/constituents: full list of stocks held by an ETF with weights and share counts. Use for portfolio decomposition.',
    EtfSymbolSchema.shape,
    async (params) => {
      const { symbol } = EtfSymbolSchema.parse(params);
      const data = await fmpFetch('etf/holdings', { symbol }, { cacheTtl: CacheTTL.MEDIUM });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_etf_info',
    'Get ETF or mutual fund information: inception date, expense ratio, AUM, asset class, strategy, and issuer details.',
    EtfSymbolSchema.shape,
    async (params) => {
      const { symbol } = EtfSymbolSchema.parse(params);
      const data = await fmpFetch('etf/info', { symbol }, { cacheTtl: CacheTTL.LONG });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_etf_country_weightings',
    'Get ETF country allocation: percentage breakdown of holdings by country. Use for geographic exposure analysis.',
    EtfSymbolSchema.shape,
    async (params) => {
      const { symbol } = EtfSymbolSchema.parse(params);
      const data = await fmpFetch('etf/country-weightings', { symbol }, { cacheTtl: CacheTTL.MEDIUM });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_etf_asset_exposure',
    'Find which ETFs hold a given stock: returns list of ETFs with exposure to a specific ticker symbol and their weight.',
    EtfAssetExposureSchema.shape,
    async (params) => {
      const { symbol } = EtfAssetExposureSchema.parse(params);
      const data = await fmpFetch('etf/asset-exposure', { symbol }, { cacheTtl: CacheTTL.MEDIUM });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_etf_sector_weightings',
    'Get ETF sector allocation: percentage breakdown of holdings by sector. Use for sector exposure analysis.',
    EtfSymbolSchema.shape,
    async (params) => {
      const { symbol } = EtfSymbolSchema.parse(params);
      const data = await fmpFetch('etf/sector-weightings', { symbol }, { cacheTtl: CacheTTL.MEDIUM });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_fund_disclosure_holders',
    'Get latest fund holders for a stock: shows which mutual funds and institutions hold a given ticker with share counts and values.',
    FundDisclosureLatestSchema.shape,
    async (params) => {
      const { symbol } = FundDisclosureLatestSchema.parse(params);
      const data = await fmpFetch('funds/disclosure-holders-latest', { symbol }, { cacheTtl: CacheTTL.MEDIUM });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_fund_disclosure',
    'Get fund disclosure by period: detailed holdings for a specific fund in a given year and quarter.',
    FundDisclosureSchema.shape,
    async (params) => {
      const { symbol, year, quarter } = FundDisclosureSchema.parse(params);
      const data = await fmpFetch('funds/disclosure', { symbol, year, quarter }, { cacheTtl: CacheTTL.LONG });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_fund_disclosure_search',
    'Search fund disclosures by fund name: find disclosure filings matching a fund name keyword.',
    FundDisclosureSearchSchema.shape,
    async (params) => {
      const { name } = FundDisclosureSearchSchema.parse(params);
      const data = await fmpFetch('funds/disclosure-holders-search', { name }, { cacheTtl: CacheTTL.LONG });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_fund_disclosure_dates',
    'Get available disclosure dates for a fund: returns list of reporting periods with disclosure filings on record.',
    FundDisclosureDatesSchema.shape,
    async (params) => {
      const { symbol } = FundDisclosureDatesSchema.parse(params);
      const data = await fmpFetch('funds/disclosure-dates', { symbol }, { cacheTtl: CacheTTL.LONG });
      return wrapResponse(data);
    },
  );
}
