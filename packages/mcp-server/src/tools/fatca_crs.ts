import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import {
  analyzeFatcaCrsReporting,
  classifyEntity,
} from "../bindings.js";
import {
  FatcaCrsReportingSchema,
  EntityClassificationSchema,
} from "../schemas/fatca_crs.js";
import { wrapResponse, coerceNumbers } from "../formatters/response.js";

export function registerFatcaCrsTools(server: McpServer) {
  server.tool(
    "fatca_crs_reporting",
    "FATCA/CRS reporting analysis: IGA model classification, reportable account identification, US indicia detection, GIIN validation, CRS participating jurisdiction mapping",
    FatcaCrsReportingSchema.shape,
    async (params) => {
      const validated = FatcaCrsReportingSchema.parse(coerceNumbers(params));
      const result = analyzeFatcaCrsReporting(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "entity_classification",
    "FATCA/CRS entity classification: FFI vs NFFE determination, passive/active NFFE test, controlling person analysis, sponsored FFI assessment, publicly traded exemption, government entity exclusion",
    EntityClassificationSchema.shape,
    async (params) => {
      const validated = EntityClassificationSchema.parse(coerceNumbers(params));
      const result = classifyEntity(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );
}
