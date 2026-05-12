#!/usr/bin/env tsx
/**
 * generate-cookbook-traces.ts — Phase 25 Tier C4 CLI runner.
 *
 * Builds one byte-deterministic synthetic-trace file per cookbook under
 * data/cookbook-traces/<slug>.trace.json. Each trace captures the FULL
 * assembled deploy payload — system prompt text, tool allowlists,
 * schemas — so PR reviewers can see exactly what would be sent to
 * Claude after the cookbook is loaded.
 *
 * Unlike replay (hash-only fingerprint) or audit (file inventory), the
 * trace is the artifact you read when you need to know "did the prompt
 * actually change, and what does it now say?".
 *
 * Modes:
 *   write (default):  rewrite data/cookbook-traces/*.trace.json
 *   --check:          regenerate in memory; non-zero exit on drift.
 *   --slug <slug>:    print one trace summary to stdout (no file write).
 *
 * Usage:
 *   npx tsx scripts/generate-cookbook-traces.ts
 *   npx tsx scripts/generate-cookbook-traces.ts --check
 *   npx tsx scripts/generate-cookbook-traces.ts --slug equity-analyst
 */

import { readFileSync, writeFileSync, existsSync, mkdirSync, readdirSync, unlinkSync } from "node:fs";
import { resolve, dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { createCookbookLoader } from "../packages/harness/src/manifests/cookbook-loader.js";
import {
  buildSyntheticTrace,
  serialiseTrace,
  extractSteeringEvents,
} from "../packages/harness/src/manifests/cookbook-trace.js";
import { parseAuditCatalog } from "../packages/harness/src/manifests/cookbook-audit.js";
import type { SkillLoader, ParsedSkill } from "../packages/harness/src/skills/types.js";

const __dirname = dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = resolve(__dirname, "..");
const COOKBOOKS_ROOT = resolve(REPO_ROOT, "managed-agent-cookbooks");
const TRACES_DIR = resolve(REPO_ROOT, "data", "cookbook-traces");
const AUDITS_PATH = resolve(REPO_ROOT, "data", "cookbook-audits.json");

const args = process.argv.slice(2);
const mode = args.includes("--check") ? "check" : "write";
const slugIdx = args.indexOf("--slug");
const slug = slugIdx >= 0 ? args[slugIdx + 1] : undefined;

function makeNullSkillLoader(): SkillLoader {
  return {
    async loadSkill(id: string): Promise<ParsedSkill> {
      throw new Error(`Unexpected from_skill reference to "${id}" in cookbook`);
    },
    async loadAgent() {
      throw new Error("not used in cookbook-traces");
    },
    clearCache() {},
  };
}

const loader = createCookbookLoader({
  cookbooksRoot: COOKBOOKS_ROOT,
  skillLoader: makeNullSkillLoader(),
});

interface SlugBundle {
  slug: string;
  version: string;
  auditHash: string;
  events: string[];
}

function loadAuditMap(): Map<string, { version: string; hash: string }> {
  if (!existsSync(AUDITS_PATH)) {
    throw new Error(
      `cookbook-audits.json missing at ${AUDITS_PATH} — run scripts/generate-cookbook-audits.ts first`,
    );
  }
  const catalog = parseAuditCatalog(readFileSync(AUDITS_PATH, "utf8"));
  const map = new Map<string, { version: string; hash: string }>();
  for (const c of catalog.cookbooks) {
    map.set(c.slug, { version: c.version, hash: c.hash });
  }
  return map;
}

function readSteeringEvents(slug: string): string[] {
  const path = join(COOKBOOKS_ROOT, slug, "steering-examples.json");
  if (!existsSync(path)) return [];
  try {
    return extractSteeringEvents(JSON.parse(readFileSync(path, "utf8")));
  } catch {
    return [];
  }
}

function discoverBundles(auditMap: Map<string, { version: string; hash: string }>): SlugBundle[] {
  const bundles: SlugBundle[] = [];
  for (const [slug, meta] of auditMap) {
    bundles.push({
      slug,
      version: meta.version,
      auditHash: meta.hash,
      events: readSteeringEvents(slug),
    });
  }
  return bundles.sort((a, b) => a.slug.localeCompare(b.slug));
}

function tracePathFor(slug: string): string {
  return join(TRACES_DIR, `${slug}.trace.json`);
}

async function buildSlugTrace(bundle: SlugBundle): Promise<string> {
  const loaded = await loader.load(bundle.slug);
  const trace = buildSyntheticTrace({
    loaded,
    auditHash: bundle.auditHash,
    version: bundle.version,
    exampleEvents: bundle.events,
  });
  return serialiseTrace(trace);
}

async function main(): Promise<void> {
  const auditMap = loadAuditMap();
  const bundles = discoverBundles(auditMap);

  // -------------------------------------------------------------------------
  // --slug — single trace summary to stdout
  // -------------------------------------------------------------------------
  if (slug) {
    const bundle = bundles.find((b) => b.slug === slug);
    if (!bundle) {
      process.stderr.write(`[cookbook-trace] no such cookbook: ${slug}\n`);
      process.exit(1);
    }
    const json = await buildSlugTrace(bundle);
    process.stdout.write(json);
    return;
  }

  // -------------------------------------------------------------------------
  // write / check
  // -------------------------------------------------------------------------
  const expected = new Map<string, string>();
  for (const b of bundles) {
    expected.set(tracePathFor(b.slug), await buildSlugTrace(b));
  }

  if (mode === "check") {
    const issues: string[] = [];
    // Drift: file content changed
    for (const [path, content] of expected) {
      if (!existsSync(path)) {
        issues.push(`missing: ${path}`);
        continue;
      }
      const onDisk = readFileSync(path, "utf8");
      if (onDisk !== content) {
        const slug = path.replace(/^.*\//, "").replace(/\.trace\.json$/, "");
        issues.push(`drift:   ${slug}`);
      }
    }
    // Drift: stale files (cookbook removed but trace file still present)
    if (existsSync(TRACES_DIR)) {
      const present = readdirSync(TRACES_DIR).filter((f) => f.endsWith(".trace.json"));
      const expectedFiles = new Set([...expected.keys()].map((p) => p.replace(/^.*\//, "")));
      for (const name of present) {
        if (!expectedFiles.has(name)) {
          issues.push(`stale:   ${name}`);
        }
      }
    }
    if (issues.length === 0) {
      process.stdout.write(
        `[cookbook-trace] OK — traces up to date (${expected.size} cookbooks).\n`,
      );
      return;
    }
    process.stderr.write(`[cookbook-trace] drift detected:\n`);
    for (const i of issues) process.stderr.write(`  ${i}\n`);
    process.stderr.write(
      `  Regenerate with: npx tsx scripts/generate-cookbook-traces.ts\n`,
    );
    process.exit(1);
  }

  mkdirSync(TRACES_DIR, { recursive: true });
  // Write expected; remove stale.
  const expectedNames = new Set([...expected.keys()].map((p) => p.replace(/^.*\//, "")));
  if (existsSync(TRACES_DIR)) {
    for (const f of readdirSync(TRACES_DIR)) {
      if (f.endsWith(".trace.json") && !expectedNames.has(f)) {
        unlinkSync(join(TRACES_DIR, f));
      }
    }
  }
  for (const [path, content] of expected) {
    writeFileSync(path, content, "utf8");
  }

  process.stdout.write(
    `[cookbook-trace] wrote ${expected.size} traces to ${TRACES_DIR}\n`,
  );
  for (const b of bundles) {
    const tracePath = tracePathFor(b.slug);
    const bytes = readFileSync(tracePath, "utf8").length;
    process.stdout.write(
      `  ${b.slug.padEnd(28)}  v${b.version.padEnd(8)}  ${bytes.toString().padStart(7)} bytes  ${b.events.length} events\n`,
    );
  }
}

main().catch((err) => {
  process.stderr.write(
    `[cookbook-trace] error: ${err instanceof Error ? err.message : String(err)}\n`,
  );
  process.exit(2);
});
