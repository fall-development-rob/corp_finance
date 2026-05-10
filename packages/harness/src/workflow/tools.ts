/**
 * Phase 35 Wave 3 — `list_workflows` and `run_workflow` virtual tools.
 *
 * Mirrors `recall-tool.ts` exactly in shape: exported `create*`, `is*`,
 * `execute*` functions; the agent loop appends both tools when
 * `options.workflow` is set and `workflowMode` is not "disabled".
 *
 * Both tools are depth=0 only (chief-level). Specialists dispatched at
 * depth=1 do not receive `options.workflow`, so they never see these tools.
 *
 * The formatter functions return compact markdown so the chief can read
 * the workflow list and deliverable inline without hitting token limits.
 */
import type { CanonicalTool, ToolCall, ToolResult } from "../types.js";
import type { WorkflowRouter } from "./router.js";
import type { WorkflowList, WorkflowResult } from "./types.js";

export const LIST_WORKFLOWS_TOOL_NAME = "list_workflows";
export const RUN_WORKFLOW_TOOL_NAME = "run_workflow";

/** Predicate: is this tool call a `list_workflows` virtual call? */
export function isListWorkflowsToolName(name: string): boolean {
  return name === LIST_WORKFLOWS_TOOL_NAME;
}

/** Predicate: is this tool call a `run_workflow` virtual call? */
export function isRunWorkflowToolName(name: string): boolean {
  return name === RUN_WORKFLOW_TOOL_NAME;
}

/** Predicate: is this tool call either workflow virtual tool? */
export function isWorkflowToolName(name: string): boolean {
  return isListWorkflowsToolName(name) || isRunWorkflowToolName(name);
}

/**
 * Build the `list_workflows` virtual tool spec. Description is verbose to
 * guide the chief toward calling this before attempting manual analysis.
 */
export function createListWorkflowsTool(): CanonicalTool {
  return {
    name: LIST_WORKFLOWS_TOOL_NAME,
    description: [
      "Return the list of pre-defined deterministic workflows the harness can execute.",
      "Use this BEFORE complex analysis to discover whether a matching workflow exists.",
      "Returns workflow slug, description, input_schema, and output_schema for each.",
      "If a workflow matches your task, use run_workflow to execute it — it is faster,",
      "cheaper, and produces audit-hashable outputs identical to the LLM path but",
      "without LLM token cost.",
    ].join(" "),
    input_schema: {
      type: "object",
      properties: {},
      required: [],
    },
  };
}

/**
 * Build the `run_workflow` virtual tool spec. Schema requires `slug` (from
 * list_workflows) and `params` (matching the workflow's input_schema).
 */
export function createRunWorkflowTool(): CanonicalTool {
  return {
    name: RUN_WORKFLOW_TOOL_NAME,
    description: [
      "Execute a pre-defined deterministic workflow.",
      "The workflow runs in Rust with audit-hashed inputs and outputs —",
      "same params yield byte-identical results.",
      "Use this when a list_workflows entry matches the current task.",
      "Returns audit_hash, deliverable, tool_calls, and duration_ms.",
      "Only call after confirming the slug exists in list_workflows output.",
      "Prefer this over delegating to a specialist for workflows that require",
      "no creative synthesis — it is faster, cheaper, and audit-hashable.",
    ].join(" "),
    input_schema: {
      type: "object",
      properties: {
        slug: {
          type: "string",
          description: "Workflow slug from list_workflows",
        },
        params: {
          type: "object",
          additionalProperties: true,
          description: "Inputs matching the workflow's input_schema",
        },
      },
      required: ["slug", "params"],
    },
  };
}

/**
 * Return both workflow virtual tools as an array. The agent loop spreads
 * this into the tools list when `options.workflow` is set.
 */
export function createWorkflowTools(): CanonicalTool[] {
  return [createListWorkflowsTool(), createRunWorkflowTool()];
}

// ---------------------------------------------------------------------------
// Formatters
// ---------------------------------------------------------------------------

/** Format a WorkflowList as compact markdown for the model to read back. */
export function formatWorkflowList(list: WorkflowList): string {
  if (list.workflows.length === 0) {
    return "No workflows available.";
  }
  const lines: string[] = [
    `${list.total} workflow${list.total === 1 ? "" : "s"} available:`,
    "",
  ];
  for (const wf of list.workflows) {
    lines.push(`**${wf.slug}** (${wf.domain})`);
    lines.push(`  ${wf.description}`);
    const requiredInputs = wf.input_schema.required.join(", ");
    if (requiredInputs) {
      lines.push(`  Required inputs: ${requiredInputs}`);
    }
    const sections = wf.output_schema.sections.join(", ");
    if (sections) {
      lines.push(`  Output sections: ${sections}`);
    }
    lines.push("");
  }
  return lines.join("\n").trimEnd();
}

/** Format a WorkflowResult as markdown for the model to read back. */
export function formatWorkflowResult(result: WorkflowResult): string {
  const lines: string[] = [
    `Workflow \`${result.slug}\` completed in ${result.duration_ms}ms.`,
    `Audit hash: \`${result.audit_hash}\``,
    `Tool calls: ${result.tool_calls.length}`,
    "",
    "## Deliverable",
    "",
    result.deliverable,
  ];
  return lines.join("\n");
}

// ---------------------------------------------------------------------------
// Executors
// ---------------------------------------------------------------------------

/** Execute a `list_workflows` virtual tool call. Never throws. */
export async function executeListWorkflowsCall(
  call: ToolCall,
  router: WorkflowRouter,
): Promise<ToolResult> {
  try {
    const list = await router.list();
    return {
      call_id: call.id,
      content: formatWorkflowList(list),
      is_error: false,
    };
  } catch (err) {
    return {
      call_id: call.id,
      content: `list_workflows failed: ${err instanceof Error ? err.message : String(err)}`,
      is_error: true,
    };
  }
}

/** Parse and validate the `run_workflow` tool input. Throws on shape errors. */
function parseRunWorkflowInput(
  input: Record<string, unknown>,
): { slug: string; params: Record<string, unknown> } {
  const slug = input["slug"];
  if (typeof slug !== "string" || slug.trim().length === 0) {
    throw new Error("`slug` must be a non-empty string");
  }
  const params = input["params"];
  if (params === undefined || params === null || typeof params !== "object" || Array.isArray(params)) {
    throw new Error("`params` must be an object");
  }
  return { slug: slug.trim(), params: params as Record<string, unknown> };
}

/** Execute a `run_workflow` virtual tool call. Never throws. */
export async function executeRunWorkflowCall(
  call: ToolCall,
  router: WorkflowRouter,
): Promise<ToolResult> {
  let parsed: { slug: string; params: Record<string, unknown> };
  try {
    parsed = parseRunWorkflowInput(call.input);
  } catch (err) {
    return {
      call_id: call.id,
      content: `run_workflow failed: ${err instanceof Error ? err.message : String(err)}`,
      is_error: true,
    };
  }
  try {
    const result = await router.run(parsed.slug, parsed.params);
    return {
      call_id: call.id,
      content: formatWorkflowResult(result),
      is_error: false,
    };
  } catch (err) {
    return {
      call_id: call.id,
      content: `run_workflow failed: ${err instanceof Error ? err.message : String(err)}`,
      is_error: true,
    };
  }
}
