import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import {
  chiefPlanEmit,
  chiefPlanReplan,
  chiefPatternDetect,
  agentInvokeRecord,
  agentTraceGet,
} from "../bindings.js";
import {
  ChiefPlanEmitSchema,
  ChiefPlanReplanSchema,
  ChiefPatternDetectSchema,
  AgentInvokeRecordSchema,
  AgentTraceGetSchema,
} from "../schemas/multi_agent.js";
import { wrapResponse, coerceNumbers } from "../formatters/response.js";

export function registerMultiAgentTools(server: McpServer) {
  server.tool(
    "chief_plan_emit",
    "Emit an A* GOAP plan over the MCP-tool + slash-command action space for the chief-analyst's goal. Input: a free-text goal. Output: a GoapPlan with a deterministic plan_hash and a sequential step DAG. RUF-ORC-003 / MAC-INV-007.",
    ChiefPlanEmitSchema.shape,
    async (params) => {
      const validated = ChiefPlanEmitSchema.parse(coerceNumbers(params));
      const result = chiefPlanEmit(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "chief_plan_replan",
    "Replan a GoapPlan from a failed step, bumping replan_count and re-deriving the plan_hash. Input: existing plan, failed step UUID, and reason. Output: mutated plan with the failed and downstream steps reset; rejects when max_replans is exceeded. RUF-ORC-006 / MAC-INV-006.",
    ChiefPlanReplanSchema.shape,
    async (params) => {
      const validated = ChiefPlanReplanSchema.parse(coerceNumbers(params));
      const result = chiefPlanReplan(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "chief_pattern_detect",
    "Detect recurring entity-graph patterns above a co-occurrence threshold for the chief-analyst's session-level reasoning loop. Input: serialised entity graph and integer threshold. Output: ranked list of detected patterns with supporting EntityRefs. RUF-ORC-005 / MAC-INV-004.",
    ChiefPatternDetectSchema.shape,
    async (params) => {
      const validated = ChiefPatternDetectSchema.parse(coerceNumbers(params));
      const result = chiefPatternDetect(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "agent_invoke_record",
    "Record an Agent-tool delegation at the chief -> specialist boundary, validating the target slug and parent-chain acyclicity. Input: AgentInvocation aggregate. Output: persisted invocation_id and started-event envelope. RUF-ORC-001 / MAC-INV-002 / MAC-INV-003.",
    AgentInvokeRecordSchema.shape,
    async (params) => {
      const validated = AgentInvokeRecordSchema.parse(coerceNumbers(params));
      const result = agentInvokeRecord(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "agent_trace_get",
    "Resolve the ancestor / descendant trace for a recorded AgentInvocation. Input: invocation_id (UUID). Output: ordered chain of parent and child invocations with status and timestamps. RUF-ORC-004 / MAC-INV-009.",
    AgentTraceGetSchema.shape,
    async (params) => {
      const validated = AgentTraceGetSchema.parse(coerceNumbers(params));
      const result = agentTraceGet(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );
}
