import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import {
  modelPpp,
  valueConcession,
} from "@fall-development-rob/corp-finance-bindings";
import {
  PppModelSchema,
  ConcessionSchema,
} from "../schemas/infrastructure.js";
import { wrapResponse, coerceNumbers } from "../formatters/response.js";

export function registerInfrastructureTools(server: McpServer) {
  server.tool(
    "ppp_model",
    "Public-private partnership modeling: risk allocation, VfM analysis, PSC comparator, equity IRR, debt sizing",
    PppModelSchema.shape,
    async (params) => {
      const validated = PppModelSchema.parse(coerceNumbers(params));
      const result = modelPpp(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "concession_valuation",
    "Infrastructure concession valuation: traffic/demand risk, toll escalation, handback provisions, regulated asset base",
    ConcessionSchema.shape,
    async (params) => {
      const validated = ConcessionSchema.parse(coerceNumbers(params));
      const result = valueConcession(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );
}
