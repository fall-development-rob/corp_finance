/**
 * Tests for API-key scoping — Phase 31 Wave 4.
 * Vitest, no third-party deps beyond the harness itself.
 */

import { describe, it, expect } from "vitest";
import {
  matchesPattern,
  scopeEnvForAgent,
  defaultKeyScopes,
  createScopedEnv,
} from "../src/security/key-scoping.js";

// ---------------------------------------------------------------------------
// Test 1: matchesPattern
// ---------------------------------------------------------------------------

describe("matchesPattern", () => {
  it("prefix wildcard: fmp_quote matches fmp_*", () => {
    expect(matchesPattern("fmp_quote", "fmp_*")).toBe(true);
  });

  it("prefix wildcard: option_pricer does NOT match fmp_*", () => {
    expect(matchesPattern("option_pricer", "fmp_*")).toBe(false);
  });

  it("global wildcard: anything matches *", () => {
    expect(matchesPattern("anything", "*")).toBe(true);
    expect(matchesPattern("option_pricer", "*")).toBe(true);
    expect(matchesPattern("fmp_quote", "*")).toBe(true);
  });

  it("exact match: matches itself only", () => {
    expect(matchesPattern("option_pricer", "option_pricer")).toBe(true);
    expect(matchesPattern("option_pricer", "option_prices")).toBe(false);
  });

  it("prefix wildcard does not match a bare prefix without underscore", () => {
    // "fmp_*" requires the prefix including the trailing underscore
    expect(matchesPattern("fmp", "fmp_*")).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// Shared test env
// ---------------------------------------------------------------------------

const testEnv: NodeJS.ProcessEnv = {
  ANTHROPIC_API_KEY:   "ant-key",
  FMP_API_KEY:         "fmp-key",
  FRED_API_KEY:        "fred-key",
  ACLED_API_KEY:       "acled-key",
  ACLED_EMAIL:         "acled@test.com",
  NASA_API_KEY:        "nasa-key",
  EIA_API_KEY:         "eia-key",
  LSEG_API_KEY:        "lseg-key",
  FACTSET_API_KEY:     "factset-key",
  MOODYS_API_KEY:      "moodys-key",
  MORNINGSTAR_API_KEY: "ms-key",
  SP_API_KEY:          "sp-key",
  PITCHBOOK_API_KEY:   "pb-key",
};

const allAvailableTools = [
  "option_pricer", "wacc_calculator",
  "fmp_quote", "fmp_balance_sheet",
  "fred_series", "fred_yield_curve",
  "acled_events", "firms_fires", "eonet_events", "eia_petroleum",
  "lseg_bond_pricing", "factset_fundamentals", "moodys_credit_rating",
  "ms_fund_rating", "sp_financials", "pb_fund_search",
];

// ---------------------------------------------------------------------------
// Test 2: ANTHROPIC_API_KEY always present (tools: "*" pattern)
// ---------------------------------------------------------------------------

describe("scopeEnvForAgent — ANTHROPIC_API_KEY always injected", () => {
  it('any agent with tools: "*" receives ANTHROPIC_API_KEY', () => {
    const result = scopeEnvForAgent({
      agentTools: "*",
      availableTools: allAvailableTools,
      env: testEnv,
    });
    expect(result["ANTHROPIC_API_KEY"]).toBe("ant-key");
  });

  it("an agent with a specific allowlist also receives ANTHROPIC_API_KEY", () => {
    const result = scopeEnvForAgent({
      agentTools: ["option_pricer"],
      availableTools: allAvailableTools,
      env: testEnv,
    });
    expect(result["ANTHROPIC_API_KEY"]).toBe("ant-key");
  });
});

// ---------------------------------------------------------------------------
// Test 3: cfa-core-only agent does NOT receive FMP_API_KEY
// ---------------------------------------------------------------------------

describe("scopeEnvForAgent — cfa-core-only agent", () => {
  it("tools: [option_pricer] does NOT receive FMP_API_KEY", () => {
    const result = scopeEnvForAgent({
      agentTools: ["option_pricer"],
      availableTools: allAvailableTools,
      env: testEnv,
    });
    expect(result["FMP_API_KEY"]).toBeUndefined();
  });

  it("tools: [option_pricer, wacc_calculator] does NOT receive FMP or LSEG keys", () => {
    const result = scopeEnvForAgent({
      agentTools: ["option_pricer", "wacc_calculator"],
      availableTools: allAvailableTools,
      env: testEnv,
    });
    expect(result["FMP_API_KEY"]).toBeUndefined();
    expect(result["LSEG_API_KEY"]).toBeUndefined();
  });
});

// ---------------------------------------------------------------------------
// Test 4: FMP agent receives FMP_API_KEY but not LSEG_API_KEY
// ---------------------------------------------------------------------------

describe("scopeEnvForAgent — FMP agent", () => {
  it("tools: [fmp_quote, fmp_balance_sheet] receives FMP_API_KEY", () => {
    const result = scopeEnvForAgent({
      agentTools: ["fmp_quote", "fmp_balance_sheet"],
      availableTools: allAvailableTools,
      env: testEnv,
    });
    expect(result["FMP_API_KEY"]).toBe("fmp-key");
  });

  it("FMP agent does NOT receive LSEG_API_KEY", () => {
    const result = scopeEnvForAgent({
      agentTools: ["fmp_quote", "fmp_balance_sheet"],
      availableTools: allAvailableTools,
      env: testEnv,
    });
    expect(result["LSEG_API_KEY"]).toBeUndefined();
  });
});

// ---------------------------------------------------------------------------
// Test 5: Mixed allowlist — receives FRED but not FMP
// ---------------------------------------------------------------------------

describe("scopeEnvForAgent — mixed allowlist", () => {
  it("tools: [option_pricer, fred_series] receives FRED_API_KEY", () => {
    const result = scopeEnvForAgent({
      agentTools: ["option_pricer", "fred_series"],
      availableTools: allAvailableTools,
      env: testEnv,
    });
    expect(result["FRED_API_KEY"]).toBe("fred-key");
  });

  it("tools: [option_pricer, fred_series] does NOT receive FMP_API_KEY", () => {
    const result = scopeEnvForAgent({
      agentTools: ["option_pricer", "fred_series"],
      availableTools: allAvailableTools,
      env: testEnv,
    });
    expect(result["FMP_API_KEY"]).toBeUndefined();
  });
});

// ---------------------------------------------------------------------------
// Test 6: tools: "*" agent receives every defined scope's env var
// ---------------------------------------------------------------------------

describe("scopeEnvForAgent — wildcard agent", () => {
  it('tools: "*" receives all defined env vars that are set', () => {
    const result = scopeEnvForAgent({
      agentTools: "*",
      availableTools: allAvailableTools,
      env: testEnv,
    });
    expect(result["ANTHROPIC_API_KEY"]).toBe("ant-key");
    expect(result["FMP_API_KEY"]).toBe("fmp-key");
    expect(result["FRED_API_KEY"]).toBe("fred-key");
    expect(result["LSEG_API_KEY"]).toBe("lseg-key");
    expect(result["FACTSET_API_KEY"]).toBe("factset-key");
    expect(result["MOODYS_API_KEY"]).toBe("moodys-key");
    expect(result["MORNINGSTAR_API_KEY"]).toBe("ms-key");
    expect(result["SP_API_KEY"]).toBe("sp-key");
    expect(result["PITCHBOOK_API_KEY"]).toBe("pb-key");
  });
});

// ---------------------------------------------------------------------------
// Test 7: Env vars not set in env parameter are silently dropped
// ---------------------------------------------------------------------------

describe("scopeEnvForAgent — missing env vars dropped silently", () => {
  it("unset env vars are omitted from the result", () => {
    const partialEnv: NodeJS.ProcessEnv = {
      ANTHROPIC_API_KEY: "ant-key",
      // FMP_API_KEY intentionally absent
    };
    const result = scopeEnvForAgent({
      agentTools: ["fmp_quote"],
      availableTools: allAvailableTools,
      env: partialEnv,
    });
    expect(result["FMP_API_KEY"]).toBeUndefined();
    expect(result["ANTHROPIC_API_KEY"]).toBe("ant-key");
  });
});

// ---------------------------------------------------------------------------
// Test 8: Custom scopes override defaults
// ---------------------------------------------------------------------------

describe("scopeEnvForAgent — custom scopes", () => {
  it("caller-supplied KeyScope[] replaces defaults", () => {
    const customScopes = [
      { env_var: "MY_CUSTOM_KEY", tools: ["my_tool_*"] },
      { env_var: "SHARED_KEY",    tools: ["*"] },
    ];
    const customEnv: NodeJS.ProcessEnv = {
      MY_CUSTOM_KEY: "custom-val",
      SHARED_KEY:    "shared-val",
      FMP_API_KEY:   "fmp-key", // should NOT appear — not in customScopes
    };
    const result = scopeEnvForAgent({
      agentTools: ["my_tool_alpha", "option_pricer"],
      availableTools: ["my_tool_alpha", "option_pricer"],
      scopes: customScopes,
      env: customEnv,
    });
    expect(result["MY_CUSTOM_KEY"]).toBe("custom-val");
    expect(result["SHARED_KEY"]).toBe("shared-val");
    expect(result["FMP_API_KEY"]).toBeUndefined();
  });

  it("custom scopes with non-matching tools omit the key", () => {
    const customScopes = [
      { env_var: "MY_CUSTOM_KEY", tools: ["my_tool_*"] },
    ];
    const customEnv: NodeJS.ProcessEnv = { MY_CUSTOM_KEY: "custom-val" };
    const result = scopeEnvForAgent({
      agentTools: ["option_pricer"],
      availableTools: ["option_pricer"],
      scopes: customScopes,
      env: customEnv,
    });
    expect(result["MY_CUSTOM_KEY"]).toBeUndefined();
  });
});

// ---------------------------------------------------------------------------
// createScopedEnv integration
// ---------------------------------------------------------------------------

describe("createScopedEnv", () => {
  it("returns a ScopedEnv whose scopeForAgent delegates correctly", () => {
    const scopedEnv = createScopedEnv({ env: testEnv });
    const result = scopedEnv.scopeForAgent(["fmp_quote"]);
    expect(result["FMP_API_KEY"]).toBe("fmp-key");
    expect(result["ANTHROPIC_API_KEY"]).toBe("ant-key");
    expect(result["LSEG_API_KEY"]).toBeUndefined();
  });

  it('scopeForAgent("*") returns all available keys', () => {
    const scopedEnv = createScopedEnv({ env: testEnv });
    const result = scopedEnv.scopeForAgent("*");
    expect(result["FMP_API_KEY"]).toBe("fmp-key");
    expect(result["ANTHROPIC_API_KEY"]).toBe("ant-key");
    expect(result["LSEG_API_KEY"]).toBe("lseg-key");
  });

  it("uses custom scopes when provided", () => {
    const customScopes = [
      { env_var: "CUSTOM_KEY", tools: ["custom_*"] },
    ];
    const customEnv: NodeJS.ProcessEnv = { CUSTOM_KEY: "cval" };
    const scopedEnv = createScopedEnv({ scopes: customScopes, env: customEnv });
    const result = scopedEnv.scopeForAgent(["custom_tool"]);
    expect(result["CUSTOM_KEY"]).toBe("cval");
  });
});
