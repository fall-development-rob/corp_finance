/**
 * Unit tests for wasm_build.ts (Phase 30 Wave 1).
 *
 * Strategy:
 *  - Use vitest's vi.mock to intercept node:child_process execFile.
 *  - Use real temp dirs (mkdtempSync) for artifact fixture files.
 *  - Each test tears down its temp dir in afterEach.
 */

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import {
  mkdtempSync,
  writeFileSync,
  mkdirSync,
  rmSync,
  existsSync,
} from "node:fs";
import { join } from "node:path";
import { tmpdir } from "node:os";
import { createHash } from "node:crypto";

// ---------------------------------------------------------------------------
// Mock child_process before importing the module under test
// ---------------------------------------------------------------------------

vi.mock("node:child_process", () => ({
  execFile: vi.fn(),
}));

import { wasmBuild, wasmBuildMcp } from "./wasm_build.js";
import { execFile as rawExecFile } from "node:child_process";

// The module under test uses promisify(execFile). We mock the raw execFile
// because promisify wraps it at module load time from the mocked version.
const mockExecFile = vi.mocked(rawExecFile);

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const ARTIFACT_NAMES = [
  "corp_finance_wasm.js",
  "corp_finance_wasm_bg.wasm",
  "corp_finance_wasm_bg.wasm.d.ts",
  "corp_finance_wasm.d.ts",
] as const;

function sha256Hex(content: Buffer | string): string {
  const buf = typeof content === "string" ? Buffer.from(content, "utf8") : content;
  return createHash("sha256").update(buf).digest("hex");
}

/** Create a fake corerepo with scripts/build-wasm.sh and optionally dist/wasm/ artifacts. */
function makeFakeCorerepo(
  opts: { withArtifacts?: boolean; artifactContent?: Record<string, string> } = {},
): string {
  const dir = mkdtempSync(join(tmpdir(), "fake-corerepo-"));
  mkdirSync(join(dir, "scripts"), { recursive: true });
  writeFileSync(join(dir, "scripts/build-wasm.sh"), "#!/usr/bin/env bash\necho ok\n");

  if (opts.withArtifacts) {
    mkdirSync(join(dir, "dist/wasm"), { recursive: true });
    for (const name of ARTIFACT_NAMES) {
      const content = opts.artifactContent?.[name] ?? `fake content for ${name}`;
      writeFileSync(join(dir, "dist/wasm", name), content);
    }
  }
  return dir;
}

/** Teach the mock execFile to resolve with stdout/stderr. */
function mockBuildSuccess(stdout = "[cfa-core] done.\n", stderr = ""): void {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  (mockExecFile as any).mockImplementation((...args: unknown[]) => {
    const cb = args[args.length - 1] as (err: null, result: { stdout: string; stderr: string }) => void;
    cb(null, { stdout, stderr });
  });
}

/** Teach the mock execFile to reject with an error. */
function mockBuildFailure(message = "build failed"): void {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  (mockExecFile as any).mockImplementation((...args: unknown[]) => {
    const cb = args[args.length - 1] as (err: Error) => void;
    cb(new Error(message));
  });
}

// ---------------------------------------------------------------------------
// Test suites
// ---------------------------------------------------------------------------

