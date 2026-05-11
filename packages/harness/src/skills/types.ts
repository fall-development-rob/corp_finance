/**
 * Public types for the Phase 33 skill loader infrastructure.
 *
 * These interfaces describe how SKILL.md and agent .md files are parsed,
 * and what the SkillLoader factory returns. The AgentDef interface
 * (src/types.ts) is the consumer-facing type; this module bridges
 * the authoring surface (markdown + YAML frontmatter) to that interface.
 */

/**
 * YAML frontmatter on a SKILL.md or agent .md file.
 *
 * Inputs come from anything in the standard Claude Code skills/agents
 * format (canonical home: plugins/cfa-core/skills/ and
 * plugins/cfa-core/agents/) used by Claude Code and the cfa managed-agent
 * CLI. Fields are optional except `name`.
 */
export interface SkillFrontmatter {
  name: string;
  description?: string;
  /** Tags / triggers — informational only at load time. */
  tags?: string[];
  /**
   * Other skill ids (basenames of directories under skills root) whose
   * bodies should be concatenated before this skill's body. Order is
   * preserved; each reference is loaded recursively (deduped).
   */
  extends?: string[];
  /** Tool allowlist — array of bare names, or "*" for all tools. */
  tools?: string[] | "*";
  /** Model override. Default decided by the harness if absent. */
  model?: string;
  /** Max output tokens. Default decided by the harness if absent. */
  max_tokens?: number;
  /** Recursion depth bound — only meaningful on agent manifests. */
  max_recursion_depth?: number;
  /** Pass-through for any other fields the loader doesn't interpret. */
  [key: string]: unknown;
}

export interface ParsedSkill {
  id: string;
  path: string;        // absolute path to the SKILL.md
  frontmatter: SkillFrontmatter;
  body: string;        // markdown body without the frontmatter block
}

export interface SkillLoaderOptions {
  skillsRoot: string;  // e.g., "<repo>/plugins/cfa-core/skills/cfa"
  agentsRoot: string;  // e.g., "<repo>/plugins/cfa-core/agents/cfa"
  /**
   * Optional multi-root override. When provided, the loader walks all roots
   * in order and returns the first match. Takes precedence over skillsRoot.
   * Added in Phase 40 Wave 4 for 3-tier plugin discovery.
   */
  skillsRoots?: string[];
  /**
   * Optional multi-root override. When provided, the loader walks all roots
   * in order and returns the first match. Takes precedence over agentsRoot.
   * Added in Phase 40 Wave 4 for 3-tier plugin discovery.
   */
  agentsRoots?: string[];
}

export interface SkillLoader {
  /** Load a single skill by id (basename of its directory under skillsRoot). */
  loadSkill(id: string): Promise<ParsedSkill>;
  /**
   * Load an agent manifest, resolve its skill references, and assemble
   * a complete AgentDef ready for harness dispatch. The manifest may be
   * either an agent .md file OR a skill .md file with `extends:` entries.
   *
   * agentId is the basename of the agent manifest under agentsRoot, OR
   * a skill id under skillsRoot if `kind` is "skill".
   */
  loadAgent(agentId: string, kind?: "agent" | "skill"): Promise<import("../types.js").AgentDef>;
  /** Clear in-memory cache. */
  clearCache(): void;
}
