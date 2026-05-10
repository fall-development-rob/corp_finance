/**
 * Direct YAML manifest loader — Phase 36.
 *
 * Reads <agentsRoot>/<id>.yaml, parses via the `yaml` library (strict mode),
 * resolves from_plugin and from_skill skill references, assembles the system
 * prompt, and projects toolset blocks into AgentDef.tools.
 *
 * from_plugin (primary) — path relative to the manifest file; reads SKILL.md
 * from_skill  (legacy)  — slug resolved via the injected SkillLoader
 */

import { readFile } from "node:fs/promises";
import { existsSync } from "node:fs";
import { resolve, dirname, basename } from "node:path";
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

export interface YamlManifestLoaderOptions {
  /** Absolute path to the agents directory, e.g. "<repo>/plugins/cfa-core/agents/cfa" */
  agentsRoot: string;
  /** For resolving skills[*].from_skill (legacy slug form). */
  skillLoader: SkillLoader;
}

export interface YamlManifestLoader {
  loadAgent(agentId: string): Promise<AgentDef>;
  clearCache(): void;
}

// ---------------------------------------------------------------------------
// Prefix stripping
// ---------------------------------------------------------------------------

/** Strip mcp__<server>__ prefix from a tool name, returning the bare name. */
function stripMcpPrefix(name: string): string {
  // matches: mcp__<anything>__<toolname>
  const m = /^mcp__[^_][^_]*__(.+)$/.exec(name);
  return m ? (m[1] ?? name) : name;
}

// ---------------------------------------------------------------------------
// Tool projection
// ---------------------------------------------------------------------------

/**
 * Project toolset blocks into a flat list of bare tool names.
 * Returns the tool list and the raw toolset configs for runtime denylist use.
 */
function projectTools(
  toolsets: ToolsetConfig[] | undefined,
): { tools: AgentDef["tools"]; toolsetsRaw?: ToolsetConfig[] } {
  if (!toolsets || toolsets.length === 0) {
    return { tools: "*" };
  }

  const collected = new Set<string>();
  let hasWildcard = false;
  let preserveRaw = false;

  for (const block of toolsets) {
    if (block.type === "tools_array") {
      for (const t of block.tools) {
        collected.add(stripMcpPrefix(t));
      }
      continue;
    }

    if (
      block.type === "agent_toolset" ||
      block.type === "agent_toolset_20260401"
    ) {
      const defaultEnabled = block.default_config?.enabled ?? true;
      const overrides = new Map<string, boolean>();
      for (const cfg of block.configs ?? []) {
        overrides.set(stripMcpPrefix(cfg.name), cfg.enabled);
      }
      if (defaultEnabled) {
        // wildcard for built-in toolset; preserve raw for runtime filtering
        hasWildcard = true;
        preserveRaw = true;
        for (const [k, v] of overrides) {
          if (v) collected.add(k);
        }
      } else {
        for (const [k, v] of overrides) {
          if (v) collected.add(k);
        }
      }
      continue;
    }

    if (block.type === "mcp_toolset") {
      const defaultEnabled = block.default_config?.enabled ?? true;
      const overrides = new Map<string, boolean>();
      for (const cfg of block.configs ?? []) {
        overrides.set(stripMcpPrefix(cfg.name), cfg.enabled);
      }
      if (defaultEnabled) {
        // denylist semantics — need live catalog at dispatch time
        hasWildcard = true;
        preserveRaw = true;
        // explicit enables from configs are also collected
        for (const [k, v] of overrides) {
          if (v) collected.add(k);
        }
      } else {
        // allowlist semantics — only named+enabled tools
        for (const [k, v] of overrides) {
          if (v) collected.add(k);
        }
      }
    }
  }

  // When any block is default-enabled (wildcard semantics), return "*" and
  // preserve raw toolsets for runtime denylist filtering at dispatch time.
  if (hasWildcard) {
    return { tools: "*", toolsetsRaw: preserveRaw ? toolsets : undefined };
  }

  const toolList = [...collected].sort();
  return {
    tools: toolList.length > 0 ? toolList : "*",
  };
}

// ---------------------------------------------------------------------------
// Factory
// ---------------------------------------------------------------------------

