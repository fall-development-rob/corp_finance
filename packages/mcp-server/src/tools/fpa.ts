import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import {
  analyzeVariance,
  analyzeBreakeven,
  analyzeWorkingCapital,
  buildRollingForecast,
} from "@fall-development-rob/corp-finance-bindings";
import {
  VarianceSchema,
  BreakevenSchema,
  WorkingCapitalSchema,
  RollingForecastSchema,
} from "../schemas/fpa.js";
import { wrapResponse, coerceNumbers } from "../formatters/response.js";

export function registerFpaTools(server: McpServer) {
  server.tool(
    "variance_analysis",
    "Perform budget-vs-actual variance analysis with price/volume/mix revenue decomposition. Computes revenue variance (price, volume, and mix components), cost variance, profit variance with margin analysis, per-line detail, and optional year-over-year comparison.",
    VarianceSchema.shape,
    async (params) => {
      const validated = VarianceSchema.parse(coerceNumbers(params));
      const result = analyzeVariance(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "breakeven_analysis",
    "Perform break-even and operating leverage analysis. Computes contribution margin, break-even units and revenue, margin of safety, operating leverage (DOL), target volume for profit goals, and what-if scenario analysis with multiple overrides.",
    BreakevenSchema.shape,
    async (params) => {
      const validated = BreakevenSchema.parse(coerceNumbers(params));
      const result = analyzeBreakeven(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "working_capital",
    "Analyse working capital efficiency across multiple periods. Computes DSO, DIO, DPO, Cash Conversion Cycle, net working capital, current/quick ratios, trend analysis, optimization opportunities (cash freed by reducing DSO/DIO), financing savings, and optional industry benchmark comparison.",
    WorkingCapitalSchema.shape,
    async (params) => {
      const validated = WorkingCapitalSchema.parse(coerceNumbers(params));
      const result = analyzeWorkingCapital(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "rolling_forecast",
    "Build a rolling financial forecast from historical data. Derives driver assumptions (COGS/OpEx/CapEx as % of revenue) from historical averages or overrides, projects revenue with growth rate, computes EBIT, EBITDA, net income, free cash flow, and summary statistics across forecast periods.",
    RollingForecastSchema.shape,
    async (params) => {
      const validated = RollingForecastSchema.parse(coerceNumbers(params));
      const result = buildRollingForecast(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );
}
