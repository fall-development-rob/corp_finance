import { z } from 'zod';
import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { eftsFetch, edgarFetch, CacheTTL } from '../client.js';
import { SearchQuerySchema, DateRangeSchema } from '../schemas/common.js';

function wrapResponse(data: unknown) {
  return { content: [{ type: 'text' as const, text: JSON.stringify(data, null, 2) }] };
}

// Full-text search schema
const FullTextSearchSchema = SearchQuerySchema.extend({
  forms: z.string().optional().describe('Comma-separated form types to filter (e.g., "10-K,10-Q")'),
  start_date: z.string().optional().describe('Start date (YYYY-MM-DD)'),
  end_date: z.string().optional().describe('End date (YYYY-MM-DD)'),
  start: z.number().int().min(0).default(0).describe('Result offset for pagination'),
});

// Advanced search schema
const AdvancedSearchSchema = SearchQuerySchema.merge(DateRangeSchema).extend({
  forms: z.string().optional().describe('Comma-separated form types (e.g., "10-K,10-Q,8-K")'),
  exact: z.boolean().default(false).describe('Exact phrase matching'),
  start: z.number().int().min(0).default(0).describe('Result offset for pagination'),
});

// Document content schema
const DocumentContentSchema = z.object({
  url: z.string().min(1).describe('Full URL of the filing document (from search results or filing index)'),
});

export function registerSearchTools(server: McpServer) {
  server.tool(
    'edgar_full_text_search',
    'Full-text search across all SEC EDGAR filings. Search for keywords, phrases, or concepts within filing documents. Filter by form type and date range. Returns matching filings with excerpts.',
    FullTextSearchSchema.shape,
    async (params) => {
      const { query, forms, start_date, end_date, start } = FullTextSearchSchema.parse(params);

      const searchParams: Record<string, string | number | boolean | undefined> = {
        q: query,
        from: start,
      };
      if (forms) searchParams.forms = forms;
      if (start_date) searchParams.startdt = start_date;
      if (end_date) searchParams.enddt = end_date;

      const data = await eftsFetch(
        'search-index',
        searchParams,
        { cacheTtl: CacheTTL.SHORT },
      );
      return wrapResponse(data);
    },
  );

  server.tool(
    'edgar_efts_search',
    'Advanced EDGAR full-text search with additional filters. Supports exact phrase matching, date ranges, and form type filtering. More granular control than basic search.',
    AdvancedSearchSchema.shape,
    async (params) => {
      const { query, forms, start_date, end_date, exact, start } = AdvancedSearchSchema.parse(params);

      const searchQuery = exact ? `"${query}"` : query;
      const searchParams: Record<string, string | number | boolean | undefined> = {
        q: searchQuery,
        from: start,
      };
      if (forms) searchParams.forms = forms;
      if (start_date) searchParams.startdt = start_date;
      if (end_date) searchParams.enddt = end_date;

      const data = await eftsFetch(
        'search-index',
        searchParams,
        { cacheTtl: CacheTTL.SHORT },
      );
      return wrapResponse(data);
    },
  );

  server.tool(
    'edgar_document_content',
    'Fetch the content of a specific SEC filing document by URL. Use URLs from search results or filing indices. Returns the raw document content (HTML/text) as JSON-wrapped text.',
    DocumentContentSchema.shape,
    async (params) => {
      const { url } = DocumentContentSchema.parse(params);

      // Validate URL is from SEC domain
      const parsedUrl = new URL(url);
      if (!parsedUrl.hostname.endsWith('.sec.gov')) {
        throw new Error('URL must be from sec.gov domain');
      }

      // Use edgarFetch for data.sec.gov URLs, eftsFetch for efts URLs
      if (parsedUrl.hostname.includes('efts')) {
        const data = await eftsFetch(
          parsedUrl.pathname + parsedUrl.search,
          {},
          { cacheTtl: CacheTTL.LONG },
        );
        return wrapResponse(data);
      }

      // Default to data.sec.gov / www.sec.gov — fetch raw
      const controller = new AbortController();
      const timeout = setTimeout(() => controller.abort(), 15_000);
      try {
        const res = await fetch(url, {
          headers: {
            'User-Agent': process.env.EDGAR_USER_AGENT || 'CFA-Agent/1.0 research@robotixai.com',
            'Accept': 'text/html, application/json, text/plain',
          },
          signal: controller.signal,
        });
        if (!res.ok) {
          throw new Error(`EDGAR document fetch failed: HTTP ${res.status}`);
        }
        const text = await res.text();
        // Truncate very large documents
        const maxLength = 100_000;
        const truncated = text.length > maxLength;
        const content = truncated ? text.slice(0, maxLength) : text;
        return wrapResponse({
          url,
          content_length: text.length,
          truncated,
          content,
        });
      } finally {
        clearTimeout(timeout);
      }
    },
  );
}
