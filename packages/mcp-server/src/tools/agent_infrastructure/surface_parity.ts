/**
 * surface_parity.ts
 *
 * Phase 30 Wave 1 — Port of check-surface-parity.mjs + lib/tool-extract.mjs
 * into a typed, agent-callable MCP tool.
 *
 * Categories:
 *   packages_only — in NAPI, not in WASM, not in allowlist → SILENT DRIFT
 *   allowlisted   — in NAPI, not in WASM, but explicitly allowlisted → OK
 *   plugin_only   — in WASM, not in NAPI → UNUSUAL
 *   shared        — in both → PARITY GOOD
 */

import { readFileSync, readdirSync } from "node:fs";
import { join, relative } from "node:path";

// ---------------------------------------------------------------------------
// Public interfaces
// ---------------------------------------------------------------------------

export interface ToolEntry {
  tool_name: string;
  source_file: string; // relative to workspace root
  description: string;
}

export interface SurfaceParityInput {
  workspace_root?: string;   // defaults to auto-detected (walk up looking for .git)
  allowlist_path?: string;   // defaults to packages/mcp-server/.surface-allowlist.json
}

export interface SurfaceParityReport {
  workspace_root: string;
  packages_total: number;
  plugin_total: number;
  shared: ToolEntry[];
  allowlisted: ToolEntry[];
  plugin_only: ToolEntry[];
  packages_only: ToolEntry[];
  drift_detected: boolean;   // true iff packages_only.length > 0
}

// ---------------------------------------------------------------------------
// Regex patterns — preserved exactly from lib/tool-extract.mjs
// ---------------------------------------------------------------------------

