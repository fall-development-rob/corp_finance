#!/usr/bin/env tsx
/**
 * generate-cookbook-audits.ts — Phase 25 Tier A2 CLI runner.
 *
 * Walks managed-agent-cookbooks/ and emits data/cookbook-audits.json — a
 * byte-stable manifest of every cookbook's content hash + per-file inventory.
 *
 * Modes:
 *   write (default):  overwrite data/cookbook-audits.json
 *   --check:          regenerate in memory; non-zero exit on drift.
 *                     Used by CI to catch developers who change cookbook
 *                     content but forget to update the committed audit.
 *   --diff <path>:    diff current audit against <path> (e.g. main branch
 *                     baseline) and print added/removed/changed cookbooks.
 *   --slug <slug>:    audit only the named cookbook (writes nothing; prints
 *                     hash + file count to stdout). Useful for spot-checks.
 *
 * Usage:
 *   npx tsx scripts/generate-cookbook-audits.ts
 *   npx tsx scripts/generate-cookbook-audits.ts --check
 *   npx tsx scripts/generate-cookbook-audits.ts --slug equity-analyst
 */

import { readFileSync, writeFileSync, existsSync, mkdirSync } from "node:fs";
import { resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";

import {
  auditAllCookbooks,
  auditCookbook,
  serialiseAuditCatalog,
  parseAuditCatalog,
  diffAuditCatalogs,
} from "../packages/harness/src/manifests/cookbook-audit.js";

const __dirname = dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = resolve(__dirname, "..");
const COOKBOOKS_ROOT = resolve(REPO_ROOT, "managed-agent-cookbooks");
const AUDITS_PATH = resolve(REPO_ROOT, "data", "cookbook-audits.json");

const args = process.argv.slice(2);
const mode = args.includes("--check") ? "check" : "write";
const slugIdx = args.indexOf("--slug");
const slug = slugIdx >= 0 ? args[slugIdx + 1] : undefined;
const diffIdx = args.indexOf("--diff");
const diffBaseline = diffIdx >= 0 ? args[diffIdx + 1] : undefined;

// ---------------------------------------------------------------------------
// --slug — audit one cookbook, print to stdout
// ---------------------------------------------------------------------------
if (slug) {
  const dir = resolve(COOKBOOKS_ROOT, slug);
  if (!existsSync(dir)) {
    process.stderr.write(`[cookbook-audit] no such cookbook: ${slug}\n`);
    process.exit(1);
  }
  const audit = auditCookbook({
    repoRoot: REPO_ROOT,
    cookbookDir: dir,
    slug,
  });
  process.stdout.write(
    `${audit.slug}  ${audit.hash}  (${audit.files.length} files)\n`,
  );
  process.exit(0);
}

// ---------------------------------------------------------------------------
// --diff — compare current audit to a baseline JSON file
// ---------------------------------------------------------------------------
if (diffBaseline) {
  if (!existsSync(diffBaseline)) {
    process.stderr.write(`[cookbook-audit] baseline not found: ${diffBaseline}\n`);
    process.exit(1);
  }
  const previous = parseAuditCatalog(readFileSync(diffBaseline, "utf8"));
  const current = auditAllCookbooks({
    repoRoot: REPO_ROOT,
    cookbooksRoot: COOKBOOKS_ROOT,
  });
  const diff = diffAuditCatalogs(previous, current);
  process.stdout.write(JSON.stringify(diff, null, 2) + "\n");
  const dirty = diff.added.length + diff.removed.length + diff.changed.length;
  process.exit(dirty > 0 ? 1 : 0);
}

// ---------------------------------------------------------------------------
// write / check — generate the committed audit catalog
// ---------------------------------------------------------------------------

const catalog = auditAllCookbooks({
  repoRoot: REPO_ROOT,
  cookbooksRoot: COOKBOOKS_ROOT,
});
const serialised = serialiseAuditCatalog(catalog);

if (mode === "check") {
  if (!existsSync(AUDITS_PATH)) {
    process.stderr.write(
      `[cookbook-audit] data/cookbook-audits.json missing — run without --check to create it.\n`,
    );
    process.exit(1);
  }
  const onDisk = readFileSync(AUDITS_PATH, "utf8");
  if (onDisk !== serialised) {
    process.stderr.write(
      `[cookbook-audit] drift detected — data/cookbook-audits.json is stale.\n`,
    );
    // Show which cookbooks changed to make the failure actionable.
    try {
      const previous = parseAuditCatalog(onDisk);
      const diff = diffAuditCatalogs(previous, catalog);
      if (diff.added.length > 0) {
        process.stderr.write(`  added:    ${diff.added.join(", ")}\n`);
      }
      if (diff.removed.length > 0) {
        process.stderr.write(`  removed:  ${diff.removed.join(", ")}\n`);
      }
      if (diff.changed.length > 0) {
        process.stderr.write(
          `  changed:  ${diff.changed.map((c) => c.slug).join(", ")}\n`,
        );
      }
    } catch {
      // Parse failure on the on-disk file — still actionable: regenerate.
    }
    process.stderr.write(
      `  Regenerate with: npx tsx scripts/generate-cookbook-audits.ts\n`,
    );
    process.exit(1);
  }
  process.stdout.write(
    `[cookbook-audit] OK — audit up to date (${catalog.cookbooks.length} cookbooks).\n`,
  );
  process.exit(0);
}

mkdirSync(dirname(AUDITS_PATH), { recursive: true });
writeFileSync(AUDITS_PATH, serialised, "utf8");

process.stdout.write(
  `[cookbook-audit] wrote ${AUDITS_PATH}\n  ${catalog.cookbooks.length} cookbooks audited\n`,
);
for (const c of catalog.cookbooks) {
  process.stdout.write(
    `  ${c.slug.padEnd(28)}  ${c.hash.slice(0, 12)}  (${c.files.length} files)\n`,
  );
}
