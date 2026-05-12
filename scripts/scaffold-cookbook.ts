#!/usr/bin/env tsx
/**
 * scaffold-cookbook.ts — Phase 25 Tier D14 CLI runner.
 *
 * Scaffolds a minimal-conformant managed-agent cookbook skeleton that
 * passes every MA-* contract out of the box. Authors then wire in domain
 * logic (skills, specific tool selections, richer output schemas).
 *
 * Usage:
 *   npx tsx scripts/scaffold-cookbook.ts --slug my-new-cookbook
 *   npx tsx scripts/scaffold-cookbook.ts --slug equity-screen --domain "equity research"
 *   npx tsx scripts/scaffold-cookbook.ts --slug demo --dry-run    # preview only
 *
 * Exit codes:
 *   0  scaffold written (or dry-run completed)
 *   1  invalid slug, target directory already exists, or missing --slug
 */

import { existsSync, mkdirSync, writeFileSync } from "node:fs";
import { resolve, dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import {
  buildScaffoldedCookbook,
  ScaffoldError,
} from "../packages/harness/src/manifests/cookbook-scaffold.js";

const __dirname = dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = resolve(__dirname, "..");
const COOKBOOKS_ROOT = resolve(REPO_ROOT, "managed-agent-cookbooks");

const args = process.argv.slice(2);

function flagValue(name: string): string | undefined {
  const idx = args.indexOf(name);
  if (idx < 0) return undefined;
  return args[idx + 1];
}

const slug = flagValue("--slug");
const domain = flagValue("--domain");
const dryRun = args.includes("--dry-run");

if (!slug) {
  process.stderr.write(
    "[scaffold-cookbook] missing --slug <name>\n" +
      "  example: npx tsx scripts/scaffold-cookbook.ts --slug my-cookbook --domain equity\n",
  );
  process.exit(1);
}

let scaffolded;
try {
  scaffolded = buildScaffoldedCookbook({ slug, domain });
} catch (err) {
  if (err instanceof ScaffoldError) {
    process.stderr.write(`[scaffold-cookbook] ${err.message}\n`);
    process.exit(1);
  }
  throw err;
}

const targetDir = join(COOKBOOKS_ROOT, slug);

if (dryRun) {
  process.stdout.write(`[scaffold-cookbook] DRY RUN — would create ${targetDir}\n`);
  for (const f of scaffolded.files) {
    process.stdout.write(`  + ${slug}/${f.relPath}  (${f.contents.length} bytes)\n`);
  }
  process.exit(0);
}

if (existsSync(targetDir)) {
  process.stderr.write(
    `[scaffold-cookbook] target already exists: ${targetDir}\n` +
      `  delete it first if you want to regenerate.\n`,
  );
  process.exit(1);
}

mkdirSync(targetDir, { recursive: true });
mkdirSync(join(targetDir, "subagents"), { recursive: true });

for (const f of scaffolded.files) {
  const path = join(targetDir, f.relPath);
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, f.contents, "utf8");
}

process.stdout.write(
  `[scaffold-cookbook] wrote ${scaffolded.files.length} files under ${targetDir}\n\n`,
);
for (const f of scaffolded.files) {
  process.stdout.write(`  ${f.relPath}\n`);
}

process.stdout.write(
  `\nNext steps:\n` +
    `  1. Edit agent.yaml + subagents/*.yaml — wire in your real skills (from_plugin paths),\n` +
    `     model tool selections (configs[].name), and richer output_schemas.\n` +
    `  2. Edit steering-examples.json — add 2-5 representative inputs for the dispatcher.\n` +
    `  3. Regenerate the catalogs:\n` +
    `       npx tsx scripts/generate-cookbook-audits.ts\n` +
    `       npx tsx scripts/generate-cookbook-replays.ts\n` +
    `       npx tsx scripts/generate-cookbook-costs.ts\n` +
    `       npx tsx scripts/generate-cookbook-traces.ts\n` +
    `  4. Run the contract tests:\n` +
    `       (cd packages/harness && npx vitest run tests/contracts/)\n` +
    `  5. Run the cookbook tool-name lint:\n` +
    `       npx tsx scripts/lint-cookbook-tool-names.ts\n` +
    `\n` +
    `  All 8 MA-* contracts should pass on the scaffold as-is. Bumps to ` +
    `cookbook count\n` +
    `  invariants (MA-INV-001/002) will be expected — update those expectations or remove\n` +
    `  the prior count if you are sure.\n`,
);
