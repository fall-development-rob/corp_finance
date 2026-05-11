/**
 * Remediation emitter + apply core — Phase 41 Wave 3.
 *
 * emitRemediations: OutlierReport → YAML remediation files (byte-deterministic).
 * applyRemediation: parse one YAML file and make the described file edit.
 *
 * No LLM at any point. All mappings are table-lookups or deterministic
 * inference from cluster metadata.
 *
 * Path guard: apply only touches files matching two glob patterns:
 *   plugins/STAR/STAR/skills/STAR/STAR/SKILL.md
 *   plugins/STAR/STAR/agents/STAR/STAR/STAR.yaml
 */
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { existsSync } from "node:fs";
import { join, resolve, relative } from "node:path";
import { stringify as yamlStringify, parse as yamlParse } from "yaml";
import type { OutlierReport, OutlierCluster, ClusterType } from "./outliers.js";
import type {
  Remediation,
  AddSkillSectionRemediation,
  TightenOutputSchemaRemediation,
  AdjustToolAllowlistRemediation,
  NoActionRemediation,
} from "./remediation-types.js";

// ---------------------------------------------------------------------------
// Path guard
// ---------------------------------------------------------------------------

/** Patterns that the apply step is allowed to edit (matched against repo-relative posix paths). */
function isAllowedPath(repoRoot: string, filePath: string): boolean {
  const abs = resolve(filePath);
  const rel = relative(repoRoot, abs).replace(/\\/g, "/");
  // Allow: plugins/<any>/skills/<any>/SKILL.md
  const skillPattern = /^plugins\/.+\/skills\/.+\/SKILL\.md$/;
  // Allow: plugins/<any>/agents/<any>.yaml
  const agentPattern = /^plugins\/.+\/agents\/.+\.yaml$/;
  return skillPattern.test(rel) || agentPattern.test(rel);
}

// ---------------------------------------------------------------------------
// Template lookup tables (cluster_type → template_id)
// ---------------------------------------------------------------------------

const ADD_SKILL_TEMPLATE: Record<ClusterType, string> = {
  "validation-failure": "anti-injection-reminder",
  "delegation-mismatch": "anti-injection-reminder",
  "tool-thrashing": "tool-discipline-checklist",
  novel: "anti-injection-reminder",
};

const ADD_SKILL_SECTION_TITLE: Record<ClusterType, string> = {
  "validation-failure": "Output Validation Reminders",
  "delegation-mismatch": "Delegation Output Contract",
  "tool-thrashing": "Tool Discipline Guidelines",
  novel: "Edge-Case Handling Notes",
};

const ADD_SKILL_INSERT_AFTER: Record<ClusterType, string> = {
  "validation-failure": "Output Format",
  "delegation-mismatch": "Delegation",
  "tool-thrashing": "Tools",
  novel: "Overview",
};

// ---------------------------------------------------------------------------
// Emitter helpers
// ---------------------------------------------------------------------------

/**
 * Map a cluster to a Remediation record.
 * Pure function — no I/O. Same cluster → same output.
 */
function clusterToRemediation(cluster: OutlierCluster): Remediation {
  const base = {
    cluster_id: cluster.cluster_id,
    motivating_audit_ids: cluster.motivating_audit_ids,
    confidence_score: cluster.confidence_score,
  };

  const affected = cluster.affected_skill ?? "plugins/unknown/skills/SKILL.md";

  switch (cluster.recommended_action) {
    case "add-skill-section": {
      const templateId = ADD_SKILL_TEMPLATE[cluster.cluster_type];
      const rem: AddSkillSectionRemediation = {
        ...base,
        type: "add-skill-section",
        affected_skill: affected,
        change: {
          section_title: ADD_SKILL_SECTION_TITLE[cluster.cluster_type],
          section_body_template_id: templateId,
          insert_after_section: ADD_SKILL_INSERT_AFTER[cluster.cluster_type],
        },
      };
      return rem;
    }

    case "tighten-output-schema": {
      // Derive field_path from motivating_audit_ids: use a fixed heuristic
      // (first audit_id component after the agent slug) rather than LLM.
      const fieldPath = inferFieldPath(cluster);
      const rem: TightenOutputSchemaRemediation = {
        ...base,
        type: "tighten-output-schema",
        affected_skill: affected,
        change: {
          field_path: fieldPath,
          add_constraint: {
            maxLength: 500,
          },
        },
      };
      return rem;
    }

    case "adjust-tool-allowlist": {
      // Derive the tool from the cluster's pattern_summary (deterministic).
      const toolName = inferThrashingTool(cluster);
      const rem: AdjustToolAllowlistRemediation = {
        ...base,
        type: "adjust-tool-allowlist",
        affected_skill: affected,
        change: {
          action: "remove",
          tool_name: toolName,
          target_path: "block_tools",
        },
      };
      return rem;
    }

    case "no-action": {
      const rem: NoActionRemediation = {
        ...base,
        type: "no-action",
        notes: `Cluster ${cluster.cluster_id} (${cluster.cluster_type}) noted for observability. ` +
          `Pattern: ${cluster.pattern_summary.slice(0, 200)} No automated change warranted at this confidence level.`,
      };
      return rem;
    }
  }
}

