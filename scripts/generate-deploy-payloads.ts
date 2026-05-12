#!/usr/bin/env tsx
/**
 * generate-deploy-payloads.ts — Phase 28 D1 snapshot generator.
 *
 * Builds one dry-run deploy payload per cookbook and writes it to
 * data/cookbook-deploy-payloads/<slug>.payload.json. Used by CI as a
 * byte-diffable contract: any PR that changes a cookbook's deployable
 * surface — system prompt, tool config, mcp_servers list, skills,
 * subagent shape — must include the regenerated payload.
 *
 * Env-var substitution uses placeholder values so the snapshots are
 * fully deterministic and don't leak real URLs. Real deploys read
 * env from process.env or --env-file via scripts/deploy-cookbook.ts.
 *
 * Modes:
 *   write (default):  rewrite data/cookbook-deploy-payloads/*.payload.json
 *   --check:          regenerate in memory; non-zero exit on drift.
 *   --slug <slug>:    print one payload to stdout (no file write).
 */

import {
  readFileSync,
  writeFileSync,
  existsSync,
  mkdirSync,
  readdirSync,
  unlinkSync,
} from "node:fs";
import { resolve, dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import {
  buildDeployPayload,
  serialiseDeployPayload,
} from "../packages/harness/src/deploy/build-payload.js";

const __dirname = dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = resolve(__dirname, "..");
const COOKBOOKS_ROOT = resolve(REPO_ROOT, "managed-agent-cookbooks");
const SNAPSHOTS_DIR = resolve(REPO_ROOT, "data", "cookbook-deploy-payloads");

// Deterministic placeholder env. Real deploys substitute real URLs at the
// CLI layer (scripts/deploy-cookbook.ts). The snapshot baseline must not
// drift just because the on-disk env changes.
const PLACEHOLDER_ENV: Record<string, string> = {
  CFA_CORE_MCP_URL: "https://example.invalid/mcp/cfa-core",
  FMP_MCP_URL: "https://example.invalid/mcp/fmp",
  DATA_MCP_URL: "https://example.invalid/mcp/data",
  VENDOR_MCP_URL: "https://example.invalid/mcp/vendor",
  IRONCLAD_MCP_URL: "https://example.invalid/mcp/ironclad",
  GDRIVE_MCP_URL: "https://example.invalid/mcp/gdrive",
  IMANAGE_MCP_URL: "https://example.invalid/mcp/imanage",
  DOCUSIGN_MCP_URL: "https://example.invalid/mcp/docusign",
};

const args = process.argv.slice(2);
const mode = args.includes("--check") ? "check" : "write";
const slugIdx = args.indexOf("--slug");
const oneSlug = slugIdx >= 0 ? args[slugIdx + 1] : undefined;

function discoverSlugs(): string[] {
  if (!existsSync(COOKBOOKS_ROOT)) return [];
  return readdirSync(COOKBOOKS_ROOT, { withFileTypes: true })
    .filter(
      (e) =>
        e.isDirectory() &&
        existsSync(join(COOKBOOKS_ROOT, e.name, "agent.yaml")),
    )
    .map((e) => e.name)
    .sort();
}

function snapshotPathFor(slug: string): string {
  return join(SNAPSHOTS_DIR, `${slug}.payload.json`);
}

function buildOne(slug: string): string {
  const cookbookDir = join(COOKBOOKS_ROOT, slug);
  const payload = buildDeployPayload({
    cookbookDir,
    slug,
    envVars: PLACEHOLDER_ENV,
  });
  return serialiseDeployPayload(payload);
}

// ---------------------------------------------------------------------------
// --slug — single payload to stdout
// ---------------------------------------------------------------------------
if (oneSlug) {
  if (!existsSync(join(COOKBOOKS_ROOT, oneSlug, "agent.yaml"))) {
    process.stderr.write(`[deploy-payloads] no cookbook at ${oneSlug}\n`);
    process.exit(1);
  }
  process.stdout.write(buildOne(oneSlug));
  process.exit(0);
}

// ---------------------------------------------------------------------------
// write / check
// ---------------------------------------------------------------------------
const slugs = discoverSlugs();
const expected = new Map<string, string>();
for (const s of slugs) {
  expected.set(snapshotPathFor(s), buildOne(s));
}

if (mode === "check") {
  const issues: string[] = [];
  for (const [path, content] of expected) {
    if (!existsSync(path)) {
      issues.push(`missing: ${path.replace(/^.*\//, "")}`);
      continue;
    }
    if (readFileSync(path, "utf8") !== content) {
      issues.push(`drift:   ${path.replace(/^.*\//, "").replace(".payload.json", "")}`);
    }
  }
  if (existsSync(SNAPSHOTS_DIR)) {
    const expectedNames = new Set(
      [...expected.keys()].map((p) => p.replace(/^.*\//, "")),
    );
    for (const f of readdirSync(SNAPSHOTS_DIR)) {
      if (f.endsWith(".payload.json") && !expectedNames.has(f)) {
        issues.push(`stale:   ${f}`);
      }
    }
  }
  if (issues.length === 0) {
    process.stdout.write(
      `[deploy-payloads] OK — payloads up to date (${expected.size} cookbooks).\n`,
    );
    process.exit(0);
  }
  process.stderr.write(`[deploy-payloads] drift detected:\n`);
  for (const i of issues) process.stderr.write(`  ${i}\n`);
  process.stderr.write(
    `  Regenerate with: npx tsx scripts/generate-deploy-payloads.ts\n`,
  );
  process.exit(1);
}

mkdirSync(SNAPSHOTS_DIR, { recursive: true });
const expectedNames = new Set(
  [...expected.keys()].map((p) => p.replace(/^.*\//, "")),
);
if (existsSync(SNAPSHOTS_DIR)) {
  for (const f of readdirSync(SNAPSHOTS_DIR)) {
    if (f.endsWith(".payload.json") && !expectedNames.has(f)) {
      unlinkSync(join(SNAPSHOTS_DIR, f));
    }
  }
}
for (const [path, content] of expected) {
  writeFileSync(path, content, "utf8");
}
process.stdout.write(
  `[deploy-payloads] wrote ${expected.size} payloads to ${SNAPSHOTS_DIR}\n`,
);
for (const s of slugs) {
  const bytes = readFileSync(snapshotPathFor(s), "utf8").length;
  process.stdout.write(`  ${s.padEnd(28)}  ${bytes.toString().padStart(7)} bytes\n`);
}
