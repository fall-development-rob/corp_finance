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

import { existsSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import type { AgentDef } from "../types.js";
import { createDirectSkillLoader } from "../skills/index.js";
import type { SkillLoader } from "../skills/types.js";

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
  /** Defaults to <repo>/plugins/cfa-core/skills/cfa */
  skillsRoot?: string;
  /** Defaults to <repo>/plugins/cfa-core/agents/cfa */
  agentsRoot?: string;
  /** Injectable for testing. */
  loader?: SkillLoader;
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
  const skillsRoot =
    options?.skillsRoot ??
    resolve(_repoRoot, "plugins", "cfa-core", "skills", "cfa");
  const agentsRoot =
    options?.agentsRoot ??
    resolve(_repoRoot, "plugins", "cfa-core", "agents", "cfa");

  const loader =
    options?.loader ?? createDirectSkillLoader({ skillsRoot, agentsRoot });

  // Load all 9 in parallel. Promise.all rejects on the first error, surfacing
  // a clear loader error message that names the missing id.
  const defs = await Promise.all(
    SKILL_REGISTRY_AGENT_IDS.map((id) => loader.loadAgent(id, "agent")),
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
