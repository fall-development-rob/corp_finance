/**
 * Phase 25 Tier D14 — cookbook scaffolder tests.
 *
 * Verifies that buildScaffoldedCookbook produces a skeleton that satisfies
 * every MA-* contract from docs/contracts/feature_managed_agents.yml when
 * materialised to disk. Failures here mean the scaffolder has drifted
 * away from the contract surface.
 */

import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { mkdtempSync, mkdirSync, writeFileSync, rmSync, existsSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, dirname } from "node:path";
import { parse as parseYaml } from "yaml";

import {
  buildScaffoldedCookbook,
  ScaffoldError,
  SLUG_RE,
} from "../src/manifests/cookbook-scaffold.js";
import { isValidSemver } from "../src/manifests/semver.js";
import type { AgentManifest, ToolsetConfig } from "../src/manifests/types.js";

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

interface Fixture {
  root: string;
  cleanup: () => void;
}

function makeFixture(): Fixture {
  const root = mkdtempSync(join(tmpdir(), "scaffold-cookbook-"));
  return { root, cleanup: () => rmSync(root, { recursive: true, force: true }) };
}

function writeScaffold(fx: Fixture, slug: string, domain?: string): string {
  const sc = buildScaffoldedCookbook({ slug, ...(domain ? { domain } : {}) });
  const cookbookDir = join(fx.root, "managed-agent-cookbooks", slug);
  for (const f of sc.files) {
    const path = join(cookbookDir, f.relPath);
    mkdirSync(dirname(path), { recursive: true });
    writeFileSync(path, f.contents, "utf8");
  }
  return cookbookDir;
}

function readManifest(path: string): AgentManifest {
  // eslint-disable-next-line @typescript-eslint/no-var-requires
  const { readFileSync } = require("node:fs") as typeof import("node:fs");
  return parseYaml(readFileSync(path, "utf8")) as AgentManifest;
}

function mcpToolsetBlocks(
  tools: ToolsetConfig[] | undefined,
): Array<{
  type: "mcp_toolset";
  mcp_server_name: string;
  default_config?: { enabled: boolean };
  configs?: unknown[];
}> {
  if (!tools) return [];
  return tools.filter(
    (t): t is {
      type: "mcp_toolset";
      mcp_server_name: string;
      default_config?: { enabled: boolean };
      configs?: unknown[];
    } => t.type === "mcp_toolset",
  );
}

// ---------------------------------------------------------------------------
// Slug validation
// ---------------------------------------------------------------------------

describe("buildScaffoldedCookbook — slug validation", () => {
  it("accepts valid slugs", () => {
    expect(() => buildScaffoldedCookbook({ slug: "valid-slug" })).not.toThrow();
    expect(() => buildScaffoldedCookbook({ slug: "abc" })).not.toThrow();
    expect(() => buildScaffoldedCookbook({ slug: "a-b-c-1-2-3" })).not.toThrow();
  });

  it("rejects empty / too-short slugs", () => {
    expect(() => buildScaffoldedCookbook({ slug: "" })).toThrow(ScaffoldError);
    expect(() => buildScaffoldedCookbook({ slug: "ab" })).toThrow(ScaffoldError);
  });

  it("rejects uppercase letters", () => {
    expect(() => buildScaffoldedCookbook({ slug: "MyCookbook" })).toThrow(
      ScaffoldError,
    );
  });

  it("rejects leading or trailing hyphens", () => {
    expect(() => buildScaffoldedCookbook({ slug: "-hello" })).toThrow(
      ScaffoldError,
    );
    expect(() => buildScaffoldedCookbook({ slug: "hello-" })).toThrow(
      ScaffoldError,
    );
  });

  it("rejects underscores", () => {
    expect(() => buildScaffoldedCookbook({ slug: "snake_case" })).toThrow(
      ScaffoldError,
    );
  });

  it("SLUG_RE matches the MA-007 contract regex", () => {
    // Sanity: each existing cookbook slug should validate.
    for (const s of [
      "equity-analyst",
      "credit-analyst",
      "private-markets-analyst",
      "lp-statement-auditor",
    ]) {
      expect(SLUG_RE.test(s)).toBe(true);
    }
  });
});

// ---------------------------------------------------------------------------
// File set
// ---------------------------------------------------------------------------

describe("buildScaffoldedCookbook — file set", () => {
  it("emits exactly 5 files: agent.yaml, steering, 3 subagents", () => {
    const sc = buildScaffoldedCookbook({ slug: "demo" });
    expect(sc.files.map((f) => f.relPath).sort()).toEqual([
      "agent.yaml",
      "steering-examples.json",
      "subagents/data-reader.yaml",
      "subagents/publisher.yaml",
      "subagents/worker.yaml",
    ]);
  });

  it("produces byte-deterministic output for the same input", () => {
    const a = buildScaffoldedCookbook({ slug: "demo", domain: "x" });
    const b = buildScaffoldedCookbook({ slug: "demo", domain: "x" });
    for (let i = 0; i < a.files.length; i++) {
      expect(a.files[i]?.contents).toBe(b.files[i]?.contents);
    }
  });
});

