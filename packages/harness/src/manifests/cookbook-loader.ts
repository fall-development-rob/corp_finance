/**
 * Cookbook deployment loader — Phase 33 smoke gate.
 *
 * Loads managed-agent-cookbook agent.yaml + subagents/*.yaml into AgentDef,
 * using the same projection logic as the YAML manifest loader (yaml-loader.ts).
 * yaml.parse() accepts both YAML and JSON 1.2, so legacy .json cookbooks
 * (if any remain) continue to load without errors.
 *
 * The projection is a faithful port of loadAgentByPath from yaml-loader.ts;
 * both implementations share the same AgentManifest → AgentDef mapping rules.
 */

import { readFile, readdir } from "node:fs/promises";
import { existsSync } from "node:fs";
import { resolve, dirname } from "node:path";
import { parse as parseYaml } from "yaml";
import type { AgentDef } from "../types.js";
import type { SkillLoader } from "../skills/types.js";
import { parseFrontmatter } from "../skills/frontmatter-parser.js";
import type {
  AgentManifest,
  McpServerRef,
  SkillRef,
  ToolsetConfig,
} from "./types.js";

// ---------------------------------------------------------------------------
// Public surface
// ---------------------------------------------------------------------------

export interface CookbookLoaderOptions {
  /** Absolute path to managed-agent-cookbooks directory. */
  cookbooksRoot: string;
  /** For resolving skills[*].from_plugin / from_skill. */
  skillLoader: SkillLoader;
}

export interface LoadedCookbook {
  slug: string;
  parent: AgentDef;
  subagents: AgentDef[];
  /** Non-fatal issues found during load (e.g., missing optional fields). */
  warnings: string[];
}

export interface CookbookLoader {
  /** Return every immediate subdirectory of cookbooksRoot that contains agent.yaml (or legacy agent.json). */
  list(): Promise<string[]>;
  /** Load one cookbook by slug: parent agent.yaml + all subagents/*.yaml → AgentDef[]. */
  load(slug: string): Promise<LoadedCookbook>;
  /** Load every cookbook under cookbooksRoot in parallel. */
  loadAll(): Promise<LoadedCookbook[]>;
}

// ---------------------------------------------------------------------------
// Tool projection (ported from yaml-loader.ts, keep aligned)
// ---------------------------------------------------------------------------

function stripMcpPrefix(name: string): string {
  const m = /^mcp__[^_][^_]*__(.+)$/.exec(name);
  return m ? (m[1] ?? name) : name;
}

function projectTools(
  toolsets: ToolsetConfig[] | undefined,
): { tools: AgentDef["tools"]; toolsetsRaw?: ToolsetConfig[] } {
  if (!toolsets || toolsets.length === 0) return { tools: "*" };

  const collected = new Set<string>();
  let hasWildcard = false;
  let preserveRaw = false;

  for (const block of toolsets) {
    if (block.type === "tools_array") {
      for (const t of block.tools) collected.add(stripMcpPrefix(t));
      continue;
    }
    if (
      block.type === "agent_toolset" ||
      block.type === "agent_toolset_20260401"
    ) {
      const defaultEnabled = block.default_config?.enabled ?? true;
      const overrides = new Map<string, boolean>();
      for (const cfg of block.configs ?? [])
        overrides.set(stripMcpPrefix(cfg.name), cfg.enabled);
      if (defaultEnabled) {
        hasWildcard = true;
        preserveRaw = true;
        for (const [k, v] of overrides) if (v) collected.add(k);
      } else {
        for (const [k, v] of overrides) if (v) collected.add(k);
      }
      continue;
    }
    if (block.type === "mcp_toolset") {
      const defaultEnabled = block.default_config?.enabled ?? true;
      const overrides = new Map<string, boolean>();
      for (const cfg of block.configs ?? [])
        overrides.set(stripMcpPrefix(cfg.name), cfg.enabled);
      if (defaultEnabled) {
        hasWildcard = true;
        preserveRaw = true;
        for (const [k, v] of overrides) if (v) collected.add(k);
      } else {
        for (const [k, v] of overrides) if (v) collected.add(k);
      }
    }
  }

  if (hasWildcard) {
    return { tools: "*", toolsetsRaw: preserveRaw ? toolsets : undefined };
  }
  const toolList = [...collected].sort();
  return { tools: toolList.length > 0 ? toolList : "*" };
}

