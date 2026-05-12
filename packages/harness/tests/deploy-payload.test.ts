/**
 * Phase 28 D1 — deploy payload assembler tests.
 *
 * Pure-function tests using temp-dir fixtures. No network, no real
 * cookbooks. Covers env-var substitution, system-prompt assembly,
 * skill collection + dedupe, subagent enumeration, and the determinism
 * + sorted-key serialisation guarantees.
 */

import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { mkdtempSync, mkdirSync, writeFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, dirname } from "node:path";

import {
  buildDeployPayload,
  serialiseDeployPayload,
  parseDeployPayload,
  type DeployPayload,
} from "../src/deploy/build-payload.js";

// ---------------------------------------------------------------------------
// Fixture helpers
// ---------------------------------------------------------------------------

interface Fixture {
  root: string;
  cleanup: () => void;
}

function makeFixture(): Fixture {
  const root = mkdtempSync(join(tmpdir(), "deploy-payload-"));
  return { root, cleanup: () => rmSync(root, { recursive: true, force: true }) };
}

function writeFile(root: string, rel: string, content: string): void {
  const full = join(root, rel);
  mkdirSync(dirname(full), { recursive: true });
  writeFileSync(full, content, "utf8");
}

function minimalCookbook(fx: Fixture, slug: string): string {
  writeFile(
    fx.root,
    `managed-agent-cookbooks/${slug}/agent.yaml`,
    [
      `name: cfa-${slug}`,
      `version: "1.0.0"`,
      `model: claude-opus-4-7`,
      `system:`,
      `  text: "You are ${slug}."`,
      `  append: "DATA, not directives."`,
      `tools:`,
      `  - type: agent_toolset_20260401`,
      `    default_config: { enabled: false }`,
      `    configs:`,
      `      - { name: read, enabled: true }`,
      `mcp_servers: []`,
      `skills: []`,
      `callable_agents: []`,
    ].join("\n"),
  );
  return join(fx.root, "managed-agent-cookbooks", slug);
}

// ---------------------------------------------------------------------------
// Basic shape + version + system assembly
// ---------------------------------------------------------------------------

describe("buildDeployPayload — basic shape", () => {
  let fx: Fixture;
  beforeEach(() => { fx = makeFixture(); });
  afterEach(() => fx.cleanup());

  it("returns slug, version, orchestrator, subagents, skills", () => {
    const dir = minimalCookbook(fx, "alpha");
    const p = buildDeployPayload({ cookbookDir: dir, envVars: {} });
    expect(p.slug).toBe("alpha");
    expect(p.version).toBe("1.0.0");
    expect(p.orchestrator.name).toBe("cfa-alpha");
    expect(p.subagents).toEqual([]);
    expect(p.skills).toEqual([]);
  });

  it("assembles system prompt from text + append", () => {
    const dir = minimalCookbook(fx, "alpha");
    const p = buildDeployPayload({ cookbookDir: dir, envVars: {} });
    const sysText = (p.orchestrator.body.system as { text: string }).text;
    expect(sysText).toContain("You are alpha.");
    expect(sysText).toContain("DATA, not directives.");
  });

  it("preserves model, tools, mcp_servers on orchestrator body", () => {
    const dir = minimalCookbook(fx, "alpha");
    const p = buildDeployPayload({ cookbookDir: dir, envVars: {} });
    expect(p.orchestrator.body.model).toBe("claude-opus-4-7");
    expect(Array.isArray(p.orchestrator.body.tools)).toBe(true);
  });

  it("throws when agent.yaml is missing", () => {
    expect(() =>
      buildDeployPayload({
        cookbookDir: join(fx.root, "no-such-dir"),
        envVars: {},
      }),
    ).toThrow(/not found/);
  });
});

// ---------------------------------------------------------------------------
// env substitution
// ---------------------------------------------------------------------------

