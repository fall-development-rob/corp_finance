import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { yfFetch, CacheTTL, quoteSummaryUrl, extractQuoteSummary } from '../client.js';
import { SymbolSchema } from '../schemas/common.js';

function wrapResponse(data: unknown) {
  return { content: [{ type: 'text' as const, text: JSON.stringify(data, null, 2) }] };
}

export function registerInfoTools(server: McpServer) {
  server.tool(
    'yf_info',
    '[UNOFFICIAL Yahoo Finance] Get full company profile: sector, industry, employees, description, website, key statistics, financial data. May break without notice.',
    SymbolSchema.shape,
    async (params) => {
      const { symbol } = SymbolSchema.parse(params);
      const url = quoteSummaryUrl(symbol, [
        'assetProfile',
        'financialData',
        'defaultKeyStatistics',
      ]);
      const raw = await yfFetch<Record<string, unknown>>(url, { cacheTtl: CacheTTL.LONG });
      const data = extractQuoteSummary(raw);
      return wrapResponse(data);
    },
  );

  server.tool(
    'yf_analyst_targets',
    '[UNOFFICIAL Yahoo Finance] Get analyst price targets and financial data: target mean, high, low, recommendation, number of analysts. May break without notice.',
    SymbolSchema.shape,
    async (params) => {
      const { symbol } = SymbolSchema.parse(params);
      const url = quoteSummaryUrl(symbol, [
        'financialData',
      ]);
      const raw = await yfFetch<Record<string, unknown>>(url, { cacheTtl: CacheTTL.MEDIUM });
      const summary = extractQuoteSummary(raw) as Record<string, unknown>;
      const financialData = summary.financialData as Record<string, unknown> | undefined;
      // Extract just the analyst-relevant fields
      if (financialData) {
        return wrapResponse({
          symbol,
          targetHighPrice: financialData.targetHighPrice,
          targetLowPrice: financialData.targetLowPrice,
          targetMeanPrice: financialData.targetMeanPrice,
          targetMedianPrice: financialData.targetMedianPrice,
          recommendationMean: financialData.recommendationMean,
          recommendationKey: financialData.recommendationKey,
          numberOfAnalystOpinions: financialData.numberOfAnalystOpinions,
          currentPrice: financialData.currentPrice,
        });
      }
      return wrapResponse(summary);
    },
  );

  server.tool(
    'yf_upgrades_downgrades',
    '[UNOFFICIAL Yahoo Finance] Get analyst upgrade/downgrade history: firm, action (upgrade/downgrade/initiated), from grade, to grade, date. May break without notice.',
    SymbolSchema.shape,
    async (params) => {
      const { symbol } = SymbolSchema.parse(params);
      const url = quoteSummaryUrl(symbol, [
        'upgradeDowngradeHistory',
      ]);
      const raw = await yfFetch<Record<string, unknown>>(url, { cacheTtl: CacheTTL.MEDIUM });
      const data = extractQuoteSummary(raw);
      return wrapResponse(data);
    },
  );
}
