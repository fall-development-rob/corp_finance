/**
 * Phase 35 — Hybrid Dispatch type definitions.
 *
 * These types mirror the Rust CLI contract for `cfa workflow list`,
 * `cfa workflow match`, and `cfa workflow run`. The TypeScript representation
 * is stable; it is the WorkflowRouter's job to translate between these types
 * and the CLI's JSON output.
 */

export interface WorkflowInputSchema {
  type: "object";
  properties: Record<string, { type: string; description: string; required: boolean }>;
  required: string[];
}

export interface WorkflowOutputSchema {
  sections: string[];
  quality_gates: string[];
}

export interface Workflow {
  slug: string;
  name: string;
  domain: string;
  description: string;
  input_schema: WorkflowInputSchema;
  output_schema: WorkflowOutputSchema;
}

export interface WorkflowList {
  total: number;
  workflows: Workflow[];
}

export interface WorkflowMatch {
  slug: string;
  /** Normalized confidence score [0.0, 1.0]. */
  confidence: number;
  extracted_params: Record<string, string | number | boolean>;
}

export interface WorkflowToolCallRecord {
  name: string;
  /** sha256 of JSON-stringified input. */
  input_hash: string;
  /** sha256 of JSON-stringified result. */
  result_hash: string;
  duration_ms: number;
}

export interface WorkflowResult {
  slug: string;
  /** sha256 of canonical execution trace: slug + tool_calls + deliverable. */
  audit_hash: string;
  /** Final markdown output produced by the workflow. */
  deliverable: string;
  tool_calls: WorkflowToolCallRecord[];
  duration_ms: number;
}

/**
 * Structured error thrown by the CLI router for specific failure modes.
 * The harness catches these and falls back to the LLM path.
 */
export class WorkflowRouterError extends Error {
  constructor(
    message: string,
    public readonly code: "PARSE_ERROR" | "TIMEOUT" | "CLI_ERROR",
    public readonly details?: Record<string, unknown>,
  ) {
    super(message);
    this.name = "WorkflowRouterError";
  }
}
