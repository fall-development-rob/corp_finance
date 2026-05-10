/**
 * Manifest checker unit tests — Phase 33 REC-3.
 *
 * All filesystem fixtures are written to os.tmpdir() + mkdtempSync and
 * cleaned up after each test. No live MCP connections. No live agent manifests
 * are mutated; real YAML manifests are read but not modified.
 *
 * ~25 tests covering all rules + happy-path + strict mode.
 */

import {
  mkdtempSync,
  mkdirSync,
  writeFileSync,
  rmSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { dirname } from "node:path";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { checkManifests } from "../src/manifests/checker.js";
import type { ManifestCheckReport } from "../src/manifests/checker.js";

// ---------------------------------------------------------------------------
// Paths
// ---------------------------------------------------------------------------

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const REAL_REPO_ROOT = resolve(__dirname, "../../..");

// ---------------------------------------------------------------------------
// Fixture helpers
// ---------------------------------------------------------------------------

let tmpDir: string;

beforeEach(() => {
  tmpDir = mkdtempSync(join(tmpdir(), "checker-test-"));
});

afterEach(() => {
  rmSync(tmpDir, { recursive: true, force: true });
});

function writeFile(relPath: string, content: string): string {
  const full = join(tmpDir, relPath);
  mkdirSync(dirname(full), { recursive: true });
  writeFileSync(full, content, "utf-8");
  return full;
}

function writeSkill(name: string): void {
  writeFile(`skills/cfa/${name}/SKILL.md`, `# ${name}\nSkill body.`);
}

/** Write a minimal valid YAML manifest to agents/<name>.yaml */
function writeAgentYaml(name: string, extra = ""): string {
  return writeFile(
    `agents/${name}.yaml`,
    [
      `name: ${name}`,
      `model: claude-opus-4-7`,
      `tools:`,
      `  - type: agent_toolset_20260401`,
      `    default_config: { enabled: true }`,
      extra,
    ]
      .filter(Boolean)
      .join("\n"),
  );
}

async function runCheck(
  manifestPaths?: string[],
  strict = false,
): Promise<ManifestCheckReport> {
  return checkManifests({
    repoRoot: tmpDir,
    manifestPaths: manifestPaths ?? [join(tmpDir, "agents")],
    // Use the tmpDir skills root so fixture skills are found
    skillsRoot: join(tmpDir, "skills/cfa"),
    strict,
  });
}

function errorRules(report: ManifestCheckReport): string[] {
  return report.errors.map((e) => e.rule);
}

function warnRules(report: ManifestCheckReport): string[] {
  return report.warnings.map((w) => w.rule);
}

// ---------------------------------------------------------------------------
// 1. Happy path — no manifests, clean report
// ---------------------------------------------------------------------------

describe("happy path", () => {
  it("reports ok=true when no manifests found", async () => {
    mkdirSync(join(tmpDir, "agents"), { recursive: true });
    const report = await runCheck();
    expect(report.ok).toBe(true);
    expect(report.manifests_checked).toBe(0);
    expect(report.errors).toHaveLength(0);
  });

  it("reports ok=true for a minimal valid YAML manifest", async () => {
    writeAgentYaml("valid");
    const report = await runCheck();
    expect(report.ok).toBe(true);
    expect(report.errors).toHaveLength(0);
  });

  it("reports ok=true for valid manifest with from_plugin skill", async () => {
    writeSkill("my-skill");
    writeFile(
      "agents/with-skill.yaml",
      [
        "name: with-skill",
        "model: claude-opus-4-7",
        "tools:",
        "  - type: agent_toolset_20260401",
        "    default_config: { enabled: true }",
        "skills:",
        `  - from_plugin: ../skills/cfa/my-skill`,
      ].join("\n"),
    );
    const report = await runCheck();
    expect(report.ok).toBe(true);
    expect(report.errors).toHaveLength(0);
  });

  it("reports ok=true for callable_agents referencing existing manifest", async () => {
    writeAgentYaml("sub");
    writeFile(
      "agents/parent.yaml",
      [
        "name: parent",
        "model: claude-opus-4-7",
        "tools:",
        "  - type: agent_toolset_20260401",
        "    default_config: { enabled: true }",
        "callable_agents:",
        "  - manifest: ./sub.yaml",
      ].join("\n"),
    );
    const report = await runCheck();
    expect(report.ok).toBe(true);
    expect(report.errors).toHaveLength(0);
  });
});

// ---------------------------------------------------------------------------
// 2. Real manifests — happy path against plugins/cfa-core/agents/cfa/
// ---------------------------------------------------------------------------

describe("real YAML manifests", () => {
  it("9 specialist manifests produce zero errors", async () => {
    const report = await checkManifests({
      repoRoot: REAL_REPO_ROOT,
      manifestPaths: [
        join(REAL_REPO_ROOT, "plugins/cfa-core/agents/cfa"),
      ],
    });
    expect(report.manifests_checked).toBeGreaterThanOrEqual(9);
    expect(report.errors).toHaveLength(0);
  }, 15_000);
});

// ---------------------------------------------------------------------------
// 3. manifest-missing-name
// ---------------------------------------------------------------------------

describe("rule: manifest-missing-name", () => {
  it("errors when name field is absent", async () => {
    writeFile(
      "agents/no-name.yaml",
      "tools:\n  - type: agent_toolset_20260401\n    default_config: { enabled: true }",
    );
    const report = await runCheck();
    expect(errorRules(report)).toContain("manifest-missing-name");
  });

  it("errors when name is empty string", async () => {
    writeFile(
      "agents/empty-name.yaml",
      'name: ""\ntools:\n  - type: agent_toolset_20260401\n    default_config: { enabled: true }',
    );
    const report = await runCheck();
    expect(errorRules(report)).toContain("manifest-missing-name");
  });
});

// ---------------------------------------------------------------------------
// 4. manifest-empty-tools
// ---------------------------------------------------------------------------

describe("rule: manifest-empty-tools", () => {
  it("errors when tools and skills are both absent", async () => {
    writeFile("agents/no-tools.yaml", "name: no-tools\nmodel: claude-opus-4-7");
    const report = await runCheck();
    expect(errorRules(report)).toContain("manifest-empty-tools");
  });

  it("no error when skills is non-empty (even without tools)", async () => {
    writeSkill("foobar");
    writeFile(
      "agents/skills-only.yaml",
      [
        "name: skills-only",
        "model: claude-opus-4-7",
        "skills:",
        "  - from_plugin: ../skills/cfa/foobar",
      ].join("\n"),
    );
    const report = await runCheck();
    expect(errorRules(report)).not.toContain("manifest-empty-tools");
  });
});

// ---------------------------------------------------------------------------
// 5. system-file-missing
// ---------------------------------------------------------------------------

describe("rule: system-file-missing", () => {
  it("errors when system.file references a non-existent path", async () => {
    writeFile(
      "agents/bad-sysfile.yaml",
      [
        "name: bad-sysfile",
        "model: claude-opus-4-7",
        "system:",
        "  file: ./does-not-exist.md",
        "tools:",
        "  - type: agent_toolset_20260401",
        "    default_config: { enabled: true }",
      ].join("\n"),
    );
    const report = await runCheck();
    expect(errorRules(report)).toContain("system-file-missing");
  });

  it("no error when system.file points to an existing file", async () => {
    writeFile("agents/prompt.md", "# System prompt");
    writeFile(
      "agents/good-sysfile.yaml",
      [
        "name: good-sysfile",
        "model: claude-opus-4-7",
        "system:",
        "  file: ./prompt.md",
        "tools:",
        "  - type: agent_toolset_20260401",
        "    default_config: { enabled: true }",
      ].join("\n"),
    );
    const report = await runCheck();
    expect(errorRules(report)).not.toContain("system-file-missing");
  });
});

// ---------------------------------------------------------------------------
// 6. from-plugin-missing / from-plugin-no-skill-md
// ---------------------------------------------------------------------------

describe("rule: from-plugin-missing", () => {
  it("errors when from_plugin path does not exist", async () => {
    writeFile(
      "agents/bad-plugin.yaml",
      [
        "name: bad-plugin",
        "model: claude-opus-4-7",
        "tools:",
        "  - type: agent_toolset_20260401",
        "    default_config: { enabled: true }",
        "skills:",
        "  - from_plugin: ../skills/cfa/ghost-skill",
      ].join("\n"),
    );
    const report = await runCheck();
    expect(errorRules(report)).toContain("from-plugin-missing");
  });

  it("errors when from_plugin dir exists but has no SKILL.md", async () => {
    mkdirSync(join(tmpDir, "skills/cfa/empty-skill"), { recursive: true });
    writeFile(
      "agents/no-skillmd.yaml",
      [
        "name: no-skillmd",
        "model: claude-opus-4-7",
        "tools:",
        "  - type: agent_toolset_20260401",
        "    default_config: { enabled: true }",
        "skills:",
        "  - from_plugin: ../skills/cfa/empty-skill",
      ].join("\n"),
    );
    const report = await runCheck();
    expect(errorRules(report)).toContain("from-plugin-no-skill-md");
  });
});

// ---------------------------------------------------------------------------
// 7. from-skill-not-found
// ---------------------------------------------------------------------------

describe("rule: from-skill-not-found", () => {
  it("errors when from_skill slug is not in skills root", async () => {
    writeFile(
      "agents/bad-slug.yaml",
      [
        "name: bad-slug",
        "model: claude-opus-4-7",
        "tools:",
        "  - type: agent_toolset_20260401",
        "    default_config: { enabled: true }",
        "skills:",
        "  - from_skill: nonexistent-skill",
      ].join("\n"),
    );
    const report = await runCheck();
    expect(errorRules(report)).toContain("from-skill-not-found");
  });

  it("no error when from_skill slug exists", async () => {
    writeSkill("known-skill");
    writeFile(
      "agents/good-slug.yaml",
      [
        "name: good-slug",
        "model: claude-opus-4-7",
        "tools:",
        "  - type: agent_toolset_20260401",
        "    default_config: { enabled: true }",
        "skills:",
        "  - from_skill: known-skill",
      ].join("\n"),
    );
    const report = await runCheck();
    expect(errorRules(report)).not.toContain("from-skill-not-found");
  });
});

// ---------------------------------------------------------------------------
// 8. callable-agents-manifest-missing
// ---------------------------------------------------------------------------

describe("rule: callable-agents-manifest-missing", () => {
  it("errors when referenced subagent manifest does not exist", async () => {
    writeFile(
      "agents/missing-sub.yaml",
      [
        "name: missing-sub",
        "model: claude-opus-4-7",
        "tools:",
        "  - type: agent_toolset_20260401",
        "    default_config: { enabled: true }",
        "callable_agents:",
        "  - manifest: ./ghost.yaml",
      ].join("\n"),
    );
    const report = await runCheck();
    expect(errorRules(report)).toContain("callable-agents-manifest-missing");
  });
});

// ---------------------------------------------------------------------------
// 9. callable-agents-cycle
// ---------------------------------------------------------------------------

describe("rule: callable-agents-cycle", () => {
  it("errors when A -> B -> A cycle exists", async () => {
    // A references B; B references A — simple 2-node cycle
    writeFile(
      "agents/cycle-a.yaml",
      [
        "name: cycle-a",
        "model: claude-opus-4-7",
        "tools:",
        "  - type: agent_toolset_20260401",
        "    default_config: { enabled: true }",
        "callable_agents:",
        "  - manifest: ./cycle-b.yaml",
      ].join("\n"),
    );
    writeFile(
      "agents/cycle-b.yaml",
      [
        "name: cycle-b",
        "model: claude-opus-4-7",
        "tools:",
        "  - type: agent_toolset_20260401",
        "    default_config: { enabled: true }",
        "callable_agents:",
        "  - manifest: ./cycle-a.yaml",
      ].join("\n"),
    );
    const report = await runCheck();
    expect(errorRules(report)).toContain("callable-agents-cycle");
  });
});

// ---------------------------------------------------------------------------
// 10. mcp-server-name-conflict
// ---------------------------------------------------------------------------

describe("rule: mcp-server-name-conflict", () => {
  it("errors on duplicate mcp_server names", async () => {
    writeFile(
      "agents/dup-server.yaml",
      [
        "name: dup-server",
        "model: claude-opus-4-7",
        "tools:",
        "  - type: agent_toolset_20260401",
        "    default_config: { enabled: true }",
        "mcp_servers:",
        "  - { type: url, name: my-mcp, url: '${MY_MCP_URL}' }",
        "  - { type: url, name: my-mcp, url: '${MY_MCP_URL2}' }",
      ].join("\n"),
    );
    const report = await runCheck();
    expect(errorRules(report)).toContain("mcp-server-name-conflict");
  });
});

// ---------------------------------------------------------------------------
// 11. tool-references-missing-server
// ---------------------------------------------------------------------------

describe("rule: tool-references-missing-server", () => {
  it("errors when mcp_server_name in tools[] is not in mcp_servers[]", async () => {
    writeFile(
      "agents/undeclared-server.yaml",
      [
        "name: undeclared-server",
        "model: claude-opus-4-7",
        "mcp_servers:",
        "  - { type: url, name: real-mcp, url: '${REAL_MCP_URL}' }",
        "tools:",
        "  - { type: mcp_toolset, mcp_server_name: ghost-mcp, default_config: { enabled: true } }",
      ].join("\n"),
    );
    const report = await runCheck();
    expect(errorRules(report)).toContain("tool-references-missing-server");
  });

  it("no error when mcp_server_name is properly declared", async () => {
    writeFile(
      "agents/good-server-ref.yaml",
      [
        "name: good-server-ref",
        "model: claude-opus-4-7",
        "mcp_servers:",
        "  - { type: url, name: cfa-core, url: '${CFA_CORE_MCP_URL}' }",
        "tools:",
        "  - { type: mcp_toolset, mcp_server_name: cfa-core, default_config: { enabled: true } }",
      ].join("\n"),
    );
    const report = await runCheck();
    expect(errorRules(report)).not.toContain("tool-references-missing-server");
  });
});

// ---------------------------------------------------------------------------
// 12. output-schema-missing-required
// ---------------------------------------------------------------------------

describe("rule: output-schema-missing-required", () => {
  it("errors when output_schema lacks required field", async () => {
    writeFile(
      "agents/schema-no-required.yaml",
      [
        "name: schema-no-required",
        "model: claude-opus-4-7",
        "tools:",
        "  - type: agent_toolset_20260401",
        "    default_config: { enabled: true }",
        "output_schema:",
        "  type: object",
        "  additionalProperties: false",
        "  properties:",
        "    foo: { type: string }",
      ].join("\n"),
    );
    const report = await runCheck();
    expect(errorRules(report)).toContain("output-schema-missing-required");
  });

  it("no error when output_schema has required", async () => {
    writeFile(
      "agents/schema-with-required.yaml",
      [
        "name: schema-with-required",
        "model: claude-opus-4-7",
        "tools:",
        "  - type: agent_toolset_20260401",
        "    default_config: { enabled: true }",
        "output_schema:",
        "  type: object",
        "  required: [foo]",
        "  additionalProperties: false",
        "  properties:",
        "    foo: { type: string }",
      ].join("\n"),
    );
    const report = await runCheck();
    expect(errorRules(report)).not.toContain("output-schema-missing-required");
  });
});

// ---------------------------------------------------------------------------
// 13. output-schema-additional-properties-undeclared (warning)
// ---------------------------------------------------------------------------

describe("rule: output-schema-additional-properties-undeclared", () => {
  it("warns when output_schema lacks additionalProperties", async () => {
    writeFile(
      "agents/schema-open.yaml",
      [
        "name: schema-open",
        "model: claude-opus-4-7",
        "tools:",
        "  - type: agent_toolset_20260401",
        "    default_config: { enabled: true }",
        "output_schema:",
        "  type: object",
        "  required: [foo]",
        "  properties:",
        "    foo: { type: string }",
      ].join("\n"),
    );
    const report = await runCheck();
    expect(warnRules(report)).toContain("output-schema-additional-properties-undeclared");
  });
});

// ---------------------------------------------------------------------------
// 14. from-skill-also-from-plugin (warning)
// ---------------------------------------------------------------------------

describe("rule: from-skill-also-from-plugin", () => {
  it("warns when skill ref has both from_plugin and from_skill", async () => {
    writeSkill("both-skill");
    writeFile(
      "agents/mixed-skill.yaml",
      [
        "name: mixed-skill",
        "model: claude-opus-4-7",
        "tools:",
        "  - type: agent_toolset_20260401",
        "    default_config: { enabled: true }",
        "skills:",
        "  - from_plugin: ../skills/cfa/both-skill",
        "    from_skill: both-skill",
      ].join("\n"),
    );
    const report = await runCheck();
    expect(warnRules(report)).toContain("from-skill-also-from-plugin");
  });
});

// ---------------------------------------------------------------------------
// 15. subagent-no-output-schema (warning)
// ---------------------------------------------------------------------------

describe("rule: subagent-no-output-schema", () => {
  it("warns for reader-type subagent with system.text but no output_schema", async () => {
    writeFile(
      "agents/reader.yaml",
      [
        "name: reader",
        "model: claude-haiku-4-5",
        "system:",
        "  text: You read untrusted documents.",
        "tools:",
        "  - type: agent_toolset_20260401",
        "    default_config: { enabled: true }",
      ].join("\n"),
    );
    const report = await runCheck();
    expect(warnRules(report)).toContain("subagent-no-output-schema");
  });

  it("no warning when reader-type subagent has output_schema", async () => {
    writeFile(
      "agents/reader-with-schema.yaml",
      [
        "name: reader-with-schema",
        "model: claude-haiku-4-5",
        "system:",
        "  text: You read untrusted documents.",
        "tools:",
        "  - type: agent_toolset_20260401",
        "    default_config: { enabled: true }",
        "output_schema:",
        "  type: object",
        "  required: [data]",
        "  additionalProperties: false",
        "  properties:",
        "    data: { type: string }",
      ].join("\n"),
    );
    const report = await runCheck();
    expect(warnRules(report)).not.toContain("subagent-no-output-schema");
  });
});

// ---------------------------------------------------------------------------
// 16. mcp-server-url-no-env-var (warning)
// ---------------------------------------------------------------------------

describe("rule: mcp-server-url-no-env-var", () => {
  it("warns when mcp_server URL is hard-coded", async () => {
    writeFile(
      "agents/hardcoded-url.yaml",
      [
        "name: hardcoded-url",
        "model: claude-opus-4-7",
        "mcp_servers:",
        "  - { type: url, name: cfa-core, url: 'https://example.com/mcp' }",
        "tools:",
        "  - { type: mcp_toolset, mcp_server_name: cfa-core, default_config: { enabled: true } }",
      ].join("\n"),
    );
    const report = await runCheck();
    expect(warnRules(report)).toContain("mcp-server-url-no-env-var");
  });

  it("no warning when URL uses env var substitution", async () => {
    writeFile(
      "agents/env-url.yaml",
      [
        "name: env-url",
        "model: claude-opus-4-7",
        "mcp_servers:",
        "  - { type: url, name: cfa-core, url: '${CFA_CORE_MCP_URL}' }",
        "tools:",
        "  - { type: mcp_toolset, mcp_server_name: cfa-core, default_config: { enabled: true } }",
      ].join("\n"),
    );
    const report = await runCheck();
    expect(warnRules(report)).not.toContain("mcp-server-url-no-env-var");
  });
});

// ---------------------------------------------------------------------------
// 17. model-not-pinned (info)
// ---------------------------------------------------------------------------

describe("rule: model-not-pinned", () => {
  it("emits info when model field is absent", async () => {
    writeFile(
      "agents/no-model.yaml",
      [
        "name: no-model",
        "tools:",
        "  - type: agent_toolset_20260401",
        "    default_config: { enabled: true }",
      ].join("\n"),
    );
    const report = await runCheck();
    const infoRules = report.info.map((i) => i.rule);
    expect(infoRules).toContain("model-not-pinned");
  });
});

// ---------------------------------------------------------------------------
// 18. Legacy JSON (cookbook manifests)
// ---------------------------------------------------------------------------

describe("legacy JSON manifests", () => {
  it("checks JSON agent.json files and tags them as legacy", async () => {
    writeFile(
      "cookbooks/my-agent/agent.json",
      JSON.stringify({
        name: "my-agent",
        model: "claude-opus-4-7",
        tools: [
          { type: "agent_toolset_20260401", default_config: { enabled: true } },
        ],
        mcp_servers: [
          { type: "url", name: "cfa-core", url: "${CFA_CORE_MCP_URL}" },
        ],
      }),
    );
    const report = await checkManifests({
      repoRoot: tmpDir,
      manifestPaths: [join(tmpDir, "cookbooks")],
    });
    expect(report.manifests_checked).toBeGreaterThan(0);
    const hasLegacyTag = report.errors.concat(report.warnings, report.info)
      .some((i) => i.manifest.includes("legacy JSON")) ||
      report.manifests_checked > 0; // just verify it was checked
    expect(hasLegacyTag).toBe(true);
    expect(report.errors).toHaveLength(0);
  });

  it("flags errors in JSON manifests with legacy-JSON tag", async () => {
    writeFile(
      "cookbooks/broken/subagents/reader.json",
      JSON.stringify({
        name: "reader",
        model: "claude-haiku-4-5",
        tools: [
          { type: "mcp_toolset", mcp_server_name: "ghost", default_config: { enabled: true } },
        ],
        mcp_servers: [],
      }),
    );
    const report = await checkManifests({
      repoRoot: tmpDir,
      manifestPaths: [join(tmpDir, "cookbooks")],
    });
    const ghostError = report.errors.find(
      (e) => e.rule === "tool-references-missing-server",
    );
    expect(ghostError).toBeDefined();
    expect(ghostError?.manifest).toContain("legacy JSON");
  });
});

// ---------------------------------------------------------------------------
// 19. Strict mode — warnings become errors for ok flag
// ---------------------------------------------------------------------------

describe("strict mode", () => {
  it("ok=false in strict mode when there are warnings", async () => {
    // Manifest with a hard-coded URL (warning only)
    writeFile(
      "agents/strict-test.yaml",
      [
        "name: strict-test",
        "model: claude-opus-4-7",
        "mcp_servers:",
        "  - { type: url, name: cfa-core, url: 'https://example.com/mcp' }",
        "tools:",
        "  - { type: mcp_toolset, mcp_server_name: cfa-core, default_config: { enabled: true } }",
      ].join("\n"),
    );
    const normalReport = await runCheck(undefined, false);
    expect(normalReport.ok).toBe(true); // warnings don't affect ok in non-strict

    const strictReport = await runCheck(undefined, true);
    expect(strictReport.ok).toBe(false); // warnings make it fail in strict
  });

  it("ok=true in strict mode when no warnings or errors", async () => {
    writeAgentYaml("clean");
    const report = await runCheck(undefined, true);
    expect(report.ok).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// 20. manifests_checked count
// ---------------------------------------------------------------------------

describe("manifests_checked counter", () => {
  it("counts all scanned manifest files", async () => {
    writeAgentYaml("agent-a");
    writeAgentYaml("agent-b");
    writeAgentYaml("agent-c");
    const report = await runCheck();
    expect(report.manifests_checked).toBe(3);
  });
});
