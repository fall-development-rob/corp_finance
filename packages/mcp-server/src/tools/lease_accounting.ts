import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import {
  classifyLease,
  analyzeSaleLeaseback,
} from "@robotixai/corp-finance-bindings";
import {
  LeaseClassificationSchema,
  SaleLeasebackSchema,
} from "../schemas/lease_accounting.js";
import { wrapResponse, coerceNumbers } from "../formatters/response.js";

export function registerLeaseAccountingTools(server: McpServer) {
  server.tool(
    "lease_classification",
    "Classify a lease under ASC 842 or IFRS 16 and compute ROU asset, lease liability, and month-by-month amortization schedule. Applies the five-test classification framework (ownership transfer, purchase option, specialized asset, 75% economic life, 90% fair value) and generates effective-interest (finance) or straight-line (operating) schedules.",
    LeaseClassificationSchema.shape,
    async (params) => {
      const validated = LeaseClassificationSchema.parse(coerceNumbers(params));
      const result = classifyLease(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "sale_leaseback_analysis",
    "Analyze a sale-leaseback transaction under ASC 842 / IFRS 16. Determines whether the transaction qualifies as a sale (ASC 606 criteria), computes gain/loss recognition with retained right ratio, deferred gains for above-FMV transactions, and failed-sale financing obligation treatment.",
    SaleLeasebackSchema.shape,
    async (params) => {
      const validated = SaleLeasebackSchema.parse(coerceNumbers(params));
      const result = analyzeSaleLeaseback(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );
}
