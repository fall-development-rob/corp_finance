/**
 * Outlier detection — Phase 41 Wave 2.
 *
 * Four query functions over the reasoning bank to identify cluster patterns
 * that the skill-editor cookbook can use to generate improvement proposals.
 *
 * Cluster types:
 *   novel              — dispatch has no near-neighbour (cosine distance > 0.7)
 *   validation-failure — entries where metadata.validation_failed === true
 *   tool-thrashing     — entries where total tool_calls count > p95 of window
 *   delegation-mismatch— chief delegated; child returned validation_failed entry
 *
 * All cluster_ids are deterministic sha256 hashes of sorted audit_ids, so
 * re-runs against the same data produce the same cluster_ids.
 */
import { createHash, randomUUID } from "node:crypto";
import { readFile, writeFile } from "node:fs/promises";
import type { ReasoningBank, ReasoningEntry } from "./bank.js";

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

export type ClusterType =
  | "novel"
  | "validation-failure"
  | "tool-thrashing"
  | "delegation-mismatch";

export type RecommendedAction =
  | "add-skill-section"
  | "tighten-output-schema"
  | "adjust-tool-allowlist"
  | "no-action";

export interface OutlierCluster {
  /** Deterministic id derived from sorted member audit_ids. */
  cluster_id: string;
  cluster_type: ClusterType;
  /** Skill slug if identifiable from agent_id or delegation. */
  affected_skill?: string;
  /** At least 3 audit_ids that motivated this cluster. */
  motivating_audit_ids: string[];
  /** ≤ 300-char human-readable pattern description. */
  pattern_summary: string;
  recommended_action: RecommendedAction;
  /** 0..1 confidence in the cluster's actionability. */
  confidence_score: number;
}

export interface RunWindow {
  from: Date;
  to: Date;
}

export interface OutlierReport {
  report_id: string;
  run_window: RunWindow;
  clusters: OutlierCluster[];
}

