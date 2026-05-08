#!/usr/bin/env node
/**
 * check-surface-parity.mjs
 *
 * Audits tool surface drift between packages/mcp-server (NAPI, ~90 tools) and
 * plugins/cfa-core/mcp (WASM, 6 tools).
 *
 * Categories:
 *   packages_only   — in NAPI, not in WASM, not in allowlist  → SILENT DRIFT (exit 1)
 *   allowlisted     — in NAPI, not in WASM, but explicitly allowlisted  → OK
 *   plugin_only     — in WASM, not in NAPI  → UNUSUAL (warn, but exit 0)
 *   shared          — in both  → PARITY GOOD
 *
 * Exit codes:
 *   0  — no silent drift detected
 *   1  — one or more packages_only tools exist (subagents will silently miss them)
 */

import { readFileSync } from "node:fs";
import { resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { extractPackagesTools, extractPluginTools } from "./lib/tool-extract.mjs";

const __dirname = dirname(fileURLToPath(import.meta.url));
// packages/mcp-server/scripts/ → up three levels to workspace root
const WORKSPACE_ROOT = resolve(__dirname, "../../..");
const ALLOWLIST_PATH = resolve(__dirname, "../.surface-allowlist.json");

function loadAllowlist() {
  const raw = JSON.parse(readFileSync(ALLOWLIST_PATH, "utf8"));
  return new Set(raw.napi_only.map((e) => e.tool));
}

function printRow(label, count, detail) {
  const padLabel = label.padEnd(20);
  const padCount = String(count).padStart(5);
  process.stdout.write(`  ${padLabel}  ${padCount}  ${detail}\n`);
}

function printSeparator() {
  process.stdout.write(`  ${"─".repeat(70)}\n`);
}

function main() {
  const packageTools = extractPackagesTools(WORKSPACE_ROOT);
  const pluginTools = extractPluginTools(WORKSPACE_ROOT);
  const allowlist = loadAllowlist();

  const packageNames = new Set(packageTools.map((t) => t.tool_name));
  const pluginNames = new Set(pluginTools.map((t) => t.tool_name));

  const packagesOnly = packageTools
    .filter((t) => !pluginNames.has(t.tool_name) && !allowlist.has(t.tool_name))
    .sort((a, b) => a.tool_name.localeCompare(b.tool_name));

  const allowlisted = packageTools
    .filter((t) => !pluginNames.has(t.tool_name) && allowlist.has(t.tool_name))
    .sort((a, b) => a.tool_name.localeCompare(b.tool_name));

  const pluginOnly = pluginTools
    .filter((t) => !packageNames.has(t.tool_name))
    .sort((a, b) => a.tool_name.localeCompare(b.tool_name));

  const shared = pluginTools
    .filter((t) => packageNames.has(t.tool_name))
    .sort((a, b) => a.tool_name.localeCompare(b.tool_name));

  process.stdout.write("\n");
  process.stdout.write("  Surface Parity Audit — packages/mcp-server vs plugins/cfa-core/mcp\n");
  process.stdout.write(`  Workspace: ${WORKSPACE_ROOT}\n`);
  process.stdout.write("\n");
  printSeparator();
  process.stdout.write(`  ${"Category".padEnd(20)}  ${"Count".padStart(5)}  Notes\n`);
  printSeparator();
  printRow("packages total", packageTools.length, "tools registered in NAPI server");
  printRow("plugin total", pluginTools.length, "tools registered in WASM plugin");
  printSeparator();
  printRow("shared", shared.length, "in both — parity good");
  printRow("allowlisted", allowlisted.length, "NAPI-only by design (native deps / untested WASM)");
  printRow("plugin_only", pluginOnly.length, pluginOnly.length ? "UNUSUAL — in WASM but not NAPI" : "none");
  printRow(
    "packages_only",
    packagesOnly.length,
    packagesOnly.length ? "SILENT DRIFT — subagents miss these" : "none — no drift detected",
  );
  printSeparator();

  if (shared.length > 0) {
    process.stdout.write("\n  Shared tools:\n");
    for (const t of shared) {
      process.stdout.write(`    + ${t.tool_name}  (${t.source_file})\n`);
    }
  }

  if (pluginOnly.length > 0) {
    process.stdout.write("\n  [WARN] Plugin-only tools (unusual — in WASM but not NAPI):\n");
    for (const t of pluginOnly) {
      process.stdout.write(`    ? ${t.tool_name}  (${t.source_file})\n`);
    }
  }

  if (packagesOnly.length > 0) {
    process.stdout.write("\n  [FAIL] Silent drift — packages_only tools not in plugin or allowlist:\n");
    for (const t of packagesOnly) {
      process.stdout.write(`    ! ${t.tool_name}  (${t.source_file})\n`);
    }
    process.stdout.write(
      "\n  Action required: either port these tools to plugins/cfa-core/mcp (Wave 16)\n" +
        "  or add them to packages/mcp-server/.surface-allowlist.json with a reason.\n",
    );
    process.stdout.write("\n");
    process.exit(1);
  }

  process.stdout.write("\n  Result: PASS — no silent drift detected.\n\n");
  process.exit(0);
}

main();
