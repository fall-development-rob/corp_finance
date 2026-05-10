#!/usr/bin/env tsx
/**
 * check-manifests.ts — Phase 33 REC-3 CLI runner.
 *
 * Static linter for all agent manifests (YAML + legacy JSON).
 * Verifies that every cross-file reference in every manifest resolves to an
 * existing path and flags schema-shape / drift issues.
 *
 * Run in < 1 second; suitable for pre-commit / pre-PR CI.
 *
 * Usage:
 *   npx tsx scripts/check-manifests.ts
 *   npx tsx scripts/check-manifests.ts --strict
 */

import { resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { checkManifests, type ManifestCheckIssue } from "../packages/harness/src/manifests/checker.js";

const __dirname = dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = resolve(__dirname, "..");

// ANSI colours
const RED    = "\x1b[31m";
const YELLOW = "\x1b[33m";
const BLUE   = "\x1b[34m";
const GREEN  = "\x1b[32m";
const BOLD   = "\x1b[1m";
const RESET  = "\x1b[0m";

const strict = process.argv.includes("--strict");

function severityColour(sev: ManifestCheckIssue["severity"]): string {
  if (sev === "error")   return RED;
  if (sev === "warning") return YELLOW;
  return BLUE;
}

function severityLabel(sev: ManifestCheckIssue["severity"]): string {
  if (sev === "error")   return "ERROR  ";
  if (sev === "warning") return "WARN   ";
  return "INFO   ";
}

function printIssue(issue: ManifestCheckIssue): void {
  const col   = severityColour(issue.severity);
  const label = severityLabel(issue.severity);
  console.log(`${col}${BOLD}${label}${RESET} [${issue.rule}] ${issue.manifest}`);
  console.log(`         path: ${issue.path}`);
  console.log(`         ${issue.message}`);
  if (issue.suggestion) {
    console.log(`         hint: ${issue.suggestion}`);
  }
}

async function main(): Promise<void> {
  console.log(`${BOLD}CFA manifest linter${RESET} — scanning…\n`);

  const report = await checkManifests({ repoRoot: REPO_ROOT, strict });

  const allIssues = [
    ...report.errors,
    ...report.warnings,
    ...report.info,
  ];

  for (const issue of allIssues) {
    printIssue(issue);
    console.log();
  }

  const { manifests_checked, errors, warnings, info } = report;

  console.log("─".repeat(60));
  console.log(
    `Checked ${manifests_checked} manifests — ` +
    `${RED}${errors.length} error(s)${RESET}  ` +
    `${YELLOW}${warnings.length} warning(s)${RESET}  ` +
    `${BLUE}${info.length} info${RESET}`,
  );

  if (report.ok) {
    const qualifier = strict ? " (strict mode)" : "";
    console.log(`\n${GREEN}${BOLD}OK${RESET}${GREEN} — all manifests clean${qualifier}${RESET}`);
    process.exit(0);
  } else {
    const reason = strict && errors.length === 0
      ? "warnings treated as errors in --strict mode"
      : `${errors.length} error(s) found`;
    console.log(`\n${RED}${BOLD}FAIL${RESET}${RED} — ${reason}${RESET}`);
    process.exit(1);
  }
}

main().catch((err: unknown) => {
  console.error(`${RED}Fatal error: ${String(err)}${RESET}`);
  process.exit(2);
});
