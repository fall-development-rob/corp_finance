import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import {
  analyzeProspectTheory,
  analyzeSentiment,
} from "@fall-development-rob/corp-finance-bindings";
import {
  ProspectTheorySchema,
  SentimentSchema,
} from "../schemas/behavioral.js";
import { wrapResponse, coerceNumbers } from "../formatters/response.js";

export function registerBehavioralTools(server: McpServer) {
  server.tool(
    "prospect_theory",
    "Prospect theory analysis: loss aversion, probability weighting, reference dependence, disposition effect, framing bias",
    ProspectTheorySchema.shape,
    async (params) => {
      const validated = ProspectTheorySchema.parse(coerceNumbers(params));
      const result = analyzeProspectTheory(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "market_sentiment",
    "Market sentiment analysis: fear/greed index, put-call ratio, VIX term structure, fund flows, crowding indicators",
    SentimentSchema.shape,
    async (params) => {
      const validated = SentimentSchema.parse(coerceNumbers(params));
      const result = analyzeSentiment(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );
}
