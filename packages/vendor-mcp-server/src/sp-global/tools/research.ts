import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { spFetch, wrapResponse, CacheTTL } from '../client.js';
import {
  TranscriptSchema,
  CreditRatingSchema,
  PeerAnalysisSchema,
  KeyDevSchema,
  IndustryBenchmarkSchema,
} from '../schemas/research.js';

export function registerResearchTools(server: McpServer) {
  server.tool(
    'sp_earnings_transcript',
    'Get earnings call transcript with speaker attribution including CEO, CFO, and analyst Q&A sections. Use for qualitative analysis of management commentary and guidance.',
    TranscriptSchema.shape,
    async (params) => {
      const { company_id, ticker, name, quarter, year } = TranscriptSchema.parse(params);
      const data = await spFetch('companies/transcripts', {
        company_id,
        ticker,
        name,
        quarter,
        year,
      }, { cacheTtl: CacheTTL.LONG });
      return wrapResponse(data);
    },
  );

  server.tool(
    'sp_credit_rating',
    'Get S&P credit rating and outlook including long-term and short-term ratings, rating history, and outlook status. Essential for credit risk assessment and fixed income analysis.',
    CreditRatingSchema.shape,
    async (params) => {
      const { company_id, ticker, name } = CreditRatingSchema.parse(params);
      const data = await spFetch('companies/credit-rating', {
        company_id,
        ticker,
        name,
      }, { cacheTtl: CacheTTL.LONG });
      return wrapResponse(data);
    },
  );

  server.tool(
    'sp_peer_analysis',
    'Compare company metrics against industry peers including valuation multiples, growth rates, margins, and returns. Use for relative valuation and competitive positioning.',
    PeerAnalysisSchema.shape,
    async (params) => {
      const { company_id, ticker, name, metric } = PeerAnalysisSchema.parse(params);
      const data = await spFetch('companies/peers', {
        company_id,
        ticker,
        name,
        metric,
      }, { cacheTtl: CacheTTL.MEDIUM });
      return wrapResponse(data);
    },
  );

  server.tool(
    'sp_key_developments',
    'Get key corporate developments and events including M&A activity, executive changes, earnings surprises, guidance updates, and regulatory actions. Use for event-driven analysis.',
    KeyDevSchema.shape,
    async (params) => {
      const { company_id, ticker, name, start_date, end_date, limit, offset } = KeyDevSchema.parse(params);
      const data = await spFetch('companies/key-developments', {
        company_id,
        ticker,
        name,
        start_date,
        end_date,
        limit,
        offset,
      }, { cacheTtl: CacheTTL.SHORT });
      return wrapResponse(data);
    },
  );

  server.tool(
    'sp_industry_benchmark',
    'Get industry benchmark data for comparison including median, mean, and quartile values for key financial metrics. Use for industry-level analysis and peer group construction.',
    IndustryBenchmarkSchema.shape,
    async (params) => {
      const { industry, metric } = IndustryBenchmarkSchema.parse(params);
      const data = await spFetch('industries/benchmarks', {
        industry,
        metric,
      }, { cacheTtl: CacheTTL.LONG });
      return wrapResponse(data);
    },
  );
}
