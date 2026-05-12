/**
 * Cookbook audit hashing — Phase 25 Tier A2.
 *
 * Produces a byte-stable sha256 over every file that defines a cookbook's
 * behavior: the cookbook directory itself (agent.yaml, subagents/*.yaml,
 * steering-examples.json, README), the resolved system.file for each agent
 * + subagent, and every file under each resolved skills[].from_plugin
 * directory. Two runs against the same disk state produce byte-identical
 * output.
 *
 * Use cases:
 *   - Deploy-time mismatch: deployed hash != current hash → block deploy.
 *   - PR review: hash diff in data/cookbook-audits.json shows which
 *     cookbooks changed (and which files inside them).
 *   - Audit: prove a specific version was deployed at a specific time.
 *
 * Pure library — no I/O beyond fs.readFileSync / readdirSync. CLI runner
 * lives in `scripts/generate-cookbook-audits.ts`.
 */

import { readFileSync, readdirSync, existsSync, statSync } from "node:fs";
import { createHash } from "node:crypto";
import { join, relative, resolve, dirname } from "node:path";
import { parse as parseYaml } from "yaml";

import type { AgentManifest, SkillRef } from "./types.js";

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

export interface FileEntry {
  /** Path relative to repo root. */
  path: string;
  /** sha256 hex of file bytes. */
  sha256: string;
  /** File size in bytes. */
  size: number;
}

export interface CookbookAudit {
  /** Cookbook slug (parent directory name under managed-agent-cookbooks/). */
  slug: string;
  /**
   * Semver release version from the parent agent.yaml's `version:` field.
   * Phase 25 Tier C2. Empty string when the parent manifest omits version
   * (caught by contract MA-008). Informational — not folded into the hash
   * separately because the version is already part of agent.yaml bytes.
   */
  version: string;
  /** Master hash: sha256 of the canonical JSON of files[]. */
  hash: string;
  /** Sorted file inventory (alphabetical by path). */
  files: FileEntry[];
}

export interface CookbookAuditCatalog {
  /** Format version. Bump when JSON shape changes. */
  version: string;
  /** Audit per cookbook, sorted alphabetically by slug. */
  cookbooks: CookbookAudit[];
}

// ---------------------------------------------------------------------------
// File inclusion — whitelist of text extensions
// ---------------------------------------------------------------------------

const TEXT_EXTENSIONS = new Set([".yaml", ".yml", ".json", ".md", ".txt"]);
const EXCLUDED_DIRS = new Set(["node_modules", "dist", ".git", "__pycache__"]);

function fileExt(name: string): string {
  const dot = name.lastIndexOf(".");
  return dot >= 0 ? name.slice(dot).toLowerCase() : "";
}

function isIncluded(name: string): boolean {
  return TEXT_EXTENSIONS.has(fileExt(name));
}

function collectFiles(dir: string): string[] {
  if (!existsSync(dir) || !statSync(dir).isDirectory()) return [];
  const out: string[] = [];
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    if (EXCLUDED_DIRS.has(entry.name)) continue;
    const full = join(dir, entry.name);
    if (entry.isDirectory()) {
      out.push(...collectFiles(full));
    } else if (entry.isFile() && isIncluded(entry.name)) {
      out.push(full);
    }
  }
  return out;
}

// ---------------------------------------------------------------------------
// Hash + canonical serialisation
// ---------------------------------------------------------------------------

function sha256(bytes: Buffer): string {
  return createHash("sha256").update(bytes).digest("hex");
}

function hashFile(absPath: string, repoRoot: string): FileEntry {
  const bytes = readFileSync(absPath);
  return {
    path: relative(repoRoot, absPath).split("\\").join("/"),
    sha256: sha256(bytes),
    size: bytes.length,
  };
}

/**
 * Canonical JSON for a single audit: 2-space indent, key order
 * {slug, files}, files sorted alphabetically by path. Used as input to
 * the master hash so the hash is byte-stable. Version is intentionally
 * excluded — it lives inside agent.yaml bytes already, so any version
 * bump changes the hash via the file content rather than a separate
 * field. This keeps the hash function focused on "what bytes are
 * shipping".
 */
function canonicalAuditJson(input: {
  slug: string;
  files: FileEntry[];
}): string {
  const sortedFiles = [...input.files].sort((a, b) =>
    a.path.localeCompare(b.path),
  );
  return (
    JSON.stringify(
      {
        slug: input.slug,
        files: sortedFiles.map((f) => ({
          path: f.path,
          sha256: f.sha256,
          size: f.size,
        })),
      },
      null,
      2,
    ) + "\n"
  );
}

