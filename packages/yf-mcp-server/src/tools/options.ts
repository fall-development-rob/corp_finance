import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { yfFetch, CacheTTL, optionsUrl, extractOptions } from '../client.js';
import { SymbolSchema, OptionsDateSchema } from '../schemas/common.js';

function wrapResponse(data: unknown) {
  return { content: [{ type: 'text' as const, text: JSON.stringify(data, null, 2) }] };
}

export function registerOptionsTools(server: McpServer) {
  server.tool(
    'yf_options_expirations',
    '[UNOFFICIAL Yahoo Finance] Get available options expiration dates for a symbol. Returns list of epoch timestamps. May break without notice.',
    SymbolSchema.shape,
    async (params) => {
      const { symbol } = SymbolSchema.parse(params);
      const url = optionsUrl(symbol);
      const raw = await yfFetch<Record<string, unknown>>(url, { cacheTtl: CacheTTL.SHORT });
      const result = extractOptions(raw) as Record<string, unknown>;
      // Return just the expiration dates
      return wrapResponse({
        symbol,
        expirationDates: result.expirationDates ?? [],
        strikes: result.strikes ?? [],
      });
    },
  );

  server.tool(
    'yf_options_chain',
    '[UNOFFICIAL Yahoo Finance] Get full options chain (calls + puts) for a specific expiration date. Provide date as Unix epoch. May break without notice.',
    OptionsDateSchema.shape,
    async (params) => {
      const { symbol, date } = OptionsDateSchema.parse(params);
      const url = optionsUrl(symbol, date);
      const raw = await yfFetch<Record<string, unknown>>(url, { cacheTtl: CacheTTL.SHORT });
      const result = extractOptions(raw) as Record<string, unknown>;
      const options = (result.options as unknown[]) ?? [];
      return wrapResponse({
        symbol,
        expirationDate: date,
        options,
      });
    },
  );

  server.tool(
    'yf_options_all',
    '[UNOFFICIAL Yahoo Finance] Get options chains for ALL available expiration dates. Fetches expirations first, then chains in parallel. May break without notice.',
    SymbolSchema.shape,
    async (params) => {
      const { symbol } = SymbolSchema.parse(params);
      // Step 1: get all expiration dates
      const firstUrl = optionsUrl(symbol);
      const firstRaw = await yfFetch<Record<string, unknown>>(firstUrl, { cacheTtl: CacheTTL.SHORT });
      const firstResult = extractOptions(firstRaw) as Record<string, unknown>;
      const expirations = (firstResult.expirationDates as number[]) ?? [];

      // Step 2: fetch each expiration chain in parallel (with rate limiting built into yfFetch)
      const chains: unknown[] = [];
      // Include the first result (nearest expiry)
      const firstOptions = (firstResult.options as unknown[]) ?? [];
      if (firstOptions.length > 0) {
        chains.push(...firstOptions);
      }

      // Fetch remaining expirations
      const remaining = expirations.slice(1); // first one already fetched
      const fetches = remaining.map(async (exp) => {
        const url = optionsUrl(symbol, exp);
        const raw = await yfFetch<Record<string, unknown>>(url, { cacheTtl: CacheTTL.SHORT });
        const result = extractOptions(raw) as Record<string, unknown>;
        return (result.options as unknown[]) ?? [];
      });

      const results = await Promise.allSettled(fetches);
      for (const r of results) {
        if (r.status === 'fulfilled') chains.push(...r.value);
      }

      return wrapResponse({
        symbol,
        totalExpirations: expirations.length,
        chains,
      });
    },
  );
}
