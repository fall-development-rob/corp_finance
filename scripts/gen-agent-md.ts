#!/usr/bin/env tsx
/**
 * gen-agent-md.ts — Phase 36 migration script.
 *
 * Reads each <id>.yaml manifest from plugins/cfa-core/agents/cfa/ and
 * regenerates the corresponding <id>.md thin manifest for Claude Code
 * session-boot compat. Idempotent: running twice produces byte-identical output.
 *
 * Usage:
 *   npx tsx scripts/gen-agent-md.ts
 *   npx tsx scripts/gen-agent-md.ts --agent equity-analyst
 */

import { readFile, writeFile } from "node:fs/promises";
import { existsSync } from "node:fs";
import { resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { parse as parseYaml } from "yaml";

const __dirname = dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = resolve(__dirname, "..");
const AGENTS_DIR = resolve(REPO_ROOT, "plugins", "cfa-core", "agents", "cfa");

const AGENT_IDS = [
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
// Tool projection (mirrors yaml-loader.ts static projection logic)
// ---------------------------------------------------------------------------

interface ToolsetBlock {
  type: string;
  mcp_server_name?: string;
  default_config?: { enabled: boolean };
  configs?: Array<{ name: string; enabled: boolean }>;
  tools?: string[];
}

function stripPrefix(name: string): string {
  const m = /^mcp__[^_][^_]*__(.+)$/.exec(name);
  return m ? (m[1] ?? name) : name;
}

function projectTools(toolsets: ToolsetBlock[]): string[] | "*" {
  if (!toolsets || toolsets.length === 0) return "*";

  const collected = new Set<string>();
  let hasWildcard = false;

  for (const block of toolsets) {
    if (block.type === "tools_array") {
      for (const t of block.tools ?? []) collected.add(stripPrefix(t));
      continue;
    }
    const defaultEnabled = block.default_config?.enabled ?? true;
    if (defaultEnabled) {
      hasWildcard = true;
      continue;
    }
    for (const cfg of block.configs ?? []) {
      if (cfg.enabled) collected.add(stripPrefix(cfg.name));
    }
  }

  if (hasWildcard) return "*";
  const list = [...collected].sort();
  return list.length > 0 ? list : "*";
}

// ---------------------------------------------------------------------------
// Frontmatter serialiser
// ---------------------------------------------------------------------------

function toYamlSeq(items: string[]): string {
  return items.map((t) => `  - ${t}`).join("\n");
}

function buildFrontmatter(
  id: string,
  manifest: Record<string, unknown>,
): string {
  const lines: string[] = ["---"];

  lines.push(`name: ${id}`);

  const desc =
    typeof manifest.description === "string" ? manifest.description : "";
  if (desc) {
    lines.push(`description: ${desc}`);
  }

  // skills → extends (slug from from_plugin basename or from_skill value)
  const skills = (manifest.skills as Array<Record<string, string>> | undefined) ?? [];
  const extendsIds = skills.map((ref) => {
    if (ref.from_plugin) {
      // basename of the path
      const parts = ref.from_plugin.split("/");
      return parts[parts.length - 1] ?? ref.from_plugin;
    }
    return ref.from_skill ?? "";
  }).filter(Boolean);

  if (extendsIds.length > 0) {
    lines.push("extends:");
    for (const e of extendsIds) lines.push(`  - ${e}`);
  }

  // tools
  const toolsets = (manifest.tools as ToolsetBlock[] | undefined) ?? [];
  const projected = projectTools(toolsets);
  if (projected === "*") {
    lines.push(`tools: "*"`);
  } else {
    lines.push("tools:");
    lines.push(toYamlSeq(projected));
  }

  if (typeof manifest.model === "string") {
    lines.push(`model: ${manifest.model}`);
  }
  if (typeof manifest.max_tokens === "number") {
    lines.push(`max_tokens: ${manifest.max_tokens}`);
  }
  if (typeof manifest.max_recursion_depth === "number") {
    lines.push(`max_recursion_depth: ${manifest.max_recursion_depth}`);
  }

  lines.push("---\n");
  return lines.join("\n");
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

async function genOne(id: string): Promise<void> {
  const yamlPath = resolve(AGENTS_DIR, `${id}.yaml`);
  const mdPath = resolve(AGENTS_DIR, `${id}.md`);

  if (!existsSync(yamlPath)) {
    console.warn(`  SKIP ${id} — no .yaml found at ${yamlPath}`);
    return;
  }

  const raw = await readFile(yamlPath, "utf-8");
  const manifest = parseYaml(raw, { strict: true }) as Record<string, unknown>;

  const content = buildFrontmatter(id, manifest);
  await writeFile(mdPath, content, "utf-8");
  console.log(`  WROTE ${mdPath}`);
}

async function main(): Promise<void> {
  const args = process.argv.slice(2);
  const agentFlag = args.indexOf("--agent");
  const ids: readonly string[] =
    agentFlag !== -1 ? [args[agentFlag + 1] ?? ""] : AGENT_IDS;

  console.log(`gen-agent-md: generating ${ids.length} manifest(s)…`);
  for (const id of ids) {
    await genOne(id);
  }
  console.log("done.");
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
