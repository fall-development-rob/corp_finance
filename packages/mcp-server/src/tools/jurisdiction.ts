import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import {
  calculateFundFees,
  reconcileAccounting,
  calculateWht,
  calculateNav,
  calculateGpEconomics,
  calculateInvestorNetReturns,
  screenUbtiEci,
} from "../bindings.js";
import {
  FundFeeSchema,
  ReconciliationSchema,
  WhtSchema,
  NavSchema,
  GpEconomicsSchema,
  InvestorNetReturnsSchema,
  UbtiScreeningSchema,
} from "../schemas/jurisdiction.js";
import { wrapResponse, coerceNumbers } from "../formatters/response.js";

export function registerJurisdictionTools(server: McpServer) {
  server.tool(
    "fund_fee_calculator",
    "Model fund economics including management fees (committed/invested/NAV basis), performance fees with hurdle rates and catch-up, European and American waterfall structures, GP co-investment returns, and LP net return analysis. Calculates fee drag, DPI, RVPI, TVPI projections across fund life.",
    FundFeeSchema.shape,
    async (params) => {
      const validated = FundFeeSchema.parse(coerceNumbers(params));
      const result = calculateFundFees(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "gaap_ifrs_reconcile",
    "Reconcile financial statements between US GAAP and IFRS accounting standards. Handles key differences including R&D capitalisation, lease accounting (ASC 842 vs IFRS 16), inventory methods (LIFO elimination), goodwill treatment (amortisation vs impairment-only), asset revaluation, and development cost capitalisation. Produces adjusted financial statements with line-by-line reconciliation.",
    ReconciliationSchema.shape,
    async (params) => {
      const validated = ReconciliationSchema.parse(coerceNumbers(params));
      const result = reconcileAccounting(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "withholding_tax",
    "Calculate withholding tax on cross-border income (dividends, interest, royalties, capital gains, rental income). Applies domestic statutory rates, double tax treaty rates where eligible, and special entity exemptions (pension funds, sovereign wealth funds, tax-exempt entities). Supports treaty eligibility checks and beneficial ownership requirements.",
    WhtSchema.shape,
    async (params) => {
      const validated = WhtSchema.parse(coerceNumbers(params));
      const result = calculateWht(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "nav_calculator",
    "Calculate Net Asset Value across multiple share classes with equalisation. Supports depreciation, series accounting, and single-price equalisation methods. Handles subscriptions and redemptions mid-period, management and performance fee accruals per share class, high water mark tracking, and time-weighted return calculations.",
    NavSchema.shape,
    async (params) => {
      const validated = NavSchema.parse(coerceNumbers(params));
      const result = calculateNav(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "gp_economics",
    "Model GP-level economics for a fund management platform. Calculates management fee revenue, carried interest projections, GP commitment returns, staff costs and overhead coverage, fee-related earnings (FRE), distributable earnings, and platform-level economics across multiple funds. Includes fee sharing/offset analysis.",
    GpEconomicsSchema.shape,
    async (params) => {
      const validated = GpEconomicsSchema.parse(coerceNumbers(params));
      const result = calculateGpEconomics(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "investor_net_returns",
    "Calculate investor net returns after all fee layers and taxes. Models management fees, performance fees/carry with hurdle and catch-up, admin fees, placement fees, withholding tax, income tax, capital gains tax, and fund-level expenses. Computes gross-to-net waterfall, total fee drag, net IRR, and net MOIC.",
    InvestorNetReturnsSchema.shape,
    async (params) => {
      const validated = InvestorNetReturnsSchema.parse(coerceNumbers(params));
      const result = calculateInvestorNetReturns(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "ubti_screening",
    "Screen investment portfolios for Unrelated Business Taxable Income (UBTI) and Effectively Connected Income (ECI) exposure. Analyses debt-financed property income, active trade/business income, controlled entity income, and blocker entity effectiveness. Supports tax-exempt foundations, pension funds, endowments, IRAs, and charitable trusts.",
    UbtiScreeningSchema.shape,
    async (params) => {
      const validated = UbtiScreeningSchema.parse(coerceNumbers(params));
      const result = screenUbtiEci(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );
}
