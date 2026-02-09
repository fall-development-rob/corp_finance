import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import {
  returnsCalculator,
  debtSchedule,
  sourcesUses,
} from "corp-finance-bindings";
import {
  ReturnsSchema,
  DebtScheduleSchema,
  SourcesUsesSchema,
} from "../schemas/pe.js";
import { wrapResponse } from "../formatters/response.js";

export function registerPETools(server: McpServer) {
  server.tool(
    "returns_calculator",
    "Calculate investment returns: IRR (internal rate of return), XIRR (with irregular dates), MOIC (multiple on invested capital), and cash-on-cash return. Accepts a series of cash flows with optional dates for XIRR computation.",
    ReturnsSchema.shape,
    async (params) => {
      const validated = ReturnsSchema.parse(params);
      const result = returnsCalculator(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "debt_schedule",
    "Build a multi-tranche debt amortisation schedule. Supports bullet, straight-line, and cash sweep repayment types. Models floating rates with base + spread, PIK interest, revolvers with commitment fees, and seniority-based repayment ordering.",
    DebtScheduleSchema.shape,
    async (params) => {
      const validated = DebtScheduleSchema.parse(params);
      const result = debtSchedule(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "sources_uses",
    "Build a sources and uses of funds table for a transaction. Validates that total sources equal total uses. Commonly used for LBO, M&A, and recapitalisation transactions.",
    SourcesUsesSchema.shape,
    async (params) => {
      const validated = SourcesUsesSchema.parse(params);
      const result = sourcesUses(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );
}