const NAPI_CALL_RE = /^\s*server\.tool\s*\(/;
const PLUGIN_CALL_RE = /^\s*tool\s*\(\s*$/;
const PLUGIN_SERVER_ARG_RE = /^\s*server\s*,?\s*$/;
const TOOL_NAME_RE = /^\s*"([^"]+)"/;
const TOOL_DESC_RE = /^\s*"((?:[^"\\]|\\.)*)"/;
const PLUGIN_INLINE_RE = /^\s*tool\s*\(\s*server\s*,\s*"([^"]+)"\s*,\s*"((?:[^"\\]|\\.)*)"\s*,/;

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

function parseNapiPattern(lines: string[], relPath: string): ToolEntry[] {
  const results: ToolEntry[] = [];
  for (let i = 0; i < lines.length; i++) {
    if (!NAPI_CALL_RE.test(lines[i]!)) continue;
    const nameLine = lines[i + 1] ?? "";
    const nameMatch = TOOL_NAME_RE.exec(nameLine);
    if (!nameMatch) continue;
    const descLine = lines[i + 2] ?? "";
    const descMatch = TOOL_DESC_RE.exec(descLine);
    results.push({
      tool_name: nameMatch[1]!,
      source_file: relPath,
      description: descMatch ? descMatch[1]! : "",
    });
  }
  return results;
}

function parsePluginPattern(lines: string[], relPath: string): ToolEntry[] {
  const results: ToolEntry[] = [];
  for (let i = 0; i < lines.length; i++) {
    const inlineMatch = PLUGIN_INLINE_RE.exec(lines[i]!);
    if (inlineMatch) {
      results.push({
        tool_name: inlineMatch[1]!,
        source_file: relPath,
        description: inlineMatch[2]!,
      });
      continue;
    }
    if (!PLUGIN_CALL_RE.test(lines[i]!)) continue;
    const serverLine = lines[i + 1] ?? "";
    if (!PLUGIN_SERVER_ARG_RE.test(serverLine)) continue;
    const nameLine = lines[i + 2] ?? "";
    const nameMatch = TOOL_NAME_RE.exec(nameLine);
    if (!nameMatch) continue;
    const descLine = lines[i + 3] ?? "";
    const descMatch = TOOL_DESC_RE.exec(descLine);
    results.push({
      tool_name: nameMatch[1]!,
      source_file: relPath,
      description: descMatch ? descMatch[1]! : "",
    });
  }
  return results;
}

function parseFile(
  filePath: string,
  workspaceRoot: string,
  strategy: "napi" | "plugin",
): ToolEntry[] {
  const lines = readFileSync(filePath, "utf8").split("\n");
  const relPath = relative(workspaceRoot, filePath);
  return strategy === "plugin"
    ? parsePluginPattern(lines, relPath)
    : parseNapiPattern(lines, relPath);
}

function collectTs(dir: string): string[] {
  const entries = readdirSync(dir, { withFileTypes: true });
  const files: string[] = [];
  for (const entry of entries) {
    const full = join(dir, entry.name);
    if (entry.isDirectory()) {
      files.push(...collectTs(full));
    } else if (entry.isFile() && entry.name.endsWith(".ts")) {
      files.push(full);
    }
  }
  return files.sort();
}

function detectWorkspaceRoot(): string {
  // Walk up from cwd looking for .git
  let dir = process.cwd();
  for (let i = 0; i < 20; i++) {
    try {
      readdirSync(join(dir, ".git"));
      return dir;
    } catch {
      const parent = join(dir, "..");
      if (parent === dir) break;
      dir = parent;
    }
  }
  return process.cwd();
}

function loadAllowlist(allowlistPath: string): Set<string> {
  interface AllowlistFile {
    napi_only: Array<{ tool: string; reason: string; owning_module: string }>;
  }
  const raw = JSON.parse(readFileSync(allowlistPath, "utf8")) as AllowlistFile;
  return new Set(raw.napi_only.map((e) => e.tool));
}

function sortByName(arr: ToolEntry[]): ToolEntry[] {
  return arr.slice().sort((a, b) => a.tool_name.localeCompare(b.tool_name));
}

// ---------------------------------------------------------------------------
// Exported extraction functions
// ---------------------------------------------------------------------------

export function extractPackagesTools(workspaceRoot: string): ToolEntry[] {
  const toolsDir = join(workspaceRoot, "packages/mcp-server/src/tools");
  const files = collectTs(toolsDir);
  return files.flatMap((f) => parseFile(f, workspaceRoot, "napi"));
}

export function extractPluginTools(workspaceRoot: string): ToolEntry[] {
  const serverFile = join(workspaceRoot, "plugins/cfa-core/mcp/src/server.ts");
  return parseFile(serverFile, workspaceRoot, "plugin");
}

// ---------------------------------------------------------------------------
// Top-level orchestration
// ---------------------------------------------------------------------------

export function surfaceParityCheck(input: SurfaceParityInput): SurfaceParityReport {
  const workspaceRoot = input.workspace_root ?? detectWorkspaceRoot();
  const allowlistPath =
    input.allowlist_path ??
    join(workspaceRoot, "packages/mcp-server/.surface-allowlist.json");

  const packageTools = extractPackagesTools(workspaceRoot);
  const pluginTools = extractPluginTools(workspaceRoot);
  const allowlist = loadAllowlist(allowlistPath);

  const packageNames = new Set(packageTools.map((t) => t.tool_name));
  const pluginNames = new Set(pluginTools.map((t) => t.tool_name));

  const packagesOnly = sortByName(
    packageTools.filter((t) => !pluginNames.has(t.tool_name) && !allowlist.has(t.tool_name)),
  );
  const allowlisted = sortByName(
    packageTools.filter((t) => !pluginNames.has(t.tool_name) && allowlist.has(t.tool_name)),
  );
  const pluginOnly = sortByName(
    pluginTools.filter((t) => !packageNames.has(t.tool_name)),
  );
  const shared = sortByName(
    pluginTools.filter((t) => packageNames.has(t.tool_name)),
  );

  return {
    workspace_root: workspaceRoot,
    packages_total: packageTools.length,
    plugin_total: pluginTools.length,
    shared,
    allowlisted,
    plugin_only: pluginOnly,
    packages_only: packagesOnly,
    drift_detected: packagesOnly.length > 0,
  };
}

// ---------------------------------------------------------------------------
// MCP wrapper — accepts JSON string, returns JSON string
// ---------------------------------------------------------------------------

export async function surfaceParityCheckMcp(inputJson: string): Promise<string> {
  const input = JSON.parse(inputJson) as SurfaceParityInput;
  const report = surfaceParityCheck(input);
  return JSON.stringify(report);
}

// CLI invocation: `tsx surface_parity.ts` — same code path as the MCP tool.
// Prints a human summary to stdout, exits non-zero on drift_detected.
if (import.meta.url === `file://${process.argv[1]}`) {
  const report = surfaceParityCheck({});
  process.stdout.write("\n  Surface Parity Audit — packages/mcp-server vs plugins/cfa-core/mcp\n");
  process.stdout.write(`  Workspace: ${report.workspace_root}\n\n`);
  process.stdout.write(`  packages total : ${report.packages_total}\n`);
  process.stdout.write(`  plugin total   : ${report.plugin_total}\n`);
  process.stdout.write(`  shared         : ${report.shared.length}\n`);
  process.stdout.write(`  allowlisted    : ${report.allowlisted.length}\n`);
  process.stdout.write(`  plugin_only    : ${report.plugin_only.length}\n`);
  process.stdout.write(`  packages_only  : ${report.packages_only.length}${report.drift_detected ? "  [FAIL]" : ""}\n\n`);
  if (report.drift_detected) {
    process.stdout.write("  [FAIL] Silent drift — packages_only tools not in plugin or allowlist:\n");
    for (const t of report.packages_only) {
      process.stdout.write(`    ! ${t.tool_name}  (${t.source_file})\n`);
    }
    process.stdout.write(
      "\n  Action: port these tools to plugins/cfa-core/mcp or add them to .surface-allowlist.json with a reason.\n\n",
    );
    process.exit(1);
  }
  process.stdout.write("  Result: PASS — no silent drift detected.\n\n");
  process.exit(0);
}
