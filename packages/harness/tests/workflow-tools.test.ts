/**
 * W3 — Workflow virtual tool unit tests.
 *
 * Tests cover:
 *   - Tool spec shape for list_workflows and run_workflow
 *   - Predicate functions: isListWorkflowsToolName, isRunWorkflowToolName, isWorkflowToolName
 *   - createWorkflowTools() returns exactly 2 tools
 *   - executeListWorkflowsCall() happy path
 *   - executeListWorkflowsCall() when router.list() throws
 *   - executeRunWorkflowCall() happy path
 *   - executeRunWorkflowCall() with missing slug → is_error=true
 *   - executeRunWorkflowCall() with missing params → is_error=true
 *   - executeRunWorkflowCall() when router.run() throws
 *   - formatWorkflowList() with workflows
 *   - formatWorkflowList() with empty list
 *   - formatWorkflowResult() structure
 */
import { describe, it, expect } from "vitest";
import {
  LIST_WORKFLOWS_TOOL_NAME,
  RUN_WORKFLOW_TOOL_NAME,
  isListWorkflowsToolName,
  isRunWorkflowToolName,
  isWorkflowToolName,
  createListWorkflowsTool,
  createRunWorkflowTool,
  createWorkflowTools,
  formatWorkflowList,
  formatWorkflowResult,
  executeListWorkflowsCall,
  executeRunWorkflowCall,
} from "../src/workflow/tools.js";
import {
  createMockWorkflowRouter,
  WorkflowRouterError,
  type Workflow,
  type WorkflowList,
  type WorkflowResult,
} from "../src/workflow/index.js";

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

const SAMPLE_WORKFLOW: Workflow = {
  slug: "fa-dcf-model",
  name: "DCF Valuation Model",
  domain: "financial_analysis",
  description: "Builds a DCF for a given ticker with WACC and sensitivity.",
  input_schema: {
    type: "object",
    properties: {
      ticker: { type: "string", description: "Stock ticker", required: true },
      wacc: { type: "number", description: "WACC assumption", required: false },
    },
    required: ["ticker"],
  },
  output_schema: {
    sections: ["wacc", "dcf_output", "sensitivity"],
    quality_gates: ["wacc_computed", "dcf_converged"],
  },
};

const SAMPLE_LIST: WorkflowList = { total: 1, workflows: [SAMPLE_WORKFLOW] };

const SAMPLE_RESULT: WorkflowResult = {
  slug: "fa-dcf-model",
  audit_hash: "abc123hash",
  deliverable: "## DCF for AAPL\n\nFair value: $200 per share",
  tool_calls: [
    { name: "wacc_calculator", input_hash: "h1", result_hash: "h2", duration_ms: 30 },
    { name: "dcf_model", input_hash: "h3", result_hash: "h4", duration_ms: 120 },
  ],
  duration_ms: 980,
};

// ---------------------------------------------------------------------------
// Tool spec shape
// ---------------------------------------------------------------------------

describe("createListWorkflowsTool", () => {
  it("Test 1: returns CanonicalTool with correct name", () => {
    const tool = createListWorkflowsTool();
    expect(tool.name).toBe(LIST_WORKFLOWS_TOOL_NAME);
    expect(tool.name).toBe("list_workflows");
  });

  it("Test 2: input_schema has empty required array", () => {
    const tool = createListWorkflowsTool();
    expect(tool.input_schema.type).toBe("object");
    expect(tool.input_schema.required).toEqual([]);
    expect(Object.keys(tool.input_schema.properties)).toHaveLength(0);
  });

  it("Test 3: description mentions discovery and deterministic", () => {
    const tool = createListWorkflowsTool();
    expect(tool.description.toLowerCase()).toContain("discover");
  });
});

describe("createRunWorkflowTool", () => {
  it("Test 4: returns CanonicalTool with correct name", () => {
    const tool = createRunWorkflowTool();
    expect(tool.name).toBe(RUN_WORKFLOW_TOOL_NAME);
    expect(tool.name).toBe("run_workflow");
  });

  it("Test 5: slug and params are in required array", () => {
    const tool = createRunWorkflowTool();
    expect(tool.input_schema.required).toContain("slug");
    expect(tool.input_schema.required).toContain("params");
  });

  it("Test 6: input_schema has slug and params properties", () => {
    const tool = createRunWorkflowTool();
    expect(tool.input_schema.properties["slug"]).toBeDefined();
    expect(tool.input_schema.properties["params"]).toBeDefined();
  });
});

describe("createWorkflowTools", () => {
  it("Test 7: returns exactly 2 tools", () => {
    const tools = createWorkflowTools();
    expect(tools).toHaveLength(2);
  });

  it("Test 8: first tool is list_workflows, second is run_workflow", () => {
    const tools = createWorkflowTools();
    expect(tools[0]?.name).toBe(LIST_WORKFLOWS_TOOL_NAME);
    expect(tools[1]?.name).toBe(RUN_WORKFLOW_TOOL_NAME);
  });
});

// ---------------------------------------------------------------------------
// Predicates
// ---------------------------------------------------------------------------

