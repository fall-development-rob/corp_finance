import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import {
  modelFundingRound,
  analyzeDilution,
  convertNote,
  convertSafe,
  modelVentureFund,
} from "@robotixai/corp-finance-bindings";
import {
  FundingRoundSchema,
  DilutionSchema,
  ConvertibleNoteSchema,
  SafeSchema,
  VentureFundSchema,
} from "../schemas/venture.js";
import { wrapResponse, coerceNumbers } from "../formatters/response.js";

export function registerVentureTools(server: McpServer) {
  server.tool(
    "funding_round",
    "Model a single VC funding round with option-pool shuffle. Calculates post-money valuation, price per share, new shares issued, option pool expansion, investor ownership, founder dilution, and produces a fully-diluted cap table.",
    FundingRoundSchema.shape,
    async (params) => {
      const validated = FundingRoundSchema.parse(coerceNumbers(params));
      const result = modelFundingRound(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "dilution_analysis",
    "Analyse dilution across multiple funding rounds. Tracks founder ownership trajectory through Seed, Series A, B, etc., showing per-round price, shares issued, option pool increases, and final cap table.",
    DilutionSchema.shape,
    async (params) => {
      const validated = DilutionSchema.parse(coerceNumbers(params));
      const result = analyzeDilution(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "convertible_note",
    "Convert a convertible note into equity at the most favourable price for the note-holder. Applies discount rate and/or valuation cap, computes accrued interest, shares issued, effective valuation, discount savings, and post-conversion ownership percentage.",
    ConvertibleNoteSchema.shape,
    async (params) => {
      const validated = ConvertibleNoteSchema.parse(coerceNumbers(params));
      const result = convertNote(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "safe_conversion",
    "Convert a SAFE (Simple Agreement for Future Equity) into shares at a qualifying financing event. Supports both pre-money and YC-style post-money SAFEs with valuation cap and/or discount. Returns conversion price, shares issued, effective valuation, and ownership percentage.",
    SafeSchema.shape,
    async (params) => {
      const validated = SafeSchema.parse(coerceNumbers(params));
      const result = convertSafe(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "venture_fund_model",
    "Model a venture capital fund's returns over its full lifecycle. Computes yearly cash flows (J-curve), fund-level metrics (net/gross IRR, DPI, TVPI, RVPI, MOIC), carried interest, portfolio statistics, and per-investment results.",
    VentureFundSchema.shape,
    async (params) => {
      const validated = VentureFundSchema.parse(coerceNumbers(params));
      const result = modelVentureFund(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );
}
