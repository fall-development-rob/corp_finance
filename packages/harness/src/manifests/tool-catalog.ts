/**
 * Tool catalog — Phase 25 Tier A1.
 *
 * Generates a canonical catalog of every MCP tool registered across the four
 * in-repo MCP servers (cfa-core, fmp, data, vendor) and lints managed-agent
 * cookbook YAMLs against it. Catches tool-name drift between cookbook
 * `configs[].name` entries and real server tool registrations — the bug class
 * documented in the best-in-class roadmap memory (build_lbo, calculate_dupont,
 * calculate_h_model_ddm, calculate_target_price).
 *
 * Pure library — no I/O abstractions beyond `fs.readFileSync` / `readdirSync`.
 * CLI runner lives in `scripts/generate-tool-catalog.ts`.
 */

import { readFileSync, readdirSync, existsSync } from "node:fs";
import { join, relative, resolve } from "node:path";
import { parse as parseYaml } from "yaml";

import type { AgentManifest, ToolsetConfig, ToolOverride } from "./types.js";

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/**
 * Canonical tool catalog: each server maps to a sorted unique list of tool
 * names. Serialised as JSON in `data/tools-catalog.json`.
 */
export interface ToolCatalog {
  /** Format version. Bump when JSON shape changes. */
  version: string;
  /** Server name → alphabetically sorted, deduped tool names. */
  servers: Record<string, string[]>;
}

/** Identifies one source root that contributes tool registrations. */
export interface ServerSource {
  /** Cookbook-side server name (matches `mcp_server_name` in cookbook YAML). */
  server: string;
  /** Absolute path to the directory or file to scan. */
  sourcePath: string;
  /** Extraction pattern. See parseFile / parseInlinePluginFile for semantics. */
  pattern: "napi-multiline" | "inline-plugin";
}

export interface CookbookLintIssue {
  /** Cookbook slug (parent directory name) — e.g. "equity-analyst". */
  cookbook: string;
  /** Path to the offending YAML file, relative to repo root. */
  manifest: string;
  /** Cookbook-declared MCP server name. */
  server: string;
  /** Tool name as it appears in `configs[].name` (with mcp__ prefix). */
  tool_name_raw: string;
  /** Tool name after stripping `mcp__<server>__` prefix. */
  tool_name: string;
  /** Why this is an issue. */
  reason: "unknown_server" | "unknown_tool" | "prefix_mismatch";
  message: string;
}

export interface CookbookLintReport {
  cookbooks_scanned: number;
  files_scanned: number;
  configs_checked: number;
  issues: CookbookLintIssue[];
}

// ---------------------------------------------------------------------------
// Source roots — single source of truth for the 4 in-repo MCP servers.
// ---------------------------------------------------------------------------

/**
 * Canonical mapping: cookbook `mcp_server_name` → in-repo source location.
 * Adding a new server is a one-line change here.
 */
export function defaultServerSources(repoRoot: string): ServerSource[] {
  return [
    {
      server: "cfa-core",
      sourcePath: join(repoRoot, "plugins/cfa-core/mcp/src/server.ts"),
      pattern: "inline-plugin",
    },
    {
      server: "fmp",
      sourcePath: join(repoRoot, "packages/fmp-mcp-server/src"),
      pattern: "napi-multiline",
    },
    {
      server: "data",
      sourcePath: join(repoRoot, "packages/data-mcp-server/src"),
      pattern: "napi-multiline",
    },
    {
      server: "vendor",
      sourcePath: join(repoRoot, "packages/vendor-mcp-server/src"),
      pattern: "napi-multiline",
    },
  ];
}

// ---------------------------------------------------------------------------
// Tool name extraction — regex patterns mirror surface_parity.ts
// ---------------------------------------------------------------------------