// ---------------------------------------------------------------------------
// Contract conformance — scaffold should pass every MA-* rule when written
// ---------------------------------------------------------------------------

describe("scaffold conformance to MA-* contracts", () => {
  let fx: Fixture;
  beforeEach(() => {
    fx = makeFixture();
  });
  afterEach(() => fx.cleanup());

  it("MA-001: agent.yaml + steering-examples.json materialise on disk", () => {
    const dir = writeScaffold(fx, "demo");
    expect(existsSync(join(dir, "agent.yaml"))).toBe(true);
    expect(existsSync(join(dir, "steering-examples.json"))).toBe(true);
  });

  it("MA-002: parent declares callable_agents[] with 3 entries", () => {
    const dir = writeScaffold(fx, "demo");
    const parent = readManifest(join(dir, "agent.yaml"));
    expect(parent.callable_agents).toBeDefined();
    expect((parent.callable_agents ?? []).length).toBe(3);
  });

  it("MA-003: scaffold ships exactly 3 subagents", () => {
    const dir = writeScaffold(fx, "demo");
    for (const role of ["data-reader", "worker", "publisher"]) {
      expect(existsSync(join(dir, "subagents", `${role}.yaml`))).toBe(true);
    }
  });

  it("MA-004: every cfa-core toolset uses explicit-allow gating + non-empty configs", () => {
    const dir = writeScaffold(fx, "demo");
    for (const role of ["data-reader", "worker"]) {
      const sub = readManifest(join(dir, "subagents", `${role}.yaml`));
      const cfaCoreBlocks = mcpToolsetBlocks(sub.tools).filter(
        (b) => b.mcp_server_name === "cfa-core",
      );
      // Publisher has no cfa-core; data-reader + worker do.
      expect(cfaCoreBlocks.length).toBeGreaterThan(0);
      for (const block of cfaCoreBlocks) {
        expect(block.default_config?.enabled).toBe(false);
        expect(Array.isArray(block.configs)).toBe(true);
        expect((block.configs ?? []).length).toBeGreaterThan(0);
      }
    }
  });

  it("MA-005: every subagent declares an output_schema", () => {
    const dir = writeScaffold(fx, "demo");
    for (const role of ["data-reader", "worker", "publisher"]) {
      const sub = readManifest(join(dir, "subagents", `${role}.yaml`));
      expect(sub.output_schema).toBeDefined();
      expect(typeof sub.output_schema).toBe("object");
    }
  });

  it("MA-006: parent system.append contains 'DATA, not directives'", () => {
    const dir = writeScaffold(fx, "demo");
    const parent = readManifest(join(dir, "agent.yaml"));
    expect(parent.system?.append ?? "").toContain("DATA, not directives");
  });

  it("MA-007: scaffolded slug satisfies the deploy-ID pattern", () => {
    for (const s of ["demo", "long-slug-with-many-parts", "abc-123"]) {
      expect(SLUG_RE.test(s)).toBe(true);
    }
  });

  it("MA-008: parent declares version that is valid semver", () => {
    const dir = writeScaffold(fx, "demo");
    const parent = readManifest(join(dir, "agent.yaml"));
    expect(isValidSemver(parent.version)).toBe(true);
    expect(parent.version).toBe("1.0.0");
  });
});

// ---------------------------------------------------------------------------
// Domain-hint pass-through
// ---------------------------------------------------------------------------

describe("buildScaffoldedCookbook — domain hint", () => {
  it("inserts the domain hint into the parent description", () => {
    const sc = buildScaffoldedCookbook({
      slug: "demo",
      domain: "equity research",
    });
    const agentYaml = sc.files.find((f) => f.relPath === "agent.yaml");
    expect(agentYaml?.contents).toContain("equity research");
  });

  it("falls back to the slug when no domain is provided", () => {
    const sc = buildScaffoldedCookbook({ slug: "demo" });
    const agentYaml = sc.files.find((f) => f.relPath === "agent.yaml");
    expect(agentYaml?.contents).toContain("demo");
  });

  it("inserts the domain hint into steering-examples descriptions", () => {
    const sc = buildScaffoldedCookbook({
      slug: "demo",
      domain: "private credit",
    });
    const steering = sc.files.find((f) => f.relPath === "steering-examples.json");
    expect(steering?.contents).toContain("private credit");
  });
});

// ---------------------------------------------------------------------------
// Steering examples shape
// ---------------------------------------------------------------------------

describe("scaffolded steering-examples.json", () => {
  it("is valid JSON array of {event, description}", () => {
    const sc = buildScaffoldedCookbook({ slug: "demo" });
    const steering = sc.files.find((f) => f.relPath === "steering-examples.json");
    const parsed = JSON.parse(steering!.contents);
    expect(Array.isArray(parsed)).toBe(true);
    expect(parsed.length).toBeGreaterThan(0);
    for (const entry of parsed) {
      expect(typeof entry.event).toBe("string");
      expect(typeof entry.description).toBe("string");
    }
  });
});
