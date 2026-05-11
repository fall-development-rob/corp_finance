/**
 * cookbook.ts — MCP-callable cookbook validation tool (Phase 30 Wave 1).
 *
 * Ports scripts/test-cookbooks.sh into a structured, agent-invocable TypeScript
 * function. Uses execFile (no shell) and returns typed JSON rather than
 * process.exit / formatted stdout.
 */

import { execFile as _execFile } from "node:child_process";
import { readdirSync, statSync } from "node:fs";
import { join, basename } from "node:path";
import { promisify } from "node:util";

// Promisified executor — injectable for unit tests.
export type ExecFn = (
  binary: string,
  args: string[],
) => Promise<{ stdout: string; stderr: string }>;

export const defaultExecFn: ExecFn = promisify(_execFile) as ExecFn;

// ---------------------------------------------------------------------------
// Public interfaces
// ---------------------------------------------------------------------------

export interface CookbookValidateInput {
  workspace_root?: string;
  cookbooks_root?: string;
  skills_root?: string;
  agents_root?: string;
  cfa_binary?: string;
  slugs?: string[];
  /** Injectable executor for unit tests — omit in production. */
  _exec?: ExecFn;
}

export interface CookbookCheckResult {
  slug: string;
  validate_ok: boolean;
  deploy_ok: boolean;
  validate_output: unknown;
  deploy_output?: unknown;
  error?: string;
}

export interface CookbookValidateAllReport {
  workspace_root: string;
  cookbooks_root: string;
  total: number;
  passed: number;
  failed: number;
  results: CookbookCheckResult[];
  all_ok: boolean;
}

// ---------------------------------------------------------------------------
// Workspace resolution
// ---------------------------------------------------------------------------