// ---------------------------------------------------------------------------
// Skill resolution (ported from yaml-loader.ts resolveSkillRef)
// ---------------------------------------------------------------------------

async function resolveSkillRef(
  ref: SkillRef,
  manifestDir: string,
  skillLoader: SkillLoader,
): Promise<string> {
  if (ref.from_plugin !== undefined) {
    const pluginDir = resolve(manifestDir, ref.from_plugin);
    const skillMdPath = resolve(pluginDir, "SKILL.md");
    if (!existsSync(skillMdPath)) {
      throw new Error(
        `from_plugin skill not found. Expected SKILL.md at: ${skillMdPath}`,
      );
    }
    const content = await readFile(skillMdPath, "utf-8");
    const { body } = parseFrontmatter(content);
    return body;
  }
  if (ref.from_skill !== undefined) {
    const skill = await skillLoader.loadSkill(ref.from_skill);
    return skill.body;
  }
  throw new Error(
    `SkillRef must have either from_plugin or from_skill: ${JSON.stringify(ref)}`,
  );
}

// ---------------------------------------------------------------------------
// Single-manifest projection
// ---------------------------------------------------------------------------

async function projectManifest(
  manifestPath: string,
  manifest: AgentManifest,
  skillLoader: SkillLoader,
  visiting: Set<string>,
): Promise<AgentDef> {
  const manifestDir = dirname(manifestPath);

  // Skill bodies
  const skillBodies: string[] = [];
  for (const ref of manifest.skills ?? []) {
    const body = await resolveSkillRef(ref, manifestDir, skillLoader);
    if (body.trim().length > 0) skillBodies.push(body.trim());
  }

  // system.file
  let systemFileBody = "";
  if (manifest.system?.file) {
    const filePath = resolve(manifestDir, manifest.system.file);
    if (!existsSync(filePath)) {
      throw new Error(
        `system.file "${manifest.system.file}" not found at: ${filePath}`,
      );
    }
    systemFileBody = await readFile(filePath, "utf-8");
  }

  // Assemble system prompt
  const sections: string[] = [...skillBodies];
  if (systemFileBody.trim().length > 0) sections.push(systemFileBody.trim());
  if (manifest.system?.text?.trim()) sections.push(manifest.system.text.trim());
  if (manifest.system?.append?.trim()) sections.push(manifest.system.append.trim());
  const systemPrompt = sections.join("\n\n");

  // Tools
  const { tools, toolsetsRaw } = projectTools(manifest.tools);

  // Callable agents (cycle-safe recursive via visiting set)
  const callableAgents: AgentDef[] = [];
  if (manifest.callable_agents && manifest.callable_agents.length > 0) {
    if (visiting.has(manifestPath)) {
      throw new Error(
        `Cycle detected in callable_agents: ${[...visiting, manifestPath].join(" → ")}`,
      );
    }
    const next = new Set(visiting);
    next.add(manifestPath);
    for (const ref of manifest.callable_agents) {
      const subPath = resolve(manifestDir, ref.manifest);
      if (next.has(subPath)) {
        throw new Error(
          `Cycle detected in callable_agents: ${[...next, subPath].join(" → ")}`,
        );
      }
      const subManifest = await readManifestYaml(subPath);
      const subDef = await projectManifest(subPath, subManifest, skillLoader, new Set(next));
      callableAgents.push(subDef);
    }
  }

  const def: AgentDef = {
    id: manifest.name,
    description: typeof manifest.description === "string" ? manifest.description : "",
    systemPrompt,
    tools,
    model: typeof manifest.model === "string" ? manifest.model : undefined,
    maxTokens: typeof manifest.max_tokens === "number" ? manifest.max_tokens : undefined,
    maxRecursionDepth:
      typeof manifest.max_recursion_depth === "number"
        ? manifest.max_recursion_depth
        : undefined,
  };

  if (callableAgents.length > 0) def.callableAgents = callableAgents;
  if (toolsetsRaw) def.toolsetsRaw = toolsetsRaw;
  if (manifest.mcp_servers && manifest.mcp_servers.length > 0) {
    (def as AgentDef & { mcpServerRefs?: McpServerRef[] }).mcpServerRefs =
      manifest.mcp_servers;
  }
  if (manifest.output_schema) def.outputSchema = manifest.output_schema;
  if (manifest.input_schema) def.inputSchema = manifest.input_schema;
  if (manifest.block_tools && manifest.block_tools.length > 0)
    def.blockTools = manifest.block_tools;

  return def;
}