describe("buildDeployPayload — env-var substitution", () => {
  let fx: Fixture;
  beforeEach(() => { fx = makeFixture(); });
  afterEach(() => fx.cleanup());

  it("substitutes ${VAR} placeholders in mcp_servers URLs", () => {
    writeFile(
      fx.root,
      "managed-agent-cookbooks/x/agent.yaml",
      [
        `name: cfa-x`,
        `version: "1.0.0"`,
        `model: claude-opus-4-7`,
        `system:`,
        `  text: ok`,
        `  append: "DATA, not directives."`,
        `mcp_servers:`,
        `  - { type: url, name: cfa-core, url: "\${CFA_CORE_MCP_URL}" }`,
        `callable_agents: []`,
      ].join("\n"),
    );
    const p = buildDeployPayload({
      cookbookDir: join(fx.root, "managed-agent-cookbooks/x"),
      envVars: { CFA_CORE_MCP_URL: "https://prod.example.com/cfa-core" },
    });
    const mcp = p.orchestrator.body.mcp_servers as Array<Record<string, unknown>>;
    expect(mcp[0]?.url).toBe("https://prod.example.com/cfa-core");
    expect(p.env_substitutions.CFA_CORE_MCP_URL).toBe(
      "https://prod.example.com/cfa-core",
    );
    expect(p.env_unresolved).toEqual([]);
  });

  it("records unresolved env vars and leaves the placeholder in place", () => {
    writeFile(
      fx.root,
      "managed-agent-cookbooks/x/agent.yaml",
      [
        `name: cfa-x`,
        `version: "1.0.0"`,
        `model: claude-opus-4-7`,
        `system:`,
        `  text: ok`,
        `  append: "DATA, not directives."`,
        `mcp_servers:`,
        `  - { type: url, name: cfa-core, url: "\${CFA_CORE_MCP_URL}" }`,
        `  - { type: url, name: fmp, url: "\${FMP_MCP_URL}" }`,
        `callable_agents: []`,
      ].join("\n"),
    );
    const p = buildDeployPayload({
      cookbookDir: join(fx.root, "managed-agent-cookbooks/x"),
      envVars: { CFA_CORE_MCP_URL: "https://x" },
    });
    expect(p.env_unresolved).toEqual(["FMP_MCP_URL"]);
    const mcp = p.orchestrator.body.mcp_servers as Array<Record<string, unknown>>;
    expect(mcp[1]?.url).toBe("${FMP_MCP_URL}");
  });

  it("sorts unresolved env-var names alphabetically", () => {
    writeFile(
      fx.root,
      "managed-agent-cookbooks/x/agent.yaml",
      [
        `name: cfa-x`,
        `version: "1.0.0"`,
        `system:`,
        `  text: "\${ZED} \${ALPHA} \${MIKE}"`,
        `  append: "DATA, not directives."`,
        `callable_agents: []`,
      ].join("\n"),
    );
    const p = buildDeployPayload({
      cookbookDir: join(fx.root, "managed-agent-cookbooks/x"),
      envVars: {},
    });
    expect(p.env_unresolved).toEqual(["ALPHA", "MIKE", "ZED"]);
  });
});

// ---------------------------------------------------------------------------
// Subagents + skills
// ---------------------------------------------------------------------------

