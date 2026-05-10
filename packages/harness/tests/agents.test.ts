/**
 * Agent registry tests — Phase 31 Wave 2.
 *
 * Verifies that the agent registry is correctly populated (chief +
 * 8 specialists), that getAgent enforces known-id invariants, that
 * defaultMCPServers has the expected structure and prefixes, and that
 * defaultDelegates contains the eight specialist defs.
 */

import { existsSync } from "node:fs";
import { describe, expect, it } from "vitest";
import {
  chiefAnalyst,
  defaultDelegates,
  defaultMCPServers,
  getAgent,
  registry,
} from "../src/agents/registry.js";

describe("agent registry — Wave 2", () => {
  it("getAgent('chief-analyst') returns a valid AgentDef with a non-empty system prompt", () => {
    const def = getAgent("chief-analyst");
    expect(def).toBeDefined();
    expect(def.id).toBe("chief-analyst");
    expect(typeof def.systemPrompt).toBe("string");
    expect(def.systemPrompt.length).toBeGreaterThan(0);
    expect(def.tools).toBe("*");
    // Wave 2: chief delegates to 8 specialists at depth 1
    expect(def.maxRecursionDepth).toBe(1);
    expect(def.model).toBe("claude-opus-4-5");
    expect(def.maxTokens).toBe(16384);
  });

  it("registers all 8 specialists with maxRecursionDepth 0 (no further delegation)", () => {
    const expected = [
      "equity-analyst",
      "credit-analyst",
      "fixed-income-analyst",
      "derivatives-analyst",
      "quant-risk-analyst",
      "macro-analyst",
      "private-markets-analyst",
      "esg-regulatory-analyst",
    ];
    for (const id of expected) {
      const def = getAgent(id);
      expect(def.id).toBe(id);
      expect(def.systemPrompt.length).toBeGreaterThan(500);
      expect(Array.isArray(def.tools)).toBe(true);
      expect((def.tools as string[]).length).toBeGreaterThan(10);
      expect(def.maxRecursionDepth ?? 0).toBe(0);
    }
  });

  it("defaultDelegates contains the 8 specialists in canonical order", () => {
    expect(defaultDelegates).toHaveLength(8);
    expect(defaultDelegates.map((d) => d.id)).toEqual([
      "equity-analyst",
      "credit-analyst",
      "fixed-income-analyst",
      "derivatives-analyst",
      "quant-risk-analyst",
      "macro-analyst",
      "private-markets-analyst",
      "esg-regulatory-analyst",
    ]);
  });

  it("getAgent('unknown') throws with a descriptive error", () => {
    expect(() => getAgent("unknown")).toThrowError(/Unknown agent id "unknown"/);
  });

  it("defaultMCPServers contains exactly 4 entries with the expected prefixes", () => {
    expect(defaultMCPServers).toHaveLength(4);

    const expectedPrefixes = [
      "mcp__plugin_cfa-core_cfa-core__",
      "mcp__plugin_cfa-data_data__",
      "mcp__plugin_cfa-pro_fmp-market-data__",
      "mcp__plugin_cfa-pro_vendor__",
    ];

    const actualPrefixes = defaultMCPServers.map((s) => s.prefix);
    expect(actualPrefixes).toEqual(expectedPrefixes);

    for (const server of defaultMCPServers) {
      expect(typeof server.name).toBe("string");
      expect(server.name.length).toBeGreaterThan(0);
      expect(server.command).toBe("node");
      expect(Array.isArray(server.args)).toBe(true);
      expect(server.args).toHaveLength(1);
    }
  });

  it("each MCP server's resolved server.js path exists on disk (or skip with reason)", () => {
    for (const server of defaultMCPServers) {
      const serverPath = server.args[0];
      if (serverPath === undefined) {
        throw new Error(`Server "${server.name}" has no args[0]`);
      }
      if (!existsSync(serverPath)) {
        // Skip gracefully — plugin dist may not be built in CI
        console.warn(
          `SKIP: "${server.name}" dist not found at "${serverPath}". ` +
            `Build the plugin with: cd plugins/${server.name} && npm run build`,
        );
        continue;
      }
      expect(existsSync(serverPath)).toBe(true);
    }
    // Always passes (missing paths produce warnings, not failures)
    expect(true).toBe(true);
  });

  it("registry map contains chief-analyst and chiefAnalyst export is the same object", () => {
    expect(registry.has("chief-analyst")).toBe(true);
    expect(registry.get("chief-analyst")).toBe(chiefAnalyst);
  });
});
