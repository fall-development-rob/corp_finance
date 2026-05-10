/**
 * Agent registry — Phase 33 Wave 4.
 *
 * Maps agent ids to AgentDef instances by loading skill files at module-init
 * time via top-level await. The registry surface (chiefAnalyst,
 * defaultDelegates, registry, getAgent, defaultMCPServers) is preserved so
 * existing consumers (CLI, integration tests, agents.test.ts,
 * specialist-routing.test.ts) keep working unchanged.
 *
 * Single source of truth for agent prose now lives at
 * `plugins/cfa-core/skills/cfa/corp-finance-analyst-<id>/SKILL.md` plus the
 * thin manifests at `plugins/cfa-core/agents/cfa/<id>.md`. See ADR-031.
 */

import { existsSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import type { AgentDef, MCPServerConfig } from "../types.js";
import { createSkillRegistry } from "./skill-registry.js";

// ---------------------------------------------------------------------------
// Skill-loaded registry (top-level await; harness is pure ESM)
// ---------------------------------------------------------------------------

const _skillRegistry = await createSkillRegistry();

export const chiefAnalyst: AgentDef = _skillRegistry.chief();
export const equityAnalyst: AgentDef = _skillRegistry.get("equity-analyst");
export const creditAnalyst: AgentDef = _skillRegistry.get("credit-analyst");
export const fixedIncomeAnalyst: AgentDef = _skillRegistry.get(
  "fixed-income-analyst",
);
export const derivativesAnalyst: AgentDef = _skillRegistry.get(
  "derivatives-analyst",
);
export const quantRiskAnalyst: AgentDef = _skillRegistry.get(
  "quant-risk-analyst",
);
export const macroAnalyst: AgentDef = _skillRegistry.get("macro-analyst");
export const privateMarketsAnalyst: AgentDef = _skillRegistry.get(
  "private-markets-analyst",
);
export const esgRegulatoryAnalyst: AgentDef = _skillRegistry.get(
  "esg-regulatory-analyst",
);

// ---------------------------------------------------------------------------
// Registry surface
// ---------------------------------------------------------------------------

export const defaultDelegates: AgentDef[] = _skillRegistry.delegates();

export const registry: Map<string, AgentDef> = new Map(
  _skillRegistry.all().map((a) => [a.id, a]),
);

export function getAgent(id: string): AgentDef {
  const def = registry.get(id);
  if (def === undefined) {
    throw new Error(
      `Unknown agent id "${id}". Known agents: ${[...registry.keys()].join(", ")}`,
    );
  }
  return def;
}

// ---------------------------------------------------------------------------
// Repo root detection
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
  // Fallback: assume three levels up from packages/harness/src/agents/
  return resolve(startDir, "..", "..", "..", "..", "..");
}

const _thisDir = dirname(fileURLToPath(import.meta.url));
const _repoRoot = findRepoRoot(_thisDir);

// ---------------------------------------------------------------------------
// Default MCP server configurations
// ---------------------------------------------------------------------------

export const defaultMCPServers: MCPServerConfig[] = [
  {
    name: "cfa-core",
    prefix: "mcp__plugin_cfa-core_cfa-core__",
    command: "node",
    args: [resolve(_repoRoot, "plugins", "cfa-core", "mcp", "dist", "server.js")],
  },
  {
    name: "data",
    prefix: "mcp__plugin_cfa-data_data__",
    command: "node",
    args: [resolve(_repoRoot, "plugins", "cfa-data", "mcp", "dist", "index.js")],
  },
  {
    name: "fmp-market-data",
    prefix: "mcp__plugin_cfa-pro_fmp-market-data__",
    command: "node",
    args: [resolve(_repoRoot, "plugins", "cfa-pro", "mcp", "dist", "index.js")],
  },
  {
    name: "vendor",
    prefix: "mcp__plugin_cfa-pro_vendor__",
    command: "node",
    args: [resolve(_repoRoot, "plugins", "cfa-pro", "mcp", "vendors", "dist", "index.js")],
  },
];
