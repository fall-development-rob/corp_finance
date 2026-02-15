// LLM-based entity extraction for financial queries
// Uses Anthropic SDK (haiku) for reliable company/ticker extraction
// Replaces fragile regex patterns in parseFinancialData

import type { ExtractedMetrics } from './financial-parser.js';

export interface ExtractedEntities {
  company?: string;
  ticker?: string;
  sector?: string;
}

export type EntityExtractor = (query: string) => Promise<ExtractedEntities>;

/**
 * Create an entity extractor backed by Anthropic haiku.
 * Returns null if the SDK or API key is unavailable.
 */
export function createEntityExtractor(): EntityExtractor | null {
  const apiKey = process.env.ANTHROPIC_API_KEY;
  if (!apiKey) return null;

  // Lazy-load the SDK to avoid hard dependency
  let clientPromise: Promise<any> | null = null;

  function getClient(): Promise<any> {
    if (!clientPromise) {
      clientPromise = import('@anthropic-ai/sdk').then(
        (mod) => new mod.default({ apiKey }),
      ).catch(() => null);
    }
    return clientPromise;
  }

  return async (query: string): Promise<ExtractedEntities> => {
    const client = await getClient();
    if (!client) return {};

    try {
      const response = await client.messages.create({
        model: 'claude-haiku-4-5-20251001',
        max_tokens: 100,
        messages: [{
          role: 'user',
          content: `Extract the company name and stock ticker from this financial query. Return ONLY valid JSON, nothing else.

Query: "${query}"

Return: {"company":"<name or null>","ticker":"<ticker or null>","sector":"<sector or null>"}`,
        }],
      });

      const text = response.content?.[0]?.type === 'text'
        ? response.content[0].text.trim()
        : '';

      // Parse the JSON response
      const match = text.match(/\{[^}]+\}/);
      if (!match) return {};

      const parsed = JSON.parse(match[0]);
      return {
        company: parsed.company || undefined,
        ticker: parsed.ticker || undefined,
        sector: parsed.sector || undefined,
      };
    } catch {
      return {};
    }
  };
}

/**
 * Apply LLM-extracted entities to metrics, filling in what regex missed.
 */
export function applyEntities(
  metrics: ExtractedMetrics,
  entities: ExtractedEntities,
): void {
  if (entities.company && !metrics._company) {
    metrics._company = entities.company;
  }
  if (entities.ticker && !metrics._symbol) {
    metrics._symbol = entities.ticker;
  }
  if (entities.sector && !metrics._sector) {
    metrics._sector = entities.sector;
  }
}
