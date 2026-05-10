/**
 * Skill loader infrastructure tests — Phase 33 Wave 1.
 *
 * Covers:
 *   - parseFrontmatter (frontmatter-parser.ts)
 *   - assembleSystemPrompt (assembler.ts)
 *   - createDirectSkillLoader (loader.ts)
 *
 * Filesystem fixtures are written to os.tmpdir() + mkdtempSync and
 * cleaned up after each suite.
 */

import { mkdtempSync, mkdirSync, writeFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { parseFrontmatter } from "../src/skills/frontmatter-parser.js";
import { assembleSystemPrompt } from "../src/skills/assembler.js";
import { createDirectSkillLoader } from "../src/skills/loader.js";
import type { ParsedSkill } from "../src/skills/types.js";

// ---------------------------------------------------------------------------
// parseFrontmatter
// ---------------------------------------------------------------------------

describe("parseFrontmatter", () => {
  it("no frontmatter: full content returned as body", () => {
    const content = "# Hello\nSome body text.";
    const { frontmatter, body } = parseFrontmatter(content);
    expect(frontmatter).toEqual({});
    expect(body).toBe(content);
  });

  it("empty frontmatter block: empty object and body after fence", () => {
    const content = "---\n---\nBody here.";
    const { frontmatter, body } = parseFrontmatter(content);
    expect(frontmatter).toEqual({});
    expect(body).toBe("Body here.");
  });

  it("single string field", () => {
    const content = "---\nname: my-skill\n---\nBody.";
    const { frontmatter } = parseFrontmatter(content);
    expect(frontmatter.name).toBe("my-skill");
  });

  it("boolean fields", () => {
    const content = "---\nactive: true\ndisabled: false\n---";
    const { frontmatter } = parseFrontmatter(content);
    expect(frontmatter.active).toBe(true);
    expect(frontmatter.disabled).toBe(false);
  });

  it("number value", () => {
    const content = "---\nmax_tokens: 16384\n---";
    const { frontmatter } = parseFrontmatter(content);
    expect(frontmatter.max_tokens).toBe(16384);
  });

  it("inline array", () => {
    const content = "---\ntags: [alpha, beta, gamma]\n---";
    const { frontmatter } = parseFrontmatter(content);
    expect(frontmatter.tags).toEqual(["alpha", "beta", "gamma"]);
  });

  it("block array (sequence items)", () => {
    const content = "---\nextends:\n  - skill-a\n  - skill-b\n---";
    const { frontmatter } = parseFrontmatter(content);
    expect(frontmatter.extends).toEqual(["skill-a", "skill-b"]);
  });

  it("quoted string value containing colons", () => {
    const content = '---\ndescription: "A: B: C"\n---';
    const { frontmatter } = parseFrontmatter(content);
    expect(frontmatter.description).toBe("A: B: C");
  });

  it("single-quoted string value", () => {
    const content = "---\nmodel: 'claude-opus-4-5'\n---";
    const { frontmatter } = parseFrontmatter(content);
    expect(frontmatter.model).toBe("claude-opus-4-5");
  });

  it("description: | literal block scalar (multi-line)", () => {
    const content =
      "---\ndescription: |\n  Line one.\n  Line two.\n---\nBody.";
    const { frontmatter, body } = parseFrontmatter(content);
    expect(frontmatter.description).toBe("Line one.\nLine two.\n");
    expect(body).toBe("Body.");
  });

  it("unclosed frontmatter: full content returned as body", () => {
    const content = "---\nname: broken";
    const { frontmatter, body } = parseFrontmatter(content);
    expect(frontmatter).toEqual({});
    expect(body).toBe(content);
  });

  it("body has leading blank line stripped after fence", () => {
    const content = "---\nname: x\n---\n\nActual body.";
    const { body } = parseFrontmatter(content);
    expect(body).toBe("Actual body.");
  });
});

// ---------------------------------------------------------------------------
// assembleSystemPrompt
// ---------------------------------------------------------------------------

function makeSkill(id: string, body: string): ParsedSkill {
  return {
    id,
    path: `/fake/${id}/SKILL.md`,
    frontmatter: { name: id },
    body,
  };
}

describe("assembleSystemPrompt", () => {
  it("single skill body returned verbatim (whitespace-normalized)", () => {
    const result = assembleSystemPrompt({
      skills: [makeSkill("alpha", "  Alpha body.  ")],
    });
    expect(result).toBe("Alpha body.");
  });

  it("two skills concatenated with blank-line separator", () => {
    const result = assembleSystemPrompt({
      skills: [makeSkill("alpha", "Alpha."), makeSkill("beta", "Beta.")],
    });
    expect(result).toBe("Alpha.\n\nBeta.");
  });

  it("skills appear before agent body", () => {
    const result = assembleSystemPrompt({
      skills: [makeSkill("shared", "Shared.")],
      agentBody: "Agent-specific.",
    });
    expect(result).toBe("Shared.\n\nAgent-specific.");
  });

  it("empty agentBody is ignored", () => {
    const result = assembleSystemPrompt({
      skills: [makeSkill("alpha", "Alpha.")],
      agentBody: "",
    });
    expect(result).toBe("Alpha.");
  });

  it("whitespace-only agentBody is ignored", () => {
    const result = assembleSystemPrompt({
      skills: [makeSkill("alpha", "Alpha.")],
      agentBody: "   \n  ",
    });
    expect(result).toBe("Alpha.");
  });

  it("empty skills list with agent body returns agent body", () => {
    const result = assembleSystemPrompt({
      skills: [],
      agentBody: "Agent only.",
    });
    expect(result).toBe("Agent only.");
  });

  it("all empty produces empty string", () => {
    const result = assembleSystemPrompt({ skills: [] });
    expect(result).toBe("");
  });

  it("trailing whitespace in skill bodies is trimmed", () => {
    const result = assembleSystemPrompt({
      skills: [makeSkill("a", "Hello\n\n\n")],
    });
    expect(result).toBe("Hello");
  });
});

// ---------------------------------------------------------------------------
// createDirectSkillLoader — filesystem fixtures
// ---------------------------------------------------------------------------

let tmpRoot: string;
let skillsRoot: string;
let agentsRoot: string;

function writeSkill(id: string, frontmatterYaml: string, body: string): void {
  const dir = join(skillsRoot, id);
  mkdirSync(dir, { recursive: true });
  writeFileSync(
    join(dir, "SKILL.md"),
    `---\n${frontmatterYaml}\n---\n${body}`,
  );
}

function writeAgent(id: string, frontmatterYaml: string, body: string): void {
  mkdirSync(agentsRoot, { recursive: true });
  writeFileSync(
    join(agentsRoot, `${id}.md`),
    `---\n${frontmatterYaml}\n---\n${body}`,
  );
}

beforeEach(() => {
  tmpRoot = mkdtempSync(join(tmpdir(), "skills-loader-test-"));
  skillsRoot = join(tmpRoot, "skills");
  agentsRoot = join(tmpRoot, "agents");
  mkdirSync(skillsRoot, { recursive: true });
  mkdirSync(agentsRoot, { recursive: true });
});

afterEach(() => {
  rmSync(tmpRoot, { recursive: true, force: true });
});

describe("createDirectSkillLoader — loadSkill", () => {
  it("loads a skill with no extends", async () => {
    writeSkill(
      "standalone",
      "name: standalone\ndescription: A lone skill.",
      "Standalone body.",
    );
    const loader = createDirectSkillLoader({ skillsRoot, agentsRoot });
    const skill = await loader.loadSkill("standalone");
    expect(skill.id).toBe("standalone");
    expect(skill.frontmatter.name).toBe("standalone");
    expect(skill.frontmatter.description).toBe("A lone skill.");
    expect(skill.body.trim()).toBe("Standalone body.");
  });

  it("throws a helpful error for a missing skill", async () => {
    const loader = createDirectSkillLoader({ skillsRoot, agentsRoot });
    await expect(loader.loadSkill("nonexistent")).rejects.toThrow(
      /Skill "nonexistent" not found/,
    );
  });

  it("caches: second loadSkill call returns the same object", async () => {
    writeSkill("cached", "name: cached", "Cached body.");
    const loader = createDirectSkillLoader({ skillsRoot, agentsRoot });
    const first = await loader.loadSkill("cached");
    const second = await loader.loadSkill("cached");
    expect(first).toBe(second); // strict reference equality
  });

  it("clearCache allows re-reading an updated file", async () => {
    writeSkill("mutable", "name: mutable", "Version 1.");
    const loader = createDirectSkillLoader({ skillsRoot, agentsRoot });
    await loader.loadSkill("mutable");
    // Overwrite
    writeSkill("mutable", "name: mutable", "Version 2.");
    loader.clearCache();
    const fresh = await loader.loadSkill("mutable");
    expect(fresh.body.trim()).toBe("Version 2.");
  });
});

describe("createDirectSkillLoader — extends resolution", () => {
  it("loads a skill that extends one parent (parent body first)", async () => {
    writeSkill("parent", "name: parent", "Parent body.");
    writeSkill("child", "name: child\nextends:\n  - parent", "Child body.");
    const loader = createDirectSkillLoader({ skillsRoot, agentsRoot });
    const agent = await loader.loadAgent("child", "skill");
    expect(agent.systemPrompt).toBe("Parent body.\n\nChild body.");
  });

  it("loads a skill that extends two parents — order preserved", async () => {
    writeSkill("p1", "name: p1", "P1.");
    writeSkill("p2", "name: p2", "P2.");
    writeSkill("child", "name: child\nextends:\n  - p1\n  - p2", "Child.");
    const loader = createDirectSkillLoader({ skillsRoot, agentsRoot });
    const agent = await loader.loadAgent("child", "skill");
    expect(agent.systemPrompt).toBe("P1.\n\nP2.\n\nChild.");
  });

  it("deduplicates shared ancestors", async () => {
    writeSkill("base", "name: base", "Base.");
    writeSkill("mid", "name: mid\nextends:\n  - base", "Mid.");
    writeSkill(
      "top",
      "name: top\nextends:\n  - base\n  - mid",
      "Top.",
    );
    const loader = createDirectSkillLoader({ skillsRoot, agentsRoot });
    const agent = await loader.loadAgent("top", "skill");
    // base appears once (first), then mid, then top
    expect(agent.systemPrompt).toBe("Base.\n\nMid.\n\nTop.");
  });

  it("throws on a direct cycle (A extends B, B extends A)", async () => {
    writeSkill("cycle-a", "name: cycle-a\nextends:\n  - cycle-b", "A.");
    writeSkill("cycle-b", "name: cycle-b\nextends:\n  - cycle-a", "B.");
    const loader = createDirectSkillLoader({ skillsRoot, agentsRoot });
    await expect(loader.loadAgent("cycle-a", "skill")).rejects.toThrow(
      /Cycle detected/,
    );
  });

  it("throws with a helpful message for a missing referenced skill", async () => {
    writeSkill(
      "broken",
      "name: broken\nextends:\n  - missing-skill",
      "Body.",
    );
    const loader = createDirectSkillLoader({ skillsRoot, agentsRoot });
    await expect(loader.loadAgent("broken", "skill")).rejects.toThrow(
      /Skill "missing-skill" not found/,
    );
  });
});

describe("createDirectSkillLoader — loadAgent", () => {
  it("builds a full AgentDef from an agent manifest + extended skills", async () => {
    writeSkill("shared", "name: shared", "Shared governance prose.");
    writeAgent(
      "chief-analyst",
      [
        "name: chief-analyst",
        "description: The chief analyst.",
        "model: claude-opus-4-5",
        "max_tokens: 16384",
        "max_recursion_depth: 1",
        "tools: [wacc_calculator, dcf_model]",
        "extends:",
        "  - shared",
      ].join("\n"),
      "Agent-specific instructions.",
    );

    const loader = createDirectSkillLoader({ skillsRoot, agentsRoot });
    const def = await loader.loadAgent("chief-analyst");

    expect(def.id).toBe("chief-analyst");
    expect(def.description).toBe("The chief analyst.");
    expect(def.model).toBe("claude-opus-4-5");
    expect(def.maxTokens).toBe(16384);
    expect(def.maxRecursionDepth).toBe(1);
    expect(def.tools).toEqual(["wacc_calculator", "dcf_model"]);
    expect(def.systemPrompt).toBe(
      "Shared governance prose.\n\nAgent-specific instructions.",
    );
  });

  it("defaults tools to '*' when frontmatter tools is absent", async () => {
    writeAgent("no-tools", "name: no-tools", "Body.");
    const loader = createDirectSkillLoader({ skillsRoot, agentsRoot });
    const def = await loader.loadAgent("no-tools");
    expect(def.tools).toBe("*");
  });

  it("uses frontmatter tools: * when set to literal star", async () => {
    writeAgent("star-tools", "name: star-tools\ntools: '*'", "Body.");
    const loader = createDirectSkillLoader({ skillsRoot, agentsRoot });
    const def = await loader.loadAgent("star-tools");
    expect(def.tools).toBe("*");
  });

  it("falls back to file basename for id when frontmatter name is missing", async () => {
    writeAgent("fallback-id", "", "Body.");
    const loader = createDirectSkillLoader({ skillsRoot, agentsRoot });
    const def = await loader.loadAgent("fallback-id");
    expect(def.id).toBe("fallback-id");
  });

  it("throws when agent manifest file is missing", async () => {
    const loader = createDirectSkillLoader({ skillsRoot, agentsRoot });
    await expect(loader.loadAgent("ghost")).rejects.toThrow(
      /Agent manifest "ghost" not found/,
    );
  });
});
