/**
 * Skill-backed agent registry — Phase 33 Wave 3.
 *
 * Provides a runtime substrate that constructs the full 9-agent registry
 * purely by loading SKILL.md + agent manifest files from
 * plugins/cfa-core/skills/cfa/ and plugins/cfa-core/agents/cfa/. Parallel
 * to the existing TS-imported `registry`
 * Map in registry.ts; Wave 4 will switch consumers over and delete the TS
 * specialist source files.
 *
 * Strict-startup pattern: createSkillRegistry rejects if any of the 9
 * manifests is missing or fails to parse — loud failure over silent runtime
 * divergence.
 */

import { existsSync, readdirSync, statSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import type { AgentDef } from "../types.js";
import { createDirectSkillLoader } from "../skills/index.js";
import type { SkillLoader } from "../skills/types.js";
import {
  createDirectYamlManifestLoader,
  type YamlManifestLoader,
} from "../manifests/yaml-loader.js";

// ---------------------------------------------------------------------------
// Public surface
// ---------------------------------------------------------------------------

export interface SkillRegistry {
  /** Returns the chief-analyst AgentDef. */
  chief(): AgentDef;
  /** Returns the 8 specialist AgentDefs in canonical delegation order. */
  delegates(): AgentDef[];
  /** Returns one AgentDef by id, or throws if not loaded. */
  get(id: string): AgentDef;
  /** Returns all 9 (chief + delegates). */
  all(): AgentDef[];
}

export interface SkillRegistryOptions {
  /**
   * Single skills root (legacy, single-root). When neither skillsRoot nor
   * skillsRoots is provided, the registry performs multi-root discovery
   * across all 3 plugin tiers automatically.
   */
  skillsRoot?: string;
  /**
   * Single agents root (legacy, single-root). When neither agentsRoot nor
   * agentsRoots is provided, the registry performs multi-root discovery
   * across all 3 plugin tiers automatically.
   */
  agentsRoot?: string;
  /**
   * Explicit multi-root skills list. Overrides automatic tier discovery.
   * Added Phase 40 Wave 4.
   */
  skillsRoots?: string[];
  /**
   * Explicit multi-root agents list. Overrides automatic tier discovery.
   * Added Phase 40 Wave 4.
   */
  agentsRoots?: string[];
  /** Injectable for testing. */
  loader?: SkillLoader;
  /** Injectable YAML manifest loader for testing. */
  yamlLoader?: YamlManifestLoader;
}

/**
 * Canonical agent ids: chief first, then the 8 specialists in the same order
 * as `defaultDelegates` in registry.ts.
 */
export const SKILL_REGISTRY_AGENT_IDS = [
  "chief-analyst",
  "equity-analyst",
  "credit-analyst",
  "fixed-income-analyst",
  "derivatives-analyst",
  "quant-risk-analyst",
  "macro-analyst",
  "private-markets-analyst",
  "esg-regulatory-analyst",
] as const;

// ---------------------------------------------------------------------------
// Repo-root detection (mirrors registry.ts; intentionally duplicated, not
// imported, to keep this module independent of registry.ts internals).
// ---------------------------------------------------------------------------

function findRepoRoot(startDir: string): string {
  let dir = startDir;
  // Walk up until we find a directory that contains plugins/cfa-core/
  for (let i = 0; i < 20; i++) {
    if (existsSync(resolve(dir, "plugins", "cfa-core"))) {
      return dir;
    }
    const parent = resolve(dir, "..");
    if (parent === dir) break; // filesystem root
    dir = parent;
  }
  // Fallback: assume four levels up from packages/harness/src/agents/
  return resolve(startDir, "..", "..", "..", "..", "..");
}

const _thisDir = dirname(fileURLToPath(import.meta.url));
const _repoRoot = findRepoRoot(_thisDir);

// ---------------------------------------------------------------------------
// Multi-root discovery (Phase 40 Wave 4)
// ---------------------------------------------------------------------------

const PLUGIN_TIERS = ["agent-plugins", "vertical-plugins", "partner-built"] as const;

/**
 * Walk the 3 plugin tiers under <repoRoot>/plugins/ and collect all
 * skills/ and agents/ subdirectories that exist. Appends the legacy
 * cfa-core paths as fallback roots at the end.
 */
export function discoverPluginRoots(repoRoot: string): {
  skillsRoots: string[];
  agentsRoots: string[];
} {
  const skillsRoots: string[] = [];
  const agentsRoots: string[] = [];

  for (const tier of PLUGIN_TIERS) {
    const tierDir = resolve(repoRoot, "plugins", tier);
    if (!existsSync(tierDir)) continue;
    let entries: string[];
    try {
      entries = readdirSync(tierDir);
    } catch {
      continue;
    }
    for (const plugin of entries) {
      const pluginDir = resolve(tierDir, plugin);
      try {
        if (!statSync(pluginDir).isDirectory()) continue;
      } catch {
        continue;
      }
      const skillsDir = resolve(pluginDir, "skills");
      if (existsSync(skillsDir)) skillsRoots.push(skillsDir);
      const agentsDir = resolve(pluginDir, "agents");
      if (existsSync(agentsDir)) agentsRoots.push(agentsDir);
    }
  }

  // Legacy fallback — W5 will delete these once cfa-core is verified empty
  const legacySkills = resolve(repoRoot, "plugins", "cfa-core", "skills");
  const legacyAgents = resolve(repoRoot, "plugins", "cfa-core", "agents", "cfa");
  if (existsSync(legacySkills)) skillsRoots.push(legacySkills);
  if (existsSync(legacyAgents)) agentsRoots.push(legacyAgents);

  return { skillsRoots, agentsRoots };
}

// ---------------------------------------------------------------------------
// Factory
// ---------------------------------------------------------------------------

/**
 * Async factory: loads all 9 agents from the skill files and returns a
 * fully-resolved registry. Throws if any of the 9 manifests is missing or
 * fails to parse.
 */
export async function createSkillRegistry(
  options?: SkillRegistryOptions,
): Promise<SkillRegistry> {
  // -------------------------------------------------------------------------
  // Resolve roots: explicit > single-root legacy > multi-root tier discovery
  // -------------------------------------------------------------------------
  let resolvedSkillsRoots: string[];
  let resolvedAgentsRoots: string[];

  if (options?.skillsRoots !== undefined || options?.agentsRoots !== undefined) {
    // Caller supplied explicit multi-root lists
    resolvedSkillsRoots = options.skillsRoots ?? [
      resolve(_repoRoot, "plugins", "cfa-core", "skills", "cfa"),
    ];
    resolvedAgentsRoots = options.agentsRoots ?? [
      resolve(_repoRoot, "plugins", "cfa-core", "agents", "cfa"),
    ];
  } else if (options?.skillsRoot !== undefined || options?.agentsRoot !== undefined) {
    // Caller supplied single legacy roots — keep single-root back-compat
    resolvedSkillsRoots = [
      options.skillsRoot ?? resolve(_repoRoot, "plugins", "cfa-core", "skills", "cfa"),
    ];
    resolvedAgentsRoots = [
      options.agentsRoot ?? resolve(_repoRoot, "plugins", "cfa-core", "agents", "cfa"),
    ];
  } else {
    // Default: walk all 3 plugin tiers + legacy fallback
    const discovered = discoverPluginRoots(_repoRoot);
    resolvedSkillsRoots = discovered.skillsRoots;
    resolvedAgentsRoots = discovered.agentsRoots;
  }

  // Keep single-root variables for YAML loader (needs one canonical root for
  // .yaml file resolution; we pick the last = legacy fallback which always
  // has the canonical 9 YAML manifests).
  const primaryAgentsRoot =
    resolvedAgentsRoots[resolvedAgentsRoots.length - 1] ??
    resolve(_repoRoot, "plugins", "cfa-core", "agents", "cfa");

  const loader =
    options?.loader ??
    createDirectSkillLoader({
      // Single-root fields are required by SkillLoaderOptions; we set them
      // to the first root and also pass the arrays for multi-root mode.
      skillsRoot: resolvedSkillsRoots[0] ?? "",
      agentsRoot: resolvedAgentsRoots[0] ?? "",
      skillsRoots: resolvedSkillsRoots,
      agentsRoots: resolvedAgentsRoots,
    });

  const yamlLoader =
    options?.yamlLoader ??
    createDirectYamlManifestLoader({
      agentsRoot: primaryAgentsRoot,
      skillLoader: loader,
    });

  /**
   * Load one agent: prefer <id>.yaml (Phase 36 canonical) over <id>.md.
   *
   * For YAML loading we use the primaryAgentsRoot (cfa-core), which has
   * correct from_plugin relative paths. The agent-plugins tier supplies .md
   * files (extends: form) that the multi-root loader handles via loadAgent.
   *
   * Walk order: check primaryAgentsRoot for .yaml first (correct refs),
   * then other agentsRoots for .yaml, then fall back to .md via multi-root loader.
   */
  async function loadOneAgent(id: string): Promise<AgentDef> {
    // 1. Prefer cfa-core canonical YAML (from_plugin paths are correct there)
    if (existsSync(resolve(primaryAgentsRoot, `${id}.yaml`))) {
      return yamlLoader.loadAgent(id);
    }
    // 2. Check remaining agentsRoots for a YAML (for future tiers with correct refs)
    for (const root of resolvedAgentsRoots) {
      if (root === primaryAgentsRoot) continue;
      const p = resolve(root, `${id}.yaml`);
      if (existsSync(p)) {
        const otherYamlLoader = createDirectYamlManifestLoader({
          agentsRoot: root,
          skillLoader: loader,
        });
        return otherYamlLoader.loadAgent(id);
      }
    }
    // 3. Fall back to .md via multi-root loader (extends: form)
    return loader.loadAgent(id, "agent");
  }

  // Load all 9 in parallel. Promise.all rejects on the first error, surfacing
  // a clear loader error message that names the missing id.
  const defs = await Promise.all(
    SKILL_REGISTRY_AGENT_IDS.map((id) => loadOneAgent(id)),
  );

  const byId = new Map<string, AgentDef>();
  for (let i = 0; i < SKILL_REGISTRY_AGENT_IDS.length; i++) {
    byId.set(SKILL_REGISTRY_AGENT_IDS[i]!, defs[i]!);
  }

  const chiefDef = byId.get("chief-analyst")!;
  const delegateDefs: AgentDef[] = SKILL_REGISTRY_AGENT_IDS.slice(1).map(
    (id) => byId.get(id)!,
  );

  return {
    chief(): AgentDef {
      return chiefDef;
    },
    delegates(): AgentDef[] {
      return delegateDefs;
    },
    get(id: string): AgentDef {
      const def = byId.get(id);
      if (def === undefined) {
        throw new Error(
          `Unknown agent id "${id}". Known agents: ${[...byId.keys()].join(", ")}`,
        );
      }
      return def;
    },
    all(): AgentDef[] {
      return [chiefDef, ...delegateDefs];
    },
  };
}
