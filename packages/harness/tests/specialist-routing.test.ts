/**
 * Specialist routing acceptance test — Phase 31 Wave 2.
 *
 * Runs the canonical Chaco Minerals memo through the chief-analyst with
 * the 8 specialists wired as delegation targets. Asserts:
 *   • chief invoked at least one `delegate_to_*` virtual tool
 *   • the dispatch produced child dispatches with non-zero tool_uses each
 *   • total tool_uses across chief + specialists ≥ 21
 *   • the final memo is substantive (length > 500)
 *
 * Skipped if ANTHROPIC_API_KEY isn't set or any plugin server.js is missing.
 */

import { existsSync } from "node:fs";
import { describe, expect, it } from "vitest";
import {
  chiefAnalyst,
  createAnthropicProvider,
  createStdioMCPClient,
  defaultDelegates,
  defaultMCPServers,
  dispatch,
} from "../src/index.js";
import {
  delegationToolName,
  isDelegationToolName,
} from "../src/agents/delegate.js";
import type { DispatchEvent } from "../src/types.js";

const CHACO_PROMPT = `\
You are dispatching an institutional IC memo for the Chaco Minerals Ltd. \
private placement (Argentina lithium-brine junior, C$5M raise, pre-money C$16.9M, \
unsecured convertible debentures with mandatory conversion to Units = 1 share \
+ 1 warrant K=C$0.25 T=2y, conversion price LESSER of C$0.10 or 30-day VWAP, \
3% PIK in equity, listing horizon Q1 2026).

Acceptance contract for THIS dispatch:
  • You MUST delegate the warrant valuation grid to the derivatives specialist \
via \`delegate_to_derivatives_analyst\`. Pass a self-contained sub_prompt \
asking the specialist to produce a 5×4 grid using \`option_pricer\`: \
spots {0.10, 0.15, 0.20, 0.25, 0.40} × vols {0.60, 0.80, 1.00, 1.20}, \
common params strike_price=0.25, time_to_expiry=2.0, risk_free_rate=0.035, \
dividend_yield=0, option_type="Call", exercise_style="American", \
binomial_steps=200.
  • You MUST delegate Argentina country risk to the macro specialist via \
\`delegate_to_macro_analyst\`. Pass a self-contained sub_prompt asking for the \
country risk premium calculation using \`country_risk_premium\` with: \
country="Argentina", country_rating="Caa3", sovereign_spread_bps=700, \
equity_vol_local=0.45, bond_vol_local=0.32, us_equity_risk_premium=0.05, \
gdp_growth=0.025, inflation_rate=0.45, fiscal_balance_pct_gdp=-0.025, \
current_account_pct_gdp=-0.01, fx_volatility=0.4, governance_score=-1, \
risk_free_rate=0.035.
  • Assemble the specialists' outputs into a brief institutional memo with \
sections: warrant valuation grid, country risk + cost-of-equity, asymmetries, \
recommendation (PASS / CONDITIONAL SPECULATIVE BUY / BUY).

Do not invoke option_pricer or country_risk_premium yourself — the specialists \
will. Your role is to delegate, aggregate, and produce the memo.`;

function shouldSkip(): { skip: boolean; reason: string } {
  if (!process.env["ANTHROPIC_API_KEY"]) {
    return { skip: true, reason: "ANTHROPIC_API_KEY not set" };
  }
  for (const server of defaultMCPServers) {
    const path = server.args[0];
    if (!path || !existsSync(path)) {
      return {
        skip: true,
        reason: `MCP server "${server.name}" dist not found at ${path ?? "<missing args>"}`,
      };
    }
  }
  return { skip: false, reason: "" };
}

describe("specialist routing acceptance — Wave 2", () => {
  const skip = shouldSkip();

  it.skipIf(skip.skip)(
    "chief delegates warrant grid to derivatives + country risk to macro, total tool_uses ≥ 21",
    async () => {
      const mcp = createStdioMCPClient(defaultMCPServers);
      await mcp.initialize();

      try {
        const provider = createAnthropicProvider();

        const events: DispatchEvent[] = [];
        const result = await dispatch({
          agent: chiefAnalyst,
          prompt: CHACO_PROMPT,
          provider,
          mcp,
          delegates: defaultDelegates,
          maxTurns: 30,
          onEvent: (e) => {
            events.push(e);
          },
        });

        const delegationCalls = events.filter(
          (e) => e.type === "tool_call" && isDelegationToolName(e.call.name),
        );
        const delegateNamesCalled = new Set(
          delegationCalls.flatMap((e) =>
            e.type === "tool_call" ? [e.call.name] : [],
          ),
        );

        const childToolUses = (result.childDispatches ?? []).reduce(
          (sum, c) => sum + c.toolUses,
          0,
        );
        const totalToolUses = result.toolUses + childToolUses;

        // eslint-disable-next-line no-console
        console.log(
          `[wave-2-routing] chief.toolUses=${result.toolUses} ` +
            `children=${(result.childDispatches ?? []).length} ` +
            `child.toolUses=${childToolUses} total=${totalToolUses} ` +
            `delegations=[${[...delegateNamesCalled].join(", ")}]`,
        );

        // The chief must have delegated to at least the derivatives + macro specialists.
        expect(delegateNamesCalled.has(delegationToolName("derivatives-analyst"))).toBe(true);
        expect(delegateNamesCalled.has(delegationToolName("macro-analyst"))).toBe(true);

        // At least one child dispatch must have run real tool calls.
        expect(result.childDispatches?.length).toBeGreaterThanOrEqual(2);
        for (const child of result.childDispatches ?? []) {
          expect(child.toolUses).toBeGreaterThan(0);
        }

        // Total tool calls (chief + specialists) must clear the Phase 30 baseline.
        expect(totalToolUses).toBeGreaterThanOrEqual(21);

        // Final memo must be substantive.
        expect(result.finalText.length).toBeGreaterThan(500);
      } finally {
        await mcp.close();
      }
    },
    600_000, // 10 minute test timeout — chief + 2 specialists is more model traffic than Wave 1
  );

  it.skipIf(skip.skip)("skips if ANTHROPIC_API_KEY is unset or plugin dist is missing", () => {
    expect(skip.skip).toBe(false);
  });
});
