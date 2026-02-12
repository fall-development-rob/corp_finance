import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { runMonteCarlo, runMcDcf } from "@robotixai/corp-finance-bindings";
import { MonteCarloSchema, McDcfSchema } from "../schemas/monte_carlo.js";
import { wrapResponse, coerceNumbers } from "../formatters/response.js";

export function registerMonteCarloTools(server: McpServer) {
  server.tool(
    "monte_carlo_simulation",
    "Run a generic Monte Carlo simulation across one or more random variables. Supports Normal, LogNormal, Triangular, and Uniform distributions. Returns per-variable statistics: mean, median, std dev, skewness, kurtosis, percentiles (P5-P95), and histogram bins.",
    MonteCarloSchema.shape,
    async (params) => {
      const validated = MonteCarloSchema.parse(coerceNumbers(params));
      const result = runMonteCarlo(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "monte_carlo_dcf",
    "Run a Monte Carlo DCF valuation with stochastic revenue growth, EBITDA margins, WACC, and terminal growth rate. Returns enterprise value distribution: mean, std dev, percentiles, implied EV range (90% CI), and probability of exceeding selected thresholds.",
    McDcfSchema.shape,
    async (params) => {
      const validated = McDcfSchema.parse(coerceNumbers(params));
      const result = runMcDcf(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );
}
