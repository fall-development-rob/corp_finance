/**
 * skill-editor CLI subcommand handler — Phase 41 Wave 3.
 *
 * Subcommands:
 *   analyse  --window <Nd>  --output <dir>  [--bucket <s3-uri>]
 *   apply    <file.yaml>    [--dry-run]
 *   apply-all <dir>         [--dry-run]
 */
import { readdir } from "node:fs/promises";
import { resolve, join } from "node:path";
import { detectAllOutliers } from "../reasoning/outliers.js";
import { emitRemediations, applyRemediation } from "../reasoning/remediation.js";
import { createInMemoryBank } from "../reasoning/skill-editor-bank.js";

// ---------------------------------------------------------------------------
// Arg helpers
// ---------------------------------------------------------------------------

function parseFlag(args: string[], flag: string): string | undefined {
  const idx = args.indexOf(flag);
  if (idx === -1) return undefined;
  return args[idx + 1];
}

function hasFlag(args: string[], flag: string): boolean {
  return args.includes(flag);
}

function parseWindowMs(window: string): number {
  const match = /^(\d+)(d|h|m)$/.exec(window);
  if (!match) throw new Error(`Invalid window format "${window}". Use e.g. 7d, 24h, 60m.`);
  const n = parseInt(match[1]!, 10);
  switch (match[2]) {
    case "d": return n * 24 * 60 * 60 * 1000;
    case "h": return n * 60 * 60 * 1000;
    case "m": return n * 60 * 1000;
    default: return 7 * 24 * 60 * 60 * 1000;
  }
}

// ---------------------------------------------------------------------------
// Subcommand: analyse
// ---------------------------------------------------------------------------

async function runAnalyse(args: string[]): Promise<void> {
  const windowStr = parseFlag(args, "--window") ?? "7d";
  const outputDir = parseFlag(args, "--output") ?? "docs/proposed-skill-updates";
  const bucket = parseFlag(args, "--bucket");
  const repoRoot = resolve(process.cwd());

  const windowMs = parseWindowMs(windowStr);
  const to = new Date();
  const from = new Date(to.getTime() - windowMs);

  process.stderr.write(
    `[skill-editor] analyse window=${windowStr} output=${outputDir}` +
      (bucket ? ` bucket=${bucket}` : "") +
      "\n",
  );

  // Build a reasoning bank (S3 from --bucket or local in-memory fallback).
  // Parallel D handles the real S3 bank wiring; here we use a stub that
  // supports the ReasoningBank interface and can be swapped later.
  const bank = await createInMemoryBank({ bucket, repoRoot });

  const report = await detectAllOutliers({
    bank,
    window: { from, to },
    minClusterSize: 3,
  });

  const emitted = await emitRemediations({
    report,
    outputDir: resolve(repoRoot, outputDir),
  });

  process.stdout.write(
    `Wrote ${emitted.length} remediation file(s) to ${outputDir}\n`,
  );
  for (const e of emitted) {
    process.stdout.write(`  ${e.filePath}\n`);
  }
}

// ---------------------------------------------------------------------------
// Subcommand: apply
// ---------------------------------------------------------------------------

async function runApply(args: string[]): Promise<void> {
  const dryRun = hasFlag(args, "--dry-run");
  const filePath = args.find((a) => !a.startsWith("--"));

  if (!filePath) {
    process.stderr.write("[error] apply requires a <file.yaml> argument\n");
    printSkillEditorUsage();
    process.exit(1);
  }

  const repoRoot = resolve(process.cwd());
  const remediationFilePath = resolve(filePath);

  try {
    const result = await applyRemediation({ remediationFilePath, repoRoot, dryRun });
    if (result.applied) {
      process.stdout.write(`Applied: ${result.affectedFile}\n`);
    } else {
      process.stdout.write(
        `No-op: ${result.affectedFile || remediationFilePath} — ${result.reason ?? "no-op"}\n`,
      );
    }
  } catch (err) {
    process.stderr.write(`[error] ${(err as Error).message}\n`);
    process.exit(1);
  }
}

// ---------------------------------------------------------------------------
// Subcommand: apply-all
// ---------------------------------------------------------------------------

async function runApplyAll(args: string[]): Promise<void> {
  const dryRun = hasFlag(args, "--dry-run");
  const dir = args.find((a) => !a.startsWith("--"));

  if (!dir) {
    process.stderr.write("[error] apply-all requires a <dir> argument\n");
    printSkillEditorUsage();
    process.exit(1);
  }

  const repoRoot = resolve(process.cwd());
  const absDir = resolve(dir);

  let files: string[];
  try {
    const entries = await readdir(absDir);
    files = entries.filter((f) => f.endsWith(".yaml")).map((f) => join(absDir, f));
  } catch (err) {
    process.stderr.write(`[error] Cannot read directory ${absDir}: ${(err as Error).message}\n`);
    process.exit(1);
  }

  if (files.length === 0) {
    process.stdout.write(`No *.yaml files found in ${dir}\n`);
    return;
  }

  let applied = 0;
  let noOp = 0;
  let errors = 0;

  for (const filePath of files) {
    try {
      const result = await applyRemediation({ remediationFilePath: filePath, repoRoot, dryRun });
      if (result.applied) {
        applied++;
        process.stdout.write(`  applied: ${filePath}\n`);
      } else {
        noOp++;
        process.stdout.write(`  no-op:   ${filePath} (${result.reason})\n`);
      }
    } catch (err) {
      errors++;
      process.stderr.write(`  error:   ${filePath}: ${(err as Error).message}\n`);
    }
  }

  process.stdout.write(
    `\nSummary: ${applied} applied, ${noOp} no-op, ${errors} errors\n`,
  );

  if (errors > 0) process.exit(1);
}

// ---------------------------------------------------------------------------
// Usage
// ---------------------------------------------------------------------------

export function printSkillEditorUsage(): void {
  process.stderr.write(`\
Usage: cfa-harness skill-editor <subcommand> [options]

Subcommands:
  analyse                     Detect outliers and emit remediation YAML files
    --window <Nd|Nh|Nm>       Analysis window (default: 7d)
    --bucket <s3-uri>         S3 bucket URI for remote bank (env: CFA_BANK_BACKEND)
    --output <dir>            Output directory (default: docs/proposed-skill-updates)

  apply <file.yaml>           Apply a single remediation file
    --dry-run                 Print diff instead of writing

  apply-all <dir>             Apply all *.yaml files in a directory
    --dry-run                 Print diff instead of writing

  --help, -h                  Print this message and exit

Examples:
  cfa-harness skill-editor analyse --window 7d --output docs/proposed-skill-updates
  cfa-harness skill-editor apply docs/proposed-skill-updates/2026-05-11-cluster-abc12345.yaml
  cfa-harness skill-editor apply-all docs/proposed-skill-updates/ --dry-run
`);
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

export async function runSkillEditorSubcommand(args: string[]): Promise<void> {
  const sub = args[0];

  if (!sub || sub === "--help" || sub === "-h") {
    printSkillEditorUsage();
    process.exit(0);
  }

  switch (sub) {
    case "analyse":
      await runAnalyse(args.slice(1));
      break;
    case "apply":
      await runApply(args.slice(1));
      break;
    case "apply-all":
      await runApplyAll(args.slice(1));
      break;
    default:
      process.stderr.write(`[error] Unknown skill-editor subcommand: ${sub}\n`);
      printSkillEditorUsage();
      process.exit(1);
  }
}