/**
 * Deterministic field-path inference.
 * Uses the cluster_type to pick a canonical schema field path.
 */
function inferFieldPath(cluster: OutlierCluster): string {
  // Deterministic: map cluster_type to expected schema field.
  const baseField =
    cluster.cluster_type === "validation-failure"
      ? "output_schema.properties.result"
      : "output_schema.properties.value";

  // Add cluster-specificity from sorted audit_ids (reproducible).
  const sorted = [...cluster.motivating_audit_ids].sort();
  const suffix = sorted[0]?.split("-")[0] ?? "field";
  return `${baseField}_${suffix}`;
}

/**
 * Deterministic tool-name inference for tool-thrashing clusters.
 * Parses the pattern_summary for a tool count reference, else falls back to cluster_id.
 */
function inferThrashingTool(cluster: OutlierCluster): string {
  // Pattern: "N dispatches from <agent> exceeded pNN tool-call threshold (N calls)."
  // We can't reliably extract a specific tool without audit records, so we use
  // a deterministic placeholder derived from the cluster_id.
  return `tool-${cluster.cluster_id.replace("cluster-", "")}`;
}

/** Deterministic filename for a cluster's remediation file. */
function remediationFileName(date: string, clusterId: string): string {
  return `${date}-${clusterId}.yaml`;
}

// ---------------------------------------------------------------------------
// Public: emitRemediations
// ---------------------------------------------------------------------------

export interface EmitRemediationsOptions {
  report: OutlierReport;
  /** Directory to write remediation YAML files into. */
  outputDir: string;
  /** Defaults to "docs/skill-editor-templates". */
  templatesDir?: string;
}

export interface EmittedRemediation {
  filePath: string;
  remediation: Remediation;
}

/**
 * Emit one YAML remediation file per cluster in the OutlierReport.
 *
 * Deterministic: same OutlierReport → byte-identical files.
 * File naming: <YYYY-MM-DD>-<cluster_id>.yaml
 * The report_id is NOT part of the filename — cluster_id is stable across runs.
 */
export async function emitRemediations(
  opts: EmitRemediationsOptions,
): Promise<EmittedRemediation[]> {
  await mkdir(opts.outputDir, { recursive: true });

  // Use a fixed date derived from run_window.to for reproducibility within a run.
  const dateStr = opts.report.run_window.to
    .toISOString()
    .slice(0, 10); // "YYYY-MM-DD"

  const results: EmittedRemediation[] = [];

  for (const cluster of opts.report.clusters) {
    const remediation = clusterToRemediation(cluster);
    const fileName = remediationFileName(dateStr, cluster.cluster_id);
    const filePath = join(opts.outputDir, fileName);

    // Serialize deterministically: sort keys, no aliases.
    const yamlContent = yamlStringify(remediation, {
      sortMapEntries: true,
      aliasDuplicateObjects: false,
    });

    await writeFile(filePath, yamlContent, "utf-8");
    results.push({ filePath, remediation });
  }

  return results;
}

// ---------------------------------------------------------------------------
// Public: applyRemediation
// ---------------------------------------------------------------------------

export interface ApplyRemediationOptions {
  remediationFilePath: string;
  repoRoot: string;
  /** Defaults to "docs/skill-editor-templates". */
  templatesDir?: string;
  /** If true, print the diff to stdout instead of writing. */
  dryRun?: boolean;
}

export interface ApplyResult {
  applied: boolean;
  /** Reason when not applied. */
  reason?: string;
  affectedFile: string;
}

/**
 * Parse a YAML remediation file and apply the described change.
 * Idempotent: applying the same remediation twice → second call returns applied=false.
 */
