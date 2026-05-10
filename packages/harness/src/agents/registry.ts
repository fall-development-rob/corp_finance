/**
 * Agent registry — Phase 31 Wave 1.
 *
 * Maps agent ids to AgentDef instances. Wave 1 contains chief-analyst only.
 * Specialist agents (equity, credit, fixed-income, derivatives, quant-risk,
 * macro, esg-regulatory, private-markets) are registered in Wave 2.
 *
 * Also exports defaultMCPServers — the four cfa plugin server configs that
 * the harness CLI and integration tests use as a starting point.
 */

import { existsSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import type { AgentDef, MCPServerConfig } from "../types.js";
import { chiefAnalyst } from "./chief-analyst.js";

export { chiefAnalyst } from "./chief-analyst.js";

// ---------------------------------------------------------------------------
// Registry
// ---------------------------------------------------------------------------

const _entries: AgentDef[] = [chiefAnalyst];

export const registry: Map<string, AgentDef> = new Map(
  _entries.map((a) => [a.id, a]),
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
