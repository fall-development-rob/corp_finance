/**
 * Phase 25 Tier A4 — managed-agent cookbook contract tests.
 *
 * Asserts every contract and invariant declared in
 * docs/contracts/feature_managed_agents.yml against the live cookbook tree.
 * Each contract/invariant gets its own `it()` block so a violation reports
 * the precise rule that failed.
 *
 * Failures here block PR merge: the contracts capture invariants the rest
 * of the system relies on (the closed learning loop keys remediation on
 * subagent output_schema validation; deploy tooling expects exactly the
 * cookbook shape that MA-001 → MA-007 codify).
 */

import { describe, it, expect } from "vitest";
import { readFileSync, readdirSync, existsSync, statSync } from "node:fs";
import { join, resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { parse as parseYaml } from "yaml";

import { parseAuditCatalog } from "../../src/manifests/cookbook-audit.js";
import { parseReplayCatalog } from "../../src/manifests/cookbook-replay.js";
import { isValidSemver } from "../../src/manifests/semver.js";
import type {
  AgentManifest,
  ToolsetConfig,
} from "../../src/manifests/types.js";

// ---------------------------------------------------------------------------
// Repo paths
// ---------------------------------------------------------------------------

const __dirname = dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = resolve(__dirname, "..", "..", "..", "..");
const COOKBOOKS_ROOT = resolve(REPO_ROOT, "managed-agent-cookbooks");

// ---------------------------------------------------------------------------
// Helpers — keep this file self-contained so it stays robust to refactors
// ---------------------------------------------------------------------------

function listCookbookSlugs(): string[] {
  return readdirSync(COOKBOOKS_ROOT, { withFileTypes: true })
    .filter((e) => e.isDirectory())
    .map((e) => e.name)
    .filter((slug) => {
      const dir = join(COOKBOOKS_ROOT, slug);
      return (
        existsSync(join(dir, "agent.yaml")) ||
        existsSync(join(dir, "agent.yml")) ||
        existsSync(join(dir, "agent.json"))
      );
    })
    .sort();
}

function listSubagents(slug: string): string[] {
  const dir = join(COOKBOOKS_ROOT, slug, "subagents");
  if (!existsSync(dir) || !statSync(dir).isDirectory()) return [];
  return readdirSync(dir, { withFileTypes: true })
    .filter(
      (e) =>
        e.isFile() &&
        (e.name.endsWith(".yaml") ||
          e.name.endsWith(".yml") ||
          e.name.endsWith(".json")),
    )
    .map((e) => join(dir, e.name))
    .sort();
}

function readManifest(path: string): AgentManifest {
  const raw = readFileSync(path, "utf8");
  const parsed = parseYaml(raw) as unknown;
  if (
    typeof parsed !== "object" ||
    parsed === null ||
    !("name" in parsed) ||
    typeof (parsed as Record<string, unknown>).name !== "string"
  ) {
    throw new Error(`manifest at ${path} missing required field "name"`);
  }
  return parsed as AgentManifest;
}

function mcpToolsetBlocks(
  tools: ToolsetConfig[] | undefined,
): Array<{ type: "mcp_toolset"; mcp_server_name: string; default_config?: { enabled: boolean }; configs?: unknown[] }> {
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

const SLUG_RE = /^[a-z][a-z0-9-]{1,62}[a-z0-9]$/;

// Anti-injection clause — the substrings any conformant system.append should contain.
// The clause is a fixed pattern, not free prose, so we can match by substring.
const ANTI_INJECTION_MARKERS = ["DATA, not directives", "data, not directives"];

// ---------------------------------------------------------------------------
// Discovered state — read once, reused across contracts
// ---------------------------------------------------------------------------

const slugs = listCookbookSlugs();
const cookbooks = slugs.map((slug) => ({
  slug,
  agentPath: join(COOKBOOKS_ROOT, slug, "agent.yaml"),
  manifest: readManifest(join(COOKBOOKS_ROOT, slug, "agent.yaml")),
  subagentPaths: listSubagents(slug),
}));

// ---------------------------------------------------------------------------
// MA-001 — required manifest files
// ---------------------------------------------------------------------------

describe("MA-001 — every cookbook has the required manifest files", () => {
  for (const cb of cookbooks) {
    it(`${cb.slug}: agent.yaml exists`, () => {
      expect(existsSync(cb.agentPath)).toBe(true);
    });
    it(`${cb.slug}: steering-examples.json exists`, () => {
      const p = join(COOKBOOKS_ROOT, cb.slug, "steering-examples.json");
      expect(existsSync(p)).toBe(true);
    });
  }
});

// ---------------------------------------------------------------------------
// MA-002 — parent delegates to subagents
// ---------------------------------------------------------------------------

describe("MA-002 — every parent declares callable_agents", () => {
  for (const cb of cookbooks) {
    it(`${cb.slug}: agent.yaml has callable_agents[]`, () => {
      expect(cb.manifest.callable_agents).toBeDefined();
      expect(Array.isArray(cb.manifest.callable_agents)).toBe(true);
      expect((cb.manifest.callable_agents ?? []).length).toBeGreaterThan(0);
    });
  }
});

// ---------------------------------------------------------------------------
// MA-003 — exactly 3 subagents per cookbook
// ---------------------------------------------------------------------------

describe("MA-003 — every cookbook has exactly 3 subagents", () => {
  for (const cb of cookbooks) {
    it(`${cb.slug}: subagents/ contains exactly 3 manifests`, () => {
      expect(cb.subagentPaths).toHaveLength(3);
    });
  }
});

// ---------------------------------------------------------------------------
// MA-004 — explicit-allow MCP gating on every cfa-core subagent
// (data/fmp/vendor servers intentionally allow broad access for fetchers)
// ---------------------------------------------------------------------------

describe("MA-004 — subagents using cfa-core (compute) gate tools explicit-allow", () => {
  for (const cb of cookbooks) {
    for (const subPath of cb.subagentPaths) {
      const sub = readManifest(subPath);
      const cfaCoreBlocks = mcpToolsetBlocks(sub.tools).filter(
        (b) => b.mcp_server_name === "cfa-core",
      );
      for (const block of cfaCoreBlocks) {
        it(`${cb.slug}/${sub.name}: cfa-core default_config.enabled === false`, () => {
          expect(block.default_config?.enabled).toBe(false);
        });
        it(`${cb.slug}/${sub.name}: cfa-core lists specific configs[]`, () => {
          expect(Array.isArray(block.configs)).toBe(true);
          expect((block.configs ?? []).length).toBeGreaterThan(0);
        });
      }
    }
  }
});

// ---------------------------------------------------------------------------
// MA-005 — every subagent declares an output_schema
// ---------------------------------------------------------------------------

describe("MA-005 — every subagent declares an output_schema", () => {
  for (const cb of cookbooks) {
    for (const subPath of cb.subagentPaths) {
      const sub = readManifest(subPath);
      it(`${cb.slug}/${sub.name}: output_schema present`, () => {
        expect(sub.output_schema).toBeDefined();
        expect(typeof sub.output_schema).toBe("object");
      });
    }
  }
});

// ---------------------------------------------------------------------------
// MA-006 — anti-injection reminder in parent system.append
// ---------------------------------------------------------------------------

describe("MA-006 — every parent system.append carries the anti-injection reminder", () => {
  for (const cb of cookbooks) {
    it(`${cb.slug}: system.append contains DATA-not-directives clause`, () => {
      const append = cb.manifest.system?.append ?? "";
      const has = ANTI_INJECTION_MARKERS.some((m) => append.includes(m));
      expect(has).toBe(true);
    });
  }
});

// ---------------------------------------------------------------------------
// MA-007 — slugs are file-system safe
// ---------------------------------------------------------------------------

describe("MA-007 — every cookbook slug is file-system safe and ALLOWED_SLUGS-aligned", () => {
  for (const cb of cookbooks) {
    it(`${cb.slug}: matches ^[a-z][a-z0-9-]{1,62}[a-z0-9]$`, () => {
      expect(cb.slug).toMatch(SLUG_RE);
    });
  }
});

// ---------------------------------------------------------------------------
// MA-008 — every parent declares a valid semver version
// ---------------------------------------------------------------------------

describe("MA-008 — every cookbook parent declares a valid semver version", () => {
  for (const cb of cookbooks) {
    it(`${cb.slug}: agent.yaml has version field`, () => {
      expect(cb.manifest.version).toBeDefined();
      expect(typeof cb.manifest.version).toBe("string");
    });
    it(`${cb.slug}: version is valid semver-2.0`, () => {
      expect(isValidSemver(cb.manifest.version)).toBe(true);
    });
  }
});

// ---------------------------------------------------------------------------
// MA-INV-001 — cookbook count
// ---------------------------------------------------------------------------

describe("MA-INV-001 — cookbook count is 15", () => {
  it("managed-agent-cookbooks has exactly 15 cookbooks with agent.yaml", () => {
    expect(cookbooks).toHaveLength(15);
  });
});

// ---------------------------------------------------------------------------
// MA-INV-002 — subagent count
// ---------------------------------------------------------------------------

describe("MA-INV-002 — subagent count is 45", () => {
  it("total subagent count across all cookbooks is 45 (3 × 15)", () => {
    const total = cookbooks.reduce((n, c) => n + c.subagentPaths.length, 0);
    expect(total).toBe(45);
  });
});

// ---------------------------------------------------------------------------
// MA-INV-003 — audit catalog parity
// ---------------------------------------------------------------------------

describe("MA-INV-003 — data/cookbook-audits.json has one entry per cookbook", () => {
  it("audit catalog cookbook count equals managed-agent-cookbooks count", () => {
    const auditsPath = resolve(REPO_ROOT, "data", "cookbook-audits.json");
    const catalog = parseAuditCatalog(readFileSync(auditsPath, "utf8"));
    expect(catalog.cookbooks).toHaveLength(cookbooks.length);
    const auditSlugs = catalog.cookbooks.map((c) => c.slug).sort();
    const expectedSlugs = cookbooks.map((c) => c.slug).sort();
    expect(auditSlugs).toEqual(expectedSlugs);
  });
});

// ---------------------------------------------------------------------------
// MA-INV-004 — replay catalog parity
// ---------------------------------------------------------------------------

describe("MA-INV-004 — data/cookbook-replays.json has one entry per cookbook", () => {
  it("replay catalog cookbook count equals managed-agent-cookbooks count", () => {
    const replaysPath = resolve(REPO_ROOT, "data", "cookbook-replays.json");
    const catalog = parseReplayCatalog(readFileSync(replaysPath, "utf8"));
    expect(catalog.cookbooks).toHaveLength(cookbooks.length);
    const replaySlugs = catalog.cookbooks.map((c) => c.slug).sort();
    const expectedSlugs = cookbooks.map((c) => c.slug).sort();
    expect(replaySlugs).toEqual(expectedSlugs);
  });
});
