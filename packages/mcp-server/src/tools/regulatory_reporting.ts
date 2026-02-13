import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import {
  generateAifmdReport,
  generateSecCftcReport,
} from "../bindings.js";
import {
  AifmdReportingSchema,
  SecCftcReportingSchema,
} from "../schemas/regulatory_reporting.js";
import { wrapResponse, coerceNumbers } from "../formatters/response.js";

export function registerRegulatoryReportingTools(server: McpServer) {
  server.tool(
    "aifmd_reporting",
    "AIFMD Annex IV report generation: AUM-based reporting frequency, leverage calculation (gross/commitment), liquidity profile analysis, counterparty concentration, stress test impact assessment, NPPR considerations",
    AifmdReportingSchema.shape,
    async (params) => {
      const validated = AifmdReportingSchema.parse(coerceNumbers(params));
      const result = generateAifmdReport(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "sec_cftc_reporting",
    "SEC Form PF and CFTC CPO-PQR report generation: large/small adviser classification, filing frequency determination, qualifying hedge fund identification, counterparty exposure analysis, derivative breakdown, commodity pool assessment",
    SecCftcReportingSchema.shape,
    async (params) => {
      const validated = SecCftcReportingSchema.parse(coerceNumbers(params));
      const result = generateSecCftcReport(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );
}
