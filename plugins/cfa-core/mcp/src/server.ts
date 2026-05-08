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
  // FP&A — Wave 16a pilot port
  variance_analysis: (json: string) => string;
  breakeven_analysis: (json: string) => string;
  working_capital: (json: string) => string;
  rolling_forecast: (json: string) => string;
  // Behavioral — Wave 16x
  prospect_theory: (json: string) => string;
  market_sentiment: (json: string) => string;
  // Performance attribution — Wave 16x
  brinson_attribution: (json: string) => string;
  factor_attribution: (json: string) => string;
  // Quant strategies — Wave 16x
  pairs_trading: (json: string) => string;
  momentum_analysis: (json: string) => string;
  // Equity research — Wave 16x
  sotp_valuation: (json: string) => string;
  target_price: (json: string) => string;
  // Commodity trading — Wave 16x
  commodity_spread: (json: string) => string;
  storage_economics: (json: string) => string;
  // Dividend policy — Wave 16x
  h_model_ddm: (json: string) => string;
  multistage_ddm: (json: string) => string;
  buyback_analysis: (json: string) => string;
  payout_sustainability: (json: string) => string;
  total_shareholder_return: (json: string) => string;
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

  // FP&A — Wave 16a pilot port
  tool(
    server,
    "variance_analysis",
    "Perform budget-vs-actual variance analysis with price/volume/mix revenue decomposition. Computes revenue variance (price, volume, and mix components), cost variance, profit variance with margin analysis, per-line detail, and optional year-over-year comparison.",
    wasm.variance_analysis,
  );

  tool(
    server,
    "breakeven_analysis",
    "Perform break-even and operating leverage analysis. Computes contribution margin, break-even units and revenue, margin of safety, operating leverage (DOL), target volume for profit goals, and what-if scenario analysis with multiple overrides.",
    wasm.breakeven_analysis,
  );

  tool(
    server,
    "working_capital",
    "Analyse working capital efficiency across multiple periods. Computes DSO, DIO, DPO, Cash Conversion Cycle, net working capital, current/quick ratios, trend analysis, optimization opportunities (cash freed by reducing DSO/DIO), financing savings, and optional industry benchmark comparison.",
    wasm.working_capital,
  );

  tool(
    server,
    "rolling_forecast",
    "Build a rolling financial forecast from historical data. Derives driver assumptions (COGS/OpEx/CapEx as % of revenue) from historical averages or overrides, projects revenue with growth rate, computes EBIT, EBITDA, net income, free cash flow, and summary statistics across forecast periods.",
    wasm.rolling_forecast,
  );

  // Behavioral — Wave 16x
  tool(
    server,
    "prospect_theory",
    "Prospect theory analysis: loss aversion, probability weighting, reference dependence, disposition effect, framing bias.",
    wasm.prospect_theory,
  );
  tool(
    server,
    "market_sentiment",
    "Market sentiment analysis: fear/greed index, put-call ratio, VIX term structure, fund flows, crowding indicators.",
    wasm.market_sentiment,
  );

  // Performance attribution — Wave 16x
  tool(
    server,
    "brinson_attribution",
    "Brinson-Fachler performance attribution: allocation, selection, interaction effects by sector with multi-period linking via Carino method.",
    wasm.brinson_attribution,
  );
  tool(
    server,
    "factor_attribution",
    "Factor-based return attribution: active exposure decomposition, R-squared, tracking error breakdown by systematic factors.",
    wasm.factor_attribution,
  );

  // Quant strategies — Wave 16x
  tool(
    server,
    "pairs_trading",
    "Statistical pairs trading analysis: cointegration, z-scores, half-life, backtested trades, Sharpe ratio.",
    wasm.pairs_trading,
  );
  tool(
    server,
    "momentum_analysis",
    "Momentum factor scoring: risk-adjusted rankings, portfolio construction, backtest, crash risk.",
    wasm.momentum_analysis,
  );

  // Equity research — Wave 16x
  tool(
    server,
    "sotp_valuation",
    "Sum-of-the-parts valuation: segment-level multiples, conglomerate discount, football field analysis.",
    wasm.sotp_valuation,
  );
  tool(
    server,
    "target_price",
    "Multi-method target price: PE, PEG, PB, PS, DDM, analyst consensus with football field and recommendation.",
    wasm.target_price,
  );

  // Commodity trading — Wave 16x
  tool(
    server,
    "commodity_spread",
    "Commodity spread analysis: crack, crush, spark, calendar, location, quality spreads with risk metrics.",
    wasm.commodity_spread,
  );
  tool(
    server,
    "storage_economics",
    "Commodity storage economics: contango/backwardation, convenience yields, cash-and-carry arbitrage, seasonal analysis.",
    wasm.storage_economics,
  );

  // Dividend policy — Wave 16x
  tool(
    server,
    "h_model_ddm",
    "H-Model DDM: Fuller & Hsia dividend valuation with linearly declining growth from short-term to long-term rate over half-life.",
    wasm.h_model_ddm,
  );
  tool(
    server,
    "multistage_ddm",
    "Multi-stage DDM: N-stage dividend discount model with explicit growth periods and terminal Gordon Growth value.",
    wasm.multistage_ddm,
  );
  tool(
    server,
    "buyback_analysis",
    "Share buyback analysis: EPS accretion/dilution, P/E breakeven, tax efficiency vs dividends, debt-funded vs cash-funded comparison.",
    wasm.buyback_analysis,
  );
  tool(
    server,
    "payout_sustainability",
    "Payout sustainability analysis: payout ratio, FCF coverage, debt capacity, dividend safety score, Lintner smoothing model.",
    wasm.payout_sustainability,
  );
  tool(
    server,
    "total_shareholder_return",
    "Total shareholder return: price appreciation, dividend yield, buyback yield, annualized TSR, component attribution.",
    wasm.total_shareholder_return,
  );

  const transport = new StdioServerTransport();
  await server.connect(transport);
  console.error(`cfa-core MCP server v${wasm.version()} listening on stdio`);
}

main().catch((err) => {
  console.error("cfa-core MCP fatal error:", err);
  process.exit(1);
});
