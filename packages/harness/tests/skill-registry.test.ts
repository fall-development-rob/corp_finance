/**
 * Skill-backed registry — Phase 33 Wave 3 tests.
 *
 * Verifies that createSkillRegistry() loads all 9 agents in parallel from
 * .claude/skills/cfa + .claude/agents/cfa, and that each AgentDef matches
 * (by id / tools / model / maxTokens / maxRecursionDepth and normalized
 * systemPrompt) the corresponding TypeScript-imported AgentDef.
 *
 * Per-agent byte-equivalence is already proven by the *-skill-canary tests;
 * this suite re-asserts the same parity through the registry surface and
 * additionally verifies registry semantics (delegation order, get/throw,
 * fail-fast on missing manifests, custom-paths propagation).
 *
 * Normalization rules (must match chief-skill-canary.test.ts):
 *   • Trim leading/trailing whitespace.
 *   • Normalize CRLF → LF.
 *   • Strip trailing whitespace from every line.
 *   • Collapse 3+ consecutive blank lines to exactly 2.
 */

import { describe, expect, it } from "vitest";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import {
  createSkillRegistry,
  SKILL_REGISTRY_AGENT_IDS,
} from "../src/agents/skill-registry.js";

import {
  chiefAnalyst,
  defaultDelegates,
} from "../src/agents/registry.js";

import type { AgentDef } from "../src/types.js";

// ---------------------------------------------------------------------------
// Path resolution + helpers
// ---------------------------------------------------------------------------

const _thisDir = dirname(fileURLToPath(import.meta.url));
// packages/harness/tests -> packages/harness -> packages -> repo root
const repoRoot = resolve(_thisDir, "..", "..", "..");

const skillsRoot = resolve(repoRoot, ".claude", "skills", "cfa");
const agentsRoot = resolve(repoRoot, ".claude", "agents", "cfa");

function normalize(s: string): string {
  return s
    .trim()
    .replace(/\r\n/g, "\n")
    .replace(/[ \t]+\n/g, "\n")
    .replace(/\n{3,}/g, "\n\n");
}

const tsById = new Map<string, AgentDef>([
  [chiefAnalyst.id, chiefAnalyst],
  ...defaultDelegates.map((d) => [d.id, d] as [string, AgentDef]),
]);

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("createSkillRegistry (Phase 33 Wave 3)", () => {
  // -------------------------------------------------------------------------
  // Test 1 — Loads all 9 in parallel, canonical ordering
  // -------------------------------------------------------------------------
  it("loads all 9 agents in parallel with canonical ordering", async () => {
    const registry = await createSkillRegistry({ skillsRoot, agentsRoot });

    expect(registry.all().length).toBe(9);
    expect(registry.chief().id).toBe("chief-analyst");

    const delegateIds = registry.delegates().map((d) => d.id);
    expect(delegateIds.length).toBe(8);
    expect(delegateIds).toEqual([
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

  // -------------------------------------------------------------------------
  // Test 2 — AgentDef parity vs TS-imported registry (the whole 9)
  // -------------------------------------------------------------------------
  it("each loaded AgentDef matches its TS-defined counterpart (id / tools / model / maxTokens / maxRecursionDepth / normalized systemPrompt)", async () => {
    const registry = await createSkillRegistry({ skillsRoot, agentsRoot });

    for (const id of SKILL_REGISTRY_AGENT_IDS) {
      const loaded = registry.get(id);
      const ts = tsById.get(id);
      expect(ts, `TS registry missing id ${id}`).toBeDefined();

      expect(loaded.id, `${id}: id mismatch`).toBe(ts!.id);
      expect(loaded.tools, `${id}: tools mismatch`).toEqual(ts!.tools);
      expect(loaded.model, `${id}: model mismatch`).toBe(ts!.model);
      expect(loaded.maxTokens, `${id}: maxTokens mismatch`).toBe(
        ts!.maxTokens,
      );
      expect(
        loaded.maxRecursionDepth,
        `${id}: maxRecursionDepth mismatch`,
      ).toBe(ts!.maxRecursionDepth);

      // systemPrompt byte-equivalence after normalization. Matches the canary
      // gate; per-agent canaries already prove this individually.
      expect(
        normalize(loaded.systemPrompt),
        `${id}: normalized systemPrompt mismatch`,
      ).toBe(normalize(ts!.systemPrompt));
    }
  });

  // -------------------------------------------------------------------------
  // Test 3 — get(id) round-trip + throws on unknown
  // -------------------------------------------------------------------------
  it("get(id) round-trips for every canonical id and throws on unknown", async () => {
    const registry = await createSkillRegistry({ skillsRoot, agentsRoot });

    for (const id of SKILL_REGISTRY_AGENT_IDS) {
      expect(registry.get(id).id).toBe(id);
    }

    expect(() => registry.get("nonexistent")).toThrow(/Unknown agent id/);
    expect(() => registry.get("nonexistent")).toThrow(/nonexistent/);
  });

  // -------------------------------------------------------------------------
  // Test 4 — delegates() ordering matches registry.ts::defaultDelegates
  // -------------------------------------------------------------------------
  it("delegates() ordering matches registry.ts::defaultDelegates", async () => {
    const registry = await createSkillRegistry({ skillsRoot, agentsRoot });

    const loadedIds = registry.delegates().map((d) => d.id);
    const tsIds = defaultDelegates.map((d) => d.id);
    expect(loadedIds).toEqual(tsIds);
  });

  // -------------------------------------------------------------------------
  // Test 5 — fails fast if a manifest is missing (custom paths honoured)
  // -------------------------------------------------------------------------
  it("fails fast with a useful error if any of the 9 manifests is missing", async () => {
    const tmpSkills = await mkdtemp(resolve(tmpdir(), "cfa-skills-"));
    const tmpAgents = await mkdtemp(resolve(tmpdir(), "cfa-agents-"));

    try {
      await expect(
        createSkillRegistry({
          skillsRoot: tmpSkills,
          agentsRoot: tmpAgents,
        }),
      ).rejects.toThrow(/chief-analyst/);
    } finally {
      await rm(tmpSkills, { recursive: true, force: true });
      await rm(tmpAgents, { recursive: true, force: true });
    }
  });
});
