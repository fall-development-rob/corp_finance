import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import {
  learnTrajectoryCapture,
  learnTrajectoryComplete,
  learnTrajectoryRetrieve,
  learnReplayRun,
  learnDriftDetect,
  learnGoldenSetFreeze,
} from "../bindings.js";
import {
  LearnTrajectoryCaptureSchema,
  LearnTrajectoryCompleteSchema,
  LearnTrajectoryRetrieveSchema,
  LearnReplayRunSchema,
  LearnDriftDetectSchema,
  LearnGoldenSetFreezeSchema,
} from "../schemas/self_learning.js";
import { wrapResponse, coerceNumbers } from "../formatters/response.js";

export function registerSelfLearningTools(server: McpServer) {
  server.tool(
    "learn_trajectory_capture",
    "Capture a single SurfaceEventRef into the in-flight Trajectory keyed by (surface, surface_event_id). Input: surface, surface_event_id, and a step (kind, name, input_hash, output_hash?, duration_ms). Output: updated trajectory_id and current step count. RUF-LEARN-001.",
    LearnTrajectoryCaptureSchema.shape,
    async (params) => {
      const validated = LearnTrajectoryCaptureSchema.parse(coerceNumbers(params));
      const result = learnTrajectoryCapture(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "learn_trajectory_complete",
    "Seal an in-flight Trajectory and attach a quality grade for downstream cluster training. Input: surface, surface_event_id, and an optional eval_grade (failed/poor/acceptable/good/excellent). Output: completed Trajectory aggregate with sealed step DAG. RUF-LEARN-002 / RUF-LEARN-INV-003.",
    LearnTrajectoryCompleteSchema.shape,
    async (params) => {
      const validated = LearnTrajectoryCompleteSchema.parse(coerceNumbers(params));
      const result = learnTrajectoryComplete(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "learn_trajectory_retrieve",
    "Retrieve trajectories most similar to a query embedding for plan biasing. Input: query_embedding vector, filter (surface? / eval_grade_min? / tenant_id?), and limit. Output: ranked list of Trajectory aggregates with similarity scores. RUF-LEARN-003 / RUF-LEARN-013.",
    LearnTrajectoryRetrieveSchema.shape,
    async (params) => {
      const validated = LearnTrajectoryRetrieveSchema.parse(coerceNumbers(params));
      const result = learnTrajectoryRetrieve(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "learn_replay_run",
    "Replay a frozen GoldenSet against recorded dispatcher outputs and compare digests against expected_digest values. Input: GoldenSet aggregate plus an ordered list of recorded outputs keyed by input_id. Output: ReplayResult with per-input pass/fail and an overall is_clean verdict. RUF-LEARN-005 / RUF-LEARN-009.",
    LearnReplayRunSchema.shape,
    async (params) => {
      const validated = LearnReplayRunSchema.parse(coerceNumbers(params));
      const result = learnReplayRun(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "learn_drift_detect",
    "Combine a byte-diff and recursive structural diff between baseline and current canonical-JSON outputs to produce a DriftVerdict. Input: baseline JSON, current JSON, and tolerance_pct (0-100). Output: DriftReport with byte_diff_pct, structural_diff array, and verdict (no_drift / within_tolerance / beyond_tolerance / critical). RUF-LEARN-007 / RUF-LEARN-INV-008.",
    LearnDriftDetectSchema.shape,
    async (params) => {
      const validated = LearnDriftDetectSchema.parse(coerceNumbers(params));
      const result = learnDriftDetect(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "learn_golden_set_freeze",
    "Freeze a GoldenSet of replay-driven contract inputs and write an ed25519-signed manifest to disk. Input: surface, surface_event_id, ordered GoldenInput fixtures, output_dir, and signing_key_path. Output: persisted GoldenSet aggregate with derived expected_output_digest and SignedManifest. RUF-LEARN-009 / RUF-LEARN-010.",
    LearnGoldenSetFreezeSchema.shape,
    async (params) => {
      const validated = LearnGoldenSetFreezeSchema.parse(coerceNumbers(params));
      const result = learnGoldenSetFreeze(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );
}
