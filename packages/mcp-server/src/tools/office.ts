import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { writeXlsxWorkbook, renderOfficeTemplate } from "../bindings.js";
import {
  WriteXlsxWorkbookInputSchema,
  WriteWorkbookResultSchema,
  RenderTemplateInputSchema,
} from "../schemas/office.js";
import { wrapResponse, coerceNumbers } from "../formatters/response.js";

export function registerOfficeTools(server: McpServer) {
  server.tool(
    "office_xlsx_write",
    "Serialise a WorkbookSpec to a .xlsx file on disk and return WriteWorkbookResult { output_path, bytes_written, sha256, sheet_count }. Workbooks are terminal deliverables — the result struct is the system-of-record handle, not the file contents. Phase 29 Wave 6.",
    WriteXlsxWorkbookInputSchema.shape,
    async (params) => {
      const validated = WriteXlsxWorkbookInputSchema.parse(coerceNumbers(params));
      const resultJson = writeXlsxWorkbook(JSON.stringify(validated));
      const parsed = WriteWorkbookResultSchema.parse(JSON.parse(String(resultJson)));
      return wrapResponse(JSON.stringify(parsed));
    }
  );

  server.tool(
    "office_render_template",
    "Convert a corp-finance-core compute result into a WorkbookSpec JSON string. " +
    "Accepts { kind: 'dcf'|'comps'|'lbo'|'three_statement', result_json: '<serialised output>' }. " +
    "Returns the WorkbookSpec as JSON — pipe the result directly into office_xlsx_write " +
    "as the spec field to produce a .xlsx file (composable two-step). Phase 29 Wave 6.",
    RenderTemplateInputSchema.shape,
    async (params) => {
      const validated = RenderTemplateInputSchema.parse(params);
      const workbookJson = renderOfficeTemplate(JSON.stringify(validated));
      return wrapResponse(String(workbookJson));
    }
  );
}
