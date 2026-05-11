/**
 * Direct filesystem skill loader — Phase 33 Wave 1.
 *
 * Reads <skillsRoot>/<id>/SKILL.md and <agentsRoot>/<id>.md (defaulting to
 * plugins/cfa-core/skills/cfa and plugins/cfa-core/agents/cfa), resolves
 * `extends:` references recursively (with cycle detection), and returns
 * fully-assembled AgentDef instances.
 *
 * This is the "direct" loader strategy (no subprocess). The "cli" loader
 * strategy (Wave 3) shells out to `cfa managed-agent deploy --json`.
 */

import { readFile } from "node:fs/promises";
import { existsSync } from "node:fs";
import { resolve, basename } from "node:path";
import type { AgentDef } from "../types.js";
import type {
  ParsedSkill,
  SkillFrontmatter,
  SkillLoader,
  SkillLoaderOptions,
} from "./types.js";
import { parseFrontmatter } from "./frontmatter-parser.js";
import { assembleSystemPrompt } from "./assembler.js";
import { createMultiRootSkillLoader } from "./multi-root-loader.js";

/**
 * Create a direct filesystem skill loader.
 *
 * The loader reads SKILL.md files from `opts.skillsRoot/<id>/SKILL.md`
 * and agent manifests from `opts.agentsRoot/<id>.md`. It caches parsed
 * skills in memory; call `clearCache()` between tests.
 *
 * When opts.skillsRoots or opts.agentsRoots are provided, the loader
 * delegates to createMultiRootSkillLoader (Phase 40 Wave 4). This
 * preserves full back-compat: callers that only supply skillsRoot/agentsRoot
 * get identical single-root behaviour.
 */
export function createDirectSkillLoader(opts: SkillLoaderOptions): SkillLoader {
  // Multi-root delegation (Phase 40 Wave 4)
  if (opts.skillsRoots !== undefined || opts.agentsRoots !== undefined) {
    return createMultiRootSkillLoader({
      skillsRoots: opts.skillsRoots ?? [opts.skillsRoot],
      agentsRoots: opts.agentsRoots ?? [opts.agentsRoot],
    });
  }
  const cache = new Map<string, ParsedSkill>();

  async function loadSkillUncached(id: string): Promise<ParsedSkill> {
    const path = resolve(opts.skillsRoot, id, "SKILL.md");
    if (!existsSync(path)) {
      throw new Error(
        `Skill "${id}" not found. Expected file at: ${path}`,
      );
    }
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

  /**
   * Resolve extends: references recursively with cycle detection.
   * Returns skills in the order they should appear in the assembled prompt
   * (deepest ancestor first, then siblings, then the skill itself).
   * Each skill id appears at most once (deduped by first occurrence).
   */
  async function resolveExtends(
    id: string,
    visiting: Set<string>,
    visited: Set<string>,
    acc: ParsedSkill[],
  ): Promise<void> {
    if (visited.has(id)) return; // already added
    if (visiting.has(id)) {
      throw new Error(
        `Cycle detected in skill extends: ${[...visiting, id].join(" → ")}`,
      );
    }

    visiting.add(id);
    const skill = await loadSkill(id);

    const extendsIds = skill.frontmatter.extends ?? [];
    for (const parentId of extendsIds) {
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
      filePath = resolve(opts.skillsRoot, agentId, "SKILL.md");
    } else {
      filePath = resolve(opts.agentsRoot, `${agentId}.md`);
    }

    if (!existsSync(filePath)) {
      throw new Error(
        `Agent manifest "${agentId}" not found. Expected file at: ${filePath}`,
      );
    }

    const content = await readFile(filePath, "utf-8");
    const { frontmatter, body: agentBody } = parseFrontmatter(content);
    const fm = frontmatter as SkillFrontmatter;

    // Resolve all extended skills
    const skills: ParsedSkill[] = [];
    const extendsIds = fm.extends ?? [];
    const visited = new Set<string>();

    for (const skillId of extendsIds) {
      await resolveExtends(skillId, new Set<string>(), visited, skills);
    }

    const systemPrompt = assembleSystemPrompt({ skills, agentBody });

    // Derive id: prefer frontmatter name, fall back to file basename (no ext)
    const rawName = fm.name;
    const id =
      typeof rawName === "string" && rawName.length > 0
        ? rawName
        : basename(filePath).replace(/\.md$/, "");

    // tools: string array or "*"
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