describe("workflow tool predicates", () => {
  it("Test 9: isListWorkflowsToolName returns true only for list_workflows", () => {
    expect(isListWorkflowsToolName("list_workflows")).toBe(true);
    expect(isListWorkflowsToolName("run_workflow")).toBe(false);
    expect(isListWorkflowsToolName("recall_similar")).toBe(false);
  });

  it("Test 10: isRunWorkflowToolName returns true only for run_workflow", () => {
    expect(isRunWorkflowToolName("run_workflow")).toBe(true);
    expect(isRunWorkflowToolName("list_workflows")).toBe(false);
    expect(isRunWorkflowToolName("dcf_model")).toBe(false);
  });

  it("Test 11: isWorkflowToolName returns true for either workflow tool", () => {
    expect(isWorkflowToolName("list_workflows")).toBe(true);
    expect(isWorkflowToolName("run_workflow")).toBe(true);
    expect(isWorkflowToolName("delegate_to_analyst")).toBe(false);
    expect(isWorkflowToolName("recall_similar")).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// Formatters
// ---------------------------------------------------------------------------

describe("formatWorkflowList", () => {
  it("Test 12: formats a list with workflows", () => {
    const text = formatWorkflowList(SAMPLE_LIST);
    expect(text).toContain("fa-dcf-model");
    expect(text).toContain("financial_analysis");
    expect(text).toContain("ticker");
  });

  it("Test 13: returns 'No workflows available.' for empty list", () => {
    const text = formatWorkflowList({ total: 0, workflows: [] });
    expect(text).toBe("No workflows available.");
  });
});

describe("formatWorkflowResult", () => {
  it("Test 14: includes slug, audit_hash, and deliverable", () => {
    const text = formatWorkflowResult(SAMPLE_RESULT);
    expect(text).toContain("fa-dcf-model");
    expect(text).toContain("abc123hash");
    expect(text).toContain("DCF for AAPL");
  });

  it("Test 15: includes tool call count", () => {
    const text = formatWorkflowResult(SAMPLE_RESULT);
    expect(text).toContain("2");
  });
});

// ---------------------------------------------------------------------------
// executeListWorkflowsCall
// ---------------------------------------------------------------------------

describe("executeListWorkflowsCall", () => {
  it("Test 16: happy path returns is_error=false with workflow list content", async () => {
    const router = createMockWorkflowRouter({ workflows: [SAMPLE_WORKFLOW] });
    const call = { id: "call-1", name: "list_workflows", input: {} };
    const result = await executeListWorkflowsCall(call, router);
    expect(result.call_id).toBe("call-1");
    expect(result.is_error).toBe(false);
    expect(typeof result.content).toBe("string");
    expect(String(result.content)).toContain("fa-dcf-model");
  });

  it("Test 17: error path returns is_error=true with error message", async () => {
    const failingRouter = {
      list: async () => { throw new Error("CLI unavailable"); },
      match: async () => null,
      run: async () => { throw new Error("CLI unavailable"); },
    };
    const call = { id: "call-2", name: "list_workflows", input: {} };
    const result = await executeListWorkflowsCall(call, failingRouter);
    expect(result.is_error).toBe(true);
    expect(String(result.content)).toContain("CLI unavailable");
  });
});

// ---------------------------------------------------------------------------
// executeRunWorkflowCall
// ---------------------------------------------------------------------------

describe("executeRunWorkflowCall", () => {
  it("Test 18: happy path returns is_error=false with deliverable content", async () => {
    const router = createMockWorkflowRouter({ runResult: SAMPLE_RESULT });
    const call = {
      id: "call-3",
      name: "run_workflow",
      input: { slug: "fa-dcf-model", params: { ticker: "AAPL" } },
    };
    const result = await executeRunWorkflowCall(call, router);
    expect(result.call_id).toBe("call-3");
    expect(result.is_error).toBe(false);
    expect(String(result.content)).toContain("DCF for AAPL");
  });

  it("Test 19: missing slug returns is_error=true", async () => {
    const router = createMockWorkflowRouter({ runResult: SAMPLE_RESULT });
    const call = {
      id: "call-4",
      name: "run_workflow",
      input: { params: { ticker: "AAPL" } }, // slug missing
    };
    const result = await executeRunWorkflowCall(call, router);
    expect(result.is_error).toBe(true);
    expect(String(result.content)).toContain("slug");
  });

  it("Test 20: missing params returns is_error=true", async () => {
    const router = createMockWorkflowRouter({ runResult: SAMPLE_RESULT });
    const call = {
      id: "call-5",
      name: "run_workflow",
      input: { slug: "fa-dcf-model" }, // params missing
    };
    const result = await executeRunWorkflowCall(call, router);
    expect(result.is_error).toBe(true);
    expect(String(result.content)).toContain("params");
  });

  it("Test 21: router.run() throws → returns is_error=true with reason", async () => {
    const router = createMockWorkflowRouter({}); // no runResult → throws
    const call = {
      id: "call-6",
      name: "run_workflow",
      input: { slug: "fa-dcf-model", params: { ticker: "AAPL" } },
    };
    const result = await executeRunWorkflowCall(call, router);
    expect(result.is_error).toBe(true);
    expect(String(result.content)).toContain("run_workflow failed");
  });

  it("Test 22: WorkflowRouterError propagates as is_error result", async () => {
    const failingRouter = {
      list: async () => ({ total: 0, workflows: [] }),
      match: async () => null,
      run: async () => {
        throw new WorkflowRouterError("workflow binary not found", "CLI_ERROR");
      },
    };
    const call = {
      id: "call-7",
      name: "run_workflow",
      input: { slug: "fa-dcf-model", params: { ticker: "AAPL" } },
    };
    const result = await executeRunWorkflowCall(call, failingRouter);
    expect(result.is_error).toBe(true);
    expect(String(result.content)).toContain("workflow binary not found");
  });
});
