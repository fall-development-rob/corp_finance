/**
 * Phase 35 — WorkflowRouter port.
 *
 * Provides two implementations:
 *   - `createCliWorkflowRouter`: shells out to `cfa workflow *` subcommands
 *     using the same `execFile` pattern as cookbook.ts.
 *   - `createMockWorkflowRouter`: in-process stub for unit tests.
 *
 * The CLI router resolves the `cfa` binary by:
 *   1. Checking `opts.cfaBinary` (explicit override)
 *   2. Trying `<repoRoot>/target/debug/cfa` (local dev build)
 *   3. Falling back to `"cfa"` on PATH
 *
 * Binary resolution order is the only difference from cookbook.ts pattern;
 * the exec mechanics (execFile, no shell, timeout) are identical.
 */
import { execFile as _execFile } from "node:child_process";
import { existsSync, statSync } from "node:fs";
import { join } from "node:path";
import { promisify } from "node:util";
import {
  WorkflowRouterError,
  type Workflow,
  type WorkflowList,
  type WorkflowMatch,
  type WorkflowResult,
} from "./types.js";

const execFileAsync = promisify(_execFile);

// ---------------------------------------------------------------------------
// Public interface
// ---------------------------------------------------------------------------

export interface WorkflowRouter {
  /** Return all workflows. Cached after first call. */
  list(): Promise<WorkflowList>;

  /**
   * Score `prompt` against all workflows. Returns the best match above the
   * configured confidence floor, or null if no workflow scores high enough.
   */
  match(prompt: string): Promise<WorkflowMatch | null>;

  /**
   * Execute the named workflow with the given params. Marshals `params` to
   * the `--input` JSON flag and parses stdout into `WorkflowResult`.
   */
  run(slug: string, params: Record<string, unknown>): Promise<WorkflowResult>;
}

// ---------------------------------------------------------------------------
// CLI router options
// ---------------------------------------------------------------------------

export interface CliWorkflowRouterOpts {
  /** Explicit binary path; overrides auto-detection. */
  cfaBinary?: string;
  /** Working directory for subprocess. Default: process.cwd(). */
  cwd?: string;
  /** Timeout for list + match calls in ms. Default: 30_000. */
  timeoutMs?: number;
  /** Timeout for run calls in ms (includes MCP round-trips). Default: 60_000. */
  runTimeoutMs?: number;
}

// ---------------------------------------------------------------------------
// Binary resolution
// ---------------------------------------------------------------------------

function resolveWorkspaceRoot(): string {
  let dir = process.cwd();
  for (let i = 0; i < 20; i++) {
    try {
      statSync(join(dir, ".git"));
      return dir;
    } catch {
      const parent = join(dir, "..");
      if (parent === dir) break;
      dir = parent;
    }
  }
  return process.cwd();
}

function resolveCfaBinary(given?: string): string {
  if (given) return given;
  const debugBinary = join(resolveWorkspaceRoot(), "target", "debug", "cfa");
  if (existsSync(debugBinary)) return debugBinary;
  return "cfa";
}

// ---------------------------------------------------------------------------
// CLI helpers
// ---------------------------------------------------------------------------

async function execCli(
  binary: string,
  args: string[],
  cwd: string,
  timeoutMs: number,
): Promise<string> {
  try {
    const { stdout, stderr } = await execFileAsync(binary, args, { cwd, timeout: timeoutMs });
    void stderr; // captured but not thrown; used only on non-zero exit
    return stdout;
  } catch (err) {
    if (err instanceof Error && "code" in err && (err as NodeJS.ErrnoException).code === "ETIMEDOUT") {
      throw new WorkflowRouterError(
        `cfa workflow timed out after ${timeoutMs}ms`,
        "TIMEOUT",
        { args },
      );
    }
    const msg = err instanceof Error ? err.message : String(err);
    throw new WorkflowRouterError(
      `cfa workflow CLI error: ${msg}`,
      "CLI_ERROR",
      { args },
    );
  }
}

function parseJson<T>(raw: string, context: string): T {
  try {
    return JSON.parse(raw) as T;
  } catch {
    throw new WorkflowRouterError(
      `${context}: stdout is not valid JSON`,
      "PARSE_ERROR",
      { raw: raw.slice(0, 300) },
    );
  }
}

// ---------------------------------------------------------------------------
// CLI WorkflowRouter factory
// ---------------------------------------------------------------------------

export function createCliWorkflowRouter(opts?: CliWorkflowRouterOpts): WorkflowRouter {
  const binary = resolveCfaBinary(opts?.cfaBinary);
  const cwd = opts?.cwd ?? process.cwd();
  const timeoutMs = opts?.timeoutMs ?? 30_000;
  const runTimeoutMs = opts?.runTimeoutMs ?? 60_000;

  let listCache: WorkflowList | undefined;

  return {
    async list(): Promise<WorkflowList> {
      if (listCache) return listCache;
      const raw = await execCli(binary, ["workflow", "list"], cwd, timeoutMs);
      const result = parseJson<WorkflowList>(raw, "cfa workflow list");
      listCache = result;
      return result;
    },

    async match(prompt: string): Promise<WorkflowMatch | null> {
      const raw = await execCli(binary, ["workflow", "match", prompt], cwd, timeoutMs);
      const trimmed = raw.trim();
      if (!trimmed || trimmed === "null") return null;
      const result = parseJson<WorkflowMatch | null>(trimmed, "cfa workflow match");
      return result;
    },

    async run(slug: string, params: Record<string, unknown>): Promise<WorkflowResult> {
      const inputJson = JSON.stringify(params);
      const raw = await execCli(
        binary,
        ["workflow", "run", slug, "--input", inputJson],
        cwd,
        runTimeoutMs,
      );
      const result = parseJson<WorkflowResult>(raw, `cfa workflow run ${slug}`);
      if ("error" in (result as unknown as Record<string, unknown>)) {
        const r = result as unknown as { error: string; slug: string };
        throw new WorkflowRouterError(
          `workflow ${slug} failed: ${r.error}`,
          "CLI_ERROR",
          { slug },
        );
      }
      return result;
    },
  };
}

// ---------------------------------------------------------------------------
// Mock WorkflowRouter factory (for unit tests)
// ---------------------------------------------------------------------------

export interface MockWorkflowRouterFixtures {
  workflows?: Workflow[];
  matchResult?: WorkflowMatch | null;
  runResult?: WorkflowResult;
}

export function createMockWorkflowRouter(
  fixtures: MockWorkflowRouterFixtures = {},
): WorkflowRouter {
  return {
    async list(): Promise<WorkflowList> {
      return {
        total: fixtures.workflows?.length ?? 0,
        workflows: fixtures.workflows ?? [],
      };
    },

    async match(_prompt: string): Promise<WorkflowMatch | null> {
      return fixtures.matchResult ?? null;
    },

    async run(_slug: string, _params: Record<string, unknown>): Promise<WorkflowResult> {
      if (!fixtures.runResult) {
        throw new WorkflowRouterError(
          "mock: no runResult fixture provided",
          "CLI_ERROR",
          {},
        );
      }
      return fixtures.runResult;
    },
  };
}
