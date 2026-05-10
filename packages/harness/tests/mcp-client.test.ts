/**
 * Tests for the MCP client: name-resolver unit tests and stdio integration test.
 */
import { describe, it, expect } from "vitest";
import { existsSync } from "node:fs";
import { buildNameMap, resolveBareToWire, extractBareFromWire } from "../src/mcp-client/name-resolver.js";
import { createStdioMCPClient } from "../src/mcp-client/stdio.js";
import type { MCPServerConfig } from "../src/types.js";

// ---------------------------------------------------------------------------
// name-resolver unit tests
// ---------------------------------------------------------------------------

describe("name-resolver", () => {
  const servers: MCPServerConfig[] = [
    {
      name: "cfa-core",
      prefix: "mcp__plugin_cfa-core_cfa-core__",
      command: "node",
      args: ["server.js"],
    },
    {
      name: "cfa-data",
      prefix: "mcp__plugin_cfa-data_data__",
      command: "node",
      args: ["server.js"],
    },
  ];

  it("round-trips bare→wire for a known tool", () => {
    const wireToolsByServer = new Map<string, string[]>([
      ["cfa-core", ["mcp__plugin_cfa-core_cfa-core__option_pricer"]],
      ["cfa-data", ["mcp__plugin_cfa-data_data__yf_quote"]],
    ]);
    const map = buildNameMap(servers, wireToolsByServer);
    const resolved = resolveBareToWire(map, "option_pricer");
    expect(resolved).toBeDefined();
    expect(resolved!.wireName).toBe("mcp__plugin_cfa-core_cfa-core__option_pricer");
    expect(resolved!.serverName).toBe("cfa-core");
  });

  it("round-trips wire→bare for the cfa-data prefix", () => {
    const bare = extractBareFromWire("mcp__plugin_cfa-data_data__yf_quote", "mcp__plugin_cfa-data_data__");
    expect(bare).toBe("yf_quote");
  });

  it("returns undefined for an unknown bare name", () => {
    const wireToolsByServer = new Map<string, string[]>([
      ["cfa-core", ["mcp__plugin_cfa-core_cfa-core__option_pricer"]],
    ]);
    const map = buildNameMap(servers, wireToolsByServer);
    expect(resolveBareToWire(map, "nonexistent_tool")).toBeUndefined();
  });

  it("throws on bare-name collision across servers", () => {
    const wireToolsByServer = new Map<string, string[]>([
      ["cfa-core", ["mcp__plugin_cfa-core_cfa-core__duplicate_tool"]],
      ["cfa-data", ["mcp__plugin_cfa-data_data__duplicate_tool"]],
    ]);
    expect(() => buildNameMap(servers, wireToolsByServer)).toThrow(/collision/i);
  });

  it("returns wire name as-is when prefix does not match", () => {
    const bare = extractBareFromWire("some_tool_without_prefix", "mcp__plugin_cfa-core_cfa-core__");
    expect(bare).toBe("some_tool_without_prefix");
  });

  it("handles multiple tools from the same server without collision", () => {
    const wireToolsByServer = new Map<string, string[]>([
      ["cfa-core", [
        "mcp__plugin_cfa-core_cfa-core__option_pricer",
        "mcp__plugin_cfa-core_cfa-core__bond_pricer",
        "mcp__plugin_cfa-core_cfa-core__wacc_calculator",
      ]],
      ["cfa-data", []],
    ]);
    const map = buildNameMap(servers, wireToolsByServer);
    expect(map.size).toBe(3);
    expect(resolveBareToWire(map, "bond_pricer")?.serverName).toBe("cfa-core");
    expect(resolveBareToWire(map, "wacc_calculator")?.wireName).toBe(
      "mcp__plugin_cfa-core_cfa-core__wacc_calculator",
    );
  });
});

// ---------------------------------------------------------------------------
// stdio integration test (skipped if plugin dist/server.js is absent)
// ---------------------------------------------------------------------------

const PLUGIN_SERVER = "/home/robert/cfa_agent/plugins/cfa-core/mcp/dist/server.js";
const pluginExists = existsSync(PLUGIN_SERVER);

describe.skipIf(!pluginExists)("stdio.integration", () => {
  it("calls option_pricer and returns the Chaco baseline price", async () => {
    const servers: MCPServerConfig[] = [
      {
        name: "cfa-core",
        prefix: "mcp__plugin_cfa-core_cfa-core__",
        command: "node",
        args: [PLUGIN_SERVER],
      },
    ];

    const client = createStdioMCPClient(servers);
    await client.initialize();

    const tools = await client.listTools();
    expect(tools.length).toBeGreaterThan(0);
    const optionTool = tools.find((t) => t.name === "option_pricer");
    expect(optionTool).toBeDefined();

    const result = await client.callTool("option_pricer", {
      spot_price: 0.10,
      strike_price: 0.25,
      time_to_expiry: 2.0,
      risk_free_rate: 0.035,
      volatility: 0.80,
      dividend_yield: 0,
      option_type: "Call",
      exercise_style: "American",
      binomial_steps: 200,
    });

    // Result is the parsed JSON from content[0].text
    const parsed = result as { result?: { price?: string } };
    const price = parseFloat(parsed?.result?.price ?? "NaN");
    expect(isNaN(price)).toBe(false);
    // Chaco baseline: 0.0207585... → toFixed(4) = "0.0208"
    expect(price.toFixed(4)).toBe("0.0208");

    await client.close();
  }, 60_000);
});
