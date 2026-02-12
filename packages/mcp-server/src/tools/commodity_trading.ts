import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import {
  analyzeCommoditySpread,
  analyzeStorageEconomics,
} from "@fall-development-rob/corp-finance-bindings";
import {
  CommoditySpreadSchema,
  StorageEconomicsSchema,
} from "../schemas/commodity_trading.js";
import { wrapResponse, coerceNumbers } from "../formatters/response.js";

export function registerCommodityTradingTools(server: McpServer) {
  server.tool(
    "commodity_spread",
    "Commodity spread analysis: crack, crush, spark, calendar, location, quality spreads with risk metrics",
    CommoditySpreadSchema.shape,
    async (params) => {
      const validated = CommoditySpreadSchema.parse(coerceNumbers(params));
      const result = analyzeCommoditySpread(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "storage_economics",
    "Commodity storage economics: contango/backwardation, convenience yields, cash-and-carry arbitrage, seasonal analysis",
    StorageEconomicsSchema.shape,
    async (params) => {
      const validated = StorageEconomicsSchema.parse(coerceNumbers(params));
      const result = analyzeStorageEconomics(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );
}
