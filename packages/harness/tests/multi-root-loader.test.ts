/**
 * Multi-root skill loader tests — Phase 40 Wave 4.
 *
 * Covers createMultiRootSkillLoader (skills/multi-root-loader.ts) and
 * the multi-root delegation path in createDirectSkillLoader (skills/loader.ts).
 *
 * 8 tests:
 *   1. Single-root mode back-compat: loadSkill finds a skill
 *   2. Single-root mode back-compat: missing skill throws
 *   3. Multi-root: skill found in first root is returned
 *   4. Multi-root: skill found in second root (not first) is returned
 *   5. Multi-root: skill absent from all roots throws with all attempted paths
 *   6. Multi-root: agent found in first agentsRoot is returned
 *   7. Multi-root: agent found in second agentsRoot is returned
 *   8. Multi-root: agent absent from all roots throws with all attempted paths
 */

import { mkdtempSync, mkdirSync, writeFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { createDirectSkillLoader } from "../src/skills/loader.js";
import { createMultiRootSkillLoader } from "../src/skills/multi-root-loader.js";

// ---------------------------------------------------------------------------
// Fixture helpers
// ---------------------------------------------------------------------------

let tmpRoot: string;
let rootA: string;
let rootB: string;
let agentsA: string;
let agentsB: string;

function writeSkill(root: string, id: string, body: string): void {
  const dir = join(root, id);
  mkdirSync(dir, { recursive: true });
  writeFileSync(join(dir, "SKILL.md"), `---\nname: ${id}\n---\n${body}`);
}

function writeAgent(agentsRoot: string, id: string, body: string): void {
  mkdirSync(agentsRoot, { recursive: true });
  writeFileSync(join(agentsRoot, `${id}.md`), `---\nname: ${id}\n---\n${body}`);
}

beforeEach(() => {
  tmpRoot = mkdtempSync(join(tmpdir(), "multi-root-loader-test-"));
  rootA = join(tmpRoot, "skillsA");
  rootB = join(tmpRoot, "skillsB");
  agentsA = join(tmpRoot, "agentsA");
  agentsB = join(tmpRoot, "agentsB");
  mkdirSync(rootA, { recursive: true });
  mkdirSync(rootB, { recursive: true });
  mkdirSync(agentsA, { recursive: true });
  mkdirSync(agentsB, { recursive: true });
});

afterEach(() => {
  rmSync(tmpRoot, { recursive: true, force: true });
});

// ---------------------------------------------------------------------------
// 1. Single-root back-compat via createDirectSkillLoader
// ---------------------------------------------------------------------------

describe("createDirectSkillLoader — single-root back-compat", () => {
  it("loadSkill finds a skill when only skillsRoot/agentsRoot provided", async () => {
    writeSkill(rootA, "alpha", "Alpha body.");
    const loader = createDirectSkillLoader({
      skillsRoot: rootA,
      agentsRoot: agentsA,
    });
    const skill = await loader.loadSkill("alpha");
    expect(skill.id).toBe("alpha");
    expect(skill.body.trim()).toBe("Alpha body.");
  });

  it("missing skill throws with helpful path in single-root mode", async () => {
    const loader = createDirectSkillLoader({
      skillsRoot: rootA,
      agentsRoot: agentsA,
    });
    await expect(loader.loadSkill("ghost")).rejects.toThrow(/ghost/);
  });
});

// ---------------------------------------------------------------------------
// 2. createMultiRootSkillLoader — skill resolution
// ---------------------------------------------------------------------------

describe("createMultiRootSkillLoader — skill resolution", () => {
  it("finds skill in first root, returns it", async () => {
    writeSkill(rootA, "first-root-skill", "Found in A.");
    const loader = createMultiRootSkillLoader({
      skillsRoots: [rootA, rootB],
      agentsRoots: [agentsA, agentsB],
    });
    const skill = await loader.loadSkill("first-root-skill");
    expect(skill.id).toBe("first-root-skill");
    expect(skill.body.trim()).toBe("Found in A.");
    expect(skill.path).toContain(rootA);
  });

  it("finds skill in second root when absent from first", async () => {
    writeSkill(rootB, "second-root-skill", "Found in B.");
    const loader = createMultiRootSkillLoader({
      skillsRoots: [rootA, rootB],
      agentsRoots: [agentsA, agentsB],
    });
    const skill = await loader.loadSkill("second-root-skill");
    expect(skill.id).toBe("second-root-skill");
    expect(skill.body.trim()).toBe("Found in B.");
    expect(skill.path).toContain(rootB);
  });

  it("throws with all attempted paths when skill absent from all roots", async () => {
    const loader = createMultiRootSkillLoader({
      skillsRoots: [rootA, rootB],
      agentsRoots: [agentsA, agentsB],
    });
    let caught: Error | undefined;
    try {
      await loader.loadSkill("nowhere");
    } catch (err) {
      caught = err as Error;
    }
    expect(caught).toBeDefined();
    expect(caught!.message).toMatch(/nowhere/);
    // Both attempted paths should appear in the error message
    expect(caught!.message).toContain(rootA);
    expect(caught!.message).toContain(rootB);
  });
});

// ---------------------------------------------------------------------------
// 3. createMultiRootSkillLoader — agent resolution
// ---------------------------------------------------------------------------

describe("createMultiRootSkillLoader — agent resolution", () => {
  it("finds agent in first agentsRoot, returns AgentDef", async () => {
    writeAgent(agentsA, "analyst-a", "You are analyst A.");
    const loader = createMultiRootSkillLoader({
      skillsRoots: [rootA, rootB],
      agentsRoots: [agentsA, agentsB],
    });
    const def = await loader.loadAgent("analyst-a");
    expect(def.id).toBe("analyst-a");
    expect(def.systemPrompt).toContain("analyst A");
  });

  it("finds agent in second agentsRoot when absent from first", async () => {
    writeAgent(agentsB, "analyst-b", "You are analyst B.");
    const loader = createMultiRootSkillLoader({
      skillsRoots: [rootA, rootB],
      agentsRoots: [agentsA, agentsB],
    });
    const def = await loader.loadAgent("analyst-b");
    expect(def.id).toBe("analyst-b");
    expect(def.systemPrompt).toContain("analyst B");
  });

  it("throws with all attempted paths when agent absent from all roots", async () => {
    const loader = createMultiRootSkillLoader({
      skillsRoots: [rootA, rootB],
      agentsRoots: [agentsA, agentsB],
    });
    let caught: Error | undefined;
    try {
      await loader.loadAgent("ghost-agent");
    } catch (err) {
      caught = err as Error;
    }
    expect(caught).toBeDefined();
    expect(caught!.message).toMatch(/ghost-agent/);
    expect(caught!.message).toContain(agentsA);
    expect(caught!.message).toContain(agentsB);
  });
});

// ---------------------------------------------------------------------------
// 4. createDirectSkillLoader multi-root delegation
// ---------------------------------------------------------------------------

describe("createDirectSkillLoader — skillsRoots/agentsRoots delegation", () => {
  it("delegates to multi-root when skillsRoots provided; finds skill in second root", async () => {
    writeSkill(rootB, "delegated-skill", "Found via delegation.");
    const loader = createDirectSkillLoader({
      skillsRoot: rootA, // legacy field still required
      agentsRoot: agentsA,
      skillsRoots: [rootA, rootB], // multi-root override
      agentsRoots: [agentsA, agentsB],
    });
    const skill = await loader.loadSkill("delegated-skill");
    expect(skill.body.trim()).toBe("Found via delegation.");
    expect(skill.path).toContain(rootB);
  });
});