export function createDirectYamlManifestLoader(
  opts: YamlManifestLoaderOptions,
): YamlManifestLoader {
  const cache = new Map<string, AgentDef>();

  async function loadAgentByPath(
    manifestPath: string,
    visiting: Set<string>,
  ): Promise<AgentDef> {
    if (cache.has(manifestPath)) return cache.get(manifestPath)!;

    if (!existsSync(manifestPath)) {
      const id = basename(manifestPath, ".yaml");
      throw new Error(
        `Agent manifest "${id}" not found. Expected file at: ${manifestPath}`,
      );
    }

    let raw: string;
    try {
      raw = await readFile(manifestPath, "utf-8");
    } catch (err) {
      throw new Error(
        `Failed to read manifest at ${manifestPath}: ${(err as Error).message}`,
      );
    }

    let parsed: unknown;
    try {
      parsed = parseYaml(raw, { strict: true });
    } catch (err) {
      throw new Error(
        `Failed to parse manifest at ${manifestPath}: ${(err as Error).message}`,
      );
    }

    if (
      typeof parsed !== "object" ||
      parsed === null ||
      !("name" in parsed) ||
      typeof (parsed as Record<string, unknown>).name !== "string"
    ) {
      throw new Error(
        `Manifest at ${manifestPath} is missing required field "name"`,
      );
    }

    const manifest = parsed as AgentManifest;
    const manifestDir = dirname(manifestPath);

    // --- Skill bodies ---
    const skillBodies: string[] = [];
    for (const ref of manifest.skills ?? []) {
      const body = await resolveSkillRef(ref, manifestDir);
      if (body.trim().length > 0) skillBodies.push(body.trim());
    }

    // --- system.file ---
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

    // --- Assemble system prompt ---
    const sections: string[] = [...skillBodies];
    if (systemFileBody.trim().length > 0) {
      sections.push(systemFileBody.trim());
    }
    if (manifest.system?.text && manifest.system.text.trim().length > 0) {
      sections.push(manifest.system.text.trim());
    }
    if (manifest.system?.append && manifest.system.append.trim().length > 0) {
      sections.push(manifest.system.append.trim());
    }
    const systemPrompt = sections.join("\n\n");

    // --- Tools ---
    const { tools, toolsetsRaw } = projectTools(manifest.tools);

    // --- Callable agents (recursive) ---
    const callableAgents: AgentDef[] = [];
    if (manifest.callable_agents && manifest.callable_agents.length > 0) {
      if (visiting.has(manifestPath)) {
        throw new Error(
          `Cycle detected in callable_agents: ${[...visiting, manifestPath].join(" → ")}`,
        );
      }
      visiting.add(manifestPath);
      for (const ref of manifest.callable_agents) {
        const subPath = resolve(manifestDir, ref.manifest);
        if (visiting.has(subPath)) {
          throw new Error(
            `Cycle detected in callable_agents: ${[...visiting, subPath].join(" → ")}`,
          );
        }
        const subDef = await loadAgentByPath(subPath, new Set(visiting));
        callableAgents.push(subDef);
      }
      visiting.delete(manifestPath);
    }

    const def: AgentDef = {
      id: manifest.name,
      description:
        typeof manifest.description === "string" ? manifest.description : "",
      systemPrompt,
      tools,
      model: typeof manifest.model === "string" ? manifest.model : undefined,
      maxTokens:
        typeof manifest.max_tokens === "number"
          ? manifest.max_tokens
          : undefined,
      maxRecursionDepth:
        typeof manifest.max_recursion_depth === "number"
          ? manifest.max_recursion_depth
          : undefined,
    };

    // Additive optional fields
    if (callableAgents.length > 0) {
      (def as AgentDef & { callableAgents?: AgentDef[] }).callableAgents =
        callableAgents;
    }
    if (toolsetsRaw) {
      (def as AgentDef & { toolsetsRaw?: ToolsetConfig[] }).toolsetsRaw =
        toolsetsRaw;
    }
    if (manifest.mcp_servers && manifest.mcp_servers.length > 0) {
      (def as AgentDef & { mcpServerRefs?: McpServerRef[] }).mcpServerRefs =
        manifest.mcp_servers;
    }
    if (manifest.output_schema) {
      (
        def as AgentDef & { outputSchema?: Record<string, unknown> }
      ).outputSchema = manifest.output_schema;
    }
    if (manifest.input_schema) {
      (
        def as AgentDef & { inputSchema?: Record<string, unknown> }
      ).inputSchema = manifest.input_schema;
    }

    cache.set(manifestPath, def);
    return def;
  }

  async function resolveSkillRef(
    ref: SkillRef,
    manifestDir: string,
  ): Promise<string> {
    if (ref.from_plugin !== undefined) {
      // PRIMARY: resolve path relative to manifest directory
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
      // LEGACY: delegate to the injected skill loader
      const skill = await opts.skillLoader.loadSkill(ref.from_skill);
      return skill.body;
    }

    throw new Error(
      `SkillRef must have either from_plugin or from_skill: ${JSON.stringify(ref)}`,
    );
  }

  return {
    async loadAgent(agentId: string): Promise<AgentDef> {
      const manifestPath = resolve(opts.agentsRoot, `${agentId}.yaml`);
      return loadAgentByPath(manifestPath, new Set());
    },

    clearCache(): void {
      cache.clear();
    },
  };
}
