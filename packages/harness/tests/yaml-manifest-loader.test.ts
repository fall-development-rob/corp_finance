/**
 * YAML manifest loader unit tests — Phase 36.
 *
 * All filesystem fixtures are written to os.tmpdir() + mkdtempSync and
 * cleaned up after each suite. No live MCP connections.
 */

import {
  mkdtempSync,
  mkdirSync,
  writeFileSync,
  rmSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { createDirectYamlManifestLoader } from "../src/manifests/yaml-loader.js";
import type { SkillLoader } from "../src/skills/types.js";
import type { ParsedSkill } from "../src/skills/types.js";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

let tmpDir: string;

beforeEach(() => {
  tmpDir = mkdtempSync(join(tmpdir(), "yaml-manifest-test-"));
});

afterEach(() => {
  rmSync(tmpDir, { recursive: true, force: true });
});

function writeFile(relPath: string, content: string): string {
  const full = join(tmpDir, relPath);
  mkdirSync(full.replace(/\/[^/]+$/, ""), { recursive: true });
  writeFileSync(full, content, "utf-8");
  return full;
}

function makeSkillLoader(skills: Record<string, string>): SkillLoader {
  return {
    async loadSkill(id: string): Promise<ParsedSkill> {
      if (!(id in skills)) throw new Error(`Skill "${id}" not found`);
      return {
        id,
        path: `/fake/skills/${id}/SKILL.md`,
        frontmatter: { name: id },
        body: skills[id] ?? "",
      };
    },
    async loadAgent() {
      throw new Error("not used in these tests");
    },
    clearCache() {},
  };
}

// ---------------------------------------------------------------------------
// 1. YAML parse — minimal manifest
// ---------------------------------------------------------------------------

describe("YAML manifest loader — minimal manifest", () => {
  it("parses name and system.text only", async () => {
    writeFile(
      "agents/minimal.yaml",
      [
        "name: test-agent",
        "system:",
        "  text: Hello world",
      ].join("\n"),
    );
    const loader = createDirectYamlManifestLoader({
      agentsRoot: join(tmpDir, "agents"),
      skillLoader: makeSkillLoader({}),
    });
    const def = await loader.loadAgent("minimal");
    expect(def.id).toBe("test-agent");
    expect(def.systemPrompt).toBe("Hello world");
    expect(def.tools).toBe("*");
  });

  it("sets model, maxTokens, maxRecursionDepth from manifest fields", async () => {
    writeFile(
      "agents/full.yaml",
      [
        "name: full-agent",
        "model: claude-sonnet-4-6",
        "max_tokens: 4096",
        "max_recursion_depth: 2",
        "system:",
        "  text: body",
      ].join("\n"),
    );
    const loader = createDirectYamlManifestLoader({
      agentsRoot: join(tmpDir, "agents"),
      skillLoader: makeSkillLoader({}),
    });
    const def = await loader.loadAgent("full");
    expect(def.model).toBe("claude-sonnet-4-6");
    expect(def.maxTokens).toBe(4096);
    expect(def.maxRecursionDepth).toBe(2);
  });
});

// ---------------------------------------------------------------------------
// 2. Malformed YAML — error message
// ---------------------------------------------------------------------------

describe("YAML manifest loader — error messages", () => {
  it("malformed YAML emits path in error message", async () => {
    writeFile("agents/bad.yaml", "name: ok\nsystem: [unclosed");
    const loader = createDirectYamlManifestLoader({
      agentsRoot: join(tmpDir, "agents"),
      skillLoader: makeSkillLoader({}),
    });
    await expect(loader.loadAgent("bad")).rejects.toThrow(/Failed to parse manifest/);
  });

  it("missing name field emits descriptive error", async () => {
    writeFile("agents/noname.yaml", "model: claude-sonnet-4-6\nsystem:\n  text: hi");
    const loader = createDirectYamlManifestLoader({
      agentsRoot: join(tmpDir, "agents"),
      skillLoader: makeSkillLoader({}),
    });
    await expect(loader.loadAgent("noname")).rejects.toThrow(/missing required field "name"/);
  });

  it("missing manifest file emits path in error", async () => {
    const loader = createDirectYamlManifestLoader({
      agentsRoot: join(tmpDir, "agents"),
      skillLoader: makeSkillLoader({}),
    });
    await expect(loader.loadAgent("ghost")).rejects.toThrow(/not found.*ghost\.yaml/);
  });
});

// ---------------------------------------------------------------------------
// 3. from_plugin resolution (canonical)
// ---------------------------------------------------------------------------

describe("YAML manifest loader — from_plugin resolution", () => {
  it("reads SKILL.md from plugin directory relative to manifest", async () => {
    mkdirSync(join(tmpDir, "skills", "my-skill"), { recursive: true });
    writeFileSync(
      join(tmpDir, "skills", "my-skill", "SKILL.md"),
      "---\nname: my-skill\n---\nSkill prose here.",
      "utf-8",
    );
    writeFile(
      "agents/agent.yaml",
      [
        "name: test-agent",
        "skills:",
        "  - from_plugin: ../skills/my-skill",
      ].join("\n"),
    );
    const loader = createDirectYamlManifestLoader({
      agentsRoot: join(tmpDir, "agents"),
      skillLoader: makeSkillLoader({}),
    });
    const def = await loader.loadAgent("agent");
    expect(def.systemPrompt).toContain("Skill prose here.");
  });

  it("throws when from_plugin path has no SKILL.md", async () => {
    mkdirSync(join(tmpDir, "skills", "missing-skill"), { recursive: true });
    writeFile(
      "agents/agent.yaml",
      [
        "name: test-agent",
        "skills:",
        "  - from_plugin: ../skills/missing-skill",
      ].join("\n"),
    );
    const loader = createDirectYamlManifestLoader({
      agentsRoot: join(tmpDir, "agents"),
      skillLoader: makeSkillLoader({}),
    });
    await expect(loader.loadAgent("agent")).rejects.toThrow(/SKILL\.md at:/);
  });
});

// ---------------------------------------------------------------------------
// 4. from_skill resolution (legacy)
// ---------------------------------------------------------------------------

describe("YAML manifest loader — from_skill resolution (legacy)", () => {
  it("resolves slug via injected skill loader", async () => {
    writeFile(
      "agents/agent.yaml",
      [
        "name: test-agent",
        "skills:",
        "  - from_skill: my-slug-skill",
      ].join("\n"),
    );
    const loader = createDirectYamlManifestLoader({
      agentsRoot: join(tmpDir, "agents"),
      skillLoader: makeSkillLoader({ "my-slug-skill": "Legacy skill body." }),
    });
    const def = await loader.loadAgent("agent");
    expect(def.systemPrompt).toContain("Legacy skill body.");
  });

  it("propagates not-found error from skill loader", async () => {
    writeFile(
      "agents/agent.yaml",
      [
        "name: test-agent",
        "skills:",
        "  - from_skill: nonexistent-skill",
      ].join("\n"),
    );
    const loader = createDirectYamlManifestLoader({
      agentsRoot: join(tmpDir, "agents"),
      skillLoader: makeSkillLoader({}),
    });
    await expect(loader.loadAgent("agent")).rejects.toThrow(/not found/);
  });
});

// ---------------------------------------------------------------------------
// 5. Mixed from_plugin + from_skill in same skills array
// ---------------------------------------------------------------------------

describe("YAML manifest loader — mixed skill refs", () => {
  it("assembles bodies from both from_plugin and from_skill in order", async () => {
    mkdirSync(join(tmpDir, "skills", "plugin-skill"), { recursive: true });
    writeFileSync(
      join(tmpDir, "skills", "plugin-skill", "SKILL.md"),
      "---\nname: plugin-skill\n---\nPlugin body.",
      "utf-8",
    );
    writeFile(
      "agents/agent.yaml",
      [
        "name: test-agent",
        "skills:",
        "  - from_plugin: ../skills/plugin-skill",
        "  - from_skill: slug-skill",
      ].join("\n"),
    );
    const loader = createDirectYamlManifestLoader({
      agentsRoot: join(tmpDir, "agents"),
      skillLoader: makeSkillLoader({ "slug-skill": "Slug body." }),
    });
    const def = await loader.loadAgent("agent");
    expect(def.systemPrompt.indexOf("Plugin body.")).toBeLessThan(
      def.systemPrompt.indexOf("Slug body."),
    );
    expect(def.systemPrompt).toContain("Plugin body.");
    expect(def.systemPrompt).toContain("Slug body.");
  });
});

// ---------------------------------------------------------------------------
// 6. system.file resolution
// ---------------------------------------------------------------------------

describe("YAML manifest loader — system.file", () => {
  it("reads external file and includes it in system prompt", async () => {
    writeFile("agents/system.md", "System file content here.");
    writeFile(
      "agents/agent.yaml",
      [
        "name: test-agent",
        "system:",
        "  file: ./system.md",
      ].join("\n"),
    );
    const loader = createDirectYamlManifestLoader({
      agentsRoot: join(tmpDir, "agents"),
      skillLoader: makeSkillLoader({}),
    });
    const def = await loader.loadAgent("agent");
    expect(def.systemPrompt).toContain("System file content here.");
  });

  it("system.file + system.append — append follows file content", async () => {
    writeFile("agents/sys.md", "File content.");
    writeFile(
      "agents/agent.yaml",
      [
        "name: test-agent",
        "system:",
        "  file: ./sys.md",
        "  append: Appended content.",
      ].join("\n"),
    );
    const loader = createDirectYamlManifestLoader({
      agentsRoot: join(tmpDir, "agents"),
      skillLoader: makeSkillLoader({}),
    });
    const def = await loader.loadAgent("agent");
    const idxFile = def.systemPrompt.indexOf("File content.");
    const idxAppend = def.systemPrompt.indexOf("Appended content.");
    expect(idxFile).toBeGreaterThanOrEqual(0);
    expect(idxAppend).toBeGreaterThan(idxFile);
  });
});

// ---------------------------------------------------------------------------
// 7. callable_agents — recursive load + cycle detection
// ---------------------------------------------------------------------------

describe("YAML manifest loader — callable_agents", () => {
  it("recursively loads subagent manifests", async () => {
    writeFile(
      "agents/sub.yaml",
      ["name: sub-agent", "system:", "  text: Sub agent."].join("\n"),
    );
    writeFile(
      "agents/parent.yaml",
      [
        "name: parent-agent",
        "system:",
        "  text: Parent agent.",
        "callable_agents:",
        "  - manifest: ./sub.yaml",
      ].join("\n"),
    );
    const loader = createDirectYamlManifestLoader({
      agentsRoot: join(tmpDir, "agents"),
      skillLoader: makeSkillLoader({}),
    });
    const def = await loader.loadAgent("parent");
    const ca = (def as any).callableAgents;
    expect(Array.isArray(ca)).toBe(true);
    expect(ca).toHaveLength(1);
    expect(ca[0].id).toBe("sub-agent");
  });

  it("detects cycles in callable_agents", async () => {
    // a.yaml → b.yaml → a.yaml
    writeFile(
      "agents/a.yaml",
      [
        "name: agent-a",
        "system:",
        "  text: A.",
        "callable_agents:",
        "  - manifest: ./b.yaml",
      ].join("\n"),
    );
    writeFile(
      "agents/b.yaml",
      [
        "name: agent-b",
        "system:",
        "  text: B.",
        "callable_agents:",
        "  - manifest: ./a.yaml",
      ].join("\n"),
    );
    const loader = createDirectYamlManifestLoader({
      agentsRoot: join(tmpDir, "agents"),
      skillLoader: makeSkillLoader({}),
    });
    await expect(loader.loadAgent("a")).rejects.toThrow(/Cycle detected/);
  });
});

// ---------------------------------------------------------------------------
// 8. Tool projection — per-tool explicit-enable (default-disabled + configs)
// ---------------------------------------------------------------------------

describe("YAML manifest loader — tool projection", () => {
  it("default_disabled + enabled configs → only those tools", async () => {
    writeFile(
      "agents/agent.yaml",
      [
        "name: test-agent",
        "system:",
        "  text: body",
        "tools:",
        "  - type: mcp_toolset",
        "    mcp_server_name: cfa-core",
        "    default_config: { enabled: false }",
        "    configs:",
        "      - { name: wacc_calculator, enabled: true }",
        "      - { name: dcf_model, enabled: true }",
        "      - { name: comps_analysis, enabled: false }",
      ].join("\n"),
    );
    const loader = createDirectYamlManifestLoader({
      agentsRoot: join(tmpDir, "agents"),
      skillLoader: makeSkillLoader({}),
    });
    const def = await loader.loadAgent("agent");
    expect(Array.isArray(def.tools)).toBe(true);
    const tools = def.tools as string[];
    expect(tools).toContain("wacc_calculator");
    expect(tools).toContain("dcf_model");
    expect(tools).not.toContain("comps_analysis");
  });

  it("default_enabled toolset → tools='*' and toolsetsRaw preserved", async () => {
    writeFile(
      "agents/agent.yaml",
      [
        "name: test-agent",
        "system:",
        "  text: body",
        "tools:",
        "  - type: mcp_toolset",
        "    mcp_server_name: cfa-core",
        "    default_config: { enabled: true }",
      ].join("\n"),
    );
    const loader = createDirectYamlManifestLoader({
      agentsRoot: join(tmpDir, "agents"),
      skillLoader: makeSkillLoader({}),
    });
    const def = await loader.loadAgent("agent");
    expect(def.tools).toBe("*");
    expect((def as any).toolsetsRaw).toBeDefined();
  });

  it("tools_array block — bare names passed through directly", async () => {
    writeFile(
      "agents/agent.yaml",
      [
        "name: test-agent",
        "system:",
        "  text: body",
        "tools:",
        "  - type: tools_array",
        "    tools:",
        "      - alpha_tool",
        "      - beta_tool",
      ].join("\n"),
    );
    const loader = createDirectYamlManifestLoader({
      agentsRoot: join(tmpDir, "agents"),
      skillLoader: makeSkillLoader({}),
    });
    const def = await loader.loadAgent("agent");
    expect(Array.isArray(def.tools)).toBe(true);
    const tools = def.tools as string[];
    expect(tools).toContain("alpha_tool");
    expect(tools).toContain("beta_tool");
  });

  it("prefix stripping: mcp__cfa-core__wacc_calculator → wacc_calculator", async () => {
    writeFile(
      "agents/agent.yaml",
      [
        "name: test-agent",
        "system:",
        "  text: body",
        "tools:",
        "  - type: mcp_toolset",
        "    mcp_server_name: cfa-core",
        "    default_config: { enabled: false }",
        "    configs:",
        "      - { name: mcp__cfa-core__wacc_calculator, enabled: true }",
      ].join("\n"),
    );
    const loader = createDirectYamlManifestLoader({
      agentsRoot: join(tmpDir, "agents"),
      skillLoader: makeSkillLoader({}),
    });
    const def = await loader.loadAgent("agent");
    const tools = def.tools as string[];
    expect(tools).toContain("wacc_calculator");
    expect(tools).not.toContain("mcp__cfa-core__wacc_calculator");
  });

  it("multi-toolset union: cfa-core allowlist + data allowlist merged", async () => {
    writeFile(
      "agents/agent.yaml",
      [
        "name: test-agent",
        "system:",
        "  text: body",
        "tools:",
        "  - type: mcp_toolset",
        "    mcp_server_name: cfa-core",
        "    default_config: { enabled: false }",
        "    configs:",
        "      - { name: dcf_model, enabled: true }",
        "  - type: mcp_toolset",
        "    mcp_server_name: data",
        "    default_config: { enabled: false }",
        "    configs:",
        "      - { name: fred_series, enabled: true }",
      ].join("\n"),
    );
    const loader = createDirectYamlManifestLoader({
      agentsRoot: join(tmpDir, "agents"),
      skillLoader: makeSkillLoader({}),
    });
    const def = await loader.loadAgent("agent");
    const tools = def.tools as string[];
    expect(tools).toContain("dcf_model");
    expect(tools).toContain("fred_series");
  });

  it("agent_toolset normalisation: accepts both type values", async () => {
    for (const typeVal of ["agent_toolset", "agent_toolset_20260401"]) {
      writeFile(
        `agents/${typeVal}.yaml`,
        [
          `name: ${typeVal}-agent`,
          "system:",
          "  text: body",
          "tools:",
          `  - type: ${typeVal}`,
          "    default_config: { enabled: false }",
          "    configs:",
          "      - { name: read, enabled: true }",
        ].join("\n"),
      );
      const loader = createDirectYamlManifestLoader({
        agentsRoot: join(tmpDir, "agents"),
        skillLoader: makeSkillLoader({}),
      });
      const def = await loader.loadAgent(typeVal);
      const tools = def.tools as string[];
      expect(tools).toContain("read");
    }
  });
});

// ---------------------------------------------------------------------------
// 9. output_schema and mcp_servers stored on AgentDef
// ---------------------------------------------------------------------------

describe("YAML manifest loader — optional metadata fields", () => {
  it("output_schema parsed and stored on def.outputSchema", async () => {
    writeFile(
      "agents/agent.yaml",
      [
        "name: test-agent",
        "system:",
        "  text: body",
        "output_schema:",
        "  type: object",
        "  properties:",
        "    result:",
        "      type: string",
      ].join("\n"),
    );
    const loader = createDirectYamlManifestLoader({
      agentsRoot: join(tmpDir, "agents"),
      skillLoader: makeSkillLoader({}),
    });
    const def = await loader.loadAgent("agent");
    const schema = (def as any).outputSchema;
    expect(schema).toBeDefined();
    expect(schema.type).toBe("object");
  });

  it("mcp_servers parsed and stored on def.mcpServerRefs", async () => {
    writeFile(
      "agents/agent.yaml",
      [
        "name: test-agent",
        "system:",
        "  text: body",
        "mcp_servers:",
        "  - type: url",
        "    name: cfa-core",
        "    url: http://localhost:3000",
      ].join("\n"),
    );
    const loader = createDirectYamlManifestLoader({
      agentsRoot: join(tmpDir, "agents"),
      skillLoader: makeSkillLoader({}),
    });
    const def = await loader.loadAgent("agent");
    const refs = (def as any).mcpServerRefs;
    expect(Array.isArray(refs)).toBe(true);
    expect(refs[0].name).toBe("cfa-core");
  });
});
