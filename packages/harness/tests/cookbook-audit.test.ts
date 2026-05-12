/**
 * Phase 25 Tier A2 — cookbook audit hashing.
 *
 * Pure-function tests using inline fixtures and temp dirs. No git, no
 * network. Covers determinism, single-file change detection, slug
 * isolation, file inclusion rules (whitelist + dir exclusions), reference
 * resolution (system.file, skills[].from_plugin), and catalog diffing.
 */

import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { mkdtempSync, mkdirSync, writeFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import {
  auditCookbook,
  auditAllCookbooks,
  serialiseAuditCatalog,
  parseAuditCatalog,
  diffAuditCatalogs,
  type CookbookAuditCatalog,
} from "../src/manifests/cookbook-audit.js";

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

interface Fixture {
  root: string;
  cleanup: () => void;
}

function makeFixture(): Fixture {
  const root = mkdtempSync(join(tmpdir(), "cookbook-audit-"));
  return { root, cleanup: () => rmSync(root, { recursive: true, force: true }) };
}

function writeFile(root: string, rel: string, content: string): void {
  const full = join(root, rel);
  mkdirSync(join(full, ".."), { recursive: true });
  writeFileSync(full, content, "utf8");
}

function makeMinimalCookbook(
  root: string,
  slug: string,
  agentExtra = "",
): void {
  writeFile(
    root,
    `managed-agent-cookbooks/${slug}/agent.yaml`,
    `name: ${slug}\nmodel: claude-opus-4-7\nsystem:\n  text: hello\n${agentExtra}`,
  );
  writeFile(
    root,
    `managed-agent-cookbooks/${slug}/steering-examples.json`,
    `{"examples":[]}`,
  );
}

// ---------------------------------------------------------------------------
// auditCookbook — basics
// ---------------------------------------------------------------------------

describe("auditCookbook — single cookbook", () => {
  let fx: Fixture;
  beforeEach(() => {
    fx = makeFixture();
  });
  afterEach(() => fx.cleanup());

  it("hashes the cookbook directory contents deterministically", () => {
    makeMinimalCookbook(fx.root, "alpha");
    const a = auditCookbook({
      repoRoot: fx.root,
      cookbookDir: join(fx.root, "managed-agent-cookbooks/alpha"),
    });
    const b = auditCookbook({
      repoRoot: fx.root,
      cookbookDir: join(fx.root, "managed-agent-cookbooks/alpha"),
    });
    expect(a.hash).toBe(b.hash);
    expect(a.hash).toMatch(/^[a-f0-9]{64}$/);
    expect(a.slug).toBe("alpha");
  });

  it("includes agent.yaml and steering-examples.json in the inventory", () => {
    makeMinimalCookbook(fx.root, "alpha");
    const audit = auditCookbook({
      repoRoot: fx.root,
      cookbookDir: join(fx.root, "managed-agent-cookbooks/alpha"),
    });
    const paths = audit.files.map((f) => f.path);
    expect(paths).toContain("managed-agent-cookbooks/alpha/agent.yaml");
    expect(paths).toContain(
      "managed-agent-cookbooks/alpha/steering-examples.json",
    );
  });

  it("changes the master hash when any file content changes", () => {
    makeMinimalCookbook(fx.root, "alpha");
    const before = auditCookbook({
      repoRoot: fx.root,
      cookbookDir: join(fx.root, "managed-agent-cookbooks/alpha"),
    });
    writeFile(
      fx.root,
      "managed-agent-cookbooks/alpha/agent.yaml",
      `name: alpha\nmodel: claude-opus-4-7\nsystem:\n  text: CHANGED\n`,
    );
    const after = auditCookbook({
      repoRoot: fx.root,
      cookbookDir: join(fx.root, "managed-agent-cookbooks/alpha"),
    });
    expect(after.hash).not.toBe(before.hash);
  });

  it("changes hash when a new file is added", () => {
    makeMinimalCookbook(fx.root, "alpha");
    const before = auditCookbook({
      repoRoot: fx.root,
      cookbookDir: join(fx.root, "managed-agent-cookbooks/alpha"),
    });
    writeFile(fx.root, "managed-agent-cookbooks/alpha/README.md", "hi");
    const after = auditCookbook({
      repoRoot: fx.root,
      cookbookDir: join(fx.root, "managed-agent-cookbooks/alpha"),
    });
    expect(after.hash).not.toBe(before.hash);
    expect(after.files.length).toBe(before.files.length + 1);
  });

  it("returns files sorted alphabetically by path", () => {
    writeFile(fx.root, "managed-agent-cookbooks/x/agent.yaml", "name: x\n");
    writeFile(fx.root, "managed-agent-cookbooks/x/zzz.md", "z");
    writeFile(fx.root, "managed-agent-cookbooks/x/aaa.md", "a");
    writeFile(fx.root, "managed-agent-cookbooks/x/mmm.md", "m");
    const audit = auditCookbook({
      repoRoot: fx.root,
      cookbookDir: join(fx.root, "managed-agent-cookbooks/x"),
    });
    const paths = audit.files.map((f) => f.path);
    const sorted = [...paths].sort();
    expect(paths).toEqual(sorted);
  });

  it("captures the parent agent.yaml version field on the audit", () => {
    writeFile(
      fx.root,
      "managed-agent-cookbooks/alpha/agent.yaml",
      `name: alpha\nversion: "1.2.3"\nsystem:\n  text: ok\n`,
    );
    const audit = auditCookbook({
      repoRoot: fx.root,
      cookbookDir: join(fx.root, "managed-agent-cookbooks/alpha"),
    });
    expect(audit.version).toBe("1.2.3");
  });

  it("returns version='' when parent manifest omits version", () => {
    writeFile(
      fx.root,
      "managed-agent-cookbooks/alpha/agent.yaml",
      `name: alpha\nsystem:\n  text: ok\n`,
    );
    const audit = auditCookbook({
      repoRoot: fx.root,
      cookbookDir: join(fx.root, "managed-agent-cookbooks/alpha"),
    });
    expect(audit.version).toBe("");
  });
});

// ---------------------------------------------------------------------------
// File inclusion — whitelist + exclusions
// ---------------------------------------------------------------------------

describe("auditCookbook — file inclusion rules", () => {
  let fx: Fixture;
  beforeEach(() => {
    fx = makeFixture();
  });
  afterEach(() => fx.cleanup());

  it("includes .yaml, .yml, .json, .md, .txt", () => {
    writeFile(fx.root, "managed-agent-cookbooks/a/agent.yaml", "name: a");
    writeFile(fx.root, "managed-agent-cookbooks/a/aux.yml", "k: v");
    writeFile(fx.root, "managed-agent-cookbooks/a/data.json", "{}");
    writeFile(fx.root, "managed-agent-cookbooks/a/readme.md", "hi");
    writeFile(fx.root, "managed-agent-cookbooks/a/notes.txt", "n");
    const audit = auditCookbook({
      repoRoot: fx.root,
      cookbookDir: join(fx.root, "managed-agent-cookbooks/a"),
    });
    expect(audit.files).toHaveLength(5);
  });

  it("excludes non-text extensions (.png, .ts, .js, etc.)", () => {
    writeFile(fx.root, "managed-agent-cookbooks/a/agent.yaml", "name: a");
    writeFile(fx.root, "managed-agent-cookbooks/a/code.ts", "x");
    writeFile(fx.root, "managed-agent-cookbooks/a/build.js", "y");
    writeFile(fx.root, "managed-agent-cookbooks/a/image.png", "z");
    const audit = auditCookbook({
      repoRoot: fx.root,
      cookbookDir: join(fx.root, "managed-agent-cookbooks/a"),
    });
    expect(audit.files).toHaveLength(1);
    expect(audit.files[0]?.path).toMatch(/agent\.yaml$/);
  });

  it("excludes node_modules / dist / .git / __pycache__ recursively", () => {
    writeFile(fx.root, "managed-agent-cookbooks/a/agent.yaml", "name: a");
    writeFile(
      fx.root,
      "managed-agent-cookbooks/a/node_modules/pkg/index.yaml",
      "x",
    );
    writeFile(fx.root, "managed-agent-cookbooks/a/dist/bundle.json", "{}");
    writeFile(fx.root, "managed-agent-cookbooks/a/.git/config.txt", "n");
    const audit = auditCookbook({
      repoRoot: fx.root,
      cookbookDir: join(fx.root, "managed-agent-cookbooks/a"),
    });
    expect(audit.files).toHaveLength(1);
  });

  it("descends into subdirectories (subagents/)", () => {
    writeFile(fx.root, "managed-agent-cookbooks/a/agent.yaml", "name: a");
    writeFile(
      fx.root,
      "managed-agent-cookbooks/a/subagents/x.yaml",
      "name: x",
    );
    writeFile(
      fx.root,
      "managed-agent-cookbooks/a/subagents/y.yaml",
      "name: y",
    );
    const audit = auditCookbook({
      repoRoot: fx.root,
      cookbookDir: join(fx.root, "managed-agent-cookbooks/a"),
    });
    expect(audit.files).toHaveLength(3);
  });
});

// ---------------------------------------------------------------------------
// Referenced content — system.file + skills[].from_plugin
// ---------------------------------------------------------------------------

describe("auditCookbook — referenced files", () => {
  let fx: Fixture;
  beforeEach(() => {
    fx = makeFixture();
  });
  afterEach(() => fx.cleanup());

  it("includes the resolved system.file content", () => {
    writeFile(
      fx.root,
      "plugins/cfa-core/agents/cfa/equity-analyst.md",
      "system prompt body",
    );
    writeFile(
      fx.root,
      "managed-agent-cookbooks/eq/agent.yaml",
      [
        `name: eq`,
        `model: claude-opus-4-7`,
        `system:`,
        `  file: ../../plugins/cfa-core/agents/cfa/equity-analyst.md`,
      ].join("\n"),
    );
    const audit = auditCookbook({
      repoRoot: fx.root,
      cookbookDir: join(fx.root, "managed-agent-cookbooks/eq"),
    });
    const paths = audit.files.map((f) => f.path);
    expect(paths).toContain(
      "plugins/cfa-core/agents/cfa/equity-analyst.md",
    );
  });

  it("hash changes when a referenced system prompt changes", () => {
    writeFile(
      fx.root,
      "plugins/cfa-core/agents/cfa/equity-analyst.md",
      "original",
    );
    writeFile(
      fx.root,
      "managed-agent-cookbooks/eq/agent.yaml",
      [
        `name: eq`,
        `model: claude-opus-4-7`,
        `system:`,
        `  file: ../../plugins/cfa-core/agents/cfa/equity-analyst.md`,
      ].join("\n"),
    );
    const before = auditCookbook({
      repoRoot: fx.root,
      cookbookDir: join(fx.root, "managed-agent-cookbooks/eq"),
    });
    writeFile(
      fx.root,
      "plugins/cfa-core/agents/cfa/equity-analyst.md",
      "edited",
    );
    const after = auditCookbook({
      repoRoot: fx.root,
      cookbookDir: join(fx.root, "managed-agent-cookbooks/eq"),
    });
    expect(after.hash).not.toBe(before.hash);
  });

  it("includes SKILL.md from each skills[].from_plugin directory", () => {
    writeFile(
      fx.root,
      "plugins/vertical-plugins/er/skills/coverage/SKILL.md",
      "coverage skill body",
    );
    writeFile(
      fx.root,
      "managed-agent-cookbooks/eq/agent.yaml",
      [
        `name: eq`,
        `model: claude-opus-4-7`,
        `system:`,
        `  text: ok`,
        `skills:`,
        `  - { from_plugin: ../../plugins/vertical-plugins/er/skills/coverage }`,
      ].join("\n"),
    );
    const audit = auditCookbook({
      repoRoot: fx.root,
      cookbookDir: join(fx.root, "managed-agent-cookbooks/eq"),
    });
    const paths = audit.files.map((f) => f.path);
    expect(paths).toContain(
      "plugins/vertical-plugins/er/skills/coverage/SKILL.md",
    );
  });

  it("hash changes when a referenced skill body changes", () => {
    writeFile(
      fx.root,
      "plugins/vertical-plugins/er/skills/coverage/SKILL.md",
      "original",
    );
    writeFile(
      fx.root,
      "managed-agent-cookbooks/eq/agent.yaml",
      [
        `name: eq`,
        `model: claude-opus-4-7`,
        `system:`,
        `  text: ok`,
        `skills:`,
        `  - { from_plugin: ../../plugins/vertical-plugins/er/skills/coverage }`,
      ].join("\n"),
    );
    const before = auditCookbook({
      repoRoot: fx.root,
      cookbookDir: join(fx.root, "managed-agent-cookbooks/eq"),
    });
    writeFile(
      fx.root,
      "plugins/vertical-plugins/er/skills/coverage/SKILL.md",
      "edited",
    );
    const after = auditCookbook({
      repoRoot: fx.root,
      cookbookDir: join(fx.root, "managed-agent-cookbooks/eq"),
    });
    expect(after.hash).not.toBe(before.hash);
  });

  it("ignores skills[].from_plugin paths that do not exist", () => {
    writeFile(
      fx.root,
      "managed-agent-cookbooks/eq/agent.yaml",
      [
        `name: eq`,
        `model: claude-opus-4-7`,
        `system:`,
        `  text: ok`,
        `skills:`,
        `  - { from_plugin: ../../plugins/no-such-skill }`,
      ].join("\n"),
    );
    expect(() =>
      auditCookbook({
        repoRoot: fx.root,
        cookbookDir: join(fx.root, "managed-agent-cookbooks/eq"),
      }),
    ).not.toThrow();
  });

  it("walks subagent manifests for their own system.file + skills references", () => {
    writeFile(
      fx.root,
      "plugins/cfa-core/skills/sub-skill/SKILL.md",
      "sub body",
    );
    writeFile(
      fx.root,
      "managed-agent-cookbooks/multi/agent.yaml",
      `name: multi\nmodel: claude-opus-4-7\nsystem:\n  text: ok\n`,
    );
    writeFile(
      fx.root,
      "managed-agent-cookbooks/multi/subagents/worker.yaml",
      [
        `name: worker`,
        `system:`,
        `  text: ok`,
        `skills:`,
        `  - { from_plugin: ../../../plugins/cfa-core/skills/sub-skill }`,
      ].join("\n"),
    );
    const audit = auditCookbook({
      repoRoot: fx.root,
      cookbookDir: join(fx.root, "managed-agent-cookbooks/multi"),
    });
    expect(audit.files.map((f) => f.path)).toContain(
      "plugins/cfa-core/skills/sub-skill/SKILL.md",
    );
  });
});

// ---------------------------------------------------------------------------
// auditAllCookbooks
// ---------------------------------------------------------------------------

describe("auditAllCookbooks", () => {
  let fx: Fixture;
  beforeEach(() => {
    fx = makeFixture();
  });
  afterEach(() => fx.cleanup());

  it("audits every cookbook with an agent manifest", () => {
    makeMinimalCookbook(fx.root, "alpha");
    makeMinimalCookbook(fx.root, "beta");
    makeMinimalCookbook(fx.root, "gamma");
    const catalog = auditAllCookbooks({
      repoRoot: fx.root,
      cookbooksRoot: join(fx.root, "managed-agent-cookbooks"),
    });
    expect(catalog.cookbooks.map((c) => c.slug)).toEqual([
      "alpha",
      "beta",
      "gamma",
    ]);
  });

  it("skips directories that lack an agent manifest", () => {
    makeMinimalCookbook(fx.root, "real");
    // Subdir with no agent.yaml → ignored
    writeFile(
      fx.root,
      "managed-agent-cookbooks/junk/notes.md",
      "no manifest here",
    );
    const catalog = auditAllCookbooks({
      repoRoot: fx.root,
      cookbooksRoot: join(fx.root, "managed-agent-cookbooks"),
    });
    expect(catalog.cookbooks.map((c) => c.slug)).toEqual(["real"]);
  });

  it("isolates per-cookbook hashes", () => {
    makeMinimalCookbook(fx.root, "alpha");
    makeMinimalCookbook(fx.root, "beta");
    const catalog = auditAllCookbooks({
      repoRoot: fx.root,
      cookbooksRoot: join(fx.root, "managed-agent-cookbooks"),
    });
    const [a, b] = catalog.cookbooks;
    expect(a?.hash).not.toBe(b?.hash);
  });

  it("returns empty catalog when cookbooks dir is absent", () => {
    const catalog = auditAllCookbooks({
      repoRoot: fx.root,
      cookbooksRoot: join(fx.root, "no-such-dir"),
    });
    expect(catalog.cookbooks).toEqual([]);
  });
});

// ---------------------------------------------------------------------------
// Serialisation determinism
// ---------------------------------------------------------------------------

describe("serialiseAuditCatalog", () => {
  it("is byte-deterministic across two calls", () => {
    const c: CookbookAuditCatalog = {
      version: "1",
      cookbooks: [
        {
          slug: "alpha",
          version: "1.0.0",
          hash: "h1",
          files: [
            { path: "a/agent.yaml", sha256: "x", size: 1 },
            { path: "a/notes.md", sha256: "y", size: 2 },
          ],
        },
      ],
    };
    expect(serialiseAuditCatalog(c)).toBe(serialiseAuditCatalog(c));
  });

  it("orders cookbooks alphabetically regardless of input order", () => {
    const c: CookbookAuditCatalog = {
      version: "1",
      cookbooks: [
        { slug: "zzz", version: "1.0.0", hash: "z", files: [] },
        { slug: "aaa", version: "1.0.0", hash: "a", files: [] },
      ],
    };
    const out = serialiseAuditCatalog(c);
    expect(out.indexOf('"aaa"')).toBeLessThan(out.indexOf('"zzz"'));
  });

  it("round-trips through parseAuditCatalog", () => {
    const c: CookbookAuditCatalog = {
      version: "1",
      cookbooks: [
        {
          slug: "alpha",
          version: "1.0.0",
          hash: "h",
          files: [{ path: "p", sha256: "s", size: 7 }],
        },
      ],
    };
    expect(parseAuditCatalog(serialiseAuditCatalog(c))).toEqual(c);
  });

  it("ends with a trailing newline", () => {
    const out = serialiseAuditCatalog({ version: "1", cookbooks: [] });
    expect(out.endsWith("\n")).toBe(true);
  });
});

describe("parseAuditCatalog — validation", () => {
  it("rejects JSON without a cookbooks array", () => {
    expect(() => parseAuditCatalog("{}")).toThrow(/cookbooks/);
  });

  it("rejects cookbooks missing slug/hash/files", () => {
    expect(() =>
      parseAuditCatalog(JSON.stringify({ cookbooks: [{ slug: "x" }] })),
    ).toThrow(/slug.*hash.*files/);
  });
});

// ---------------------------------------------------------------------------
// diffAuditCatalogs
// ---------------------------------------------------------------------------

describe("diffAuditCatalogs", () => {
  function cb(slug: string, hash: string): {
    slug: string;
    version: string;
    hash: string;
    files: never[];
  } {
    return { slug, version: "1.0.0", hash, files: [] };
  }

  it("returns empty diff for identical catalogs", () => {
    const a: CookbookAuditCatalog = {
      version: "1",
      cookbooks: [cb("alpha", "h"), cb("beta", "g")],
    };
    const diff = diffAuditCatalogs(a, a);
    expect(diff.added).toEqual([]);
    expect(diff.removed).toEqual([]);
    expect(diff.changed).toEqual([]);
  });

  it("flags added cookbooks", () => {
    const before: CookbookAuditCatalog = { version: "1", cookbooks: [] };
    const after: CookbookAuditCatalog = {
      version: "1",
      cookbooks: [cb("new", "h")],
    };
    expect(diffAuditCatalogs(before, after).added).toEqual(["new"]);
  });

  it("flags removed cookbooks", () => {
    const before: CookbookAuditCatalog = {
      version: "1",
      cookbooks: [cb("old", "h")],
    };
    const after: CookbookAuditCatalog = { version: "1", cookbooks: [] };
    expect(diffAuditCatalogs(before, after).removed).toEqual(["old"]);
  });

  it("flags changed cookbooks with previous + current hash", () => {
    const before: CookbookAuditCatalog = {
      version: "1",
      cookbooks: [cb("alpha", "h1")],
    };
    const after: CookbookAuditCatalog = {
      version: "1",
      cookbooks: [cb("alpha", "h2")],
    };
    const diff = diffAuditCatalogs(before, after);
    expect(diff.changed).toEqual([
      { slug: "alpha", previous_hash: "h1", current_hash: "h2" },
    ]);
  });
});
