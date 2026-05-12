#!/usr/bin/env tsx
/**
 * deploy-cookbook.ts — Phase 28 D1 deploy CLI.
 *
 * Replaces the broken scripts/deploy-managed-agent.sh (which reads
 * legacy agent.json — cookbooks are agent.yaml since Phase 36) and the
 * sibling-repo Rust deploy.rs (also legacy-shape).
 *
 * Modes:
 *   --dry-run (default): print the assembled JSON payload to stdout.
 *                        No network. Surfaces any unresolved ${VAR}.
 *   --apply:             POST skills → subagents → orchestrator to
 *                        api.anthropic.com/v1 in the correct order.
 *                        Requires ANTHROPIC_API_KEY.
 *
 * Usage:
 *   npx tsx scripts/deploy-cookbook.ts <slug>            # dry-run (default)
 *   npx tsx scripts/deploy-cookbook.ts <slug> --apply    # actually deploy
 *   npx tsx scripts/deploy-cookbook.ts <slug> --env-file deploy.env  # load env from file
 *
 * Env vars consumed by the cookbook (must all resolve before --apply):
 *   CFA_CORE_MCP_URL, FMP_MCP_URL, DATA_MCP_URL, VENDOR_MCP_URL (and others
 *   referenced in agent.yaml). Run --dry-run to see exactly which.
 *
 * Required for --apply:
 *   ANTHROPIC_API_KEY                 (Anthropic API auth)
 *   ANTHROPIC_VERSION                 (default "2023-06-01")
 *   ANTHROPIC_MANAGED_AGENTS_BETA     (default "managed-agents-2026-05-06")
 */

import { readFileSync, existsSync } from "node:fs";
import { resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";

import {
  buildDeployPayload,
  serialiseDeployPayload,
  type DeployPayload,
} from "../packages/harness/src/deploy/build-payload.js";

const __dirname = dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = resolve(__dirname, "..");
const COOKBOOKS_ROOT = resolve(REPO_ROOT, "managed-agent-cookbooks");

const args = process.argv.slice(2);

function flagValue(name: string): string | undefined {
  const idx = args.indexOf(name);
  if (idx < 0) return undefined;
  return args[idx + 1];
}

const apply = args.includes("--apply");
const envFile = flagValue("--env-file");
const slug = args.find((a) => !a.startsWith("--") && a !== envFile);

if (!slug) {
  process.stderr.write(
    "[deploy-cookbook] missing <slug>\n" +
      "  usage: npx tsx scripts/deploy-cookbook.ts <slug> [--apply] [--env-file <path>]\n",
  );
  process.exit(1);
}

const cookbookDir = resolve(COOKBOOKS_ROOT, slug);
if (!existsSync(cookbookDir)) {
  process.stderr.write(`[deploy-cookbook] no cookbook at ${cookbookDir}\n`);
  process.exit(1);
}

// ---------------------------------------------------------------------------
// Env-var collection
// ---------------------------------------------------------------------------

function loadEnvFile(path: string): Record<string, string> {
  if (!existsSync(path)) {
    throw new Error(`env file not found: ${path}`);
  }
  const out: Record<string, string> = {};
  for (const line of readFileSync(path, "utf8").split("\n")) {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith("#")) continue;
    const eq = trimmed.indexOf("=");
    if (eq < 0) continue;
    const key = trimmed.slice(0, eq).trim();
    let value = trimmed.slice(eq + 1).trim();
    // Strip wrapping quotes
    if (
      (value.startsWith('"') && value.endsWith('"')) ||
      (value.startsWith("'") && value.endsWith("'"))
    ) {
      value = value.slice(1, -1);
    }
    out[key] = value;
  }
  return out;
}

let envVars: Record<string, string> = {};
for (const [k, v] of Object.entries(process.env)) {
  if (typeof v === "string") envVars[k] = v;
}
if (envFile) {
  envVars = { ...envVars, ...loadEnvFile(envFile) };
}

// ---------------------------------------------------------------------------
// Build payload
// ---------------------------------------------------------------------------

let payload: DeployPayload;
try {
  payload = buildDeployPayload({ cookbookDir, slug, envVars });
} catch (err) {
  process.stderr.write(
    `[deploy-cookbook] build error: ${err instanceof Error ? err.message : String(err)}\n`,
  );
  process.exit(2);
}

if (!apply) {
  // Dry-run: print JSON to stdout, summary + warnings to stderr.
  process.stdout.write(serialiseDeployPayload(payload));
  process.stderr.write(
    `\n[deploy-cookbook] dry-run summary for ${payload.slug} v${payload.version}\n` +
      `  skills:    ${payload.skills.length}\n` +
      `  subagents: ${payload.subagents.length}\n`,
  );
  if (payload.env_unresolved.length > 0) {
    process.stderr.write(
      `  WARNING: ${payload.env_unresolved.length} unresolved env vars: ${payload.env_unresolved.join(", ")}\n` +
        `  Set them in process env or via --env-file before running --apply.\n`,
    );
  }
  process.exit(0);
}

