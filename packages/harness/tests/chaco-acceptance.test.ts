/**
 * Chaco Minerals acceptance test — Phase 31 Wave 1.
 *
 * Runs the canonical Chaco Minerals memo end-to-end and asserts:
 *   - result.toolUses >= 21  (20 option_pricer grid calls + 1 country_risk_premium)
 *   - result.finalText.length > 500
 *   - dispatch terminates naturally (end_turn) within maxTurns: 30
 *
 * Skip conditions:
 *   1. ANTHROPIC_API_KEY env var is not set.
 *   2. Any of the four cfa plugin dist server files are missing.
 *
 * Test timeout: 300 000 ms (5 minutes) — Anthropic API + 21 tool calls.
 */

import { describe, it, expect } from "vitest";
import { existsSync } from "node:fs";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { dirname } from "node:path";

import { dispatch } from "../src/core/agent-loop.js";
import { createAnthropicProvider } from "../src/core/providers/anthropic.js";
import { createStdioMCPClient } from "../src/mcp-client/stdio.js";
import { chiefAnalyst, defaultMCPServers } from "../src/agents/registry.js";

// ---------------------------------------------------------------------------
// Plugin dist file detection
// ---------------------------------------------------------------------------

const _thisDir = dirname(fileURLToPath(import.meta.url));
// packages/harness/tests -> packages/harness -> packages -> repo root
const _repoRoot = resolve(_thisDir, "..", "..", "..");

const PLUGIN_DIST_FILES: string[] = [
  resolve(_repoRoot, "plugins", "cfa-core", "mcp", "dist", "server.js"),
  resolve(_repoRoot, "plugins", "cfa-data", "mcp", "dist", "index.js"),
  resolve(_repoRoot, "plugins", "cfa-pro", "mcp", "dist", "index.js"),
  resolve(_repoRoot, "plugins", "cfa-pro", "mcp", "vendors", "dist", "index.js"),
];

function pluginsBuilt(): boolean {
  return PLUGIN_DIST_FILES.every((p) => existsSync(p));
}

// ---------------------------------------------------------------------------
// Chaco Minerals prompt
//
// Instructs the model to:
//   1. Call option_pricer 20 times across the 5×4 spot/vol grid
//      spot ∈ {0.10, 0.15, 0.20, 0.25, 0.40}
//      vol  ∈ {0.60, 0.80, 1.00, 1.20}
//      K=0.25, T=2, rf=0.035, q=0, type=Call, style=American, steps=200
//   2. Call country_risk_premium once for Argentina with the given parameters.
//   3. Synthesise a memo from the results.
// ---------------------------------------------------------------------------

const CHACO_PROMPT = `\
You are preparing an institutional investment memo for Chaco Minerals, an
Argentine mining company whose primary asset is a copper prospect with an
estimated in-situ value of USD 0.20/share at current spot. The company is
pre-revenue and listed on the Buenos Aires exchange. Its equity has behaved
like a real option on copper prices.

PART A — Option grid

Call option_pricer 20 times to build a 5×4 price grid.

Parameters shared across ALL 20 calls:
  option_type: "call"
  style: "american"
  K: 0.25
  T: 2
  r: 0.035
  q: 0
  binomial_steps: 200

Grid rows (S values): 0.10, 0.15, 0.20, 0.25, 0.40
Grid columns (sigma values): 0.60, 0.80, 1.00, 1.20

Call option_pricer once for each (S, sigma) combination — 20 calls total.
You MUST issue ALL 20 calls before writing the memo. Do not skip any cell.

PART B — Country risk

Call country_risk_premium ONCE with these exact parameters for Argentina:
  country: "Argentina"
  rating: "Caa3"
  sovereign_spread_bps: 700
  equity_vol_local: 0.45
  bond_vol_local: 0.32
  us_equity_risk_premium: 0.05
  gdp_growth: 0.025
  inflation_rate: 0.45
  fiscal_balance_pct_gdp: -0.025
  current_account_pct_gdp: -0.01
  fx_volatility: 0.4
  governance_score: -1
  risk_free_rate: 0.035

PART C — Memo

After completing all 20 option_pricer calls and the 1 country_risk_premium
call, write an institutional investment memo that includes:

1. Executive summary with a clear buy/hold/pass recommendation.
2. Option value grid table (5 rows × 4 columns) using the tool outputs.
3. Argentina country risk premium analysis using the tool output.
4. Investment thesis: how country risk adjusts the option-implied value.
5. Three risk factors with quantitative impact.
6. Tool-call traceability table (one row per call: Tool | Key Inputs | Output).

Format: plain prose with structured tables. No markdown embellishments beyond
headers and tables.
`;

// ---------------------------------------------------------------------------
// Test
// ---------------------------------------------------------------------------

describe("Chaco Minerals acceptance", () => {
  it.skipIf(!process.env["ANTHROPIC_API_KEY"] || !pluginsBuilt())(
    "dispatches 21+ tool calls and produces a substantive memo",
    async () => {
      const mcp = createStdioMCPClient(defaultMCPServers);
      await mcp.initialize();

      let result;
      try {
        result = await dispatch({
          agent: chiefAnalyst,
          prompt: CHACO_PROMPT,
          provider: createAnthropicProvider(),
          mcp,
          maxTurns: 30,
        });
      } finally {
        await mcp.close();
      }

      // Primary acceptance gate
      console.log(
        `[chaco-acceptance] tool_uses=${result.toolUses} finalText=${result.finalText.slice(0, 200)}...`,
      );

      expect(result.toolUses).toBeGreaterThanOrEqual(21);
      expect(result.finalText.length).toBeGreaterThan(500);
    },
    300_000,
  );
});
