#!/usr/bin/env tsx
/**
 * generate-tool-catalog.ts — Phase 25 Tier A1 CLI runner.
 *
 * Scans every in-repo MCP server (cfa-core via plugins/cfa-core/mcp,
 * fmp/data/vendor via packages/*-mcp-server) and emits a canonical JSON
 * catalog at data/tools-catalog.json. Two modes:
 *
 *   - write (default): overwrite data/tools-catalog.json
 *   - check (--check): regenerate in memory and diff against the committed
 *                      file; non-zero exit on drift. Used by CI.
 *
 * Usage:
 *   npx tsx scripts/generate-tool-catalog.ts
 *   npx tsx scripts/generate-tool-catalog.ts --check
 */

import { readFileSync, writeFileSync, existsSync, mkdirSync } from "node:fs";
import { resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";

import {
  generateToolCatalog,
  serialiseCatalog,
} from "../packages/harness/src/manifests/tool-catalog.js";

const __dirname = dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = resolve(__dirname, "..");
const CATALOG_PATH = resolve(REPO_ROOT, "data", "tools-catalog.json");

const mode = process.argv.includes("--check") ? "check" : "write";

const catalog = generateToolCatalog({ repoRoot: REPO_ROOT });
const serialised = serialiseCatalog(catalog);

if (mode === "check") {
  if (!existsSync(CATALOG_PATH)) {
    process.stderr.write(
      `[generate-tool-catalog] data/tools-catalog.json missing — run without --check to create it.\n`,
    );
    process.exit(1);
  }
  const onDisk = readFileSync(CATALOG_PATH, "utf8");
  if (onDisk !== serialised) {
    process.stderr.write(
      `[generate-tool-catalog] drift detected — data/tools-catalog.json is stale.\n`,
    );
    process.stderr.write(`  Regenerate with: npx tsx scripts/generate-tool-catalog.ts\n`);
    process.exit(1);
  }
  const total = Object.values(catalog.servers).reduce((n, arr) => n + arr.length, 0);
  process.stdout.write(
    `[generate-tool-catalog] OK — catalog up to date (${total} tools across ${Object.keys(catalog.servers).length} servers).\n`,
  );
  process.exit(0);
}

mkdirSync(dirname(CATALOG_PATH), { recursive: true });
writeFileSync(CATALOG_PATH, serialised, "utf8");

const counts = Object.entries(catalog.servers)
  .map(([s, t]) => `${s}=${t.length}`)
  .join(", ");
process.stdout.write(
  `[generate-tool-catalog] wrote ${CATALOG_PATH}\n  servers: ${counts}\n`,
);