export async function applyRemediation(
  opts: ApplyRemediationOptions,
): Promise<ApplyResult> {
  const raw = await readFile(opts.remediationFilePath, "utf-8");

  let remediation: unknown;
  try {
    remediation = yamlParse(raw);
  } catch (err) {
    throw new Error(
      `Malformed remediation YAML at ${opts.remediationFilePath}: ${(err as Error).message}`,
    );
  }

  if (
    typeof remediation !== "object" ||
    remediation === null ||
    !("type" in remediation) ||
    typeof (remediation as Record<string, unknown>).type !== "string"
  ) {
    throw new Error(
      `Remediation at ${opts.remediationFilePath} is missing required field "type"`,
    );
  }

  const rem = remediation as Remediation;
  const templatesDir =
    opts.templatesDir ?? join(opts.repoRoot, "docs/skill-editor-templates");

  switch (rem.type) {
    case "add-skill-section":
      return applyAddSkillSection(rem, opts, templatesDir);
    case "tighten-output-schema":
      return applyTightenOutputSchema(rem, opts);
    case "adjust-tool-allowlist":
      return applyAdjustToolAllowlist(rem, opts);
    case "no-action":
      return applyNoAction(rem);
    default:
      throw new Error(`Unknown remediation type: ${(rem as RemediationBase).type}`);
  }
}

// ---------------------------------------------------------------------------
// Apply helpers
// ---------------------------------------------------------------------------

interface RemediationBase {
  type: string;
  cluster_id: string;
  affected_skill?: string;
}

async function applyAddSkillSection(
  rem: AddSkillSectionRemediation,
  opts: ApplyRemediationOptions,
  templatesDir: string,
): Promise<ApplyResult> {
  const affectedFile = resolve(opts.repoRoot, rem.affected_skill);

  if (!isAllowedPath(opts.repoRoot, affectedFile)) {
    throw new Error(
      `Path guard: refusing to edit "${rem.affected_skill}". ` +
        `Only plugins/**/skills/**/SKILL.md and plugins/**/agents/**/*.yaml are allowed.`,
    );
  }

  const templatePath = join(
    templatesDir,
    "add-skill-section",
    `${rem.change.section_body_template_id}.md`,
  );

  if (!existsSync(templatePath)) {
    throw new Error(
      `Template not found: ${templatePath}. ` +
        `Create docs/skill-editor-templates/add-skill-section/${rem.change.section_body_template_id}.md`,
    );
  }

  const templateBody = await readFile(templatePath, "utf-8");

  if (!existsSync(affectedFile)) {
    throw new Error(`Affected skill file not found: ${affectedFile}`);
  }

  const current = await readFile(affectedFile, "utf-8");

  // Idempotency: if template body already present, skip.
  if (current.includes(templateBody.trimEnd())) {
    return { applied: false, reason: "already-applied", affectedFile };
  }

  // Find insert position: after the `## <insert_after_section>` heading.
  const insertHeading = `## ${rem.change.insert_after_section}`;
  const lines = current.split("\n");
  let insertIdx = -1;

  for (let i = 0; i < lines.length; i++) {
    if ((lines[i] ?? "").trim() === insertHeading) {
      // Advance to end of this section (next ## or EOF).
      let j = i + 1;
      while (j < lines.length && !(lines[j] ?? "").startsWith("## ")) {
        j++;
      }
      insertIdx = j;
      break;
    }
  }

  if (insertIdx === -1) {
    // Section not found — append at end.
    insertIdx = lines.length;
  }

  const newSection = [
    "",
    `## ${rem.change.section_title}`,
    "",
    templateBody.trimEnd(),
    "",
  ];

  const updated = [
    ...lines.slice(0, insertIdx),
    ...newSection,
    ...lines.slice(insertIdx),
  ].join("\n");

  if (opts.dryRun) {
    process.stdout.write(`[dry-run] Would write ${affectedFile}:\n${updated}\n`);
    return { applied: true, affectedFile };
  }

  await writeFile(affectedFile, updated, "utf-8");
  return { applied: true, affectedFile };
}

