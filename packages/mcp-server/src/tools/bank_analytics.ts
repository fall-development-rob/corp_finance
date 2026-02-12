import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import {
  analyzeNim,
  calculateCamelsRating,
  calculateCeclProvision,
  analyzeDepositBeta,
  analyzeLoanBook,
} from "@fall-development-rob/corp-finance-bindings";
import {
  NimAnalysisSchema,
  CamelsRatingSchema,
  CeclProvisioningSchema,
  DepositBetaSchema,
  LoanBookAnalysisSchema,
} from "../schemas/bank_analytics.js";
import { wrapResponse, coerceNumbers } from "../formatters/response.js";

export function registerBankAnalyticsTools(server: McpServer) {
  server.tool(
    "nim_analysis",
    "Net interest margin analysis: NIM calculation, rate/volume decomposition, asset/liability mix contribution, interest rate gap",
    NimAnalysisSchema.shape,
    async (params) => {
      const validated = NimAnalysisSchema.parse(coerceNumbers(params));
      const result = analyzeNim(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "camels_rating",
    "CAMELS bank rating: Capital adequacy, Asset quality, Management, Earnings, Liquidity, Sensitivity composite score (1-5)",
    CamelsRatingSchema.shape,
    async (params) => {
      const validated = CamelsRatingSchema.parse(coerceNumbers(params));
      const result = calculateCamelsRating(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "cecl_provisioning",
    "CECL/IFRS 9 expected credit loss: multi-scenario weighted ECL by segment, stage classification, lifetime vs 12-month provision",
    CeclProvisioningSchema.shape,
    async (params) => {
      const validated = CeclProvisioningSchema.parse(coerceNumbers(params));
      const result = calculateCeclProvision(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "deposit_beta",
    "Deposit beta analysis: pass-through rate estimation, cumulative beta, asymmetry analysis (up vs down cycles), repricing lag",
    DepositBetaSchema.shape,
    async (params) => {
      const validated = DepositBetaSchema.parse(coerceNumbers(params));
      const result = analyzeDepositBeta(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "loan_book_analysis",
    "Loan book analysis: sector/geography concentration (HHI), NPL analysis, provision adequacy, weighted average rate and maturity",
    LoanBookAnalysisSchema.shape,
    async (params) => {
      const validated = LoanBookAnalysisSchema.parse(coerceNumbers(params));
      const result = analyzeLoanBook(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );
}
