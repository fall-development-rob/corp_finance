import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import {
  calculateReturns,
  buildDebtSchedule,
  sourcesAndUses,
  buildLbo,
  calculateWaterfall,
  altmanZscore,
} from "corp-finance-bindings";
import {
  ReturnsSchema,
  DebtScheduleSchema,
  SourcesUsesSchema,
  LboSchema,
  WaterfallSchema,
  AltmanSchema,
} from "../schemas/pe.js";
import { wrapResponse, coerceNumbers } from "../formatters/response.js";

export function registerPETools(server: McpServer) {
  server.tool(
    "returns_calculator",
    "Calculate investment returns: IRR (internal rate of return), XIRR (with irregular dates), MOIC (multiple on invested capital), and cash-on-cash return. Accepts a series of cash flows with optional dates for XIRR computation.",
    ReturnsSchema.shape,
    async (params) => {
      const validated = ReturnsSchema.parse(coerceNumbers(params));
      const result = calculateReturns(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "debt_schedule",
    "Build a multi-tranche debt amortisation schedule. Supports bullet, straight-line, and cash sweep repayment types. Models floating rates with base + spread, PIK interest, revolvers with commitment fees, and seniority-based repayment ordering.",
    DebtScheduleSchema.shape,
    async (params) => {
      const validated = DebtScheduleSchema.parse(coerceNumbers(params));
      const result = buildDebtSchedule(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "sources_uses",
    "Build a sources and uses of funds table for a transaction. Validates that total sources equal total uses. Commonly used for LBO, M&A, and recapitalisation transactions.",
    SourcesUsesSchema.shape,
    async (params) => {
      const validated = SourcesUsesSchema.parse(coerceNumbers(params));
      const result = sourcesAndUses(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "lbo_model",
    "Build a full leveraged buyout model with multi-tranche debt, cash sweep, year-by-year projections, and exit returns (IRR, MOIC). Includes sources & uses, debt schedules, and credit metrics at entry/exit.",
    LboSchema.shape,
    async (params) => {
      const validated = LboSchema.parse(coerceNumbers(params));
      const result = buildLbo(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "waterfall_calculator",
    "Calculate GP/LP distribution waterfall with return of capital, preferred return (hurdle), GP catch-up, and carried interest tiers. Supports European and American waterfall structures.",
    WaterfallSchema.shape,
    async (params) => {
      const validated = WaterfallSchema.parse(coerceNumbers(params));
      const result = calculateWaterfall(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "altman_zscore",
    "Calculate Altman Z-Score for bankruptcy prediction. Supports original Z (public manufacturing), Z-prime (private), and Z-double-prime (non-manufacturing/emerging) variants. Returns score, zone classification, and component breakdown.",
    AltmanSchema.shape,
    async (params) => {
      const validated = AltmanSchema.parse(coerceNumbers(params));
      const result = altmanZscore(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );
}
