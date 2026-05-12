/**
 * Cookbook replay contracts — Phase 25 Tier A3.
 *
 * For each cookbook, records the deterministic projection produced by
 * `createCookbookLoader().load(slug)`: parent and subagent IDs, models,
 * sorted tool sets, block_tools, system-prompt fingerprints, and
 * output-schema fingerprints. Two runs against the same disk state
 * produce byte-identical output.
 *
 * Complements Tier A2 cookbook audit:
 *   - audit  → catches BYTE changes in cookbook content (file diff).
 *   - replay → catches PROJECTION changes (what the loader produces).
 *
 * The replay catches three bug classes the audit cannot:
 *   1. Loader regressions (projectTools / system-prompt assembly bugs)
 *      that change loader output without any file diff.
 *   2. Structured drift: PR review sees "analyst subagent lost tool X"
 *      instead of "yaml file changed".
 *   3. Subagent surface drift: tool count + model + block_tools diff in
 *      a single reviewable JSON block per cookbook.
 *
 * Pure library — depends only on the cookbook-loader. CLI runner lives
 * in `scripts/generate-cookbook-replays.ts`.
 */

import { createHash } from "node:crypto";

import type { CookbookLoader, LoadedCookbook } from "./cookbook-loader.js";
import type { AgentDef } from "../types.js";

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

export interface AgentFingerprint {
  id: string;
  model?: string;
  /** Number of tools in the allowlist; "*" → -1 sentinel. */
  tool_count: number;
  /** Sorted bare tool names; ["*"] when allowlist is "*". */
  tools: string[];
  /** Sorted block_tools list (empty array if absent). */
  block_tools: string[];
  /** sha256 of the assembled system prompt (post-skill, post-system.file, post-append). */
  system_prompt_sha256: string;
  /** Byte length of the assembled system prompt. */
  system_prompt_bytes: number;
  /** sha256 of canonical JSON of output_schema, or "" if absent. */
  output_schema_sha256: string;
  /** sha256 of canonical JSON of input_schema, or "" if absent. */
  input_schema_sha256: string;
}

export interface CookbookReplay {
  slug: string;
  parent: AgentFingerprint & {
    /** IDs of callable subagents in load order. */
    subagent_ids: string[];
  };
  subagents: AgentFingerprint[];
}

export interface CookbookReplayCatalog {
  /** Format version; bump when the fingerprint shape changes. */
  version: string;
  /** Replays per cookbook, sorted alphabetically by slug. */
  cookbooks: CookbookReplay[];
}

// ---------------------------------------------------------------------------
// Fingerprint helpers
// ---------------------------------------------------------------------------

function sha256Hex(s: string): string {
  return createHash("sha256").update(s, "utf8").digest("hex");
}

/**
 * Canonical JSON for a JSON Schema-style object: sorted keys, 2-space
 * indent, no trailing whitespace. Used to fingerprint output/input schemas
 * so two equivalent schemas hash identically regardless of key order in
 * the source YAML.
 */
function canonicalJsonSorted(value: unknown): string {
  return JSON.stringify(value, sortedReplacer(value), 2);
}

function sortedReplacer(_root: unknown): (key: string, value: unknown) => unknown {
  return (_key, value) => {
    if (value && typeof value === "object" && !Array.isArray(value)) {
      const obj = value as Record<string, unknown>;
      const sorted: Record<string, unknown> = {};
      for (const k of Object.keys(obj).sort()) sorted[k] = obj[k];
      return sorted;
    }
    return value;
  };
}

