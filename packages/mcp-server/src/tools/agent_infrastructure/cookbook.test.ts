/**
 * Unit tests for cookbook.ts (Phase 30 Wave 1).
 *
 * Uses the injectable _exec / exec parameter rather than mocking child_process
 * directly — avoids ChildProcess typing complications and keeps tests clean.
 */

import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { mkdtempSync, mkdirSync, writeFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import {
  discoverCookbookSlugs,
  validateOneCookbook,
  cookbookValidateAll,
  cookbookValidateAllMcp,
  parseCliArgs,
  type ExecFn,
} from "./cookbook.js";

// ---------------------------------------------------------------------------
// Exec helpers
// ---------------------------------------------------------------------------

function makeExecResolve(stdout: string): ExecFn {
  return async () => ({ stdout, stderr: "" });
}

function makeExecReject(message: string): ExecFn {
  return async () => {
    throw new Error(message);
  };
}

type ExecResponse = { stdout: string } | { error: string };

function makeExecSequence(responses: ExecResponse[]): ExecFn {
  let idx = 0;
  return async () => {
    const resp = responses[idx++] ?? { error: "no more mock responses" };
    if ("error" in resp) throw new Error(resp.error);
    return { stdout: resp.stdout, stderr: "" };
  };
}

// ---------------------------------------------------------------------------
// parseCliArgs
// ---------------------------------------------------------------------------

describe("parseCliArgs", () => {
  it("parses flags only", () => {
    const { flags, slugs } = parseCliArgs([
      "--skills-root", "/sk",
      "--agents-root", "/ag",
      "--cookbooks-root", "/cb",
    ]);
    expect(flags["skills-root"]).toBe("/sk");
    expect(flags["agents-root"]).toBe("/ag");
    expect(flags["cookbooks-root"]).toBe("/cb");
    expect(slugs).toEqual([]);
  });

  it("parses slugs only (no flags)", () => {
    const { flags, slugs } = parseCliArgs(["alpha", "bravo"]);
    expect(flags).toEqual({});
    expect(slugs).toEqual(["alpha", "bravo"]);
  });

  it("parses mixed flags and slugs", () => {
    const { flags, slugs } = parseCliArgs([
      "--skills-root", "/sk",
      "my-cookbook",
      "--cfa-binary", "/usr/bin/cfa",
    ]);
    expect(flags["skills-root"]).toBe("/sk");
    expect(flags["cfa-binary"]).toBe("/usr/bin/cfa");
    expect(slugs).toEqual(["my-cookbook"]);
  });

  it("returns empty flags and slugs for empty argv", () => {
    const { flags, slugs } = parseCliArgs([]);
    expect(flags).toEqual({});
    expect(slugs).toEqual([]);
  });

  it("treats a flag with no following value as 'true'", () => {
    const { flags } = parseCliArgs(["--verbose"]);
    expect(flags["verbose"]).toBe("true");
  });
});

// ---------------------------------------------------------------------------
// cookbookValidateAll defaults (skills-root / agents-root)
// ---------------------------------------------------------------------------

describe("cookbookValidateAll defaults", () => {
  let tmpRoot: string;

  beforeEach(() => {
    tmpRoot = mkdtempSync(join(tmpdir(), "cb-defaults-"));
  });

  afterEach(() => {
    rmSync(tmpRoot, { recursive: true, force: true });
  });

  it("uses plugins/cfa-core/skills when skills_root is omitted", async () => {
    const capturedArgs: string[][] = [];
    const _exec: ExecFn = async (_bin, args) => {
      capturedArgs.push(args);
      return { stdout: '{"ok":true}', stderr: "" };
    };
    // validate succeeds; deploy also needs a stub
    const _execFull: ExecFn = async (_bin, args) => {
      capturedArgs.push(args);
      return { stdout: '{"ok":true,"payload":"x"}', stderr: "" };
    };

    await cookbookValidateAll({
      workspace_root: tmpRoot,
      cookbooks_root: join(tmpRoot, "cb"),
      // skills_root and agents_root intentionally omitted
      slugs: ["my-agent"],
      _exec: _execFull,
    });

    const allArgs = capturedArgs.flat();
    const srIdx = allArgs.indexOf("--skills-root");
    expect(srIdx).toBeGreaterThan(-1);
    expect(allArgs[srIdx + 1]).toContain(join("plugins", "cfa-core", "skills"));

    const arIdx = allArgs.indexOf("--agents-root");
    expect(arIdx).toBeGreaterThan(-1);
    expect(allArgs[arIdx + 1]).toContain(join("plugins", "cfa-core", "agents", "cfa"));
  });

  it("flag values override defaults when supplied", async () => {
    const capturedArgs: string[][] = [];
    const _exec: ExecFn = async (_bin, args) => {
      capturedArgs.push(args);
      return { stdout: '{"ok":true,"payload":"x"}', stderr: "" };
    };

    await cookbookValidateAll({
      workspace_root: tmpRoot,
      cookbooks_root: join(tmpRoot, "cb"),
      skills_root: "/custom/skills",
      agents_root: "/custom/agents",
      slugs: ["my-agent"],
      _exec,
    });

    const allArgs = capturedArgs.flat();
    const srIdx = allArgs.indexOf("--skills-root");
    expect(allArgs[srIdx + 1]).toBe("/custom/skills");
    const arIdx = allArgs.indexOf("--agents-root");
    expect(allArgs[arIdx + 1]).toBe("/custom/agents");
  });
});

// ---------------------------------------------------------------------------
// discoverCookbookSlugs
// ---------------------------------------------------------------------------

describe("discoverCookbookSlugs", () => {
  let tmpRoot: string;

  beforeEach(() => {
    tmpRoot = mkdtempSync(join(tmpdir(), "cb-discover-"));
  });

  afterEach(() => {
    rmSync(tmpRoot, { recursive: true, force: true });
  });

  it("returns sorted slugs for directories that contain agent.json", () => {
    for (const slug of ["zebra", "alpha", "bravo"]) {
      const dir = join(tmpRoot, slug);
      mkdirSync(dir);
      writeFileSync(join(dir, "agent.json"), "{}");
    }
    expect(discoverCookbookSlugs(tmpRoot)).toEqual(["alpha", "bravo", "zebra"]);
  });

  it("excludes directories without agent.json", () => {
    mkdirSync(join(tmpRoot, "has-agent"));
    writeFileSync(join(tmpRoot, "has-agent", "agent.json"), "{}");
    mkdirSync(join(tmpRoot, "no-agent"));
    expect(discoverCookbookSlugs(tmpRoot)).toEqual(["has-agent"]);
  });

  it("returns empty array for non-existent root", () => {
    expect(discoverCookbookSlugs("/does-not-exist-xyz-abc")).toEqual([]);
  });
});

// ---------------------------------------------------------------------------
// validateOneCookbook
// ---------------------------------------------------------------------------

const baseInput = {
  slug: "test-slug",
  cfaBinary: "cfa",
  cookbooksRoot: "/cb",
  skillsRoot: "/sk",
  agentsRoot: "/ag",
};

describe("validateOneCookbook", () => {
  it("happy path: validate ok + deploy ok", async () => {
    const exec = makeExecSequence([
      { stdout: '{"ok":true,"checks":[]}' },
      { stdout: '{"payload":"assembled"}' },
    ]);

    const result = await validateOneCookbook({ ...baseInput, exec });
    expect(result.validate_ok).toBe(true);
    expect(result.deploy_ok).toBe(true);
    expect(result.validate_output).toEqual({ ok: true, checks: [] });
    expect(result.deploy_output).toEqual({ payload: "assembled" });
    expect(result.error).toBeUndefined();
  });

  it("validate ok=false: deploy is skipped", async () => {
    let callCount = 0;
    const exec: ExecFn = async () => {
      callCount++;
      return { stdout: '{"ok":false,"checks":[{"passed":false}]}', stderr: "" };
    };

    const result = await validateOneCookbook({ ...baseInput, exec });
    expect(result.validate_ok).toBe(false);
    expect(result.deploy_ok).toBe(false);
    expect(result.deploy_output).toBeUndefined();
    expect(callCount).toBe(1); // only validate, no deploy
  });

  it("validate ok + deploy subprocess fails", async () => {
    const exec = makeExecSequence([
      { stdout: '{"ok":true}' },
      { error: "deploy command failed" },
    ]);

    const result = await validateOneCookbook({ ...baseInput, exec });
    expect(result.validate_ok).toBe(true);
    expect(result.deploy_ok).toBe(false);
    expect(result.error).toContain("deploy subprocess failed");
  });

  it("validate subprocess throws: returns error", async () => {
    const exec = makeExecReject("binary not found");

    const result = await validateOneCookbook({ ...baseInput, exec });
    expect(result.validate_ok).toBe(false);
    expect(result.deploy_ok).toBe(false);
    expect(result.error).toContain("validate subprocess failed");
    expect(result.error).toContain("binary not found");
  });

  it("validate stdout not valid JSON: returns error", async () => {
    const exec = makeExecResolve("not json at all");

    const result = await validateOneCookbook({ ...baseInput, exec });
    expect(result.validate_ok).toBe(false);
    expect(result.error).toContain("not valid JSON");
  });

  it("deploy stdout not valid JSON: returns error with validate_ok true", async () => {
    const exec = makeExecSequence([
      { stdout: '{"ok":true}' },
      { stdout: "bad json" },
    ]);

    const result = await validateOneCookbook({ ...baseInput, exec });
    expect(result.validate_ok).toBe(true);
    expect(result.deploy_ok).toBe(false);
    expect(result.error).toContain("not valid JSON");
  });
});

// ---------------------------------------------------------------------------
// cookbookValidateAll
// ---------------------------------------------------------------------------

describe("cookbookValidateAll", () => {
  let tmpRoot: string;

  beforeEach(() => {
    tmpRoot = mkdtempSync(join(tmpdir(), "cb-all-"));
  });

  afterEach(() => {
    rmSync(tmpRoot, { recursive: true, force: true });
  });

  it("throws when no slugs discovered and none provided", async () => {
    await expect(
      cookbookValidateAll({
        workspace_root: tmpRoot,
        cookbooks_root: join(tmpRoot, "managed-agent-cookbooks"),
      }),
    ).rejects.toThrow(/No cookbooks discovered/);
  });

  it("processes explicit slugs list and reports all_ok=true", async () => {
    const _exec = makeExecSequence([
      { stdout: '{"ok":true}' },
      { stdout: '{"payload":1}' },
      { stdout: '{"ok":true}' },
      { stdout: '{"payload":2}' },
    ]);

    const report = await cookbookValidateAll({
      workspace_root: tmpRoot,
      cookbooks_root: join(tmpRoot, "cb"),
      skills_root: join(tmpRoot, "sk"),
      agents_root: join(tmpRoot, "ag"),
      slugs: ["alpha", "bravo"],
      _exec,
    });

    expect(report.total).toBe(2);
    expect(report.passed).toBe(2);
    expect(report.failed).toBe(0);
    expect(report.all_ok).toBe(true);
  });

  it("reports all_ok=false when one cookbook fails", async () => {
    const _exec = makeExecSequence([
      { stdout: '{"ok":false}' }, // alpha validate fails
      { stdout: '{"ok":true}' },  // bravo validate
      { stdout: '{"payload":2}' }, // bravo deploy
    ]);

    const report = await cookbookValidateAll({
      workspace_root: tmpRoot,
      cookbooks_root: join(tmpRoot, "cb"),
      skills_root: join(tmpRoot, "sk"),
      agents_root: join(tmpRoot, "ag"),
      slugs: ["alpha", "bravo"],
      _exec,
    });

    expect(report.passed).toBe(1);
    expect(report.failed).toBe(1);
    expect(report.all_ok).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// cookbookValidateAllMcp
// ---------------------------------------------------------------------------

describe("cookbookValidateAllMcp", () => {
  let tmpRoot: string;

  beforeEach(() => {
    tmpRoot = mkdtempSync(join(tmpdir(), "cb-mcp-"));
  });

  afterEach(() => {
    rmSync(tmpRoot, { recursive: true, force: true });
  });

  it("returns valid JSON string", async () => {
    // cookbookValidateAllMcp parses JSON then calls cookbookValidateAll.
    // _exec cannot be passed through the JSON boundary, so we stub a cookbook
    // directory and rely on the subprocess failing gracefully.
    const cbRoot = join(tmpRoot, "cb");
    const slugDir = join(cbRoot, "my-agent");
    mkdirSync(slugDir, { recursive: true });
    writeFileSync(join(slugDir, "agent.json"), "{}");

    // The cfa binary won't be found, so we expect an error result, but valid JSON.
    const json = await cookbookValidateAllMcp(
      JSON.stringify({
        workspace_root: tmpRoot,
        cookbooks_root: cbRoot,
        skills_root: join(tmpRoot, "sk"),
        agents_root: join(tmpRoot, "ag"),
        slugs: ["my-agent"],
      }),
    );

    const parsed = JSON.parse(json) as Record<string, unknown>;
    expect(typeof parsed["total"]).toBe("number");
    expect(typeof parsed["all_ok"]).toBe("boolean");
    // With no real cfa binary, all_ok should be false.
    expect(parsed["all_ok"]).toBe(false);
  });
});