// ---------------------------------------------------------------------------
// Manifest parsing (best-effort — failures degrade gracefully)
// ---------------------------------------------------------------------------

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

/**
 * Resolve a manifest-relative path to an absolute path, returning null if
 * the file/directory does not exist. The caller decides whether to ignore
 * missing references or fail.
 */
function resolveRef(manifestDir: string, ref: string): string | null {
  const abs = resolve(manifestDir, ref);
  return existsSync(abs) ? abs : null;
}

function collectSystemFile(
  manifest: AgentManifest,
  manifestDir: string,
): string[] {
  if (!manifest.system?.file) return [];
  const abs = resolveRef(manifestDir, manifest.system.file);
  return abs ? [abs] : [];
}

function collectSkillFiles(skills: SkillRef[], manifestDir: string): string[] {
  const out: string[] = [];
  for (const ref of skills) {
    if (!ref.from_plugin) continue;
    const abs = resolveRef(manifestDir, ref.from_plugin);
    if (!abs) continue;
    out.push(...collectFiles(abs));
  }
  return out;
}

function collectReferencedFiles(
  manifest: AgentManifest,
  manifestDir: string,
): string[] {
  return [
    ...collectSystemFile(manifest, manifestDir),
    ...collectSkillFiles(manifest.skills ?? [], manifestDir),
  ];
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

export interface AuditCookbookInput {
  repoRoot: string;
  cookbookDir: string;
  /** Slug override; defaults to basename(cookbookDir). */
  slug?: string;
}

/**
 * Hash a single cookbook. The slug is the cookbook directory name unless
 * overridden. The returned audit is deterministic — two runs against the
 * same disk state produce byte-identical output (including the master
 * hash).
 */
export function auditCookbook(input: AuditCookbookInput): CookbookAudit {
  const slug =
    input.slug ?? input.cookbookDir.split(/[/\\]/).filter(Boolean).pop() ?? "";

  // 1. All files directly under the cookbook directory.
  const directFiles = collectFiles(input.cookbookDir);

  // 2. Walk every manifest (parent + subagents) for system.file + skill refs.
  //    Also capture the parent manifest's version field.
  const referenced = new Set<string>();
  let version = "";
  for (const manifestPath of directFiles.filter((p) =>
    /agent\.yaml$|agent\.yml$|\.yaml$|\.yml$/.test(p),
  )) {
    const manifest = readManifest(manifestPath);
    if (!manifest) continue;
    // Parent manifest carries the cookbook version.
    if (/\/agent\.(yaml|yml|json)$/.test(manifestPath) && manifest.version) {
      version = manifest.version;
    }
    for (const ref of collectReferencedFiles(manifest, dirname(manifestPath))) {
      referenced.add(ref);
    }
  }

  // 3. Hash every file (direct + referenced), dedupe by absolute path.
  const allFiles = Array.from(new Set([...directFiles, ...referenced]));
  const entries: FileEntry[] = allFiles
    .map((p) => hashFile(p, input.repoRoot))
    .sort((a, b) => a.path.localeCompare(b.path));

  // 4. Master hash = sha256 of canonical JSON of files[]. Version isn't
  //    folded in separately because it lives inside agent.yaml bytes
  //    already; surfacing it on the audit object is purely informational.
  const canonical = canonicalAuditJson({ slug, files: entries });
  const masterHash = sha256(Buffer.from(canonical, "utf8"));

  return { slug, version, hash: masterHash, files: entries };
}

export interface AuditAllInput {
  repoRoot: string;
  /** Absolute path to managed-agent-cookbooks directory. */
  cookbooksRoot: string;
}

/**
 * Audit every cookbook discovered under cookbooksRoot. Cookbooks are
 * subdirectories that contain an agent.yaml, agent.yml, or agent.json.
 * Results are sorted alphabetically by slug.
 */
export function auditAllCookbooks(input: AuditAllInput): CookbookAuditCatalog {
  const cookbooks: CookbookAudit[] = [];
  if (!existsSync(input.cookbooksRoot)) {
    return { version: "1", cookbooks: [] };
  }

  const slugs = readdirSync(input.cookbooksRoot, { withFileTypes: true })
    .filter((e) => e.isDirectory())
    .map((e) => e.name)
    .filter((slug) => {
      const dir = join(input.cookbooksRoot, slug);
      return (
        existsSync(join(dir, "agent.yaml")) ||
        existsSync(join(dir, "agent.yml")) ||
        existsSync(join(dir, "agent.json"))
      );
    })
    .sort();

  for (const slug of slugs) {
    cookbooks.push(
      auditCookbook({
        repoRoot: input.repoRoot,
        cookbookDir: join(input.cookbooksRoot, slug),
        slug,
      }),
    );
  }

  return { version: "1", cookbooks };
}

/**
 * Stable JSON serialisation: 2-space indent, alphabetical cookbook order,
 * alphabetical file order within each cookbook, trailing newline. Two runs
 * against the same catalog produce byte-identical strings.
 */
export function serialiseAuditCatalog(catalog: CookbookAuditCatalog): string {
  const sortedCookbooks = [...catalog.cookbooks].sort((a, b) =>
    a.slug.localeCompare(b.slug),
  );
  const ordered: CookbookAuditCatalog = {
    version: catalog.version,
    cookbooks: sortedCookbooks.map((c) => ({
      slug: c.slug,
      version: c.version ?? "",
      hash: c.hash,
      files: [...c.files]
        .sort((a, b) => a.path.localeCompare(b.path))
        .map((f) => ({ path: f.path, sha256: f.sha256, size: f.size })),
    })),
  };
  return JSON.stringify(ordered, null, 2) + "\n";
}

export function parseAuditCatalog(json: string): CookbookAuditCatalog {
  const parsed = JSON.parse(json) as unknown;
  if (
    typeof parsed !== "object" ||
    parsed === null ||
    !("cookbooks" in parsed) ||
    !Array.isArray((parsed as { cookbooks: unknown }).cookbooks)
  ) {
    throw new Error("audit catalog JSON missing 'cookbooks' array");
  }
  const obj = parsed as {
    version?: string;
    cookbooks: Array<{
      slug?: unknown;
      hash?: unknown;
      files?: unknown;
    }>;
  };
  const cookbooks: CookbookAudit[] = obj.cookbooks.map((c, i) => {
    if (
      typeof c.slug !== "string" ||
      typeof c.hash !== "string" ||
      !Array.isArray(c.files)
    ) {
      throw new Error(
        `audit catalog cookbook[${i}] missing required fields slug/hash/files`,
      );
    }
    // version is optional in the legacy on-disk shape; default to "".
    const version =
      typeof (c as { version?: unknown }).version === "string"
        ? ((c as { version: string }).version)
        : "";
    const files = (c.files as unknown[]).map((f, j) => {
      if (
        typeof f !== "object" ||
        f === null ||
        typeof (f as { path?: unknown }).path !== "string" ||
        typeof (f as { sha256?: unknown }).sha256 !== "string" ||
        typeof (f as { size?: unknown }).size !== "number"
      ) {
        throw new Error(
          `audit catalog cookbook[${i}].files[${j}] missing required fields`,
        );
      }
      const e = f as { path: string; sha256: string; size: number };
      return { path: e.path, sha256: e.sha256, size: e.size };
    });
    return { slug: c.slug, version, hash: c.hash, files };
  });
  return { version: obj.version ?? "1", cookbooks };
}

/**
 * Compare two catalogs and report cookbook-level differences. Useful for
 * CI / PR review summaries.
 */
export interface AuditDiff {
  added: string[];
  removed: string[];
  changed: Array<{ slug: string; previous_hash: string; current_hash: string }>;
}

export function diffAuditCatalogs(
  previous: CookbookAuditCatalog,
  current: CookbookAuditCatalog,
): AuditDiff {
  const prevMap = new Map(previous.cookbooks.map((c) => [c.slug, c.hash]));
  const currMap = new Map(current.cookbooks.map((c) => [c.slug, c.hash]));

  const added: string[] = [];
  const changed: AuditDiff["changed"] = [];
  for (const [slug, hash] of currMap) {
    if (!prevMap.has(slug)) {
      added.push(slug);
    } else if (prevMap.get(slug) !== hash) {
      changed.push({
        slug,
        previous_hash: prevMap.get(slug)!,
        current_hash: hash,
      });
    }
  }
  const removed: string[] = [];
  for (const slug of prevMap.keys()) {
    if (!currMap.has(slug)) removed.push(slug);
  }

  return {
    added: added.sort(),
    removed: removed.sort(),
    changed: changed.sort((a, b) => a.slug.localeCompare(b.slug)),
  };
}
