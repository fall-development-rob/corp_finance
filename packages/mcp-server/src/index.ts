#!/usr/bin/env node
import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import { registerValuationTools } from "./tools/valuation.js";
import { registerCreditTools } from "./tools/credit.js";
import { registerPETools } from "./tools/pe.js";
import { registerPortfolioTools } from "./tools/portfolio.js";
import { registerScenarioTools } from "./tools/scenarios.js";
import { registerMATools } from "./tools/ma.js";
import { registerJurisdictionTools } from "./tools/jurisdiction.js";
import { registerFixedIncomeTools } from "./tools/fixed_income.js";
import { registerDerivativesTools } from "./tools/derivatives.js";
import { registerThreeStatementTools } from "./tools/three_statement.js";
import { registerMonteCarloTools } from "./tools/monte_carlo.js";
import { registerQuantRiskTools } from "./tools/quant_risk.js";
import { registerRestructuringTools } from "./tools/restructuring.js";
import { registerRealAssetsTools } from "./tools/real_assets.js";
import { registerFxCommoditiesTools } from "./tools/fx_commodities.js";
import { registerSecuritizationTools } from "./tools/securitization.js";
import { registerVentureTools } from "./tools/venture.js";
import { registerEsgTools } from "./tools/esg.js";
import { registerRegulatoryTools } from "./tools/regulatory.js";
import { registerPrivateCreditTools } from "./tools/private_credit.js";
import { registerInsuranceTools } from "./tools/insurance.js";
import { registerFpaTools } from "./tools/fpa.js";
import { registerWealthTools } from "./tools/wealth.js";
import { registerCryptoTools } from "./tools/crypto.js";
import { registerMunicipalTools } from "./tools/municipal.js";
import { registerStructuredProductsTools } from "./tools/structured_products.js";
import { registerTradeFinanceTools } from "./tools/trade_finance.js";

const server = new McpServer({
  name: "corp-finance-mcp",
  version: "0.1.0",
});

registerValuationTools(server);
registerCreditTools(server);
registerPETools(server);
registerPortfolioTools(server);
registerScenarioTools(server);
registerMATools(server);
registerJurisdictionTools(server);
registerFixedIncomeTools(server);
registerDerivativesTools(server);
registerThreeStatementTools(server);
registerMonteCarloTools(server);
registerQuantRiskTools(server);
registerRestructuringTools(server);
registerRealAssetsTools(server);
registerFxCommoditiesTools(server);
registerSecuritizationTools(server);
registerVentureTools(server);
registerEsgTools(server);
registerRegulatoryTools(server);
registerPrivateCreditTools(server);
registerInsuranceTools(server);
registerFpaTools(server);
registerWealthTools(server);
registerCryptoTools(server);
registerMunicipalTools(server);
registerStructuredProductsTools(server);
registerTradeFinanceTools(server);

const transport = new StdioServerTransport();
await server.connect(transport);
