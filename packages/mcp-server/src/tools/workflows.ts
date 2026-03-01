import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import {
  workflowList,
  workflowDescribe,
  workflowValidate,
  workflowQualityCheck,
  workflowAudit,
} from "../bindings.js";
import {
  WorkflowListSchema,
  WorkflowDescribeSchema,
  WorkflowValidateSchema,
  WorkflowQualityCheckSchema,
  WorkflowAuditSchema,
} from "../schemas/workflows.js";
import { wrapResponse, coerceNumbers } from "../formatters/response.js";

export function registerWorkflowTools(server: McpServer) {
  server.tool(
    "workflow_list",
    "List available institutional document workflows with domain filtering. Returns workflow IDs, names, descriptions, and step counts for equity research, investment banking, private equity, wealth management, and financial analysis domains.",
    WorkflowListSchema.shape,
    async (params) => {
      const validated = WorkflowListSchema.parse(coerceNumbers(params));
      const result = workflowList(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "workflow_describe",
    "Describe a workflow in full detail including required inputs, step-by-step process with required tools, quality gates, and expected output sections. Use after workflow_list to get full workflow specification.",
    WorkflowDescribeSchema.shape,
    async (params) => {
      const validated = WorkflowDescribeSchema.parse(coerceNumbers(params));
      const result = workflowDescribe(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "workflow_validate",
    "Validate provided inputs against a workflow's requirements. Returns missing required fields, provided fields, and warnings for optional fields not supplied.",
    WorkflowValidateSchema.shape,
    async (params) => {
      const validated = WorkflowValidateSchema.parse(coerceNumbers(params));
      const result = workflowValidate(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "workflow_quality_check",
    "Run quality gates against workflow outputs. Checks completeness, source verification, scenario inclusion, risk-first ordering, formatting, confidentiality, and citations. Returns pass/fail per gate with overall score.",
    WorkflowQualityCheckSchema.shape,
    async (params) => {
      const validated = WorkflowQualityCheckSchema.parse(coerceNumbers(params));
      const result = workflowQualityCheck(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "workflow_audit",
    "Generate a deterministic audit trail for a workflow execution. Returns step-by-step tool call records, quality gate results, and a reproducible audit hash for compliance.",
    WorkflowAuditSchema.shape,
    async (params) => {
      const validated = WorkflowAuditSchema.parse(coerceNumbers(params));
      const result = workflowAudit(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );
}