function resolveWorkspaceRoot(given?: string): string {
  if (given) return given;
  // Walk up from cwd until we find a .git directory.
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

// ---------------------------------------------------------------------------
// Discovery
// ---------------------------------------------------------------------------

export function discoverCookbookSlugs(cookbooksRoot: string): string[] {
  const slugs: string[] = [];
  let entries: string[];
  try {
    entries = readdirSync(cookbooksRoot);
  } catch {
    return slugs;
  }
  for (const entry of entries) {
    const dir = join(cookbooksRoot, entry);
    try {
      if (!statSync(dir).isDirectory()) continue;
    } catch {
      continue;
    }
    // Post-Phase-36: cookbooks are .yaml. Pre-Phase-36 legacy still .json.
    // Accept either at depth-2 ; .yaml takes precedence if both present.
    const candidates = ["agent.yaml", "agent.yml", "agent.json"];
    for (const candidate of candidates) {
      try {
        statSync(join(dir, candidate));
        slugs.push(basename(dir));
        break; // first match wins; don't double-count
      } catch {
        // try next extension
      }
    }
  }
  return slugs.sort();
}

// ---------------------------------------------------------------------------
// Single-cookbook validation
// ---------------------------------------------------------------------------

interface ValidateOneInput {
  slug: string;
  cfaBinary: string;
  cookbooksRoot: string;
  skillsRoot: string;
  agentsRoot: string;
  exec?: ExecFn;
}

export async function validateOneCookbook(
  input: ValidateOneInput,
): Promise<CookbookCheckResult> {
  const { slug, cfaBinary, cookbooksRoot, skillsRoot, agentsRoot } = input;
  const exec = input.exec ?? defaultExecFn;

  // --- validate ---
  let validateOutput: unknown;
  try {
    const { stdout } = await exec(cfaBinary, [
      "managed-agent",
      "validate",
      slug,
      "--cookbooks-root",
      cookbooksRoot,
      "--skills-root",
      skillsRoot,
      "--agents-root",
      agentsRoot,
    ]);
    try {
      validateOutput = JSON.parse(stdout);
    } catch {
      return {
        slug,
        validate_ok: false,
        deploy_ok: false,
        validate_output: null,
        error: `validate stdout is not valid JSON: ${stdout.slice(0, 200)}`,
      };
    }
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    return {
      slug,
      validate_ok: false,
      deploy_ok: false,
      validate_output: null,
      error: `validate subprocess failed: ${msg}`,
    };
  }

  const validateOk =
    typeof validateOutput === "object" &&
    validateOutput !== null &&
    (validateOutput as Record<string, unknown>)["ok"] === true;

  if (!validateOk) {
    return { slug, validate_ok: false, deploy_ok: false, validate_output: validateOutput };
  }

  // --- deploy (payload assembly only — no side effects, no --dry-run flag) ---
  let deployOutput: unknown;
  try {
    const { stdout } = await exec(cfaBinary, [
      "managed-agent",
      "deploy",
      slug,
      "--cookbooks-root",
      cookbooksRoot,
      "--skills-root",
      skillsRoot,
      "--agents-root",
      agentsRoot,
    ]);
    try {
      deployOutput = JSON.parse(stdout);
    } catch {
      return {
        slug,
        validate_ok: true,
        deploy_ok: false,
        validate_output: validateOutput,
        error: `deploy stdout is not valid JSON: ${stdout.slice(0, 200)}`,
      };
    }
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    return {
      slug,
      validate_ok: true,
      deploy_ok: false,
      validate_output: validateOutput,
      error: `deploy subprocess failed: ${msg}`,
    };
  }

  return {
    slug,
    validate_ok: true,
    deploy_ok: true,
    validate_output: validateOutput,
    deploy_output: deployOutput,
  };
}

// ---------------------------------------------------------------------------
// Orchestrator
// ---------------------------------------------------------------------------

export async function cookbookValidateAll(
  input: CookbookValidateInput,
): Promise<CookbookValidateAllReport> {
  const workspaceRoot = resolveWorkspaceRoot(input.workspace_root);
  const cookbooksRoot = input.cookbooks_root ?? join(workspaceRoot, "managed-agent-cookbooks");
  const skillsRoot = input.skills_root ?? join(workspaceRoot, "plugins", "cfa-core", "skills");
  const agentsRoot = input.agents_root ?? join(workspaceRoot, "plugins", "cfa-core", "agents", "cfa");
  const cfaBinary = input.cfa_binary ?? "cfa";
  const exec = input._exec ?? defaultExecFn;

  const slugs =
    input.slugs && input.slugs.length > 0
      ? input.slugs
      : discoverCookbookSlugs(cookbooksRoot);

  if (slugs.length === 0) {
    throw new Error(`No cookbooks discovered under ${cookbooksRoot}`);
  }

  const results: CookbookCheckResult[] = [];
  for (const slug of slugs) {
    const result = await validateOneCookbook({
      slug,
      cfaBinary,
      cookbooksRoot,
      skillsRoot,
      agentsRoot,
      exec,
    });
    results.push(result);
  }

  const passed = results.filter((r) => r.validate_ok && r.deploy_ok).length;
  const failed = results.length - passed;

  return {
    workspace_root: workspaceRoot,
    cookbooks_root: cookbooksRoot,
    total: results.length,
    passed,
    failed,
    results,
    all_ok: failed === 0,
  };
}

// ---------------------------------------------------------------------------
// MCP entry-point (JSON-in / JSON-out)
// ---------------------------------------------------------------------------

export async function cookbookValidateAllMcp(inputJson: string): Promise<string> {
  const parsed = JSON.parse(inputJson) as CookbookValidateInput;
  const report = await cookbookValidateAll(parsed);
  return JSON.stringify(report);
}

// ---------------------------------------------------------------------------
// CLI flag parser (no external deps)
// ---------------------------------------------------------------------------

export interface ParsedCliArgs {
  flags: Record<string, string>;
  slugs: string[];
}

export function parseCliArgs(argv: string[]): ParsedCliArgs {
  const flags: Record<string, string> = {};
  const slugs: string[] = [];
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i];
    if (a.startsWith("--")) {
      const key = a.slice(2);
      const next = argv[i + 1];
      if (next !== undefined && !next.startsWith("--")) {
        flags[key] = next;
        i++;
      } else {
        flags[key] = "true";
      }
    } else {
      slugs.push(a);
    }
  }
  return { flags, slugs };
}

// CLI invocation: `tsx cookbook.ts [--flag value ...] [slug ...]` — same code
// path as the MCP tool. Positional slugs validate just those; omit to validate
// all. Supported flags:
//   --skills-root <path>     default: plugins/cfa-core/skills
//   --agents-root <path>     default: plugins/cfa-core/agents/cfa
//   --cookbooks-root <path>  default: managed-agent-cookbooks
//   --cfa-binary <path>      default: cfa (from PATH)
if (import.meta.url === `file://${process.argv[1]}`) {
  (async () => {
    const { flags, slugs } = parseCliArgs(process.argv.slice(2));
    const workspaceRoot = resolveWorkspaceRoot();
    const report = await cookbookValidateAll({
      workspace_root: workspaceRoot,
      skills_root: flags["skills-root"] ?? join(workspaceRoot, "plugins", "cfa-core", "skills"),
      agents_root: flags["agents-root"] ?? join(workspaceRoot, "plugins", "cfa-core", "agents", "cfa"),
      cookbooks_root: flags["cookbooks-root"] ?? join(workspaceRoot, "managed-agent-cookbooks"),
      cfa_binary: flags["cfa-binary"] ?? "cfa",
      slugs: slugs.length > 0 ? slugs : undefined,
    });
    process.stdout.write(JSON.stringify(report, null, 2) + "\n");
    process.exit(report.all_ok ? 0 : 1);
  })().catch((err) => {
    process.stderr.write(`cookbook validation failed: ${err instanceof Error ? err.message : String(err)}\n`);
    process.exit(2);
  });
}
