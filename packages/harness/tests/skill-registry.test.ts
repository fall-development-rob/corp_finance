/**
 * Skill-backed registry — Phase 33 Wave 3 tests (updated Wave 4).
 *
 * Wave 3 verified that createSkillRegistry() loads all 9 agents in parallel
 * from plugins/cfa-core/skills/cfa + plugins/cfa-core/agents/cfa, and that
 * each loaded AgentDef matched the legacy TypeScript-imported AgentDef. Per-agent
 * byte-equivalence was independently proven by 9 *-skill-canary test files.
 *
 * Wave 4 deletes the legacy TS specialist sources; the canaries become
 * circular and were also deleted. The TS-vs-skill parity test in this file
 * is replaced with a structural shape assertion that verifies each loaded
 * AgentDef has the expected fields and types.
 *
 * The remaining tests (1, 3, 4, 5) verify registry semantics:
 *   • parallel load + canonical ordering
 *   • get(id) round-trip + throws on unknown id
 *   • delegates() ordering
 *   • fail-fast on missing manifests with custom paths
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

import { defaultDelegates } from "../src/agents/registry.js";

// ---------------------------------------------------------------------------
// Path resolution
// ---------------------------------------------------------------------------

const _thisDir = dirname(fileURLToPath(import.meta.url));
// packages/harness/tests -> packages/harness -> packages -> repo root
const repoRoot = resolve(_thisDir, "..", "..", "..");

const skillsRoot = resolve(repoRoot, "plugins", "cfa-core", "skills", "cfa");
const agentsRoot = resolve(repoRoot, "plugins", "cfa-core", "agents", "cfa");

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
  // Test 2 — Loaded AgentDefs have the expected shape
  //
  // Replaces the Wave-3 TS-vs-skill parity test (now circular post-Wave-4
  // since registry.ts is itself skill-loaded). Byte-equivalence pre-deletion
  // was proven by the 9 *-skill-canary tests; post-deletion we verify
  // structural correctness against the AgentDef contract.
  // -------------------------------------------------------------------------
  it("loaded AgentDefs have the expected shape across all 9 ids", async () => {
    const registry = await createSkillRegistry({ skillsRoot, agentsRoot });
    for (const id of SKILL_REGISTRY_AGENT_IDS) {
      const def = registry.get(id);
      expect(def.id).toBe(id);
      expect(typeof def.systemPrompt).toBe("string");
      expect(def.systemPrompt.length).toBeGreaterThan(500);
      expect(def.tools === "*" || Array.isArray(def.tools)).toBe(true);
      expect(typeof def.model).toBe("string");
      expect(typeof def.maxTokens).toBe("number");
      expect(def.maxTokens).toBeGreaterThan(0);
      expect(typeof (def.maxRecursionDepth ?? 0)).toBe("number");
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
