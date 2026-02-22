import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { calculateSscmfiBond } from "../bindings.js";
import {
  SscmfiBondSchema,
  SscmfiBatchSchema,
  SscmfiPriceToYieldSchema,
  SscmfiYieldToPriceSchema,
} from "../schemas/sscmfi.js";
import { wrapResponse, coerceNumbers } from "../formatters/response.js";

export function registerSscmfiTools(server: McpServer) {
  // -------------------------------------------------------------------------
  // Full SSCMFI bond calculator — all 7 payment types
  // -------------------------------------------------------------------------
  server.tool(
    "sscmfi_bond",
    "SSCMFI-compatible bond math — price/yield, accrued interest, duration, convexity, PV01, YV32, yield-to-worst for callable bonds. Supports Periodic, Discount, IAM, Stepped, Multistep, PIK, and PartPIK payment types with 128-bit decimal precision.",
    SscmfiBondSchema.shape,
    async (params) => {
      const validated = SscmfiBondSchema.parse(coerceNumbers(params));
      const result = calculateSscmfiBond(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  // -------------------------------------------------------------------------
  // Batch — process multiple bonds in one call
  // -------------------------------------------------------------------------
  server.tool(
    "sscmfi_bond_batch",
    "Batch SSCMFI bond calculations — process up to 100 bonds in one call. Returns array of results matching input order.",
    SscmfiBatchSchema.shape,
    async (params) => {
      const validated = SscmfiBatchSchema.parse(coerceNumbers(params));
      const results = validated.bonds.map((bond) => {
        try {
          const result = calculateSscmfiBond(JSON.stringify(bond));
          return JSON.parse(String(result));
        } catch (e: unknown) {
          const message = e instanceof Error ? e.message : String(e);
          return { error: message };
        }
      });
      return {
        content: [{ type: "text" as const, text: JSON.stringify(results) }],
      };
    }
  );

  // -------------------------------------------------------------------------
  // Shortcut: price → yield (common workflow)
  // -------------------------------------------------------------------------
  server.tool(
    "sscmfi_price_to_yield",
    "Quick SSCMFI price-to-yield — given a clean price, compute yield, accrued interest, duration, and yield-to-worst for callable bonds",
    SscmfiPriceToYieldSchema.shape,
    async (params) => {
      const validated = SscmfiPriceToYieldSchema.parse(coerceNumbers(params));
      const input = {
        ...validated,
        given_type: "Price" as const,
        given_value: validated.price,
        payment_type: "Periodic" as const,
        calc_analytics: true,
        calc_cashflows: false,
      };
      const result = calculateSscmfiBond(JSON.stringify(input));
      return wrapResponse(result);
    }
  );

  // -------------------------------------------------------------------------
  // Shortcut: yield → price (common workflow)
  // -------------------------------------------------------------------------
  server.tool(
    "sscmfi_yield_to_price",
    "Quick SSCMFI yield-to-price — given a yield, compute clean price, dirty price, accrued interest, and full analytics",
    SscmfiYieldToPriceSchema.shape,
    async (params) => {
      const validated = SscmfiYieldToPriceSchema.parse(coerceNumbers(params));
      const input = {
        ...validated,
        given_type: "Yield" as const,
        given_value: validated.yield_value,
        payment_type: "Periodic" as const,
        calc_analytics: true,
        calc_cashflows: false,
      };
      const result = calculateSscmfiBond(JSON.stringify(input));
      return wrapResponse(result);
    }
  );
}
