import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import {
  chiefPlanEmit,
  chiefPlanReplan,
  chiefPatternDetect,
  agentTraceGet,
} from "../bindings.js";
import {
  ChiefPlanEmitSchema,
  ChiefPlanReplanSchema,
  ChiefPatternDetectSchema,
  AgentTraceGetSchema,
} from "../schemas/multi_agent.js";
import { wrapResponse, coerceNumbers } from "../formatters/response.js";

// Phase 29 Wave 2: `agent_invoke_record` moved to `tools/agent_invoke.ts`.
// The new tool covers the same RUF-ORC-001 / MAC-INV-002 / MAC-INV-003 contract
// surface and additionally writes the `.audit.json` companion at the manifest
// root. The old binding signature (`{ invocation: AgentInvocation }` -> `{ ok }`)
// was replaced by the flat `{ invocation_id, target_agent, ... }` ->
// `InvocationAuditPaths` shape in `agent_invoke.ts`.

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
