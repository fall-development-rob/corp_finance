import { z } from 'zod';
import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { edgarFetch, eftsFetch, CacheTTL, padCik } from '../client.js';
import { CikSchema, TickerSchema } from '../schemas/common.js';

function wrapResponse(data: unknown) {
  return { content: [{ type: 'text' as const, text: JSON.stringify(data, null, 2) }] };
}

// Types for company tickers data
interface CompanyTickerEntry {
  cik_str: number;
  ticker: string;
  title: string;
}

// Schemas
const CompanySearchSchema = z.object({
  company_name: z.string().min(1).describe('Company name or partial name to search'),
  limit: z.number().int().min(1).max(100).default(10).describe('Maximum results'),
});

const SicLookupSchema = z.object({
  sic_code: z.string().min(1).describe('SIC code to look up (e.g., 7372 for prepackaged software)'),
});

const MutualFundSearchSchema = z.object({
  query: z.string().min(1).describe('Mutual fund name or ticker to search'),
  limit: z.number().int().min(1).max(100).default(10).describe('Maximum results'),
});

const SeriesSearchSchema = z.object({
  query: z.string().min(1).describe('Series or class name to search'),
  limit: z.number().int().min(1).max(100).default(10).describe('Maximum results'),
});

export function registerIdentifierTools(server: McpServer) {
  server.tool(
    'edgar_cik_from_ticker',
    'Resolve a stock ticker symbol to its SEC CIK number. Returns CIK (zero-padded), numeric CIK, ticker, and company name. Essential for using other EDGAR tools that require CIK.',
    TickerSchema.shape,
    async (params) => {
      const { ticker } = TickerSchema.parse(params);
      const data = await edgarFetch<Record<string, CompanyTickerEntry>>(
        'files/company_tickers.json',
        {},
        { cacheTtl: CacheTTL.STATIC },
      );

      const upperTicker = ticker.toUpperCase();
      for (const entry of Object.values(data)) {
        if (entry.ticker?.toUpperCase() === upperTicker) {
          return wrapResponse({
            ticker: entry.ticker,
            cik: padCik(String(entry.cik_str)),
            cik_numeric: entry.cik_str,
            company: entry.title,
          });
        }
      }

      return wrapResponse({ ticker, error: `No CIK found for ticker "${ticker}"` });
    },
  );

  server.tool(
    'edgar_ticker_from_cik',
    'Resolve a CIK number to its stock ticker symbol. Returns ticker, company name, and formatted CIK. Reverse lookup from CIK to ticker.',
    CikSchema.shape,
    async (params) => {
      const { cik } = CikSchema.parse(params);
      const data = await edgarFetch<Record<string, CompanyTickerEntry>>(
        'files/company_tickers.json',
        {},
        { cacheTtl: CacheTTL.STATIC },
      );

      const numericCik = parseInt(cik.replace(/\D/g, ''), 10);
      for (const entry of Object.values(data)) {
        if (entry.cik_str === numericCik) {
          return wrapResponse({
            cik: padCik(String(entry.cik_str)),
            cik_numeric: entry.cik_str,
            ticker: entry.ticker,
            company: entry.title,
          });
        }
      }

      return wrapResponse({ cik, error: `No ticker found for CIK "${cik}"` });
    },
  );

  server.tool(
    'edgar_company_search',
    'Search for SEC-registered companies by name. Returns matching companies with CIK numbers, tickers, and names. Supports partial name matching.',
    CompanySearchSchema.shape,
    async (params) => {
      const { company_name, limit } = CompanySearchSchema.parse(params);
      const data = await edgarFetch<Record<string, CompanyTickerEntry>>(
        'files/company_tickers.json',
        {},
        { cacheTtl: CacheTTL.STATIC },
      );

      const searchLower = company_name.toLowerCase();
      const matches: Array<{
        cik: string;
        cik_numeric: number;
        ticker: string;
        company: string;
      }> = [];

      for (const entry of Object.values(data)) {
        if (entry.title?.toLowerCase().includes(searchLower)) {
          matches.push({
            cik: padCik(String(entry.cik_str)),
            cik_numeric: entry.cik_str,
            ticker: entry.ticker,
            company: entry.title,
          });
          if (matches.length >= limit) break;
        }
      }

      return wrapResponse({
        query: company_name,
        count: matches.length,
        companies: matches,
      });
    },
  );

  server.tool(
    'edgar_sic_lookup',
    'Look up companies by SIC (Standard Industrial Classification) code. Returns all companies in the SEC database with the given SIC code along with company metadata.',
    SicLookupSchema.shape,
    async (params) => {
      const { sic_code } = SicLookupSchema.parse(params);

      // Use EFTS search to find companies by SIC
      const data = await eftsFetch(
        'search-index',
        {
          q: `"SIC ${sic_code}"`,
          forms: '10-K',
        },
        { cacheTtl: CacheTTL.LONG },
      );

      return wrapResponse({
        sic_code,
        results: data,
      });
    },
  );

  server.tool(
    'edgar_mutual_fund_search',
    'Search for mutual funds in SEC EDGAR by name or ticker. Returns matching funds from the SEC mutual fund series and classes database.',
    MutualFundSearchSchema.shape,
    async (params) => {
      const { query, limit } = MutualFundSearchSchema.parse(params);

      // Search via EFTS for mutual fund filings
      const data = await eftsFetch(
        'search-index',
        {
          q: query,
          forms: 'N-CSR,N-CEN,485BPOS,N-1A',
          from: 0,
        },
        { cacheTtl: CacheTTL.MEDIUM },
      );

      // Also check company tickers for exchange-traded entries
      const tickerData = await edgarFetch<Record<string, CompanyTickerEntry>>(
        'files/company_tickers.json',
        {},
        { cacheTtl: CacheTTL.STATIC },
      );

      const searchLower = query.toLowerCase();
      const tickerMatches: Array<{
        cik: string;
        ticker: string;
        company: string;
      }> = [];

      for (const entry of Object.values(tickerData)) {
        if (
          entry.title?.toLowerCase().includes(searchLower) ||
          entry.ticker?.toLowerCase().includes(searchLower)
        ) {
          tickerMatches.push({
            cik: padCik(String(entry.cik_str)),
            ticker: entry.ticker,
            company: entry.title,
          });
          if (tickerMatches.length >= limit) break;
        }
      }

      return wrapResponse({
        query,
        filing_results: data,
        ticker_matches: tickerMatches,
      });
    },
  );

  server.tool(
    'edgar_series_search',
    'Search for investment company series and classes in SEC EDGAR. Returns series information for mutual funds, ETFs, and other registered investment companies.',
    SeriesSearchSchema.shape,
    async (params) => {
      const { query, limit } = SeriesSearchSchema.parse(params);

      // Search via EFTS for series-related filings
      const data = await eftsFetch(
        'search-index',
        {
          q: query,
          forms: 'N-CEN,N-CSR,485BPOS',
          from: 0,
        },
        { cacheTtl: CacheTTL.MEDIUM },
      );

      return wrapResponse({
        query,
        limit,
        results: data,
      });
    },
  );
}
