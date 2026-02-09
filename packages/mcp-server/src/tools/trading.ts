import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { analyzeTradingDay, analyzeTradingPerformance } from "corp-finance-bindings";
import { TradingDaySchema, TradingAnalyticsSchema } from "../schemas/trading.js";
import { wrapResponse } from "../formatters/response.js";

export function registerTradingTools(server: McpServer) {
  server.tool(
    "trading_day_analyzer",
    "Analyze a single trading day from a trade diary — calculates win rate, risk-reward ratio, drawdown, profit factor, and per-trade details",
    TradingDaySchema.shape,
    async (params) => {
      const validated = TradingDaySchema.parse(params);
      const result = analyzeTradingDay(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "trading_performance_analyzer",
    "Analyze multi-day trading performance — equity curve, Sharpe ratio, max drawdown, consecutive streaks, expectancy, and confidence correlation",
    TradingAnalyticsSchema.shape,
    async (params) => {
      const validated = TradingAnalyticsSchema.parse(params);
      const result = analyzeTradingPerformance(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );
}
