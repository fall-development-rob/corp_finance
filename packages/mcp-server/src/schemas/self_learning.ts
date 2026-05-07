import { z } from "zod";

/**
 * Self-Learning bounded-context schemas. Mirrors
 * `crates/corp-finance-core/src/self_learning/types.rs`:
 *   - Trajectory aggregate (SurfaceEventRef steps, EvalGrade)
 *   - GoldenSet / GoldenInput / SignedManifest (replay-driven contract)
 *   - DriftReport / StructuralDelta / DriftVerdict (replay-time diff)
 *
 * Per ADR-020 / RUF-LEARN-001..013.
 */

const SurfaceEnum = z.enum(["cli", "mcp", "skill", "plugin"]);

const SurfaceEventKindEnum = z.enum([
  "mcp_tool",
  "cli_subcommand",
  "skill_invocation",
  "plugin_hook",
]);

const EvalGradeEnum = z.enum([
  "failed",
  "poor",
  "acceptable",
  "good",
  "excellent",
]);

const SurfaceEventRefSchema = z.object({
  kind: SurfaceEventKindEnum.describe(
    "Surface-event kind: mcp_tool / cli_subcommand / skill_invocation / plugin_hook (RUF-LEARN-001)."
  ),
  name: z
    .string()
    .describe(
      "Surface-specific identifier — MCP tool name, CLI subcommand path, slash-command id, or plugin hook id."
    ),
  input_hash: z
    .string()
    .describe("SHA-256 hex of the canonical JSON of the event input."),
  output_hash: z
    .string()
    .optional()
    .describe(
      "SHA-256 hex of the canonical JSON of the event output, or omitted for failed steps."
    ),
  duration_ms: z
    .number()
    .int()
    .nonnegative()
    .describe("Wall-clock duration of the surface event in milliseconds."),
});

export const LearnTrajectoryCaptureSchema = z.object({
  surface: SurfaceEnum.describe(
    "Top-level surface that initiated this trajectory (cli / mcp / skill / plugin)."
  ),
  surface_event_id: z
    .string()
    .describe(
      "Top-level surface-event id keying the in-flight Trajectory aggregate."
    ),
  step: SurfaceEventRefSchema.describe(
    "One SurfaceEventRef to append to the trajectory step DAG."
  ),
});

export const LearnTrajectoryCompleteSchema = z.object({
  surface: SurfaceEnum.describe(
    "Top-level surface of the trajectory being completed."
  ),
  surface_event_id: z
    .string()
    .describe(
      "Top-level surface-event id of the trajectory being completed."
    ),
  eval_grade: EvalGradeEnum.optional().describe(
    "Quality grade assigned at completion. Required before the trajectory feeds k-means clustering (RUF-LEARN-INV-003)."
  ),
});

const TrajectoryFilterSchema = z.object({
  surface: SurfaceEnum.optional().describe(
    "Restrict candidates to one surface."
  ),
  eval_grade_min: EvalGradeEnum.optional().describe(
    "Floor on EvalGrade — candidates strictly below are filtered out."
  ),
  tenant_id: z
    .string()
    .optional()
    .describe("Federation tenant scope filter (RUF-LEARN-013)."),
});

export const LearnTrajectoryRetrieveSchema = z.object({
  query_embedding: z
    .array(z.number())
    .describe(
      "Dense embedding vector to match against persisted trajectory centroids."
    ),
  filter: TrajectoryFilterSchema.describe(
    "Filter applied before similarity ranking (surface / eval_grade_min / tenant_id)."
  ),
  limit: z
    .number()
    .int()
    .positive()
    .describe("Maximum number of trajectories to return."),
});

const GoldenInputSchema = z.object({
  input_id: z
    .string()
    .uuid()
    .optional()
    .describe(
      "UUID of the frozen replay input. If omitted the core allocates a UUID v7."
    ),
  input_json: z
    .record(z.unknown())
    .describe("Canonical JSON of the surface-event input fixture."),
  expected_digest: z
    .string()
    .describe(
      "SHA-256 hex of the canonical JSON of the surface-event's expected output for this input."
    ),
});

export const LearnReplayRunSchema = z.object({
  golden_set: z
    .record(z.unknown())
    .describe(
      "Frozen GoldenSet aggregate (surface, surface_event_id, inputs, expected_output_digest, signed_manifest)."
    ),
  recorded_outputs: z
    .array(
      z.object({
        input_id: z
          .string()
          .uuid()
          .describe("UUID of the GoldenInput this output corresponds to."),
        output: z
          .record(z.unknown())
          .describe(
            "Canonical JSON output produced by the dispatcher for this input."
          ),
      })
    )
    .describe(
      "Replay outputs recorded by the dispatcher, one per GoldenInput in declared order."
    ),
});

export const LearnDriftDetectSchema = z.object({
  baseline: z
    .record(z.unknown())
    .describe(
      "Baseline canonical JSON output for the (surface, surface_event_id) pair."
    ),
  current: z
    .record(z.unknown())
    .describe(
      "Current canonical JSON output produced by the live surface event."
    ),
  tolerance_pct: z
    .number()
    .min(0)
    .max(100)
    .describe(
      "Maximum allowed byte-diff percentage (0-100). Default per RUF-LEARN-INV-008 is 5%."
    ),
});

export const LearnGoldenSetFreezeSchema = z.object({
  surface: SurfaceEnum.describe(
    "Top-level surface of the GoldenSet being frozen."
  ),
  surface_event_id: z
    .string()
    .describe("Top-level surface-event id of the GoldenSet being frozen."),
  inputs: z
    .array(GoldenInputSchema)
    .describe(
      "Ordered list of GoldenInput fixtures to freeze; expected_output_digest is derived from these."
    ),
  output_dir: z
    .string()
    .describe(
      "Absolute root directory under which <surface>/<surface_event_id>/{manifest.json,inputs/} is written."
    ),
  signing_key_path: z
    .string()
    .describe(
      "Absolute path to the ed25519 signing key used to produce the SignedManifest (RUF-LEARN-009)."
    ),
});
