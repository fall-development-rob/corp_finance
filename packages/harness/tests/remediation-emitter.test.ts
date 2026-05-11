/**
 * Phase 41 Wave 3 — unit tests for the remediation emitter.
 *
 * Test plan (~12 cases):
 *  1. Empty OutlierReport → 0 remediation files
 *  2. add-skill-section cluster → correctly-shaped AddSkillSectionRemediation
 *  3. tighten-output-schema cluster → correctly-shaped TightenOutputSchemaRemediation
 *  4. adjust-tool-allowlist cluster → correctly-shaped AdjustToolAllowlistRemediation
 *  5. no-action cluster → correctly-shaped NoActionRemediation
 *  6. Idempotent emitter: same report → byte-identical files in two different dirs
 *  7. File naming determinism: <YYYY-MM-DD>-<cluster_id>.yaml
 *  8. Multiple clusters → N files
 *  9. delegation-mismatch maps to add-skill-section with anti-injection template
 * 10. tool-thrashing maps to adjust-tool-allowlist
 * 11. novel cluster maps to add-skill-section with anti-injection template
 * 12. validation-failure cluster maps to tighten-output-schema
 */
import { mkdtempSync, rmSync, readFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { parse as yamlParse } from "yaml";
import { emitRemediations } from "../src/reasoning/remediation.js";
import type { OutlierReport, OutlierCluster } from "../src/reasoning/outliers.js";
import type {
  AddSkillSectionRemediation,
  TightenOutputSchemaRemediation,
  AdjustToolAllowlistRemediation,
  NoActionRemediation,
} from "../src/reasoning/remediation-types.js";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

let tmpDir: string;

beforeEach(() => {
  tmpDir = mkdtempSync(join(tmpdir(), "remediation-emitter-test-"));
});

afterEach(() => {
  rmSync(tmpDir, { recursive: true, force: true });
});

function makeCluster(
  overrides: Partial<OutlierCluster> & { cluster_id: string },
): OutlierCluster {
  return {
    cluster_type: "validation-failure",
    affected_skill: "plugins/cfa-core/agents/cfa/equity-analyst.yaml",
    motivating_audit_ids: ["audit-a", "audit-b", "audit-c"],
    pattern_summary: "Test cluster pattern summary.",
    recommended_action: "tighten-output-schema",
    confidence_score: 0.75,
    ...overrides,
  };
}

function makeReport(clusters: OutlierCluster[]): OutlierReport {
  const to = new Date("2026-05-11T12:00:00.000Z");
  const from = new Date("2026-05-04T12:00:00.000Z");
  return {
    report_id: "test-report-id",
    run_window: { from, to },
    clusters,
  };
}

// ---------------------------------------------------------------------------
// Test 1: Empty report → 0 files
// ---------------------------------------------------------------------------

describe("emitRemediations — empty report", () => {
  it("emits no files when clusters is empty", async () => {
    const report = makeReport([]);
    const outputDir = join(tmpDir, "output");
    const emitted = await emitRemediations({ report, outputDir });
    expect(emitted).toHaveLength(0);
  });
});

// ---------------------------------------------------------------------------
// Test 2: add-skill-section cluster → AddSkillSectionRemediation
// ---------------------------------------------------------------------------

describe("emitRemediations — add-skill-section", () => {
  it("emits AddSkillSectionRemediation for delegation-mismatch cluster", async () => {
    const cluster = makeCluster({
      cluster_id: "cluster-aabbccdd",
      cluster_type: "delegation-mismatch",
      affected_skill: "plugins/cfa-core/skills/equity/SKILL.md",
      recommended_action: "add-skill-section",
      motivating_audit_ids: ["a1", "a2", "a3"],
    });
    const report = makeReport([cluster]);
    const outputDir = join(tmpDir, "out1");
    const emitted = await emitRemediations({ report, outputDir });

    expect(emitted).toHaveLength(1);
    const rem = emitted[0]!.remediation as AddSkillSectionRemediation;
    expect(rem.type).toBe("add-skill-section");
    expect(rem.cluster_id).toBe("cluster-aabbccdd");
    expect(rem.affected_skill).toBe("plugins/cfa-core/skills/equity/SKILL.md");
    expect(rem.change.section_body_template_id).toBe("anti-injection-reminder");
    expect(rem.change.section_title).toBeTruthy();
    expect(rem.change.insert_after_section).toBeTruthy();
    expect(rem.motivating_audit_ids).toEqual(["a1", "a2", "a3"]);
    expect(rem.confidence_score).toBe(0.75);
  });
});

// ---------------------------------------------------------------------------
// Test 3: tighten-output-schema → TightenOutputSchemaRemediation
// ---------------------------------------------------------------------------

describe("emitRemediations — tighten-output-schema", () => {
  it("emits TightenOutputSchemaRemediation for validation-failure cluster", async () => {
    const cluster = makeCluster({
      cluster_id: "cluster-12345678",
      cluster_type: "validation-failure",
      affected_skill: "plugins/cfa-core/agents/cfa/equity-analyst.yaml",
      recommended_action: "tighten-output-schema",
    });
    const report = makeReport([cluster]);
    const outputDir = join(tmpDir, "out2");
    const emitted = await emitRemediations({ report, outputDir });

    const rem = emitted[0]!.remediation as TightenOutputSchemaRemediation;
    expect(rem.type).toBe("tighten-output-schema");
    expect(rem.change.field_path).toMatch(/^output_schema\./);
    expect(rem.change.add_constraint).toBeDefined();
  });
});

// ---------------------------------------------------------------------------
// Test 4: adjust-tool-allowlist → AdjustToolAllowlistRemediation
// ---------------------------------------------------------------------------

describe("emitRemediations — adjust-tool-allowlist", () => {
  it("emits AdjustToolAllowlistRemediation for tool-thrashing cluster", async () => {
    const cluster = makeCluster({
      cluster_id: "cluster-deadbeef",
      cluster_type: "tool-thrashing",
      affected_skill: "plugins/cfa-core/agents/cfa/equity-analyst.yaml",
      recommended_action: "adjust-tool-allowlist",
    });
    const report = makeReport([cluster]);
    const outputDir = join(tmpDir, "out3");
    const emitted = await emitRemediations({ report, outputDir });

    const rem = emitted[0]!.remediation as AdjustToolAllowlistRemediation;
    expect(rem.type).toBe("adjust-tool-allowlist");
    expect(rem.change.action).toBe("remove");
    expect(rem.change.tool_name).toBeTruthy();
    expect(rem.change.target_path).toBe("block_tools");
  });
});

// ---------------------------------------------------------------------------
// Test 5: no-action → NoActionRemediation
// ---------------------------------------------------------------------------

describe("emitRemediations — no-action", () => {
  it("emits NoActionRemediation with fixed notes", async () => {
    const cluster = makeCluster({
      cluster_id: "cluster-00000001",
      cluster_type: "novel",
      recommended_action: "no-action",
    });
    const report = makeReport([cluster]);
    const outputDir = join(tmpDir, "out4");
    const emitted = await emitRemediations({ report, outputDir });

    const rem = emitted[0]!.remediation as NoActionRemediation;
    expect(rem.type).toBe("no-action");
    expect(rem.notes).toContain("cluster-00000001");
    expect(rem.notes).not.toContain("{");  // no template variables left un-filled
  });
});

// ---------------------------------------------------------------------------
// Test 6: Idempotent emitter — same report → byte-identical files
// ---------------------------------------------------------------------------

describe("emitRemediations — byte-determinism", () => {
  it("produces byte-identical YAML for the same report emitted twice", async () => {
    const cluster = makeCluster({
      cluster_id: "cluster-cafecafe",
      cluster_type: "validation-failure",
      recommended_action: "tighten-output-schema",
    });
    const report = makeReport([cluster]);
    const outA = join(tmpDir, "runA");
    const outB = join(tmpDir, "runB");

    const emA = await emitRemediations({ report, outputDir: outA });
    const emB = await emitRemediations({ report, outputDir: outB });

    expect(emA).toHaveLength(1);
    expect(emB).toHaveLength(1);

    const contentA = readFileSync(emA[0]!.filePath, "utf-8");
    const contentB = readFileSync(emB[0]!.filePath, "utf-8");
    expect(contentA).toBe(contentB);
  });
});

// ---------------------------------------------------------------------------
// Test 7: File naming determinism
// ---------------------------------------------------------------------------

describe("emitRemediations — file naming", () => {
  it("names files <YYYY-MM-DD>-<cluster_id>.yaml using run_window.to", async () => {
    const cluster = makeCluster({ cluster_id: "cluster-abcdef01" });
    const report = makeReport([cluster]);
    const outputDir = join(tmpDir, "naming");
    const emitted = await emitRemediations({ report, outputDir });

    const fileName = emitted[0]!.filePath.split("/").pop()!;
    expect(fileName).toBe("2026-05-11-cluster-abcdef01.yaml");
  });
});

// ---------------------------------------------------------------------------
// Test 8: Multiple clusters → N files
// ---------------------------------------------------------------------------

describe("emitRemediations — multiple clusters", () => {
  it("emits one file per cluster", async () => {
    const clusters = [
      makeCluster({ cluster_id: "cluster-11111111", recommended_action: "add-skill-section", cluster_type: "novel" }),
      makeCluster({ cluster_id: "cluster-22222222", recommended_action: "tighten-output-schema", cluster_type: "validation-failure" }),
      makeCluster({ cluster_id: "cluster-33333333", recommended_action: "no-action", cluster_type: "novel" }),
    ];
    const report = makeReport(clusters);
    const outputDir = join(tmpDir, "multi");
    const emitted = await emitRemediations({ report, outputDir });
    expect(emitted).toHaveLength(3);
  });
});

// ---------------------------------------------------------------------------
// Test 9: delegation-mismatch → anti-injection template
// ---------------------------------------------------------------------------

describe("emitRemediations — delegation-mismatch template", () => {
  it("uses anti-injection-reminder template for delegation-mismatch", async () => {
    const cluster = makeCluster({
      cluster_id: "cluster-d1d2d3d4",
      cluster_type: "delegation-mismatch",
      recommended_action: "add-skill-section",
    });
    const report = makeReport([cluster]);
    const outputDir = join(tmpDir, "dm");
    const emitted = await emitRemediations({ report, outputDir });

    const rem = emitted[0]!.remediation as AddSkillSectionRemediation;
    expect(rem.change.section_body_template_id).toBe("anti-injection-reminder");
  });
});

// ---------------------------------------------------------------------------
// Test 10: tool-thrashing → adjust-tool-allowlist
// ---------------------------------------------------------------------------

describe("emitRemediations — tool-thrashing mapping", () => {
  it("maps tool-thrashing to adjust-tool-allowlist with remove action", async () => {
    const cluster = makeCluster({
      cluster_id: "cluster-t1t2t3t4",
      cluster_type: "tool-thrashing",
      recommended_action: "adjust-tool-allowlist",
    });
    const report = makeReport([cluster]);
    const outputDir = join(tmpDir, "tt");
    const emitted = await emitRemediations({ report, outputDir });

    const rem = emitted[0]!.remediation as AdjustToolAllowlistRemediation;
    expect(rem.type).toBe("adjust-tool-allowlist");
    expect(rem.change.action).toBe("remove");
  });
});

// ---------------------------------------------------------------------------
// Test 11: novel → add-skill-section with anti-injection template
// ---------------------------------------------------------------------------

describe("emitRemediations — novel cluster template", () => {
  it("uses anti-injection-reminder template for novel clusters", async () => {
    const cluster = makeCluster({
      cluster_id: "cluster-n0v3ln0v3",
      cluster_type: "novel",
      recommended_action: "add-skill-section",
    });
    const report = makeReport([cluster]);
    const outputDir = join(tmpDir, "novel");
    const emitted = await emitRemediations({ report, outputDir });

    const rem = emitted[0]!.remediation as AddSkillSectionRemediation;
    expect(rem.change.section_body_template_id).toBe("anti-injection-reminder");
  });
});

// ---------------------------------------------------------------------------
// Test 12: validation-failure → tighten-output-schema
// ---------------------------------------------------------------------------

describe("emitRemediations — validation-failure maps to tighten-output-schema", () => {
  it("emitted YAML is valid and parseable", async () => {
    const cluster = makeCluster({
      cluster_id: "cluster-vf123456",
      cluster_type: "validation-failure",
      recommended_action: "tighten-output-schema",
    });
    const report = makeReport([cluster]);
    const outputDir = join(tmpDir, "vf");
    const emitted = await emitRemediations({ report, outputDir });

    const yamlContent = readFileSync(emitted[0]!.filePath, "utf-8");
    const parsed = yamlParse(yamlContent) as { type: string };
    expect(parsed.type).toBe("tighten-output-schema");
  });
});
