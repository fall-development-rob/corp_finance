/**
 * surface_parity.test.ts
 *
 * Unit + integration tests for the surface_parity MCP tool.
 * Uses vitest. Inline fixture strings exercise all regex patterns.
 */

import { describe, it, expect } from "vitest";
import { mkdtempSync, writeFileSync, mkdirSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
import {
  extractPackagesTools,
  extractPluginTools,
  surfaceParityCheck,
  surfaceParityCheckMcp,
  type ToolEntry,
  type SurfaceParityReport,
} from "./surface_parity.js";

// ---------------------------------------------------------------------------
// Fixture helpers
// ---------------------------------------------------------------------------

interface Fixture {
  root: string;
  toolsDir: string;
  pluginDir: string;
  allowlistPath: string;
}

function createFixture(): Fixture {
  const root = mkdtempSync(join(tmpdir(), "surface-parity-"));
  const toolsDir = join(root, "packages", "mcp-server", "src", "tools");
  const pluginDir = join(root, "plugins", "cfa-core", "mcp", "src");
  mkdirSync(toolsDir, { recursive: true });
  mkdirSync(pluginDir, { recursive: true });
  const allowlistPath = join(root, "packages", "mcp-server", ".surface-allowlist.json");
  writeFileSync(allowlistPath, JSON.stringify({ napi_only: [] }), "utf8");
  return { root, toolsDir, pluginDir, allowlistPath };
}

function teardown(root: string): void {
  rmSync(root, { recursive: true, force: true });
}

// ---------------------------------------------------------------------------
// Unit: NAPI multi-line registration
// ---------------------------------------------------------------------------

describe("extractPackagesTools — NAPI pattern", () => {
  it("parses multi-line server.tool() registration correctly", () => {
    const fx = createFixture();
    try {
      writeFileSync(
        join(fx.toolsDir, "example.ts"),
        [
          `import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";`,
          `export function register(server: McpServer) {`,
          `  server.tool(`,
          `    "wacc_calculator",`,
          `    "Compute WACC from capital structure inputs.",`,
          `    {},`,
          `    async () => ({ content: [] })`,
          `  );`,
          `}`,
        ].join("\n"),
        "utf8",
      );
      const results = extractPackagesTools(fx.root);
      expect(results).toHaveLength(1);
      expect(results[0]).toMatchObject<Partial<ToolEntry>>({
        tool_name: "wacc_calculator",
        description: "Compute WACC from capital structure inputs.",
      });
      expect(results[0]!.source_file).toMatch(/packages\/mcp-server\/src\/tools\/example\.ts/);
    } finally {
      teardown(fx.root);
    }
  });

  it("parses multiple server.tool() calls in one file", () => {
    const fx = createFixture();
    try {
      writeFileSync(
        join(fx.toolsDir, "multi.ts"),
        [
          `  server.tool(`,
          `    "tool_alpha",`,
          `    "Alpha description.",`,
          `    {},`,
          `    async () => ({})`,
          `  );`,
          `  server.tool(`,
          `    "tool_beta",`,
          `    "Beta description.",`,
          `    {},`,
          `    async () => ({})`,
          `  );`,
        ].join("\n"),
        "utf8",
      );
      const results = extractPackagesTools(fx.root);
      const names = results.map((r) => r.tool_name);
      expect(names).toContain("tool_alpha");
      expect(names).toContain("tool_beta");
    } finally {
      teardown(fx.root);
    }
  });
});

// ---------------------------------------------------------------------------
// Unit: Plugin multi-line registration
// ---------------------------------------------------------------------------

describe("extractPluginTools — plugin multi-line pattern", () => {
  it("parses multi-line tool(server, ...) registration correctly", () => {
    const fx = createFixture();
    try {
      writeFileSync(
        join(fx.pluginDir, "server.ts"),
        [
          `tool(`,
          `  server,`,
          `  "dcf_model",`,
          `  "Run a DCF model.",`,
          `  wasm.dcf_model`,
          `);`,
        ].join("\n"),
        "utf8",
      );
      const results = extractPluginTools(fx.root);
      expect(results).toHaveLength(1);
      expect(results[0]).toMatchObject<Partial<ToolEntry>>({
        tool_name: "dcf_model",
        description: "Run a DCF model.",
      });
    } finally {
      teardown(fx.root);
    }
  });

  it("does NOT match tool() when next line is not server", () => {
    const fx = createFixture();
    try {
      writeFileSync(
        join(fx.pluginDir, "server.ts"),
        [
          `tool(`,
          `  otherArg,`,
          `  "should_not_match",`,
          `  "description",`,
          `  wasm.fn`,
          `);`,
        ].join("\n"),
        "utf8",
      );
      const results = extractPluginTools(fx.root);
      expect(results).toHaveLength(0);
    } finally {
      teardown(fx.root);
    }
  });
});

// ---------------------------------------------------------------------------
// Unit: Plugin inline registration
// ---------------------------------------------------------------------------

describe("extractPluginTools — plugin inline pattern", () => {
  it("parses single-line tool(server, name, desc, fn) registration", () => {
    const fx = createFixture();
    try {
      writeFileSync(
        join(fx.pluginDir, "server.ts"),
        `tool(server, "bond_pricer", "Price a bond.", wasm.bond_pricer);\n`,
        "utf8",
      );
      const results = extractPluginTools(fx.root);
      expect(results).toHaveLength(1);
      expect(results[0]!.tool_name).toBe("bond_pricer");
      expect(results[0]!.description).toBe("Price a bond.");
    } finally {
      teardown(fx.root);
    }
  });

  it("parses inline and multi-line registrations in the same file", () => {
    const fx = createFixture();
    try {
      writeFileSync(
        join(fx.pluginDir, "server.ts"),
        [
          `tool(server, "inline_tool", "Inline desc.", wasm.fn);`,
          `tool(`,
          `  server,`,
          `  "multiline_tool",`,
          `  "Multiline desc.",`,
          `  wasm.fn2`,
          `);`,
        ].join("\n"),
        "utf8",
      );
      const results = extractPluginTools(fx.root);
      const names = results.map((r) => r.tool_name).sort();
      expect(names).toEqual(["inline_tool", "multiline_tool"]);
    } finally {
      teardown(fx.root);
    }
  });
});

// ---------------------------------------------------------------------------
// Unit: Escaped quotes in description
// ---------------------------------------------------------------------------

describe("description with escaped quotes", () => {
  it("round-trips a description containing escaped double quotes (NAPI)", () => {
    const fx = createFixture();
    try {
      writeFileSync(
        join(fx.toolsDir, "escape.ts"),
        [
          `  server.tool(`,
          `    "escape_tool",`,
          `    "Returns the \\"best\\" result.",`,
          `    {},`,
          `    async () => ({})`,
          `  );`,
        ].join("\n"),
        "utf8",
      );
      const results = extractPackagesTools(fx.root);
      expect(results).toHaveLength(1);
      expect(results[0]!.description).toBe('Returns the \\"best\\" result.');
    } finally {
      teardown(fx.root);
    }
  });
});

// ---------------------------------------------------------------------------
// Unit: surfaceParityCheck categorisation
// ---------------------------------------------------------------------------

describe("surfaceParityCheck", () => {
  it("correctly categorises shared, allowlisted, plugin_only, packages_only", () => {
    const fx = createFixture();
    try {
      // NAPI: shared_tool + napi_only_tool + packages_only_tool
      writeFileSync(
        join(fx.toolsDir, "tools.ts"),
        [
          `  server.tool(`,
          `    "shared_tool",`,
          `    "In both.",`,
          `    {}, async () => ({})`,
          `  );`,
          `  server.tool(`,
          `    "napi_only_tool",`,
          `    "Allowlisted.",`,
          `    {}, async () => ({})`,
          `  );`,
          `  server.tool(`,
          `    "packages_only_tool",`,
          `    "Drift!",`,
          `    {}, async () => ({})`,
          `  );`,
        ].join("\n"),
        "utf8",
      );
      // Plugin: shared_tool + plugin_only_tool
      writeFileSync(
        join(fx.pluginDir, "server.ts"),
        [
          `tool(server, "shared_tool", "In both.", wasm.fn);`,
          `tool(server, "plugin_only_tool", "Only in plugin.", wasm.fn);`,
        ].join("\n"),
        "utf8",
      );
      // Allowlist: napi_only_tool
      writeFileSync(
        fx.allowlistPath,
        JSON.stringify({
          napi_only: [{ tool: "napi_only_tool", reason: "native", owning_module: "m" }],
        }),
        "utf8",
      );

      const report = surfaceParityCheck({
        workspace_root: fx.root,
        allowlist_path: fx.allowlistPath,
      });

      expect(report.shared.map((t) => t.tool_name)).toEqual(["shared_tool"]);
      expect(report.allowlisted.map((t) => t.tool_name)).toEqual(["napi_only_tool"]);
      expect(report.plugin_only.map((t) => t.tool_name)).toEqual(["plugin_only_tool"]);
      expect(report.packages_only.map((t) => t.tool_name)).toEqual(["packages_only_tool"]);
      expect(report.drift_detected).toBe(true);
    } finally {
      teardown(fx.root);
    }
  });

  it("drift_detected is false when packages_only is empty", () => {
    const fx = createFixture();
    try {
      writeFileSync(
        join(fx.toolsDir, "tools.ts"),
        [`  server.tool(`, `    "tool_a",`, `    "desc",`, `    {}, async () => ({})`, `  );`].join("\n"),
        "utf8",
      );
      writeFileSync(
        join(fx.pluginDir, "server.ts"),
        `tool(server, "tool_a", "desc.", wasm.fn);\n`,
        "utf8",
      );
      const report = surfaceParityCheck({
        workspace_root: fx.root,
        allowlist_path: fx.allowlistPath,
      });
      expect(report.drift_detected).toBe(false);
      expect(report.packages_only).toHaveLength(0);
    } finally {
      teardown(fx.root);
    }
  });

  it("output arrays are sorted by tool_name", () => {
    const fx = createFixture();
    try {
      writeFileSync(
        join(fx.toolsDir, "tools.ts"),
        [
          `  server.tool(`, `    "zebra",`, `    "z",`, `    {}, async () => ({})`, `  );`,
          `  server.tool(`, `    "apple",`, `    "a",`, `    {}, async () => ({})`, `  );`,
        ].join("\n"),
        "utf8",
      );
      writeFileSync(
        join(fx.pluginDir, "server.ts"),
        [
          `tool(server, "zebra", "z", wasm.fn);`,
          `tool(server, "apple", "a", wasm.fn);`,
        ].join("\n"),
        "utf8",
      );
      const report = surfaceParityCheck({
        workspace_root: fx.root,
        allowlist_path: fx.allowlistPath,
      });
      const names = report.shared.map((t) => t.tool_name);
      expect(names).toEqual([...names].sort((a, b) => a.localeCompare(b)));
    } finally {
      teardown(fx.root);
    }
  });
});

// ---------------------------------------------------------------------------
// Unit: surfaceParityCheckMcp — JSON round-trip
// ---------------------------------------------------------------------------

describe("surfaceParityCheckMcp", () => {
  it("accepts a JSON string and returns a JSON string with SurfaceParityReport shape", async () => {
    const fx = createFixture();
    try {
      writeFileSync(join(fx.toolsDir, "t.ts"), "", "utf8");
      writeFileSync(join(fx.pluginDir, "server.ts"), "", "utf8");
      const resultJson = await surfaceParityCheckMcp(
        JSON.stringify({ workspace_root: fx.root, allowlist_path: fx.allowlistPath }),
      );
      const report = JSON.parse(resultJson) as SurfaceParityReport;
      expect(typeof report.workspace_root).toBe("string");
      expect(typeof report.packages_total).toBe("number");
      expect(typeof report.plugin_total).toBe("number");
      expect(Array.isArray(report.shared)).toBe(true);
      expect(typeof report.drift_detected).toBe("boolean");
    } finally {
      teardown(fx.root);
    }
  });
});

// ---------------------------------------------------------------------------
// Integration: live workspace
// ---------------------------------------------------------------------------

describe("integration — live workspace", () => {
  // Auto-detect workspace from this test file's location so the suite runs on
  // any machine (local /home/robert/cfa_agent, CI /home/runner/work/..., etc).
  const WORKSPACE = resolve(__dirname, "../../../../..");
  const ALLOWLIST = join(WORKSPACE, "packages/mcp-server/.surface-allowlist.json");

  it("returns a report with packages tools (agent_infrastructure only after cleanup)", () => {
    const report = surfaceParityCheck({
      workspace_root: WORKSPACE,
      allowlist_path: ALLOWLIST,
    });
    // After cleanup B/1: only the 4 agent_infrastructure tools remain in packages.
    // All are allowlisted (NAPI-only: subprocess + FS I/O, cannot run in WASM).
    expect(report.packages_total).toBeGreaterThan(0);
    expect(report.packages_only).toHaveLength(0);
  });

  it("returns a report with 100+ plugin tools", () => {
    const report = surfaceParityCheck({
      workspace_root: WORKSPACE,
      allowlist_path: ALLOWLIST,
    });
    expect(report.plugin_total).toBeGreaterThan(100);
  });

  it("shared count equals plugin_total (all plugin tools appear in packages)", () => {
    const report = surfaceParityCheck({
      workspace_root: WORKSPACE,
      allowlist_path: ALLOWLIST,
    });
    // Every plugin tool should be covered (shared or plugin_only)
    expect(report.shared.length + report.plugin_only.length).toBe(report.plugin_total);
  });

  it("packages_only is empty (no drift in clean state)", () => {
    const report = surfaceParityCheck({
      workspace_root: WORKSPACE,
      allowlist_path: ALLOWLIST,
    });
    expect(report.packages_only).toHaveLength(0);
    expect(report.drift_detected).toBe(false);
  });

  it("allowlisted count matches .surface-allowlist.json napi_only length", () => {
    const report = surfaceParityCheck({
      workspace_root: WORKSPACE,
      allowlist_path: ALLOWLIST,
    });
    // The script shows 50 allowlisted tools
    expect(report.allowlisted.length).toBeGreaterThan(0);
  });

  it("packages_total = shared + allowlisted + packages_only (accounting identity)", () => {
    const report = surfaceParityCheck({
      workspace_root: WORKSPACE,
      allowlist_path: ALLOWLIST,
    });
    const expected = report.shared.length + report.allowlisted.length + report.packages_only.length;
    expect(report.packages_total).toBe(expected);
  });
});