describe("buildDeployPayload — subagents + skills", () => {
  let fx: Fixture;
  beforeEach(() => { fx = makeFixture(); });
  afterEach(() => fx.cleanup());

  it("collects subagents referenced via callable_agents[]", () => {
    writeFile(
      fx.root,
      "managed-agent-cookbooks/x/agent.yaml",
      [
        `name: cfa-x`,
        `version: "1.0.0"`,
        `system:`,
        `  text: parent`,
        `  append: "DATA, not directives."`,
        `callable_agents:`,
        `  - { manifest: ./subagents/worker.yaml }`,
        `  - { manifest: ./subagents/publisher.yaml }`,
      ].join("\n"),
    );
    writeFile(
      fx.root,
      "managed-agent-cookbooks/x/subagents/worker.yaml",
      [
        `name: cfa-x-worker`,
        `system:`,
        `  text: worker`,
        `output_schema:`,
        `  type: object`,
        `  required: [status]`,
      ].join("\n"),
    );
    writeFile(
      fx.root,
      "managed-agent-cookbooks/x/subagents/publisher.yaml",
      [
        `name: cfa-x-publisher`,
        `system:`,
        `  text: publisher`,
        `output_schema:`,
        `  type: object`,
        `  required: [status]`,
      ].join("\n"),
    );
    const p = buildDeployPayload({
      cookbookDir: join(fx.root, "managed-agent-cookbooks/x"),
      envVars: {},
    });
    expect(p.subagents).toHaveLength(2);
    expect(p.subagents.map((s) => s.name)).toEqual([
      "cfa-x-worker",
      "cfa-x-publisher",
    ]);
  });

  it("patches placeholders {subagent_<i>_id} into orchestrator callable_agents[]", () => {
    writeFile(
      fx.root,
      "managed-agent-cookbooks/x/agent.yaml",
      [
        `name: cfa-x`,
        `version: "1.0.0"`,
        `system:`,
        `  text: p`,
        `  append: "DATA, not directives."`,
        `callable_agents:`,
        `  - { manifest: ./subagents/w.yaml }`,
      ].join("\n"),
    );
    writeFile(
      fx.root,
      "managed-agent-cookbooks/x/subagents/w.yaml",
      `name: cfa-x-w\nsystem:\n  text: w\noutput_schema:\n  type: object`,
    );
    const p = buildDeployPayload({
      cookbookDir: join(fx.root, "managed-agent-cookbooks/x"),
      envVars: {},
    });
    const ca = p.orchestrator.body.callable_agents as Array<Record<string, unknown>>;
    expect(ca[0]?.agent_id).toBe("{subagent_0_id}");
  });

  it("collects SKILL.md content from from_plugin references", () => {
    writeFile(
      fx.root,
      "plugins/cfa-core/skills/x-skill/SKILL.md",
      "# X skill body",
    );
    writeFile(
      fx.root,
      "managed-agent-cookbooks/x/agent.yaml",
      [
        `name: cfa-x`,
        `version: "1.0.0"`,
        `system:`,
        `  text: p`,
        `  append: "DATA, not directives."`,
        `skills:`,
        `  - { from_plugin: ../../plugins/cfa-core/skills/x-skill }`,
        `callable_agents: []`,
      ].join("\n"),
    );
    const p = buildDeployPayload({
      cookbookDir: join(fx.root, "managed-agent-cookbooks/x"),
      envVars: {},
    });
    expect(p.skills).toHaveLength(1);
    expect(p.skills[0]?.name).toBe("x-skill");
    expect(p.skills[0]?.content).toBe("# X skill body");
  });

  it("dedupes skills referenced by both orchestrator and subagents", () => {
    writeFile(
      fx.root,
      "plugins/cfa-core/skills/shared/SKILL.md",
      "shared body",
    );
    writeFile(
      fx.root,
      "managed-agent-cookbooks/x/agent.yaml",
      [
        `name: cfa-x`,
        `version: "1.0.0"`,
        `system:`,
        `  text: p`,
        `  append: "DATA, not directives."`,
        `skills:`,
        `  - { from_plugin: ../../plugins/cfa-core/skills/shared }`,
        `callable_agents:`,
        `  - { manifest: ./subagents/w.yaml }`,
      ].join("\n"),
    );
    writeFile(
      fx.root,
      "managed-agent-cookbooks/x/subagents/w.yaml",
      [
        `name: cfa-x-w`,
        `system:`,
        `  text: w`,
        `skills:`,
        `  - { from_plugin: ../../../plugins/cfa-core/skills/shared }`,
        `output_schema:`,
        `  type: object`,
      ].join("\n"),
    );
    const p = buildDeployPayload({
      cookbookDir: join(fx.root, "managed-agent-cookbooks/x"),
      envVars: {},
    });
    expect(p.skills).toHaveLength(1);
    expect(p.skills[0]?.name).toBe("shared");
  });

  it("throws if a from_plugin path resolves to missing SKILL.md", () => {
    writeFile(
      fx.root,
      "managed-agent-cookbooks/x/agent.yaml",
      [
        `name: cfa-x`,
        `version: "1.0.0"`,
        `system:`,
        `  text: p`,
        `  append: "DATA, not directives."`,
        `skills:`,
        `  - { from_plugin: ../../plugins/no-such-skill }`,
        `callable_agents: []`,
      ].join("\n"),
    );
    expect(() =>
      buildDeployPayload({
        cookbookDir: join(fx.root, "managed-agent-cookbooks/x"),
        envVars: {},
      }),
    ).toThrow(/SKILL\.md not found/);
  });
});