describe("wasmBuild", () => {
  let fakeCorerepo: string;
  let pluginWasmDir: string;
  let workspaceDir: string;

  beforeEach(() => {
    workspaceDir = mkdtempSync(join(tmpdir(), "fake-workspace-"));
    pluginWasmDir = join(workspaceDir, "wasm-out");
  });

  afterEach(() => {
    if (fakeCorerepo && existsSync(fakeCorerepo)) rmSync(fakeCorerepo, { recursive: true, force: true });
    if (workspaceDir && existsSync(workspaceDir)) rmSync(workspaceDir, { recursive: true, force: true });
    vi.clearAllMocks();
  });

  // -------------------------------------------------------------------------
  // Happy-path: build runs, artifacts copied, report returned
  // -------------------------------------------------------------------------

  it("runs build, copies all four artifacts, and returns a valid report", async () => {
    fakeCorerepo = makeFakeCorerepo({ withArtifacts: true });
    mockBuildSuccess("build output line 1\nbuild done\n");

    const report = await wasmBuild({
      corerepo: fakeCorerepo,
      plugin_wasm_dir: pluginWasmDir,
      workspace_root: workspaceDir,
    });

    expect(report.build_ran).toBe(true);
    expect(report.corerepo).toBe(fakeCorerepo);
    expect(report.plugin_wasm_dir).toBe(pluginWasmDir);
    expect(report.artifacts).toHaveLength(4);
    expect(report.duration_ms).toBeGreaterThanOrEqual(0);

    // All four artifacts should land in pluginWasmDir
    for (const name of ARTIFACT_NAMES) {
      expect(existsSync(join(pluginWasmDir, name))).toBe(true);
    }

    // bundle_sha256 must be a 64-char hex string
    expect(/^[0-9a-f]{64}$/.test(report.bundle_sha256)).toBe(true);
    expect(report.bundle_bytes).toBeGreaterThan(0);
  });

  // -------------------------------------------------------------------------
  // Error: corerepo directory does not exist
  // -------------------------------------------------------------------------

  it("throws when corerepo directory does not exist", async () => {
    await expect(
      wasmBuild({
        corerepo: "/tmp/does-not-exist-xyzzy",
        plugin_wasm_dir: pluginWasmDir,
        workspace_root: workspaceDir,
      }),
    ).rejects.toThrow("corerepo not found");
  });

  // -------------------------------------------------------------------------
  // Error: corerepo exists but build script is absent
  // -------------------------------------------------------------------------

  it("throws when scripts/build-wasm.sh is missing in corerepo", async () => {
    fakeCorerepo = mkdtempSync(join(tmpdir(), "no-script-"));
    // No scripts/ directory created

    await expect(
      wasmBuild({
        corerepo: fakeCorerepo,
        plugin_wasm_dir: pluginWasmDir,
        workspace_root: workspaceDir,
      }),
    ).rejects.toThrow("build script not found");
  });

  // -------------------------------------------------------------------------
  // Error: build "succeeds" but artifacts are missing
  // -------------------------------------------------------------------------

  it("throws with artifact name when build succeeds but dist/wasm artifact is absent", async () => {
    // withArtifacts: false — dist/wasm/ won't be created
    fakeCorerepo = makeFakeCorerepo({ withArtifacts: false });
    mockBuildSuccess();

    await expect(
      wasmBuild({
        corerepo: fakeCorerepo,
        plugin_wasm_dir: pluginWasmDir,
        workspace_root: workspaceDir,
      }),
    ).rejects.toThrow("artifact missing after build");
  });

  // -------------------------------------------------------------------------
  // skip_build: true — subprocess never called, existing artifacts copied
  // -------------------------------------------------------------------------

  it("does not invoke execFile when skip_build is true", async () => {
    fakeCorerepo = makeFakeCorerepo({ withArtifacts: true });

    const report = await wasmBuild({
      corerepo: fakeCorerepo,
      plugin_wasm_dir: pluginWasmDir,
      workspace_root: workspaceDir,
      skip_build: true,
    });

    expect(mockExecFile).not.toHaveBeenCalled();
    expect(report.build_ran).toBe(false);
    expect(report.artifacts).toHaveLength(4);

    for (const name of ARTIFACT_NAMES) {
      expect(existsSync(join(pluginWasmDir, name))).toBe(true);
    }
  });

  // -------------------------------------------------------------------------
  // sha256 reproducibility: known bytes yield known hash
  // -------------------------------------------------------------------------

  it("produces a reproducible sha256 for a known artifact content", async () => {
    const knownContent = "hello wasm world";
    fakeCorerepo = makeFakeCorerepo({
      withArtifacts: true,
      artifactContent: {
        "corp_finance_wasm_bg.wasm": knownContent,
      },
    });
    mockBuildSuccess();

    const report = await wasmBuild({
      corerepo: fakeCorerepo,
      plugin_wasm_dir: pluginWasmDir,
      workspace_root: workspaceDir,
    });

    const expected = sha256Hex(Buffer.from(knownContent, "utf8"));
    expect(report.bundle_sha256).toBe(expected);

    // Also verify via the artifact array
    const wasmArtifact = report.artifacts.find(
      (a) => a.name === "corp_finance_wasm_bg.wasm",
    );
    expect(wasmArtifact?.sha256).toBe(expected);
    expect(wasmArtifact?.bytes).toBe(Buffer.byteLength(knownContent, "utf8"));
  });

  // -------------------------------------------------------------------------
  // build_log_tail: stdout is captured and trimmed to ~50 lines
  // -------------------------------------------------------------------------

  it("captures build stdout in build_log_tail", async () => {
    fakeCorerepo = makeFakeCorerepo({ withArtifacts: true });
    const buildOutput = Array.from({ length: 100 }, (_, i) => `line ${i + 1}`).join("\n");
    mockBuildSuccess(buildOutput);

    const report = await wasmBuild({
      corerepo: fakeCorerepo,
      plugin_wasm_dir: pluginWasmDir,
      workspace_root: workspaceDir,
    });

    // build_log_tail must only contain the last 50 lines
    const tailLines = report.build_log_tail.split("\n").filter((l) => l.trim() !== "");
    expect(tailLines.length).toBeLessThanOrEqual(50);
    // The last captured line should be line 100
    expect(report.build_log_tail).toContain("line 100");
    // Line 1 should not appear (truncated away)
    expect(report.build_log_tail).not.toContain("line 1\n");
  });

  // -------------------------------------------------------------------------
  // MCP wrapper: round-trips through JSON
  // -------------------------------------------------------------------------

  it("wasmBuildMcp accepts JSON string and returns JSON string", async () => {
    fakeCorerepo = makeFakeCorerepo({ withArtifacts: true });
    mockBuildSuccess();

    const inputJson = JSON.stringify({
      corerepo: fakeCorerepo,
      plugin_wasm_dir: pluginWasmDir,
      workspace_root: workspaceDir,
    });

    const resultJson = await wasmBuildMcp(inputJson);
    const result = JSON.parse(resultJson) as { build_ran: boolean; artifacts: unknown[] };

    expect(result.build_ran).toBe(true);
    expect(Array.isArray(result.artifacts)).toBe(true);
    expect((result.artifacts as unknown[]).length).toBe(4);
  });
});
