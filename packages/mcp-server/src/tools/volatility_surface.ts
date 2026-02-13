import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import {
  buildImpliedVolSurface,
  calibrateSabr,
} from "../bindings.js";
import {
  ImpliedVolSurfaceSchema,
  SabrCalibrationSchema,
} from "../schemas/volatility_surface.js";
import { wrapResponse, coerceNumbers } from "../formatters/response.js";

export function registerVolatilitySurfaceTools(server: McpServer) {
  server.tool(
    "implied_vol_surface",
    "Build implied volatility surface: interpolation (linear/cubic spline/SVI), Greeks surface (delta/gamma/vega/theta), skew and term structure analysis, smile fitting, arbitrage detection (calendar spread/butterfly), ATM structure, risk reversal, and butterfly spreads",
    ImpliedVolSurfaceSchema.shape,
    async (params) => {
      const validated = ImpliedVolSurfaceSchema.parse(coerceNumbers(params));
      const result = buildImpliedVolSurface(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "sabr_calibration",
    "SABR stochastic volatility model calibration: alpha/beta/rho/nu parameter estimation, Hagan approximation, model vol surface generation, calibration error analysis, ATM vol, skew, backbone, and smile curvature",
    SabrCalibrationSchema.shape,
    async (params) => {
      const validated = SabrCalibrationSchema.parse(coerceNumbers(params));
      const result = calibrateSabr(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );
}
