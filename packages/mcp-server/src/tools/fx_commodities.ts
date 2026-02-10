import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import {
  priceFxForward,
  calculateCrossRate,
  priceCommodityForward,
  analyzeCommodityCurve,
} from "corp-finance-bindings";
import {
  FxForwardSchema,
  CrossRateSchema,
  CommodityForwardSchema,
  CommodityCurveSchema,
} from "../schemas/fx_commodities.js";
import { wrapResponse } from "../formatters/response.js";

export function registerFxCommoditiesTools(server: McpServer) {
  server.tool(
    "fx_forward",
    "Price an FX forward using covered interest rate parity: F = S * ((1 + r_d) / (1 + r_f))^T. Supports deliverable and non-deliverable (NDF) contracts. Returns forward rate, forward points (pips), annualised premium/discount, notional in domestic currency, present value, implied rate differential, and covered interest parity check.",
    FxForwardSchema.shape,
    async (params) => {
      const validated = FxForwardSchema.parse(params);
      const result = priceFxForward(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "cross_rate",
    "Calculate a cross rate from two currency pairs sharing a common currency. For example, given USD/EUR and USD/JPY, derive EUR/JPY. Handles algebraic manipulation to identify the common currency and compute the target cross rate.",
    CrossRateSchema.shape,
    async (params) => {
      const validated = CrossRateSchema.parse(params);
      const result = calculateCrossRate(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "commodity_forward",
    "Price a commodity forward using the discrete cost-of-carry model: F = S * (1 + r + c - y)^T. Returns forward price, net cost of carry, basis, contango/backwardation classification, and approximate roll yield. Supports Energy, Metals, Agriculture, and Precious commodity types.",
    CommodityForwardSchema.shape,
    async (params) => {
      const validated = CommodityForwardSchema.parse(params);
      const result = priceCommodityForward(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "commodity_curve",
    "Analyse a commodity futures term structure. Extracts implied convenience yields from each futures price, calculates calendar spreads between consecutive contracts, determines contango vs backwardation curve shape, and computes average annualised roll yield across the curve.",
    CommodityCurveSchema.shape,
    async (params) => {
      const validated = CommodityCurveSchema.parse(params);
      const result = analyzeCommodityCurve(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );
}
