/**
 * Phase 25 Tier A1 — tool-catalog generator + cookbook lint.
 *
 * Pure-function tests using inline TS fixtures and temp dirs. No git, no
 * network. Covers regex parsers (NAPI multi-line, plugin inline + multi-line),
 * deterministic serialisation, and the three failure modes of the lint.
 */

import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { mkdtempSync, mkdirSync, writeFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import {
  generateToolCatalog,
  serialiseCatalog,
  parseCatalog,
  lintCookbookToolNames,
  defaultServerSources,
  type ToolCatalog,
} from "../src/manifests/tool-catalog.js";

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

interface Fixture {
  root: string;
  cleanup: () => void;
}

function makeFixture(): Fixture {
  const root = mkdtempSync(join(tmpdir(), "tool-catalog-"));
  return { root, cleanup: () => rmSync(root, { recursive: true, force: true }) };
}

function writeFile(root: string, rel: string, content: string): void {
  const full = join(root, rel);
  mkdirSync(join(full, ".."), { recursive: true });
  writeFileSync(full, content, "utf8");
}

// ---------------------------------------------------------------------------
// generateToolCatalog — extractors
// ---------------------------------------------------------------------------

describe("generateToolCatalog — NAPI multi-line pattern", () => {
  let fx: Fixture;
  beforeEach(() => {
    fx = makeFixture();
  });
  afterEach(() => fx.cleanup());

  it("extracts tool names from double-quoted server.tool() registrations", () => {
    writeFile(
      fx.root,
      "packages/fmp-mcp-server/src/tools/financials.ts",
      [
        `import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";`,
        `export function register(server: McpServer) {`,
        `  server.tool(`,
        `    "fmp_income_statement",`,
        `    "Get income statement data",`,
        `    {},`,
        `    async () => ({}),`,
        `  );`,
        `}`,
      ].join("\n"),
    );
    const cat = generateToolCatalog({ repoRoot: fx.root });
    expect(cat.servers["fmp"]).toEqual(["fmp_income_statement"]);
  });

  it("extracts tool names from single-quoted server.tool() registrations", () => {
    writeFile(
      fx.root,
      "packages/fmp-mcp-server/src/tools/x.ts",
      [
        `  server.tool(`,
        `    'fmp_balance_sheet',`,
        `    'desc',`,
        `    {},`,
        `    async () => ({}),`,
        `  );`,
      ].join("\n"),
    );
    const cat = generateToolCatalog({ repoRoot: fx.root });
    expect(cat.servers["fmp"]).toEqual(["fmp_balance_sheet"]);
  });

  it("dedupes and sorts within a server", () => {
    writeFile(
      fx.root,
      "packages/data-mcp-server/src/a.ts",
      [`  server.tool(`, `    "z_tool",`, `    "z",`, `  );`].join("\n"),
    );
    writeFile(
      fx.root,
      "packages/data-mcp-server/src/b.ts",
      [
        `  server.tool(`,
        `    "a_tool",`,
        `    "a",`,
        `  );`,
        `  server.tool(`,
        `    "a_tool",`,
        `    "duplicate",`,
        `  );`,
      ].join("\n"),
    );
    const cat = generateToolCatalog({ repoRoot: fx.root });
    expect(cat.servers["data"]).toEqual(["a_tool", "z_tool"]);
  });

  it("skips .test.ts files", () => {
    writeFile(
      fx.root,
      "packages/data-mcp-server/src/a.test.ts",
      [`  server.tool(`, `    "test_only",`, `    "x",`, `  );`].join("\n"),
    );
    const cat = generateToolCatalog({ repoRoot: fx.root });
    expect(cat.servers["data"]).toEqual([]);
  });
});

describe("generateToolCatalog — plugin patterns", () => {
  let fx: Fixture;
  beforeEach(() => {
    fx = makeFixture();
  });
  afterEach(() => fx.cleanup());

  it("extracts inline tool(server, 'name', 'desc', fn) form", () => {
    writeFile(
      fx.root,
      "plugins/cfa-core/mcp/src/server.ts",
      [
        `import { tool } from "./helpers.js";`,
        `function register(server) {`,
        `  tool(server, "wacc_calculator", "WACC via CAPM.", wasm.calculate_wacc);`,
        `  tool(server, "dcf_model", "Discounted Cash Flow.", wasm.build_dcf);`,
        `}`,
      ].join("\n"),
    );
    const cat = generateToolCatalog({ repoRoot: fx.root });
    expect(cat.servers["cfa-core"]).toEqual(["dcf_model", "wacc_calculator"]);
  });

  it("extracts multi-line tool(\\n server,\\n 'name',\\n ...) form", () => {
    writeFile(
      fx.root,
      "plugins/cfa-core/mcp/src/server.ts",
      [
        `function register(server) {`,
        `  tool(`,
        `    server,`,
        `    "lbo_model",`,
        `    "Leveraged buyout model.",`,
        `    wasm.lbo_model,`,
        `  );`,
        `}`,
      ].join("\n"),
    );
    const cat = generateToolCatalog({ repoRoot: fx.root });
    expect(cat.servers["cfa-core"]).toEqual(["lbo_model"]);
  });

  it("handles descriptions containing apostrophes via permissive name regex", () => {
    writeFile(
      fx.root,
      "plugins/cfa-core/mcp/src/server.ts",
      [
        `  tool(server, "tool_a", "S&P's rating logic — note the apostrophe.", wasm.a);`,
      ].join("\n"),
    );
    const cat = generateToolCatalog({ repoRoot: fx.root });
    expect(cat.servers["cfa-core"]).toEqual(["tool_a"]);
  });
});

describe("generateToolCatalog — empty / missing sources", () => {
  let fx: Fixture;
  beforeEach(() => {
    fx = makeFixture();
  });
  afterEach(() => fx.cleanup());

  it("returns empty arrays when source directories are absent", () => {
    const cat = generateToolCatalog({ repoRoot: fx.root });
    expect(cat.servers["cfa-core"]).toEqual([]);
    expect(cat.servers["fmp"]).toEqual([]);
    expect(cat.servers["data"]).toEqual([]);
    expect(cat.servers["vendor"]).toEqual([]);
  });

  it("includes all 4 default servers in catalog shape", () => {
    const sources = defaultServerSources(fx.root);
    expect(sources.map((s) => s.server).sort()).toEqual([
      "cfa-core",
      "data",
      "fmp",
      "vendor",
    ]);
  });
});

// ---------------------------------------------------------------------------
// serialiseCatalog / parseCatalog round-trip + determinism
// ---------------------------------------------------------------------------

describe("serialiseCatalog", () => {
  it("emits servers in alphabetical key order regardless of input order", () => {
    const cat: ToolCatalog = {
      version: "1",
      servers: { z: ["t1"], a: ["t2"] },
    };
    const out = serialiseCatalog(cat);
    expect(out.indexOf('"a"')).toBeLessThan(out.indexOf('"z"'));
  });

  it("is byte-deterministic across two calls", () => {
    const cat: ToolCatalog = {
      version: "1",
      servers: { "cfa-core": ["wacc_calculator", "dcf_model"], fmp: [] },
    };
    expect(serialiseCatalog(cat)).toBe(serialiseCatalog(cat));
  });

  it("round-trips through parseCatalog", () => {
    const cat: ToolCatalog = {
      version: "1",
      servers: { "cfa-core": ["a", "b"], fmp: ["c"] },
    };
    expect(parseCatalog(serialiseCatalog(cat))).toEqual(cat);
  });

  it("ends with a trailing newline", () => {
    const out = serialiseCatalog({ version: "1", servers: {} });
    expect(out.endsWith("\n")).toBe(true);
  });
});

describe("parseCatalog — validation", () => {
  it("rejects JSON without a servers key", () => {
    expect(() => parseCatalog("{}")).toThrow(/servers/);
  });

  it("rejects servers entries that aren't string arrays", () => {
    expect(() =>
      parseCatalog(JSON.stringify({ servers: { x: [1, 2] } })),
    ).toThrow(/array of strings/);
  });
});

// ---------------------------------------------------------------------------
// lintCookbookToolNames
// ---------------------------------------------------------------------------

describe("lintCookbookToolNames", () => {
  let fx: Fixture;
  beforeEach(() => {
    fx = makeFixture();
  });
  afterEach(() => fx.cleanup());

  const catalog: ToolCatalog = {
    version: "1",
    servers: {
      "cfa-core": ["wacc_calculator", "dcf_model", "target_price"],
      fmp: ["fmp_income_statement"],
    },
  };

  function writeCookbook(slug: string, parentYaml: string): void {
    writeFile(fx.root, `managed-agent-cookbooks/${slug}/agent.yaml`, parentYaml);
  }

  function writeSubagent(slug: string, name: string, yaml: string): void {
    writeFile(
      fx.root,
      `managed-agent-cookbooks/${slug}/subagents/${name}.yaml`,
      yaml,
    );
  }

  it("returns zero issues when every configs[].name resolves", () => {
    writeCookbook(
      "ok-cookbook",
      [
        `name: a`,
        `tools:`,
        `  - type: mcp_toolset`,
        `    mcp_server_name: cfa-core`,
        `    configs:`,
        `      - { name: mcp__cfa-core__wacc_calculator, enabled: true }`,
        `      - { name: mcp__cfa-core__dcf_model, enabled: true }`,
      ].join("\n"),
    );
    const report = lintCookbookToolNames({
      catalog,
      cookbooksRoot: join(fx.root, "managed-agent-cookbooks"),
      repoRoot: fx.root,
    });
    expect(report.issues).toHaveLength(0);
    expect(report.cookbooks_scanned).toBe(1);
    expect(report.configs_checked).toBe(2);
  });

  it("flags unknown_tool when bare name is not in catalog for that server", () => {
    writeCookbook(
      "drift",
      [
        `name: a`,
        `tools:`,
        `  - type: mcp_toolset`,
        `    mcp_server_name: cfa-core`,
        `    configs:`,
        `      - { name: mcp__cfa-core__calculate_target_price, enabled: true }`,
      ].join("\n"),
    );
    const report = lintCookbookToolNames({
      catalog,
      cookbooksRoot: join(fx.root, "managed-agent-cookbooks"),
      repoRoot: fx.root,
    });
    expect(report.issues).toHaveLength(1);
    expect(report.issues[0]?.reason).toBe("unknown_tool");
    expect(report.issues[0]?.tool_name).toBe("calculate_target_price");
  });

  it("flags unknown_server when mcp_server_name has no catalog entry", () => {
    writeCookbook(
      "third-party",
      [
        `name: a`,
        `tools:`,
        `  - type: mcp_toolset`,
        `    mcp_server_name: unknown-vendor`,
        `    configs:`,
        `      - { name: mcp__unknown-vendor__some_tool, enabled: true }`,
      ].join("\n"),
    );
    const report = lintCookbookToolNames({
      catalog,
      cookbooksRoot: join(fx.root, "managed-agent-cookbooks"),
      repoRoot: fx.root,
    });
    expect(report.issues).toHaveLength(1);
    expect(report.issues[0]?.reason).toBe("unknown_server");
  });

  it("flags prefix_mismatch when prefix server != block.mcp_server_name", () => {
    writeCookbook(
      "mismatch",
      [
        `name: a`,
        `tools:`,
        `  - type: mcp_toolset`,
        `    mcp_server_name: cfa-core`,
        `    configs:`,
        `      - { name: mcp__fmp__fmp_income_statement, enabled: true }`,
      ].join("\n"),
    );
    const report = lintCookbookToolNames({
      catalog,
      cookbooksRoot: join(fx.root, "managed-agent-cookbooks"),
      repoRoot: fx.root,
    });
    expect(report.issues).toHaveLength(1);
    expect(report.issues[0]?.reason).toBe("prefix_mismatch");
  });

  it("skips blocks without explicit configs (default_config: enabled)", () => {
    writeCookbook(
      "all-tools",
      [
        `name: a`,
        `tools:`,
        `  - type: mcp_toolset`,
        `    mcp_server_name: cfa-core`,
        `    default_config: { enabled: true }`,
      ].join("\n"),
    );
    const report = lintCookbookToolNames({
      catalog,
      cookbooksRoot: join(fx.root, "managed-agent-cookbooks"),
      repoRoot: fx.root,
    });
    expect(report.issues).toHaveLength(0);
    expect(report.configs_checked).toBe(0);
  });

  it("ignores non-mcp toolset blocks (agent_toolset_20260401)", () => {
    writeCookbook(
      "builtin",
      [
        `name: a`,
        `tools:`,
        `  - type: agent_toolset_20260401`,
        `    default_config: { enabled: false }`,
        `    configs:`,
        `      - { name: read, enabled: true }`,
      ].join("\n"),
    );
    const report = lintCookbookToolNames({
      catalog,
      cookbooksRoot: join(fx.root, "managed-agent-cookbooks"),
      repoRoot: fx.root,
    });
    expect(report.issues).toHaveLength(0);
    expect(report.configs_checked).toBe(0);
  });

  it("scans subagent manifests alongside parent agent.yaml", () => {
    writeCookbook("multi", `name: parent\ntools: []\n`);
    writeSubagent(
      "multi",
      "worker",
      [
        `name: worker`,
        `tools:`,
        `  - type: mcp_toolset`,
        `    mcp_server_name: cfa-core`,
        `    configs:`,
        `      - { name: mcp__cfa-core__not_a_tool, enabled: true }`,
      ].join("\n"),
    );
    const report = lintCookbookToolNames({
      catalog,
      cookbooksRoot: join(fx.root, "managed-agent-cookbooks"),
      repoRoot: fx.root,
    });
    expect(report.files_scanned).toBe(2);
    expect(report.issues).toHaveLength(1);
    expect(report.issues[0]?.manifest).toContain("subagents/worker.yaml");
  });

  it("reports all issues — does not stop at first failure", () => {
    writeCookbook(
      "many",
      [
        `name: a`,
        `tools:`,
        `  - type: mcp_toolset`,
        `    mcp_server_name: cfa-core`,
        `    configs:`,
        `      - { name: mcp__cfa-core__bad1, enabled: true }`,
        `      - { name: mcp__cfa-core__bad2, enabled: true }`,
        `      - { name: mcp__cfa-core__bad3, enabled: true }`,
      ].join("\n"),
    );
    const report = lintCookbookToolNames({
      catalog,
      cookbooksRoot: join(fx.root, "managed-agent-cookbooks"),
      repoRoot: fx.root,
    });
    expect(report.issues).toHaveLength(3);
  });

  it("returns zero issues when cookbooks dir does not exist", () => {
    const report = lintCookbookToolNames({
      catalog,
      cookbooksRoot: join(fx.root, "no-such-dir"),
      repoRoot: fx.root,
    });
    expect(report.cookbooks_scanned).toBe(0);
    expect(report.files_scanned).toBe(0);
    expect(report.issues).toHaveLength(0);
  });
});
