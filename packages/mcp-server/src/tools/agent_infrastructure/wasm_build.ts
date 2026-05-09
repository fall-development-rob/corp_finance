/**
 * wasm_build.ts
 *
 * Phase 30 Wave 1 — Port of plugins/cfa-core/scripts/build-wasm.sh into a
 * typed, agent-callable MCP tool.
 *
 * Orchestrates the WASM build pipeline:
 *  1. Verify corerepo exists and contains scripts/build-wasm.sh
 *  2. Run bash scripts/build-wasm.sh inside corerepo (unless skip_build)
 *  3. Copy four artifacts from corerepo/dist/wasm/ to plugin_wasm_dir
 *  4. Return structured WasmBuildReport with timing and artifact metadata
 */

import { execFile as _execFile } from "node:child_process";
import {
  existsSync,
  mkdirSync,
  copyFileSync,
  readFileSync,
  readdirSync,
  statSync,
} from "node:fs";
import { join } from "node:path";
import { createHash } from "node:crypto";
import { promisify } from "node:util";

const execFile = promisify(_execFile);

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const ARTIFACT_NAMES = [
  "corp_finance_wasm.js",
  "corp_finance_wasm_bg.wasm",
  "corp_finance_wasm_bg.wasm.d.ts",
  "corp_finance_wasm.d.ts",
] as const;

const BUILD_LOG_TAIL_LINES = 50;
const BUILD_LOG_MAX_BYTES = 1_048_576; // 1 MB cap
const BUILD_TIMEOUT_MS = 600_000;      // 10 minutes

// ---------------------------------------------------------------------------
// Public interfaces
// ---------------------------------------------------------------------------

export interface WasmBuildInput {
  corerepo?: string;        // defaults to process.env.COREREPO ?? "/home/robert/corp-finance-core"
  plugin_wasm_dir?: string; // defaults to <workspace>/plugins/cfa-core/mcp/wasm
  workspace_root?: string;  // auto-detected if not given (walk up to .git)
  skip_build?: boolean;     // default false; if true, only copy existing artifacts
}

export interface WasmArtifact {
  name: string;
  source_path: string;  // path inside corerepo dist/wasm/
  dest_path: string;    // path inside plugin_wasm_dir
  bytes: number;
  sha256: string;
}

export interface WasmBuildReport {
  corerepo: string;
  plugin_wasm_dir: string;
  build_ran: boolean;
  build_log_tail: string;
  artifacts: WasmArtifact[];
  bundle_bytes: number;   // bytes of corp_finance_wasm_bg.wasm
  bundle_sha256: string;  // sha256 of corp_finance_wasm_bg.wasm
  duration_ms: number;
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

function detectWorkspaceRoot(): string {
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

function sha256Hex(filePath: string): string {
  const buf = readFileSync(filePath);
  return createHash("sha256").update(buf).digest("hex");
}

function tailLines(text: string, n: number): string {
  const lines = text.split("\n");
  return lines.slice(Math.max(0, lines.length - n)).join("\n");
}

function capOutput(raw: string): string {
  if (Buffer.byteLength(raw, "utf8") <= BUILD_LOG_MAX_BYTES) return raw;
  const buf = Buffer.from(raw, "utf8");
  return buf.subarray(buf.length - BUILD_LOG_MAX_BYTES).toString("utf8");
}

// ---------------------------------------------------------------------------
// Core implementation
// ---------------------------------------------------------------------------

export async function wasmBuild(input: WasmBuildInput): Promise<WasmBuildReport> {
  const start = Date.now();

  const corerepo =
    input.corerepo ??
    process.env["COREREPO"] ??
    "/home/robert/corp-finance-core";

  const workspaceRoot = input.workspace_root ?? detectWorkspaceRoot();

  const pluginWasmDir =
    input.plugin_wasm_dir ??
    join(workspaceRoot, "plugins/cfa-core/mcp/wasm");

  const skipBuild = input.skip_build ?? false;

  // --- 1. Verify corerepo ---
  if (!existsSync(corerepo)) {
    throw new Error(`corerepo not found: ${corerepo}`);
  }

  const buildScript = join(corerepo, "scripts/build-wasm.sh");
  if (!existsSync(buildScript)) {
    throw new Error(`build script not found: ${buildScript}`);
  }

  // --- 2. Optionally run build ---
  let buildLogTail = "";
  let buildRan = false;

  if (!skipBuild) {
    const { stdout, stderr } = await execFile("bash", [buildScript], {
      cwd: corerepo,
      timeout: BUILD_TIMEOUT_MS,
    });
    const combined = capOutput(stdout + stderr);
    buildLogTail = tailLines(combined, BUILD_LOG_TAIL_LINES);
    buildRan = true;
  }

  // --- 3. Verify artifacts exist ---
  const srcDir = join(corerepo, "dist/wasm");
  for (const name of ARTIFACT_NAMES) {
    const srcPath = join(srcDir, name);
    if (!existsSync(srcPath)) {
      throw new Error(`artifact missing after build: ${srcPath}`);
    }
  }

  // --- 4. Ensure dest dir exists ---
  mkdirSync(pluginWasmDir, { recursive: true });

  // --- 5. Copy artifacts and collect metadata ---
  const artifacts: WasmArtifact[] = [];

  for (const name of ARTIFACT_NAMES) {
    const sourcePath = join(srcDir, name);
    const destPath = join(pluginWasmDir, name);
    copyFileSync(sourcePath, destPath);
    const bytes = statSync(destPath).size;
    const hash = sha256Hex(destPath);
    artifacts.push({ name, source_path: sourcePath, dest_path: destPath, bytes, sha256: hash });
  }

  // --- 6. Bundle stats for .wasm binary ---
  const wasmEntry = artifacts.find((a) => a.name === "corp_finance_wasm_bg.wasm");
  const bundleBytes = wasmEntry?.bytes ?? 0;
  const bundleSha256 = wasmEntry?.sha256 ?? "";

  return {
    corerepo,
    plugin_wasm_dir: pluginWasmDir,
    build_ran: buildRan,
    build_log_tail: buildLogTail,
    artifacts,
    bundle_bytes: bundleBytes,
    bundle_sha256: bundleSha256,
    duration_ms: Date.now() - start,
  };
}

// ---------------------------------------------------------------------------
// MCP wrapper — accepts JSON string, returns JSON string
// ---------------------------------------------------------------------------

export async function wasmBuildMcp(inputJson: string): Promise<string> {
  const input = JSON.parse(inputJson) as WasmBuildInput;
  const report = await wasmBuild(input);
  return JSON.stringify(report);
}

// CLI invocation: `tsx wasm_build.ts [--skip-build]` — same code path as the
// MCP tool, exposed for npm scripts and CI without introducing a separate file.
if (import.meta.url === `file://${process.argv[1]}`) {
  (async () => {
    const skipBuild = process.argv.includes("--skip-build");
    const report = await wasmBuild({ skip_build: skipBuild });
    process.stdout.write(JSON.stringify(report, null, 2) + "\n");
    process.exit(0);
  })().catch((err) => {
    process.stderr.write(`wasm_build failed: ${err instanceof Error ? err.message : String(err)}\n`);
    process.exit(1);
  });
}