// ---------------------------------------------------------------------------
// YAML/JSON reader with descriptive errors
// yaml.parse() accepts both YAML and JSON 1.2.
// ---------------------------------------------------------------------------

async function readManifestYaml(filePath: string): Promise<AgentManifest> {
  if (!existsSync(filePath)) {
    throw new Error(`Cookbook manifest not found at: ${filePath}`);
  }
  let raw: string;
  try {
    raw = await readFile(filePath, "utf-8");
  } catch (err) {
    throw new Error(`Failed to read ${filePath}: ${(err as Error).message}`);
  }
  let parsed: unknown;
  try {
    parsed = parseYaml(raw);
  } catch (err) {
    throw new Error(`Failed to parse YAML at ${filePath}: ${(err as Error).message}`);
  }
  if (
    typeof parsed !== "object" ||
    parsed === null ||
    !("name" in parsed) ||
    typeof (parsed as Record<string, unknown>).name !== "string"
  ) {
    throw new Error(`Manifest at ${filePath} is missing required field "name"`);
  }
  return parsed as AgentManifest;
}

// ---------------------------------------------------------------------------
// Factory
// ---------------------------------------------------------------------------

export function createCookbookLoader(opts: CookbookLoaderOptions): CookbookLoader {
  const { cookbooksRoot, skillLoader } = opts;

  async function list(): Promise<string[]> {
    let entries: string[];
    try {
      entries = await readdir(cookbooksRoot);
    } catch (err) {
      throw new Error(
        `Failed to read cookbooks root at ${cookbooksRoot}: ${(err as Error).message}`,
      );
    }
    const slugs: string[] = [];
    for (const entry of entries) {
      // Prefer .yaml; fall back to legacy .json
      const agentYamlPath = resolve(cookbooksRoot, entry, "agent.yaml");
      const agentJsonPath = resolve(cookbooksRoot, entry, "agent.json");
      if (existsSync(agentYamlPath) || existsSync(agentJsonPath)) slugs.push(entry);
    }
    return slugs.sort();
  }

  function resolveAgentManifestPath(dir: string): string {
    const yamlPath = resolve(dir, "agent.yaml");
    if (existsSync(yamlPath)) return yamlPath;
    const jsonPath = resolve(dir, "agent.json");
    if (existsSync(jsonPath)) return jsonPath;
    return yamlPath; // let the reader produce the descriptive error
  }

  async function load(slug: string): Promise<LoadedCookbook> {
    const cookbookDir = resolve(cookbooksRoot, slug);
    const agentManifestPath = resolveAgentManifestPath(cookbookDir);

    if (!existsSync(agentManifestPath)) {
      throw new Error(
        `Cookbook "${slug}" has no agent.yaml (or agent.json). Expected: ${agentManifestPath}`,
      );
    }

    const parentManifest = await readManifestYaml(agentManifestPath);
    const warnings: string[] = [];

    const parent = await projectManifest(
      agentManifestPath,
      parentManifest,
      skillLoader,
      new Set(),
    );

    // Load subagents/*.yaml (also accepts legacy .json and .yml)
    const subagentsDir = resolve(cookbookDir, "subagents");
    const subagents: AgentDef[] = [];
    if (existsSync(subagentsDir)) {
      let subFiles: string[];
      try {
        subFiles = await readdir(subagentsDir);
      } catch {
        subFiles = [];
        warnings.push(`Could not read subagents directory: ${subagentsDir}`);
      }
      const manifestFiles = subFiles
        .filter((f) => f.endsWith(".yaml") || f.endsWith(".yml") || f.endsWith(".json"))
        .sort();
      for (const filename of manifestFiles) {
        const subPath = resolve(subagentsDir, filename);
        const subManifest = await readManifestYaml(subPath);
        const subDef = await projectManifest(subPath, subManifest, skillLoader, new Set());
        subagents.push(subDef);
      }
    }

    return { slug, parent, subagents, warnings };
  }

  async function loadAll(): Promise<LoadedCookbook[]> {
    const slugs = await list();
    return Promise.all(slugs.map((s) => load(s)));
  }

  return { list, load, loadAll };
}
