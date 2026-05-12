#!/usr/bin/env tsx
/**
 * generate-cookbook-replays.ts — Phase 25 Tier A3 CLI runner.
 *
 * Walks managed-agent-cookbooks/, loads every cookbook via the harness
 * CookbookLoader, and emits data/cookbook-replays.json — a byte-stable
 * snapshot of what the loader produces (parent + subagent fingerprints).
 *
 * Catches three bug classes the audit hash cannot:
 *   1. Loader regressions (projectTools / system-prompt assembly bugs)
 *      that change loader output without any file diff.
 *   2. Structured drift visibility: PR reviewers see "analyst subagent
 *      lost tool X" instead of "this YAML file changed".
 *   3. Subagent surface drift: tools + model + block_tools per subagent
 *      reviewable in one JSON block per cookbook.
 *
 * Modes:
 *   write (default):  overwrite data/cookbook-replays.json
 *   --check:          regenerate in memory; non-zero exit on drift.
 *   --diff <path>:    diff current replay against <path> (e.g. main
 *                     branch baseline) — JSON output to stdout.
 *   --slug <slug>:    replay only the named cookbook; print fingerprint
 *                     summary to stdout. Useful for spot-checks.
 *
 * Usage:
 *   npx tsx scripts/generate-cookbook-replays.ts
 *   npx tsx scripts/generate-cookbook-replays.ts --check
 *   npx tsx scripts/generate-cookbook-replays.ts --slug equity-analyst
 */

