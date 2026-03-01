import { z } from 'zod';
import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { edgarFetch, CacheTTL, padCik } from '../client.js';
import { CikSchema, FormTypeSchema, LimitSchema, AccessionSchema, TickerSchema } from '../schemas/common.js';

function wrapResponse(data: unknown) {
  return { content: [{ type: 'text' as const, text: JSON.stringify(data, null, 2) }] };
}

// Composite schemas
const SubmissionsSchema = CikSchema;

const FilingsFilterSchema = CikSchema.merge(FormTypeSchema).merge(LimitSchema);

const RecentFilingsSchema = CikSchema.merge(LimitSchema);

const FilingByAccessionSchema = CikSchema.merge(AccessionSchema);

// Type for submission data
interface SubmissionsData {
  cik?: string;
  entityType?: string;
  sic?: string;
  sicDescription?: string;
  name?: string;
  tickers?: string[];
  exchanges?: string[];
  filings?: {
    recent?: {
      accessionNumber?: string[];
      filingDate?: string[];
      reportDate?: string[];
      form?: string[];
      primaryDocument?: string[];
      primaryDocDescription?: string[];
      [key: string]: unknown;
    };
    files?: Array<{ name: string; filingCount: number }>;
  };
  [key: string]: unknown;
}

// Type for company tickers
interface CompanyTickerEntry {
  cik_str: number;
  ticker: string;
  title: string;
}

export function registerFilingTools(server: McpServer) {
  server.tool(
    'edgar_submissions',
    'Get full submission history for a company by CIK. Returns company metadata (name, SIC, tickers, exchanges) plus all recent filings with dates, form types, and document links.',
    SubmissionsSchema.shape,
    async (params) => {
      const { cik } = SubmissionsSchema.parse(params);
      const data = await edgarFetch(
        `submissions/CIK${padCik(cik)}.json`,
        {},
        { cacheTtl: CacheTTL.MEDIUM },
      );
      return wrapResponse(data);
    },
  );

  server.tool(
    'edgar_filings',
    'Get filings for a company filtered by form type (10-K, 10-Q, 8-K, etc.). Extracts matching filings from the submission history. Returns filing dates, accession numbers, and document links.',
    FilingsFilterSchema.shape,
    async (params) => {
      const { cik, form_type, limit } = FilingsFilterSchema.parse(params);
      const data = await edgarFetch<SubmissionsData>(
        `submissions/CIK${padCik(cik)}.json`,
        {},
        { cacheTtl: CacheTTL.MEDIUM },
      );

      const recent = data.filings?.recent;
      if (!recent || !recent.form) {
        return wrapResponse({ cik, filings: [], message: 'No filings found' });
      }

      // Filter and zip into objects
      const indices: number[] = [];
      for (let i = 0; i < recent.form.length; i++) {
        if (!form_type || recent.form[i] === form_type) {
          indices.push(i);
          if (indices.length >= limit) break;
        }
      }

      const filings = indices.map(i => ({
        accession_number: recent.accessionNumber?.[i],
        filing_date: recent.filingDate?.[i],
        report_date: recent.reportDate?.[i],
        form: recent.form?.[i],
        primary_document: recent.primaryDocument?.[i],
        primary_doc_description: recent.primaryDocDescription?.[i],
      }));

      return wrapResponse({
        cik,
        company: data.name,
        form_type_filter: form_type || 'all',
        count: filings.length,
        filings,
      });
    },
  );

  server.tool(
    'edgar_filing_by_accession',
    'Get a specific filing by CIK and accession number. Returns the filing index page with all documents in the filing package.',
    FilingByAccessionSchema.shape,
    async (params) => {
      const { cik, accession_number } = FilingByAccessionSchema.parse(params);
      const accessionClean = accession_number.replace(/-/g, '');
      const data = await edgarFetch(
        `Archives/edgar/data/${padCik(cik).replace(/^0+/, '')}/${accessionClean}/index.json`,
        {},
        { cacheTtl: CacheTTL.LONG },
      );
      return wrapResponse(data);
    },
  );

  server.tool(
    'edgar_recent_filings',
    'Get the most recent N filings for a company regardless of form type. Quick way to see latest SEC activity for any entity.',
    RecentFilingsSchema.shape,
    async (params) => {
      const { cik, limit } = RecentFilingsSchema.parse(params);
      const data = await edgarFetch<SubmissionsData>(
        `submissions/CIK${padCik(cik)}.json`,
        {},
        { cacheTtl: CacheTTL.MEDIUM },
      );

      const recent = data.filings?.recent;
      if (!recent || !recent.form) {
        return wrapResponse({ cik, filings: [], message: 'No filings found' });
      }

      const count = Math.min(limit, recent.form.length);
      const filings = [];
      for (let i = 0; i < count; i++) {
        filings.push({
          accession_number: recent.accessionNumber?.[i],
          filing_date: recent.filingDate?.[i],
          report_date: recent.reportDate?.[i],
          form: recent.form?.[i],
          primary_document: recent.primaryDocument?.[i],
          primary_doc_description: recent.primaryDocDescription?.[i],
        });
      }

      return wrapResponse({
        cik,
        company: data.name,
        count: filings.length,
        filings,
      });
    },
  );

  server.tool(
    'edgar_company_tickers',
    'Get the complete SEC ticker-to-CIK mapping. Returns all companies with their tickers, CIK numbers, and names. Use for bulk lookups or building a local mapping table.',
    z.object({}).shape,
    async (params) => {
      z.object({}).parse(params);
      const data = await edgarFetch(
        'files/company_tickers.json',
        {},
        { cacheTtl: CacheTTL.STATIC },
      );
      return wrapResponse(data);
    },
  );

  server.tool(
    'edgar_cik_lookup',
    'Look up a CIK number from a ticker symbol. Searches the SEC company_tickers mapping. Returns CIK, ticker, and company name.',
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
            cik: String(entry.cik_str).padStart(10, '0'),
            cik_numeric: entry.cik_str,
            company: entry.title,
          });
        }
      }

      return wrapResponse({ ticker, error: `No CIK found for ticker ${ticker}` });
    },
  );
}
