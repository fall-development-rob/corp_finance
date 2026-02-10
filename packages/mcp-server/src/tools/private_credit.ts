import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import {
  priceUnitranche,
  modelDirectLoan,
  analyzeSyndication,
} from "corp-finance-bindings";
import {
  UnitrancheSchema,
  DirectLoanSchema,
  SyndicationSchema,
} from "../schemas/private_credit.js";
import { wrapResponse } from "../formatters/response.js";

export function registerPrivateCreditTools(server: McpServer) {
  server.tool(
    "unitranche_pricing",
    "Price a unitranche facility with first-out / last-out split. Computes blended spread, effective yield, all-in cost, leverage and coverage metrics, FO/LO economics split, amortization schedule, and call protection analysis.",
    UnitrancheSchema.shape,
    async (params) => {
      const validated = UnitrancheSchema.parse(params);
      const result = priceUnitranche(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "direct_loan",
    "Model a direct lending loan with cash/PIK interest, delayed draw, amortization schedules, and prepayment penalties. Projects year-by-year cash flows, yield metrics (cash yield, PIK yield, YTM, default-adjusted yield), credit risk metrics (expected loss, credit VaR), and PIK accrual analysis.",
    DirectLoanSchema.shape,
    async (params) => {
      const validated = DirectLoanSchema.parse(params);
      const result = modelDirectLoan(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "syndication_analysis",
    "Analyse loan syndication economics including arranger vs participant fees, oversubscription, pro-rata allocations, arranger economics (arrangement fee, net spread income, total first-year income), and per-participant economics.",
    SyndicationSchema.shape,
    async (params) => {
      const validated = SyndicationSchema.parse(params);
      const result = analyzeSyndication(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );
}