function fingerprintAgent(def: AgentDef): AgentFingerprint {
  const tools: string[] =
    def.tools === "*"
      ? ["*"]
      : [...def.tools].sort((a, b) => a.localeCompare(b));
  const tool_count = def.tools === "*" ? -1 : def.tools.length;

  const block_tools = def.blockTools
    ? [...def.blockTools].sort((a, b) => a.localeCompare(b))
    : [];

  const systemPrompt = def.systemPrompt ?? "";

  const outputSchemaHash = def.outputSchema
    ? sha256Hex(canonicalJsonSorted(def.outputSchema))
    : "";
  const inputSchemaHash = def.inputSchema
    ? sha256Hex(canonicalJsonSorted(def.inputSchema))
    : "";

  return {
    id: def.id,
    ...(def.model !== undefined ? { model: def.model } : {}),
    tool_count,
    tools,
    block_tools,
    system_prompt_sha256: sha256Hex(systemPrompt),
    system_prompt_bytes: Buffer.byteLength(systemPrompt, "utf8"),
    output_schema_sha256: outputSchemaHash,
    input_schema_sha256: inputSchemaHash,
  };
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/**
 * Project a loaded cookbook into a deterministic fingerprint. The result
 * is byte-stable: two runs against the same `LoadedCookbook` produce the
 * same JSON.
 */
export function replayCookbook(loaded: LoadedCookbook): CookbookReplay {
  const parentFp = fingerprintAgent(loaded.parent);
  return {
    slug: loaded.slug,
    parent: {
      ...parentFp,
      subagent_ids: loaded.subagents.map((s) => s.id),
    },
    subagents: loaded.subagents.map(fingerprintAgent),
  };
}

export interface ReplayAllInput {
  loader: CookbookLoader;
}

/**
 * Replay every cookbook discoverable via `loader.loadAll()`. Results are
 * sorted alphabetically by slug.
 */
export async function replayAllCookbooks(
  input: ReplayAllInput,
): Promise<CookbookReplayCatalog> {
  const loadedAll = await input.loader.loadAll();
  const replays = loadedAll
    .map(replayCookbook)
    .sort((a, b) => a.slug.localeCompare(b.slug));
  return { version: "1", cookbooks: replays };
}

/**
 * Stable JSON serialisation: 2-space indent, alphabetical key ordering
 * within each agent fingerprint, alphabetical cookbook ordering by slug.
 * Trailing newline. Two runs produce byte-identical strings.
 */
export function serialiseReplayCatalog(
  catalog: CookbookReplayCatalog,
): string {
  const ordered: CookbookReplayCatalog = {
    version: catalog.version,
    cookbooks: [...catalog.cookbooks].sort((a, b) =>
      a.slug.localeCompare(b.slug),
    ),
  };
  return JSON.stringify(ordered, sortedReplacer(ordered), 2) + "\n";
}

export function parseReplayCatalog(json: string): CookbookReplayCatalog {
  const parsed = JSON.parse(json) as unknown;
  if (
    typeof parsed !== "object" ||
    parsed === null ||
    !("cookbooks" in parsed) ||
    !Array.isArray((parsed as { cookbooks: unknown }).cookbooks)
  ) {
    throw new Error("replay catalog JSON missing 'cookbooks' array");
  }
  return parsed as CookbookReplayCatalog;
}

// ---------------------------------------------------------------------------
// Diffing — for PR review summaries
// ---------------------------------------------------------------------------

export interface ReplayDiffAgent {
  field: keyof AgentFingerprint | "subagent_ids";
  previous: unknown;
  current: unknown;
}

export interface ReplayDiffCookbook {
  slug: string;
  parent_changes: ReplayDiffAgent[];
  subagent_changes: Array<{ id: string; changes: ReplayDiffAgent[] }>;
}

export interface ReplayDiff {
  added: string[];
  removed: string[];
  changed: ReplayDiffCookbook[];
}

function diffAgentFingerprint(
  a: AgentFingerprint,
  b: AgentFingerprint,
): ReplayDiffAgent[] {
  const changes: ReplayDiffAgent[] = [];
  const keys: Array<keyof AgentFingerprint> = [
    "id",
    "model",
    "tool_count",
    "tools",
    "block_tools",
    "system_prompt_sha256",
    "system_prompt_bytes",
    "output_schema_sha256",
    "input_schema_sha256",
  ];
  for (const k of keys) {
    const av = a[k];
    const bv = b[k];
    const eq =
      Array.isArray(av) && Array.isArray(bv)
        ? av.length === bv.length && av.every((x, i) => x === bv[i])
        : av === bv;
    if (!eq) changes.push({ field: k, previous: av, current: bv });
  }
  return changes;
}

/**
 * Compare two replay catalogs and report cookbook-level + agent-level
 * differences. Useful for PR review summaries.
 */
export function diffReplayCatalogs(
  previous: CookbookReplayCatalog,
  current: CookbookReplayCatalog,
): ReplayDiff {
  const prevMap = new Map(previous.cookbooks.map((c) => [c.slug, c]));
  const currMap = new Map(current.cookbooks.map((c) => [c.slug, c]));

  const added: string[] = [];
  const removed: string[] = [];
  const changed: ReplayDiffCookbook[] = [];

  for (const [slug, currCb] of currMap) {
    const prevCb = prevMap.get(slug);
    if (!prevCb) {
      added.push(slug);
      continue;
    }
    // Parent diff (compare without the subagent_ids extension)
    const { subagent_ids: prevIds, ...prevFp } = prevCb.parent;
    const { subagent_ids: currIds, ...currFp } = currCb.parent;
    const parentChanges = diffAgentFingerprint(prevFp, currFp);
    if (
      prevIds.length !== currIds.length ||
      prevIds.some((id, i) => id !== currIds[i])
    ) {
      parentChanges.push({
        field: "subagent_ids",
        previous: prevIds,
        current: currIds,
      });
    }

    // Subagent diff (matched by id)
    const prevSubs = new Map(prevCb.subagents.map((s) => [s.id, s]));
    const currSubs = new Map(currCb.subagents.map((s) => [s.id, s]));
    const subagentChanges: ReplayDiffCookbook["subagent_changes"] = [];
    for (const [id, currSub] of currSubs) {
      const prevSub = prevSubs.get(id);
      if (!prevSub) {
        subagentChanges.push({
          id,
          changes: [{ field: "id", previous: undefined, current: id }],
        });
        continue;
      }
      const diffs = diffAgentFingerprint(prevSub, currSub);
      if (diffs.length > 0) subagentChanges.push({ id, changes: diffs });
    }
    for (const id of prevSubs.keys()) {
      if (!currSubs.has(id)) {
        subagentChanges.push({
          id,
          changes: [{ field: "id", previous: id, current: undefined }],
        });
      }
    }

    if (parentChanges.length > 0 || subagentChanges.length > 0) {
      changed.push({
        slug,
        parent_changes: parentChanges,
        subagent_changes: subagentChanges,
      });
    }
  }

  for (const slug of prevMap.keys()) {
    if (!currMap.has(slug)) removed.push(slug);
  }

  return {
    added: added.sort(),
    removed: removed.sort(),
    changed: changed.sort((a, b) => a.slug.localeCompare(b.slug)),
  };
}
