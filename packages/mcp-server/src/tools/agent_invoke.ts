import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { agentInvokeRecord, agentInvokeComplete } from "../bindings.js";
import {
  AgentInvokeRecordInputSchema,
  AgentInvokeCompleteInputSchema,
  InvocationAuditPathsSchema,
} from "../schemas/agent_invoke.js";
import { wrapResponse, coerceNumbers } from "../formatters/response.js";

export function registerAgentInvokeTools(server: McpServer) {
  server.tool(
    "agent_invoke_record",
    "Record an agent_invocation_started event and persist a companion .audit.json file under manifest_root. Input: invocation_id (UUID v7), target_agent slug, input_summary, optional parent_invocation_id, optional tenant_id, and manifest_root (absolute path). Output: event_path, audit_path, and surface_audit_hash. Phase 29 Wave 2 — ADR-021 / MAC-INV-002.",
    AgentInvokeRecordInputSchema.shape,
    async (params) => {
      const validated = AgentInvokeRecordInputSchema.parse(coerceNumbers(params));
      const resultJson = agentInvokeRecord(JSON.stringify(validated));
      const parsed = InvocationAuditPathsSchema.parse(JSON.parse(String(resultJson)));
      return wrapResponse(JSON.stringify(parsed));
    }
  );

  server.tool(
    "agent_invoke_complete",
    "Record an agent_invocation_completed event and persist a companion .audit.json file under manifest_root. Input: invocation_id (UUID), output_summary, optional entities array (kind + value), optional tenant_id, and manifest_root (absolute path). Output: event_path, audit_path, and surface_audit_hash. Phase 29 Wave 2 — ADR-021 / MAC-INV-004.",
    AgentInvokeCompleteInputSchema.shape,
    async (params) => {
      const validated = AgentInvokeCompleteInputSchema.parse(coerceNumbers(params));
      const resultJson = agentInvokeComplete(JSON.stringify(validated));
      const parsed = InvocationAuditPathsSchema.parse(JSON.parse(String(resultJson)));
      return wrapResponse(JSON.stringify(parsed));
    }
  );
}
