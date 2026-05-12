#!/usr/bin/env tsx
/**
 * generate-cookbook-costs.ts — Phase 25 Tier C1 CLI runner.
 *
 * Loads every cookbook via the harness CookbookLoader, estimates a
 * deterministic worst-case dollar cost per invocation, and emits
 * data/cookbook-costs.json. Cost = sum over (parent + all subagents) of
 * (sys_prompt_tokens × input_rate + max_tokens × output_rate) using
 * the static pricing table in packages/harness/src/manifests/cookbook-cost.ts.
 *
 * The estimate is a worst-case ceiling — every agent assumed to use its
 * full max_tokens budget. Real invocations cost less. Tool-call round-trips
 * are not yet modeled.
 *
 * Modes:
 *   write (default):  overwrite data/cookbook-costs.json
 *   --check:          regenerate in memory; non-zero exit on drift.
 *   --slug <slug>:    estimate one cookbook; print breakdown to stdout.
 *
 * Usage:
 *   npx tsx scripts/generate-cookbook-costs.ts
 *   npx tsx scripts/generate-cookbook-costs.ts --check
 *   npx tsx scripts/generate-cookbook-costs.ts --slug equity-analyst
 */

import { readFileSync, writeFileSync, existsSync, mkdirSync } from "node:fs";
import { resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";

import { createCookbookLoader } from "../packages/harness/src/manifests/cookbook-loader.js";
import {
  buildCostCatalog,
  estimateCookbookCost,
  serialiseCostCatalog,
} from "../packages/harness/src/manifests/cookbook-cost.js";
import type { SkillLoader, ParsedSkill } from "../packages/harness/src/skills/types.js";

const __dirname = dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = resolve(__dirname, "..");
const COOKBOOKS_ROOT = resolve(REPO_ROOT, "managed-agent-cookbooks");
const COSTS_PATH = resolve(REPO_ROOT, "data", "cookbook-costs.json");

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
      throw new Error("not used in cookbook-costs");
    },
    clearCache() {},
  };
}

const loader = createCookbookLoader({
  cookbooksRoot: COOKBOOKS_ROOT,
  skillLoader: makeNullSkillLoader(),
});

async function main(): Promise<void> {
  // -------------------------------------------------------------------------
  // --slug — estimate one cookbook
  // -------------------------------------------------------------------------
  if (slug) {
    const loaded = await loader.load(slug);
    const est = estimateCookbookCost(loaded);
    process.stdout.write(`${est.slug}\n`);
    for (const a of est.agents) {
      const flag = a.model_priced ? " " : "*";
      process.stdout.write(
        `  ${flag}${a.id.padEnd(40)}  ${a.model.padEnd(22)}  ` +
          `in=${a.input_tokens_est.toString().padStart(6)}t  ` +
          `out=${a.output_tokens_max.toString().padStart(5)}t  ` +
          `$${a.total_cost_usd.toFixed(6)}\n`,
      );
    }
    process.stdout.write(`  TOTAL ${" ".repeat(94)}$${est.total_cost_usd.toFixed(6)}\n`);
    return;
  }

  // -------------------------------------------------------------------------
  // write / check
  // -------------------------------------------------------------------------
  const loadedAll = await loader.loadAll();
  const catalog = buildCostCatalog(loadedAll);
  const serialised = serialiseCostCatalog(catalog);

  if (mode === "check") {
    if (!existsSync(COSTS_PATH)) {
      process.stderr.write(
        `[cookbook-costs] data/cookbook-costs.json missing — run without --check to create it.\n`,
      );
      process.exit(1);
    }
    const onDisk = readFileSync(COSTS_PATH, "utf8");
    if (onDisk !== serialised) {
      process.stderr.write(
        `[cookbook-costs] drift detected — data/cookbook-costs.json is stale.\n` +
          `  Regenerate with: npx tsx scripts/generate-cookbook-costs.ts\n`,
      );
      process.exit(1);
    }
    process.stdout.write(
      `[cookbook-costs] OK — catalog up to date ` +
        `(${catalog.cookbooks.length} cookbooks, ` +
        `worst-case grand total $${catalog.grand_total_usd.toFixed(4)}).\n`,
    );
    return;
  }

  mkdirSync(dirname(COSTS_PATH), { recursive: true });
  writeFileSync(COSTS_PATH, serialised, "utf8");
  process.stdout.write(
    `[cookbook-costs] wrote ${COSTS_PATH}\n` +
      `  ${catalog.cookbooks.length} cookbooks estimated\n` +
      `  worst-case grand total: $${catalog.grand_total_usd.toFixed(4)} per invocation cycle\n\n`,
  );
  for (const c of catalog.cookbooks) {
    process.stdout.write(
      `  ${c.slug.padEnd(28)}  $${c.total_cost_usd.toFixed(6)}  (${c.agents.length} agents)\n`,
    );
  }
}

main().catch((err) => {
  process.stderr.write(
    `[cookbook-costs] error: ${err instanceof Error ? err.message : String(err)}\n`,
  );
  process.exit(2);
});
