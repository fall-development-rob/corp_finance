import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import {
  tenantSchedule,
  compAdjustmentGrid,
  hbuAnalysis,
  costApproach,
  ncreifAttribution,
  acquisitionModel,
} from "../bindings.js";
import {
  RentRollSchema,
  ComparableSalesSchema,
  HbuAnalysisSchema,
  ReplacementCostSchema,
  BenchmarkSchema,
  AcquisitionSchema,
} from "../schemas/institutional_real_estate.js";
import { wrapResponse, coerceNumbers } from "../formatters/response.js";

export function registerInstitutionalRealEstateTools(server: McpServer) {
  server.tool(
    "institutional_rent_roll",
    "Argus-style tenant-by-tenant cash flow projection. Models each lease individually with escalation schedules (fixed step, CPI-linked, percentage rent, flat rent), renewal probability, downtime assumptions, and credit quality scoring. Produces year-by-year gross potential rent, effective gross income, vacancy loss, weighted average lease term (WALT), lease expiry profile, and mark-to-market gap versus current market rents.",
    RentRollSchema.shape,
    async (params) => {
      const validated = RentRollSchema.parse(coerceNumbers(params));
      const result = tenantSchedule(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "institutional_comparable_sales",
    "Sales comparison approach with structured adjustment grid. Takes a minimum of three comparable sales and applies category-level adjustments (location, condition, size, age, amenities, market conditions, financing terms, conditions of sale, property rights) to derive an adjusted price per SF for each comp. Reconciles via equal weight, quality score, or inverse distance weighting to produce an indicated value for the subject property.",
    ComparableSalesSchema.shape,
    async (params) => {
      const validated = ComparableSalesSchema.parse(coerceNumbers(params));
      const result = compAdjustmentGrid(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "institutional_hbu_analysis",
    "Highest and best use (HBU) analysis evaluating alternative development scenarios against zoning constraints. Tests each potential use for legal permissibility (zoning use class, permitted uses, FAR, height, setback, lot coverage), physical possibility, financial feasibility (residual land value via NOI capitalisation minus development cost), and maximum productivity. Returns ranked uses with residual land values, development yields, and a recommended HBU.",
    HbuAnalysisSchema.shape,
    async (params) => {
      const validated = HbuAnalysisSchema.parse(coerceNumbers(params));
      const result = hbuAnalysis(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "institutional_replacement_cost",
    "Cost approach valuation computing replacement cost new (RCN) from Marshall & Swift base costs by building class (steel, concrete, masonry, wood, metal) and occupancy type, adjusted by local cost modifier. Deducts accrued depreciation using age-life method (effective age / total economic life), plus functional and external obsolescence. Adds land value to produce an indicated value via the cost approach.",
    ReplacementCostSchema.shape,
    async (params) => {
      const validated = ReplacementCostSchema.parse(coerceNumbers(params));
      const result = costApproach(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "institutional_benchmark",
    "NCREIF-style property return attribution decomposing quarterly total returns into income return (NOI / beginning value) and appreciation return (capital value change / beginning value). Chains quarterly returns geometrically for annualised and cumulative metrics. Optionally compares against benchmark index returns to compute alpha, tracking error, and information ratio. Supports leverage adjustment via cost of debt and LTV for levered return analysis.",
    BenchmarkSchema.shape,
    async (params) => {
      const validated = BenchmarkSchema.parse(coerceNumbers(params));
      const result = ncreifAttribution(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "institutional_acquisition",
    "Full acquisition underwriting model for institutional real estate. Computes going-in cap rate, year-by-year NOI projection with growth, exit valuation via terminal cap rate, and unlevered cash flow series. Models multi-tranche debt (senior and mezzanine) with interest-only periods and amortisation schedules to produce levered cash flows. Calculates unlevered and levered IRR, NPV, equity multiple, cash-on-cash yield, DSCR by year, and go/no-go assessment against target IRR and minimum DSCR thresholds.",
    AcquisitionSchema.shape,
    async (params) => {
      const validated = AcquisitionSchema.parse(coerceNumbers(params));
      const result = acquisitionModel(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );
}
