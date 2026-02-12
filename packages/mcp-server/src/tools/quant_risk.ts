import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import {
  runFactorModel,
  runBlackLitterman,
  calculateRiskParity,
  runStressTest,
} from "@fall-development-rob/corp-finance-bindings";
import {
  FactorModelSchema,
  BlackLittermanSchema,
  RiskParitySchema,
  StressTestSchema,
} from "../schemas/quant_risk.js";
import { wrapResponse, coerceNumbers } from "../formatters/response.js";

export function registerQuantRiskTools(server: McpServer) {
  server.tool(
    "factor_model",
    "Run an OLS factor-model regression on asset returns. Supports CAPM (single factor), Fama-French 3-factor (MKT, SMB, HML), Carhart 4-factor (adds MOM), and Custom (any factors). Returns alpha, factor betas with t-stats and significance, R-squared, adjusted R-squared, Durbin-Watson statistic, and information ratio.",
    FactorModelSchema.shape,
    async (params) => {
      const validated = FactorModelSchema.parse(coerceNumbers(params));
      const result = runFactorModel(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "black_litterman",
    "Run the Black-Litterman portfolio optimisation model. Combines market equilibrium returns (implied by market-cap weights and covariance) with investor views (absolute or relative) to produce posterior expected returns and optimal portfolio weights. Returns equilibrium returns, posterior returns, optimal weights, prior vs posterior comparison, portfolio expected return, volatility, and Sharpe ratio.",
    BlackLittermanSchema.shape,
    async (params) => {
      const validated = BlackLittermanSchema.parse(coerceNumbers(params));
      const result = runBlackLitterman(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "risk_parity",
    "Construct a risk-parity portfolio using inverse volatility, equal risk contribution (ERC), or minimum variance methods. Returns optimal weights, risk contributions per asset, portfolio volatility, expected return, Sharpe ratio, diversification ratio, and effective number of assets.",
    RiskParitySchema.shape,
    async (params) => {
      const validated = RiskParitySchema.parse(coerceNumbers(params));
      const result = calculateRiskParity(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "stress_test",
    "Run portfolio stress tests across multiple historical or hypothetical scenarios. Maps market shocks (equity, rates, credit spreads, FX, commodities, volatility) to portfolio positions based on asset class, beta, and duration. Returns per-scenario P&L impact, per-position breakdown, worst case scenario, average loss, and VaR breach detection.",
    StressTestSchema.shape,
    async (params) => {
      const validated = StressTestSchema.parse(coerceNumbers(params));
      const result = runStressTest(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );
}
