import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import {
  analyzePairsTrading,
  analyzeMomentum,
} from "corp-finance-bindings";
import {
  PairsTradingSchema,
  MomentumSchema,
} from "../schemas/quant_strategies.js";
import { wrapResponse, coerceNumbers } from "../formatters/response.js";

export function registerQuantStrategiesTools(server: McpServer) {
  server.tool(
    "pairs_trading",
    "Statistical pairs trading analysis: cointegration, z-scores, half-life, backtested trades, Sharpe ratio",
    PairsTradingSchema.shape,
    async (params) => {
      const validated = PairsTradingSchema.parse(coerceNumbers(params));
      const result = analyzePairsTrading(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "momentum_analysis",
    "Momentum factor scoring: risk-adjusted rankings, portfolio construction, backtest, crash risk",
    MomentumSchema.shape,
    async (params) => {
      const validated = MomentumSchema.parse(coerceNumbers(params));
      const result = analyzeMomentum(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );
}
