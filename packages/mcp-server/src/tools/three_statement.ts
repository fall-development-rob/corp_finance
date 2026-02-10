import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { buildThreeStatement } from "corp-finance-bindings";
import { ThreeStatementSchema } from "../schemas/three_statement.js";
import { wrapResponse, coerceNumbers } from "../formatters/response.js";

export function registerThreeStatementTools(server: McpServer) {
  server.tool(
    "three_statement_model",
    "Build a fully linked three-statement financial model (Income Statement, Balance Sheet, Cash Flow Statement) with circular reference resolution for interest expense. Projects revenue, COGS, SG&A, R&D, D&A, working capital, capex, debt repayment, and dividends over multiple years. Computes EBITDA margins, net margins, CAGR, leverage, and cumulative FCF.",
    ThreeStatementSchema.shape,
    async (params) => {
      const validated = ThreeStatementSchema.parse(coerceNumbers(params));
      const result = buildThreeStatement(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );
}
