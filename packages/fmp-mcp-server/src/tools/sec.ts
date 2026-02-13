import { z } from 'zod';
import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { fmpFetch, CacheTTL } from '../client.js';
import {
  SecFilingsDateSchema,
  SecFilingsByFormSchema,
  SecFilingsBySymbolSchema,
  SecFilingsByCikSchema,
  SecCompanySearchNameSchema,
  SecCompanySearchSymbolSchema,
  SecCompanySearchCikSchema,
  SecProfileSchema,
} from '../schemas/sec.js';

function wrapResponse(data: unknown) {
  return { content: [{ type: 'text' as const, text: JSON.stringify(data, null, 2) }] };
}

export function registerSecTools(server: McpServer) {
  server.tool(
    'fmp_sec_filings_8k',
    'Get latest 8-K filings from the SEC. Returns recent material event disclosures across all companies.',
    SecFilingsDateSchema.shape,
    async (params) => {
      const { from, to, page, limit } = SecFilingsDateSchema.parse(params);
      const data = await fmpFetch('sec-filings-8k', { from, to, page, limit }, { cacheTtl: CacheTTL.SHORT });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_sec_filings_financials',
    'Get latest financial filings from the SEC. Returns recent 10-K and 10-Q financial statement filings.',
    SecFilingsDateSchema.shape,
    async (params) => {
      const { from, to, page, limit } = SecFilingsDateSchema.parse(params);
      const data = await fmpFetch('sec-filings-financials', { from, to, page, limit }, { cacheTtl: CacheTTL.SHORT });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_sec_filings_by_form',
    'Search SEC filings by form type (e.g., 8-K, 10-K, 10-Q, S-1). Returns filings matching the specified form type.',
    SecFilingsByFormSchema.shape,
    async (params) => {
      const { formType, from, to, page, limit } = SecFilingsByFormSchema.parse(params);
      const data = await fmpFetch('sec-filings-search/form-type', { formType, from, to, page, limit }, { cacheTtl: CacheTTL.SHORT });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_sec_filings_by_symbol',
    'Search SEC filings by stock ticker symbol. Returns all filing types for a specific company.',
    SecFilingsBySymbolSchema.shape,
    async (params) => {
      const { symbol, from, to, page, limit } = SecFilingsBySymbolSchema.parse(params);
      const data = await fmpFetch('sec-filings-search/symbol', { symbol, from, to, page, limit }, { cacheTtl: CacheTTL.SHORT });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_sec_filings_by_cik',
    'Search SEC filings by CIK number. Returns all filing types for a specific entity identified by CIK.',
    SecFilingsByCikSchema.shape,
    async (params) => {
      const { cik, from, to, page, limit } = SecFilingsByCikSchema.parse(params);
      const data = await fmpFetch('sec-filings-search/cik', { cik, from, to, page, limit }, { cacheTtl: CacheTTL.SHORT });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_sec_company_search_name',
    'Search SEC-registered companies by name. Returns matching companies with CIK, ticker, and exchange info.',
    SecCompanySearchNameSchema.shape,
    async (params) => {
      const { company } = SecCompanySearchNameSchema.parse(params);
      const data = await fmpFetch('sec-filings-company-search/name', { company }, { cacheTtl: CacheTTL.LONG });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_sec_company_search_symbol',
    'Search SEC-registered companies by stock ticker symbol. Returns company details with CIK and exchange info.',
    SecCompanySearchSymbolSchema.shape,
    async (params) => {
      const { symbol } = SecCompanySearchSymbolSchema.parse(params);
      const data = await fmpFetch('sec-filings-company-search/symbol', { symbol }, { cacheTtl: CacheTTL.LONG });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_sec_company_search_cik',
    'Search SEC-registered companies by CIK number. Returns company details with ticker and exchange info.',
    SecCompanySearchCikSchema.shape,
    async (params) => {
      const { cik } = SecCompanySearchCikSchema.parse(params);
      const data = await fmpFetch('sec-filings-company-search/cik', { cik }, { cacheTtl: CacheTTL.LONG });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_sec_profile',
    'Get full SEC company profile including CIK, SIC code, business address, filing history, and registration details.',
    SecProfileSchema.shape,
    async (params) => {
      const { symbol } = SecProfileSchema.parse(params);
      const data = await fmpFetch('sec-profile', { symbol }, { cacheTtl: CacheTTL.LONG });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_sic_list',
    'Get the full list of Standard Industrial Classification (SIC) codes. Returns all SIC codes with industry descriptions.',
    z.object({}).shape,
    async (params) => {
      z.object({}).parse(params);
      const data = await fmpFetch('standard-industrial-classification-list', {}, { cacheTtl: CacheTTL.STATIC });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_sic_search',
    'Search Standard Industrial Classification (SIC) codes by keyword. Returns matching SIC codes and industry descriptions.',
    z.object({}).shape,
    async (params) => {
      z.object({}).parse(params);
      const data = await fmpFetch('industry-classification-search', {}, { cacheTtl: CacheTTL.STATIC });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_all_sic',
    'Get all industry classifications. Returns complete list of industries with SIC codes and sector groupings.',
    z.object({}).shape,
    async (params) => {
      z.object({}).parse(params);
      const data = await fmpFetch('all-industry-classification', {}, { cacheTtl: CacheTTL.STATIC });
      return wrapResponse(data);
    },
  );
}
