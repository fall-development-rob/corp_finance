import { z } from "zod";

export const WorkflowListSchema = z.object({
  domain: z.string().optional().describe("Filter by domain: equity_research, investment_banking, private_equity, wealth_management, financial_analysis, deal_documents"),
});

export const WorkflowDescribeSchema = z.object({
  workflow_id: z.string().describe("Workflow ID (e.g., 'er-initiating-coverage', 'ib-cim-builder')"),
});

export const WorkflowValidateSchema = z.object({
  workflow_id: z.string().describe("Workflow ID to validate inputs against"),
  provided_inputs: z.record(z.unknown()).describe("Map of input name to value"),
});

export const WorkflowQualityCheckSchema = z.object({
  workflow_id: z.string().describe("Workflow ID"),
  output_sections: z.array(z.string()).describe("List of sections produced"),
  tool_calls: z.array(z.object({
    tool_name: z.string(),
    input_hash: z.string(),
    output_hash: z.string(),
    timestamp: z.string(),
  })).describe("Record of tool calls made"),
  has_scenarios: z.boolean().describe("Whether base/bull/bear scenarios are included"),
  has_risk_section: z.boolean().describe("Whether risk section is present"),
  has_confidentiality: z.boolean().describe("Whether confidentiality disclaimer is present"),
  has_citations: z.boolean().describe("Whether source citations are included"),
});

export const WorkflowAuditSchema = z.object({
  workflow_id: z.string().describe("Workflow ID"),
  execution: z.object({
    workflow_id: z.string(),
    status: z.string(),
    current_step: z.number(),
    total_steps: z.number(),
    completed_steps: z.array(z.object({
      step_order: z.number(),
      step_name: z.string(),
      tools_used: z.array(z.object({
        tool_name: z.string(),
        input_hash: z.string(),
        output_hash: z.string(),
        timestamp: z.string(),
      })),
      outputs: z.unknown(),
      completed: z.boolean(),
    })),
    quality_results: z.array(z.object({
      gate_name: z.string(),
      check_type: z.string(),
      passed: z.boolean(),
      details: z.string(),
    })),
  }).describe("Workflow execution state"),
});
