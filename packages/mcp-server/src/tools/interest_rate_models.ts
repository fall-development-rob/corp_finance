import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import {
  analyzeShortRate,
  fitTermStructure,
} from "corp-finance-bindings";
import {
  ShortRateSchema,
  TermStructureSchema,
} from "../schemas/interest_rate_models.js";
import { wrapResponse, coerceNumbers } from "../formatters/response.js";

export function registerInterestRateModelsTools(server: McpServer) {
  server.tool(
    "short_rate_model",
    "Short rate model analysis: Vasicek (mean-reverting Gaussian), CIR (square-root diffusion, non-negative), Hull-White (market-calibrated). Outputs expected rate path, variance, zero-coupon bond prices, yield to maturity, forward rates, Feller condition (CIR), theta calibration (HW)",
    ShortRateSchema.shape,
    async (params) => {
      const validated = ShortRateSchema.parse(coerceNumbers(params));
      const result = analyzeShortRate(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "term_structure_fit",
    "Yield curve fitting: Nelson-Siegel (4-param level/slope/curvature), Svensson (6-param extended NS), Bootstrapping (exact fit from par/zero/swap instruments). Outputs fitted parameters, rates, residuals, RMSE, R-squared, discount factors, forward rates",
    TermStructureSchema.shape,
    async (params) => {
      const validated = TermStructureSchema.parse(coerceNumbers(params));
      const result = fitTermStructure(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );
}