export interface OutlierDetectorOptions {
  bank: ReasoningBank;
  /** Time window — default last 7 days. */
  window?: RunWindow;
  /** Cosine distance threshold for novel clusters (default 0.7). */
  novelThreshold?: number;
  /** Percentile cap for tool-thrashing (default 95). */
  toolThrashingPercentile?: number;
  /** Minimum cluster size — singletons don't justify a proposal. Default 3. */
  minClusterSize?: number;
  /**
   * Optional embedding fn for novel-cluster distance computation. If absent,
   * novel-cluster query is skipped and returns [].
   */
  embed?: (text: string) => Promise<number[]>;
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

const DEFAULT_NOVEL_THRESHOLD = 0.7;
const DEFAULT_TOOL_THRASHING_PERCENTILE = 95;
const DEFAULT_MIN_CLUSTER_SIZE = 3;
const GRAPH_FETCH_LIMIT = 500;

/** Build the default 7-day window ending now. */
function defaultWindow(): RunWindow {
  const to = new Date();
  const from = new Date(to.getTime() - 7 * 24 * 60 * 60 * 1000);
  return { from, to };
}

/**
 * Deterministic cluster_id from a sorted list of audit_ids.
 * Format: "cluster-" + first 8 hex chars of sha256(sortedIds.join(",")).
 */
export function computeClusterId(auditIds: string[]): string {
  const sorted = [...auditIds].sort((a, b) => a.localeCompare(b));
  const hash = createHash("sha256")
    .update(sorted.join(","), "utf-8")
    .digest("hex");
  return `cluster-${hash.slice(0, 8)}`;
}

/**
 * Cosine similarity between two vectors.
 * Both must be the same length. Returns a value in [-1, 1].
 */
function cosineSimilarity(a: number[], b: number[]): number {
  let dot = 0;
  let normA = 0;
  let normB = 0;
  const len = Math.min(a.length, b.length);
  for (let i = 0; i < len; i++) {
    const ai = a[i] ?? 0;
    const bi = b[i] ?? 0;
    dot += ai * bi;
    normA += ai * ai;
    normB += bi * bi;
  }
  const denom = Math.sqrt(normA) * Math.sqrt(normB);
  if (denom === 0) return 0;
  return dot / denom;
}

/**
 * Cosine distance = 1 - cosine similarity.
 * Range [0, 2]; for unit vectors range is [0, 1].
 */
function cosineDistance(a: number[], b: number[]): number {
  return 1 - cosineSimilarity(a, b);
}

/**
 * Compute the p-th percentile (0-100) of an array of numbers.
 * Uses nearest-rank method. Returns 0 for empty arrays.
 */
function percentile(values: number[], p: number): number {
  if (values.length === 0) return 0;
  const sorted = [...values].sort((a, b) => a - b);
  const rank = Math.ceil((p / 100) * sorted.length);
  return sorted[Math.min(rank, sorted.length) - 1] ?? 0;
}

/**
 * Extract an affected_skill slug from an agent_id.
 * Convention: agent_id may be "cfa-equity-analyst" or "equity-analyst".
 */
function skillFromAgentId(agentId: string): string {
  return agentId;
}

/**
 * Sum all tool invocation counts for an entry.
 */
function totalToolCalls(entry: ReasoningEntry): number {
  return entry.tool_calls.reduce((sum, tc) => sum + tc.count, 0);
}

/**
 * Group entries by a string key. Returns a Map<key, entries[]>.
 */
function groupBy<T>(items: T[], key: (item: T) => string): Map<string, T[]> {
  const map = new Map<string, T[]>();
  for (const item of items) {
    const k = key(item);
    const group = map.get(k) ?? [];
    group.push(item);
    map.set(k, group);
  }
  return map;
}

// ---------------------------------------------------------------------------
// Detector: validation failures
// ---------------------------------------------------------------------------

/**
 * Find clusters of dispatches where metadata.validation_failed === true.
 * Groups by agent_id; emits a cluster per group with ≥ minClusterSize.
 */
export async function detectValidationFailures(
  opts: OutlierDetectorOptions,
): Promise<OutlierCluster[]> {
  const win = opts.window ?? defaultWindow();
  const minSize = opts.minClusterSize ?? DEFAULT_MIN_CLUSTER_SIZE;

  const entries = await opts.bank.recallByGraph({
    metadata: { validation_failed: true },
    since: win.from,
    until: win.to,
    limit: GRAPH_FETCH_LIMIT,
  });

  const grouped = groupBy(entries, (e) => e.agent_id);
  const clusters: OutlierCluster[] = [];

  for (const [agentId, members] of grouped) {
    if (members.length < minSize) continue;
    const auditIds = members.map((m) => m.audit_id);
    const clusterId = computeClusterId(auditIds);
    const capped = auditIds.slice(0, 20);
    const score = Math.min(0.95, members.length / 10);
    clusters.push({
      cluster_id: clusterId,
      cluster_type: "validation-failure",
      affected_skill: skillFromAgentId(agentId),
      motivating_audit_ids: capped,
      pattern_summary: `${members.length} validation failures from agent ${agentId} in window.`,
      recommended_action: "tighten-output-schema",
      confidence_score: score,
    });
  }

  return clusters;
}

// ---------------------------------------------------------------------------
// Detector: tool-thrashing
// ---------------------------------------------------------------------------

/**
 * Find clusters of dispatches above the p95 tool-call threshold.
 * Groups by agent_id; emits a cluster per group with ≥ minClusterSize.
 */
export async function detectToolThrashing(
  opts: OutlierDetectorOptions,
): Promise<OutlierCluster[]> {
  const win = opts.window ?? defaultWindow();
  const minSize = opts.minClusterSize ?? DEFAULT_MIN_CLUSTER_SIZE;
  const pct = opts.toolThrashingPercentile ?? DEFAULT_TOOL_THRASHING_PERCENTILE;

  const entries = await opts.bank.recallByGraph({
    since: win.from,
    until: win.to,
    limit: GRAPH_FETCH_LIMIT,
  });

  if (entries.length === 0) return [];

  const toolCounts = entries.map(totalToolCalls);
  const threshold = percentile(toolCounts, pct);
  const thrashing = entries.filter((e) => totalToolCalls(e) > threshold);

  const grouped = groupBy(thrashing, (e) => e.agent_id);
  const clusters: OutlierCluster[] = [];

  for (const [agentId, members] of grouped) {
    if (members.length < minSize) continue;
    const auditIds = members.map((m) => m.audit_id);
    const clusterId = computeClusterId(auditIds);
    const capped = auditIds.slice(0, 20);
    clusters.push({
      cluster_id: clusterId,
      cluster_type: "tool-thrashing",
      affected_skill: skillFromAgentId(agentId),
      motivating_audit_ids: capped,
      pattern_summary:
        `${members.length} dispatches from ${agentId} exceeded p${pct} ` +
        `tool-call threshold (${threshold} calls).`,
      recommended_action: "adjust-tool-allowlist",
      confidence_score: 0.6,
    });
  }

  return clusters;
}

// ---------------------------------------------------------------------------
// Detector: delegation mismatch
// ---------------------------------------------------------------------------

/**
 * Find clusters where the chief delegated to a specialist that returned a
 * validation-failed output. Cross-references parent delegations against child
 * metadata.validation_failed.
 */
export async function detectDelegationMismatch(
  opts: OutlierDetectorOptions,
): Promise<OutlierCluster[]> {
  const win = opts.window ?? defaultWindow();
  const minSize = opts.minClusterSize ?? DEFAULT_MIN_CLUSTER_SIZE;

  // Fetch all entries with delegations in the window.
  const parentEntries = await opts.bank.recallByGraph({
    since: win.from,
    until: win.to,
    limit: GRAPH_FETCH_LIMIT,
  });

  const delegatingParents = parentEntries.filter(
    (e) => e.delegations.length > 0,
  );

  if (delegatingParents.length === 0) return [];

  // Fetch all validation-failed entries so we can cross-reference.
  const failedEntries = await opts.bank.recallByGraph({
    metadata: { validation_failed: true },
    since: win.from,
    until: win.to,
    limit: GRAPH_FETCH_LIMIT,
  });

  const failedAgentIds = new Set(failedEntries.map((e) => e.agent_id));

  // Group mismatches by chief→specialist pair.
  // A mismatch = parent has delegations overlapping failedAgentIds.
  type PairKey = string;
  const pairMap = new Map<PairKey, ReasoningEntry[]>();

  for (const parent of delegatingParents) {
    for (const specialist of parent.delegations) {
      if (!failedAgentIds.has(specialist)) continue;
      const key = `${parent.agent_id}→${specialist}`;
      const group = pairMap.get(key) ?? [];
      group.push(parent);
      pairMap.set(key, group);
    }
  }

  const clusters: OutlierCluster[] = [];

  for (const [pairKey, members] of pairMap) {
    if (members.length < minSize) continue;
    const auditIds = members.map((m) => m.audit_id);
    const clusterId = computeClusterId(auditIds);
    const capped = auditIds.slice(0, 20);
    clusters.push({
      cluster_id: clusterId,
      cluster_type: "delegation-mismatch",
      affected_skill: pairKey,
      motivating_audit_ids: capped,
      pattern_summary:
        `${members.length} delegations on pair ${pairKey} produced ` +
        `validation-failed specialist outputs.`,
      recommended_action: "add-skill-section",
      confidence_score: 0.8,
    });
  }

  return clusters;
}

// ---------------------------------------------------------------------------
// Detector: novel clusters
// ---------------------------------------------------------------------------

/**
 * Find clusters of dispatches whose nearest neighbour has cosine distance
 * above novelThreshold (i.e. they are unlike anything else in the bank).
 *
 * Requires opts.embed. If absent, returns [] immediately.
 */
export async function detectNovelClusters(
  opts: OutlierDetectorOptions,
): Promise<OutlierCluster[]> {
  if (!opts.embed) return [];

  const win = opts.window ?? defaultWindow();
  const minSize = opts.minClusterSize ?? DEFAULT_MIN_CLUSTER_SIZE;
  const threshold = opts.novelThreshold ?? DEFAULT_NOVEL_THRESHOLD;

  const entries = await opts.bank.recallByGraph({
    since: win.from,
    until: win.to,
    limit: GRAPH_FETCH_LIMIT,
  });

  if (entries.length < 2) return [];

  // Compute embeddings for any entry missing one (should be rare).
  const withEmbeddings = await Promise.all(
    entries.map(async (e) => {
      const embedding =
        e.embedding.length > 0
          ? e.embedding
          : await opts.embed!(e.prompt_summary);
      return { ...e, embedding };
    }),
  );

  // For each entry, find the minimum cosine distance to any other entry.
  const novelEntries: ReasoningEntry[] = [];
  for (let i = 0; i < withEmbeddings.length; i++) {
    const current = withEmbeddings[i]!;
    let minDist = Infinity;
    for (let j = 0; j < withEmbeddings.length; j++) {
      if (i === j) continue;
      const other = withEmbeddings[j]!;
      const dist = cosineDistance(current.embedding, other.embedding);
      if (dist < minDist) minDist = dist;
    }
    if (minDist > threshold) {
      novelEntries.push(current);
    }
  }

  if (novelEntries.length < minSize) return [];

  // Group novel entries by agent_id (simple grouping strategy).
  const grouped = groupBy(novelEntries, (e) => e.agent_id);
  const clusters: OutlierCluster[] = [];

  for (const [agentId, members] of grouped) {
    if (members.length < minSize) continue;
    const auditIds = members.map((m) => m.audit_id);
    const clusterId = computeClusterId(auditIds);
    const capped = auditIds.slice(0, 20);

    // Confidence: based on average pairwise cosine similarity within the cluster.
    const avgIntraSim = computeAvgIntraSimilarity(members);
    const confidence = Math.min(
      0.85,
      Math.max(0.4, 0.5 + (avgIntraSim - 0.3) / 2),
    );

    clusters.push({
      cluster_id: clusterId,
      cluster_type: "novel",
      affected_skill: skillFromAgentId(agentId),
      motivating_audit_ids: capped,
      pattern_summary:
        `${members.length} dispatches from ${agentId} are unlike ` +
        `anything else in the reasoning bank (cosine distance > ${threshold}).`,
      recommended_action: "add-skill-section",
      confidence_score: confidence,
    });
  }

  return clusters;
}

/** Average pairwise cosine similarity within a set of entries. */
function computeAvgIntraSimilarity(entries: ReasoningEntry[]): number {
  if (entries.length < 2) return 0;
  let total = 0;
  let count = 0;
  for (let i = 0; i < entries.length; i++) {
    for (let j = i + 1; j < entries.length; j++) {
      total += cosineSimilarity(entries[i]!.embedding, entries[j]!.embedding);
      count++;
    }
  }
  return count === 0 ? 0 : total / count;
}

// ---------------------------------------------------------------------------
// Top-level orchestrator
// ---------------------------------------------------------------------------

/**
 * Run all 4 detectors in parallel, concat results, dedupe by cluster_id.
 *
 * Priority order for deduplication:
 *   validation-failure > delegation-mismatch > tool-thrashing > novel
 */
export async function detectAllOutliers(
  opts: OutlierDetectorOptions,
): Promise<OutlierReport> {
  const win = opts.window ?? defaultWindow();

  const [validationFailures, delegationMismatches, toolThrashing, novel] =
    await Promise.all([
      detectValidationFailures(opts),
      detectDelegationMismatch(opts),
      detectToolThrashing(opts),
      detectNovelClusters(opts),
    ]);

  // Priority order: validation-failure, delegation-mismatch, tool-thrashing, novel.
  const ordered = [
    ...validationFailures,
    ...delegationMismatches,
    ...toolThrashing,
    ...novel,
  ];

  // Dedupe by cluster_id — first occurrence wins (priority ordering above).
  const seen = new Set<string>();
  const clusters: OutlierCluster[] = [];
  for (const c of ordered) {
    if (!seen.has(c.cluster_id)) {
      seen.add(c.cluster_id);
      clusters.push(c);
    }
  }

  return {
    report_id: randomUUID(),
    run_window: win,
    clusters,
  };
}

// ---------------------------------------------------------------------------
// Watermark store
// ---------------------------------------------------------------------------

export interface WatermarkStore {
  read(): Promise<Date | undefined>;
  write(t: Date): Promise<void>;
}

interface WatermarkData {
  timestamp: string;
}

/**
 * File-backed watermark store. Reads and writes a single JSON file containing
 * the last-run timestamp. If the file does not exist on read(), returns undefined.
 */
export function createFileWatermarkStore(path: string): WatermarkStore {
  return {
    async read(): Promise<Date | undefined> {
      try {
        const raw = await readFile(path, "utf-8");
        const data = JSON.parse(raw) as WatermarkData;
        if (typeof data.timestamp !== "string") return undefined;
        const d = new Date(data.timestamp);
        if (isNaN(d.getTime())) return undefined;
        return d;
      } catch {
        return undefined;
      }
    },

    async write(t: Date): Promise<void> {
      const data: WatermarkData = { timestamp: t.toISOString() };
      await writeFile(path, JSON.stringify(data, null, 2), "utf-8");
    },
  };
}