// ---------------------------------------------------------------------------
// --apply: POST to Anthropic Managed Agents API
// ---------------------------------------------------------------------------

const apiKey = envVars.ANTHROPIC_API_KEY;
if (!apiKey) {
  process.stderr.write(
    `[deploy-cookbook] --apply requires ANTHROPIC_API_KEY in env (or via --env-file).\n`,
  );
  process.exit(1);
}
if (payload.env_unresolved.length > 0) {
  process.stderr.write(
    `[deploy-cookbook] cannot --apply with unresolved env vars: ${payload.env_unresolved.join(", ")}\n`,
  );
  process.exit(1);
}

const API_BASE = envVars.ANTHROPIC_API_BASE ?? "https://api.anthropic.com";
const VERSION = envVars.ANTHROPIC_VERSION ?? "2023-06-01";
const BETA = envVars.ANTHROPIC_MANAGED_AGENTS_BETA ?? "managed-agents-2026-05-06";

async function postJson<T>(path: string, body: unknown): Promise<T> {
  const url = `${API_BASE}${path}`;
  const res = await fetch(url, {
    method: "POST",
    headers: {
      "x-api-key": apiKey!,
      "anthropic-version": VERSION,
      "anthropic-beta": BETA,
      "content-type": "application/json",
    },
    body: JSON.stringify(body),
  });
  if (!res.ok) {
    const text = await res.text();
    throw new Error(`POST ${path} → ${res.status} ${res.statusText}: ${text.slice(0, 500)}`);
  }
  return (await res.json()) as T;
}

async function deploy(): Promise<void> {
  process.stderr.write(`[deploy-cookbook] APPLY ${payload.slug} v${payload.version}\n`);

  // 1. Upload skills, collect IDs.
  const skillIds = new Map<string, string>();
  for (const s of payload.skills) {
    process.stderr.write(`  uploading skill ${s.name}…\n`);
    const r = await postJson<{ id: string }>("/v1/skills", {
      name: s.name,
      content: s.content,
    });
    skillIds.set(s.name, r.id);
    process.stderr.write(`    skill_id=${r.id}\n`);
  }

  // 2. Create subagents, collect IDs.
  const subagentIds: string[] = [];
  for (const sa of payload.subagents) {
    process.stderr.write(`  creating subagent ${sa.name}…\n`);
    // Patch in resolved skill_ids on the body.
    const body = patchSkillIds(sa.body, skillIds);
    const r = await postJson<{ id: string }>("/v1/agents", body);
    subagentIds.push(r.id);
    process.stderr.write(`    agent_id=${r.id}\n`);
  }

  // 3. Create orchestrator with patched skill_ids + agent_ids.
  process.stderr.write(`  creating orchestrator ${payload.orchestrator.name}…\n`);
  const orchBody = patchSkillIds(payload.orchestrator.body, skillIds);
  patchSubagentIds(orchBody, subagentIds);
  const r = await postJson<{ id: string }>("/v1/agents", orchBody);
  process.stderr.write(`    orchestrator_id=${r.id}\n`);

  // 4. Summary.
  process.stderr.write(`\n[deploy-cookbook] APPLIED ${payload.slug} v${payload.version}\n`);
  process.stdout.write(
    JSON.stringify(
      {
        slug: payload.slug,
        version: payload.version,
        orchestrator_id: r.id,
        subagent_ids: subagentIds,
        skill_ids: Object.fromEntries(skillIds),
      },
      null,
      2,
    ) + "\n",
  );
}

function patchSkillIds(
  body: Record<string, unknown>,
  skillIds: Map<string, string>,
): Record<string, unknown> {
  const copy = JSON.parse(JSON.stringify(body)) as Record<string, unknown>;
  const skills = copy.skills as Array<Record<string, unknown>> | undefined;
  if (!skills) return copy;
  for (const s of skills) {
    // Resolve skill_id by name. The skill name we use is the basename of
    // the from_plugin directory (matches the upload step).
    const fp = typeof s.from_plugin === "string" ? s.from_plugin : "";
    const name = fp.split("/").filter(Boolean).pop() ?? "";
    const id = skillIds.get(name);
    if (id) s.skill_id = id;
  }
  return copy;
}

function patchSubagentIds(
  orchBody: Record<string, unknown>,
  subagentIds: string[],
): void {
  const ca = orchBody.callable_agents as Array<Record<string, unknown>> | undefined;
  if (!ca) return;
  for (let i = 0; i < ca.length; i++) {
    ca[i] = { ...ca[i], agent_id: subagentIds[i] };
  }
}

deploy().catch((err) => {
  process.stderr.write(
    `\n[deploy-cookbook] FAILED: ${err instanceof Error ? err.message : String(err)}\n`,
  );
  process.exit(2);
});
