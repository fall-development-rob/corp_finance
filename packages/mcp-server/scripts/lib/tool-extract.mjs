/**
 * tool-extract.mjs
 *
 * Extracts tool registrations from TypeScript source files.
 * Used by check-surface-parity.mjs to detect drift between packages/mcp-server
 * (NAPI) and plugins/cfa-core/mcp (WASM).
 *
 * Two registration patterns are supported:
 *
 *   NAPI pattern (packages/mcp-server):
 *     server.tool(
 *       "tool_name",        ← name on line i+1
 *       "description",      ← description on line i+2
 *       ...
 *
 *   Plugin pattern (plugins/cfa-core/mcp/src/server.ts):
 *     tool(
 *       server,             ← line i+1 contains `server`
 *       "tool_name",        ← name on line i+2
 *       "description",      ← description on line i+3
 *       ...
 *
 * Both patterns are stable and verified against all source files in this repo.
 */

import { readFileSync, readdirSync } from "node:fs";
import { join, relative } from "node:path";

/**
 * @typedef {{ tool_name: string, source_file: string, description: string }} ToolEntry
 */

const NAPI_CALL_RE = /^\s*server\.tool\s*\(/;
const PLUGIN_CALL_RE = /^\s*tool\s*\(\s*$/;
const PLUGIN_SERVER_ARG_RE = /^\s*server\s*,?\s*$/;
const TOOL_NAME_RE = /^\s*"([^"]+)"/;
const TOOL_DESC_RE = /^\s*"((?:[^"\\]|\\.)*)"/;
// Single-line plugin form: `tool(server, "tool_name", "description", wasm.fn);`
const PLUGIN_INLINE_RE = /^\s*tool\s*\(\s*server\s*,\s*"([^"]+)"\s*,\s*"((?:[^"\\]|\\.)*)"\s*,/;

/**
 * Parse a single file for NAPI-style server.tool("name", ...) registrations.
 * @param {string[]} lines
 * @param {string} relPath
 * @returns {ToolEntry[]}
 */
function parseNapiPattern(lines, relPath) {
  const results = [];
  for (let i = 0; i < lines.length; i++) {
    if (!NAPI_CALL_RE.test(lines[i])) continue;
    const nameLine = lines[i + 1] ?? "";
    const nameMatch = TOOL_NAME_RE.exec(nameLine);
    if (!nameMatch) continue;
    const descLine = lines[i + 2] ?? "";
    const descMatch = TOOL_DESC_RE.exec(descLine);
    results.push({
      tool_name: nameMatch[1],
      source_file: relPath,
      description: descMatch ? descMatch[1] : "",
    });
  }
  return results;
}

/**
 * Parse a single file for plugin-style tool(server, "name", ...) registrations.
 * @param {string[]} lines
 * @param {string} relPath
 * @returns {ToolEntry[]}
 */
function parsePluginPattern(lines, relPath) {
  const results = [];
  for (let i = 0; i < lines.length; i++) {
    // Single-line form: `tool(server, "name", "desc", wasm.fn);`
    const inlineMatch = PLUGIN_INLINE_RE.exec(lines[i]);
    if (inlineMatch) {
      results.push({
        tool_name: inlineMatch[1],
        source_file: relPath,
        description: inlineMatch[2],
      });
      continue;
    }
    if (!PLUGIN_CALL_RE.test(lines[i])) continue;
    const serverLine = lines[i + 1] ?? "";
    if (!PLUGIN_SERVER_ARG_RE.test(serverLine)) continue;
    const nameLine = lines[i + 2] ?? "";
    const nameMatch = TOOL_NAME_RE.exec(nameLine);
    if (!nameMatch) continue;
    const descLine = lines[i + 3] ?? "";
    const descMatch = TOOL_DESC_RE.exec(descLine);
    results.push({
      tool_name: nameMatch[1],
      source_file: relPath,
      description: descMatch ? descMatch[1] : "",
    });
  }
  return results;
}

/**
 * Parse a file using the specified strategy.
 * @param {string} filePath  Absolute path to the TypeScript source file.
 * @param {string} workspaceRoot  Workspace root for relative path display.
 * @param {"napi"|"plugin"} strategy
 * @returns {ToolEntry[]}
 */
function parseFile(filePath, workspaceRoot, strategy) {
  const lines = readFileSync(filePath, "utf8").split("\n");
  const relPath = relative(workspaceRoot, filePath);
  return strategy === "plugin"
    ? parsePluginPattern(lines, relPath)
    : parseNapiPattern(lines, relPath);
}

/**
 * Recursively collect .ts files from a directory (stable, sorted order).
 * @param {string} dir
 * @returns {string[]}
 */
function collectTs(dir) {
  const entries = readdirSync(dir, { withFileTypes: true });
  const files = [];
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

/**
 * Extract all server.tool() registrations from packages/mcp-server/src/tools/.
 * Uses the NAPI pattern: server.tool(\n  "name", ...).
 * @param {string} workspaceRoot  Absolute path to the repo root.
 * @returns {ToolEntry[]}
 */
export function extractPackagesTools(workspaceRoot) {
  const toolsDir = join(workspaceRoot, "packages/mcp-server/src/tools");
  const files = collectTs(toolsDir);
  return files.flatMap((f) => parseFile(f, workspaceRoot, "napi"));
}

/**
 * Extract all tool() registrations from plugins/cfa-core/mcp/src/server.ts.
 * Uses the plugin wrapper pattern: tool(\n  server,\n  "name", ...).
 * @param {string} workspaceRoot  Absolute path to the repo root.
 * @returns {ToolEntry[]}
 */
export function extractPluginTools(workspaceRoot) {
  const serverFile = join(
    workspaceRoot,
    "plugins/cfa-core/mcp/src/server.ts",
  );
  return parseFile(serverFile, workspaceRoot, "plugin");
}
