import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import {
  writeXlsxWorkbook,
  renderOfficeTemplate,
  writeWordDoc,
  writeSlideDeck,
  renderDocTemplate,
  renderDeckTemplate,
} from "../bindings.js";
// Tool-specific input wrappers (spec + output_path) have no Rust equivalent;
// they remain in the hand-maintained schema file.
import {
  WriteXlsxWorkbookInputSchema,
  WriteWordDocInputSchema,
  WriteSlideDeckInputSchema,
} from "../schemas/office.js";
// Result schemas and render-input schemas are now sourced from generated (Phase 29 Wave 11).
import {
  WriteWorkbookResultSchema,
  RenderTemplateInputSchema,
  WriteDocResultSchema,
  WriteDeckResultSchema,
  RenderDocTemplateInputSchema,
  RenderDeckTemplateInputSchema,
} from "../schemas/generated/office/index.js";
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

  server.tool(
    "office_docx_write",
    "Serialise a WordDocSpec to a .docx file on disk and return WriteDocResult { output_path, bytes_written, sha256, section_count }. Documents are terminal deliverables — the result struct is the system-of-record handle, not the file contents. Phase 29 Wave 7.",
    WriteWordDocInputSchema.shape,
    async (params) => {
      const validated = WriteWordDocInputSchema.parse(coerceNumbers(params));
      const resultJson = writeWordDoc(JSON.stringify(validated));
      const parsed = WriteDocResultSchema.parse(JSON.parse(String(resultJson)));
      return wrapResponse(JSON.stringify(parsed));
    }
  );

  server.tool(
    "office_pptx_write",
    "Serialise a SlideDeckSpec to a .pptx file on disk and return WriteDeckResult { output_path, bytes_written, sha256, slide_count }. Slide decks are terminal deliverables — the result struct is the system-of-record handle, not the file contents. Phase 29 Wave 8.",
    WriteSlideDeckInputSchema.shape,
    async (params) => {
      const validated = WriteSlideDeckInputSchema.parse(coerceNumbers(params));
      const resultJson = writeSlideDeck(JSON.stringify(validated));
      const parsed = WriteDeckResultSchema.parse(JSON.parse(String(resultJson)));
      return wrapResponse(JSON.stringify(parsed));
    }
  );

  server.tool(
    "office_render_doc_template",
    "Convert a deliverable input (IcMemoInput | ResearchInitInput) into a WordDocSpec JSON string. " +
    "Accepts { kind: 'ic_memo'|'research_init', input_json: '<serialised input struct>' }. " +
    "Returns the WordDocSpec as JSON — pipe directly into office_docx_write to produce a .docx (composable two-step). Phase 29 Wave 9.",
    RenderDocTemplateInputSchema.shape,
    async (params) => {
      const validated = RenderDocTemplateInputSchema.parse(params);
      const docJson = renderDocTemplate(JSON.stringify(validated));
      return wrapResponse(String(docJson));
    }
  );

  server.tool(
    "office_render_deck_template",
    "Convert a deliverable input (PitchDeckInput | IcPresentationInput) into a SlideDeckSpec JSON string. " +
    "Accepts { kind: 'pitch_deck'|'ic_presentation', input_json: '<serialised input struct>' }. " +
    "Returns the SlideDeckSpec as JSON — pipe directly into office_pptx_write to produce a .pptx (composable two-step). Phase 29 Wave 9.",
    RenderDeckTemplateInputSchema.shape,
    async (params) => {
      const validated = RenderDeckTemplateInputSchema.parse(params);
      const deckJson = renderDeckTemplate(JSON.stringify(validated));
      return wrapResponse(String(deckJson));
    }
  );
}
