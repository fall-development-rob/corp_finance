/**
 * Phase 41 — Skill-editor end-to-end smoke test.
 *
 * Exercises the full closed loop:
 *   seed bank → detect outliers → emit remediations → apply → archive (idempotency)
 *
 * Six test groups:
 *  1. Happy path: validation-failure cluster → proposal → apply → archive
 *  2. Byte-determinism: two independent banks, same seeds → identical YAML files
 *  3. Path guard: forbidden affected_skill → throws/returns false
 *  4. Apply idempotency: all 4 RemediationTypes, second apply is no-op
 *  5. No proposals when bank is empty
 *  6. No proposals when cluster size < minClusterSize
 *
 * All file operations use mkdtempSync() and are cleaned up in afterEach.
 * No network, no API keys, no source-code modifications.
 */

import { mkdtempSync, rmSync, readdirSync, readFileSync, mkdirSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { stringify as yamlStringify } from "yaml";

// ---------------------------------------------------------------------------
// Sources under test
// ---------------------------------------------------------------------------

import { createRuVectorBank } from "../src/reasoning/bank.js";
import { indexAuditRecord } from "../src/reasoning/indexer.js";
import { detectAllOutliers } from "../src/reasoning/outliers.js";
import { emitRemediations, applyRemediation } from "../src/reasoning/remediation.js";
import type { EmitRemediationsOptions } from "../src/reasoning/remediation.js";
import type {
  AddSkillSectionRemediation,
  TightenOutputSchemaRemediation,
  AdjustToolAllowlistRemediation,
  NoActionRemediation,
} from "../src/reasoning/remediation-types.js";
import type { OutlierReport } from "../src/reasoning/outliers.js";

// ---------------------------------------------------------------------------
// Test fixtures (from __helpers__)
// ---------------------------------------------------------------------------

import {
  createDeterministicEmbedder,
  makeSyntheticAuditRecord,
  makeValidationResult,
  makeSyntheticSkillFile,
  makeSyntheticManifest,
  writeStandardTemplates,
} from "./__helpers__/skill-editor-fixtures.js";

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const AGENT_ID = "equity-analyst-cookbook-analyst";
const REAL_TEMPLATES_DIR = join(
  import.meta.dirname,
  "../../../docs/skill-editor-templates",
);

// ---------------------------------------------------------------------------
// Per-test temp directory lifecycle
// ---------------------------------------------------------------------------

let tmpRoot: string;

beforeEach(() => {
  tmpRoot = mkdtempSync(join(tmpdir(), "skill-editor-e2e-"));
});

afterEach(() => {
  rmSync(tmpRoot, { recursive: true, force: true });
});

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

/** Build and seed a reasoning bank with N validation-failure records. */
async function seedBank(
  dir: string,
  count: number,
  agentId: string = AGENT_ID,
): Promise<void> {
  const embed = createDeterministicEmbedder();
  const bank = await createRuVectorBank({ dir, embed });
  const now = new Date();

  for (let i = 0; i < count; i++) {
    const record = makeSyntheticAuditRecord(
      { agent_id: agentId, validation_failed: true, tool_uses: 3 },
      i,
    );
    // Use a timestamp within the last 7 days so the default window includes it.
    const ts = new Date(now.getTime() - i * 60_000);
    record.timestamp = ts.toISOString();

    const validationResult = makeValidationResult(true);
    await indexAuditRecord({
      bank,
      record,
      prompt: `Analyse equity position for agent ${agentId}, run ${i}.`,
      finalText: `Result from agent ${agentId}, run ${i}: partial output.`,
      validationResult,
    });
  }

  await bank.close();
}

/** Run analyse → emit against a seeded bank, return the output dir path. */
async function analyseAndEmit(
  bankDir: string,
  outputDir: string,
  minClusterSize = 3,
): Promise<OutlierReport> {
  const embed = createDeterministicEmbedder();
  const bank = await createRuVectorBank({ dir: bankDir, embed });

  const report = await detectAllOutliers({
    bank,
    minClusterSize,
    embed,
  });

  await emitRemediations({ report, outputDir });
  await bank.close();
  return report;
}

// ===========================================================================
// Test 1: Happy path — validation-failure cluster → proposal → apply → archive
// ===========================================================================

describe("skill-editor e2e — happy path (validation-failure cluster)", () => {
  it("seeds 5 records, detects cluster, emits YAML, applies to SKILL.md, idempotency check", async () => {
    const bankDir = join(tmpRoot, "bank");
    const outputDir = join(tmpRoot, "remediations");

    // 1a. Seed the bank with 5 validation-failure records for the same agent.
    await seedBank(bankDir, 5);

    // 1b. Detect outliers — expect a validation-failure cluster of size 5.
    const embed = createDeterministicEmbedder();
    const bank = await createRuVectorBank({ dir: bankDir, embed });

    const report = await detectAllOutliers({
      bank,
      minClusterSize: 3,
      embed,
    });

    await bank.close();

    const vfClusters = report.clusters.filter(
      (c) => c.cluster_type === "validation-failure",
    );
    expect(vfClusters.length).toBeGreaterThanOrEqual(1);
    const cluster = vfClusters[0]!;
    expect(cluster.motivating_audit_ids.length).toBeGreaterThanOrEqual(3);
    expect(cluster.recommended_action).toBe("tighten-output-schema");
    expect(cluster.affected_skill).toBe(AGENT_ID);

    // 1c. Emit remediations — one YAML file per cluster.
    const emitted = await emitRemediations({ report, outputDir });
    expect(emitted.length).toBeGreaterThanOrEqual(1);

    const yamlFiles = readdirSync(outputDir).filter((f) => f.endsWith(".yaml"));
    expect(yamlFiles.length).toBeGreaterThanOrEqual(1);

    // 1d. Read the emitted YAML and verify shape.
    const firstEmitted = emitted[0]!;
    const parsedRem = firstEmitted.remediation;
    expect(parsedRem.cluster_id).toBeTruthy();
    // validation-failure maps to tighten-output-schema
    expect(parsedRem.type).toBe("tighten-output-schema");
    const tsRem = parsedRem as TightenOutputSchemaRemediation;
    expect(tsRem.change.field_path).toMatch(/^output_schema\./);
    expect(tsRem.change.add_constraint.maxLength).toBe(500);

    // 1e. Create a synthetic SKILL.md and agent manifest at the path the
    //     remediation targets. For tighten-output-schema the affected file is
    //     a YAML manifest, so we create a manifest there and point the
    //     remediation at it. We need to adjust the affected_skill to a
    //     path-guard-compliant location.
    const repoRoot = join(tmpRoot, "repo");
    const agentSlug = "test-equity-analyst";
    const manifestPath = makeSyntheticManifest(repoRoot, agentSlug);

    // Build a tighten-output-schema remediation pointing at our manifest.
    const adjustedRem: TightenOutputSchemaRemediation = {
      ...(parsedRem as TightenOutputSchemaRemediation),
      affected_skill: `plugins/cfa-core/agents/cfa/${agentSlug}.yaml`,
    };
    const remPath = join(outputDir, "adjusted-remediation.yaml");
    writeFileSync(remPath, yamlStringify(adjustedRem), "utf-8");

    // 1f. Apply the remediation — expect applied: true.
    const applyResult = await applyRemediation({
      remediationFilePath: remPath,
      repoRoot,
    });
    expect(applyResult.applied).toBe(true);
    expect(applyResult.affectedFile).toBe(manifestPath);

    // Verify the constraint was written.
    const updatedManifest = readFileSync(manifestPath, "utf-8");
    expect(updatedManifest).toContain("maxLength");

    // 1g. Re-apply the same remediation — expect applied: false, reason: "already-applied".
    const secondApply = await applyRemediation({
      remediationFilePath: remPath,
      repoRoot,
    });
    expect(secondApply.applied).toBe(false);
    expect(secondApply.reason).toBe("already-applied");
  });
});

// ===========================================================================
// Test 2: Byte-determinism — two independent banks → identical YAML files
// ===========================================================================

describe("skill-editor e2e — byte-determinism (central determinism claim)", () => {
  it("produces byte-identical YAML files from two independent banks seeded identically", async () => {
    const bankDirA = join(tmpRoot, "bankA");
    const bankDirB = join(tmpRoot, "bankB");
    const outputDirA = join(tmpRoot, "outA");
    const outputDirB = join(tmpRoot, "outB");

    // Seed two independent banks with identical records.
    await Promise.all([
      seedBank(bankDirA, 5),
      seedBank(bankDirB, 5),
    ]);

    // Run analyse → emit against each bank independently.
    const [reportA, reportB] = await Promise.all([
      analyseAndEmit(bankDirA, outputDirA),
      analyseAndEmit(bankDirB, outputDirB),
    ]);

    // Both runs should produce at least 1 cluster.
    expect(reportA.clusters.length).toBeGreaterThanOrEqual(1);
    expect(reportB.clusters.length).toBeGreaterThanOrEqual(1);

    // Both output dirs should contain the same number of YAML files.
    const filesA = readdirSync(outputDirA).filter((f) => f.endsWith(".yaml")).sort();
    const filesB = readdirSync(outputDirB).filter((f) => f.endsWith(".yaml")).sort();

    // The number of emitted files matches.
    expect(filesA.length).toEqual(filesB.length);
    expect(filesA.length).toBeGreaterThanOrEqual(1);

    // Byte-for-byte: same filenames.
    expect(filesA).toEqual(filesB);

    // Byte-for-byte: identical file contents.
    for (let i = 0; i < filesA.length; i++) {
      const contentA = readFileSync(join(outputDirA, filesA[i]!), "utf-8");
      const contentB = readFileSync(join(outputDirB, filesB[i]!), "utf-8");

      // The central determinism assertion:
      expect(contentA).toBe(contentB);
    }
  });
});

// ===========================================================================
// Test 3: Path guard rejects forbidden paths
// ===========================================================================

describe("skill-editor e2e — path guard rejects forbidden paths", () => {
  it("throws or returns applied=false when affected_skill is packages/harness/src/index.ts", async () => {
    const repoRoot = join(tmpRoot, "repo");
    mkdirSync(repoRoot, { recursive: true });

    // Craft a remediation pointing at a forbidden internal source file.
    const forbiddenRem: TightenOutputSchemaRemediation = {
      type: "tighten-output-schema",
      cluster_id: "cluster-pathguard01",
      motivating_audit_ids: ["a", "b", "c"],
      confidence_score: 0.5,
      affected_skill: "packages/harness/src/index.ts",
      change: {
        field_path: "output_schema.properties.x",
        add_constraint: { maxLength: 100 },
      },
    };

    const remPath = join(tmpRoot, "forbidden-remediation.yaml");
    writeFileSync(remPath, yamlStringify(forbiddenRem), "utf-8");

    // The path guard must reject this — expect either a throw containing "path"
    // or applied: false with a reason containing "path".
    let threw = false;
    let result: Awaited<ReturnType<typeof applyRemediation>> | undefined;
    try {
      result = await applyRemediation({ remediationFilePath: remPath, repoRoot });
    } catch (err) {
      threw = true;
      expect((err as Error).message.toLowerCase()).toContain("path");
    }

    if (!threw) {
      // If it didn't throw, it should have returned a rejection result.
      expect(result).toBeDefined();
      // Either applied=false or the reason contains "path".
      const reasonLower = (result!.reason ?? "").toLowerCase();
      const affectedLower = (result!.affectedFile ?? "").toLowerCase();
      const rejected =
        result!.applied === false ||
        reasonLower.includes("path") ||
        affectedLower === "";
      expect(rejected).toBe(true);
    }
  });

  it("also rejects src/reasoning/remediation.ts (another internal path)", async () => {
    const repoRoot = join(tmpRoot, "repo");
    mkdirSync(repoRoot, { recursive: true });

    const forbiddenRem: AddSkillSectionRemediation = {
      type: "add-skill-section",
      cluster_id: "cluster-pathguard02",
      motivating_audit_ids: ["a", "b", "c"],
      confidence_score: 0.6,
      affected_skill: "src/reasoning/remediation.ts",
      change: {
        section_title: "Injected",
        section_body_template_id: "anti-injection-reminder",
        insert_after_section: "Overview",
      },
    };

    const remPath = join(tmpRoot, "forbidden2.yaml");
    writeFileSync(remPath, yamlStringify(forbiddenRem), "utf-8");

    await expect(
      applyRemediation({ remediationFilePath: remPath, repoRoot }),
    ).rejects.toThrow(/[Pp]ath/);
  });
});

// ===========================================================================
// Test 4: Apply idempotency across all 4 RemediationTypes
// ===========================================================================

describe("skill-editor e2e — apply idempotency for all RemediationTypes", () => {
  it("add-skill-section: second apply returns applied=false, reason=already-applied", async () => {
    const repoRoot = join(tmpRoot, "repo-add");
    writeStandardTemplates(repoRoot);
    makeSyntheticSkillFile(repoRoot, "idem-skill");

    const rem: AddSkillSectionRemediation = {
      type: "add-skill-section",
      cluster_id: "cluster-idem-add-01",
      motivating_audit_ids: ["a", "b", "c"],
      confidence_score: 0.8,
      affected_skill: "plugins/cfa-core/skills/idem-skill/SKILL.md",
      change: {
        section_title: "Output Validation Reminders",
        section_body_template_id: "anti-injection-reminder",
        insert_after_section: "Output Format",
      },
    };

    const remPath = join(tmpRoot, "idem-add.yaml");
    writeFileSync(remPath, yamlStringify(rem), "utf-8");

    const first = await applyRemediation({
      remediationFilePath: remPath,
      repoRoot,
      templatesDir: join(repoRoot, "docs/skill-editor-templates"),
    });
    expect(first.applied).toBe(true);

    const second = await applyRemediation({
      remediationFilePath: remPath,
      repoRoot,
      templatesDir: join(repoRoot, "docs/skill-editor-templates"),
    });
    expect(second.applied).toBe(false);
    expect(second.reason).toBe("already-applied");
  });

  it("tighten-output-schema: second apply returns applied=false, reason=already-applied", async () => {
    const repoRoot = join(tmpRoot, "repo-tighten");
    makeSyntheticManifest(repoRoot, "idem-agent");

    const rem: TightenOutputSchemaRemediation = {
      type: "tighten-output-schema",
      cluster_id: "cluster-idem-ts-01",
      motivating_audit_ids: ["a", "b", "c"],
      confidence_score: 0.9,
      affected_skill: "plugins/cfa-core/agents/cfa/idem-agent.yaml",
      change: {
        field_path: "output_schema.properties.result",
        add_constraint: { maxLength: 500 },
      },
    };

    const remPath = join(tmpRoot, "idem-ts.yaml");
    writeFileSync(remPath, yamlStringify(rem), "utf-8");

    const first = await applyRemediation({ remediationFilePath: remPath, repoRoot });
    expect(first.applied).toBe(true);

    const second = await applyRemediation({ remediationFilePath: remPath, repoRoot });
    expect(second.applied).toBe(false);
    expect(second.reason).toBe("already-applied");
  });

  it("adjust-tool-allowlist: second apply returns applied=false, reason=already-applied", async () => {
    const repoRoot = join(tmpRoot, "repo-adjust");
    makeSyntheticManifest(repoRoot, "idem-agent-adjust");

    const rem: AdjustToolAllowlistRemediation = {
      type: "adjust-tool-allowlist",
      cluster_id: "cluster-idem-ata-01",
      motivating_audit_ids: ["a", "b", "c"],
      confidence_score: 0.6,
      affected_skill: "plugins/cfa-core/agents/cfa/idem-agent-adjust.yaml",
      change: { action: "add", tool_name: "idempotent-tool", target_path: "block_tools" },
    };

    const remPath = join(tmpRoot, "idem-ata.yaml");
    writeFileSync(remPath, yamlStringify(rem), "utf-8");

    const first = await applyRemediation({ remediationFilePath: remPath, repoRoot });
    expect(first.applied).toBe(true);

    const second = await applyRemediation({ remediationFilePath: remPath, repoRoot });
    expect(second.applied).toBe(false);
    expect(second.reason).toBe("already-applied");
  });

  it("no-action: both applies return applied=false with no-action-type", async () => {
    const repoRoot = join(tmpRoot, "repo-noaction");
    mkdirSync(repoRoot, { recursive: true });

    const rem: NoActionRemediation = {
      type: "no-action",
      cluster_id: "cluster-idem-na-01",
      motivating_audit_ids: ["a", "b", "c"],
      confidence_score: 0.3,
      notes: "Noted for observability. No automated change warranted.",
    };

    const remPath = join(tmpRoot, "idem-na.yaml");
    writeFileSync(remPath, yamlStringify(rem), "utf-8");

    const first = await applyRemediation({ remediationFilePath: remPath, repoRoot });
    expect(first.applied).toBe(false);
    expect(first.reason).toBe("no-action-type");

    const second = await applyRemediation({ remediationFilePath: remPath, repoRoot });
    expect(second.applied).toBe(false);
    expect(second.reason).toBe("no-action-type");
  });
});

// ===========================================================================
// Test 5: No proposals when bank is empty
// ===========================================================================

describe("skill-editor e2e — empty bank produces no proposals", () => {
  it("detects 0 clusters from an empty bank, emitting 0 YAML files", async () => {
    const bankDir = join(tmpRoot, "empty-bank");
    const outputDir = join(tmpRoot, "empty-out");

    // Create the bank without indexing anything.
    const embed = createDeterministicEmbedder();
    const bank = await createRuVectorBank({ dir: bankDir, embed });
    await bank.close();

    // Run detect + emit.
    const bank2 = await createRuVectorBank({ dir: bankDir, embed });
    const report = await detectAllOutliers({ bank: bank2, minClusterSize: 3, embed });
    await bank2.close();

    expect(report.clusters).toHaveLength(0);

    const emitted = await emitRemediations({ report, outputDir });
    expect(emitted).toHaveLength(0);

    const yamlFiles = readdirSync(outputDir).filter((f) => f.endsWith(".yaml"));
    expect(yamlFiles).toHaveLength(0);
  });
});

// ===========================================================================
// Test 6: No proposals when cluster size < minClusterSize
// ===========================================================================

describe("skill-editor e2e — sub-threshold cluster produces no proposals", () => {
  it("indexes 2 validation-failure records with minClusterSize=3 → 0 clusters", async () => {
    const bankDir = join(tmpRoot, "small-bank");
    const outputDir = join(tmpRoot, "small-out");

    // Seed only 2 records (below the min cluster size of 3).
    await seedBank(bankDir, 2);

    const embed = createDeterministicEmbedder();
    const bank = await createRuVectorBank({ dir: bankDir, embed });
    const report = await detectAllOutliers({
      bank,
      minClusterSize: 3,
      embed,
    });
    await bank.close();

    // With minClusterSize=3 and only 2 entries, no validation-failure cluster
    // should be emitted.
    const vfClusters = report.clusters.filter(
      (c) => c.cluster_type === "validation-failure",
    );
    expect(vfClusters).toHaveLength(0);

    // Emit → 0 files for validation-failure specifically, though other cluster
    // types (tool-thrashing, novel) might appear; just confirm nothing from vf.
    const emitted = await emitRemediations({ report, outputDir });
    const vfEmitted = emitted.filter((e) =>
      (e.remediation as TightenOutputSchemaRemediation).type === "tighten-output-schema" &&
      (e.remediation as TightenOutputSchemaRemediation).affected_skill === AGENT_ID,
    );
    expect(vfEmitted).toHaveLength(0);
  });

  it("indexes 2 records with minClusterSize=2 → detects cluster", async () => {
    const bankDir = join(tmpRoot, "min2-bank");

    // Seed 2 records — exactly minClusterSize=2 so a cluster should form.
    await seedBank(bankDir, 2);

    const embed = createDeterministicEmbedder();
    const bank = await createRuVectorBank({ dir: bankDir, embed });
    const report = await detectAllOutliers({
      bank,
      minClusterSize: 2,
      embed,
    });
    await bank.close();

    const vfClusters = report.clusters.filter(
      (c) => c.cluster_type === "validation-failure",
    );
    expect(vfClusters.length).toBeGreaterThanOrEqual(1);
    expect(vfClusters[0]!.motivating_audit_ids.length).toBeGreaterThanOrEqual(2);
  });
});
