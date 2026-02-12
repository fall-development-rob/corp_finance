import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import {
  analyzePensionFunding,
  designLdiStrategy,
} from "@rob-otixai/corp-finance-bindings";
import {
  PensionFundingSchema,
  LdiStrategySchema,
} from "../schemas/pension.js";
import { wrapResponse, coerceNumbers } from "../formatters/response.js";

export function registerPensionTools(server: McpServer) {
  server.tool(
    "pension_funding",
    "Comprehensive pension funding analysis. Computes PBO and ABO using unit credit method with salary projections, funding status/ratio, unfunded liability, service cost, interest cost, expected return, net periodic pension cost (NPPC), minimum required and maximum deductible contributions, participant summary, and liability by age cohort.",
    PensionFundingSchema.shape,
    async (params) => {
      const validated = PensionFundingSchema.parse(coerceNumbers(params));
      const result = analyzePensionFunding(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "ldi_strategy",
    "Design a Liability-Driven Investing (LDI) strategy for a pension plan. Analyzes duration gap, constructs a hedging portfolio to match liability duration, evaluates immunization quality, computes surplus-at-risk, and generates a glide-path schedule for transitioning from growth to hedging allocation as funded ratio improves.",
    LdiStrategySchema.shape,
    async (params) => {
      const validated = LdiStrategySchema.parse(coerceNumbers(params));
      const result = designLdiStrategy(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );
}