import { readFileSync, writeFileSync, existsSync, mkdirSync } from "node:fs";
import { resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";

import { createCookbookLoader } from "../packages/harness/src/manifests/cookbook-loader.js";
import {
  replayCookbook,
  replayAllCookbooks,
  serialiseReplayCatalog,
  parseReplayCatalog,
  diffReplayCatalogs,
} from "../packages/harness/src/manifests/cookbook-replay.js";
import type { SkillLoader, ParsedSkill } from "../packages/harness/src/skills/types.js";

const __dirname = dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = resolve(__dirname, "..");
const COOKBOOKS_ROOT = resolve(REPO_ROOT, "managed-agent-cookbooks");
const REPLAYS_PATH = resolve(REPO_ROOT, "data", "cookbook-replays.json");

const args = process.argv.slice(2);
const mode = args.includes("--check") ? "check" : "write";
const slugIdx = args.indexOf("--slug");
const slug = slugIdx >= 0 ? args[slugIdx + 1] : undefined;
const diffIdx = args.indexOf("--diff");
const diffBaseline = diffIdx >= 0 ? args[diffIdx + 1] : undefined;

/**
 * Cookbooks use `from_plugin` (path-based) rather than `from_skill` (slug-based),
 * so the loader never invokes loadSkill. The null implementation throws if it
 * is invoked, which surfaces any unexpected `from_skill` reference loudly.
 */
function makeNullSkillLoader(): SkillLoader {
  return {
    async loadSkill(id: string): Promise<ParsedSkill> {
      throw new Error(`Unexpected from_skill reference to "${id}" in cookbook`);
    },
    async loadAgent() {
      throw new Error("not used in cookbook-replay");
    },
    clearCache() {},
  };
}

const loader = createCookbookLoader({
  cookbooksRoot: COOKBOOKS_ROOT,
  skillLoader: makeNullSkillLoader(),
});

async function main(): Promise<void> {
// ---------------------------------------------------------------------------
// --slug — replay one cookbook, print summary
// ---------------------------------------------------------------------------
if (slug) {
  const loaded = await loader.load(slug);
  const replay = replayCookbook(loaded);
  process.stdout.write(`${replay.slug}\n`);
  const parentTools =
    replay.parent.tool_count === -1
      ? "*"
      : `${replay.parent.tool_count} tools`;
  process.stdout.write(
    `  parent ${replay.parent.id.padEnd(36)} ${parentTools}  ` +
      `prompt=${replay.parent.system_prompt_sha256.slice(0, 12)}\n`,
  );
  for (const sub of replay.subagents) {
    const subTools =
      sub.tool_count === -1 ? "*" : `${sub.tool_count} tools`;
    process.stdout.write(
      `    sub ${sub.id.padEnd(38)} ${subTools}  ` +
        `prompt=${sub.system_prompt_sha256.slice(0, 12)}\n`,
    );
  }
  process.exit(0);
}

// ---------------------------------------------------------------------------
// --diff — compare current replay to a baseline JSON file
// ---------------------------------------------------------------------------
if (diffBaseline) {
  if (!existsSync(diffBaseline)) {
    process.stderr.write(`[cookbook-replay] baseline not found: ${diffBaseline}\n`);
    process.exit(1);
  }
  const previous = parseReplayCatalog(readFileSync(diffBaseline, "utf8"));
  const current = await replayAllCookbooks({ loader });
  const diff = diffReplayCatalogs(previous, current);
  process.stdout.write(JSON.stringify(diff, null, 2) + "\n");
  const dirty = diff.added.length + diff.removed.length + diff.changed.length;
  process.exit(dirty > 0 ? 1 : 0);
}

// ---------------------------------------------------------------------------
// write / check — generate the committed replay catalog
// ---------------------------------------------------------------------------

const catalog = await replayAllCookbooks({ loader });
const serialised = serialiseReplayCatalog(catalog);

if (mode === "check") {
  if (!existsSync(REPLAYS_PATH)) {
    process.stderr.write(
      `[cookbook-replay] data/cookbook-replays.json missing — run without --check to create it.\n`,
    );
    process.exit(1);
  }
  const onDisk = readFileSync(REPLAYS_PATH, "utf8");
  if (onDisk !== serialised) {
    process.stderr.write(
      `[cookbook-replay] drift detected — data/cookbook-replays.json is stale.\n`,
    );
    try {
      const previous = parseReplayCatalog(onDisk);
      const diff = diffReplayCatalogs(previous, catalog);
      if (diff.added.length > 0) {
        process.stderr.write(`  added:    ${diff.added.join(", ")}\n`);
      }
      if (diff.removed.length > 0) {
        process.stderr.write(`  removed:  ${diff.removed.join(", ")}\n`);
      }
      if (diff.changed.length > 0) {
        for (const c of diff.changed) {
          const parentFields = c.parent_changes
            .map((p) => String(p.field))
            .join(", ");
          const subSummary = c.subagent_changes
            .map((s) => `${s.id}(${s.changes.map((d) => String(d.field)).join(",")})`)
            .join("; ");
          process.stderr.write(
            `  changed:  ${c.slug}  parent[${parentFields || "—"}]  subs[${subSummary || "—"}]\n`,
          );
        }
      }
    } catch {
      // ignore — still actionable: regenerate
    }
    process.stderr.write(
      `  Regenerate with: npx tsx scripts/generate-cookbook-replays.ts\n`,
    );
    process.exit(1);
  }
  process.stdout.write(
    `[cookbook-replay] OK — replay up to date (${catalog.cookbooks.length} cookbooks).\n`,
  );
  process.exit(0);
}

mkdirSync(dirname(REPLAYS_PATH), { recursive: true });
writeFileSync(REPLAYS_PATH, serialised, "utf8");

process.stdout.write(
  `[cookbook-replay] wrote ${REPLAYS_PATH}\n  ${catalog.cookbooks.length} cookbooks replayed\n`,
);
for (const c of catalog.cookbooks) {
  const tc =
    c.parent.tool_count === -1
      ? "* tools"
      : `${String(c.parent.tool_count).padStart(3)} tools`;
  process.stdout.write(
    `  ${c.slug.padEnd(28)}  parent=${tc}  subs=${c.subagents.length}\n`,
  );
}
}

main().catch((err) => {
  process.stderr.write(`[cookbook-replay] error: ${err instanceof Error ? err.message : String(err)}\n`);
  process.exit(2);
});
