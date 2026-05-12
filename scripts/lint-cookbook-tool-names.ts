#!/usr/bin/env tsx
/**
 * lint-cookbook-tool-names.ts — Phase 25 Tier A1 CLI runner.
 *
 * Reads data/tools-catalog.json and lints every managed-agent cookbook
 * (parent agent.yaml + subagents/*.yaml) for tool names that reference a
 * non-existent MCP tool. Catches the tool-name drift bug class.
 *
 * Usage:
 *   npx tsx scripts/lint-cookbook-tool-names.ts
 *   npx tsx scripts/lint-cookbook-tool-names.ts --json     # machine output
 */

import { readFileSync, existsSync } from "node:fs";
import { resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";

import {
  lintCookbookToolNames,
  parseCatalog,
} from "../packages/harness/src/manifests/tool-catalog.js";

const __dirname = dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = resolve(__dirname, "..");
const CATALOG_PATH = resolve(REPO_ROOT, "data", "tools-catalog.json");
const COOKBOOKS_ROOT = resolve(REPO_ROOT, "managed-agent-cookbooks");

const jsonOutput = process.argv.includes("--json");

if (!existsSync(CATALOG_PATH)) {
  process.stderr.write(
    `[lint-cookbook-tool-names] missing ${CATALOG_PATH}\n` +
      `  Run: npx tsx scripts/generate-tool-catalog.ts\n`,
  );
  process.exit(2);
}

const catalog = parseCatalog(readFileSync(CATALOG_PATH, "utf8"));
const report = lintCookbookToolNames({
  catalog,
  cookbooksRoot: COOKBOOKS_ROOT,
  repoRoot: REPO_ROOT,
});

if (jsonOutput) {
  process.stdout.write(JSON.stringify(report, null, 2) + "\n");
  process.exit(report.issues.length > 0 ? 1 : 0);
}

// ANSI colours
const RED = "\x1b[31m";
const YELLOW = "\x1b[33m";
const GREEN = "\x1b[32m";
const DIM = "\x1b[2m";
const BOLD = "\x1b[1m";
const RESET = "\x1b[0m";

process.stdout.write(
  `\n  Cookbook Tool-Name Lint — Phase 25 Tier A1\n` +
    `  ${DIM}catalog: ${CATALOG_PATH}${RESET}\n\n`,
);
process.stdout.write(`  cookbooks scanned : ${report.cookbooks_scanned}\n`);
process.stdout.write(`  manifests scanned : ${report.files_scanned}\n`);
process.stdout.write(`  configs checked   : ${report.configs_checked}\n`);
process.stdout.write(
  `  issues            : ${report.issues.length > 0 ? RED : GREEN}${report.issues.length}${RESET}\n\n`,
);

if (report.issues.length === 0) {
  process.stdout.write(`  ${GREEN}PASS${RESET} — no tool-name drift detected.\n\n`);
  process.exit(0);
}

const byCookbook = new Map<string, typeof report.issues>();
for (const issue of report.issues) {
  if (!byCookbook.has(issue.cookbook)) byCookbook.set(issue.cookbook, []);
  byCookbook.get(issue.cookbook)!.push(issue);
}

for (const [cookbook, issues] of byCookbook) {
  process.stdout.write(`  ${BOLD}${cookbook}${RESET}  (${issues.length} issue${issues.length === 1 ? "" : "s"})\n`);
  for (const issue of issues) {
    const colour = issue.reason === "unknown_tool" ? RED : YELLOW;
    process.stdout.write(`    ${colour}${issue.reason}${RESET}  ${issue.tool_name_raw}\n`);
    process.stdout.write(`      ${DIM}${issue.manifest}${RESET}\n`);
    process.stdout.write(`      ${issue.message}\n`);
  }
  process.stdout.write("\n");
}

process.stdout.write(`  ${RED}FAIL${RESET} — fix tool-name drift above.\n\n`);
process.exit(1);