// `server.tool(` on its own line (multi-line registration in fmp/data/vendor).
const NAPI_CALL_RE = /^\s*server\.tool\s*\(/;
// `"<tool_name>",` or `'<tool_name>',` as the first argument on the next line.
const TOOL_NAME_RE = /^\s*['"]([^'"]+)['"]/;
// Inline `tool(server, "<tool_name>", "<desc>", ...)` — used by cfa-core plugin server.
// We only need to capture the tool name; descriptions may contain apostrophes
// so we don't try to fully parse the second argument.
const PLUGIN_INLINE_RE =
  /^\s*tool\s*\(\s*server\s*,\s*["']([A-Za-z_][A-Za-z0-9_]*)["']\s*,/;
// Multi-line plugin form: `tool(\n    server,\n    "<tool_name>",`
const PLUGIN_MULTILINE_OPEN_RE = /^\s*tool\s*\(\s*$/;
const PLUGIN_SERVER_ARG_RE = /^\s*server\s*,?\s*$/;

function collectTsFiles(dir: string): string[] {
  const out: string[] = [];
  const entries = readdirSync(dir, { withFileTypes: true });
  for (const entry of entries) {
    const full = join(dir, entry.name);
    if (entry.isDirectory()) {
      if (entry.name === "node_modules" || entry.name === "dist") continue;
      out.push(...collectTsFiles(full));
    } else if (
      entry.isFile() &&
      entry.name.endsWith(".ts") &&
      !entry.name.endsWith(".test.ts")
    ) {
      out.push(full);
    }
  }
  return out.sort();
}

function extractNapiMultiline(filePath: string): string[] {
  const lines = readFileSync(filePath, "utf8").split("\n");
  const names: string[] = [];
  for (let i = 0; i < lines.length; i++) {
    if (!NAPI_CALL_RE.test(lines[i]!)) continue;
    const nameLine = lines[i + 1] ?? "";
    const m = TOOL_NAME_RE.exec(nameLine);
    if (m) names.push(m[1]!);
  }
  return names;
}

function extractInlinePlugin(filePath: string): string[] {
  const lines = readFileSync(filePath, "utf8").split("\n");
  const names: string[] = [];
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i]!;
    // Case 1: inline `tool(server, "name", ...)` — name on same line.
    const inline = PLUGIN_INLINE_RE.exec(line);
    if (inline) {
      names.push(inline[1]!);
      continue;
    }
    // Case 2: multi-line `tool(\n    server,\n    "name",` — name two lines down.
    if (PLUGIN_MULTILINE_OPEN_RE.test(line)) {
      const serverLine = lines[i + 1] ?? "";
      if (!PLUGIN_SERVER_ARG_RE.test(serverLine)) continue;
      const nameLine = lines[i + 2] ?? "";
      const m = TOOL_NAME_RE.exec(nameLine);
      if (m) names.push(m[1]!);
    }
  }
  return names;
}

function extractFromSource(source: ServerSource): string[] {
  if (source.pattern === "inline-plugin") {
    if (!existsSync(source.sourcePath)) return [];
    return extractInlinePlugin(source.sourcePath);
  }
  // napi-multiline — directory walk
  if (!existsSync(source.sourcePath)) return [];
  const files = collectTsFiles(source.sourcePath);
  return files.flatMap(extractNapiMultiline);
}

// ---------------------------------------------------------------------------
// Generation
// ---------------------------------------------------------------------------

export interface GenerateInput {
  /** Absolute repo root. */
  repoRoot: string;
  /** Override server source list. Defaults to defaultServerSources(repoRoot). */
  sources?: ServerSource[];
}

/**
 * Build the canonical catalog by scanning every server source. Tool names per
 * server are alphabetically sorted and deduplicated.
 */
export function generateToolCatalog(input: GenerateInput): ToolCatalog {
  const sources = input.sources ?? defaultServerSources(input.repoRoot);
  const servers: Record<string, string[]> = {};
  for (const src of sources) {
    const raw = extractFromSource(src);
    const unique = Array.from(new Set(raw)).sort((a, b) => a.localeCompare(b));
    servers[src.server] = unique;
  }
  return { version: "1", servers };
}

/**
 * Stable JSON serialisation: 2-space indent, alphabetical key ordering for
 * the `servers` map, trailing newline. Two runs against the same source state
 * produce byte-identical output.
 */
export function serialiseCatalog(catalog: ToolCatalog): string {
  const orderedServers: Record<string, string[]> = {};
  for (const key of Object.keys(catalog.servers).sort()) {
    orderedServers[key] = [...(catalog.servers[key] ?? [])];
  }
  const ordered: ToolCatalog = {
    version: catalog.version,
    servers: orderedServers,
  };
  return JSON.stringify(ordered, null, 2) + "\n";
}

export function parseCatalog(json: string): ToolCatalog {
  const parsed = JSON.parse(json) as unknown;
  if (
    typeof parsed !== "object" ||
    parsed === null ||
    !("servers" in parsed) ||
    typeof (parsed as { servers: unknown }).servers !== "object"
  ) {
    throw new Error("tool catalog JSON missing 'servers' object");
  }
  const obj = parsed as { version?: string; servers: Record<string, unknown> };
  const servers: Record<string, string[]> = {};
  for (const [k, v] of Object.entries(obj.servers)) {
    if (!Array.isArray(v) || !v.every((s) => typeof s === "string")) {
      throw new Error(`tool catalog server '${k}' must be array of strings`);
    }
    servers[k] = v as string[];
  }
  return { version: obj.version ?? "1", servers };
}

// ---------------------------------------------------------------------------
// Cookbook lint
// ---------------------------------------------------------------------------

const MCP_PREFIX_RE = /^mcp__([a-zA-Z0-9_-]+)__(.+)$/;

interface ManifestFile {
  cookbook: string;
  path: string;
  manifest: AgentManifest;
}

function readManifest(path: string): AgentManifest | null {
  try {
    const raw = readFileSync(path, "utf8");
    const parsed = parseYaml(raw) as unknown;
    if (typeof parsed !== "object" || parsed === null) return null;
    return parsed as AgentManifest;
  } catch {
    return null;
  }
}

function collectManifests(cookbooksRoot: string): ManifestFile[] {
  const out: ManifestFile[] = [];
  if (!existsSync(cookbooksRoot)) return out;
  const slugs = readdirSync(cookbooksRoot, { withFileTypes: true })
    .filter((e) => e.isDirectory())
    .map((e) => e.name)
    .sort();
  for (const slug of slugs) {
    const cookbookDir = join(cookbooksRoot, slug);
    // Parent manifest
    for (const fname of ["agent.yaml", "agent.yml", "agent.json"]) {
      const p = join(cookbookDir, fname);
      if (existsSync(p)) {
        const m = readManifest(p);
        if (m) out.push({ cookbook: slug, path: p, manifest: m });
        break;
      }
    }
    // Subagent manifests
    const subDir = join(cookbookDir, "subagents");
    if (existsSync(subDir)) {
      const subs = readdirSync(subDir, { withFileTypes: true })
        .filter(
          (e) =>
            e.isFile() &&
            (e.name.endsWith(".yaml") ||
              e.name.endsWith(".yml") ||
              e.name.endsWith(".json")),
        )
        .map((e) => join(subDir, e.name))
        .sort();
      for (const p of subs) {
        const m = readManifest(p);
        if (m) out.push({ cookbook: slug, path: p, manifest: m });
      }
    }
  }
  return out;
}

interface ConfigRef {
  server: string;
  configs: ToolOverride[];
}

function collectMcpConfigs(tools: ToolsetConfig[] | undefined): ConfigRef[] {
  if (!tools) return [];
  const out: ConfigRef[] = [];
  for (const block of tools) {
    if (block.type !== "mcp_toolset") continue;
    if (!block.configs || block.configs.length === 0) continue;
    out.push({ server: block.mcp_server_name, configs: block.configs });
  }
  return out;
}

export interface LintInput {
  catalog: ToolCatalog;
  cookbooksRoot: string;
  /** Used to make `manifest` paths in issues relative + readable. */
  repoRoot: string;
}

/**
 * Pure function: walk every cookbook manifest + subagent manifest, find every
 * `mcp_toolset` block with explicit `configs[]`, and flag any entry whose
 * `name` cannot be resolved to a registered tool for the named server.
 */
export function lintCookbookToolNames(input: LintInput): CookbookLintReport {
  const manifests = collectManifests(input.cookbooksRoot);
  const cookbookSet = new Set<string>();
  let configsChecked = 0;
  const issues: CookbookLintIssue[] = [];

  for (const mf of manifests) {
    cookbookSet.add(mf.cookbook);
    const refs = collectMcpConfigs(mf.manifest.tools);
    for (const ref of refs) {
      for (const cfg of ref.configs) {
        configsChecked += 1;
        const issue = classify(cfg, ref.server, input.catalog);
        if (issue) {
          issues.push({
            cookbook: mf.cookbook,
            manifest: relative(input.repoRoot, mf.path),
            ...issue,
          });
        }
      }
    }
  }

  return {
    cookbooks_scanned: cookbookSet.size,
    files_scanned: manifests.length,
    configs_checked: configsChecked,
    issues,
  };
}

function classify(
  cfg: ToolOverride,
  declaredServer: string,
  catalog: ToolCatalog,
): Omit<CookbookLintIssue, "cookbook" | "manifest"> | null {
  const m = MCP_PREFIX_RE.exec(cfg.name);
  if (!m) {
    // Bare name without mcp__ prefix — not in scope of this lint.
    return null;
  }
  const prefixServer = m[1]!;
  const bareTool = m[2]!;

  if (prefixServer !== declaredServer) {
    return {
      server: declaredServer,
      tool_name_raw: cfg.name,
      tool_name: bareTool,
      reason: "prefix_mismatch",
      message: `tool name prefix declares server '${prefixServer}' but block.mcp_server_name='${declaredServer}'`,
    };
  }

  const serverCatalog = catalog.servers[declaredServer];
  if (!serverCatalog) {
    return {
      server: declaredServer,
      tool_name_raw: cfg.name,
      tool_name: bareTool,
      reason: "unknown_server",
      message: `mcp_server_name '${declaredServer}' is not in the canonical tool catalog (known: ${Object.keys(catalog.servers).join(", ")})`,
    };
  }

  if (!serverCatalog.includes(bareTool)) {
    return {
      server: declaredServer,
      tool_name_raw: cfg.name,
      tool_name: bareTool,
      reason: "unknown_tool",
      message: `tool '${bareTool}' is not registered in MCP server '${declaredServer}'`,
    };
  }

  return null;
}

// Re-exported helpers (also used by callers that already have a catalog).
export function resolveRepoRoot(fromDir: string): string {
  // Walk up looking for managed-agent-cookbooks/.
  let cur = resolve(fromDir);
  for (let i = 0; i < 12; i++) {
    if (existsSync(join(cur, "managed-agent-cookbooks"))) return cur;
    const parent = resolve(cur, "..");
    if (parent === cur) break;
    cur = parent;
  }
  return resolve(fromDir);
}
