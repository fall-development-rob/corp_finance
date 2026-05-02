#!/usr/bin/env node
/**
 * cfa-core MCP server.
 *
 * Loads the WASM-compiled corp-finance-wasm module and exposes its functions
 * as MCP tools. v0.1 ships 6 tools (WACC, DCF, comps, credit metrics, debt
 * capacity, covenant compliance) — the smallest set needed for the AAPL-style
 * equity workflows + credit screens.
 *
 * Schemas are deliberately permissive: the Rust side validates each input
 * struct via serde and returns descriptive errors. Adding strict zod schemas
 * is a v0.2 enhancement.
 */
import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import { z } from "zod";
import { createRequire } from "node:module";

const require = createRequire(import.meta.url);
// Resolve relative to this file so the server works whether invoked from dist/ or src/.
const wasm = require("../wasm/corp_finance_wasm.js") as {
  calculate_wacc: (json: string) => string;
  build_dcf: (json: string) => string;
  comps_analysis: (json: string) => string;
  credit_metrics: (json: string) => string;
  debt_capacity: (json: string) => string;
  covenant_compliance: (json: string) => string;
  version: () => string;
};

function wrap(jsonResult: string) {
  return {
    content: [{ type: "text" as const, text: jsonResult }],
  };
}

// v0.1: pass-through schema. The Rust side validates each input via serde and
// returns a descriptive error if a field is missing or wrongly typed. v0.2
// will add per-tool zod schemas for richer Claude Code tool hints.
const passthroughShape = {
  input: z
    .record(z.any())
    .describe("Tool inputs as a JSON object — see tool description for fields"),
};

function tool<T extends (json: string) => string>(
  server: McpServer,
  name: string,
  description: string,
  fn: T,
) {
  server.tool(
    name,
    description,
    passthroughShape,
    async (params: { input: Record<string, unknown> }) => {
      try {
        const result = fn(JSON.stringify(params.input ?? {}));
        return wrap(result);
      } catch (err) {
        const message = err instanceof Error ? err.message : String(err);
        return {
          content: [{ type: "text" as const, text: `error: ${message}` }],
          isError: true,
        };
      }
    },
  );
}

async function main() {
  const server = new McpServer({
    name: "cfa-core",
    version: wasm.version(),
  });

  tool(
    server,
    "wacc_calculator",
    "Weighted Average Cost of Capital via CAPM. Inputs (all decimals as strings or numbers): risk_free_rate, equity_risk_premium, beta, cost_of_debt, tax_rate, debt_weight, equity_weight. Optional: size_premium, country_risk_premium, specific_risk_premium, unlevered_beta, target_debt_equity (Hamada re-levering). Returns wacc, cost_of_equity, after_tax_cost_of_debt, levered_beta.",
    wasm.calculate_wacc,
  );

  tool(
    server,
    "dcf_model",
    "Discounted Cash Flow valuation with multi-stage projection and Gordon Growth or exit-multiple terminal value. Returns enterprise value, equity value, per-share value, and year-by-year projections.",
    wasm.build_dcf,
  );

  tool(
    server,
    "comps_analysis",
    "Trading comparables analysis across peer set. Computes EV/EBITDA, EV/Revenue, P/E, P/B, PEG mean/median/high/low and derives implied target valuations.",
    wasm.comps_analysis,
  );

  tool(
    server,
    "credit_metrics",
    "Standard credit metrics from financial statements: leverage ratios (Debt/EBITDA, Net Debt/EBITDA), coverage ratios (EBITDA/Interest, FCF/Debt), and liquidity ratios. Returns synthetic rating estimate.",
    wasm.credit_metrics,
  );

  tool(
    server,
    "debt_capacity",
    "Maximum sustainable debt capacity given target leverage, interest coverage, and cash flow projections. Solves backwards from covenant headroom.",
    wasm.debt_capacity,
  );

  tool(
    server,
    "covenant_compliance",
    "Test debt covenant compliance: maintenance covenants (leverage, coverage, fixed-charge), incurrence covenants, and headroom analysis with breach scenarios.",
    wasm.covenant_compliance,
  );

  const transport = new StdioServerTransport();
  await server.connect(transport);
  console.error(`cfa-core MCP server v${wasm.version()} listening on stdio`);
}

main().catch((err) => {
  console.error("cfa-core MCP fatal error:", err);
  process.exit(1);
});
