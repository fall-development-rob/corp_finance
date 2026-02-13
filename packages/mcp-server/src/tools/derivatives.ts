import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { priceOption, impliedVolatility, priceForward, valueForwardPosition, futuresBasisAnalysis, valueInterestRateSwap, valueCurrencySwap, analyzeStrategy } from "../bindings.js";
import { OptionPriceSchema, ImpliedVolSchema, ForwardPriceSchema, ForwardPositionSchema, BasisAnalysisSchema, IrsSchema, CurrencySwapSchema, StrategySchema } from "../schemas/derivatives.js";
import { wrapResponse, coerceNumbers } from "../formatters/response.js";

export function registerDerivativesTools(server: McpServer) {
  server.tool(
    "option_pricer",
    "Price an option using Black-Scholes or binomial model — price, Greeks (delta, gamma, theta, vega, rho), intrinsic/time value",
    OptionPriceSchema.shape,
    async (params) => {
      const validated = OptionPriceSchema.parse(coerceNumbers(params));
      const result = priceOption(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "implied_volatility",
    "Solve for implied volatility from a market option price using Newton-Raphson on Black-Scholes",
    ImpliedVolSchema.shape,
    async (params) => {
      const validated = ImpliedVolSchema.parse(coerceNumbers(params));
      const result = impliedVolatility(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "forward_pricer",
    "Price a forward/futures contract using cost-of-carry model — financial, commodity, or currency underlying",
    ForwardPriceSchema.shape,
    async (params) => {
      const validated = ForwardPriceSchema.parse(coerceNumbers(params));
      const result = priceForward(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "forward_position_value",
    "Value an existing forward position — current value, unrealised P&L, updated forward price",
    ForwardPositionSchema.shape,
    async (params) => {
      const validated = ForwardPositionSchema.parse(coerceNumbers(params));
      const result = valueForwardPosition(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "futures_basis_analysis",
    "Analyse futures basis — raw basis, fair value, mispricing, implied repo rate, contango/backwardation",
    BasisAnalysisSchema.shape,
    async (params) => {
      const validated = BasisAnalysisSchema.parse(coerceNumbers(params));
      const result = futuresBasisAnalysis(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "interest_rate_swap",
    "Value an interest rate swap — NPV, fixed/floating leg PV, cashflow schedule, par swap rate, DV01",
    IrsSchema.shape,
    async (params) => {
      const validated = IrsSchema.parse(coerceNumbers(params));
      const result = valueInterestRateSwap(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "currency_swap",
    "Value a currency swap — NPV in domestic currency, domestic/foreign leg PVs",
    CurrencySwapSchema.shape,
    async (params) => {
      const validated = CurrencySwapSchema.parse(coerceNumbers(params));
      const result = valueCurrencySwap(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "option_strategy",
    "Analyse a multi-leg option strategy — payoff diagram, breakevens, max profit/loss, aggregated Greeks",
    StrategySchema.shape,
    async (params) => {
      const validated = StrategySchema.parse(coerceNumbers(params));
      const result = analyzeStrategy(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );
}
