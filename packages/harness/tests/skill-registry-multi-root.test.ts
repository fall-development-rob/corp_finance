/**
 * Skill registry multi-root tests — Phase 40 Wave 4.
 *
 * Verifies that createSkillRegistry() correctly discovers agents and skills
 * from the 3-tier plugin structure. Tests use the real filesystem layout
 * under plugins/ so they exercise actual agent-plugins + legacy cfa-core paths.
 *
 * 5 tests:
 *   1. discoverPluginRoots includes paths from all 3 tiers
 *   2. discoverPluginRoots includes legacy cfa-core fallbacks
 *   3. createSkillRegistry() with explicit skillsRoots/agentsRoots loads chief-analyst
 *   4. Single-root legacy mode still works (back-compat)
 *   5. Multi-root mode: agent from agent-plugins tier is loadable
 */

import { describe, expect, it } from "vitest";
import { mkdtemp, rm } from "node:fs/promises";
import { mkdirSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import {
  createSkillRegistry,
  discoverPluginRoots,
} from "../src/agents/skill-registry.js";
import { createMultiRootSkillLoader } from "../src/skills/multi-root-loader.js";

// ---------------------------------------------------------------------------
// Path resolution
// ---------------------------------------------------------------------------

const _thisDir = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(_thisDir, "..", "..", "..");

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("discoverPluginRoots", () => {
  it("includes roots from all 3 plugin tiers (agent-plugins, vertical-plugins, partner-built)", () => {
    const { skillsRoots, agentsRoots } = discoverPluginRoots(repoRoot);

    // agent-plugins tier has skills in several slugs
    const agentPluginsSkills = skillsRoots.filter((r) =>
      r.includes("agent-plugins"),
    );
    expect(agentPluginsSkills.length).toBeGreaterThan(0);

    // vertical-plugins tier has skills (foundations, equity-research, etc.)
    const verticalSkills = skillsRoots.filter((r) =>
      r.includes("vertical-plugins"),
    );
    expect(verticalSkills.length).toBeGreaterThan(0);

    // partner-built tier has skills (fmp, factset, etc.)
    const partnerSkills = skillsRoots.filter((r) =>
      r.includes("partner-built"),
    );
    expect(partnerSkills.length).toBeGreaterThan(0);

    // agent-plugins tier has agents
    const agentPluginsAgents = agentsRoots.filter((r) =>
      r.includes("agent-plugins"),
    );
    expect(agentPluginsAgents.length).toBeGreaterThan(0);
  });

  it("includes legacy cfa-core fallbacks at the end of each list", () => {
    const { skillsRoots, agentsRoots } = discoverPluginRoots(repoRoot);

    // Legacy paths are appended last
    expect(skillsRoots[skillsRoots.length - 1]).toContain("cfa-core");
    expect(agentsRoots[agentsRoots.length - 1]).toContain("cfa-core");
  });
});

describe("createSkillRegistry — multi-root discovery", () => {
  it("loads chief-analyst from agent-plugins tier via auto-discovery (no options)", async () => {
    // Default call — exercises the full tier walk + legacy fallback
    const registry = await createSkillRegistry();
    const chief = registry.chief();
    expect(chief.id).toBe("chief-analyst");
    expect(chief.systemPrompt.length).toBeGreaterThan(200);
    expect(registry.all().length).toBe(9);
  });

  it("single-root legacy mode (explicit skillsRoot/agentsRoot) still works", async () => {
    const skillsRoot = resolve(repoRoot, "plugins", "cfa-core", "skills", "cfa");
    const agentsRoot = resolve(repoRoot, "plugins", "cfa-core", "agents", "cfa");
    const registry = await createSkillRegistry({ skillsRoot, agentsRoot });
    expect(registry.chief().id).toBe("chief-analyst");
    expect(registry.all().length).toBe(9);
  });

  it("explicit skillsRoots/agentsRoots from agent-plugins tier can load equity-analyst", async () => {
    const { skillsRoots, agentsRoots } = discoverPluginRoots(repoRoot);
    const registry = await createSkillRegistry({ skillsRoots, agentsRoots });
    const equity = registry.get("equity-analyst");
    expect(equity.id).toBe("equity-analyst");
    expect(equity.systemPrompt.length).toBeGreaterThan(200);
  });

  it("fails fast with useful error when custom empty roots given (back-compat)", async () => {
    const tmpSkills = await mkdtemp(resolve(tmpdir(), "cfa-mr-skills-"));
    const tmpAgents = await mkdtemp(resolve(tmpdir(), "cfa-mr-agents-"));
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

  it("createMultiRootSkillLoader resolves skills from vertical-plugins/foundations", async () => {
    const { skillsRoots, agentsRoots } = discoverPluginRoots(repoRoot);
    const loader = createMultiRootSkillLoader({ skillsRoots, agentsRoots });

    // foundations/ has corp-finance-analyst-core skill in vertical-plugins
    const skill = await loader.loadSkill("corp-finance-analyst-core");
    expect(skill.id).toBe("corp-finance-analyst-core");
    expect(skill.body.length).toBeGreaterThan(100);
    // Verify it came from vertical-plugins
    expect(skill.path).toContain("vertical-plugins");
  });
});
