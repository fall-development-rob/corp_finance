import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import {
  priceLetterOfCredit,
  analyzeSupplyChainFinance,
} from "corp-finance-bindings";
import {
  LetterOfCreditSchema,
  SupplyChainFinanceSchema,
} from "../schemas/trade_finance.js";
import { wrapResponse, coerceNumbers } from "../formatters/response.js";

export function registerTradeFinanceTools(server: McpServer) {
  server.tool(
    "letter_of_credit",
    "Price and analyze letters of credit: total fee calculation (issuance, confirmation, advising, negotiation, amendment fees), all-in cost as percentage and annualized basis points, multi-dimensional risk assessment (country, bank, tenor, documentary risk on 1-10 scale), and banker's acceptance discounting for usance LCs. Supports commercial, standby, revolving, back-to-back, and transferable LCs.",
    LetterOfCreditSchema.shape,
    async (params) => {
      const validated = LetterOfCreditSchema.parse(coerceNumbers(params));
      const result = priceLetterOfCredit(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "supply_chain_finance",
    "Analyze supply chain finance structures: reverse factoring (supplier savings from early payment via buyer's credit), dynamic discounting (optimal discount rates, buyer ROI vs opportunity cost), forfaiting (discount pricing for trade receivables with avalised/non-avalised analysis), and export credit (ECA-backed financing with blended rate, OECD CIRR, amortization schedule). Returns cost analysis, savings, and risk metrics.",
    SupplyChainFinanceSchema.shape,
    async (params) => {
      const validated = SupplyChainFinanceSchema.parse(coerceNumbers(params));
      const result = analyzeSupplyChainFinance(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );
}
