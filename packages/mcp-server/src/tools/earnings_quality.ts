import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import {
  calculateBeneishMscore,
  calculatePiotroskiFscore,
  calculateAccrualQuality,
  calculateRevenueQuality,
  calculateEarningsQualityComposite,
} from "@fall-development-rob/corp-finance-bindings";
import {
  BeneishMscoreSchema,
  PiotroskiFscoreSchema,
  AccrualQualitySchema,
  RevenueQualitySchema,
  EarningsQualityCompositeSchema,
} from "../schemas/earnings_quality.js";
import { wrapResponse, coerceNumbers } from "../formatters/response.js";

export function registerEarningsQualityTools(server: McpServer) {
  server.tool(
    "beneish_mscore",
    "Beneish M-Score: 8-variable earnings manipulation model (DSRI, GMI, AQI, SGI, DEPI, SGAI, LVGI, TATA) with probability flag",
    BeneishMscoreSchema.shape,
    async (params) => {
      const validated = BeneishMscoreSchema.parse(coerceNumbers(params));
      const result = calculateBeneishMscore(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "piotroski_fscore",
    "Piotroski F-Score: 9-signal fundamental strength score (profitability, leverage, operating efficiency) with component breakdown",
    PiotroskiFscoreSchema.shape,
    async (params) => {
      const validated = PiotroskiFscoreSchema.parse(coerceNumbers(params));
      const result = calculatePiotroskiFscore(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "accrual_quality",
    "Accrual quality analysis: Sloan ratio, Dechow-Dichev model, Jones model, modified Jones model, cash conversion metrics",
    AccrualQualitySchema.shape,
    async (params) => {
      const validated = AccrualQualitySchema.parse(coerceNumbers(params));
      const result = calculateAccrualQuality(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "revenue_quality",
    "Revenue quality assessment: receivables quality, deferred revenue trends, revenue concentration (HHI), allowance analysis",
    RevenueQualitySchema.shape,
    async (params) => {
      const validated = RevenueQualitySchema.parse(coerceNumbers(params));
      const result = calculateRevenueQuality(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "earnings_quality_composite",
    "Composite earnings quality score: weighted blend of Beneish, Piotroski, accrual quality, and revenue quality with traffic-light rating",
    EarningsQualityCompositeSchema.shape,
    async (params) => {
      const validated = EarningsQualityCompositeSchema.parse(coerceNumbers(params));
      const result = calculateEarningsQualityComposite(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );
}