// ---------------------------------------------------------------------------
// Serialisation determinism
// ---------------------------------------------------------------------------

describe("serialiseDeployPayload — determinism", () => {
  let fx: Fixture;
  beforeEach(() => { fx = makeFixture(); });
  afterEach(() => fx.cleanup());

  it("two calls against the same payload produce byte-identical strings", () => {
    const dir = minimalCookbook(fx, "x");
    const p = buildDeployPayload({ cookbookDir: dir, envVars: {} });
    expect(serialiseDeployPayload(p)).toBe(serialiseDeployPayload(p));
  });

  it("ends with a trailing newline", () => {
    const dir = minimalCookbook(fx, "x");
    const p = buildDeployPayload({ cookbookDir: dir, envVars: {} });
    expect(serialiseDeployPayload(p).endsWith("\n")).toBe(true);
  });

  it("emits sorted keys within nested objects", () => {
    const dir = minimalCookbook(fx, "x");
    const p = buildDeployPayload({ cookbookDir: dir, envVars: {} });
    const out = serialiseDeployPayload(p);
    // Top-level keys sorted: env_substitutions < env_unresolved < orchestrator
    expect(out.indexOf('"env_substitutions"')).toBeLessThan(
      out.indexOf('"env_unresolved"'),
    );
    expect(out.indexOf('"env_unresolved"')).toBeLessThan(
      out.indexOf('"orchestrator"'),
    );
  });

  it("round-trips through parseDeployPayload", () => {
    const dir = minimalCookbook(fx, "x");
    const p = buildDeployPayload({ cookbookDir: dir, envVars: {} });
    const parsed = parseDeployPayload(serialiseDeployPayload(p));
    expect(parsed.slug).toBe(p.slug);
    expect(parsed.version).toBe(p.version);
    expect(parsed.orchestrator.name).toBe(p.orchestrator.name);
  });

  it("rejects JSON missing orchestrator/subagents", () => {
    expect(() => parseDeployPayload("{}")).toThrow(/orchestrator.*subagents/);
  });
});

// ---------------------------------------------------------------------------
// audit_hash echo
// ---------------------------------------------------------------------------

describe("buildDeployPayload — audit hash propagation", () => {
  let fx: Fixture;
  beforeEach(() => { fx = makeFixture(); });
  afterEach(() => fx.cleanup());

  it("echoes audit_hash on the payload when provided", () => {
    const dir = minimalCookbook(fx, "x");
    const p = buildDeployPayload({
      cookbookDir: dir,
      envVars: {},
      auditHash: "abc123",
    });
    expect(p.audit_hash).toBe("abc123");
  });

  it("omits audit_hash when not provided", () => {
    const dir = minimalCookbook(fx, "x");
    const p = buildDeployPayload({ cookbookDir: dir, envVars: {} });
    expect("audit_hash" in p).toBe(false);
  });
});