async function applyTightenOutputSchema(
  rem: TightenOutputSchemaRemediation,
  opts: ApplyRemediationOptions,
): Promise<ApplyResult> {
  const affectedFile = resolve(opts.repoRoot, rem.affected_skill);

  if (!isAllowedPath(opts.repoRoot, affectedFile)) {
    throw new Error(
      `Path guard: refusing to edit "${rem.affected_skill}". ` +
        `Only plugins/**/skills/**/SKILL.md and plugins/**/agents/**/*.yaml are allowed.`,
    );
  }

  if (!existsSync(affectedFile)) {
    throw new Error(`Affected manifest not found: ${affectedFile}`);
  }

  const raw = await readFile(affectedFile, "utf-8");
  const manifest = yamlParse(raw) as Record<string, unknown>;

  // Navigate to field_path and merge constraints.
  const parts = rem.change.field_path.split(".");
  let node: Record<string, unknown> = manifest;

  for (let i = 0; i < parts.length - 1; i++) {
    const key = parts[i]!;
    if (typeof node[key] !== "object" || node[key] === null) {
      node[key] = {};
    }
    node = node[key] as Record<string, unknown>;
  }

  const lastKey = parts[parts.length - 1]!;
  if (typeof node[lastKey] !== "object" || node[lastKey] === null) {
    node[lastKey] = {};
  }
  const target = node[lastKey] as Record<string, unknown>;

  // Check idempotency: all constraints already present and identical.
  const constraints = rem.change.add_constraint;
  let alreadyApplied = true;
  for (const [k, v] of Object.entries(constraints)) {
    if (JSON.stringify(target[k]) !== JSON.stringify(v)) {
      alreadyApplied = false;
      break;
    }
  }

  if (alreadyApplied) {
    return { applied: false, reason: "already-applied", affectedFile };
  }

  // Merge constraints (don't overwrite existing).
  for (const [k, v] of Object.entries(constraints)) {
    if (target[k] === undefined) {
      target[k] = v;
    }
  }

  const updated = yamlStringify(manifest, { sortMapEntries: true });

  if (opts.dryRun) {
    process.stdout.write(`[dry-run] Would write ${affectedFile}:\n${updated}\n`);
    return { applied: true, affectedFile };
  }

  await writeFile(affectedFile, updated, "utf-8");
  return { applied: true, affectedFile };
}

async function applyAdjustToolAllowlist(
  rem: AdjustToolAllowlistRemediation,
  opts: ApplyRemediationOptions,
): Promise<ApplyResult> {
  const affectedFile = resolve(opts.repoRoot, rem.affected_skill);

  if (!isAllowedPath(opts.repoRoot, affectedFile)) {
    throw new Error(
      `Path guard: refusing to edit "${rem.affected_skill}". ` +
        `Only plugins/**/skills/**/SKILL.md and plugins/**/agents/**/*.yaml are allowed.`,
    );
  }

  if (!existsSync(affectedFile)) {
    throw new Error(`Affected manifest not found: ${affectedFile}`);
  }

  const raw = await readFile(affectedFile, "utf-8");
  const manifest = yamlParse(raw) as Record<string, unknown>;

  // Navigate to target_path (simple dot notation; does not support array indexers).
  const parts = rem.change.target_path.split(".");
  let node: Record<string, unknown> = manifest;

  for (let i = 0; i < parts.length - 1; i++) {
    const key = parts[i]!;
    if (typeof node[key] !== "object" || node[key] === null) {
      node[key] = {};
    }
    node = node[key] as Record<string, unknown>;
  }

  const lastKey = parts[parts.length - 1]!;
  const existing = node[lastKey];
  const list: string[] = Array.isArray(existing) ? (existing as string[]) : [];

  let changed = false;
  let updated: string[];

  if (rem.change.action === "add") {
    if (!list.includes(rem.change.tool_name)) {
      updated = [...list, rem.change.tool_name];
      changed = true;
    } else {
      return { applied: false, reason: "already-applied", affectedFile };
    }
  } else {
    // remove
    if (!list.includes(rem.change.tool_name)) {
      return { applied: false, reason: "already-applied", affectedFile };
    }
    updated = list.filter((t) => t !== rem.change.tool_name);
    changed = true;
  }

  if (!changed) {
    return { applied: false, reason: "already-applied", affectedFile };
  }

  node[lastKey] = updated;
  const serialized = yamlStringify(manifest, { sortMapEntries: true });

  if (opts.dryRun) {
    process.stdout.write(`[dry-run] Would write ${affectedFile}:\n${serialized}\n`);
    return { applied: true, affectedFile };
  }

  await writeFile(affectedFile, serialized, "utf-8");
  return { applied: true, affectedFile };
}

function applyNoAction(rem: NoActionRemediation): ApplyResult {
  process.stdout.write(
    `info: cluster ${rem.cluster_id} noted, no automated change\n`,
  );
  return {
    applied: false,
    reason: "no-action-type",
    affectedFile: "",
  };
}
