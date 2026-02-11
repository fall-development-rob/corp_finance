import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import {
  analyzeBestExecution,
  generateGipsReport,
} from "corp-finance-bindings";
import {
  BestExecutionSchema,
  GipsReportSchema,
} from "../schemas/compliance.js";
import { wrapResponse, coerceNumbers } from "../formatters/response.js";

export function registerComplianceTools(server: McpServer) {
  server.tool(
    "best_execution",
    "MiFID II best execution analysis: Perold implementation shortfall TCA, benchmark deviation, execution quality scoring, compliance assessment",
    BestExecutionSchema.shape,
    async (params) => {
      const validated = BestExecutionSchema.parse(coerceNumbers(params));
      const result = analyzeBestExecution(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "gips_report",
    "GIPS-compliant performance reporting: Modified Dietz returns, geometric linking, composite dispersion, Sharpe/Information ratios, compliance checklist",
    GipsReportSchema.shape,
    async (params) => {
      const validated = GipsReportSchema.parse(coerceNumbers(params));
      const result = generateGipsReport(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );
}
