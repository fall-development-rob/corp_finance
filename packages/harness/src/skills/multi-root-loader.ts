/**
 * Multi-root skill loader — Phase 40 Wave 4.
 *
 * createMultiRootSkillLoader walks multiple skillsRoots (in order) and
 * multiple agentsRoots (in order) to resolve skill and agent lookups,
 * returning the first match found. This enables the harness to discover
 * skills and agents from the 3-tier plugin structure:
 *   - plugins/agent-plugins/<slug>/skills/
 *   - plugins/vertical-plugins/<name>/skills/
 *   - plugins/partner-built/<name>/skills/
 *   - plugins/cfa-core/skills/cfa  (legacy fallback)
 *
 * The SkillLoader interface is unchanged — consumers that inject a loader
 * need not know whether it is single- or multi-root.
 */

import { readFile } from "node:fs/promises";
import { existsSync } from "node:fs";
import { resolve, basename } from "node:path";
import type { AgentDef } from "../types.js";
import type {
  ParsedSkill,
  SkillFrontmatter,
  SkillLoader,
} from "./types.js";
import { parseFrontmatter } from "./frontmatter-parser.js";
import { assembleSystemPrompt } from "./assembler.js";

// ---------------------------------------------------------------------------
// Options
// ---------------------------------------------------------------------------

export interface MultiRootSkillLoaderOptions {
  /** Ordered list of roots to search for skill dirs (<root>/<id>/SKILL.md). */
  skillsRoots: string[];
  /** Ordered list of roots to search for agent manifests (<root>/<id>.md). */
  agentsRoots: string[];
}

// ---------------------------------------------------------------------------
// Factory
// ---------------------------------------------------------------------------

/**
 * Create a multi-root skill loader that walks all provided roots in order,
 * returning the first match. Throws with all attempted paths listed when no
 * root contains the requested skill or agent.
 *
 * Caches parsed skills in memory; call clearCache() between tests.
 */
export function createMultiRootSkillLoader(
  opts: MultiRootSkillLoaderOptions,
): SkillLoader {
  const cache = new Map<string, ParsedSkill>();

  // -------------------------------------------------------------------------
  // Path resolution helpers
  // -------------------------------------------------------------------------

  function resolveSkillPath(id: string): string {
    const attempted: string[] = [];
    for (const root of opts.skillsRoots) {
      const p = resolve(root, id, "SKILL.md");
      if (existsSync(p)) return p;
      attempted.push(p);
    }
    throw new Error(
      `Skill "${id}" not found in any skills root.\nAttempted paths:\n  ${attempted.join("\n  ")}`,
    );
  }

  function resolveAgentPath(agentId: string): string {
    const attempted: string[] = [];
    for (const root of opts.agentsRoots) {
      const p = resolve(root, `${agentId}.md`);
      if (existsSync(p)) return p;
      attempted.push(p);
    }
    throw new Error(
      `Agent manifest "${agentId}" not found in any agents root.\nAttempted paths:\n  ${attempted.join("\n  ")}`,
    );
  }

  // -------------------------------------------------------------------------
  // Core loading logic (mirrors createDirectSkillLoader)
  // -------------------------------------------------------------------------

  async function loadSkillUncached(id: string): Promise<ParsedSkill> {
    const path = resolveSkillPath(id);
    const content = await readFile(path, "utf-8");
    const { frontmatter, body } = parseFrontmatter(content);
    return {
      id,
      path,
      frontmatter: frontmatter as SkillFrontmatter,
      body,
    };
  }

  async function loadSkill(id: string): Promise<ParsedSkill> {
    const cached = cache.get(id);
    if (cached !== undefined) return cached;
    const skill = await loadSkillUncached(id);
    cache.set(id, skill);
    return skill;
  }

  async function resolveExtends(
    id: string,
    visiting: Set<string>,
    visited: Set<string>,
    acc: ParsedSkill[],
  ): Promise<void> {
    if (visited.has(id)) return;
    if (visiting.has(id)) {
      throw new Error(
        `Cycle detected in skill extends: ${[...visiting, id].join(" → ")}`,
      );
    }

    visiting.add(id);
    const skill = await loadSkill(id);

    for (const parentId of skill.frontmatter.extends ?? []) {
      await resolveExtends(parentId, visiting, visited, acc);
    }

    visiting.delete(id);
    visited.add(id);
    acc.push(skill);
  }

  async function loadAgent(
    agentId: string,
    kind: "agent" | "skill" = "agent",
  ): Promise<AgentDef> {
    let filePath: string;
    if (kind === "skill") {
      filePath = resolveSkillPath(agentId);
    } else {
      filePath = resolveAgentPath(agentId);
    }

    const content = await readFile(filePath, "utf-8");
    const { frontmatter, body: agentBody } = parseFrontmatter(content);
    const fm = frontmatter as SkillFrontmatter;

    const skills: ParsedSkill[] = [];
    const visited = new Set<string>();
    for (const skillId of fm.extends ?? []) {
      await resolveExtends(skillId, new Set<string>(), visited, skills);
    }

    const systemPrompt = assembleSystemPrompt({ skills, agentBody });

    const rawName = fm.name;
    const id =
      typeof rawName === "string" && rawName.length > 0
        ? rawName
        : basename(filePath).replace(/\.md$/, "");

    let tools: AgentDef["tools"] = "*";
    const rawTools = fm.tools;
    if (Array.isArray(rawTools)) {
      tools = rawTools as string[];
    } else if (rawTools === "*") {
      tools = "*";
    }

    return {
      id,
      description: typeof fm.description === "string" ? fm.description : "",
      systemPrompt,
      tools,
      model: typeof fm.model === "string" ? fm.model : undefined,
      maxTokens:
        typeof fm.max_tokens === "number" ? fm.max_tokens : undefined,
      maxRecursionDepth:
        typeof fm.max_recursion_depth === "number"
          ? fm.max_recursion_depth
          : undefined,
    };
  }

  function clearCache(): void {
    cache.clear();
  }

  return { loadSkill, loadAgent, clearCache };
}
